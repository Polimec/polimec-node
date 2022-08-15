#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::{DispatchResult, DispatchResultWithPostInfo, Dispatchable, GetDispatchInfo, PostDispatchInfo},
	ensure,
	traits::Get,
	weights::Weight,
	Parameter,
};
use sp_io::storage;
use sp_runtime::{traits::Hash, DispatchError, RuntimeDebug};
use sp_std::{boxed::Box, prelude::Vec};

/// Simple index type for proposal counting.
pub type ProposalIndex = u32;

// TODO: why PartialEq?
#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq)]
pub struct Proposal<Call, Meta> {
	pub call: Box<Call>,
	pub metadata: Meta,
}

/// The pallet's configuration trait.
///
/// Uses two types of Call:
/// 1. `Call<Self>` calls defined in here (pallet_proposal)
/// 2. `Self::Call` for runtime calls, will contain a
/// Self::Call::ProposalModuleName(Call<Self>) variant for extrinsics defined
/// here
pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The outer call dispatch type.
	type Call: Parameter
		+ Dispatchable<Origin = <Self as frame_system::Config>::Origin, PostInfo = PostDispatchInfo>
		+ From<frame_system::Call<Self>>
		+ GetDispatchInfo;

	/// Maximum number of proposals allowed to be active in parallel.
	type ProposalMetadata: Parameter + PartialEq + Default;

	/// Maximum number of proposals allowed to be active in parallel.
	type MaxProposals: Get<ProposalIndex>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Config> as ProposalStore {
		/// The hashes of the active proposals.
		pub Proposals get(fn proposals): Vec<T::Hash>;
		/// Actual proposal for a given hash, if it's current.
		pub ProposalOf get(fn proposal_of):
			map hasher(identity) T::Hash => Option<Proposal<<T as Config>::Call, T::ProposalMetadata>>;
		/// Proposals so far.
		pub ProposalCount get(fn proposal_count): u32;
	}
}

// The pallet's events
decl_event!(
	pub enum Event<T> where
		<T as frame_system::Config>::Hash,
		// <T as frame_system::Config>::AccountId,
	{
		/// A motion was executed; result will be `Ok` if it returned without error.
		/// \[proposal_hash, result\]
		Executed(Hash, DispatchResult),
	}
);

// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Config> {
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
}

// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::Origin {

		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		fn deposit_event() = default;

	}
}

type TraitProposal<T> = Proposal<<T as Config>::Call, <T as Config>::ProposalMetadata>;

impl<T: Config> Module<T> {
	/// Ensure that the right proposal bounds were passed and get the proposal
	/// from storage.
	///
	/// Checks the length in storage via `storage::read` which adds an extra
	/// `size_of::<u32>() == 4` to the length.
	pub fn validate_and_get_proposal(
		hash: T::Hash,
		length_bound: u32,
		weight_bound: Weight,
	) -> Result<(TraitProposal<T>, usize), DispatchError> {
		let key = ProposalOf::<T>::hashed_key_for(hash);
		// read the length of the proposal storage entry directly
		let proposal_len = storage::read(&key, &mut [0; 0], 0).ok_or(Error::<T>::ProposalMissing)?;
		ensure!(proposal_len <= length_bound, Error::<T>::WrongProposalLength);

		let proposal = ProposalOf::<T>::get(hash).ok_or(Error::<T>::ProposalMissing)?;

		let proposal_weight = proposal.call.get_dispatch_info().weight;
		ensure!(proposal_weight <= weight_bound, Error::<T>::WrongProposalWeight);

		Ok((proposal, proposal_len as usize))
	}

	pub fn execute_proposal(hash: T::Hash, length_bound: u32, weight_bound: Weight) -> DispatchResultWithPostInfo {
		let (prop, _) = Self::validate_and_get_proposal(hash, length_bound, weight_bound)?;
		Self::remove_proposal(hash);
		Self::execute(prop)
	}

	fn execute(proposal: TraitProposal<T>) -> DispatchResultWithPostInfo {
		let proposal_len = proposal.using_encoded(|x| x.len());

		let proposal_hash = T::Hashing::hash_of(&proposal);
		let result = proposal.call.dispatch(frame_system::RawOrigin::Root.into());
		Self::deposit_event(RawEvent::Executed(
			proposal_hash,
			result.map(|_| ()).map_err(|e| e.error),
		));

		Ok(get_result_weight(result)
			.map(|w| {
				T::WeightInfo::execute(
					proposal_len as u32, // B
				)
				.saturating_add(w) // P
			})
			.into())
	}

	/// Adds a proposal to the active proposals.
	///
	/// The index of the proposal is relative to active and past proposals.
	/// Returns (Hash of proposal, number of active proposals, index of
	/// proposal)
	pub fn add_proposal(proposal: TraitProposal<T>, length_bound: u32) -> Result<(T::Hash, usize, u32), DispatchError> {
		let proposal_len = proposal.using_encoded(|x| x.len());
		ensure!(proposal_len <= length_bound as usize, Error::<T>::WrongProposalLength);
		let proposal_hash = T::Hashing::hash_of(&proposal);
		ensure!(
			!<ProposalOf<T>>::contains_key(proposal_hash),
			Error::<T>::DuplicateProposal
		);

		let active_proposals = <Proposals<T>>::try_mutate(|proposals| -> Result<usize, DispatchError> {
			proposals.push(proposal_hash);
			ensure!(
				proposals.len() <= T::MaxProposals::get() as usize,
				Error::<T>::TooManyProposals
			);
			Ok(proposals.len())
		})?;

		let index = Self::proposal_count();
		<ProposalCount>::mutate(|i| *i += 1);
		<ProposalOf<T>>::insert(proposal_hash, proposal);

		Ok((proposal_hash, active_proposals, index))
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

/// Return the weight of a dispatch call result as an `Option`.
///
/// Will return the weight regardless of what the state of the result is.
fn get_result_weight(result: DispatchResultWithPostInfo) -> Option<Weight> {
	match result {
		Ok(post_info) => post_info.actual_weight,
		Err(err) => err.post_info.actual_weight,
	}
}
