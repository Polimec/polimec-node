// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@polimec.org
use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_start_settlement(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let token_information =
			ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?.token_information;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful ||
				project_details.status == ProjectStatus::FundingFailed,
			Error::<T>::NotAllowed
		);

		// * Calculate new variables *
		project_details.cleanup =
			Cleaner::try_from(project_details.status.clone()).map_err(|_| Error::<T>::NotAllowed)?;
		project_details.funding_end_block = Some(now);

		// * Update storage *

		ProjectsDetails::<T>::insert(project_id, &project_details);

		if project_details.status == ProjectStatus::FundingSuccessful {
			T::ContributionTokenCurrency::create(project_id, project_details.issuer.clone(), false, 1_u32.into())?;
			T::ContributionTokenCurrency::set(
				project_id,
				&project_details.issuer,
				token_information.name.into(),
				token_information.symbol.into(),
				token_information.decimals,
			)?;
		}

		Ok(())
	}

    pub fn do_bid_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = bid.final_ct_amount;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!bid.ct_minted, Error::<T>::NotAllowed);
		ensure!(matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)), Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		bid.ct_minted = true;

		// * Update storage *
		T::ContributionTokenCurrency::mint_into(project_id, &bid.bidder, ct_amount)?;
		Bids::<T>::insert((project_id, bidder, bid_id), &bid);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id: bid.project_id,
			claimer: bidder.clone(),
			amount: ct_amount,
		});

		Ok(())
	}

	pub fn do_contribution_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = contribution.ct_amount;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!contribution.ct_minted, Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		contribution.ct_minted = true;

		// * Update storage *
		T::ContributionTokenCurrency::mint_into(project_id, &contribution.contributor, ct_amount)?;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id,
			claimer: contributor.clone(),
			amount: ct_amount,
		});

		Ok(())
	}

	pub fn do_evaluation_unbond_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut released_evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;
		let release_amount = released_evaluation.current_plmc_bond;

		// * Validity checks *
		ensure!(
			(project_details.evaluation_round_info.evaluators_outcome == EvaluatorsOutcomeOf::<T>::Unchanged ||
				released_evaluation.rewarded_or_slashed.is_some()) &&
				matches!(
					project_details.status,
					ProjectStatus::EvaluationFailed | ProjectStatus::FundingFailed | ProjectStatus::FundingSuccessful
				),
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(
			&LockType::Evaluation(project_id),
			evaluator,
			released_evaluation.current_plmc_bond,
			Precision::Exact,
		)?;

		released_evaluation.current_plmc_bond = Zero::zero();
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), released_evaluation);

		// FIXME: same question as removing bid
		// Evaluations::<T>::remove((project_id, evaluator, evaluation_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: release_amount,
			bonder: evaluator.clone(),
			releaser: releaser.clone(),
		});

		Ok(())
	}

	pub fn do_evaluation_reward_payout_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let reward_info =
			if let EvaluatorsOutcome::Rewarded(info) = project_details.evaluation_round_info.evaluators_outcome {
				info
			} else {
				return Err(Error::<T>::NotAllowed.into())
			};
		let mut evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			evaluation.rewarded_or_slashed.is_none() &&
				matches!(project_details.status, ProjectStatus::FundingSuccessful),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let early_reward_weight =
			Perquintill::from_rational(evaluation.early_usd_amount, reward_info.early_evaluator_total_bonded_usd);
		let normal_reward_weight = Perquintill::from_rational(
			evaluation.late_usd_amount.saturating_add(evaluation.early_usd_amount),
			reward_info.normal_evaluator_total_bonded_usd,
		);
		let early_evaluators_rewards = early_reward_weight * reward_info.early_evaluator_reward_pot;
		let normal_evaluators_rewards = normal_reward_weight * reward_info.normal_evaluator_reward_pot;
		let total_reward_amount = early_evaluators_rewards.saturating_add(normal_evaluators_rewards);
		// * Update storage *
		T::ContributionTokenCurrency::mint_into(project_id, &evaluation.evaluator, total_reward_amount)?;
		evaluation.rewarded_or_slashed = Some(RewardOrSlash::Reward(total_reward_amount));
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), evaluation);

		// * Emit events *
		Self::deposit_event(Event::EvaluationRewarded {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			amount: total_reward_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	pub fn do_evaluation_slash_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::TreasuryAccount::get();

		let mut evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			evaluation.rewarded_or_slashed.is_none() &&
				matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		// * Update storage *
		evaluation.rewarded_or_slashed = Some(RewardOrSlash::Slash(slashed_amount));

		T::NativeCurrency::transfer_on_hold(
			&LockType::Evaluation(project_id),
			evaluator,
			&treasury_account,
			slashed_amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;

		evaluation.current_plmc_bond.saturating_reduce(slashed_amount);
		Evaluations::<T>::insert((project_id, evaluator, evaluation.id), evaluation);

		// * Emit events *
		Self::deposit_event(Event::EvaluationSlashed {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			amount: slashed_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	pub fn do_start_bid_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;

		// * Validity checks *
		ensure!(
			bid.plmc_vesting_info.is_none() &&
				project_details.status == ProjectStatus::FundingSuccessful &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let vest_info =
			Self::calculate_vesting_info(bidder, bid.multiplier, bid.plmc_bond).map_err(|_| Error::<T>::BadMath)?;
		bid.plmc_vesting_info = Some(vest_info);

		// * Update storage *
		T::Vesting::add_release_schedule(
			bidder,
			vest_info.total_amount,
			vest_info.amount_per_block,
			funding_end_block,
			LockType::Participation(project_id),
		)?;
		Bids::<T>::insert((project_id, bidder, bid_id), bid);

		// * Emit events *
		Self::deposit_event(Event::BidPlmcVestingScheduled {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: vest_info.total_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	pub fn do_start_contribution_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;

		// * Validity checks *
		ensure!(
			contribution.plmc_vesting_info.is_none() && project_details.status == ProjectStatus::FundingSuccessful,
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let vest_info = Self::calculate_vesting_info(contributor, contribution.multiplier, contribution.plmc_bond)
			.map_err(|_| Error::<T>::BadMath)?;
		contribution.plmc_vesting_info = Some(vest_info);

		// * Update storage *
		T::Vesting::add_release_schedule(
			contributor,
			vest_info.total_amount,
			vest_info.amount_per_block,
			funding_end_block,
			LockType::Participation(project_id),
		)?;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionPlmcVestingScheduled {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: vest_info.total_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

    /// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_info(
		_caller: &AccountIdOf<T>,
		multiplier: MultiplierOf<T>,
		bonded_amount: BalanceOf<T>,
	) -> Result<VestingInfo<BlockNumberFor<T>, BalanceOf<T>>, DispatchError> {
		// TODO: duration should depend on `_multiplier` and `_caller` credential
		let duration: BlockNumberFor<T> = multiplier.calculate_vesting_duration::<T>();
		let duration_as_balance = T::BlockNumberToBalance::convert(duration);
		let amount_per_block = if duration_as_balance == Zero::zero() {
			bonded_amount
		} else {
			bonded_amount.checked_div(&duration_as_balance).ok_or(Error::<T>::BadMath)?
		};

		Ok(VestingInfo { total_amount: bonded_amount, amount_per_block, duration })
	}

	pub fn do_vest_plmc_for(
		caller: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(matches!(project_details.status, ProjectStatus::FundingSuccessful), Error::<T>::NotAllowed);

		// * Update storage *
		let vested_amount = T::Vesting::vest(participant.clone(), LockType::Participation(project_id))?;

		// * Emit events *
		Self::deposit_event(Event::ParticipantPlmcVested { project_id, participant, amount: vested_amount, caller });

		Ok(())
	}

	pub fn do_release_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingFailed &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = bid.funding_asset_amount_locked;
		let payout_asset = bid.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
			&project_pot,
			bidder,
			payout_amount,
			Preservation::Expendable,
		)?;
		bid.funds_released = true;
		Bids::<T>::insert((project_id, bidder, bid_id), bid);

		// * Emit events *
		Self::deposit_event(Event::BidFundingReleased {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	// Unbond the PLMC of a bid instantly, following a failed funding outcome.
	// Unbonding of PLMC in a successful funding outcome is handled by the vesting schedule.
	pub fn do_bid_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingFailed &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)) &&
				bid.funds_released,
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(&LockType::Participation(project_id), bidder, bid.plmc_bond, Precision::Exact)?;

		Bids::<T>::remove((project_id, bidder, bid_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: bid.plmc_bond,
			bonder: bidder.clone(),
			releaser: caller.clone(),
		});

		Ok(())
	}

	pub fn do_release_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution = Contributions::<T>::get((project_id, contributor, contribution_id))
			.ok_or(Error::<T>::ContributionNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingFailed, Error::<T>::NotAllowed);

		// * Calculate variables *
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = contribution.funding_asset_amount;
		let payout_asset = contribution.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
			&project_pot,
			contributor,
			payout_amount,
			Preservation::Expendable,
		)?;
		contribution.funds_released = true;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionFundingReleased {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	// Unbond the PLMC of a contribution instantly, following a failed funding outcome.
	// Unbonding of PLMC in a successful funding outcome is handled by the vesting schedule.
	pub fn do_contribution_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bid = Contributions::<T>::get((project_id, contributor, contribution_id))
			.ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingFailed, Error::<T>::NotAllowed);

		// * Update Storage *
		T::NativeCurrency::release(&LockType::Participation(project_id), contributor, bid.plmc_bond, Precision::Exact)?;

		Contributions::<T>::remove((project_id, contributor, contribution_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: bid.plmc_bond,
			bonder: contributor.clone(),
			releaser: caller.clone(),
		});

		Ok(())
	}

	pub fn do_payout_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let issuer = project_details.issuer;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = bid.funding_asset_amount_locked;
		let payout_asset = bid.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
			&project_pot,
			&issuer,
			payout_amount,
			Preservation::Expendable,
		)?;
		bid.funds_released = true;
		Bids::<T>::insert((project_id, bidder, bid_id), &bid);

		// * Emit events *
		Self::deposit_event(Event::BidFundingPaidOut {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	pub fn do_payout_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);

		// * Calculate variables *
		let issuer = project_details.issuer;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = contribution.funding_asset_amount;
		let payout_asset = contribution.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
			&project_pot,
			&issuer,
			payout_amount,
			Preservation::Expendable,
		)?;
		contribution.funds_released = true;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionFundingPaidOut {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}


}