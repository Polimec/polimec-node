use super::*;
use crate::traits::VestingDurationCalculation;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::MutateHold as FungibleMutateHold,
		fungibles::{Inspect, Mutate as FungiblesMutate},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
};
use polimec_common::{
	migration_types::{MigrationInfo, MigrationOrigin, MigrationStatus, ParticipationType},
	ReleaseSchedule,
};
use sp_runtime::{
	traits::{Convert, Zero},
	Perquintill,
};

impl<T: Config> Pallet<T> {
	pub fn do_settle_successful_evaluation(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(matches!(project_details.status, ProjectStatus::FundingSuccessful), Error::<T>::NotAllowed);

		// Based on the results of the funding round, the evaluator is either:
		// 1. Slashed
		// 2. Rewarded with CT tokens
		// 3. Not slashed or Rewarded.
		let (bond, reward): (BalanceOf<T>, BalanceOf<T>) =
			match project_details.evaluation_round_info.evaluators_outcome {
				EvaluatorsOutcome::Slashed => (Self::slash_evaluator(project_id, &evaluation)?, Zero::zero()),
				EvaluatorsOutcome::Rewarded(info) => Self::reward_evaluator(project_id, &evaluation, &info)?,
				EvaluatorsOutcome::Unchanged => (evaluation.current_plmc_bond, Zero::zero()),
			};

		// Release the held PLMC bond
		T::NativeCurrency::release(
			&HoldReason::Evaluation(project_id).into(),
			&evaluation.evaluator,
			bond,
			Precision::Exact,
		)?;

		// Create Migration
		if reward > Zero::zero() {
			let multiplier = MultiplierOf::<T>::try_from(1u8).map_err(|_| Error::<T>::BadMath)?;
			let duration = multiplier.calculate_vesting_duration::<T>();
			Self::create_migration(
				project_id,
				&evaluation.evaluator,
				evaluation.id,
				ParticipationType::Evaluation,
				reward,
				duration,
			)?;
		}
		Evaluations::<T>::remove((project_id, evaluation.evaluator.clone(), evaluation.id));

		Self::deposit_event(Event::EvaluationSettled {
			project_id,
			account: evaluation.evaluator,
			id: evaluation.id,
			ct_amount: reward,
			slashed_amount: evaluation.current_plmc_bond.saturating_sub(bond),
		});

		Ok(())
	}

	pub fn do_settle_failed_evaluation(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(
			matches!(project_details.status, ProjectStatus::FundingFailed | ProjectStatus::EvaluationFailed),
			Error::<T>::NotAllowed
		);

		let bond = if matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed) {
			Self::slash_evaluator(project_id, &evaluation)?
		} else {
			evaluation.current_plmc_bond
		};

		// Release the held PLMC bond
		T::NativeCurrency::release(
			&HoldReason::Evaluation(project_id).into(),
			&evaluation.evaluator,
			bond,
			Precision::Exact,
		)?;

		Evaluations::<T>::remove((project_id, evaluation.evaluator.clone(), evaluation.id));

		Self::deposit_event(Event::EvaluationSettled {
			project_id,
			account: evaluation.evaluator,
			id: evaluation.id,
			ct_amount: Zero::zero(),
			slashed_amount: evaluation.current_plmc_bond.saturating_sub(bond),
		});

		Ok(())
	}

	pub fn do_settle_successful_bid(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)), Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		let bidder = bid.bidder;

		// Calculate the vesting info and add the release schedule
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;
		let vest_info =
			Self::calculate_vesting_info(&bidder, bid.multiplier, bid.plmc_bond).map_err(|_| Error::<T>::BadMath)?;

		// If the multiplier is greater than 1, add the release schedule else release the held PLMC bond
		if bid.multiplier.into() > 1u8 {
			T::Vesting::add_release_schedule(
				&bidder,
				vest_info.total_amount,
				vest_info.amount_per_block,
				funding_end_block,
				HoldReason::Participation(project_id).into(),
			)?;
		} else {
			// Release the held PLMC bond
			Self::release_participation_bond(project_id, &bidder, bid.plmc_bond)?;
		}

		// Mint the contribution tokens
		Self::mint_contribution_tokens(project_id, &bidder, bid.final_ct_amount)?;

		// Payout the bid funding asset amount to the project account
		Self::release_funding_asset(
			project_id,
			&project_details.issuer_account,
			bid.funding_asset_amount_locked,
			bid.funding_asset,
		)?;

		Self::create_migration(
			project_id,
			&bidder,
			bid.id,
			ParticipationType::Bid,
			bid.final_ct_amount,
			vest_info.duration,
		)?;

		Bids::<T>::remove((project_id, bidder.clone(), bid.id));

		Self::deposit_event(Event::BidSettled {
			project_id,
			account: bidder,
			id: bid.id,
			ct_amount: bid.final_ct_amount,
		});

		Ok(())
	}

	pub fn do_settle_failed_bid(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(matches!(project_details.status, ProjectStatus::FundingFailed), Error::<T>::NotAllowed);

		let bidder = bid.bidder;

		// Return the funding assets to the bidder
		Self::release_funding_asset(project_id, &bidder, bid.funding_asset_amount_locked, bid.funding_asset)?;

		// Release the held PLMC bond
		Self::release_participation_bond(project_id, &bidder, bid.plmc_bond)?;

		// Remove the bid from the storage
		Bids::<T>::remove((project_id, bidder.clone(), bid.id));

		Self::deposit_event(Event::BidSettled { project_id, account: bidder, id: bid.id, ct_amount: Zero::zero() });

		Ok(())
	}

	pub fn do_settle_successful_contribution(
		contribution: ContributionInfoOf<T>,
		project_id: ProjectId,
	) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// Ensure that:
		// 1. The project is in the FundingSuccessful state
		// 2. The contribution token exists
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		let contributor = contribution.contributor;

		// Calculate the vesting info and add the release schedule
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;
		let vest_info = Self::calculate_vesting_info(&contributor, contribution.multiplier, contribution.plmc_bond)
			.map_err(|_| Error::<T>::BadMath)?;

		if contribution.multiplier.into() > 1u8 {
			T::Vesting::add_release_schedule(
				&contributor,
				vest_info.total_amount,
				vest_info.amount_per_block,
				funding_end_block,
				HoldReason::Participation(project_id).into(),
			)?;
		} else {
			// Release the held PLMC bond
			Self::release_participation_bond(project_id, &contributor, contribution.plmc_bond)?;
		}

		// Mint the contribution tokens
		Self::mint_contribution_tokens(project_id, &contributor, contribution.ct_amount)?;

		// Payout the bid funding asset amount to the project account
		Self::release_funding_asset(
			project_id,
			&project_details.issuer_account,
			contribution.funding_asset_amount,
			contribution.funding_asset,
		)?;

		// Create Migration
		Self::create_migration(
			project_id,
			&contributor,
			contribution.id,
			ParticipationType::Contribution,
			contribution.ct_amount,
			vest_info.duration,
		)?;

		Contributions::<T>::remove((project_id, contributor.clone(), contribution.id));

		Self::deposit_event(Event::ContributionSettled {
			project_id,
			account: contributor,
			id: contribution.id,
			ct_amount: contribution.ct_amount,
		});

		Ok(())
	}

	pub fn do_settle_failed_contribution(contribution: ContributionInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(matches!(project_details.status, ProjectStatus::FundingFailed), Error::<T>::NotAllowed);

		// Check if the bidder has a future deposit held
		let contributor = contribution.contributor;

		// Return the funding assets to the contributor
		Self::release_funding_asset(
			project_id,
			&contributor,
			contribution.funding_asset_amount,
			contribution.funding_asset,
		)?;

		// Release the held PLMC bond
		Self::release_participation_bond(project_id, &contributor, contribution.plmc_bond)?;

		// Remove the bid from the storage
		Contributions::<T>::remove((project_id, contributor.clone(), contribution.id));

		Self::deposit_event(Event::ContributionSettled {
			project_id,
			account: contributor,
			id: contribution.id,
			ct_amount: Zero::zero(),
		});

		Ok(())
	}

	fn mint_contribution_tokens(project_id: ProjectId, participant: &AccountIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		if !T::ContributionTokenCurrency::contains(&project_id, participant) {
			T::ContributionTokenCurrency::touch(project_id, participant, participant)?;
		}
		T::ContributionTokenCurrency::mint_into(project_id, participant, amount)?;
		Ok(())
	}

	fn release_funding_asset(
		project_id: ProjectId,
		participant: &AccountIdOf<T>,
		amount: BalanceOf<T>,
		asset: AcceptedFundingAsset,
	) -> DispatchResult {
		let project_pot = Self::fund_account_id(project_id);
		T::FundingCurrency::transfer(
			asset.to_assethub_id(),
			&project_pot,
			&participant,
			amount,
			Preservation::Expendable,
		)?;
		Ok(())
	}

	fn release_participation_bond(project_id: ProjectId, participant: &AccountIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		// Release the held PLMC bond
		T::NativeCurrency::release(
			&HoldReason::Participation(project_id).into(),
			&participant,
			amount,
			Precision::Exact,
		)?;
		Ok(())
	}

	fn slash_evaluator(project_id: ProjectId, evaluation: &EvaluationInfoOf<T>) -> Result<BalanceOf<T>, DispatchError> {
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::ProtocolGrowthTreasury::get();

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		T::NativeCurrency::transfer_on_hold(
			&HoldReason::Evaluation(project_id).into(),
			&evaluation.evaluator,
			&treasury_account,
			slashed_amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;

		Ok(evaluation.current_plmc_bond.saturating_sub(slashed_amount))
	}

	fn reward_evaluator(
		project_id: ProjectId,
		evaluation: &EvaluationInfoOf<T>,
		info: &RewardInfoOf<T>,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		let reward = Self::calculate_evaluator_reward(evaluation, &info);
		Self::mint_contribution_tokens(project_id, &evaluation.evaluator, reward)?;

		Ok((evaluation.current_plmc_bond, reward))
	}

	pub fn calculate_evaluator_reward(evaluation: &EvaluationInfoOf<T>, info: &RewardInfoOf<T>) -> BalanceOf<T> {
		let early_reward_weight =
			Perquintill::from_rational(evaluation.early_usd_amount, info.early_evaluator_total_bonded_usd);
		let normal_reward_weight = Perquintill::from_rational(
			evaluation.late_usd_amount.saturating_add(evaluation.early_usd_amount),
			info.normal_evaluator_total_bonded_usd,
		);
		let early_evaluators_rewards = early_reward_weight * info.early_evaluator_reward_pot;
		let normal_evaluators_rewards = normal_reward_weight * info.normal_evaluator_reward_pot;
		early_evaluators_rewards.saturating_add(normal_evaluators_rewards)
	}

	pub fn create_migration(
		project_id: ProjectId,
		origin: &AccountIdOf<T>,
		id: u32,
		participation_type: ParticipationType,
		ct_amount: BalanceOf<T>,
		vesting_time: BlockNumberFor<T>,
	) -> DispatchResult {
		UserMigrations::<T>::try_mutate(project_id, origin, |maybe_migrations| -> DispatchResult {
			let migration_origin =
				MigrationOrigin { user: T::AccountId32Conversion::convert(origin.clone()), id, participation_type };
			let vesting_time: u64 = vesting_time.try_into().map_err(|_| Error::<T>::BadMath)?;
			let migration_info: MigrationInfo = (ct_amount.into(), vesting_time.into()).into();
			let migration = Migration::new(migration_origin, migration_info);
			if let Some((_, migrations)) = maybe_migrations {
				migrations.try_push(migration).map_err(|_| Error::<T>::TooManyMigrations)?;
			} else {
				let mut migrations = BoundedVec::<_, MaxParticipationsPerUser<T>>::new();
				migrations.try_push(migration).map_err(|_| Error::<T>::TooManyMigrations)?;
				*maybe_migrations = Some((MigrationStatus::NotStarted, migrations))
			}

			Ok(())
		})
	}
}
