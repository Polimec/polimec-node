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
	}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Projects<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ProjectMetadata<T::AccountId>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProjectCreated(T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		MetadataError,
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

			ensure!(project_metadata.is_valid(), Error::<T>::MetadataError);

			Self::do_create(issuer, project_metadata)
		}
	}
}

use frame_support::pallet_prelude::DispatchError;

impl<T: Config> Pallet<T> {
	pub fn do_create(
		who: T::AccountId,
		_project_metadata: ProjectMetadata<T::AccountId>,
	) -> Result<(), DispatchError> {
		Self::deposit_event(Event::<T>::ProjectCreated(who));
		Ok(())
	}
}
