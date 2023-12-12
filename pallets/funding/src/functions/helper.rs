
use super::*;
// Helper functions
impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: T::ProjectIdentifier) -> AccountIdOf<T> {
		T::PalletId::get().into_sub_account_truncating(index)
	}

	/// Adds a project to the ProjectsToUpdate storage, so it can be updated at some later point in time.
	pub fn add_to_update_store(block_number: BlockNumberFor<T>, store: (&T::ProjectIdentifier, UpdateType)) {
		// Try to get the project into the earliest possible block to update.
		// There is a limit for how many projects can update each block, so we need to make sure we don't exceed that limit
		let mut block_number = block_number;
		while ProjectsToUpdate::<T>::try_append(block_number, store.clone()).is_err() {
			// TODO: Should we end the loop if we iterated over too many blocks?
			block_number += 1u32.into();
		}
	}

	pub fn remove_from_update_store(project_id: &T::ProjectIdentifier) -> DispatchResult {
		let (block_position, project_index) = ProjectsToUpdate::<T>::iter()
			.find_map(|(block, project_vec)| {
				let project_index = project_vec.iter().position(|(id, _update_type)| id == project_id)?;
				Some((block, project_index))
			})
			.ok_or(Error::<T>::ProjectNotInUpdateStore)?;

		ProjectsToUpdate::<T>::mutate(block_position, |project_vec| {
			project_vec.remove(project_index);
		});

		Ok(())
	}

	pub fn calculate_plmc_bond(
		ticket_size: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		plmc_price: PriceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let usd_bond = multiplier.calculate_bonding_requirement::<T>(ticket_size).map_err(|_| Error::<T>::BadMath)?;
		plmc_price.reciprocal().ok_or(Error::<T>::BadMath)?.checked_mul_int(usd_bond).ok_or(Error::<T>::BadMath.into())
	}

	pub fn try_plmc_participation_lock(
		who: &T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		// Check if the user has already locked tokens in the evaluation period
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, who));

		let mut to_convert = amount;
		for mut evaluation in user_evaluations {
			if to_convert == Zero::zero() {
				break
			}
			let slash_deposit = <T as Config>::EvaluatorSlash::get() * evaluation.original_plmc_bond;
			let available_to_convert = evaluation.current_plmc_bond.saturating_sub(slash_deposit);
			let converted = to_convert.min(available_to_convert);
			evaluation.current_plmc_bond = evaluation.current_plmc_bond.saturating_sub(converted);
			Evaluations::<T>::insert((project_id, who, evaluation.id), evaluation);
			T::NativeCurrency::release(&LockType::Evaluation(project_id), who, converted, Precision::Exact)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&LockType::Participation(project_id), who, converted)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&LockType::Participation(project_id), who, to_convert)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
		asset_id: AssetIdOf<T>,
	) -> DispatchResult {
		let fund_account = Self::fund_account_id(project_id);

		T::FundingCurrency::transfer(asset_id, who, &fund_account, amount, Preservation::Expendable)?;

		Ok(())
	}

	pub fn make_project_funding_successful(
		project_id: T::ProjectIdentifier,
		mut project_details: ProjectDetailsOf<T>,
		reason: SuccessReason,
		settlement_delta: BlockNumberFor<T>,
	) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingSuccessful;
		ProjectsDetails::<T>::insert(project_id, project_details);

		Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement));

		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Success(reason) });

		Ok(())
	}

	pub fn make_project_funding_fail(
		project_id: T::ProjectIdentifier,
		mut project_details: ProjectDetailsOf<T>,
		reason: FailureReason,
		settlement_delta: BlockNumberFor<T>,
	) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingFailed;
		ProjectsDetails::<T>::insert(project_id, project_details);

		Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement));
		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Failure(reason) });
		Ok(())
	}

	

	pub fn construct_migration_xcm_messages(migrations: Migrations) -> Vec<(Migrations, Xcm<()>)> {
		// TODO: adjust this as benchmarks for polimec-receiver are written
		const MAX_WEIGHT: Weight = Weight::from_parts(10_000, 0);

		// const MAX_WEIGHT: Weight = Weight::from_parts(100_003_000_000_000, 10_000_196_608);
		let _polimec_receiver_info = T::PolimecReceiverInfo::get();
		// TODO: use the actual pallet index when the fields are not private anymore (https://github.com/paritytech/polkadot-sdk/pull/2231)
		let mut output = Vec::new();

		for migrations_slice in migrations.inner().chunks(MaxMigrationsPerXcm::<T>::get() as usize) {
			let migrations_vec = migrations_slice.to_vec();
			let migrations_item = Migrations::from(migrations_vec);

			let mut encoded_call = vec![51u8, 0];
			encoded_call.extend_from_slice(migrations_item.encode().as_slice());
			let xcm: Xcm<()> = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Native,
					require_weight_at_most: MAX_WEIGHT,
					call: encoded_call.into(),
				},
				// ReportTransactStatus should be appended here after knowing the query_id
			]);

			output.push((migrations_item, xcm));
		}

		// TODO: we probably want to ensure we dont build too many messages to overflow the queue. Which we know from the parameter `T::RequiredMaxCapacity`.
		// the problem is that we don't know the existing messages in the destination queue. So for now we assume all messages will succeed
		output
	}
}
