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
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let remainder_end_block = project_details.phase_transition_points.remainder.end();
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();

		// * Validity checks *
		ensure!(
			// Can end due to running out of CTs
			remaining_cts == Zero::zero() ||
				// or the auction being empty
				project_details.status == ProjectStatus::AuctionClosing ||
				// or the last funding round ending
				matches!(remainder_end_block, Some(end_block) if now > end_block),
			Error::<T>::TooEarlyForRound
		);
		// do_end_funding was already executed, but automatic transition was included in the
		// do_remainder_funding function. We gracefully skip the this transition.
		ensure!(
			!matches!(
				project_details.status,
				ProjectStatus::FundingSuccessful |
					ProjectStatus::FundingFailed |
					ProjectStatus::AwaitingProjectDecision
			),
			Error::<T>::RoundTransitionAlreadyHappened
		);

		// * Calculate new variables *
		let funding_target = project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let funding_reached = project_details.funding_amount_reached_usd;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		DidWithActiveProjects::<T>::set(issuer_did, None);
		if funding_ratio <= Perquintill::from_percent(33u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			let insertion_iterations =
				Self::finalize_funding(project_id, project_details, ProjectOutcome::FundingFailed, 1u32.into())?;
			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_automatically_rejected_evaluators_slashed(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			});
		} else if funding_ratio <= Perquintill::from_percent(75u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			let insertion_iterations = match Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_slashed(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			})
		} else if funding_ratio < Perquintill::from_percent(90u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Unchanged;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			let insertion_iterations = match Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_unchanged(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			})
		} else {
			let (reward_info, evaluations_count) = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Rewarded(reward_info);

			let insertion_iterations = Self::finalize_funding(
				project_id,
				project_details,
				ProjectOutcome::FundingSuccessful,
				T::SuccessToSettlementTime::get(),
			)?;
			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_automatically_accepted_evaluators_rewarded(
					insertion_iterations,
					evaluations_count,
				)),
				pays_fee: Pays::Yes,
			});
		}
	}

	#[transactional]
	pub fn do_project_decision(project_id: ProjectId, decision: FundingOutcomeDecision) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(
			project_details.status == ProjectStatus::AwaitingProjectDecision,
			Error::<T>::RoundTransitionAlreadyHappened
		);
		let outcome = match decision {
			FundingOutcomeDecision::AcceptFunding => ProjectOutcome::FundingAccepted,
			FundingOutcomeDecision::RejectFunding => ProjectOutcome::FundingRejected,
		};

		// * Update storage *
		Self::finalize_funding(project_id, project_details, outcome, T::SuccessToSettlementTime::get())?;
		Ok(PostDispatchInfo { actual_weight: Some(WeightInfoOf::<T>::project_decision()), pays_fee: Pays::Yes })
	}

	#[transactional]
	pub fn do_decide_project_outcome(
		issuer: AccountIdOf<T>,
		project_id: ProjectId,
		decision: FundingOutcomeDecision,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer_account == issuer, Error::<T>::NotIssuer);
		ensure!(project_details.status == ProjectStatus::AwaitingProjectDecision, Error::<T>::IncorrectRound);

		// * Update storage *
		let insertion_attempts =
			match Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::ProjectDecision(decision))) {
				Ok(iterations) => iterations,
				Err(iterations) =>
					return Err(DispatchErrorWithPostInfo {
						post_info: PostDispatchInfo {
							actual_weight: Some(WeightInfoOf::<T>::decide_project_outcome(iterations)),
							pays_fee: Pays::Yes,
						},
						error: Error::<T>::TooManyInsertionAttempts.into(),
					}),
			};

		Self::deposit_event(Event::ProjectOutcomeDecided { project_id, decision });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::decide_project_outcome(insertion_attempts)),
			pays_fee: Pays::Yes,
		})
	}

	pub fn finalize_funding(
		project_id: ProjectId,
		mut project_details: ProjectDetailsOf<T>,
		outcome: ProjectOutcome,
		settlement_delta: BlockNumberFor<T>,
	) -> Result<u32, DispatchError> {
		let now = <frame_system::Pallet<T>>::block_number();

		project_details.status = match outcome {
			ProjectOutcome::FundingSuccessful | ProjectOutcome::FundingAccepted => ProjectStatus::FundingSuccessful,
			_ => ProjectStatus::FundingFailed,
		};
		ProjectsDetails::<T>::insert(project_id, project_details);

		let insertion_iterations =
			Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement))
				.map_err(|_| Error::<T>::TooManyInsertionAttempts)?;
		Self::deposit_event(Event::ProjectPhaseTransition {
			project_id,
			phase: ProjectPhases::FundingFinalization(outcome),
		});
		Ok(insertion_iterations)
	}
}
