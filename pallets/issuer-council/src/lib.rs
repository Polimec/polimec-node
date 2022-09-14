#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode};
use orml_traits::arithmetic::Zero;
use sp_arithmetic::Perbill;
use scale_info::TypeInfo;

type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

type AmountOf<T> = <T as orml_tokens::Config>::Amount;

/// A number of members.
///
/// This also serves as a number of voting members, and since for motions, each
/// member may vote exactly once, therefore also the number of votes for any
/// given motion.
pub type MemberCount = u32;
pub type IssuerPoints = u32;

#[derive(Clone, Encode, Decode, Debug)]
pub struct CouncilMember<T: Config> {
	pub points: IssuerPoints,
	pub currency_id: CurrencyIdOf<T>,
	pub account_id: T::AccountId,
	pub validator_id: T::ValidatorId,
}

#[derive(Clone, Encode, Decode, Debug)]
pub struct CouncilProposal<T: Config> {
	pub proposal_hash: T::Hash,
	pub closing_block: T::BlockNumber,
}

#[derive(Clone, Encode, Decode, Debug, PartialEq)]
pub struct Ballot<T: Config> {
	pub yes_votes: BalanceOf<T>,
	pub no_votes: BalanceOf<T>,
}

impl<T: Config> Default for Ballot<T> {
	fn default() -> Self {
		Self { yes_votes: T::Balance::zero(), no_votes: T::Balance::zero() }
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq)]
pub struct UserVote<T: Config> {
	pub amount: BalanceOf<T>,
	pub approve: bool,
}

impl<T: Config> Default for UserVote<T> {
	fn default() -> Self {
		Self { amount: T::Balance::zero(), approve: false }
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq)]
pub enum PayoutsEnabled<Balance> {
	No,
	Limit(Balance),
}

#[derive(Clone, Encode, Decode, Debug, PartialEq)]
pub struct BondingConfig<T: Config> {
	pub payout: PayoutsEnabled<BalanceOf<T>>,
	pub vote: bool,
}

impl<T: Config> Default for BondingConfig<T> {
	fn default() -> Self {
		BondingConfig { payout: PayoutsEnabled::<_>::No, vote: false }
	}
}

impl<T: Config> BondingConfig<T> {
	fn get_payout_limit(&self) -> BalanceOf<T> {
		if let PayoutsEnabled::Limit(amount) = self.payout {
			amount
		} else {
			T::Balance::zero()
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Debug)]
pub enum SessionStatus {
	/// there are new validators that need to be announced to the session pallet
	Outdated,

	/// All council members should be known to the session pallet as validators
	UpToDate,
}

impl Default for SessionStatus {
	fn default() -> Self {
		SessionStatus::Outdated
	}
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug, TypeInfo)]
pub enum SlashReason {
	Offline,
	FaultyBlock,
	InitProposal,
	MissingVote,
}

impl SlashReason {
	fn get_penalty(&self) -> u32 {
		match self {
			SlashReason::FaultyBlock => 20,
			SlashReason::Offline => 5,
			SlashReason::InitProposal => 10,
			SlashReason::MissingVote => 10,
		}
	}
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug, TypeInfo)]
pub enum LeftCouncilReason {
	SlashedOut,
	Expelled,
	Voluntarily,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use crate::{
		BalanceOf, CurrencyIdOf, IssuerPoints, LeftCouncilReason, MajorityCount, MemberCount,
		Perbill, SlashReason,
	};
	use frame_system::WeightInfo;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Uses three types of Call:
	/// 1. `Call<Self>` for all calls within `issuer_council`
	/// 2. `Self::Call` for runtime calls
	/// 3. `pallet_proposal::Call` for runtime calls with restrictions from `pallet_proposal`
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_multi_mint::Config + pallet_proposal::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type GetSessionDuration: Get<Self::BlockNumber>;
		type InitialCouncilPoints: Get<IssuerPoints>;
		type CouncilRegistrationFee: Get<BalanceOf<Self>>;
		// Where the registration fee should go to
		// TODO: Needs to be set by governance
		type TreasuryAddress: Get<<Self as frame_system::Config>::AccountId>;

		/// The maximum number of members supported by the pallet. Used for weight
		/// estimation.
		///
		/// NOTE:
		/// + Benchmarks will need to be re-run and weights adjusted if this
		/// changes. + This pallet assumes that dependents keep to the limit without
		/// enforcing it.
		type MaxMembers: Get<MemberCount>;

		/// The time-out for council motions.
		type MotionDuration: Get<Self::BlockNumber>;

		//
		type Call: Parameter + From<Call<Self>> + Into<<Self as pallet_proposal::Config>::Call>;

		type ValidatorId: Member + Parameter;

		type MajorityCount: MajorityCount<<Self as pallet_proposal::Config>::Call>;

		type BondedAmount: traits::BondedAmount<
			<Self as frame_system::Config>::AccountId,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		type PayoutPool: traits::PayoutPool<CurrencyIdOf<Self>, BalanceOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// \[member_address\]
		NewMember(T::AccountId),
		/// \[applicant_address, proposal_hash\]
		NewApplicant(T::AccountId, T::Hash),
		/// \[proposal_hash, yes_votes, no_votes\]
		ProposalAccepted(T::Hash, Perbill, Perbill),

		/// \[proposal_hash, yes_votes, no_votes\]
		ProposalRejected(T::Hash, Perbill, Perbill),

		/// \[issuer_address, slash_reason\]
		Slashed(T::AccountId, SlashReason),
		LeftCouncil(T::AccountId, LeftCouncilReason),
		/// A motion for an applicant has been voted on by given account,
		/// leaving a tally (yes votes and no votes given respectively as
		/// `Amount`). \[voter, applicant, currency, approved, yes, no\]
		Voted(T::AccountId, T::Hash, T::CurrencyId, bool, T::Balance, T::Balance),

		/// BondingConfig has been changed for the given currency by the issuer
		/// \[currency, payout_limit_now, payout_limit_after, voting_now,
		/// voting_before\]
		CurrencyConfigChanged(T::CurrencyId, T::Balance, T::Balance, bool, bool),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(_origin: OriginFor<T>, _something: u32) -> DispatchResult {
			Ok(())
		}
	}
}

pub trait MajorityCount<C> {
	fn is_majority(call: C, yes_votes: Perbill, no_votes: Perbill) -> bool;
}
