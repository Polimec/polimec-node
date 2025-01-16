#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_end_funding(project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();

		// * Validity checks *
		ensure!(
			// Can end due to running out of CTs
			remaining_cts == Zero::zero() ||
				// or the last funding round ending
				project_details.round_duration.ended(now) && matches!(project_details.status, ProjectStatus::AuctionRound),
			Error::<T>::TooEarlyForRound
		);

		// * Calculate WAP *
		let auction_allocation_size = project_metadata.total_allocation_size;
		let weighted_token_price = bucket.calculate_wap(auction_allocation_size);

		// * Update Storage *
		let _calculation_result =
			Self::decide_winning_bids(project_id, project_metadata.total_allocation_size, weighted_token_price)?;
		let mut updated_project_details =
			ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		DidWithActiveProjects::<T>::set(issuer_did, None);

		// * Calculate new variables *
		let funding_target = updated_project_details.fundraising_target_usd;
		let funding_reached = updated_project_details.funding_amount_reached_usd;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update project status *
		let next_status = if funding_ratio < T::FundingSuccessThreshold::get() {
			updated_project_details.evaluation_round_info.evaluators_outcome = Some(EvaluatorsOutcome::Slashed);
			ProjectStatus::FundingFailed
		} else {
			let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
			updated_project_details.evaluation_round_info.evaluators_outcome =
				Some(EvaluatorsOutcome::Rewarded(reward_info));
			ProjectStatus::FundingSuccessful
		};

		Self::transition_project(
			project_id,
			updated_project_details.clone(),
			project_details.status,
			next_status,
			None,
			true,
		)?;

		Ok(())
	}
}
