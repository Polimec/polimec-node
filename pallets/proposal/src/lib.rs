#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::DispatchResultWithPostInfo;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::{
			DispatchResult, DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo,
			PostDispatchInfo,
		},
		ensure,
		pallet_prelude::{ValueQuery, Weight, *},
		sp_runtime::traits::Hash,
		traits::Get,
		Parameter,
	};
	use frame_system::pallet_prelude::*;

	/// Simple index type for proposal counting.
	pub type ProposalIndex = u32;

	// TODO: why PartialEq?
	// More about TypeInfo trait: https://github.com/paritytech/scale-info
	#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, TypeInfo)]
	pub struct Proposal<Call, Meta> {
		pub call: Box<Call>,
		pub metadata: Meta,
	}

	// TODO: Remove `#[pallet::without_storage_info]` and implement MaxEncodedLen for `Proposal`
	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// The pallet's configuration trait.
	///
	/// Uses two types of Call:
	/// 1. `Call<Self>` calls defined in here (pallet_proposal)
	/// 2. `Self::Call` for runtime calls, will contain a
	/// Self::Call::ProposalModuleName(Call<Self>) variant for extrinsics defined here
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Maximum number of proposals allowed to be active in parallel.
		type ProposalMetadata: Parameter + PartialEq + Default;

		/// Maximum number of proposals allowed to be active in parallel.
		type MaxProposals: Get<ProposalIndex>;

		/// The outer call dispatch type.
		type Call: Parameter
			+ Dispatchable<
				Origin = <Self as frame_system::Config>::Origin,
				PostInfo = PostDispatchInfo,
			> + From<frame_system::Call<Self>>
			+ GetDispatchInfo;

		// TODO: Add weights
		// Weight information for extrinsics in this pallet.
		type WeightInfo: frame_system::WeightInfo;
	}

	// The pallet's runtime storage items.
	#[pallet::storage]
	#[pallet::getter(fn proposal_count)]
	pub type ProposalCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_of)]
	pub type ProposalOf<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::Hash,
		Proposal<<T as Config>::Call, T::ProposalMetadata>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config> =
		StorageValue<_, BoundedVec<T::Hash, T::MaxProposals>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A motion was executed; result will be `Ok` if it returned without error.
		/// \[proposal_hash, result\]
		Executed(T::Hash, DispatchResult),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// There can only be a maximum of `MaxProposals` active proposals.
		TooManyProposals,
		/// The given weight bound for the proposal was too low.
		WrongProposalWeight,
		/// The given length bound for the proposal was too low.
		WrongProposalLength,
		/// Duplicate proposals not allowed
		DuplicateProposal,
		/// Proposal must exist
		ProposalMissing,
		/// Mismatched index
		WrongIndex,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn test(_origin: OriginFor<T>) -> DispatchResult {
			Ok(())
		}
	}

	type TraitProposal<T> = Proposal<<T as Config>::Call, <T as Config>::ProposalMetadata>;
	impl<T: Config> Pallet<T> {
		/// Ensure that the right proposal bounds were passed and get the proposal
		/// from storage.
		///
		/// Checks the length in storage via `storage::read` which adds an extra
		/// `size_of::<u32>() == 4` to the length.
		pub fn validate_and_get_proposal(
			hash: T::Hash,
			length_bound: u32,
			weight_bound: Weight,
		) -> Result<TraitProposal<T>, DispatchError> {
			let key = ProposalOf::<T>::hashed_key_for(hash);
			// read the length of the proposal storage entry directly

			// TODO: Check the proposal_len
			// TODO: Why we need to use sp_io::storage?

			// let proposal_len = storage::read(&key, &mut [0; 0],
			// 0).ok_or(Error::<T>::ProposalMissing)?; ensure!(proposal_len <= length_bound,
			// Error::<T>::WrongProposalLength);

			let proposal = ProposalOf::<T>::get(hash).ok_or(Error::<T>::ProposalMissing)?;

			let proposal_weight = proposal.call.get_dispatch_info().weight;
			ensure!(proposal_weight <= weight_bound, Error::<T>::WrongProposalWeight);

			Ok(proposal)
		}

		fn execute(proposal: TraitProposal<T>) -> DispatchResultWithPostInfo {
			let proposal_len = proposal.using_encoded(|x| x.len());

			let proposal_hash = T::Hashing::hash_of(&proposal);
			let result = proposal.call.dispatch(frame_system::RawOrigin::Root.into());
			Self::deposit_event(Event::Executed(
				proposal_hash,
				result.map(|_| ()).map_err(|e| e.error),
			));

			Ok(().into())
			// TODO: Explore and fix `T::WeightInfo::execute`

			// Ok(crate::get_result_weight(result)
			// 	.map(|w| {
			// 		T::WeightInfo::execute(
			// 			proposal_len as u32, // B
			// 		)
			// 		.saturating_add(w) // P
			// 	})
			// 	.into())
		}

		pub fn execute_proposal(
			hash: T::Hash,
			length_bound: u32,
			weight_bound: Weight,
		) -> DispatchResultWithPostInfo {
			let prop = Self::validate_and_get_proposal(hash, length_bound, weight_bound)?;
			Self::remove_proposal(hash);
			Self::execute(prop)
		}

		/// Adds a proposal to the active proposals.
		///
		/// The index of the proposal is relative to active and past proposals.
		/// Returns (Hash of proposal, number of active proposals, index of
		/// proposal)
		pub fn add_proposal(
			proposal: TraitProposal<T>,
			length_bound: u32,
		) -> Result<(T::Hash, usize, u32), DispatchError> {
			let proposal_len = proposal.using_encoded(|x| x.len());
			ensure!(proposal_len <= length_bound as usize, Error::<T>::WrongProposalLength);
			let proposal_hash = T::Hashing::hash_of(&proposal);
			ensure!(!<ProposalOf<T>>::contains_key(proposal_hash), Error::<T>::DuplicateProposal);

			if let Some(active_proposals) = <Proposals<T>>::decode_len() {
				ensure!(
					active_proposals < T::MaxProposals::get() as usize,
					Error::<T>::TooManyProposals
				);
				// TODO: There is a better way to append a value
				let _ = <Proposals<T>>::try_append(proposal_hash);
				let index = Self::proposal_count();
				// TODO: Use safe math
				<ProposalCount<T>>::mutate(|i| *i += 1);
				<ProposalOf<T>>::insert(proposal_hash, proposal);

				Ok((proposal_hash, active_proposals, index))
			} else {
				Err(Error::<T>::DuplicateProposal.into())
			}
		}

		/// Removes a proposal from the pallet, cleaning up the vector of proposals.
		///
		/// Returns the original length of the proposal vector for calculating the
		/// weight.
		pub fn remove_proposal(proposal_hash: T::Hash) -> u32 {
			// remove proposal and vote
			ProposalOf::<T>::remove(&proposal_hash);

			let num_proposals = Proposals::<T>::mutate(|proposals| {
				let orig_length = proposals.len();
				proposals.retain(|h| h != &proposal_hash);
				// calculate weight based on original length
				orig_length
			});
			num_proposals as u32
		}
	}
}

use frame_support::pallet_prelude::Weight;

/// Return the weight of a dispatch call result as an `Option`.
///
/// Will return the weight regardless of what the state of the result is.
fn get_result_weight(result: DispatchResultWithPostInfo) -> Option<Weight> {
	match result {
		Ok(post_info) => post_info.actual_weight,
		Err(err) => err.post_info.actual_weight,
	}
}
