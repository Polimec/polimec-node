#[allow(clippy::wildcard_imports)]
use super::*;
use itertools::Itertools;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_end_funding(project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();
		let ct_amount_oversubscribed = CTAmountOversubscribed::<T>::get(project_id);

		// * Validity checks *
		ensure!(
			project_details.round_duration.ended(now) && matches!(project_details.status, ProjectStatus::AuctionRound),
			Error::<T>::TooEarlyForRound
		);
		ensure!(ct_amount_oversubscribed.is_zero(), Error::<T>::OversubscribedBidsRemaining);

		let mut project_ids = ProjectsInAuctionRound::<T>::get().to_vec();
		let (pos, _) = project_ids.iter().find_position(|id| **id == project_id).ok_or(Error::<T>::ImpossibleState)?;
		project_ids.remove(pos);
		ProjectsInAuctionRound::<T>::put(WeakBoundedVec::force_from(project_ids, None));

		let auction_allocation_size = project_metadata.total_allocation_size;

		let bucket_price_higher_than_initial = bucket.current_price > bucket.initial_price;
		let sold_percent =
			Perquintill::from_rational(auction_allocation_size - bucket.amount_left, auction_allocation_size);
		let threshold = T::FundingSuccessThreshold::get();
		let sold_more_than_min = sold_percent >= threshold;

		let funding_successful = bucket_price_higher_than_initial || sold_more_than_min;

		DidWithActiveProjects::<T>::set(issuer_did, None);

		let usd_raised = bucket.calculate_usd_raised(auction_allocation_size);
		project_details.funding_amount_reached_usd = usd_raised;
		project_details.remaining_contribution_tokens =
			if bucket.current_price == bucket.initial_price { bucket.amount_left } else { Zero::zero() };
		ProjectsDetails::<T>::insert(project_id, project_details.clone());

		// * Update project status *
		let next_status = if funding_successful {
			let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = Some(EvaluatorsOutcome::Rewarded(reward_info));
			ProjectStatus::FundingSuccessful
		} else {
			project_details.evaluation_round_info.evaluators_outcome = Some(EvaluatorsOutcome::Slashed);
			ProjectStatus::FundingFailed
		};

		Self::transition_project(project_id, project_details.clone(), project_details.status, next_status, None, true)?;

		Ok(())
	}
}
