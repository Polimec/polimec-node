#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	/// Called by user extrinsic
	/// Starts the evaluation round of a project. It needs to be called by the project issuer.
	///
	/// # Arguments
	/// * `project_id` - The id of the project to start the evaluation round for.
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Checking and updating the round status, transition points and freezing the project.
	///
	/// # Success path
	/// The project information is found, its round status was in Application round, and It's not yet frozen.
	/// The pertinent project info is updated on the storage, and the project is scheduled for automatic transition by on_initialize.
	///
	/// # Next step
	/// Users will pond PLMC for this project, and when the time comes, the project will be transitioned
	/// to the next round by `on_initialize` using [`do_evaluation_end`](Self::do_end_evaluation)
	#[transactional]
	pub fn do_start_evaluation(caller: AccountIdOf<T>, project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer_account == caller, Error::<T>::NotIssuer);
		ensure!(!project_details.is_frozen, Error::<T>::ProjectAlreadyFrozen);
		ensure!(project_metadata.policy_ipfs_cid.is_some(), Error::<T>::CidNotProvided);

		// * Update storage *
		project_details.is_frozen = true;

		// * Transition Round *
		Self::transition_project(
			project_id,
			project_details,
			ProjectStatus::Application,
			ProjectStatus::EvaluationRound,
			Some(T::EvaluationRoundDuration::get()),
			false,
		)
	}

	/// Called automatically by on_initialize.
	/// Ends the evaluation round, and sets the current round to `AuctionInitializePeriod` if it
	/// reached enough PLMC bonding, or to `FundingFailed` if it didn't.
	///
	/// # Arguments
	/// * `project_id` - The id of the project to end the evaluation round for.
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Checking the round status and transition points for validity, and updating
	/// the round status and transition points in case of success or failure of the evaluation.
	/// * [`Evaluations`] - Checking that the threshold for PLMC bonded was reached, to decide
	/// whether the project failed or succeeded.
	///
	/// # Possible paths
	/// * Project achieves its evaluation goal. >=10% of the target funding was reached through bonding,
	/// so the project is transitioned to the [`AuctionInitializePeriod`](ProjectStatus::AuctionInitializePeriod) round. The project information
	/// is updated with the new transition points and round status.
	///
	/// * Project doesn't reach the evaluation goal - <10% of the target funding was reached
	/// through bonding, so the project is transitioned to the `FundingFailed` round. The project
	/// information is updated with the new rounds status and it is scheduled for automatic unbonding.
	///
	/// # Next step
	/// * Bonding achieved - The issuer calls an extrinsic within the set period to initialize the
	/// auction round. `auction` is called
	///
	/// * Bonding failed - `on_idle` at some point checks for failed evaluation projects, and
	/// unbonds the evaluators funds.
	#[transactional]
	pub fn do_end_evaluation(project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Calculate new variables *
		let usd_total_amount_bonded = project_details.evaluation_round_info.total_bonded_usd;
		let evaluation_target_usd =
			<T as Config>::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;

		// Check which logic path to follow
		let is_funded = usd_total_amount_bonded >= evaluation_target_usd;

		// * Branch in possible project paths *
		// Successful path
		return if is_funded {
			Self::transition_project(
				project_id,
				project_details,
				ProjectStatus::EvaluationRound,
				ProjectStatus::AuctionRound,
				Some(T::AuctionRoundDuration::get()),
				false,
			)
		// Unsuccessful path
		} else {
			let issuer_did = project_details.issuer_did.clone();
			DidWithActiveProjects::<T>::set(issuer_did, None);
			Self::transition_project(
				project_id,
				project_details,
				ProjectStatus::EvaluationRound,
				ProjectStatus::FundingFailed,
				Some(One::one()),
				false,
			)
		}
	}

	// Note: usd_amount needs to have the same amount of decimals as PLMC, so when multiplied by the plmc-usd price, it gives us the PLMC amount with the decimals we wanted.
	#[transactional]
	pub fn do_evaluate(
		evaluator: &AccountIdOf<T>,
		project_id: ProjectId,
		usd_amount: BalanceOf<T>,
		did: Did,
		whitelisted_policy: Cid,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = NextEvaluationId::<T>::get();
		let plmc_usd_price = T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.ok_or(Error::<T>::PriceNotFound)?;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
		let evaluation_round_info = &mut project_details.evaluation_round_info;
		let total_evaluations_count = EvaluationCounts::<T>::get(project_id);
		let user_evaluations_count = Evaluations::<T>::iter_prefix((project_id, evaluator)).count() as u32;
		let project_policy = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;

		// * Validity Checks *
		ensure!(project_policy == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(usd_amount >= T::MinUsdPerEvaluation::get(), Error::<T>::TooLow);
		ensure!(project_details.issuer_did != did, Error::<T>::ParticipationToOwnProject);
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::IncorrectRound);
		ensure!(total_evaluations_count < T::MaxEvaluationsPerProject::get(), Error::<T>::TooManyProjectParticipations);
		ensure!(user_evaluations_count < T::MaxEvaluationsPerUser::get(), Error::<T>::TooManyUserParticipations);

		let plmc_bond = plmc_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_amount)
			.ok_or(Error::<T>::BadMath)?;
		let previous_total_evaluation_bonded_usd = evaluation_round_info.total_bonded_usd;

		let remaining_bond_to_reach_threshold =
			early_evaluation_reward_threshold_usd.saturating_sub(previous_total_evaluation_bonded_usd);

		let early_usd_amount = if usd_amount <= remaining_bond_to_reach_threshold {
			usd_amount
		} else {
			remaining_bond_to_reach_threshold
		};

		let late_usd_amount = usd_amount.checked_sub(&early_usd_amount).ok_or(Error::<T>::BadMath)?;

		let new_evaluation = EvaluationInfoOf::<T> {
			id: evaluation_id,
			did: did.clone(),
			project_id,
			evaluator: evaluator.clone(),
			original_plmc_bond: plmc_bond,
			current_plmc_bond: plmc_bond,
			early_usd_amount,
			late_usd_amount,
			when: now,
		};

		T::NativeCurrency::hold(&HoldReason::Evaluation.into(), evaluator, plmc_bond)?; // TODO: Check the `Reason`
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), new_evaluation);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));
		evaluation_round_info.total_bonded_usd += usd_amount;
		evaluation_round_info.total_bonded_plmc += plmc_bond;
		ProjectsDetails::<T>::insert(project_id, project_details);
		EvaluationCounts::<T>::mutate(project_id, |c| *c += 1);

		// * Emit events *
		Self::deposit_event(Event::Evaluation {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			plmc_amount: plmc_bond,
		});

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::evaluate(user_evaluations_count)),
			pays_fee: Pays::Yes,
		})
	}
}
