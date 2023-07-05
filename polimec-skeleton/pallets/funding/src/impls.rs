use frame_support::weights::Weight;
use sp_runtime::DispatchError;
use crate::{Evaluations, FailureRemainingOperations, RemainingOperations, SuccessRemainingOperations};

impl RemainingOperations {
    pub fn do_one_operation(&mut self) -> Result<Weight, ()> {
        match self {
            RemainingOperations::None => Err(()),
            RemainingOperations::Success(ops) => Ok(Weight::from_parts(100_000u64, 100_000_000u64)),
            RemainingOperations::Failure(ops) => Ok(Weight::from_parts(100_000u64, 100_000_000u64))
        }
    }
}

fn unbond_evaluators<T: crate::Config>(project_id: T::ProjectIdentifier, max_weight: Weight) -> Result<Weight, DispatchError> {
    // Unbond the plmc from failed evaluation projects
    let evaluations = Evaluations::<T>::iter_prefix_values(project_id)
        .map(|(_evaluator, evaluations)| evaluations)
        .flatten()
        .collect::<Vec<_>>();
    
    let mut used_weight = Weight::zero();
    
    let unbond_results = evaluations
        // Retrieve as many as possible for the given weight
        .take_while(|_bond| {
            let new_used_weight = used_weight.saturating_add(T::WeightInfo::evaluation_unbond_for());
            if new_used_weight <= max_weight {
                used_weight = new_used_weight;
                true
            } else {
                false
            }
        })
        // Unbond the plmc
        .map(|bond|
            Self::do_evaluation_unbond_for()
        
        .collect::<Vec<_>>();


    
    // Make sure no unbonding failed
    for result in unbond_results {
        if let Err(e) = result {
            Self::deposit_event(Event::<T>::FailedEvaluationUnbondFailed { error: e });
        }
    }
}