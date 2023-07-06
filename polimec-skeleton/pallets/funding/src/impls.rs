use crate::traits::DoRemainingOperation;
use crate::{Config, EvaluationInfoOf, Evaluations, Event, FailureRemainingOperations, Pallet, RemainingOperations, SuccessRemainingOperations, WeightInfo};
use frame_support::traits::Get;
use frame_support::weights::Weight;
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

impl DoRemainingOperation for RemainingOperations {
	fn is_done(&self) -> bool {
		matches!(self, RemainingOperations::None)
	}
	fn do_one_operation<T: crate::Config>(&mut self, project_id: T::ProjectIdentifier) -> Result<Weight, ()> {
		match self {
			RemainingOperations::None => Err(()),
			RemainingOperations::Success(ops) => {
				let weight = ops.do_one_operation::<T>(project_id);
				if ops.is_done() {
					*self = RemainingOperations::None;
				}
				weight
			}
			RemainingOperations::Failure(ops) => {
				let weight = ops.do_one_operation::<T>(project_id);
				if ops.is_done() {
					*self = RemainingOperations::None;
				}
				weight
			}
		}
	}
}

impl DoRemainingOperation for FailureRemainingOperations {
	fn is_done(&self) -> bool {
		!self.evaluation_unbonding
			&& !self.bidder_plmc_unbonding
			&& !self.contributor_plmc_unbonding
			&& !self.bids_funding_to_bidder_return
			&& !self.contributions_funding_to_contributor_return
	}
	fn do_one_operation<T: crate::Config>(&mut self, project_id: T::ProjectIdentifier) -> Result<Weight, ()> {
		if self.evaluation_reward_or_slash {


		} else if self.evaluation_unbonding {
			let evaluations = Evaluations::<T>::iter_prefix_values(project_id)
				.flatten()
				.collect::<Vec<EvaluationInfoOf<T>>>();

			let evaluation = evaluations
				.iter()
				.find(|evaluation| evaluation.rewarded_or_slashed == true)
				.ok_or(())?;
			Pallet::<T>::do_evaluation_unbond_for(
				T::PalletId::get().into_account_truncating(),
				evaluation.project_id,
				evaluation.evaluator.clone(),
				evaluation.id,
			)
			.map_err(|_| ())?;

			if evaluations.len() == 1 {
				self.evaluation_unbonding = false;
			}

			Ok(T::WeightInfo::evaluation_unbond_for())
		} else if self.bidder_plmc_unbonding {
			todo!();
		} else if self.contributor_plmc_unbonding {
			todo!();
		} else if self.bids_funding_to_bidder_return {
			todo!();
		} else if self.contributions_funding_to_contributor_return {
			todo!();
		} else {
			todo!();
		}
	}
}

impl DoRemainingOperation for SuccessRemainingOperations {
	fn is_done(&self) -> bool {
		!self.evaluation_unbonding
			&& !self.bidder_plmc_vesting
			&& !self.bidder_ct_mint
			&& !self.contributor_plmc_vesting
			&& !self.contributor_ct_mint
			&& !self.bids_funding_to_issuer_transfer
			&& !self.contributions_funding_to_issuer_transfer
	}
	fn do_one_operation<T: crate::Config>(&mut self, project_id: T::ProjectIdentifier) -> Result<Weight, ()> {
		if self.evaluation_reward_or_slash {
			reward_or_slash_one_evaluation::<T>(project_id).or_else(|_| {
				self.evaluation_reward_or_slash = false;
				Ok(Weight::zero())
			})
		} else if self.evaluation_unbonding {
			todo!();
		} else if self.bidder_plmc_vesting {
			todo!();
		} else if self.bidder_ct_mint {
			todo!();
		} else if self.contributor_plmc_vesting {
			todo!();
		} else if self.contributor_ct_mint {
			todo!();
		} else if self.bids_funding_to_issuer_transfer {
			todo!();
		} else if self.contributions_funding_to_issuer_transfer {
			todo!();
		} else {
			todo!();
		}
	}
}

enum OperationsLeft {
	Some(u64),
	None,
}

fn reward_or_slash_one_evaluation<T: Config>(project_id: T::ProjectIdentifier) -> Result<Weight, ()> {
	let mut user_evaluations = Evaluations::<T>::iter_prefix_values(project_id)
		.find(|evaluations| evaluations.iter().any(|e| !e.rewarded_or_slashed)).ok_or(())?;

	let mut evaluation = user_evaluations
		.iter_mut()
		.find(|evaluation| !evaluation.rewarded_or_slashed)
		.expect("user_evaluations can only be Some if an item here is found; qed");

	Pallet::<T>::do_evaluation_reward_or_slash(
		T::PalletId::get().into_account_truncating(),
		evaluation.project_id,
		evaluation.evaluator.clone(),
		evaluation.id,
	)
		.map_err(|_| ())?;

	evaluation.rewarded_or_slashed = true;

	Evaluations::<T>::insert(project_id, evaluation.evaluator.clone(), user_evaluations);

	Ok(Weight::zero())
}

fn unbond_one_evaluation<T: crate::Config>(project_id: T::ProjectIdentifier) -> Result<Weight, ()> {
	let mut user_evaluations = Evaluations::<T>::iter_prefix_values(project_id)
		.find(|evaluations| evaluations.iter().any(|e| e.rewarded_or_slashed)).ok_or(())?;

	let mut evaluation = user_evaluations
		.iter_mut()
		.find(|evaluation| evaluation.rewarded_or_slashed)
		.expect("user_evaluations can only be Some if an item here is found; qed");

	Pallet::<T>::do_evaluation_unbond_for(
		T::PalletId::get().into_account_truncating(),
		evaluation.project_id,
		evaluation.evaluator.clone(),
		evaluation.id,
	)
		.map_err(|_| ())?;

	Evaluations::<T>::insert(project_id, evaluation.evaluator.clone(), user_evaluations);

	Ok(Weight::zero())
}

fn unbond_evaluators<T: crate::Config>(
	project_id: T::ProjectIdentifier, max_weight: Weight,
) -> (Weight, OperationsLeft) {
	let evaluations = Evaluations::<T>::iter_prefix_values(project_id)
		.flatten()
		.collect::<Vec<EvaluationInfoOf<T>>>();

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

	let successful_results = unbond_results
		.into_iter()
		.filter(|result| {
			if let Err(e) = result {
				Pallet::<T>::deposit_event(Event::EvaluationUnbondFailed { error: *e });
				false
			} else {
				true
			}
		})
		.collect::<Vec<_>>();

	let operations_left = if successful_results.len() == evaluations.len() {
		OperationsLeft::None
	} else {
		OperationsLeft::Some(evaluations.len().saturating_sub(successful_results.len()) as u64)
	};

	(used_weight, operations_left)
}
