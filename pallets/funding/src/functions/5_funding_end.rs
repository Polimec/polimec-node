#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_end_funding(project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();

		// * Validity checks *
		ensure!(
			// Can end due to running out of CTs
			remaining_cts == Zero::zero() ||
				// or the last funding round ending
				project_details.round_duration.ended(now) && matches!(project_details.status, ProjectStatus::CommunityRound(..)),
			Error::<T>::TooEarlyForRound
		);

		// * Calculate new variables *
		let funding_target = project_details.fundraising_target_usd;
		let funding_reached = project_details.funding_amount_reached_usd;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		DidWithActiveProjects::<T>::set(issuer_did, None);

		let next_status = if funding_ratio < T::FundingSuccessThreshold::get() {
			project_details.evaluation_round_info.evaluators_outcome = Some(EvaluatorsOutcome::Slashed);
			ProjectStatus::FundingFailed
		} else {
			let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = Some(EvaluatorsOutcome::Rewarded(reward_info));
			ProjectStatus::FundingSuccessful
		};

		Self::transition_project(project_id, project_details.clone(), project_details.status, next_status, None, true)?;

		Ok(())
	}
}
