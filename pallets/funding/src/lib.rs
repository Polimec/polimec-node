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

use frame_support::traits::{Get, LockIdentifier, LockableCurrency, WithdrawReasons};
// use sp_runtime::traits::CheckedAdd;

/// The balance type of this pallet.
pub type BalanceOf<T> = <T as Config>::CurrencyBalance;

const LOCKING_ID: LockIdentifier = *b"evaluate";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::{ValueQuery, *};
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

		/// The bonding balance.
		type Currency: LockableCurrency<
			Self::AccountId,
			Moment = Self::BlockNumber,
			Balance = Self::CurrencyBalance,
		>;

		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to
		/// `From<u64>`.
		type CurrencyBalance: sp_runtime::traits::AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u64>
			+ TypeInfo
			+ MaxEncodedLen;

		#[pallet::constant]
		type EvaluationDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type AuctionDuration: Get<Self::BlockNumber>;
	}

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// Information of a Project.
	pub type Projects<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::ProjectId,
		Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn evaluations)]
	/// Information of a Project.
	pub type Evaluations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::ProjectId,
		//TODO: Use EvaluationMetadata<T>
		EvaluationMetadata,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ProjectCreated(T::ProjectId, T::AccountId),
		EvaluationStarted(T::ProjectId, T::AccountId),
		// FundsBonded(T::ProjectId, T::AccountId, T::AccountId, BalanceOf<T>),
		FundsBonded(T::ProjectId, T::AccountId, T::AccountId, u64),
	}

	#[pallet::error]
	pub enum Error<T> {
		PriceTooLow,
		ParticipantsSizeError,
		TicketSizeError,
		ProjectIdInUse,
		ProjectNotExists,
		EvaluationAlreadyStarted,
		ContributionToThemself,
		EvaluationNotStarted,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn create(
			origin: OriginFor<T>,
			project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>,
			// TODO: Check if the "project_id" logic is correct.
			// from an UX PoV can this be a problem? Is there a better way to do it?
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
						!Projects::<T>::contains_key(issuer.clone(), project_id),
						Error::<T>::ProjectIdInUse
					);
					Self::do_create(issuer, project, project_id)
				},
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		// Set the evaluation status to EvaluationStatus::Started
		// TODO: Is it better to create a second StorageMap to store ONLY the projects in the
		// evaluation phase?
		pub fn start_evaluation(origin: OriginFor<T>, project_id: T::ProjectId) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(
				Projects::<T>::contains_key(issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			ensure!(
				Evaluations::<T>::get(issuer.clone(), project_id).evaluation_status ==
					EvaluationStatus::NotYetStarted,
				Error::<T>::EvaluationAlreadyStarted
			);
			Self::do_start_evaluation(issuer, project_id)
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn bond(
			origin: OriginFor<T>,
			project_issuer: T::AccountId,
			project_id: T::ProjectId,
			// #[pallet::compact] amount: BalanceOf<T>,
			amount: u64,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			ensure!(
				Projects::<T>::contains_key(project_issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			ensure!(from != project_issuer, Error::<T>::ContributionToThemself);
			ensure!(
				Evaluations::<T>::get(project_issuer.clone(), project_id).evaluation_status ==
					EvaluationStatus::Started,
				Error::<T>::EvaluationNotStarted
			);
			T::Currency::set_lock(LOCKING_ID, &from, amount.into(), WithdrawReasons::all());
			Evaluations::<T>::mutate(project_issuer.clone(), project_id, |project| {
				project.amount_bonded =
					project.amount_bonded.checked_add(amount).unwrap_or(project.amount_bonded)
			});
			Self::deposit_event(Event::<T>::FundsBonded(project_id, project_issuer, from, amount));
			Ok(())
		}
	}
}

use frame_support::{pallet_prelude::DispatchError, BoundedVec};
// use sp_runtime::traits::Zero;

impl<T: Config> Pallet<T> {
	pub fn do_create(
		who: T::AccountId,
		project_info: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, T::BlockNumber>,
		project_id: T::ProjectId,
	) -> Result<(), DispatchError> {
		Projects::<T>::insert(who.clone(), project_id, project_info);
		// When a project is created the evaluation phase doesn't start automatically
		let evaluation_metadata = EvaluationMetadata {
			evaluation_status: EvaluationStatus::NotYetStarted,
			// evaluation_period_ends: NOW + T::EvaluationDuration::get(),
			evaluation_period_ends: 100,
			amount_bonded: 0,
		};
		Evaluations::<T>::insert(who.clone(), project_id, evaluation_metadata);
		Self::deposit_event(Event::<T>::ProjectCreated(project_id, who));
		Ok(())
	}

	pub fn do_start_evaluation(
		who: T::AccountId,
		project_id: T::ProjectId,
	) -> Result<(), DispatchError> {
		Evaluations::<T>::try_mutate(who.clone(), project_id, |project| {
			project.evaluation_status = EvaluationStatus::Started;
			Self::deposit_event(Event::<T>::EvaluationStarted(project_id, who));
			Ok(())
		})
	}
}
