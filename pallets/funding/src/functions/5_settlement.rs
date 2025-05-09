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
	/// Start the settlement round. Now users can mint their contribution tokens or get their funds back, and the issuer
	/// will get the funds in their funding account.
	#[transactional]
	pub fn do_start_settlement(project_id: ProjectId) -> DispatchResult {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let token_information =
			ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?.token_information;
		let now = <T as Config>::BlockNumberProvider::current_block_number();

		project_details.funding_end_block = Some(now);

		let escrow_account = Self::fund_account_id(project_id);
		if project_details.status == ProjectStatus::FundingSuccessful {
			let otm_release_type = {
				let multiplier: MultiplierOf<T> =
					ParticipationMode::OTM.multiplier().try_into().map_err(|_| Error::<T>::ImpossibleState)?;
				let duration = multiplier.calculate_vesting_duration::<T>();
				let now = <T as Config>::BlockNumberProvider::current_block_number();
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
				&escrow_account,
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

	/// Settle an evaluation, by maybe minting CTs, and releasing the PLMC bond.
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

	/// Settle a bid. If bid was successful mint the CTs and release the PLMC bond (if multiplier > 1 and mode is Classic).
	/// If was unsuccessful, release the PLMC bond and refund the funds.
	/// If the project was successful, the issuer will get the funds.
	pub fn do_settle_bid(project_id: ProjectId, bid_id: u32) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let funding_success =
			matches!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Success));
		let mut bid = Bids::<T>::get(project_id, bid_id).ok_or(Error::<T>::ParticipationNotFound)?;

		ensure!(
			matches!(project_details.status, ProjectStatus::SettlementStarted(..)) || bid.status == BidStatus::Rejected,
			Error::<T>::SettlementNotStarted
		);

		if bid.status == BidStatus::YetUnknown {
			bid.status = BidStatus::Accepted;
		}

		// Return the full bid amount to refund if bid is rejected or project failed,
		// Return a partial amount if the project succeeded, and the wap > paid price or bid is partially accepted
		let BidRefund { final_ct_amount, refunded_plmc, refunded_funding_asset_amount } =
			Self::calculate_refund(&bid, funding_success)?;

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

		if funding_success {
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

		Bids::<T>::remove(project_id, bid.id);

		Self::deposit_event(Event::BidSettled {
			project_id,
			account: bid.bidder,
			id: bid.id,
			status: bid.status,
			final_ct_amount,
		});

		Ok(())
	}

	/// Calculate the amount of funds the bidder should receive back based on the original bid
	/// amount and price compared to the final bid amount and price.
	fn calculate_refund(bid: &BidInfoOf<T>, funding_success: bool) -> Result<BidRefund, DispatchError> {
		let multiplier: MultiplierOf<T> = bid.mode.multiplier().try_into().map_err(|_| Error::<T>::BadMath)?;
		let ct_price = bid.original_ct_usd_price;

		match bid.status {
			BidStatus::Accepted if funding_success => Ok(BidRefund {
				final_ct_amount: bid.original_ct_amount,
				refunded_plmc: Zero::zero(),
				refunded_funding_asset_amount: Zero::zero(),
			}),
			BidStatus::PartiallyAccepted(accepted_amount) if funding_success => {
				let new_ticket_size = ct_price.checked_mul_int(accepted_amount).ok_or(Error::<T>::BadMath)?;
				let new_plmc_bond = Self::calculate_plmc_bond(new_ticket_size, multiplier)?;
				let new_funding_asset_amount =
					Self::calculate_funding_asset_amount(new_ticket_size, bid.funding_asset)?;
				let refunded_plmc = bid.plmc_bond.saturating_sub(new_plmc_bond);
				let refunded_funding_asset_amount =
					bid.funding_asset_amount_locked.saturating_sub(new_funding_asset_amount);
				Ok(BidRefund { final_ct_amount: accepted_amount, refunded_plmc, refunded_funding_asset_amount })
			},
			_ => Ok(BidRefund {
				final_ct_amount: Zero::zero(),
				refunded_plmc: bid.plmc_bond,
				refunded_funding_asset_amount: bid.funding_asset_amount_locked,
			}),
		}
	}

	/// Mark a project as fully settled. Only once this is done we can mark migrations as completed.
	pub fn do_mark_project_as_settled(project_id: ProjectId) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let outcome = match project_details.status {
			ProjectStatus::SettlementStarted(ref outcome) => outcome.clone(),
			_ => return Err(Error::<T>::IncorrectRound.into()),
		};

		// We use closers to do an early return if just one of these storage iterators returns a value.
		let no_evaluations_remaining = || Evaluations::<T>::iter_prefix((project_id,)).next().is_none();
		let no_bids_remaining = || Bids::<T>::iter_prefix(project_id).next().is_none();

		// Check if there are any evaluations, bids or contributions remaining
		ensure!(no_evaluations_remaining() && no_bids_remaining(), Error::<T>::SettlementNotComplete);

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

	/// Helper function to Mint CTs and handle the payment of new storage with "touch"
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

	/// Helper function to release the funding asset to the participant
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

	/// Helper function to release the PLMC bond to the participant
	fn release_participation_bond_for(participant: &AccountIdOf<T>, amount: Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		// Release the held PLMC bond
		T::NativeCurrency::release(&HoldReason::Participation.into(), participant, amount, Precision::Exact)?;
		Ok(())
	}

	/// Set the PLMC release schedule if mode was `Classic`. Return the schedule either way.
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

	/// Calculate the vesting info and add the PLMC release schedule to the user, or fully release the funds if possible.
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

	/// Slash an evaluator and transfer funds to the treasury.
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

		T::OnSlash::on_slash(&evaluation.evaluator, &slashed_amount);

		Ok(evaluation.current_plmc_bond.saturating_sub(slashed_amount))
	}

	/// Reward an evaluator and mint CTs.
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
		participation_type: ParticipationType,
		ct_amount: Balance,
		vesting_time: BlockNumberFor<T>,
		receiving_account: Junction,
	) -> DispatchResult {
		let (status, user_migrations) = UserMigrations::<T>::get((project_id, origin))
			.unwrap_or((MigrationStatus::NotStarted, WeakBoundedVec::<_, ConstU32<10_000>>::force_from(vec![], None)));

		if user_migrations.is_empty() {
			UnmigratedCounter::<T>::mutate(project_id, |counter| *counter = counter.saturating_add(1));
		}

		let mut user_migrations = user_migrations.to_vec();
		let migration_origin = MigrationOrigin { user: receiving_account, participation_type };
		let vesting_time: u64 = vesting_time.try_into().map_err(|_| Error::<T>::BadMath)?;
		let migration_info: MigrationInfo = (ct_amount, vesting_time).into();
		let migration = Migration::new(migration_origin, migration_info);
		user_migrations.push(migration);

		UserMigrations::<T>::insert(
			(project_id, origin),
			(status, WeakBoundedVec::<_, ConstU32<10_000>>::force_from(user_migrations, None)),
		);

		Ok(())
	}
}
