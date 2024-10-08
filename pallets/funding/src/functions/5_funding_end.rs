#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	/// Called automatically by on_initialize
	/// Ends the project funding, and calculates if the project was successfully funded or not.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, the current block is after the remainder funding end period.
	/// Update the project information with the new round status.
	///
	/// # Success Path
	/// The validity checks pass, and either of 2 paths happen:
	///
	/// * Project achieves its funding target - the project info is set to a successful funding state,
	/// and the contribution token asset class is created with the same id as the project.
	///
	/// * Project doesn't achieve its funding target - the project info is set to an unsuccessful funding state.
	///
	/// # Next step
	/// If **successful**, bidders can claim:
	///	* Contribution tokens with [`vested_contribution_token_bid_mint_for`](Self::vested_contribution_token_bid_mint_for)
	/// * Bonded plmc with [`vested_plmc_bid_unbond_for`](Self::vested_plmc_bid_unbond_for)
	///
	/// And contributors can claim:
	/// * Contribution tokens with [`vested_contribution_token_purchase_mint_for`](Self::vested_contribution_token_purchase_mint_for)
	/// * Bonded plmc with [`vested_plmc_purchase_unbond_for`](Self::vested_plmc_purchase_unbond_for)
	///
	/// If **unsuccessful**, users every user should have their PLMC vesting unbonded.
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
