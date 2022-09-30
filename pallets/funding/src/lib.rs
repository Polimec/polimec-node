#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
pub use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type NumberOfCurrencies: Get<u32>;

		#[pallet::constant]
		type NumberOfProjects: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn project_of)]
	/// Metadata of a Project.
	pub type ProjectsOf<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ProjectMetadata<T::AccountId>, T::NumberOfProjects>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProjectCreated(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		MetadataError,
		MetadataErrorNotEnoughParticipationCurrencies,
		MetadaraErrorNotEnoughParticipants,
		TooManyProjects,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn create(
			origin: OriginFor<T>,
			project_metadata: ProjectMetadata<T::AccountId>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let issuer = ensure_signed(origin)?;

			match project_metadata.validity_check() {
				Ok(()) => Self::do_create(issuer, project_metadata),
				Err(error) => match error {
					ValidityError::NotEnoughParticipationCurrencies =>
						Err(Error::<T>::MetadataErrorNotEnoughParticipationCurrencies.into()),
					ValidityError::NotEnoughParticipants =>
						Err(Error::<T>::MetadaraErrorNotEnoughParticipants.into()),
				},
			}
		}
	}
}

use frame_support::{pallet_prelude::DispatchError, BoundedVec};

impl<T: Config> Pallet<T> {
	pub fn do_create(
		who: T::AccountId,
		project_metadata: ProjectMetadata<T::AccountId>,
	) -> Result<(), DispatchError> {
		if let Some(mut alredy_existing_projects) = ProjectsOf::<T>::get(&who) {
			alredy_existing_projects
				.try_push(project_metadata)
				.map_err(|_| Error::<T>::TooManyProjects)?;
			ProjectsOf::<T>::insert(&who, alredy_existing_projects);
		} else {
			let mut new_projects = BoundedVec::with_bounded_capacity(4);
			// TODO: This `try_push` never fails
			new_projects
				.try_push(project_metadata)
				.map_err(|_| Error::<T>::TooManyProjects)?;
			ProjectsOf::<T>::insert(&who, new_projects)
		}
		Self::deposit_event(Event::<T>::ProjectCreated(who));
		Ok(())
	}
}
