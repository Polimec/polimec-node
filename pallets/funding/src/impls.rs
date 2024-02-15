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

use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, GetDispatchInfo},
	traits::{fungible::InspectHold, Get},
	weights::Weight,
};
use itertools::Itertools;
use sp_arithmetic::traits::Zero;
use sp_runtime::{traits::AccountIdConversion, DispatchError};
use sp_std::{collections::btree_set::BTreeSet, marker::PhantomData};

use crate::{
	traits::{SettlementOperations, SettlementTarget},
	*,
};

impl<T: Config> SettlementOperations<T> for SettlementMachine {
	fn has_remaining_operations(&self) -> bool {
		match self {
			SettlementMachine::NotReady => false,
			SettlementMachine::Success(state) =>
				<SettlementType<Success> as SettlementOperations<T>>::has_remaining_operations(state),
			SettlementMachine::Failure(state) =>
				<SettlementType<Failure> as SettlementOperations<T>>::has_remaining_operations(state),
		}
	}

	fn do_one_operation(
		&mut self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		match self {
			SettlementMachine::NotReady => Err((Weight::zero(), "SettlementMachine not ready for operations".into())),
			SettlementMachine::Success(state) =>
				<SettlementType<Success> as SettlementOperations<T>>::do_one_operation(state, project_id, target),
			SettlementMachine::Failure(state) =>
				<SettlementType<Failure> as SettlementOperations<T>>::do_one_operation(state, project_id, target),
		}
	}

	fn update_target(
		&self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		match self {
			SettlementMachine::NotReady => Err((Weight::zero(), "SettlementMachine not ready for operations".into())),
			SettlementMachine::Success(state) =>
				<SettlementType<Success> as SettlementOperations<T>>::update_target(state, project_id, target),
			SettlementMachine::Failure(state) =>
				<SettlementType<Failure> as SettlementOperations<T>>::update_target(state, project_id, target),
		}
	}

	fn execute_with_given_weight(
		&mut self,
		weight: Weight,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		match self {
			SettlementMachine::NotReady => Err((Weight::zero(), "SettlementMachine not ready for operations".into())),
			SettlementMachine::Success(state) =>
				<SettlementType<Success> as SettlementOperations<T>>::execute_with_given_weight(
					state, weight, project_id, target,
				),
			SettlementMachine::Failure(state) =>
				<SettlementType<Failure> as SettlementOperations<T>>::execute_with_given_weight(
					state, weight, project_id, target,
				),
		}
	}
}

impl<T: Config> SettlementOperations<T> for SettlementType<Success> {
	fn has_remaining_operations(&self) -> bool {
		!matches!(self, SettlementType::Finished(_))
	}

	fn do_one_operation(
		&mut self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		match self {
			SettlementType::Initialized(PhantomData::<Success>) => {
				*self = Self::EvaluationRewardOrSlash(PhantomData);
				Ok(Weight::zero())
			},
			SettlementType::EvaluationRewardOrSlash(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = Self::EvaluationUnbonding(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = reward_or_slash_one_evaluation::<T>(project_id, target)?;
					Ok(consumed_weight)
				},
			SettlementType::EvaluationUnbonding(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::StartBidderVestingSchedule(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = unbond_one_evaluation::<T>(project_id, target);

					Ok(consumed_weight)
				},
			SettlementType::StartBidderVestingSchedule(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::StartContributorVestingSchedule(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = start_one_bid_vesting_schedule::<T>(project_id, target);

					Ok(consumed_weight)
				},
			SettlementType::StartContributorVestingSchedule(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::BidCTMint(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = start_one_contribution_vesting_schedule::<T>(project_id, target);
					Ok(consumed_weight)
				},
			SettlementType::BidCTMint(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionCTMint(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = mint_ct_for_one_bid::<T>(project_id, target);
					Ok(consumed_weight)
				},
			SettlementType::ContributionCTMint(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::BidFundingPayout(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = mint_ct_for_one_contribution::<T>(project_id, target);

					Ok(consumed_weight)
				},
			SettlementType::BidFundingPayout(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionFundingPayout(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = issuer_funding_payout_one_bid::<T>(project_id, target);

					Ok(consumed_weight)
				},
			SettlementType::ContributionFundingPayout(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::Finished(PhantomData);
					let consumed_weight = self.update_target(project_id, target)?;
					Ok(consumed_weight)
				} else {
					let consumed_weight = issuer_funding_payout_one_contribution::<T>(project_id, target);
					Ok(consumed_weight)
				},
			SettlementType::Finished(PhantomData::<Success>) => Err(Error::<T>::FinalizerFinished.into()),

			// Transitions enforced by the type system
			_ => Err(Error::<T>::ImpossibleState.into()),
		}
	}

	fn update_target(
		&self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		match self {
			SettlementType::Initialized(_) => Weight::zero(),
			SettlementType::EvaluationRewardOrSlash(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::EvaluationUnbonding(_) => {
				*target = remaining_evaluations::<T>(project_id);
				Weight::zero()
			},
			SettlementType::BidCTMint(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::ContributionCTMint(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::StartBidderVestingSchedule(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::StartContributorVestingSchedule(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::BidFundingPayout(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::ContributionFundingPayout(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::BidFundingRelease(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::BidUnbonding(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::ContributionFundingRelease(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::ContributionUnbonding(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::FutureDepositRelease(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
			SettlementType::Finished(_) => {
				*target = remaining_evaluators_to_reward_or_slash::<T>(project_id);
				Weight::zero()
			},
		}
	}

	fn execute_with_given_weight(
		&mut self,
		weight: Weight,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		let mut remaining_weight = weight;
		let mut used_weight = Weight::zero();
		let has_remaining_operations =
			<SettlementType<Success> as SettlementOperations<T>>::has_remaining_operations(self);

		while has_remaining_operations && remaining_weight.all_gt(Weight::zero()) {
			match self.do_one_operation(project_id, target) {
				Ok(weight) => {
					used_weight = used_weight.saturating_add(weight);
				},
				Err((weight, err)) => {
					used_weight = used_weight.saturating_add(weight);
					return Err((used_weight, err));
				},
			}
		}

		Ok(used_weight)
	}
}
impl<T: Config> SettlementOperations<T> for SettlementType<Failure> {
	fn has_remaining_operations(&self) -> bool {
		!matches!(self, SettlementType::Finished(PhantomData::<Failure>))
	}

	fn do_one_operation(
		&mut self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		let base_weight = Weight::from_parts(10_000_000, 0);
		match self {
			SettlementType::Initialized(PhantomData::<Failure>) => {
				*self = SettlementType::EvaluationRewardOrSlash(PhantomData::<Failure>);
				Ok(Weight::zero())
			},

			SettlementType::EvaluationRewardOrSlash(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::FutureDepositRelease(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = reward_or_slash_one_evaluation::<T>(project_id, target)
						.map_err(|error_info| error_info.error)?;
					*self = SettlementType::EvaluationRewardOrSlash(PhantomData);
					Ok(consumed_weight)
				},
			SettlementType::FutureDepositRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::EvaluationUnbonding(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = release_future_ct_deposit_one_participant::<T>(project_id, target);
					*self = SettlementType::FutureDepositRelease(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::EvaluationUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::BidFundingRelease(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = unbond_one_evaluation::<T>(project_id, target);
					*self = SettlementType::EvaluationUnbonding(PhantomData);
					Ok(consumed_weight)
				},
			SettlementType::BidFundingRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::BidUnbonding(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = release_funds_one_bid::<T>(project_id, target);
					*self = SettlementType::BidFundingRelease(PhantomData);
					Ok(consumed_weight)
				},
			SettlementType::BidUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionFundingRelease(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = unbond_one_bid::<T>(project_id, target);
					*self = SettlementType::BidUnbonding(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::ContributionFundingRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionUnbonding(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = release_funds_one_contribution::<T>(project_id, target);
					*self = SettlementType::ContributionFundingRelease(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::ContributionUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::Finished(PhantomData::<Failure>);
					Ok(base_weight)
				} else {
					let consumed_weight = unbond_one_contribution::<T>(project_id, target);
					*self = SettlementType::ContributionUnbonding(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::Finished(PhantomData::<Failure>) => Err(Error::<T>::FinalizerFinished.into()),

			_ => Err(Error::<T>::ImpossibleState.into()),
		}
	}

	fn update_target(
		&self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		todo!()
	}

	fn execute_with_given_weight(
		&mut self,
		weight: Weight,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		todo!()
	}
}

fn remaining_evaluators_to_reward_or_slash<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// let evaluators_outcome = ProjectsDetails::<T>::get(project_id)
	// 	.ok_or(Error::<T>::ImpossibleState)?
	// 	.evaluation_round_info
	// 	.evaluators_outcome;
	// if evaluators_outcome == EvaluatorsOutcomeOf::<T>::Unchanged {
	// 	SettlementTarget::Evaluations(vec![])
	// } else {
	// 	SettlementTarget::Evaluations(
	// 		Evaluations::<T>::iter_prefix_values((project_id,))
	// 			.filter(|evaluation| evaluation.rewarded_or_slashed.is_none())
	// 			.to_vec(),
	// 	)
	// }
	todo!()
}

fn remaining_evaluations<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Evaluations::<T>::iter_prefix_values((project_id,))
	todo!()
}

fn remaining_bids_to_release_funds<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Bids::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released)
	todo!()
}

fn remaining_bids<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Bids::<T>::iter_prefix_values((project_id,))
	todo!()
}

fn remaining_successful_bids<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Bids::<T>::iter_prefix_values((project_id,))
	// 	.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
	todo!()
}

fn remaining_contributions_to_release_funds<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Contributions::<T>::iter_prefix_values((project_id,)).filter(|contribution| !contribution.funds_released)
	todo!()
}

fn remaining_contributions<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Contributions::<T>::iter_prefix_values((project_id,))
	todo!()
}

fn remaining_bids_without_ct_minted<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	// project_bids.filter(|bid| !bid.ct_minted)
	todo!()
}

fn remaining_contributions_without_ct_minted<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	// project_contributions.filter(|contribution| !contribution.ct_minted)
	//
	todo!()
}

fn remaining_bids_without_issuer_payout<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Bids::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released)

	todo!()
}

fn remaining_contributions_without_issuer_payout<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// Contributions::<T>::iter_prefix_values((project_id,)).filter(|bid| !bid.funds_released)

	todo!()
}

fn remaining_participants_with_future_ct_deposit<T: Config>(project_id: ProjectId) -> SettlementTarget<T> {
	// let evaluators = Evaluations::<T>::iter_key_prefix((project_id,)).map(|(evaluator, _evaluation_id)| evaluator);
	// let bidders = Bids::<T>::iter_key_prefix((project_id,)).map(|(bidder, _bid_id)| bidder);
	// let contributors =
	// 	Contributions::<T>::iter_key_prefix((project_id,)).map(|(contributor, _contribution_id)| contributor);
	// let all_participants = evaluators.chain(bidders).chain(contributors).collect::<BTreeSet<AccountIdOf<T>>>();
	// all_participants.into_iter().filter(|account| {
	// 	<T as Config>::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), account) >
	// 		Zero::zero()
	// })

	todo!()
}

fn reward_or_slash_one_evaluation<T: Config>(
	project_id: ProjectId,
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	// let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
	// let mut remaining_evaluations: Vec<EvaluationInfoOf<T>> =
	// 	if let SettlementTarget::Evaluations(evaluations) = *target {
	// 		evaluations
	// 	} else {
	// 		return Err("Expected evaluations at settlement target".into());
	// 	};
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(evaluation) = remaining_evaluations.next() {
	// 	// TODO: This base weight and the one in all other functions below should be calculated with a benchmark
	// 	let remaining = remaining_evaluations.count() as u64;
	// 	match project_details.evaluation_round_info.evaluators_outcome {
	// 		EvaluatorsOutcome::Rewarded(_) => {
	// 			let mut weight_consumed = crate::Call::<T>::evaluation_reward_payout_for {
	// 				project_id: evaluation.project_id,
	// 				evaluator: evaluation.evaluator.clone(),
	// 				bond_id: evaluation.id,
	// 			}
	// 			.get_dispatch_info()
	// 			.weight;
	//
	// 			match Pallet::<T>::do_evaluation_reward_payout_for(
	// 				&T::PalletId::get().into_account_truncating(),
	// 				evaluation.project_id,
	// 				&evaluation.evaluator,
	// 				evaluation.id,
	// 			) {
	// 				Ok(result) => {
	// 					if let Some(weight) = result.actual_weight {
	// 						weight_consumed = weight
	// 					};
	// 				},
	// 				Err(e) => {
	// 					if let Some(weight) = e.post_info.actual_weight {
	// 						weight_consumed = weight
	// 					};
	// 					Pallet::<T>::deposit_event(Event::EvaluationRewardFailed {
	// 						project_id: evaluation.project_id,
	// 						evaluator: evaluation.evaluator.clone(),
	// 						id: evaluation.id,
	// 						error: e.error,
	// 					})
	// 				},
	// 			};
	//
	// 			Ok((base_weight.saturating_add(weight_consumed), remaining))
	// 		},
	// 		EvaluatorsOutcome::Slashed => {
	// 			match Pallet::<T>::do_evaluation_slash_for(
	// 				&T::PalletId::get().into_account_truncating(),
	// 				evaluation.project_id,
	// 				&evaluation.evaluator,
	// 				evaluation.id,
	// 			) {
	// 				Ok(_) => (),
	// 				Err(e) => Pallet::<T>::deposit_event(Event::EvaluationSlashFailed {
	// 					project_id: evaluation.project_id,
	// 					evaluator: evaluation.evaluator.clone(),
	// 					id: evaluation.id,
	// 					error: e,
	// 				}),
	// 			};
	//
	// 			Ok((base_weight.saturating_add(WeightInfoOf::<T>::evaluation_slash_for()), remaining))
	// 		},
	// 		_ => {
	// 			#[cfg(debug_assertions)]
	// 			unreachable!("EvaluatorsOutcome should be either Slashed or Rewarded if this function is called");
	// 			#[cfg(not(debug_assertions))]
	// 			Err(Error::<T>::ImpossibleState.into())
	// 		},
	// 	}
	// } else {
	// 	Ok((base_weight, 0u64))
	// }

	todo!();
}

fn unbond_one_evaluation<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_evaluations = Evaluations::<T>::iter_prefix_values((project_id,));
	// let mut remaining_evaluations =
	// 	project_evaluations.filter(|evaluation| evaluation.current_plmc_bond > Zero::zero());
	// let base_weight = Weight::from_parts(10_000_000, 0);
	// if let Some(evaluation) = remaining_evaluations.next() {
	// 	match Pallet::<T>::do_evaluation_unbond_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		evaluation.project_id,
	// 		&evaluation.evaluator,
	// 		evaluation.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::EvaluationUnbondFailed {
	// 			project_id: evaluation.project_id,
	// 			evaluator: evaluation.evaluator.clone(),
	// 			id: evaluation.id,
	// 			error: e,
	// 		}),
	// 	};
	// 	(base_weight.saturating_add(WeightInfoOf::<T>::evaluation_unbond_for()), remaining_evaluations.count() as u64)
	// } else {
	// 	(base_weight, 0u64)
	// }
	todo!()
}

fn release_funds_one_bid<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	// let mut remaining_bids = project_bids.filter(|bid| !bid.funds_released);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(bid) = remaining_bids.next() {
	// 	match Pallet::<T>::do_release_bid_funds_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		bid.project_id,
	// 		&bid.bidder,
	// 		bid.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::ReleaseBidFundsFailed {
	// 			project_id: bid.project_id,
	// 			bidder: bid.bidder.clone(),
	// 			id: bid.id,
	// 			error: e,
	// 		}),
	// 	};
	//
	// 	(base_weight.saturating_add(WeightInfoOf::<T>::release_bid_funds_for()), remaining_bids.count() as u64)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn unbond_one_bid<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	// let mut remaining_bids = project_bids.filter(|bid| bid.funds_released);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(bid) = remaining_bids.next() {
	// 	match Pallet::<T>::do_bid_unbond_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		bid.project_id,
	// 		&bid.bidder,
	// 		bid.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::BidUnbondFailed {
	// 			project_id: bid.project_id,
	// 			bidder: bid.bidder.clone(),
	// 			id: bid.id,
	// 			error: e,
	// 		}),
	// 	};
	// 	(base_weight.saturating_add(WeightInfoOf::<T>::bid_unbond_for()), remaining_bids.count() as u64)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn release_future_ct_deposit_one_participant<T: Config>(
	project_id: ProjectId,
	target: &mut SettlementTarget<T>,
) -> Weight {
	// let base_weight = Weight::from_parts(10_000_000, 0);
	// let evaluators = Evaluations::<T>::iter_key_prefix((project_id,)).map(|(evaluator, _evaluation_id)| evaluator);
	// let bidders = Bids::<T>::iter_key_prefix((project_id,)).map(|(bidder, _bid_id)| bidder);
	// let contributors =
	// 	Contributions::<T>::iter_key_prefix((project_id,)).map(|(contributor, _contribution_id)| contributor);
	// let all_participants = evaluators.chain(bidders).chain(contributors).collect::<BTreeSet<AccountIdOf<T>>>();
	// let remaining_participants = all_participants
	// 	.into_iter()
	// 	.filter(|account| {
	// 		<T as Config>::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), account) >
	// 			Zero::zero()
	// 	})
	// 	.collect_vec();
	// let mut iter_participants = remaining_participants.into_iter();
	//
	// if let Some(account) = iter_participants.next() {
	// 	match Pallet::<T>::do_release_future_ct_deposit_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		project_id,
	// 		&account,
	// 	) {
	// 		// TODO: replace when benchmark is done
	// 		// Ok(_) => return (base_weight.saturating_add(WeightInfoOf::<T>::release_future_ct_deposit_for()), iter_participants.collect_vec()),
	// 		Ok(_) => return (base_weight, iter_participants.count() as u64),
	// 		// TODO: use when storing remaining accounts in outer function calling do_one_operation https://linear.app/polimec/issue/PLMC-410/cleaner-remaining-users-calculation
	// 		// Err(e) if e == Error::<T>::NoFutureDepositHeld.into() => continue,
	// 		Err(e) => {
	// 			Pallet::<T>::deposit_event(Event::ReleaseFutureCTDepositFailed {
	// 				project_id,
	// 				participant: account.clone(),
	// 				error: e,
	// 			});
	// 			// TODO: replace when benchmark is done
	// 			// return (base_weight.saturating_add(WeightInfoOf::<T>::release_future_ct_deposit_for()), iter_participants.collect_vec());
	// 			return (base_weight, iter_participants.count() as u64);
	// 		},
	// 	};
	// }
	// (base_weight, 0u64)

	todo!()
}

fn release_funds_one_contribution<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	// let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.funds_released);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(contribution) = remaining_contributions.next() {
	// 	match Pallet::<T>::do_release_contribution_funds_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		contribution.project_id,
	// 		&contribution.contributor,
	// 		contribution.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::ReleaseContributionFundsFailed {
	// 			project_id: contribution.project_id,
	// 			contributor: contribution.contributor.clone(),
	// 			id: contribution.id,
	// 			error: e,
	// 		}),
	// 	};
	//
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::release_contribution_funds_for()),
	// 		remaining_contributions.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn unbond_one_contribution<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	//
	// let mut remaining_contributions =
	// 	project_contributions.into_iter().filter(|contribution| contribution.funds_released);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(contribution) = remaining_contributions.next() {
	// 	match Pallet::<T>::do_contribution_unbond_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		contribution.project_id,
	// 		&contribution.contributor,
	// 		contribution.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::ContributionUnbondFailed {
	// 			project_id: contribution.project_id,
	// 			contributor: contribution.contributor.clone(),
	// 			id: contribution.id,
	// 			error: e,
	// 		}),
	// 	};
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::contribution_unbond_for()),
	// 		remaining_contributions.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn start_one_bid_vesting_schedule<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	// let mut unscheduled_bids = project_bids.filter(|bid| {
	// 	bid.plmc_vesting_info.is_none() && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..))
	// });
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(bid) = unscheduled_bids.next() {
	// 	match Pallet::<T>::do_start_bid_vesting_schedule_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		project_id,
	// 		&bid.bidder,
	// 		bid.id,
	// 	) {
	// 		Ok(_) => {},
	// 		Err(e) => {
	// 			// TODO: Handle `MAX_VESTING_SCHEDULES` error
	//
	// 			Pallet::<T>::deposit_event(Event::StartBidderVestingScheduleFailed {
	// 				project_id: bid.project_id,
	// 				bidder: bid.bidder.clone(),
	// 				id: bid.id,
	// 				error: e,
	// 			});
	// 		},
	// 	}
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::start_bid_vesting_schedule_for()),
	// 		unscheduled_bids.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn start_one_contribution_vesting_schedule<T: Config>(
	project_id: ProjectId,
	target: &mut SettlementTarget<T>,
) -> Weight {
	// let project_bids = Contributions::<T>::iter_prefix_values((project_id,));
	// let mut unscheduled_contributions = project_bids.filter(|contribution| contribution.plmc_vesting_info.is_none());
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(contribution) = unscheduled_contributions.next() {
	// 	match Pallet::<T>::do_start_contribution_vesting_schedule_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		project_id,
	// 		&contribution.contributor,
	// 		contribution.id,
	// 	) {
	// 		Ok(_) => {},
	// 		Err(e) => {
	// 			Pallet::<T>::deposit_event(Event::StartContributionVestingScheduleFailed {
	// 				project_id: contribution.project_id,
	// 				contributor: contribution.contributor.clone(),
	// 				id: contribution.id,
	// 				error: e,
	// 			});
	// 		},
	// 	}
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::start_contribution_vesting_schedule_for()),
	// 		unscheduled_contributions.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }
	//
	todo!()
}

fn mint_ct_for_one_bid<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	// let mut remaining_bids = project_bids
	// 	.filter(|bid| !bid.ct_minted && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(bid) = remaining_bids.next() {
	// 	match Pallet::<T>::do_bid_ct_mint_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		bid.project_id,
	// 		&bid.bidder,
	// 		bid.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::CTMintFailed {
	// 			project_id: bid.project_id,
	// 			claimer: bid.bidder.clone(),
	// 			id: bid.id,
	// 			error: e.error,
	// 		}),
	// 	};
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::bid_ct_mint_for_with_ct_account_creation()),
	// 		remaining_bids.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn mint_ct_for_one_contribution<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	// let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.ct_minted);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(contribution) = remaining_contributions.next() {
	// 	match Pallet::<T>::do_contribution_ct_mint_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		contribution.project_id,
	// 		&contribution.contributor,
	// 		contribution.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::CTMintFailed {
	// 			project_id: contribution.project_id,
	// 			claimer: contribution.contributor.clone(),
	// 			id: contribution.id,
	// 			error: e.error,
	// 		}),
	// 	};
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::contribution_ct_mint_for_with_ct_account_creation()),
	// 		remaining_contributions.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }
	//
	todo!()
}

fn issuer_funding_payout_one_bid<T: Config>(project_id: ProjectId, target: &mut SettlementTarget<T>) -> Weight {
	// let project_bids = Bids::<T>::iter_prefix_values((project_id,));
	//
	// let mut remaining_bids = project_bids.filter(|bid| {
	// 	!bid.funds_released && matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..))
	// });
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(bid) = remaining_bids.next() {
	// 	match Pallet::<T>::do_payout_bid_funds_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		bid.project_id,
	// 		&bid.bidder,
	// 		bid.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::PayoutContributionFundsFailed {
	// 			project_id: bid.project_id,
	// 			contributor: bid.bidder.clone(),
	// 			id: bid.id,
	// 			error: e,
	// 		}),
	// 	};
	// 	(base_weight.saturating_add(WeightInfoOf::<T>::payout_bid_funds_for()), remaining_bids.count() as u64)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}

fn issuer_funding_payout_one_contribution<T: Config>(
	project_id: ProjectId,
	target: &mut SettlementTarget<T>,
) -> Weight {
	// let project_contributions = Contributions::<T>::iter_prefix_values((project_id,));
	//
	// let mut remaining_contributions = project_contributions.filter(|contribution| !contribution.funds_released);
	// let base_weight = Weight::from_parts(10_000_000, 0);
	//
	// if let Some(contribution) = remaining_contributions.next() {
	// 	match Pallet::<T>::do_payout_contribution_funds_for(
	// 		&T::PalletId::get().into_account_truncating(),
	// 		contribution.project_id,
	// 		&contribution.contributor,
	// 		contribution.id,
	// 	) {
	// 		Ok(_) => (),
	// 		Err(e) => Pallet::<T>::deposit_event(Event::PayoutContributionFundsFailed {
	// 			project_id: contribution.project_id,
	// 			contributor: contribution.contributor.clone(),
	// 			id: contribution.id,
	// 			error: e,
	// 		}),
	// 	};
	//
	// 	(
	// 		base_weight.saturating_add(WeightInfoOf::<T>::payout_contribution_funds_for()),
	// 		remaining_contributions.count() as u64,
	// 	)
	// } else {
	// 	(base_weight, 0u64)
	// }

	todo!()
}
