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
	pub fn do_end_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let round_end_block = project_details.round_duration.end().ok_or(Error::<T>::ImpossibleState)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();

		// * Validity checks *
		ensure!(
			// Can end due to running out of CTs
			remaining_cts == Zero::zero() ||
				// or the last funding round ending
				now > round_end_block && matches!(project_details.status, ProjectStatus::CommunityRound(..)),
			Error::<T>::TooEarlyForRound
		);

		// * Calculate new variables *
		let funding_target = project_details.fundraising_target_usd;
		let funding_reached = project_details.funding_amount_reached_usd;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		DidWithActiveProjects::<T>::set(issuer_did, None);
		let evaluator_outcome = match funding_ratio {
			ratio if ratio <= Perquintill::from_percent(75u64) => EvaluatorsOutcome::Slashed,
			ratio if ratio < Perquintill::from_percent(90u64) => EvaluatorsOutcome::Unchanged,
			_ => {
				let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
				EvaluatorsOutcome::Rewarded(reward_info)
			},
		};

		project_details.evaluation_round_info.evaluators_outcome = evaluator_outcome;

		let (next_status, duration, actual_weight) = if funding_ratio <= T::FundingSuccessThreshold::get() {
			(
				ProjectStatus::FundingFailed,
				1u32.into(),
				WeightInfoOf::<T>::end_funding_automatically_rejected_evaluators_slashed(1),
			)
		} else {
			(
				ProjectStatus::FundingSuccessful,
				T::SuccessToSettlementTime::get(),
				WeightInfoOf::<T>::end_funding_automatically_accepted_evaluators_rewarded(1, 1),
			)
		};

		let round_end = now.saturating_add(duration).saturating_sub(One::one());
		project_details.round_duration.update(Some(now), Some(round_end));
		project_details.status = next_status;

		Ok(PostDispatchInfo { actual_weight: Some(actual_weight), pays_fee: Pays::Yes })
	}
}
