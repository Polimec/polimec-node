#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{traits::VestingDurationCalculation, Balance};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	traits::{
		fungible::{Inspect, MutateHold as FungibleMutateHold},
		fungibles::Mutate as FungiblesMutate,
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
};
use on_slash_vesting::OnSlash;
use pallet_proxy_bonding::ReleaseType;
use polimec_common::{
	assets::AcceptedFundingAsset,
	migration_types::{MigrationInfo, MigrationOrigin, MigrationStatus, ParticipationType},
	ReleaseSchedule,
};
use sp_runtime::{traits::Zero, Perquintill};

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_start_settlement(project_id: ProjectId) -> DispatchResult {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let token_information =
			ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?.token_information;
		let now = <frame_system::Pallet<T>>::block_number();

		project_details.funding_end_block = Some(now);

		let escrow_account = Self::fund_account_id(project_id);
		if project_details.status == ProjectStatus::FundingSuccessful {
			let otm_release_type = {
				let multiplier: MultiplierOf<T> =
					ParticipationMode::OTM.multiplier().try_into().map_err(|_| Error::<T>::ImpossibleState)?;
				let duration = multiplier.calculate_vesting_duration::<T>();
				let now = <frame_system::Pallet<T>>::block_number();
				ReleaseType::Locked(duration.saturating_add(now))
			};
			<pallet_proxy_bonding::Pallet<T>>::set_release_type(
				project_id,
				HoldReason::Participation.into(),
				otm_release_type,
			);

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

			Self::transition_project(
				project_id,
				project_details,
				ProjectStatus::FundingSuccessful,
				ProjectStatus::SettlementStarted(FundingOutcome::Success),
				None,
				false,
			)?;
		} else {
			let otm_release_type = ReleaseType::Refunded;
			<pallet_proxy_bonding::Pallet<T>>::set_release_type(
				project_id,
				HoldReason::Participation.into(),
				otm_release_type,
			);

			Self::transition_project(
				project_id,
				project_details,
				ProjectStatus::FundingFailed,
				ProjectStatus::SettlementStarted(FundingOutcome::Failure),
				None,
				false,
			)?;
		}

		Ok(())
	}

	pub fn do_settle_evaluation(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(..)),
			Error::<T>::SettlementNotStarted
		);

		let (plmc_released, ct_rewarded): (Balance, Balance) =
			match project_details.evaluation_round_info.evaluators_outcome {
				Some(EvaluatorsOutcome::Slashed) => (Self::slash_evaluator(&evaluation)?, Zero::zero()),
				Some(EvaluatorsOutcome::Rewarded(info)) => Self::reward_evaluator(project_id, &evaluation, &info)?,
				None => (evaluation.current_plmc_bond, Zero::zero()),
			};

		// Release the held PLMC bond
		T::NativeCurrency::release(
			&HoldReason::Evaluation.into(),
			&evaluation.evaluator,
			plmc_released,
			Precision::Exact,
		)?;

		// Create Migration
		if ct_rewarded > Zero::zero() {
			let multiplier = MultiplierOf::<T>::try_from(1u8).map_err(|_| Error::<T>::BadMath)?;
			let duration = multiplier.calculate_vesting_duration::<T>();
			Self::create_migration(
				project_id,
				&evaluation.evaluator,
				evaluation.id,
				ParticipationType::Evaluation,
				ct_rewarded,
				duration,
				evaluation.receiving_account,
			)?;
		}
		Evaluations::<T>::remove((project_id, evaluation.evaluator.clone(), evaluation.id));

		Self::deposit_event(Event::EvaluationSettled {
			project_id,
			account: evaluation.evaluator,
			id: evaluation.id,
			plmc_released,
			ct_rewarded,
		});

		Ok(())
	}

	pub fn do_settle_bid(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let funding_success =
			matches!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Success));
		let wap = project_details.weighted_average_price.ok_or(Error::<T>::ImpossibleState)?;

		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(..)) || bid.status == BidStatus::Rejected,
			Error::<T>::SettlementNotStarted
		);

		// Return the full bid amount to refund if bid is rejected or project failed,
		// Return a partial amount if the project succeeded, and the wap > paid price or bid is partially accepted
		let BidRefund { final_ct_usd_price, final_ct_amount, refunded_plmc, refunded_funding_asset_amount } =
			Self::calculate_refund(&bid, funding_success, wap)?;

		Self::release_funding_asset(project_id, &bid.bidder, refunded_funding_asset_amount, bid.funding_asset)?;

		if bid.mode == ParticipationMode::OTM {
			if refunded_plmc > T::NativeCurrency::minimum_balance() {
				<pallet_proxy_bonding::Pallet<T>>::refund_fee(
					project_id,
					&bid.bidder,
					refunded_plmc,
					bid.funding_asset.id(),
				)?;
			}
		} else {
			Self::release_participation_bond_for(&bid.bidder, refunded_plmc)?;
		}

		if funding_success && bid.status != BidStatus::Rejected {
			let ct_vesting_duration = Self::set_plmc_bond_release_with_mode(
				bid.bidder.clone(),
				bid.plmc_bond.saturating_sub(refunded_plmc),
				bid.mode,
				project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?,
			)?;

			Self::mint_contribution_tokens(project_id, &bid.bidder, final_ct_amount)?;

			Self::create_migration(
				project_id,
				&bid.bidder,
				bid.id,
				ParticipationType::Bid,
				final_ct_amount,
				ct_vesting_duration,
				bid.receiving_account,
			)?;

			Self::release_funding_asset(
				project_id,
				&project_metadata.funding_destination_account,
				bid.funding_asset_amount_locked.saturating_sub(refunded_funding_asset_amount),
				bid.funding_asset,
			)?;
		}

		Bids::<T>::remove((project_id, bid.bidder.clone(), bid.id));

		Self::deposit_event(Event::BidSettled {
			project_id,
			account: bid.bidder,
			id: bid.id,
			final_ct_amount,
			final_ct_usd_price,
		});

		Ok(())
	}

	/// Calculate the amount of funds the bidder should receive back based on the original bid
	/// amount and price compared to the final bid amount and price.
	fn calculate_refund(
		bid: &BidInfoOf<T>,
		funding_success: bool,
		wap: PriceOf<T>,
	) -> Result<BidRefund<T>, DispatchError> {
		let final_ct_usd_price = if bid.original_ct_usd_price > wap { wap } else { bid.original_ct_usd_price };
		let multiplier: MultiplierOf<T> = bid.mode.multiplier().try_into().map_err(|_| Error::<T>::BadMath)?;
		if bid.status == BidStatus::Rejected || !funding_success {
			return Ok(BidRefund::<T> {
				final_ct_usd_price,
				final_ct_amount: Zero::zero(),
				refunded_plmc: bid.plmc_bond,
				refunded_funding_asset_amount: bid.funding_asset_amount_locked,
			});
		}
		let final_ct_amount = bid.final_ct_amount();

		let new_ticket_size = final_ct_usd_price.checked_mul_int(final_ct_amount).ok_or(Error::<T>::BadMath)?;
		let new_plmc_bond = Self::calculate_plmc_bond(new_ticket_size, multiplier)?;
		let new_funding_asset_amount = Self::calculate_funding_asset_amount(new_ticket_size, bid.funding_asset)?;
		let refunded_plmc = bid.plmc_bond.saturating_sub(new_plmc_bond);
		let refunded_funding_asset_amount = bid.funding_asset_amount_locked.saturating_sub(new_funding_asset_amount);

		Ok(BidRefund::<T> { final_ct_usd_price, final_ct_amount, refunded_plmc, refunded_funding_asset_amount })
	}

	pub fn do_settle_contribution(contribution: ContributionInfoOf<T>, project_id: ProjectId) -> DispatchResult {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut final_ct_amount = Zero::zero();

		let ProjectStatus::SettlementStarted(outcome) = project_details.status else {
			return Err(Error::<T>::SettlementNotStarted.into());
		};
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;

		if outcome == FundingOutcome::Failure {
			Self::release_funding_asset(
				project_id,
				&contribution.contributor,
				contribution.funding_asset_amount,
				contribution.funding_asset,
			)?;

			if contribution.mode == ParticipationMode::OTM {
				<pallet_proxy_bonding::Pallet<T>>::refund_fee(
					project_id,
					&contribution.contributor,
					contribution.plmc_bond,
					contribution.funding_asset.id(),
				)?;
			} else {
				Self::release_participation_bond_for(&contribution.contributor, contribution.plmc_bond)?;
			}
		} else {
			let ct_vesting_duration = Self::set_plmc_bond_release_with_mode(
				contribution.contributor.clone(),
				contribution.plmc_bond,
				contribution.mode,
				funding_end_block,
			)?;

			// Mint the contribution tokens
			Self::mint_contribution_tokens(project_id, &contribution.contributor, contribution.ct_amount)?;

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
				&contribution.contributor,
				contribution.id,
				ParticipationType::Contribution,
				contribution.ct_amount,
				ct_vesting_duration,
				contribution.receiving_account,
			)?;

			final_ct_amount = contribution.ct_amount;
		}

		Contributions::<T>::remove((project_id, contribution.contributor.clone(), contribution.id));

		Self::deposit_event(Event::ContributionSettled {
			project_id,
			account: contribution.contributor,
			id: contribution.id,
			ct_amount: final_ct_amount,
		});

		Ok(())
	}

	pub fn do_mark_project_as_settled(project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let outcome = match project_details.status {
			ProjectStatus::SettlementStarted(ref outcome) => outcome.clone(),
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
		Self::transition_project(
			project_id,
			project_details,
			ProjectStatus::SettlementStarted(outcome.clone()),
			ProjectStatus::SettlementFinished(outcome),
			None,
			false,
		)?;

		Ok(())
	}

	fn mint_contribution_tokens(
		project_id: ProjectId,
		participant: &AccountIdOf<T>,
		amount: Balance,
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
		amount: Balance,
		asset: AcceptedFundingAsset,
	) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		let project_pot = Self::fund_account_id(project_id);
		T::FundingCurrency::transfer(asset.id(), &project_pot, participant, amount, Preservation::Expendable)?;
		Ok(())
	}

	fn release_participation_bond_for(participant: &AccountIdOf<T>, amount: Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		// Release the held PLMC bond
		T::NativeCurrency::release(&HoldReason::Participation.into(), participant, amount, Precision::Exact)?;
		Ok(())
	}

	fn set_plmc_bond_release_with_mode(
		participant: AccountIdOf<T>,
		plmc_amount: Balance,
		mode: ParticipationMode,
		funding_end_block: BlockNumberFor<T>,
	) -> Result<BlockNumberFor<T>, DispatchError> {
		let multiplier: MultiplierOf<T> = mode.multiplier().try_into().map_err(|_| Error::<T>::ImpossibleState)?;
		match mode {
			ParticipationMode::OTM => Ok(multiplier.calculate_vesting_duration::<T>()),
			ParticipationMode::Classic(_) =>
				Self::set_release_schedule_for(&participant, plmc_amount, multiplier, funding_end_block),
		}
	}

	fn set_release_schedule_for(
		participant: &AccountIdOf<T>,
		plmc_amount: Balance,
		multiplier: MultiplierOf<T>,
		funding_end_block: BlockNumberFor<T>,
	) -> Result<BlockNumberFor<T>, DispatchError> {
		// Calculate the vesting info and add the release schedule
		let vesting_info = Self::calculate_vesting_info(participant, multiplier, plmc_amount)?;

		if vesting_info.duration == 1u32.into() {
			Self::release_participation_bond_for(participant, vesting_info.total_amount)?;
		} else {
			VestingOf::<T>::add_release_schedule(
				participant,
				vesting_info.total_amount,
				vesting_info.amount_per_block,
				funding_end_block,
				HoldReason::Participation.into(),
			)?;
		}

		Ok(vesting_info.duration)
	}

	fn slash_evaluator(evaluation: &EvaluationInfoOf<T>) -> Result<Balance, DispatchError> {
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::BlockchainOperationTreasury::get();

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		T::NativeCurrency::transfer_on_hold(
			&HoldReason::Evaluation.into(),
			&evaluation.evaluator,
			&treasury_account,
			slashed_amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;

		T::OnSlash::on_slash(&evaluation.evaluator, slashed_amount);

		Ok(evaluation.current_plmc_bond.saturating_sub(slashed_amount))
	}

	fn reward_evaluator(
		project_id: ProjectId,
		evaluation: &EvaluationInfoOf<T>,
		info: &RewardInfo,
	) -> Result<(Balance, Balance), DispatchError> {
		let reward = Self::calculate_evaluator_reward(evaluation, info);
		Self::mint_contribution_tokens(project_id, &evaluation.evaluator, reward)?;

		Ok((evaluation.current_plmc_bond, reward))
	}

	pub fn calculate_evaluator_reward(evaluation: &EvaluationInfoOf<T>, info: &RewardInfo) -> Balance {
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
		ct_amount: Balance,
		vesting_time: BlockNumberFor<T>,
		receiving_account: Junction,
	) -> DispatchResult {
		UserMigrations::<T>::try_mutate((project_id, origin), |maybe_migrations| -> DispatchResult {
			let migration_origin = MigrationOrigin { user: receiving_account, id, participation_type };
			let vesting_time: u64 = vesting_time.try_into().map_err(|_| Error::<T>::BadMath)?;
			let migration_info: MigrationInfo = (ct_amount, vesting_time).into();
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
