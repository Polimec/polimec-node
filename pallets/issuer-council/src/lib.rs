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
	use orml_traits::MultiCurrency;
	use sp_runtime::traits::Saturating;
	use traits::PayoutPool;

	use crate::{
		AmountOf, BalanceOf, Ballot, BondingConfig, CouncilMember, CouncilProposal, CurrencyIdOf,
		IssuerPoints, LeftCouncilReason, MajorityCount, MemberCount, PayoutsEnabled, Perbill,
		Permill, SessionStatus, SlashReason, UserVote,
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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			Self::execute_proposals();

			0
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn apply_for_seat(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
			total_issuance: AmountOf<T>,
			currency_id: CurrencyIdOf<T>,
			payout_rate: Permill,
			metadata: T::ProposalMetadata,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check registration fee
			let registration_fee = T::CouncilRegistrationFee::get();
			let dot_id = <T as pallet_multi_mint::Config>::GetNativeCurrencyId::get();
			orml_tokens::Pallet::<T>::ensure_can_withdraw(dot_id, &who, registration_fee)?;

			// add proposal
			let call = Call::admit_new_member {
				account_id: who.clone(),
				validator_id,
				total_issuance,
				currency_id,
				payout_rate,
			};
			let (proposal_hash, _proposal_count) = Self::add_proposal(call.into(), metadata)?;

			<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
				dot_id,
				&who,
				&T::TreasuryAddress::get(),
				registration_fee,
			)?;

			Self::deposit_event(Event::NewApplicant(who, proposal_hash));
			// TODO: Return DispatchResultWithPostInfo
			// Ok(Some(<T as Config>::WeightInfo::apply_for_seat(proposal_count as u32)).into())
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn exit_council(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let _member_count = Self::do_exit_council(&who)?;

			Self::deposit_event(Event::LeftCouncil(who, LeftCouncilReason::Voluntarily));
			// TODO: Return DispatchResultWithPostInfo
			// Ok(Some(<T as Config>::WeightInfo::exit_council(member_count as u32)).into())
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn vote(
			origin: OriginFor<T>,
			proposal_hash: T::Hash,
			currency_id: CurrencyIdOf<T>,
			approve: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				pallet_proposal::ProposalOf::<T>::contains_key(proposal_hash),
				Error::<T>::ProposalNotFound
			);
			ensure!(
				pallet_multi_mint::Pallet::<T>::currency_metadata(currency_id).is_some(),
				pallet_multi_mint::Error::<T>::CurrencyNotFound
			);

			if CurrencyConfig::<T>::get(currency_id).vote {
				Ok(Self::do_vote(&who, &proposal_hash, &currency_id, approve)?)
			} else if let Some((issuer, _)) =
				pallet_multi_mint::Pallet::<T>::currency_metadata(currency_id)
			{
				if issuer == who {
					Ok(Self::do_vote(&who, &proposal_hash, &currency_id, approve)?)
				} else {
					Err(Error::<T>::VotingDisabled.into())
				}
			} else {
				Err(Error::<T>::VotingDisabled.into())
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		#[allow(clippy::boxed_local)]
		pub fn submit_proposal(
			origin: OriginFor<T>,
			call: Box<<T as Config>::Call>,
			metadata: T::ProposalMetadata,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let _member_count = Self::ensure_member(&who)?;

			let (_, _active_proposals) = Self::add_proposal(*call, metadata)?;

			Self::do_slash(&who, SlashReason::InitProposal)?;
			// TODO: Return DispatchResultWithPostInfo
			// Ok(Some(<T as Config>::WeightInfo::submit_proposal(member_count as u32,
			// active_proposals as u32)).into())
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn expel_member(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			let _member_count = Self::do_exit_council(&who)?;

			Self::deposit_event(Event::LeftCouncil(who, LeftCouncilReason::Expelled));
			// TODO: Return DispatchResultWithPostInfo
			// Ok(Some(<T as Config>::WeightInfo::expel_member(member_count as u32)).into())
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn admit_new_member(
			origin: OriginFor<T>,
			account_id: T::AccountId,
			validator_id: T::ValidatorId,
			total_issuance: AmountOf<T>,
			currency_id: CurrencyIdOf<T>,
			payout_rate: Permill,
		) -> DispatchResult {
			ensure_root(origin)?;

			let council_member = CouncilMember::<CurrencyIdOf<T>, T::AccountId, T::ValidatorId> {
				points: T::InitialCouncilPoints::get(),
				currency_id,
				account_id: account_id.clone(),
				validator_id,
			};

			let members_local = Members::<T>::get();
			let _member_count = members_local.len();
			match members_local.binary_search_by_key(&&account_id, |m| &m.account_id) {
				Ok(_) => Err(Error::<T>::DuplicateAccountId.into()),
				Err(_i) => {
					// register & mint currency
					pallet_multi_mint::Pallet::<T>::do_register_currency(
						account_id.clone(),
						&currency_id,
					)?;
					pallet_multi_mint::Pallet::<T>::do_mint(
						account_id.clone(),
						&currency_id,
						total_issuance,
					)?;
					T::PayoutPool::set_rate(&currency_id, &payout_rate);

					// update members
					// TODO: Handle the `try_append`
					let _ = <Members<T>>::try_append(council_member);
					Members::<T>::set(members_local);
					Status::<T>::set(SessionStatus::Outdated);
					Self::deposit_event(Event::NewMember(account_id));
					// TODO: Return DispatchResultWithPostInfo
					// Ok(Some(<T as Config>::WeightInfo::admit_new_member(member_count as
					// u32)).into())
					Ok(())
				},
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn slash(
			origin: OriginFor<T>,
			who: T::AccountId,
			reason: SlashReason,
		) -> DispatchResult {
			ensure_root(origin)?;
			let (_member_count, member_dropped) = Self::do_slash(&who, reason)?;
			if member_dropped {
				// TODO: Return DispatchResultWithPostInfo
				//Ok(Some(<T as Config>::WeightInfo::slash_drop_member(member_count as
				// u32)).into())
				Ok(())
			} else {
				// TODO: Return DispatchResultWithPostInfo
				//Ok(Some(<T as Config>::WeightInfo::slash_keep_member(member_count as
				// u32)).into())
				Ok(())
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn set_bonding_config(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			set_stake: Option<PayoutsEnabled<BalanceOf<T>>>,
			set_vote: Option<bool>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pallet_multi_mint::Pallet::<T>::currency_metadata(currency_id).is_some(),
				pallet_multi_mint::Error::<T>::CurrencyNotFound
			);

			if let Some((issuer, _)) =
				pallet_multi_mint::Pallet::<T>::currency_metadata(currency_id)
			{
				ensure!(issuer == who, pallet_multi_mint::Error::<T>::Unauthorized);

				// get current config and set upcoming one
				let cfg_old = CurrencyConfig::<T>::get(currency_id);
				let limit_old = cfg_old.get_payout_limit();
				let cfg_new = BondingConfig {
					payout: set_stake.unwrap_or_else(|| cfg_old.payout.clone()),
					vote: set_vote.unwrap_or(cfg_old.vote),
				};

				// check and update payout pool
				let pool = T::PayoutPool::get_amount(&currency_id);
				let limit_new: BalanceOf<T> = cfg_new.get_payout_limit();
				ensure!(limit_new >= limit_old, Error::<T>::PayoutPoolUnderflow);
				if limit_new > limit_old {
					T::PayoutPool::set_amount(
						&currency_id,
						&(pool.saturating_add(limit_new - limit_old)),
					);
				}

				<CurrencyConfig<T>>::insert(currency_id, cfg_new.clone());
				Self::deposit_event(Event::CurrencyConfigChanged(
					currency_id,
					limit_new,
					limit_old,
					cfg_new.vote,
					cfg_old.vote,
				));
				Ok(())
			} else {
				Err(pallet_multi_mint::Error::<T>::CurrencyNotFound.into())
			}
		}
	}
}

use frame_support::{dispatch::DispatchError, traits::Get, BoundedVec};
use sp_arithmetic::traits::{Saturating, Zero};
use traits::BondedAmount;

impl<T: Config> Pallet<T> {
	fn ensure_member(account_id: &T::AccountId) -> Result<usize, DispatchError> {
		let members = Members::<T>::get();
		let member_count = members.len();
		match members.binary_search_by_key(&account_id, |m| &m.account_id) {
			Err(_) => Err(Error::<T>::NotMember.into()),
			Ok(_) => Ok(member_count),
		}
	}

	pub fn add_proposal(
		call: <T as Config>::Call,
		metadata: T::ProposalMetadata,
	) -> Result<(T::Hash, usize), DispatchError> {
		let proposal = pallet_proposal::Proposal {
			// unbox call, convert into pallet_proposal::Call and box again
			call: Box::new(call.into()),
			metadata,
		};
		let (proposal_hash, active_proposals, _) = pallet_proposal::Pallet::<T>::add_proposal(
			proposal, // FIXME: Set reasonable limit
			0xFFFFFFF,
		)?;
		let now = frame_system::Pallet::<T>::block_number();
		let closing_block = now.saturating_add(T::GetSessionDuration::get());

		// TODO: Check how to use the `MultiRemovalResult`
		let _ = <CouncilProposals<T>>::try_append(CouncilProposal { closing_block, proposal_hash });

		Ok((proposal_hash, active_proposals))
	}

	/// 1 read (Applicants)
	/// 1 write (Applicants)
	pub fn do_reject_proposal(proposal_hash: T::Hash) {
		// remove proposal hash
		pallet_proposal::Pallet::<T>::remove_proposal(proposal_hash);
		// remove all votes
		// TODO: Check the limit
		// TODO: Check what `maybe_cursor` means
		// TODO: Check how to use the `MultiRemovalResult`
		let _ = ProposalVotes::<T>::clear_prefix(proposal_hash, 100, None);
	}

	/// Leave the member pool
	pub fn do_exit_council(account_id: &T::AccountId) -> Result<usize, DispatchError> {
		let mut members = Members::<T>::get();
		match members.binary_search_by_key(&account_id, |m| &m.account_id) {
			Err(_) => Err(Error::<T>::NotMember.into()),
			Ok(i) => {
				let member_count = members.len() - 1;
				members.remove(i);
				Members::<T>::set(members);
				Status::<T>::set(SessionStatus::Outdated);
				Ok(member_count as usize)
			},
		}
	}

	/// Cast vote for a single currency
	pub fn do_vote(
		voter_id: &T::AccountId,
		proposal_hash: &T::Hash,
		currency_id: &CurrencyIdOf<T>,
		approve: bool,
	) -> Result<(), DispatchError> {
		// get bonded amount of voter_id
		if let Some(amount_new) = T::BondedAmount::get_active(voter_id, currency_id) {
			// get last voted amount of voter_id
			let UserVote { amount: amount_old, approve: approve_old } =
				UserVotes::<T>::get(proposal_hash, (voter_id, currency_id)).unwrap_or_default();
			// update ProposalVotes if either amount or approval has changed
			if Self::do_update_applicant_votes(
				voter_id,
				currency_id,
				proposal_hash,
				&amount_new,
				&amount_old,
				approve,
				approve_old,
			) {
				<UserVotes<T>>::insert(
					proposal_hash,
					(voter_id, currency_id),
					UserVote { amount: amount_new, approve },
				);
			}
		}
		Ok(())
	}

	/// Update ProposalVotes if user has changed amount or decision
	/// max 1 storage write: ProposalVotes
	fn do_update_applicant_votes(
		voter_id: &T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		proposal_hash: &T::Hash,
		amount_new: &BalanceOf<T>,
		amount_old: &BalanceOf<T>,
		approve_new: bool,
		approve_old: bool,
	) -> bool {
		if amount_old != amount_new || approve_old != approve_new {
			let ballot_new = Self::do_update_ballot(
				proposal_hash,
				*currency_id,
				*amount_new,
				*amount_old,
				approve_new,
				approve_old,
			);
			<ProposalVotes<T>>::insert(proposal_hash, currency_id, ballot_new.clone());
			Self::deposit_event(Event::Voted(
				voter_id.clone(),
				*proposal_hash,
				*currency_id,
				approve_new,
				ballot_new.yes_votes,
				ballot_new.no_votes,
			));
			true
		} else {
			false
		}
	}

	/// Update a user's currency ballot based on the last vote (if existent)
	/// 1 storage read: ProposalVotes
	fn do_update_ballot(
		proposal_hash: &T::Hash,
		currency_id: CurrencyIdOf<T>,
		amount_new: BalanceOf<T>,
		amount_old: BalanceOf<T>,
		approve_new: bool,
		approve_old: bool,
	) -> Ballot<BalanceOf<T>> {
		let Ballot { mut yes_votes, mut no_votes } =
			ProposalVotes::<T>::get(proposal_hash, currency_id);
		match (approve_old, approve_new) {
			// approval changed from `no` to `yes`
			(false, true) => {
				yes_votes = yes_votes.saturating_add(amount_new);
				no_votes = no_votes.saturating_sub(amount_old);
			},
			// approval changed from `yes` to `no`
			(true, false) => {
				yes_votes = yes_votes.saturating_sub(amount_old);
				no_votes = no_votes.saturating_add(amount_new);
			},
			// approval has not changed
			_ => match (amount_new > amount_old, approve_new) {
				// `yes` votes increased
				(true, true) => {
					yes_votes = yes_votes.saturating_add(amount_new.saturating_sub(amount_old));
				},
				// `yes` votes decreased
				(false, true) => {
					yes_votes = yes_votes.saturating_sub(amount_old.saturating_sub(amount_new));
				},
				// `no` votes increased
				(true, false) => {
					no_votes = no_votes.saturating_add(amount_new.saturating_sub(amount_old));
				},
				// `yes` votes increased
				(false, false) => {
					no_votes = no_votes.saturating_sub(amount_old.saturating_sub(amount_new));
				},
			},
		}
		Ballot { yes_votes, no_votes }
	}

	/// Count an applicants votes for all currencies
	/// Either adds applicant to member pool or rejects application
	///
	/// 1 read (Members)
	pub fn do_tally_proposal(proposal_hash: T::Hash) -> Result<(), DispatchError> {
		if let Some(pallet_proposal::Proposal { call, .. }) =
			pallet_proposal::Pallet::<T>::proposal_of(proposal_hash)
		{
			let members = Members::<T>::get();
			let native_currency = <T as pallet_multi_mint::Config>::GetNativeCurrencyId::get();
			let mut ayes = Perbill::from_percent(0);
			let mut nays = Perbill::from_percent(0);

			// count votes for all currencies
			for CouncilMember { currency_id, points, .. } in members {
				// exclude native currency from counting
				if currency_id != native_currency {
					let Ballot { yes_votes, no_votes } =
						ProposalVotes::<T>::get(proposal_hash, currency_id);
					if yes_votes + no_votes > T::Balance::zero() {
						// 50% + 1 votes are required to be approved by an issuer, e.g. per currency
						// each member has 1 vote * points
						if yes_votes > no_votes {
							ayes = ayes.saturating_add(Perbill::from_percent(points));
						} else {
							nays = nays.saturating_add(Perbill::from_percent(points));
						}
					} else if let Some((issuer, _)) =
						pallet_multi_mint::Pallet::<T>::currency_metadata(currency_id)
					{
						Self::do_slash(&issuer, SlashReason::MissingVote)?;
					}
				}
			}

			if T::MajorityCount::is_majority(*call, ayes, nays) {
				Self::do_approve_proposal(proposal_hash)?;
				Self::deposit_event(Event::ProposalAccepted(proposal_hash, ayes, nays));
			} else {
				Self::do_reject_proposal(proposal_hash);
				Self::deposit_event(Event::ProposalRejected(proposal_hash, ayes, nays));
			}

			Ok(())
		} else {
			Err(Error::<T>::ProposalNotFound.into())
		}
	}

	/// Move applicant to member pool
	/// Registers & mints currency_id with total_issuance as set in application
	///
	/// 1 read (Members)
	/// 2 writes (Members, Status)
	fn do_approve_proposal(proposal_hash: T::Hash) -> Result<(), DispatchError> {
		// TODO: Implement `execute_proposal` in `pallet_proposal`
		// TODO - FIXME: add sane bounds, use weights, check errors(?)
		// let _ = pallet_proposal::Event::<T>::execute_proposal(
		// 	proposal_hash,
		// 	0xFFFFFFFF,
		// 	0xFFFFFFFFFFFFFFFF,
		// );
		// TODO: Check the limit
		// TODO: Check what `maybe_cursor` means
		// TODO: Check how to use the `MultiRemovalResult`
		let _ = ProposalVotes::<T>::clear_prefix(proposal_hash, 100, None);

		Ok(())
	}

	/// Reduce a member's points
	/// The member is removed from the council if this reduction leads leq 0
	/// points
	pub fn do_slash(
		account_id: &T::AccountId,
		reason: SlashReason,
	) -> Result<(usize, bool), DispatchError> {
		let amount = reason.get_penalty();
		let mut status = None;
		let members: Vec<CouncilMember<CurrencyIdOf<T>, T::AccountId, T::ValidatorId>> =
			Members::<T>::get()
				.into_iter()
				.filter_map(|mut m| {
					if &m.account_id == account_id {
						Self::deposit_event(Event::Slashed(account_id.clone(), reason));
						if m.points > amount {
							m.points = m.points.saturating_sub(amount);
							Some(m)
						} else {
							Self::deposit_event(Event::LeftCouncil(
								account_id.clone(),
								LeftCouncilReason::SlashedOut,
							));
							status = Some(SessionStatus::Outdated);
							None
						}
					} else {
						Some(m)
					}
				})
				.collect();
		let member_count = members.len();
		// TODO (UNSAFE!): Check the bound instead of unwrap the result!
		let bounded_members = BoundedVec::try_from(members).unwrap();
		Members::<T>::set(bounded_members);
		if let Some(status) = status {
			Status::<T>::set(status);
			Ok((member_count, true))
		} else {
			Ok((member_count, false))
		}
	}

	pub fn execute_proposals() {
		let now = frame_system::Pallet::<T>::block_number();
		let proposals = CouncilProposals::<T>::get();
		let proposal_count = proposals.len();

		let remaining_proposals: Vec<CouncilProposal<T::Hash, T::BlockNumber>> = proposals
			.into_iter()
			.filter(|CouncilProposal { closing_block, proposal_hash }| {
				if closing_block <= &now {
					let res = Self::do_tally_proposal(*proposal_hash);
					if res.is_err() {
						log::error!(
							"🏛 error while tally proposal. (proposal_hash: {:?})",
							proposal_hash
						);
					}
					false
				} else {
					true
				}
			})
			.collect();

		if proposal_count != remaining_proposals.len() {
			// TODO (UNSAFE!): Check the bound instead of unwrap the result!
			let bounded_remaining_proposals = BoundedVec::try_from(remaining_proposals).unwrap();
			CouncilProposals::<T>::set(bounded_remaining_proposals);
		}
	}

	pub fn get_current_era(
	) -> (<T as frame_system::Config>::BlockNumber, <T as frame_system::Config>::BlockNumber) {
		let duration = T::GetSessionDuration::get();
		let now = frame_system::Pallet::<T>::block_number();
		let era_start = now - (now % duration);

		(era_start, era_start.saturating_add(duration))
	}
}

use pallet_session::SessionManager;
use sp_staking::SessionIndex;

impl<T: Config> SessionManager<T::ValidatorId> for Pallet<T> {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		let session_status = Status::<T>::get();
		let members = Members::<T>::get();

		if session_status == SessionStatus::Outdated && !members.is_empty() {
			log::info!("🏛 Council members become validators.");
			let new_validators = members.iter().map(|m| m.validator_id.clone()).collect();
			log::trace!("🏛 New members: {:#?}", new_validators);
			Members::<T>::set(members);
			Status::<T>::set(SessionStatus::UpToDate);
			Some(new_validators)
		} else if session_status == SessionStatus::Outdated {
			log::warn!("🏛 No Council members! Keep old validator set.");
			None
		} else {
			None
		}
	}

	fn end_session(_end_index: SessionIndex) {}
	fn start_session(_start_index: SessionIndex) {}
}

use pallet_session::ShouldEndSession;

impl<T: Config> ShouldEndSession<T::BlockNumber> for Pallet<T>
where
	T::BlockNumber: From<u32>,
{
	fn should_end_session(now: T::BlockNumber) -> bool {
		let session_status = Status::<T>::get();
		let duration = T::GetSessionDuration::get();
		let should_end = session_status == SessionStatus::Outdated && (now % duration).is_zero();

		log::trace!("🏛 Next rotation? {:?}", should_end);
		should_end
	}
}

use frame_support::traits::EstimateNextSessionRotation;
use sp_runtime::Permill;

impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
	fn estimate_next_session_rotation(now: T::BlockNumber) -> (Option<T::BlockNumber>, u64) {
		match Status::<T>::get() {
			SessionStatus::Outdated => {
				let duration = T::GetSessionDuration::get();
				let remaining_blocks = duration - (now % duration);

				let rotation_at = now + remaining_blocks;
				log::trace!("🏛 Next session rotation at: {:?}", rotation_at);
				(Some(rotation_at), 1)
			},
			SessionStatus::UpToDate => (None, 0),
		}
	}

	fn average_session_length() -> T::BlockNumber {
		todo!()
	}

	fn estimate_current_session_progress(
		_now: T::BlockNumber,
	) -> (Option<Permill>, frame_support::weights::Weight) {
		todo!()
	}
}

use sp_runtime::traits::Convert;

impl<T: Config> Convert<T::AccountId, Option<T::ValidatorId>> for Pallet<T> {
	fn convert(account: T::AccountId) -> Option<T::ValidatorId> {
		let members = Members::<T>::get();
		if let Ok(i) = members.binary_search_by_key(&&account, |a| &a.account_id) {
			Some(members[i].validator_id.clone())
		} else {
			None
		}
	}
}

use traits::BondedVote;

impl<T: Config> BondedVote<T::AccountId, CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	/// Update voting weight of a user after changing the bonded amount
	/// # <weight>
	/// - Time complexity: O(A * U), where A is number of active applications, U number of
	///   applicants user voted for
	/// ---------------
	/// DB Weight:
	/// - Read: UserVotes, Applicants
	/// - Write: ProposalVotes, UserVotes
	/// # </weight>
	fn update_amount(
		controller: &T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount_new: &BalanceOf<T>,
		amount_old: &BalanceOf<T>,
	) -> u32 {
		let mut votes: u32 = 0;
		for CouncilProposal { proposal_hash: hash, .. } in <CouncilProposals<T>>::get().into_iter()
		{
			if let Some(UserVote { approve, .. }) =
				UserVotes::<T>::get(hash, (controller, currency_id))
			{
				Self::do_update_applicant_votes(
					controller,
					currency_id,
					&hash,
					amount_new,
					amount_old,
					approve,
					approve,
				);
				<UserVotes<T>>::insert(
					hash,
					(controller, currency_id),
					UserVote { amount: *amount_new, approve },
				);
				votes += 1;
			}
		}
		votes
	}
}

pub trait MajorityCount<C> {
	fn is_majority(call: C, yes_votes: Perbill, no_votes: Perbill) -> bool;
}
