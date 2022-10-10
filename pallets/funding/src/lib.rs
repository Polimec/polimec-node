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

		/// Identifier for the collection of item.
		type ProjectId: Member + Parameter + MaxEncodedLen + Copy;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn projects_of)]
	/// Metadata of a Project.
	/// TODO: Change to DoubleStorageMap { k1: T::AccountID, k2: T::SomeProjectID, v: Project }
	/// In this way we can remove the NumberOfProjects
	pub type ProjectsOf<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::ProjectId,
		(Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>, EvaluationStatus),
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProjectCreated(T::ProjectId, T::AccountId),
		EvaluationStarted(T::ProjectId, T::AccountId),
	}

	#[pallet::error]
	pub enum Error<T> {
		PriceTooLow,
		TooManyProjects,
		ParticipantsSizeError,
		TicketSizeError,
		ProjectIdInUse,
		ProjectNotExists,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn create(
			origin: OriginFor<T>,
			project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>,
			project_id: T::ProjectId,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let issuer = ensure_signed(origin)?;

			match project.validity_check() {
				Err(error) => match error {
					ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
					ValidityError::ParticipantsSizeError =>
						Err(Error::<T>::ParticipantsSizeError.into()),
					ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
				},
				Ok(()) => {
					ensure!(
						!ProjectsOf::<T>::contains_key(issuer.clone(), project_id),
						Error::<T>::ProjectIdInUse
					);
					Self::do_create(issuer, project, project_id)
				},
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn start_evaluation(origin: OriginFor<T>, project_id: T::ProjectId) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(
				ProjectsOf::<T>::contains_key(issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			Self::do_start_evaluation(issuer, project_id)
		}
	}
}

use frame_support::{pallet_prelude::DispatchError, BoundedVec};

impl<T: Config> Pallet<T> {
	pub fn do_create(
		who: T::AccountId,
		project_info: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>,
		project_id: T::ProjectId,
	) -> Result<(), DispatchError> {
		// When a project is created the evaluation phase doesn't start automatically
		ProjectsOf::<T>::insert(
			who.clone(),
			project_id,
			(project_info, EvaluationStatus::NotYetStarted),
		);
		Self::deposit_event(Event::<T>::ProjectCreated(project_id, who));
		Ok(())
	}

	pub fn do_start_evaluation(
		who: T::AccountId,
		project_id: T::ProjectId,
	) -> Result<(), DispatchError> {
		ProjectsOf::<T>::try_mutate(who.clone(), project_id, |maybe_project| {
			let (_, evaluation_status) =
				maybe_project.as_mut().ok_or(Error::<T>::ProjectNotExists)?;
			*evaluation_status = EvaluationStatus::Started;
			Self::deposit_event(Event::<T>::EvaluationStarted(project_id, who));
			Ok(())
		})
	}
}
