use frame_support::{
	traits::{fungible::InspectHold, Get},
	weights::Weight,
};
use itertools::Itertools;
use sp_arithmetic::traits::Zero;
use sp_runtime::{traits::AccountIdConversion, DispatchError};
use sp_std::{collections::btree_set::BTreeSet, marker::PhantomData};

use crate::{traits::CleanerOperations, *};

impl<T: Config> CleanerOperations<T> for Cleaner {
	fn has_remaining_operations(&self) -> bool {
		match self {
			Cleaner::NotReady => false,
			Cleaner::Success(state) =>
				<CleanerState<Success> as CleanerOperations<T>>::has_remaining_operations(state),
			Cleaner::Failure(state) =>
				<CleanerState<Failure> as CleanerOperations<T>>::has_remaining_operations(state),
		}
	}

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError> {
		match self {
			Cleaner::NotReady => Err(DispatchError::Other("Cleaner not ready")),
			Cleaner::Success(state) =>
				<CleanerState<Success> as CleanerOperations<T>>::do_one_operation(state, project_id),
			Cleaner::Failure(state) =>
				<CleanerState<Failure> as CleanerOperations<T>>::do_one_operation(state, project_id),
		}
	}
}

impl<T: Config> CleanerOperations<T> for CleanerState<Success> {
	fn has_remaining_operations(&self) -> bool {
		!matches!(self, CleanerState::Finished(_))
	}

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError> {
		let evaluators_outcome = ProjectsDetails::<T>::get(project_id)
			.ok_or(Error::<T>::ImpossibleState)?
			.evaluation_round_info
			.evaluators_outcome;
		let base_weight = Weight::from_parts(10_000_000, 0);

		match self {
			CleanerState::Initialized(PhantomData) => {
				let remaining = CleanerQueueOf::<T>::Success(
					SuccessCleanerQueueOf::<T>::EvaluationRewardsOrSlashes(
						remaining_evaluations_to_reward_or_slash::<T>(project_id, evaluators_outcome),
					),
				);
				CleanerQueue::<T>::insert(project_id, remaining);
				*self = Self::EvaluationRewardOrSlash(
					PhantomData,
				);
				Ok(base_weight)
			},
			CleanerState::EvaluationRewardOrSlash(PhantomData) =>
				let remaining_evaluations =
				if *remaining == 0 {
					*self = Self::EvaluationUnbonding(remaining_evaluations::<T>(project_id), PhantomData);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluations) = reward_or_slash_one_evaluation::<T>(project_id)?;
					*self = CleanerState::EvaluationRewardOrSlash(remaining_evaluations, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::EvaluationUnbonding(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::StartBidderVestingSchedule(
						remaining_successful_bids::<T>(project_id),
						PhantomData,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluations) = unbond_one_evaluation::<T>(project_id);
					*self = CleanerState::EvaluationUnbonding(remaining_evaluations, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::StartBidderVestingSchedule(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::StartContributorVestingSchedule(
						remaining_contributions::<T>(project_id),
						PhantomData,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluations) = start_one_bid_vesting_schedule::<T>(project_id);
					*self = CleanerState::StartBidderVestingSchedule(remaining_evaluations, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::StartContributorVestingSchedule(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::BidCTMint(remaining_bids_without_ct_minted::<T>(project_id), PhantomData);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluations) =
						start_one_contribution_vesting_schedule::<T>(project_id);
					*self = CleanerState::StartContributorVestingSchedule(remaining_evaluations, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::BidCTMint(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::ContributionCTMint(
						remaining_contributions_without_ct_minted::<T>(project_id),
						PhantomData,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_bids) = mint_ct_for_one_bid::<T>(project_id);
					*self = CleanerState::BidCTMint(remaining_bids, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::ContributionCTMint(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::BidFundingPayout(
						remaining_bids_without_issuer_payout::<T>(project_id),
						PhantomData,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_contributions) = mint_ct_for_one_contribution::<T>(project_id);
					*self = CleanerState::ContributionCTMint(remaining_contributions, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::BidFundingPayout(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::ContributionFundingPayout(
						remaining_contributions_without_issuer_payout::<T>(project_id),
						PhantomData,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_contributions) = issuer_funding_payout_one_bid::<T>(project_id);
					*self = CleanerState::BidFundingPayout(remaining_contributions, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::ContributionFundingPayout(PhantomData) =>
				if *remaining == 0 {
					*self = CleanerState::Finished(PhantomData);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_contributions) =
						issuer_funding_payout_one_contribution::<T>(project_id);
					*self = CleanerState::ContributionFundingPayout(remaining_contributions, PhantomData);
					Ok(consumed_weight)
				},
			CleanerState::Finished(PhantomData) => Err(Error::<T>::FinalizerFinished.into()),

			_ => Err(Error::<T>::ImpossibleState.into()),
		}
	}
}
impl<T: Config> CleanerOperations<T> for CleanerState<Failure> {
	fn has_remaining_operations(&self) -> bool {
		!matches!(self, CleanerState::Finished(PhantomData::<Failure>))
	}

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError> {
		let evaluators_outcome = ProjectsDetails::<T>::get(project_id)
			.ok_or(Error::<T>::ImpossibleState)?
			.evaluation_round_info
			.evaluators_outcome;
		let base_weight = Weight::from_parts(10_000_000, 0);

		match self {
			CleanerState::Initialized(PhantomData::<Failure>) => {
				*self = CleanerState::EvaluationRewardOrSlash(PhantomData::<Failure>);
				CleanerQueue::<T>::remove(project_id);
				Ok(base_weight)
			},

			CleanerState::EvaluationRewardOrSlash(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::FutureDepositRelease(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluators) = reward_or_slash_one_evaluation::<T>(project_id)?;
					*self = CleanerState::EvaluationRewardOrSlash(remaining_evaluators, PhantomData);
					Ok(consumed_weight)
				},

			CleanerState::FutureDepositRelease(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::EvaluationUnbonding(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_participants) =
						release_future_ct_deposit_one_participant::<T>(project_id);
					*self = CleanerState::FutureDepositRelease(remaining_participants, PhantomData::<Failure>);
					Ok(consumed_weight)
				},

			CleanerState::EvaluationUnbonding(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::BidFundingRelease(
						remaining_bids_to_release_funds::<T>(project_id),
						PhantomData::<Failure>,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_evaluators) = unbond_one_evaluation::<T>(project_id);
					*self = CleanerState::EvaluationUnbonding(remaining_evaluators, PhantomData);
					Ok(consumed_weight)
				},

			CleanerState::BidFundingRelease(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::BidUnbonding(remaining_bids::<T>(project_id), PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_bids) = release_funds_one_bid::<T>(project_id);
					*self = CleanerState::BidFundingRelease(remaining_bids, PhantomData);
					Ok(consumed_weight)
				},

			CleanerState::BidUnbonding(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::ContributionFundingRelease(
						remaining_contributions_to_release_funds::<T>(project_id),
						PhantomData::<Failure>,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_bids) = unbond_one_bid::<T>(project_id);
					*self = CleanerState::BidUnbonding(remaining_bids, PhantomData::<Failure>);
					Ok(consumed_weight)
				},

			CleanerState::ContributionFundingRelease(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::ContributionUnbonding(
						remaining_contributions::<T>(project_id),
						PhantomData::<Failure>,
					);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_contributions) = release_funds_one_contribution::<T>(project_id);
					*self = CleanerState::ContributionFundingRelease(remaining_contributions, PhantomData::<Failure>);
					Ok(consumed_weight)
				},

			CleanerState::ContributionUnbonding(PhantomData::<Failure>) =>
				if *remaining == 0 {
					*self = CleanerState::Finished(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let (consumed_weight, remaining_contributions) = unbond_one_contribution::<T>(project_id);
					*self = CleanerState::ContributionUnbonding(remaining_contributions, PhantomData::<Failure>);
					Ok(consumed_weight)
				},

			CleanerState::Finished(PhantomData::<Failure>) => Err(Error::<T>::FinalizerFinished.into()),

			_ => Err(Error::<T>::ImpossibleState.into()),
		}
	}
}

fn remaining_evaluations_to_reward_or_slash<T: Config>(
	project_id: ProjectId,
	outcome: EvaluatorsOutcomeOf<T>,
) -> Vec<EvaluationInfoOf<T>> {
	if outcome == EvaluatorsOutcomeOf::<T>::Unchanged {
		vec![]
	} else {
		let output = Evaluations::<T>::iter_prefix_values((project_id,))
			.filter(|evaluation| evaluation.rewarded_or_slashed.is_none())
			.collect_vec();
		output
	}
}

fn remaining_evaluations<T: Config>(project_id: ProjectId) -> Vec<EvaluationInfoOf<T>> {
	let output = Evaluations::<T>::iter_prefix_values((project_id,)).collect_vec();
	output
}

fn remaining_bids_to_release_funds<T: Config>(project_id: ProjectId) -> Vec<BidInfoOf<T>> {
	let output = Bids::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released).collect_vec();
	output
}

fn remaining_bids<T: Config>(project_id: ProjectId) -> Vec<BidInfoOf<T>> {
	let output = Bids::<T>::iter_prefix_values((project_id,)).collect_vec();
	output
}

fn remaining_successful_bids<T: Config>(project_id: ProjectId) -> Vec<BidInfoOf<T>> {
	let output = Bids::<T>::iter_prefix_values((project_id,))
		.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
		.collect_vec();
	output
}

fn remaining_contributions_to_release_funds<T: Config>(project_id: ProjectId) -> Vec<ContributionInfoOf<T>> {
	let output = Contributions::<T>::iter_prefix_values((project_id,))
		.filter(|contribution| !contribution.funds_released)
		.collect_vec();
	output
}

fn remaining_contributions<T: Config>(project_id: ProjectId) -> Vec<ContributionInfoOf<T>> {
	let output = Contributions::<T>::iter_prefix_values((project_id,)).collect_vec();
	output
}

fn remaining_bids_without_ct_minted<T: Config>(project_id: ProjectId) -> Vec<BidInfoOf<T>> {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	let output = project_bids.filter(|bid| !bid.ct_minted).collect_vec();
	output
}

fn remaining_contributions_without_ct_minted<T: Config>(project_id: ProjectId) -> Vec<ContributionInfoOf<T>> {
	let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	let output = project_contributions.filter(|contribution| !contribution.ct_minted).collect_vec();
	output
}

fn remaining_bids_without_issuer_payout<T: Config>(project_id: ProjectId) -> Vec<BidInfoOf<T>> {
	let output = Bids::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released).collect_vec();
	output
}

fn remaining_contributions_without_issuer_payout<T: Config>(project_id: ProjectId) -> Vec<ContributionInfoOf<T>> {
	let output = Contributions::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released).collect_vec();
	output
}

fn remaining_participants_with_future_ct_deposit<T: Config>(project_id: ProjectId) -> Vec<AccountIdOf<T>> {
	let evaluators = Evaluations::<T>::iter_key_prefix((project_id,)).map(|(evaluator, _evaluation_id)| evaluator);
	let bidders = Bids::<T>::iter_key_prefix((project_id,)).map(|(bidder, _bid_id)| bidder);
	let contributors =
		Contributions::<T>::iter_key_prefix((project_id,)).map(|(contributor, _contribution_id)| contributor);
	let all_participants = evaluators.chain(bidders).chain(contributors).collect::<BTreeSet<AccountIdOf<T>>>();
	let output = all_participants
		.into_iter()
		.filter(|account| {
			<T as Config>::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), account) >
				Zero::zero()
		})
		.collect_vec();
	output
}

fn reward_or_slash_one_evaluation<T: Config>(project_id: ProjectId) -> Result<(Weight, u64), DispatchError> {
	let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
	let project_evaluations = Evaluations::<T>::iter_prefix_values((project_id,));
	let mut remaining_evaluations = project_evaluations.filter(|evaluation| evaluation.rewarded_or_slashed.is_none());
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(evaluation) = remaining_evaluations.next() {
		// TODO: This base weight and the one in all other functions below should be calculated with a benchmark
		let remaining = remaining_evaluations.count() as u64;

		match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(_) => {
				match Pallet::<T>::do_evaluation_reward_payout_for(
					&T::PalletId::get().into_account_truncating(),
					evaluation.project_id,
					&evaluation.evaluator,
					evaluation.id,
				) {
					Ok(_) => (),
					Err(e) => Pallet::<T>::deposit_event(Event::EvaluationRewardFailed {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator.clone(),
						id: evaluation.id,
						error: e,
					}),
				};

				Ok((base_weight.saturating_add(WeightInfoOf::<T>::evaluation_reward_payout_for()), remaining))
			},
			EvaluatorsOutcome::Slashed => {
				match Pallet::<T>::do_evaluation_slash_for(
					&T::PalletId::get().into_account_truncating(),
					evaluation.project_id,
					&evaluation.evaluator,
					evaluation.id,
				) {
					Ok(_) => (),
					Err(e) => Pallet::<T>::deposit_event(Event::EvaluationSlashFailed {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator.clone(),
						id: evaluation.id,
						error: e,
					}),
				};

				Ok((base_weight.saturating_add(WeightInfoOf::<T>::evaluation_slash_for()), remaining))
			},
			_ => {
				#[cfg(debug_assertions)]
				unreachable!("EvaluatorsOutcome should be either Slashed or Rewarded if this function is called");
				#[cfg(not(debug_assertions))]
				Err(Error::<T>::ImpossibleState.into())
			},
		}
	} else {
		Ok((base_weight, 0u64))
	}
}

fn unbond_one_evaluation<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_evaluations = Evaluations::<T>::iter_prefix_values((project_id,));
	let mut remaining_evaluations =
		project_evaluations.filter(|evaluation| evaluation.current_plmc_bond > Zero::zero());
	let base_weight = Weight::from_parts(10_000_000, 0);
	if let Some(evaluation) = remaining_evaluations.next() {
		match Pallet::<T>::do_evaluation_unbond_for(
			&T::PalletId::get().into_account_truncating(),
			evaluation.project_id,
			&evaluation.evaluator,
			evaluation.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::EvaluationUnbondFailed {
				project_id: evaluation.project_id,
				evaluator: evaluation.evaluator.clone(),
				id: evaluation.id,
				error: e,
			}),
		};
		(base_weight.saturating_add(WeightInfoOf::<T>::evaluation_unbond_for()), remaining_evaluations.count() as u64)
	} else {
		(base_weight, 0u64)
	}
}

fn release_funds_one_bid<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	let mut remaining_bids = project_bids.filter(|bid| !bid.funds_released);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(bid) = remaining_bids.next() {
		match Pallet::<T>::do_release_bid_funds_for(
			&T::PalletId::get().into_account_truncating(),
			bid.project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::ReleaseBidFundsFailed {
				project_id: bid.project_id,
				bidder: bid.bidder.clone(),
				id: bid.id,
				error: e,
			}),
		};

		(base_weight.saturating_add(WeightInfoOf::<T>::release_bid_funds_for()), remaining_bids.count() as u64)
	} else {
		(base_weight, 0u64)
	}
}

fn unbond_one_bid<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	let mut remaining_bids = project_bids.filter(|bid| bid.funds_released);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(bid) = remaining_bids.next() {
		match Pallet::<T>::do_bid_unbond_for(
			&T::PalletId::get().into_account_truncating(),
			bid.project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::BidUnbondFailed {
				project_id: bid.project_id,
				bidder: bid.bidder.clone(),
				id: bid.id,
				error: e,
			}),
		};
		(base_weight.saturating_add(WeightInfoOf::<T>::bid_unbond_for()), remaining_bids.count() as u64)
	} else {
		(base_weight, 0u64)
	}
}

fn release_future_ct_deposit_one_participant<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let base_weight = Weight::from_parts(10_000_000, 0);
	let evaluators = Evaluations::<T>::iter_key_prefix((project_id,)).map(|(evaluator, _evaluation_id)| evaluator);
	let bidders = Bids::<T>::iter_key_prefix((project_id,)).map(|(bidder, _bid_id)| bidder);
	let contributors =
		Contributions::<T>::iter_key_prefix((project_id,)).map(|(contributor, _contribution_id)| contributor);
	let all_participants = evaluators.chain(bidders).chain(contributors).collect::<BTreeSet<AccountIdOf<T>>>();
	let remaining_participants = all_participants
		.into_iter()
		.filter(|account| {
			<T as Config>::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), account) >
				Zero::zero()
		})
		.collect_vec();
	let mut iter_participants = remaining_participants.into_iter();

	if let Some(account) = iter_participants.next() {
		match Pallet::<T>::do_release_future_ct_deposit_for(
			&T::PalletId::get().into_account_truncating(),
			project_id,
			&account,
		) {
			// TODO: replace when benchmark is done
			// Ok(_) => return (base_weight.saturating_add(WeightInfoOf::<T>::release_future_ct_deposit_for()), iter_participants.collect_vec()),
			Ok(_) => return (base_weight, iter_participants.count() as u64),
			// TODO: use when storing remaining accounts in outer function calling do_one_operation https://linear.app/polimec/issue/PLMC-410/cleaner-remaining-users-calculation
			// Err(e) if e == Error::<T>::NoFutureDepositHeld.into() => continue,
			Err(e) => {
				Pallet::<T>::deposit_event(Event::ReleaseFutureCTDepositFailed {
					project_id,
					participant: account.clone(),
					error: e,
				});
				// TODO: replace when benchmark is done
				// return (base_weight.saturating_add(WeightInfoOf::<T>::release_future_ct_deposit_for()), iter_participants.collect_vec());
				return (base_weight, iter_participants.count() as u64)
			},
		};
	}
	return (base_weight, 0u64)
}

fn release_funds_one_contribution<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.funds_released);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(contribution) = remaining_contributions.next() {
		match Pallet::<T>::do_release_contribution_funds_for(
			&T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			&contribution.contributor,
			contribution.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::ReleaseContributionFundsFailed {
				project_id: contribution.project_id,
				contributor: contribution.contributor.clone(),
				id: contribution.id,
				error: e,
			}),
		};

		(
			base_weight.saturating_add(WeightInfoOf::<T>::release_contribution_funds_for()),
			remaining_contributions.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}

fn unbond_one_contribution<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));

	let mut remaining_contributions =
		project_contributions.into_iter().filter(|contribution| contribution.funds_released);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(contribution) = remaining_contributions.next() {
		match Pallet::<T>::do_contribution_unbond_for(
			&T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			&contribution.contributor,
			contribution.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::ContributionUnbondFailed {
				project_id: contribution.project_id,
				contributor: contribution.contributor.clone(),
				id: contribution.id,
				error: e,
			}),
		};
		(
			base_weight.saturating_add(WeightInfoOf::<T>::contribution_unbond_for()),
			remaining_contributions.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}

fn start_one_bid_vesting_schedule<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	let mut unscheduled_bids = project_bids.filter(|bid| {
		bid.plmc_vesting_info.is_none() && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..))
	});
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(bid) = unscheduled_bids.next() {
		match Pallet::<T>::do_start_bid_vesting_schedule_for(
			&T::PalletId::get().into_account_truncating(),
			project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => {},
			Err(e) => {
				// TODO: Handle `MAX_VESTING_SCHEDULES` error

				Pallet::<T>::deposit_event(Event::StartBidderVestingScheduleFailed {
					project_id: bid.project_id,
					bidder: bid.bidder.clone(),
					id: bid.id,
					error: e,
				});
			},
		}
		(
			base_weight.saturating_add(WeightInfoOf::<T>::start_bid_vesting_schedule_for()),
			unscheduled_bids.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}

fn start_one_contribution_vesting_schedule<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Contributions::<T>::iter_prefix_values((project_id,));
	let mut unscheduled_contributions = project_bids.filter(|contribution| contribution.plmc_vesting_info.is_none());
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(contribution) = unscheduled_contributions.next() {
		match Pallet::<T>::do_start_contribution_vesting_schedule_for(
			&T::PalletId::get().into_account_truncating(),
			project_id,
			&contribution.contributor,
			contribution.id,
		) {
			Ok(_) => {},
			Err(e) => {
				Pallet::<T>::deposit_event(Event::StartContributionVestingScheduleFailed {
					project_id: contribution.project_id,
					contributor: contribution.contributor.clone(),
					id: contribution.id,
					error: e,
				});
			},
		}
		(
			base_weight.saturating_add(WeightInfoOf::<T>::start_contribution_vesting_schedule_for()),
			unscheduled_contributions.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}

fn mint_ct_for_one_bid<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	let mut remaining_bids = project_bids
		.filter(|bid| !bid.ct_minted && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(bid) = remaining_bids.next() {
		match Pallet::<T>::do_bid_ct_mint_for(
			&T::PalletId::get().into_account_truncating(),
			bid.project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::CTMintFailed {
				project_id: bid.project_id,
				claimer: bid.bidder.clone(),
				id: bid.id,
				error: e,
			}),
		};
		(base_weight.saturating_add(WeightInfoOf::<T>::bid_ct_mint_for()), remaining_bids.count() as u64)
	} else {
		(base_weight, 0u64)
	}
}

fn mint_ct_for_one_contribution<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.ct_minted);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(contribution) = remaining_contributions.next() {
		match Pallet::<T>::do_contribution_ct_mint_for(
			&T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			&contribution.contributor,
			contribution.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::CTMintFailed {
				project_id: contribution.project_id,
				claimer: contribution.contributor.clone(),
				id: contribution.id,
				error: e,
			}),
		};
		(
			base_weight.saturating_add(WeightInfoOf::<T>::contribution_ct_mint_for()),
			remaining_contributions.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}

fn issuer_funding_payout_one_bid<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_bids = Bids::<T>::iter_prefix_values((project_id,));

	let mut remaining_bids = project_bids.filter(|bid| {
		!bid.funds_released && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..))
	});
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(bid) = remaining_bids.next() {
		match Pallet::<T>::do_payout_bid_funds_for(
			&T::PalletId::get().into_account_truncating(),
			bid.project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::PayoutContributionFundsFailed {
				project_id: bid.project_id,
				contributor: bid.bidder.clone(),
				id: bid.id,
				error: e,
			}),
		};
		(base_weight.saturating_add(WeightInfoOf::<T>::payout_bid_funds_for()), remaining_bids.count() as u64)
	} else {
		(base_weight, 0u64)
	}
}

fn issuer_funding_payout_one_contribution<T: Config>(project_id: ProjectId) -> (Weight, u64) {
	let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));

	let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.funds_released);
	let base_weight = Weight::from_parts(10_000_000, 0);

	if let Some(contribution) = remaining_contributions.next() {
		match Pallet::<T>::do_payout_contribution_funds_for(
			&T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			&contribution.contributor,
			contribution.id,
		) {
			Ok(_) => (),
			Err(e) => Pallet::<T>::deposit_event(Event::PayoutContributionFundsFailed {
				project_id: contribution.project_id,
				contributor: contribution.contributor.clone(),
				id: contribution.id,
				error: e,
			}),
		};

		(
			base_weight.saturating_add(WeightInfoOf::<T>::payout_contribution_funds_for()),
			remaining_contributions.count() as u64,
		)
	} else {
		(base_weight, 0u64)
	}
}
