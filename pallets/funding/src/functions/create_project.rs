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
	/// Creates a project and assigns it to the `issuer` account.
	///
	/// # Arguments
	/// * `issuer` - The account that will be the issuer of the project.
	/// * `project` - The project struct containing all the necessary information.
	///
	/// # Storage access
	/// * [`ProjectsMetadata`] - Inserting the main project information. 1 to 1 with the `project` argument.
	/// * [`ProjectsDetails`] - Inserting the project information. constructed from the `project` argument.
	/// * [`ProjectsIssuers`] - Inserting the issuer of the project. Mapping of the two parameters `project_id` and `issuer`.
	/// * [`NextProjectId`] - Getting the next usable id, and updating it for the next project.
	///
	/// # Success path
	/// The `project` argument is valid. A ProjectInfo struct is constructed, and the storage is updated
	/// with the new structs and mappings to reflect the new project creation
	///
	/// # Next step
	/// The issuer will call an extrinsic to start the evaluation round of the project.
	/// [`do_evaluation_start`](Self::do_evaluation_start) will be executed.
	pub fn do_create(issuer: &AccountIdOf<T>, initial_metadata: ProjectMetadataOf<T>) -> DispatchResult {
		// * Get variables *
		let project_id = Self::next_project_id();

		// * Validity checks *
		if let Some(metadata) = initial_metadata.offchain_information_hash {
			ensure!(!Images::<T>::contains_key(metadata), Error::<T>::MetadataAlreadyExists);
		}

		if let Err(error) = initial_metadata.validity_check() {
			return match error {
				ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
				ValidityError::ParticipantsSizeError => Err(Error::<T>::ParticipantsSizeError.into()),
				ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
			}
		}
		let total_allocation_size =
			initial_metadata.total_allocation_size.0.saturating_add(initial_metadata.total_allocation_size.1);

		// * Calculate new variables *
		let fundraising_target =
			initial_metadata.minimum_price.checked_mul_int(total_allocation_size).ok_or(Error::<T>::BadMath)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let project_details = ProjectDetails {
			issuer: issuer.clone(),
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints::new(now),
			remaining_contribution_tokens: initial_metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<T>::zero(),
			cleanup: Cleaner::NotReady,
			evaluation_round_info: EvaluationRoundInfoOf::<T> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
			parachain_id: None,
			migration_readiness_check: None,
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Closed,
				polimec_to_project: ChannelStatus::Closed,
			},
		};

		let bucket: BucketOf<T> = Self::create_bucket_from_metadata(&initial_metadata)?;
		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, &initial_metadata);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());
		if let Some(metadata) = initial_metadata.offchain_information_hash {
			Images::<T>::insert(metadata, issuer);
		}

		// * Emit events *
		Self::deposit_event(Event::ProjectCreated { project_id, issuer: issuer.clone() });

		Ok(())
	}

    /// Change the metadata hash of a project
	///
	/// # Arguments
	/// * `issuer` - The project issuer account
	/// * `project_id` - The project identifier
	/// * `project_metadata_hash` - The hash of the image that contains the metadata
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the issuer is the owner of the project
	/// * [`Images`] - Check that the image exists
	/// * [`ProjectsDetails`] - Check that the project is not frozen
	/// * [`ProjectsMetadata`] - Update the metadata hash
	pub fn do_edit_metadata(
		issuer: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		project_metadata_hash: T::Hash,
	) -> DispatchResult {
		// * Get variables *
		let mut project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer == issuer, Error::<T>::NotAllowed);
		ensure!(!project_details.is_frozen, Error::<T>::Frozen);
		ensure!(!Images::<T>::contains_key(project_metadata_hash), Error::<T>::MetadataAlreadyExists);

		// * Calculate new variables *

		// * Update Storage *
		project_metadata.offchain_information_hash = Some(project_metadata_hash);
		ProjectsMetadata::<T>::insert(project_id, project_metadata);

		// * Emit events *
		Self::deposit_event(Event::MetadataEdited { project_id });

		Ok(())
	}


    pub fn create_bucket_from_metadata(metadata: &ProjectMetadataOf<T>) -> Result<BucketOf<T>, DispatchError> {
		let bucket_delta_amount = Percent::from_percent(10) * metadata.total_allocation_size.0;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).ok_or(Error::<T>::BadMath)?;
		let bucket_delta_price: <T as Config>::Price = metadata.minimum_price.saturating_mul(ten_percent_in_price);

		let bucket: BucketOf<T> = Bucket::new(
			metadata.total_allocation_size.0,
			metadata.minimum_price,
			bucket_delta_price,
			bucket_delta_amount,
		);

		Ok(bucket)
	}
}