#![allow(clippy::wildcard_imports)]
#![allow(clippy::type_complexity)]

use super::*;

impl<T: Config> Pallet<T> {
	fn project_validation(
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		did: Did,
	) -> Result<(ProjectMetadataOf<T>, ProjectDetailsOf<T>, BucketOf<T>), DispatchError> {
		if let Err(error) = project_metadata.is_valid() {
			let pallet_error = match error {
				MetadataError::PriceTooLow => Error::<T>::PriceTooLow,
				MetadataError::TicketSizeError => Error::<T>::TicketSizeError,
				MetadataError::ParticipationCurrenciesError => Error::<T>::ParticipationCurrenciesError,
				MetadataError::AllocationSizeError => Error::<T>::AllocationSizeError,
				MetadataError::AuctionRoundPercentageError => Error::<T>::AuctionRoundPercentageError,
				MetadataError::FundingTargetTooLow => Error::<T>::FundingTargetTooLow,
				MetadataError::FundingTargetTooHigh => Error::<T>::FundingTargetTooHigh,
				MetadataError::CidNotProvided => Error::<T>::CidNotProvided,
				MetadataError::BadDecimals => Error::<T>::BadDecimals,
				MetadataError::BadTokenomics => Error::<T>::BadTokenomics,
			};
			return Err(pallet_error.into());
		}
		let total_allocation_size = project_metadata.total_allocation_size;

		let fundraising_target =
			project_metadata.minimum_price.checked_mul_int(total_allocation_size).ok_or(Error::<T>::BadMath)?;

		let project_details = ProjectDetails {
			issuer_account: issuer.clone(),
			issuer_did: did.clone(),
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target_usd: fundraising_target,
			status: ProjectStatus::Application,
			round_duration: BlockNumberPair::new(None, None),
			remaining_contribution_tokens: project_metadata.total_allocation_size,
			funding_amount_reached_usd: Balance::zero(),
			evaluation_round_info: EvaluationRoundInfo {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: None,
			},
			usd_bid_on_oversubscription: None,
			funding_end_block: None,
			migration_type: None,
		};

		let bucket: BucketOf<T> = Self::create_bucket_from_metadata(&project_metadata)?;

		Ok((project_metadata, project_details, bucket))
	}

	#[transactional]
	pub fn do_create_project(
		issuer: &AccountIdOf<T>,
		project_metadata: ProjectMetadataOf<T>,
		did: Did,
	) -> DispatchResult {
		// * Get variables *
		let project_id = NextProjectId::<T>::get();
		let maybe_active_project = DidWithActiveProjects::<T>::get(did.clone());

		// * Validity checks *
		ensure!(maybe_active_project.is_none(), Error::<T>::HasActiveProject);

		let (project_metadata, project_details, bucket) =
			Self::project_validation(project_metadata, issuer.clone(), did.clone())?;

		// Each project needs an escrow system account to temporarily hold the USDT/USDC. We need to create it by depositing `ED` amount of PLMC into it.
		// This should be paid by the issuer.
		let escrow_account = Self::fund_account_id(project_id);
		// transfer ED from issuer to escrow
		T::NativeCurrency::transfer(
			issuer,
			&escrow_account,
			<T as pallet_balances::Config>::ExistentialDeposit::get(),
			Preservation::Preserve,
		)
		.map_err(|_| Error::<T>::IssuerNotEnoughFunds)?;

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, project_metadata.clone());
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());
		DidWithActiveProjects::<T>::set(did, Some(project_id));

		// * Emit events *
		Self::deposit_event(Event::ProjectCreated { project_id, issuer: issuer.clone(), metadata: project_metadata });

		Ok(())
	}

	#[transactional]
	pub fn do_edit_project(
		issuer: AccountIdOf<T>,
		project_id: ProjectId,
		new_project_metadata: ProjectMetadataOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer_account == issuer, Error::<T>::NotIssuer);
		ensure!(!project_details.is_frozen, Error::<T>::ProjectIsFrozen);

		// * Calculate new variables *
		let (new_project_metadata, project_details, bucket) =
			Self::project_validation(new_project_metadata, issuer.clone(), project_details.issuer_did.clone())?;

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, new_project_metadata.clone());
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);

		// * Emit events *
		Self::deposit_event(Event::MetadataEdited { project_id, metadata: new_project_metadata });

		Ok(())
	}

	#[transactional]
	pub fn do_remove_project(issuer: AccountIdOf<T>, project_id: ProjectId, did: Did) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer_account == issuer, Error::<T>::NotIssuer);
		ensure!(project_details.is_frozen.not(), Error::<T>::ProjectIsFrozen);

		// * Update storage *
		ProjectsDetails::<T>::remove(project_id);
		ProjectsMetadata::<T>::remove(project_id);
		DidWithActiveProjects::<T>::set(did, None);
		Buckets::<T>::remove(project_id);

		// * Emit events *
		Self::deposit_event(Event::ProjectRemoved { project_id, issuer });

		Ok(())
	}
}
