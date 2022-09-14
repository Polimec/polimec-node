#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, HasCompact};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_arithmetic::Perbill;

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

#[derive(Clone, Encode, Decode, Debug, TypeInfo)]
pub struct CouncilMember<CurrencyId, AccountId, ValidatorId> {
	pub points: IssuerPoints,
	pub currency_id: CurrencyId,
	pub account_id: AccountId,
	pub validator_id: ValidatorId,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct CouncilProposal<Hash, BlockNumber> {
	pub proposal_hash: Hash,
	pub closing_block: BlockNumber,
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
pub struct Ballot<Balance: HasCompact> {
	pub yes_votes: Balance,
	pub no_votes: Balance,
}

impl<Balance: HasCompact + orml_traits::arithmetic::Zero> Default for Ballot<Balance> {
	fn default() -> Self {
		Self { yes_votes: Balance::zero(), no_votes: Balance::zero() }
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
pub struct UserVote<Balance: HasCompact> {
	pub amount: Balance,
	pub approve: bool,
}

impl<Balance: HasCompact + orml_traits::arithmetic::Zero> Default for UserVote<Balance> {
	fn default() -> Self {
		Self { amount: Balance::zero(), approve: false }
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
pub enum PayoutsEnabled<Balance> {
	No,
	Limit(Balance),
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
pub struct BondingConfig<Balance: HasCompact> {
	pub payout: PayoutsEnabled<Balance>,
	pub vote: bool,
}

impl<Balance: HasCompact> Default for BondingConfig<Balance> {
	fn default() -> Self {
		BondingConfig { payout: PayoutsEnabled::<_>::No, vote: false }
	}
}

impl<Balance: HasCompact + Clone + orml_traits::arithmetic::Zero> BondingConfig<Balance> {
	fn get_payout_limit(&self) -> Balance {
		match &self.payout {
			PayoutsEnabled::Limit(amount) => amount.to_owned(),
			_ => Balance::zero(),
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Debug, Eq, TypeInfo)]
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
	use frame_support::pallet_prelude::{ValueQuery, *};
	use frame_system::pallet_prelude::*;

	use crate::{
		BalanceOf, Ballot, BondingConfig, CouncilMember, CouncilProposal, CurrencyIdOf,
		IssuerPoints, LeftCouncilReason, MajorityCount, MemberCount, Perbill, SessionStatus,
		SlashReason, UserVote,
	};
	use frame_system::WeightInfo;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
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

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type CouncilProposals<T: Config> = StorageValue<
		_,
		BoundedVec<CouncilProposal<T::Hash, T::BlockNumber>, ConstU32<128>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn session_status)]
	pub type Status<T: Config> = StorageValue<_, SessionStatus, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn members)]
	// Members: () -> [member]
	pub type Members<T: Config> = StorageValue<
		_,
		BoundedVec<CouncilMember<CurrencyIdOf<T>, T::AccountId, T::ValidatorId>, ConstU32<128>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn proposal_votes)]
	// ProposalVotes: applicant, currency -> { yes_votes, no_votes }
	pub type ProposalVotes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::Hash,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Ballot<BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_votes)]
	// UserVotes: voter, currency -> (staked_balance, approve, applicants)?
	pub type UserVotes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::Hash,
		Blake2_128Concat,
		(T::AccountId, CurrencyIdOf<T>),
		UserVote<BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn currency_config)]
	pub type CurrencyConfig<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, BondingConfig<BalanceOf<T>>, ValueQuery>;

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
		DuplicateAccountId,
		CurrencyAlreadyProposed,
		NotMember,
		ProposalNotFound,
		AlreadyMember,
		MemberLimitReached,
		ChoiceMissing,
		VotingDisabled,
		PayoutPoolUnderflow,
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
