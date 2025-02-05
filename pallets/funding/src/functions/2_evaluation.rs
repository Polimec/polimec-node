#[allow(clippy::wildcard_imports)]
use super::*;
use polimec_common::ProvideAssetPrice;
impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_start_evaluation(caller: AccountIdOf<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
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
		)?;

		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
	}

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
			let mut project_ids = ProjectsInAuctionRound::<T>::get().to_vec();
			project_ids.push(project_id);
			let project_ids = WeakBoundedVec::force_from(project_ids, None);
			ProjectsInAuctionRound::<T>::put(project_ids);
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
				None,
				false,
			)
		}
	}

	#[transactional]
	pub fn do_evaluate(
		evaluator: &AccountIdOf<T>,
		project_id: ProjectId,
		usd_amount: Balance,
		did: Did,
		whitelisted_policy: Cid,
		receiving_account: Junction,
	) -> DispatchResult {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = NextEvaluationId::<T>::get();
		let plmc_usd_price =
			<PriceProviderOf<T>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, PLMC_DECIMALS)
				.ok_or(Error::<T>::PriceNotFound)?;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
		let evaluation_round_info = &mut project_details.evaluation_round_info;
		let project_policy = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity Checks *
		ensure!(project_policy == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(usd_amount >= T::MinUsdPerEvaluation::get(), Error::<T>::TooLow);
		ensure!(project_details.issuer_did != did, Error::<T>::ParticipationToOwnProject);
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::IncorrectRound);
		ensure!(
			project_details.round_duration.started(now) && !project_details.round_duration.ended(now),
			Error::<T>::IncorrectRound
		);
		ensure!(
			project_metadata.participants_account_type.junction_is_supported(&receiving_account),
			Error::<T>::UnsupportedReceiverAccountJunction
		);

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

		let late_usd_amount = usd_amount.checked_sub(early_usd_amount).ok_or(Error::<T>::BadMath)?;

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
			receiving_account,
		};

		T::NativeCurrency::hold(&HoldReason::Evaluation.into(), evaluator, plmc_bond)?;
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), new_evaluation);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));
		evaluation_round_info.total_bonded_usd = evaluation_round_info.total_bonded_usd.saturating_add(usd_amount);
		evaluation_round_info.total_bonded_plmc = evaluation_round_info.total_bonded_plmc.saturating_add(plmc_bond);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::Evaluation {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			plmc_amount: plmc_bond,
		});

		Ok(())
	}
}
