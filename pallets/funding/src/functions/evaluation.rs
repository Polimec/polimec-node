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

// If you feel like getting in touch with us, you can do so at info@polimec.org
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
	/// * [`ProjectsToUpdate`] - Scheduling the project for automatic transition by on_initialize later on.
	///
	/// # Success path
	/// The project information is found, its round status was in Application round, and It's not yet frozen.
	/// The pertinent project info is updated on the storage, and the project is scheduled for automatic transition by on_initialize.
	///
	/// # Next step
	/// Users will pond PLMC for this project, and when the time comes, the project will be transitioned
	/// to the next round by `on_initialize` using [`do_evaluation_end`](Self::do_evaluation_end)
	pub fn do_evaluation_start(caller: AccountIdOf<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer == caller, Error::<T>::NotAllowed);
		ensure!(project_details.status == ProjectStatus::Application, Error::<T>::ProjectNotInApplicationRound);
		ensure!(!project_details.is_frozen, Error::<T>::ProjectAlreadyFrozen);
		ensure!(project_metadata.offchain_information_hash.is_some(), Error::<T>::MetadataNotProvided);

		// * Calculate new variables *
		let evaluation_end_block = now + T::EvaluationDuration::get();
		project_details.phase_transition_points.application.update(None, Some(now));
		project_details.phase_transition_points.evaluation.update(Some(now + 1u32.into()), Some(evaluation_end_block));
		project_details.is_frozen = true;
		project_details.status = ProjectStatus::EvaluationRound;

		// * Update storage *
		ProjectsDetails::<T>::insert(project_id, project_details);
		Self::add_to_update_store(evaluation_end_block + 1u32.into(), (&project_id, UpdateType::EvaluationEnd));

		// * Emit events *
		Self::deposit_event(Event::EvaluationStarted { project_id });

		Ok(())
	}

	/// Called automatically by on_initialize.
	/// Ends the evaluation round, and sets the current round to `AuctionInitializePeriod` if it
	/// reached enough PLMC bonding, or to `EvaluationFailed` if it didn't.
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
	/// through bonding, so the project is transitioned to the `EvaluationFailed` round. The project
	/// information is updated with the new rounds status and it is scheduled for automatic unbonding.
	///
	/// # Next step
	/// * Bonding achieved - The issuer calls an extrinsic within the set period to initialize the
	/// auction round. `auction` is called
	///
	/// * Bonding failed - `on_idle` at some point checks for failed evaluation projects, and
	/// unbonds the evaluators funds.
	pub fn do_evaluation_end(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_end_block =
			project_details.phase_transition_points.evaluation.end().ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target_usd = project_details.fundraising_target;
		let current_plmc_price =
			T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::ProjectNotInEvaluationRound);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// * Calculate new variables *
		let initial_balance: BalanceOf<T> = 0u32.into();
		let total_amount_bonded = Evaluations::<T>::iter_prefix((project_id,))
			.fold(initial_balance, |total, (_evaluator, bond)| total.saturating_add(bond.original_plmc_bond));

		let evaluation_target_usd = <T as Config>::EvaluationSuccessThreshold::get() * fundraising_target_usd;
		let evaluation_target_plmc = current_plmc_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(evaluation_target_usd)
			.ok_or(Error::<T>::BadMath)?;

		let auction_initialize_period_start_block = now + 1u32.into();
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block + T::AuctionInitializePeriodDuration::get();

		// Check which logic path to follow
		let is_funded = total_amount_bonded >= evaluation_target_plmc;

		// * Branch in possible project paths *
		// Successful path
		if is_funded {
			// * Update storage *
			project_details
				.phase_transition_points
				.auction_initialize_period
				.update(Some(auction_initialize_period_start_block), Some(auction_initialize_period_end_block));
			project_details.status = ProjectStatus::AuctionInitializePeriod;
			ProjectsDetails::<T>::insert(project_id, project_details);
			Self::add_to_update_store(
				auction_initialize_period_end_block + 1u32.into(),
				(&project_id, UpdateType::EnglishAuctionStart),
			);

			// * Emit events *
			Self::deposit_event(Event::AuctionInitializePeriod {
				project_id,
				start_block: auction_initialize_period_start_block,
				end_block: auction_initialize_period_end_block,
			});

		// Unsuccessful path
		} else {
			// * Update storage *
			project_details.status = ProjectStatus::EvaluationFailed;
			project_details.cleanup = Cleaner::Failure(CleanerState::Initialized(PhantomData::<Failure>));
			ProjectsDetails::<T>::insert(project_id, project_details);

			// * Emit events *
			Self::deposit_event(Event::EvaluationFailed { project_id });
		}

		Ok(())
	}

    // Note: usd_amount needs to have the same amount of decimals as PLMC, so when multiplied by the plmc-usd price, it gives us the PLMC amount with the decimals we wanted.
	pub fn do_evaluate(
		evaluator: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		usd_amount: BalanceOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = Self::next_evaluation_id();
		let caller_existing_evaluations: Vec<(u32, EvaluationInfoOf<T>)> =
			Evaluations::<T>::iter_prefix((project_id, evaluator)).collect();
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target;
		let evaluation_round_info = &mut project_details.evaluation_round_info;

		// * Validity Checks *
		ensure!(evaluator.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::EvaluationNotStarted);

		// * Calculate new variables *
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
			project_id,
			evaluator: evaluator.clone(),
			original_plmc_bond: plmc_bond,
			current_plmc_bond: plmc_bond,
			early_usd_amount,
			late_usd_amount,
			when: now,
			rewarded_or_slashed: None,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		// * Update Storage *
		if caller_existing_evaluations.len() < T::MaxEvaluationsPerUser::get() as usize {
			T::NativeCurrency::hold(&LockType::Evaluation(project_id), evaluator, plmc_bond)?;
		} else {
			let (low_id, lowest_evaluation) = caller_existing_evaluations
				.iter()
				.min_by_key(|(_, evaluation)| evaluation.original_plmc_bond)
				.ok_or(Error::<T>::ImpossibleState)?;

			ensure!(lowest_evaluation.original_plmc_bond < plmc_bond, Error::<T>::EvaluationBondTooLow);
			ensure!(
				lowest_evaluation.original_plmc_bond == lowest_evaluation.current_plmc_bond,
				"Using evaluation funds for participating should not be possible in the evaluation round"
			);

			T::NativeCurrency::release(
				&LockType::Evaluation(project_id),
				&lowest_evaluation.evaluator,
				lowest_evaluation.original_plmc_bond,
				Precision::Exact,
			)?;

			T::NativeCurrency::hold(&LockType::Evaluation(project_id), evaluator, plmc_bond)?;

			Evaluations::<T>::remove((project_id, evaluator, low_id));
		}

		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), new_evaluation);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));
		evaluation_round_info.total_bonded_usd += usd_amount;
		evaluation_round_info.total_bonded_plmc += plmc_bond;
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::FundsBonded { project_id, amount: plmc_bond, bonder: evaluator.clone() });

		Ok(())
	}
}