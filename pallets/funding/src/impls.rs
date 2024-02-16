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
	traits::{fungible::InspectHold, Get},
	weights::{GetDispatchInfo, Weight},
};
use itertools::Itertools;
use sp_arithmetic::traits::Zero;
use sp_runtime::{traits::AccountIdConversion, DispatchError};
use sp_std::{collections::btree_set::BTreeSet, marker::PhantomData};

use crate::{
	traits::{ParticipantExtractor, SettlementOperations, SettlementParticipantsOf, SettlementTarget},
	*,
};

fn get_current_settlement_participants<T: Config>() -> Result<SettlementParticipantsOf<T>, (Weight, DispatchError)> {
	if let Some(participations) = CurrentSettlementParticipations::<T>::get() {
		Ok(participations)
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected current settlement participations to be set");
		#[cfg(not(debug_assertions))]
		return Err((Weight::zero(), "Expected current settlement participations to be set".into()));
	}
}

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
				let current_settlement_participations = get_current_settlement_participants::<T>()?;
				*target = ParticipantExtractor::evaluations(current_settlement_participations);
				Ok(T::DbWeight::get().reads(1))
			},

			SettlementType::EvaluationRewardOrSlash(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = Self::EvaluationUnbonding(PhantomData);
					let project_details = ProjectsDetails::<T>::get(project_id)
						.ok_or((T::DbWeight::get().reads(1), Error::<T>::ImpossibleState.into()))?;
					if project_details.evaluation_round_info.evaluators_outcome == EvaluatorsOutcome::Unchanged {
						*target = SettlementTarget::Evaluations(vec![]);
						return Ok(T::DbWeight::get().reads(1))
					}
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::evaluations(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = reward_or_slash_one_evaluation::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::EvaluationUnbonding(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::StartBidderVestingSchedule(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = unbond_one_evaluation::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::StartBidderVestingSchedule(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::StartContributorVestingSchedule(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::contributions(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = start_one_bid_vesting_schedule::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::StartContributorVestingSchedule(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::BidCTMint(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = start_one_contribution_vesting_schedule::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::BidCTMint(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionCTMint(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::contributions(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = mint_ct_for_one_bid::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::ContributionCTMint(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::BidFundingPayout(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = mint_ct_for_one_contribution::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::BidFundingPayout(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionFundingPayout(PhantomData);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = issuer_funding_payout_one_bid::<T>(target)?;
					Ok(consumed_weight)
				},

			SettlementType::ContributionFundingPayout(PhantomData::<Success>) =>
				if target.is_empty() {
					*self = SettlementType::Finished(PhantomData);
					*target = SettlementTarget::Empty;
					Ok(Weight::zero())
				} else {
					let consumed_weight = issuer_funding_payout_one_contribution::<T>(target)?;
					Ok(consumed_weight)
				},
			SettlementType::Finished(PhantomData::<Success>) =>
				Err((Weight::zero(), "Cannot operate on finished settlement machine".into())),

			// Transitions enforced by the type system
			_ => Err((Weight::zero(), Error::<T>::ImpossibleState.into())),
		}
	}

	fn execute_with_given_weight(
		&mut self,
		weight: Weight,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)> {
		let mut used_weight = Weight::zero();
		let has_remaining_operations =
			<SettlementType<Success> as SettlementOperations<T>>::has_remaining_operations(self);

		while has_remaining_operations && weight.saturating_sub(used_weight).all_gt(Weight::zero()) {
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
		match self {
			SettlementType::Initialized(PhantomData::<Failure>) => {
				*self = SettlementType::EvaluationRewardOrSlash(PhantomData::<Failure>);
				let current_settlement_participations = get_current_settlement_participants::<T>()?;
				*target = ParticipantExtractor::evaluations(current_settlement_participations);
				Ok(T::DbWeight::get().reads(1))
			},

			SettlementType::EvaluationRewardOrSlash(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::FutureDepositRelease(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::accounts(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = reward_or_slash_one_evaluation::<T>(target)?;
					*self = SettlementType::EvaluationRewardOrSlash(PhantomData);
					Ok(consumed_weight)
				},

			SettlementType::FutureDepositRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::EvaluationUnbonding(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::evaluations(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = release_future_ct_deposit_one_participant::<T>(project_id, target)?;
					*self = SettlementType::FutureDepositRelease(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::EvaluationUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::BidFundingRelease(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = unbond_one_evaluation::<T>(target)?;
					*self = SettlementType::EvaluationUnbonding(PhantomData);
					Ok(consumed_weight)
				},
			SettlementType::BidFundingRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::BidUnbonding(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::bids(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = release_funds_one_bid::<T>(target)?;
					*self = SettlementType::BidFundingRelease(PhantomData);
					Ok(consumed_weight)
				},
			SettlementType::BidUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionFundingRelease(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::contributions(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = unbond_one_bid::<T>(target)?;
					*self = SettlementType::BidUnbonding(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::ContributionFundingRelease(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::ContributionUnbonding(PhantomData::<Failure>);
					let current_settlement_participations = get_current_settlement_participants::<T>()?;
					*target = ParticipantExtractor::contributions(current_settlement_participations);
					Ok(T::DbWeight::get().reads(1))
				} else {
					let consumed_weight = release_funds_one_contribution::<T>(target)?;
					*self = SettlementType::ContributionFundingRelease(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::ContributionUnbonding(PhantomData::<Failure>) =>
				if target.is_empty() {
					*self = SettlementType::Finished(PhantomData::<Failure>);
					*target = SettlementTarget::Empty;
					Ok(Weight::zero())
				} else {
					let consumed_weight = unbond_one_contribution::<T>(target)?;
					*self = SettlementType::ContributionUnbonding(PhantomData::<Failure>);
					Ok(consumed_weight)
				},
			SettlementType::Finished(PhantomData::<Failure>) =>
				Err((Weight::zero(), Error::<T>::FinalizerFinished.into())),

			_ => Err((Weight::zero(), Error::<T>::ImpossibleState.into())),
		}
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

fn reward_or_slash_one_evaluation<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_evaluations = if let SettlementTarget::Evaluations(evaluations) = target.clone() {
		evaluations.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected evaluations at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected evaluations at settlement target".into()));
	};

	if let Some(evaluation) = remaining_evaluations.next() {
		let project_id = evaluation.project_id;
		let project_details = ProjectsDetails::<T>::get(project_id)
			.ok_or((consumed_weight.saturating_add(T::DbWeight::get().reads(1)), Error::<T>::ImpossibleState.into()))?;
		consumed_weight = consumed_weight.saturating_add(T::DbWeight::get().reads(1));

		match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(_) => {
				match Pallet::<T>::do_evaluation_reward_payout_for(
					&T::PalletId::get().into_account_truncating(),
					evaluation.project_id,
					&evaluation.evaluator,
					evaluation.id,
				) {
					Ok(result) => {
						if let Some(weight) = result.actual_weight {
							consumed_weight = consumed_weight.saturating_add(weight)
						};
					},
					Err(e) => {
						if let Some(weight) = e.post_info.actual_weight {
							consumed_weight = consumed_weight.saturating_add(weight)
						};
						Pallet::<T>::deposit_event(Event::EvaluationRewardFailed {
							project_id: evaluation.project_id,
							evaluator: evaluation.evaluator.clone(),
							id: evaluation.id,
							error: e.error,
						})
					},
				};

				consumed_weight = consumed_weight.saturating_add(
					crate::Call::<T>::evaluation_reward_payout_for {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator.clone(),
						evaluation_id: evaluation.id,
					}
					.get_dispatch_info()
					.weight,
				)
			},

			EvaluatorsOutcome::Slashed => {
				match Pallet::<T>::do_evaluation_slash_for(
					&T::PalletId::get().into_account_truncating(),
					evaluation.project_id,
					&evaluation.evaluator,
					evaluation.id,
				) {
					Ok(_) => {},
					Err(e) => Pallet::<T>::deposit_event(Event::EvaluationSlashFailed {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator.clone(),
						id: evaluation.id,
						error: e,
					}),
				};

				consumed_weight = consumed_weight.saturating_add(
					crate::Call::<T>::evaluation_slash_for {
						project_id: evaluation.project_id,
						evaluator: evaluation.evaluator,
						evaluation_id: evaluation.id,
					}
					.get_dispatch_info()
					.weight,
				)
			},
			_ => {
				#[cfg(debug_assertions)]
				unreachable!("EvaluatorsOutcome should be either Slashed or Rewarded if this function is called");
				#[cfg(not(debug_assertions))]
				return Err((consumed_weight, Error::<T>::ImpossibleState.into()))
			},
		}
	}

	*target = SettlementTarget::Evaluations(remaining_evaluations.collect_vec());

	Ok(consumed_weight)
}

fn unbond_one_evaluation<T: Config>(target: &mut SettlementTarget<T>) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_evaluations = if let SettlementTarget::Evaluations(evaluations) = target.clone() {
		evaluations.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected evaluations at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected evaluations at settlement target".into()));
	};
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
		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::evaluation_unbond_for {
				project_id: evaluation.project_id,
				evaluator: evaluation.evaluator,
				evaluation_id: evaluation.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Evaluations(remaining_evaluations.collect_vec());
	Ok(consumed_weight)
}

fn release_funds_one_bid<T: Config>(target: &mut SettlementTarget<T>) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_bids = if let SettlementTarget::Bids(bids) = target.clone() {
		bids.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected bids at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected bids at settlement target".into()));
	};

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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::release_bid_funds_for { project_id: bid.project_id, bidder: bid.bidder, bid_id: bid.id }
				.get_dispatch_info()
				.weight,
		);
	}
	*target = SettlementTarget::Bids(remaining_bids.collect_vec());
	Ok(consumed_weight)
}

fn unbond_one_bid<T: Config>(target: &mut SettlementTarget<T>) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();

	let mut remaining_bids = if let SettlementTarget::Bids(bids) = target.clone() {
		bids.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected bids at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected bids at settlement target".into()));
	};

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
	}

	*target = SettlementTarget::Bids(remaining_bids.collect_vec());
	Ok(consumed_weight)
}

fn release_future_ct_deposit_one_participant<T: Config>(
	project_id: ProjectId,
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_participants = if let SettlementTarget::Accounts(accounts) = target.clone() {
		accounts.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected participants at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected participants at settlement target".into()));
	};

	if let Some(account) = remaining_participants.next() {
		match Pallet::<T>::do_release_future_ct_deposit_for(
			&T::PalletId::get().into_account_truncating(),
			project_id,
			&account,
		) {
			Ok(_) => (),
			Err(e) => {
				Pallet::<T>::deposit_event(Event::ReleaseFutureCTDepositFailed {
					project_id,
					participant: account.clone(),
					error: e,
				});
			},
		};

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::release_future_ct_deposit_for { project_id, participant: account }
				.get_dispatch_info()
				.weight,
		);
	}

	*target = SettlementTarget::Accounts(remaining_participants.collect_vec());
	Ok(consumed_weight)
}

fn release_funds_one_contribution<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_contributions = if let SettlementTarget::Contributions(contributions) = target.clone() {
		contributions.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected contributions at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected contributions at settlement target".into()));
	};
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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::release_contribution_funds_for {
				project_id: contribution.project_id,
				contributor: contribution.contributor,
				contribution_id: contribution.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Contributions(remaining_contributions.collect_vec());
	Ok(consumed_weight)
}

fn unbond_one_contribution<T: Config>(target: &mut SettlementTarget<T>) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_contributions = if let SettlementTarget::Contributions(contributions) = target.clone() {
		contributions.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected contributions at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected contributions at settlement target".into()));
	};

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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::contribution_unbond_for {
				project_id: contribution.project_id,
				contributor: contribution.contributor,
				contribution_id: contribution.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Contributions(remaining_contributions.collect_vec());
	Ok(consumed_weight)
}

fn start_one_bid_vesting_schedule<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_bids = if let SettlementTarget::Bids(bids) = target.clone() {
		bids.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected bids at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected bids at settlement target".into()));
	};

	if let Some(bid) = remaining_bids.next() {
		match Pallet::<T>::do_start_bid_vesting_schedule_for(
			&T::PalletId::get().into_account_truncating(),
			bid.project_id,
			&bid.bidder,
			bid.id,
		) {
			Ok(_) => (),
			Err(e) => {
				Pallet::<T>::deposit_event(Event::StartBidderVestingScheduleFailed {
					project_id: bid.project_id,
					bidder: bid.bidder.clone(),
					id: bid.id,
					error: e,
				});
			},
		}

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::start_bid_vesting_schedule_for {
				project_id: bid.project_id,
				bidder: bid.bidder,
				bid_id: bid.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Bids(remaining_bids.collect_vec());
	Ok(consumed_weight)
}

fn start_one_contribution_vesting_schedule<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_contributions = if let SettlementTarget::Contributions(contributions) = target.clone() {
		contributions.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected contributions at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected contributions at settlement target".into()));
	};

	if let Some(contribution) = remaining_contributions.next() {
		match Pallet::<T>::do_start_contribution_vesting_schedule_for(
			&T::PalletId::get().into_account_truncating(),
			contribution.project_id,
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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::start_contribution_vesting_schedule_for {
				project_id: contribution.project_id,
				contributor: contribution.contributor,
				contribution_id: contribution.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Contributions(remaining_contributions.collect_vec());
	Ok(consumed_weight)
}

fn mint_ct_for_one_bid<T: Config>(target: &mut SettlementTarget<T>) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_bids = if let SettlementTarget::Bids(bids) = target.clone() {
		bids.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected bids at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected bids at settlement target".into()));
	};

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
				error: e.error,
			}),
		};

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::bid_ct_mint_for { project_id: bid.project_id, bidder: bid.bidder, bid_id: bid.id }
				.get_dispatch_info()
				.weight,
		);
	}

	*target = SettlementTarget::Bids(remaining_bids.collect_vec());
	Ok(consumed_weight)
}

fn mint_ct_for_one_contribution<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_contributions = if let SettlementTarget::Contributions(contributions) = target.clone() {
		contributions.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected contributions at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected contributions at settlement target".into()));
	};

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
				error: e.error,
			}),
		};

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::contribution_ct_mint_for {
				project_id: contribution.project_id,
				contributor: contribution.contributor,
				contribution_id: contribution.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Contributions(remaining_contributions.collect_vec());
	Ok(consumed_weight)
}

fn issuer_funding_payout_one_bid<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_bids = if let SettlementTarget::Bids(bids) = target.clone() {
		bids.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected bids at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected bids at settlement target".into()));
	};

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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::payout_contribution_funds_for {
				project_id: bid.project_id,
				contributor: bid.bidder,
				contribution_id: bid.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Bids(remaining_bids.collect_vec());
	Ok(consumed_weight)
}

fn issuer_funding_payout_one_contribution<T: Config>(
	target: &mut SettlementTarget<T>,
) -> Result<Weight, (Weight, DispatchError)> {
	let mut consumed_weight = Weight::zero();
	let mut remaining_contributions = if let SettlementTarget::Contributions(contributions) = target.clone() {
		contributions.into_iter()
	} else {
		#[cfg(debug_assertions)]
		unreachable!("Expected contributions at settlement target");
		#[cfg(not(debug_assertions))]
		return Err((consumed_weight, "Expected contributions at settlement target".into()));
	};

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

		consumed_weight = consumed_weight.saturating_add(
			crate::Call::<T>::payout_contribution_funds_for {
				project_id: contribution.project_id,
				contributor: contribution.contributor,
				contribution_id: contribution.id,
			}
			.get_dispatch_info()
			.weight,
		);
	}

	*target = SettlementTarget::Contributions(remaining_contributions.collect_vec());
	Ok(consumed_weight)
}
