use super::*;
use crate::traits::VestingDurationCalculation;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::{
		fungible::{Inspect, MutateHold as FungibleMutateHold},
		fungibles::Mutate as FungiblesMutate,
		tokens::{DepositConsequence, Fortitude, Precision, Preservation, Provenance, Restriction},
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
	#[transactional]
	pub fn do_start_settlement(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let token_information =
			ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?.token_information;
		let now = <frame_system::Pallet<T>>::block_number();
		let round_end_block = project_details.round_duration.end().ok_or(Error::<T>::ImpossibleState)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful ||
				project_details.status == ProjectStatus::FundingFailed,
			Error::<T>::IncorrectRound
		);
		ensure!(now > round_end_block, Error::<T>::TooEarlyForRound);

		// * Calculate new variables *
		project_details.funding_end_block = Some(now);

		let escrow_account = Self::fund_account_id(project_id);
		if project_details.status == ProjectStatus::FundingSuccessful {
			T::ContributionTokenCurrency::create(project_id, escrow_account.clone(), false, 1_u32.into())?;
			T::ContributionTokenCurrency::set(
				project_id,
				&escrow_account.clone(),
				token_information.name.into(),
				token_information.symbol.into(),
				token_information.decimals,
			)?;

			let contribution_token_treasury_account = T::ContributionTreasury::get();
			T::ContributionTokenCurrency::touch(
				project_id,
				&contribution_token_treasury_account,
				&contribution_token_treasury_account,
			)?;

			let (liquidity_pools_ct_amount, long_term_holder_bonus_ct_amount) =
				Self::generate_liquidity_pools_and_long_term_holder_rewards(project_id)?;

			T::ContributionTokenCurrency::mint_into(
				project_id,
				&contribution_token_treasury_account,
				long_term_holder_bonus_ct_amount,
			)?;
			T::ContributionTokenCurrency::mint_into(
				project_id,
				&contribution_token_treasury_account,
				liquidity_pools_ct_amount,
			)?;

			project_details.status = ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful);
			ProjectsDetails::<T>::insert(project_id, &project_details);

			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::start_settlement_funding_success()),
				pays_fee: Pays::Yes,
			})
		} else {
			project_details.status = ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed);
			ProjectsDetails::<T>::insert(project_id, &project_details);

			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::start_settlement_funding_failure()),
				pays_fee: Pays::Yes,
			})
		}
	}

	pub fn do_settle_successful_evaluation(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(
			project_details.status == ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful),
			Error::<T>::FundingSuccessSettlementNotStarted
		);

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
			slashed_plmc_amount: evaluation.current_plmc_bond.saturating_sub(bond),
		});

		Ok(())
	}

	pub fn do_settle_failed_evaluation(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)),
			Error::<T>::FundingFailedSettlementNotStarted
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
			slashed_plmc_amount: evaluation.current_plmc_bond.saturating_sub(bond),
		});

		Ok(())
	}

	pub fn do_settle_successful_bid(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(
			project_details.status == ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful),
			Error::<T>::FundingSuccessSettlementNotStarted
		);
		ensure!(
			matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::ImpossibleState
		);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::TooEarlyForRound);

		let (refund_plmc, refund_funding_asset) = Self::calculate_refund(&bid)?;

		let bidder = bid.bidder;
		// Calculate the vesting info and add the release schedule
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;
		let new_bond = bid.plmc_bond.saturating_sub(refund_plmc);
		let vest_info =
			Self::calculate_vesting_info(&bidder, bid.multiplier, new_bond).map_err(|_| Error::<T>::BadMath)?;

		// If the multiplier is greater than 1, add the release schedule else release the held PLMC bond
		if bid.multiplier.into() > 1u8 {
			if refund_plmc > Zero::zero() {
				Self::release_participation_bond(project_id, &bidder, refund_plmc)?;
			}
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

		let new_funding_asset_amount_locked = bid.funding_asset_amount_locked.saturating_sub(refund_funding_asset);
		if refund_funding_asset > Zero::zero() {
			Self::release_funding_asset(project_id, &bidder, refund_funding_asset, bid.funding_asset)?;
		}

		// Payout the bid funding asset amount to the project account
		Self::release_funding_asset(
			project_id,
			&project_metadata.funding_destination_account,
			new_funding_asset_amount_locked,
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

	/// Calculate the amount of funds the bidder should receive back based on the original bid
	/// amount and price compared to the final bid amount and price.
	fn calculate_refund(bid: &BidInfoOf<T>) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		let new_ticket_size = bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;

		let new_plmc_bond = Self::calculate_plmc_bond(new_ticket_size, bid.multiplier)?;
		let new_funding_asset_amount = Self::calculate_funding_asset_amount(new_ticket_size, bid.funding_asset)?;
		let mut refund_plmc = bid.plmc_bond.saturating_sub(new_plmc_bond);
		let mut refund_funding_asset = bid.funding_asset_amount_locked.saturating_sub(new_funding_asset_amount);
		if T::FundingCurrency::can_deposit(
			bid.funding_asset.to_assethub_id(),
			&bid.bidder,
			refund_funding_asset,
			Provenance::Extant,
		) != DepositConsequence::Success
		{
			refund_funding_asset = Zero::zero();
		}
		if T::NativeCurrency::can_deposit(&bid.bidder, refund_plmc, Provenance::Extant) != DepositConsequence::Success {
			refund_plmc = Zero::zero();
		}

		Ok((refund_plmc, refund_funding_asset))
	}

	pub fn do_settle_failed_bid(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)) ||
				bid.status == BidStatus::Rejected,
			Error::<T>::FundingFailedSettlementNotStarted
		);

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
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// Ensure that:
		// 1. The project is in the FundingSuccessful state
		// 2. The contribution token exists
		ensure!(
			project_details.status == ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful),
			Error::<T>::FundingSuccessSettlementNotStarted
		);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::TooEarlyForRound);

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
			&project_metadata.funding_destination_account,
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

		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)),
			Error::<T>::FundingFailedSettlementNotStarted
		);

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

	pub fn do_mark_project_as_settled(project_id: ProjectId) -> DispatchResult {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let outcome = match project_details.status {
			ProjectStatus::SettlementStarted(outcome) => outcome,
			_ => return Err(Error::<T>::IncorrectRound.into()),
		};

		// We use closers to do an early return if just one of these storage iterators returns a value.
		let no_evaluations_remaining = || Evaluations::<T>::iter_prefix((project_id,)).next().is_none();
		let no_bids_remaining = || Bids::<T>::iter_prefix((project_id,)).next().is_none();
		let no_contributions_remaining = || Contributions::<T>::iter_prefix((project_id,)).next().is_none();

		// Check if there are any evaluations, bids or contributions remaining
		ensure!(
			no_evaluations_remaining() && no_bids_remaining() && no_contributions_remaining(),
			Error::<T>::SettlementNotComplete
		);

		// Mark the project as settled
		project_details.status = ProjectStatus::SettlementFinished(outcome);
		ProjectsDetails::<T>::insert(project_id, project_details);

		Ok(())
	}

	fn mint_contribution_tokens(
		project_id: ProjectId,
		participant: &AccountIdOf<T>,
		amount: BalanceOf<T>,
	) -> DispatchResult {
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
			participant,
			amount,
			Preservation::Expendable,
		)?;
		Ok(())
	}

	fn release_participation_bond(
		project_id: ProjectId,
		participant: &AccountIdOf<T>,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		// Release the held PLMC bond
		T::NativeCurrency::release(
			&HoldReason::Participation(project_id).into(),
			participant,
			amount,
			Precision::Exact,
		)?;
		Ok(())
	}

	fn slash_evaluator(project_id: ProjectId, evaluation: &EvaluationInfoOf<T>) -> Result<BalanceOf<T>, DispatchError> {
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::BlockchainOperationTreasury::get();

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
		let reward = Self::calculate_evaluator_reward(evaluation, info);
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
		UserMigrations::<T>::try_mutate((project_id, origin), |maybe_migrations| -> DispatchResult {
			let multilocation_user = MultiLocation::new(
				0,
				X1(AccountId32 { network: None, id: T::AccountId32Conversion::convert(origin.clone()) }),
			);
			let migration_origin = MigrationOrigin { user: multilocation_user, id, participation_type };
			let vesting_time: u64 = vesting_time.try_into().map_err(|_| Error::<T>::BadMath)?;
			let migration_info: MigrationInfo = (ct_amount.into(), vesting_time).into();
			let migration = Migration::new(migration_origin, migration_info);
			if let Some((_, migrations)) = maybe_migrations {
				migrations.try_push(migration).map_err(|_| Error::<T>::TooManyMigrations)?;
			} else {
				let mut migrations = BoundedVec::<_, MaxParticipationsPerUser<T>>::new();
				migrations.try_push(migration).map_err(|_| Error::<T>::TooManyMigrations)?;
				*maybe_migrations = Some((MigrationStatus::NotStarted, migrations));

				UnmigratedCounter::<T>::mutate(project_id, |counter| *counter = counter.saturating_add(1));
			}

			Ok(())
		})
	}
}
