use crate::{traits::DoRemainingOperation, *};
use frame_support::{traits::Get, weights::Weight};
use sp_runtime::{traits::AccountIdConversion, DispatchError};
use sp_std::prelude::*;

impl DoRemainingOperation for ProjectFinalizer {
	fn is_done(&self) -> bool {
		matches!(self, ProjectFinalizer::None)
	}

	fn do_one_operation<T: crate::Config>(
		&mut self,
		project_id: T::ProjectIdentifier,
	) -> Result<Weight, DispatchError> {
		match self {
			ProjectFinalizer::None => Err(Error::<T>::NoFinalizerSet.into()),
			ProjectFinalizer::Success(ops) => {
				let weight = ops.do_one_operation::<T>(project_id)?;
				if ops.is_done() {
					*self = ProjectFinalizer::None;
				}
				Ok(weight)
			},
			ProjectFinalizer::Failure(ops) => {
				let weight = ops.do_one_operation::<T>(project_id)?;
				if ops.is_done() {
					*self = ProjectFinalizer::None;
				}
				Ok(weight)
			},
		}
	}
}

impl DoRemainingOperation for SuccessFinalizer {
	fn is_done(&self) -> bool {
		matches!(self, SuccessFinalizer::Finished)
	}

	fn do_one_operation<T: Config>(&mut self, project_id: T::ProjectIdentifier) -> Result<Weight, DispatchError> {
		match self {
			SuccessFinalizer::Initialized => {
				*self =
					SuccessFinalizer::EvaluationRewardOrSlash(remaining_evaluators_to_reward_or_slash::<T>(project_id));
				Ok(Weight::zero())
			},
			SuccessFinalizer::EvaluationRewardOrSlash(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::EvaluationUnbonding(remaining_evaluations::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_evaluations) = reward_or_slash_one_evaluation::<T>(project_id)?;
					*self = SuccessFinalizer::EvaluationRewardOrSlash(remaining_evaluations);
					Ok(consumed_weight)
				},
			SuccessFinalizer::EvaluationUnbonding(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::BidPLMCVesting(remaining_bids_without_plmc_vesting::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_evaluations) = unbond_one_evaluation::<T>(project_id);
					*self = SuccessFinalizer::EvaluationUnbonding(remaining_evaluations);
					Ok(consumed_weight)
				},
			SuccessFinalizer::BidPLMCVesting(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::BidCTMint(remaining_bids_without_ct_minted::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_bids) = start_bid_plmc_vesting_schedule::<T>(project_id);
					*self = SuccessFinalizer::BidPLMCVesting(remaining_bids);
					Ok(consumed_weight)
				},
			SuccessFinalizer::BidCTMint(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::ContributionPLMCVesting(
						remaining_contributions_without_plmc_vesting::<T>(project_id),
					);
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_bids) = mint_ct_for_one_bid::<T>(project_id);
					*self = SuccessFinalizer::BidCTMint(remaining_bids);
					Ok(consumed_weight)
				},
			SuccessFinalizer::ContributionPLMCVesting(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::ContributionCTMint(remaining_contributions_without_ct_minted::<T>(
						project_id,
					));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) =
						start_contribution_plmc_vesting_schedule::<T>(project_id);
					*self = SuccessFinalizer::ContributionPLMCVesting(remaining_contributions);
					Ok(consumed_weight)
				},
			SuccessFinalizer::ContributionCTMint(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::BidFundingPayout(remaining_bids_without_issuer_payout::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) = mint_ct_for_one_contribution::<T>(project_id);
					*self = SuccessFinalizer::ContributionCTMint(remaining_contributions);
					Ok(consumed_weight)
				},
			SuccessFinalizer::BidFundingPayout(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::ContributionFundingPayout(
						remaining_contributions_without_issuer_payout::<T>(project_id),
					);
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) = issuer_funding_payout_one_bid::<T>(project_id);
					*self = SuccessFinalizer::BidFundingPayout(remaining_contributions);
					Ok(consumed_weight)
				},
			SuccessFinalizer::ContributionFundingPayout(remaining) =>
				if *remaining == 0 {
					*self = SuccessFinalizer::Finished;
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) =
						issuer_funding_payout_one_contribution::<T>(project_id);
					*self = SuccessFinalizer::ContributionFundingPayout(remaining_contributions);
					Ok(consumed_weight)
				},
			SuccessFinalizer::Finished => Err(Error::<T>::FinalizerFinished.into()),
		}
	}
}

impl DoRemainingOperation for FailureFinalizer {
	fn is_done(&self) -> bool {
		matches!(self, FailureFinalizer::Finished)
	}

	fn do_one_operation<T: Config>(&mut self, project_id: T::ProjectIdentifier) -> Result<Weight, DispatchError> {
		match self {
			FailureFinalizer::Initialized => {
				*self =
					FailureFinalizer::EvaluationRewardOrSlash(remaining_evaluators_to_reward_or_slash::<T>(project_id));
				Ok(Weight::zero())
			},

			FailureFinalizer::EvaluationRewardOrSlash(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::EvaluationUnbonding(remaining_evaluations::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_evaluators) = reward_or_slash_one_evaluation::<T>(project_id)?;
					*self = FailureFinalizer::EvaluationRewardOrSlash(remaining_evaluators);
					Ok(consumed_weight)
				},

			FailureFinalizer::EvaluationUnbonding(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::BidFundingRelease(remaining_bids_to_release_funds::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_evaluators) = unbond_one_evaluation::<T>(project_id);
					*self = FailureFinalizer::EvaluationUnbonding(remaining_evaluators);
					Ok(consumed_weight)
				},

			FailureFinalizer::BidFundingRelease(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::BidUnbonding(remaining_bids::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_bids) = release_funds_one_bid::<T>(project_id);
					*self = FailureFinalizer::BidFundingRelease(remaining_bids);
					Ok(consumed_weight)
				},

			FailureFinalizer::BidUnbonding(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::ContributionFundingRelease(
						remaining_contributions_to_release_funds::<T>(project_id),
					);
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_bids) = unbond_one_bid::<T>(project_id);
					*self = FailureFinalizer::BidUnbonding(remaining_bids);
					Ok(consumed_weight)
				},

			FailureFinalizer::ContributionFundingRelease(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::ContributionUnbonding(remaining_contributions::<T>(project_id));
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) = release_funds_one_contribution::<T>(project_id);
					*self = FailureFinalizer::ContributionFundingRelease(remaining_contributions);
					Ok(consumed_weight)
				},

			FailureFinalizer::ContributionUnbonding(remaining) =>
				if *remaining == 0 {
					*self = FailureFinalizer::Finished;
					Ok(Weight::zero())
				} else {
					let (consumed_weight, remaining_contributions) = unbond_one_contribution::<T>(project_id);
					*self = FailureFinalizer::ContributionUnbonding(remaining_contributions);
					Ok(consumed_weight)
				},

			FailureFinalizer::Finished => Err(Error::<T>::FinalizerFinished.into()),
		}
	}
}

enum OperationsLeft {
	Some(u64),
	None,
}

fn remaining_evaluators_to_reward_or_slash<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Evaluations::<T>::iter_prefix_values(project_id)
		.flatten()
		.filter(|evaluation| !evaluation.rewarded_or_slashed)
		.count() as u64
}

fn remaining_evaluations<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Evaluations::<T>::iter_prefix_values(project_id).flatten().count() as u64
}

fn remaining_bids_to_release_funds<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Bids::<T>::iter_prefix_values(project_id).flatten().filter(|bid| !bid.funds_released).count() as u64
}

fn remaining_bids<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Bids::<T>::iter_prefix_values(project_id).flatten().count() as u64
}

fn remaining_contributions_to_release_funds<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Contributions::<T>::iter_prefix_values(project_id)
		.flatten()
		.filter(|contribution| !contribution.funds_released)
		.count() as u64
}

fn remaining_contributions<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Contributions::<T>::iter_prefix_values(project_id).flatten().count() as u64
}

fn remaining_bids_without_plmc_vesting<T: Config>(_project_id: T::ProjectIdentifier) -> u64 {
	// TODO: current vesting implementation starts the schedule on bid creation. We should later on use pallet_vesting
	// 	and add a check in the bid struct for initializing the vesting schedule
	0u64
}

fn remaining_bids_without_ct_minted<T: Config>(_project_id: T::ProjectIdentifier) -> u64 {
	// TODO: currently we vest the contribution tokens. We should change this to a direct mint.
	0u64
}

fn remaining_contributions_without_plmc_vesting<T: Config>(_project_id: T::ProjectIdentifier) -> u64 {
	// TODO: current vesting implementation starts the schedule on contribution creation. We should later on use pallet_vesting
	// 	and add a check in the contribution struct for initializing the vesting schedule
	0u64
}

fn remaining_contributions_without_ct_minted<T: Config>(_project_id: T::ProjectIdentifier) -> u64 {
	// TODO: currently we vest the contribution tokens. We should change this to a direct mint.
	0u64
}

fn remaining_bids_without_issuer_payout<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Bids::<T>::iter_prefix_values(project_id).flatten().filter(|bid| !bid.funds_released).count() as u64
}

fn remaining_contributions_without_issuer_payout<T: Config>(project_id: T::ProjectIdentifier) -> u64 {
	Contributions::<T>::iter_prefix_values(project_id).flatten().filter(|bid| !bid.funds_released).count() as u64
}

fn reward_or_slash_one_evaluation<T: Config>(project_id: T::ProjectIdentifier) -> Result<(Weight, u64), DispatchError> {
	let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
	let project_evaluations: Vec<_> = Evaluations::<T>::iter_prefix_values(project_id).collect();
	let remaining_evaluations =
		project_evaluations.iter().flatten().filter(|evaluation| !evaluation.rewarded_or_slashed).count() as u64;

	let maybe_user_evaluations = project_evaluations
		.into_iter()
		.find(|evaluations| evaluations.iter().any(|evaluation| !evaluation.rewarded_or_slashed));

	if let Some(mut user_evaluations) = maybe_user_evaluations {
		let mut evaluation = user_evaluations
			.iter_mut()
			.find(|evaluation| !evaluation.rewarded_or_slashed)
			.expect("user_evaluations can only exist if an item here is found; qed");

		match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(_) => {
				match Pallet::<T>::do_evaluation_reward(
					T::PalletId::get().into_account_truncating(),
					evaluation.project_id,
					evaluation.evaluator.clone(),
					evaluation.id,
				) {
					Ok(_) => (),
					Err(e) => Pallet::<T>::deposit_event(Event::EvaluationRewardOrSlashFailed {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator.clone(),
						id: evaluation.id,
						error: e,
					}),
				};
			},
			_ => (),
		}

		// if the evaluation outcome failed, we still want to flag it as rewarded or slashed. Otherwise the automatic
		// transition will get stuck.
		evaluation.rewarded_or_slashed = true;
		Evaluations::<T>::insert(project_id, evaluation.evaluator.clone(), user_evaluations);

		Ok((Weight::zero(), remaining_evaluations.saturating_sub(1u64)))
	} else {
		Ok((Weight::zero(), 0u64))
	}
}

fn unbond_one_evaluation<T: crate::Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_evaluations: Vec<_> = Evaluations::<T>::iter_prefix_values(project_id).collect();
	let evaluation_count = project_evaluations.iter().flatten().count() as u64;

	let maybe_user_evaluations =
		project_evaluations.into_iter().find(|evaluations| evaluations.iter().any(|e| e.rewarded_or_slashed));

	if let Some(mut user_evaluations) = maybe_user_evaluations {
		let evaluation = user_evaluations
			.iter_mut()
			.find(|evaluation| evaluation.rewarded_or_slashed)
			.expect("user_evaluations can only exist if an item here is found; qed");

		match Pallet::<T>::do_evaluation_unbond_for(
			T::PalletId::get().into_account_truncating(),
			evaluation.project_id,
			evaluation.evaluator.clone(),
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
		(Weight::zero(), evaluation_count.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn release_funds_one_bid<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_bids: Vec<_> = Bids::<T>::iter_prefix_values(project_id).collect();
	let remaining_bids = project_bids.iter().flatten().filter(|bid| !bid.funds_released).count() as u64;
	let maybe_user_bids = project_bids.into_iter().find(|bids| bids.iter().any(|bid| !bid.funds_released));

	if let Some(mut user_bids) = maybe_user_bids {
		let mut bid = user_bids
			.iter_mut()
			.find(|bid| !bid.funds_released)
			.expect("user_bids can only exist if an item here is found; qed");

		match Pallet::<T>::do_release_bid_funds_for(
			T::PalletId::get().into_account_truncating(),
			bid.project_id,
			bid.bidder.clone(),
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

		bid.funds_released = true;

		Bids::<T>::insert(project_id, bid.bidder.clone(), user_bids);

		(Weight::zero(), remaining_bids.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn unbond_one_bid<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_bids: Vec<_> = Bids::<T>::iter_prefix_values(project_id).collect();
	// let bids_count = project_bids.iter().flatten().count() as u64;
	// remove when do_bid_unbond_for is correctly implemented
	let bids_count = 0u64;

	let maybe_user_bids = project_bids.into_iter().find(|bids| bids.iter().any(|e| e.funds_released));

	if let Some(mut user_bids) = maybe_user_bids {
		let bid = user_bids
			.iter_mut()
			.find(|bid| bid.funds_released)
			.expect("user_evaluations can only exist if an item here is found; qed");

		match Pallet::<T>::do_bid_unbond_for(
			T::PalletId::get().into_account_truncating(),
			bid.project_id,
			bid.bidder.clone(),
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
		(Weight::zero(), bids_count.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn release_funds_one_contribution<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_contributions: Vec<_> = Contributions::<T>::iter_prefix_values(project_id).collect();
	// let remaining_contributions = project_contributions
	// 	.iter()
	// 	.flatten()
	// 	.filter(|contribution| !contribution.funds_released)
	// 	.count() as u64;
	// remove when do_release_contribution_funds_for is correctly implemented
	let remaining_contributions = 0u64;
	let maybe_user_contributions = project_contributions
		.into_iter()
		.find(|contributions| contributions.iter().any(|contribution| !contribution.funds_released));

	if let Some(mut user_contributions) = maybe_user_contributions {
		let mut contribution = user_contributions
			.iter_mut()
			.find(|contribution| !contribution.funds_released)
			.expect("user_contributions can only exist if an item here is found; qed");

		match Pallet::<T>::do_release_contribution_funds_for(
			T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			contribution.contributor.clone(),
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

		contribution.funds_released = true;

		Contributions::<T>::insert(project_id, contribution.contributor.clone(), user_contributions);

		(Weight::zero(), remaining_contributions.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn unbond_one_contribution<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_contributions: Vec<_> = Contributions::<T>::iter_prefix_values(project_id).collect();

	// let contributions_count = project_contributions.iter().flatten().count() as u64;
	let contributions_count = 0u64;

	let maybe_user_contributions =
		project_contributions.into_iter().find(|contributions| contributions.iter().any(|e| e.funds_released));

	if let Some(mut user_contributions) = maybe_user_contributions {
		let contribution = user_contributions
			.iter_mut()
			.find(|contribution| contribution.funds_released)
			.expect("user_evaluations can only exist if an item here is found; qed");

		match Pallet::<T>::do_contribution_unbond_for(
			T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			contribution.contributor.clone(),
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
		(Weight::zero(), contributions_count.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn start_bid_plmc_vesting_schedule<T: Config>(_project_id: T::ProjectIdentifier) -> (Weight, u64) {
	// TODO: change when new vesting schedule is implemented
	(Weight::zero(), 0u64)
}

fn start_contribution_plmc_vesting_schedule<T: Config>(_project_id: T::ProjectIdentifier) -> (Weight, u64) {
	// TODO: change when new vesting schedule is implemented
	(Weight::zero(), 0u64)
}

fn mint_ct_for_one_bid<T: Config>(_project_id: T::ProjectIdentifier) -> (Weight, u64) {
	// TODO: Change when new vesting schedule is implemented
	(Weight::zero(), 0u64)
}

fn mint_ct_for_one_contribution<T: Config>(_project_id: T::ProjectIdentifier) -> (Weight, u64) {
	// TODO: Change when new vesting schedule is implemented
	(Weight::zero(), 0u64)
}

fn issuer_funding_payout_one_bid<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_bids: Vec<_> = Bids::<T>::iter_prefix_values(project_id).collect();

	// let remaining_bids = project_bids
	// 	.iter()
	// 	.flatten()
	// 	.filter(|bid| !bid.funds_released)
	// 	.count() as u64;
	let remaining_bids = 0u64;

	let maybe_user_bids = project_bids.into_iter().find(|bids| bids.iter().any(|bid| !bid.funds_released));

	if let Some(mut user_bids) = maybe_user_bids {
		let mut bid = user_bids
			.iter_mut()
			.find(|bid| !bid.funds_released)
			.expect("user_bids can only exist if an item here is found; qed");

		match Pallet::<T>::do_payout_bid_funds_for(
			T::PalletId::get().into_account_truncating(),
			bid.project_id,
			bid.bidder.clone(),
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

		bid.funds_released = true;

		Bids::<T>::insert(project_id, bid.bidder.clone(), user_bids);

		(Weight::zero(), remaining_bids.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

fn issuer_funding_payout_one_contribution<T: Config>(project_id: T::ProjectIdentifier) -> (Weight, u64) {
	let project_contributions: Vec<_> = Contributions::<T>::iter_prefix_values(project_id).collect();

	// let remaining_contributions = project_contributions
	// 	.iter()
	// 	.flatten()
	// 	.filter(|contribution| !contribution.funds_released)
	// 	.count() as u64;
	let remaining_contributions = 0u64;

	let maybe_user_contributions = project_contributions
		.into_iter()
		.find(|contributions| contributions.iter().any(|contribution| !contribution.funds_released));

	if let Some(mut user_contributions) = maybe_user_contributions {
		let mut contribution = user_contributions
			.iter_mut()
			.find(|contribution| !contribution.funds_released)
			.expect("user_contributions can only exist if an item here is found; qed");

		match Pallet::<T>::do_payout_contribution_funds_for(
			T::PalletId::get().into_account_truncating(),
			contribution.project_id,
			contribution.contributor.clone(),
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

		contribution.funds_released = true;

		Contributions::<T>::insert(project_id, contribution.contributor.clone(), user_contributions);

		(Weight::zero(), remaining_contributions.saturating_sub(1u64))
	} else {
		(Weight::zero(), 0u64)
	}
}

// might come in handy later
#[allow(unused)]
fn unbond_evaluators<T: Config>(project_id: T::ProjectIdentifier, max_weight: Weight) -> (Weight, OperationsLeft) {
	let evaluations = Evaluations::<T>::iter_prefix_values(project_id).flatten().collect::<Vec<EvaluationInfoOf<T>>>();

	let mut used_weight = Weight::zero();

	let unbond_results = evaluations
		.iter()
		.take_while(|_evaluation| {
			let new_used_weight = used_weight.saturating_add(T::WeightInfo::evaluation_unbond_for());
			if new_used_weight.any_gt(max_weight) {
				false
			} else {
				used_weight = new_used_weight;
				true
			}
		})
		.map(|evaluation| {
			Pallet::<T>::do_evaluation_unbond_for(
				T::PalletId::get().into_account_truncating(),
				evaluation.project_id,
				evaluation.evaluator.clone(),
				evaluation.id,
			)
		})
		.collect::<Vec<_>>();

	let successful_results =
		unbond_results.into_iter().filter(|result| if let Err(e) = result { false } else { true }).collect::<Vec<_>>();

	let operations_left = if successful_results.len() == evaluations.len() {
		OperationsLeft::None
	} else {
		OperationsLeft::Some(evaluations.len().saturating_sub(successful_results.len()) as u64)
	};

	(used_weight, operations_left)
}
