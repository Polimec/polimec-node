#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, dispatch,
	dispatch::Weight,
	ensure,
	traits::{EstimateNextSessionRotation, Get},
	Parameter,
};
use frame_system::{ensure_root, ensure_signed};
use orml_traits::currency::MultiCurrency;
use session::{SessionManager, ShouldEndSession};
use sp_runtime::{
	traits::{Convert, Member, Saturating, Zero},
	Perbill, Permill,
};
use sp_staking::SessionIndex;
use sp_std::{boxed::Box, prelude::Vec};
use traits::{BondedAmount, BondedVote, PayoutPool};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod default_weights;

pub use default_weights::WeightInfo;

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
		Self {
			yes_votes: T::Balance::zero(),
			no_votes: T::Balance::zero(),
		}
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq)]
pub struct UserVote<T: Config> {
	pub amount: BalanceOf<T>,
	pub approve: bool,
}

impl<T: Config> Default for UserVote<T> {
	fn default() -> Self {
		Self {
			amount: T::Balance::zero(),
			approve: false,
		}
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
		BondingConfig {
			payout: PayoutsEnabled::<_>::No,
			vote: false,
		}
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

/// The pallet's configuration trait.
///
/// Uses three types of Call:
/// 1. `Call<Self>` for all calls within `issuer_council`
/// 2. `Self::Call` for runtime calls
/// 3. `pallet_proposal::Call` for runtime calls with restrictions from
/// `pallet_proposal`
pub trait Config: frame_system::Config + pallet_multi_mint::Config + pallet_proposal::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

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

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug)]
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

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug)]
pub enum LeftCouncilReason {
	SlashedOut,
	Expelled,
	Voluntarily,
}

// This pallet's storage items.
decl_storage! {
	trait Store for Module<T: Config> as IssuerCouncilModule {
		// CouncilProposals: (closing block, proposal_hash)
		pub CouncilProposals get(fn proposals): Vec<CouncilProposal<T>>;

		pub Status get(fn session_status): SessionStatus;

		// Members: () -> [member]
		pub Members get(fn members) build(|config: &GenesisConfig<T>| {
			// TODO: Add sorting
			config.members.iter().map(|(account_id, validator_id, currency_id)| CouncilMember {
				points: T::InitialCouncilPoints::get(),
				currency_id: *currency_id,
				account_id: account_id.clone(),
				validator_id: validator_id.clone(),
			}).collect()
		}): Vec<CouncilMember<T>>;

		// ProposalVotes: applicant, currency -> { yes_votes, no_votes }
		pub ProposalVotes get(fn proposal_votes): double_map
			hasher(blake2_128_concat) T::Hash,
			hasher(blake2_128_concat) CurrencyIdOf<T>
			=> Ballot<T>;

		// UserVotes: voter, currency -> (staked_balance, approve, applicants)?
		pub UserVotes get(fn user_votes): double_map
			hasher(blake2_128_concat) T::Hash,
			hasher(blake2_128_concat) (T::AccountId, CurrencyIdOf<T>)
			=> Option<UserVote<T>>;

		pub CurrencyConfig get(fn currency_config): map hasher(opaque_blake2_256) CurrencyIdOf<T> => BondingConfig<T>;
	}
	add_extra_genesis {
		config(members): Vec<(T::AccountId, T::ValidatorId, CurrencyIdOf<T>)>;
	}
}

// The pallet's events
decl_event!(
	pub enum Event<T>
	where
		AccountId = <T as frame_system::Config>::AccountId,
		CurrencyId = CurrencyIdOf<T>,
		Balance = BalanceOf<T>,
		// BlockNumber = <T as frame_system::Config>::BlockNumber,
		Hash = <T as frame_system::Config>::Hash,
	{
		/// \[member_address\]
		NewMember(AccountId),
		/// \[applicant_address, proposal_hash\]
		NewApplicant(AccountId, Hash),
		/// \[proposal_hash, yes_votes, no_votes\]
		ProposalAccepted(Hash, Perbill, Perbill),

		/// \[proposal_hash, yes_votes, no_votes\]
		ProposalRejected(Hash, Perbill, Perbill),

		/// \[issuer_address, slash_reason\]
		Slashed(AccountId, SlashReason),
		LeftCouncil(AccountId, LeftCouncilReason),
		/// A motion for an applicant has been voted on by given account,
		/// leaving a tally (yes votes and no votes given respectively as
		/// `Amount`). \[voter, applicant, currency, approved, yes, no\]
		Voted(AccountId, Hash, CurrencyId, bool, Balance, Balance),

		/// BondingConfig has been changed for the given currency by the issuer
		/// \[currency, payout_limit_now, payout_limit_after, voting_now,
		/// voting_before\]
		CurrencyConfigChanged(CurrencyId, Balance, Balance, bool, bool),
	}
);

// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Config> {
		DuplicateAccountId,
		CurrencyAlreadyProposed,
		NotMember,
		ProposalNotFound,
		AlreadyMember,
		MemberLimitReached,
		ChoiceMissing,
		VotingDisabled,
		PayoutPoolUnderflow
	}
}

// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;

		/// Initialization
		fn on_initialize(now: T::BlockNumber) -> Weight {
			Self::execute_proposals();

			0
		}

		/// Create a new application and adds it to the store.
		#[weight = <T as Config>::WeightInfo::apply_for_seat(T::MaxProposals::get())]
		pub fn apply_for_seat(origin, validator_id: T::ValidatorId, total_issuance: AmountOf<T>, currency_id: CurrencyIdOf<T>, payout_rate: Permill, metadata: T::ProposalMetadata) -> dispatch::DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			// check registration fee
			let registration_fee = T::CouncilRegistrationFee::get();
			let dot_id = <T as pallet_multi_mint::Config>::GetNativeCurrencyId::get();
			orml_tokens::Module::<T>::ensure_can_withdraw(dot_id, &who, registration_fee)?;

			// add proposal
			let call = Call::<T>::admit_new_member(who.clone(), validator_id, total_issuance, currency_id, payout_rate);
			let (proposal_hash, proposal_count) = Self::add_proposal(call.into(), metadata)?;

			<orml_tokens::Module<T> as MultiCurrency<T::AccountId>>::transfer(
				dot_id,
				&who,
				&T::TreasuryAddress::get(),
				registration_fee,
			)?;

			Self::deposit_event(RawEvent::NewApplicant(who, proposal_hash));
			Ok(Some(<T as Config>::WeightInfo::apply_for_seat(proposal_count as u32)).into())
		}

		#[weight = <T as Config>::WeightInfo::exit_council(T::MaxMembers::get())]
		pub fn exit_council(origin) -> dispatch::DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let member_count = Self::do_exit_council(&who)?;

			Self::deposit_event(RawEvent::LeftCouncil(who, LeftCouncilReason::Voluntarily));
			Ok(Some(<T as Config>::WeightInfo::exit_council(member_count as u32)).into())
		}

		#[weight = <T as Config>::WeightInfo::vote()]
		pub fn vote(origin, proposal_hash: T::Hash, currency_id: CurrencyIdOf<T>, approve: bool) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(
				pallet_proposal::ProposalOf::<T>::contains_key(proposal_hash),
				Error::<T>::ProposalNotFound
			);
			ensure!(pallet_multi_mint::CurrencyMetadata::<T>::contains_key(currency_id), pallet_multi_mint::Error::<T>::CurrencyNotFound);

			if CurrencyConfig::<T>::get(currency_id).vote {
				Ok(Self::do_vote(&who, &proposal_hash, &currency_id, approve)?)
			} else if let Some((issuer, _)) = pallet_multi_mint::CurrencyMetadata::<T>::get(&currency_id) {
				if issuer == who {
					Ok(Self::do_vote(&who, &proposal_hash, &currency_id, approve)?)
				} else {
					Err(Error::<T>::VotingDisabled.into())
				}
			} else {
				Err(Error::<T>::VotingDisabled.into())
			}
		}

		/// Submit a generic proposal.
		#[weight = <T as Config>::WeightInfo::submit_proposal(T::MaxMembers::get(), T::MaxProposals::get())]
		#[allow(clippy::boxed_local)]
		pub fn submit_proposal(origin, call: Box<<T as Config>::Call>, metadata: T::ProposalMetadata) -> dispatch::DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let member_count = Self::ensure_member(&who)?;

			let (_, active_proposals) = Self::add_proposal(*call, metadata)?;

			Self::do_slash(&who, SlashReason::InitProposal)?;

			Ok(Some(<T as Config>::WeightInfo::submit_proposal(member_count as u32, active_proposals as u32)).into())
		}

		#[weight = <T as Config>::WeightInfo::expel_member(T::MaxMembers::get())]
		pub fn expel_member(origin, who: T::AccountId) -> dispatch::DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let member_count = Self::do_exit_council(&who)?;

			Self::deposit_event(RawEvent::LeftCouncil(who, LeftCouncilReason::Expelled));
			Ok(Some(<T as Config>::WeightInfo::expel_member(member_count as u32)).into())
		}

		#[weight = <T as Config>::WeightInfo::admit_new_member(T::MaxMembers::get())]
		pub fn admit_new_member(
			origin,
			account_id: T::AccountId,
			validator_id: T::ValidatorId,
			total_issuance: AmountOf<T>,
			currency_id: CurrencyIdOf<T>,
			payout_rate: Permill,
		) -> dispatch::DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let council_member = CouncilMember::<T> {
				points: T::InitialCouncilPoints::get(),
				currency_id,
				account_id: account_id.clone(),
				validator_id
			};

			let mut members = Members::<T>::get();
			let member_count = members.len();
			match members.binary_search_by_key(&&account_id, |m| &m.account_id) {
				Ok(_) => Err(Error::<T>::DuplicateAccountId.into()),
				Err(i) => {
					// register & mint currency
					pallet_multi_mint::Module::<T>::do_register_currency(account_id.clone(), &currency_id)?;
					pallet_multi_mint::Module::<T>::do_mint(account_id.clone(), &currency_id, total_issuance)?;
					T::PayoutPool::set_rate(&currency_id, &payout_rate);

					// update members
					members.insert(i, council_member);
					Members::<T>::set(members);
					Status::set(SessionStatus::Outdated);
					Self::deposit_event(RawEvent::NewMember(
						account_id,
					));
					Ok(Some(<T as Config>::WeightInfo::admit_new_member(member_count as u32)).into())
				}
			}
		}

		#[weight = <T as Config>::WeightInfo::slash_keep_member(T::MaxMembers::get())
			  .max(<T as Config>::WeightInfo::slash_drop_member(T::MaxMembers::get()))]
		pub fn slash(origin, who: T::AccountId, reason: SlashReason) -> dispatch::DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let (member_count, member_dropped) = Self::do_slash(&who, reason)?;
			if member_dropped {
				Ok(Some(<T as Config>::WeightInfo::slash_drop_member(member_count as u32)).into())
			}
			else {
				Ok(Some(<T as Config>::WeightInfo::slash_keep_member(member_count as u32)).into())
			}
		}

		/// Enables an issuer to set the bonding config for their currency, e.g.
		/// (1) Enabling or disabling voting except for the issuer
		/// (2) Disabling or setting an upper limit for the bonded payouts amount.
		// TODO: Add WeightInfo
		#[weight = 10_000]
		pub fn set_bonding_config(origin, currency_id: CurrencyIdOf<T>, set_stake: Option<PayoutsEnabled<BalanceOf<T>>>, set_vote: Option<bool>) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			if let Some((issuer, _)) = pallet_multi_mint::CurrencyMetadata::<T>::get(&currency_id) {
				ensure!(issuer == who, pallet_multi_mint::Error::<T>::Unauthorized);

				// get current config and set upcoming one
				let cfg_old = CurrencyConfig::<T>::get(&currency_id);
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
					T::PayoutPool::set_amount(&currency_id, &(pool.saturating_add(limit_new - limit_old)));
				}

				<CurrencyConfig<T>>::insert(currency_id, cfg_new.clone());
				Self::deposit_event(RawEvent::CurrencyConfigChanged(currency_id, limit_new, limit_old, cfg_new.vote, cfg_old.vote));
				Ok(())

			} else {
				Err(pallet_multi_mint::Error::<T>::CurrencyNotFound.into())
			}
		}
	}
}

impl<T: Config> Module<T> {
	fn ensure_member(account_id: &T::AccountId) -> Result<usize, dispatch::DispatchError> {
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
	) -> Result<(T::Hash, usize), dispatch::DispatchError> {
		let proposal = pallet_proposal::Proposal {
			// unbox call, convert into pallet_proposal::Call and box again
			call: Box::new(call.into()),
			metadata,
		};
		let (proposal_hash, active_proposals, _) = pallet_proposal::Module::<T>::add_proposal(
			proposal, // FIXME: Set reasonable limit
			0xFFFFFFF,
		)?;
		let now = frame_system::Module::<T>::block_number();
		let closing_block = now.saturating_add(T::GetSessionDuration::get());

		<CouncilProposals<T>>::append(CouncilProposal {
			closing_block,
			proposal_hash,
		});

		Ok((proposal_hash, active_proposals))
	}

	/// 1 read (Applicants)
	/// 1 write (Applicants)
	pub fn do_reject_proposal(proposal_hash: T::Hash) {
		// remove proposal hash
		pallet_proposal::Module::<T>::remove_proposal(proposal_hash);
		// remove all votes
		ProposalVotes::<T>::remove_prefix(proposal_hash);
	}

	/// Leave the member pool
	pub fn do_exit_council(account_id: &T::AccountId) -> Result<usize, dispatch::DispatchError> {
		let mut members = Members::<T>::get();
		match members.binary_search_by_key(&account_id, |m| &m.account_id) {
			Err(_) => Err(Error::<T>::NotMember.into()),
			Ok(i) => {
				let member_count = members.len() - 1;
				members.remove(i);
				Members::<T>::set(members);
				Status::set(SessionStatus::Outdated);
				Ok(member_count as usize)
			}
		}
	}

	/// Cast vote for a single currency
	pub fn do_vote(
		voter_id: &T::AccountId,
		proposal_hash: &T::Hash,
		currency_id: &CurrencyIdOf<T>,
		approve: bool,
	) -> Result<(), dispatch::DispatchError> {
		// get bonded amount of voter_id
		if let Some(amount_new) = T::BondedAmount::get_active(&voter_id, &currency_id) {
			// get last voted amount of voter_id
			let UserVote {
				amount: amount_old,
				approve: approve_old,
			} = UserVotes::<T>::get(proposal_hash, (voter_id, currency_id)).unwrap_or_default();
			// update ProposalVotes if either amount or approval has changed
			if Self::do_update_applicant_votes(
				voter_id,
				&currency_id,
				proposal_hash,
				&amount_new,
				&amount_old,
				approve,
				approve_old,
			) {
				<UserVotes<T>>::insert(
					proposal_hash,
					(voter_id, currency_id),
					UserVote {
						amount: amount_new,
						approve,
					},
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
			Self::deposit_event(RawEvent::Voted(
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
	) -> Ballot<T> {
		let Ballot {
			mut yes_votes,
			mut no_votes,
		} = ProposalVotes::<T>::get(proposal_hash, currency_id);
		match (approve_old, approve_new) {
			// approval changed from `no` to `yes`
			(false, true) => {
				yes_votes = yes_votes.saturating_add(amount_new);
				no_votes = no_votes.saturating_sub(amount_old);
			}
			// approval changed from `yes` to `no`
			(true, false) => {
				yes_votes = yes_votes.saturating_sub(amount_old);
				no_votes = no_votes.saturating_add(amount_new);
			}
			// approval has not changed
			_ => match (amount_new > amount_old, approve_new) {
				// `yes` votes increased
				(true, true) => {
					yes_votes = yes_votes.saturating_add(amount_new.saturating_sub(amount_old));
				}
				// `yes` votes decreased
				(false, true) => {
					yes_votes = yes_votes.saturating_sub(amount_old.saturating_sub(amount_new));
				}
				// `no` votes increased
				(true, false) => {
					no_votes = no_votes.saturating_add(amount_new.saturating_sub(amount_old));
				}
				// `yes` votes increased
				(false, false) => {
					no_votes = no_votes.saturating_sub(amount_old.saturating_sub(amount_new));
				}
			},
		}
		Ballot { yes_votes, no_votes }
	}

	/// Count an applicants votes for all currencies
	/// Either adds applicant to member pool or rejects application
	///
	/// 1 read (Members)
	pub fn do_tally_proposal(proposal_hash: T::Hash) -> Result<(), dispatch::DispatchError> {
		if let Some(pallet_proposal::Proposal { call, .. }) = pallet_proposal::Module::<T>::proposal_of(proposal_hash) {
			let members = Members::<T>::get();
			let native_currency = <T as pallet_multi_mint::Config>::GetNativeCurrencyId::get();
			let mut ayes = Perbill::from_percent(0);
			let mut nays = Perbill::from_percent(0);

			// count votes for all currencies
			for CouncilMember {
				currency_id, points, ..
			} in members
			{
				// exclude native currency from counting
				if currency_id != native_currency {
					let Ballot { yes_votes, no_votes } = ProposalVotes::<T>::get(proposal_hash, &currency_id);
					if yes_votes + no_votes > T::Balance::zero() {
						// 50% + 1 votes are required to be approved by an issuer, e.g. per currency
						// each member has 1 vote * points
						if yes_votes > no_votes {
							ayes = ayes.saturating_add(Perbill::from_percent(points));
						} else {
							nays = nays.saturating_add(Perbill::from_percent(points));
						}
					} else if let Some((issuer, _)) = pallet_multi_mint::Module::<T>::issuer_of_currency(currency_id) {
						Self::do_slash(&issuer, SlashReason::MissingVote)?;
					}
				}
			}

			if T::MajorityCount::is_majority(*call, ayes, nays) {
				Self::do_approve_proposal(proposal_hash)?;
				Self::deposit_event(RawEvent::ProposalAccepted(proposal_hash, ayes, nays));
			} else {
				Self::do_reject_proposal(proposal_hash);
				Self::deposit_event(RawEvent::ProposalRejected(proposal_hash, ayes, nays));
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
	fn do_approve_proposal(proposal_hash: T::Hash) -> Result<(), dispatch::DispatchError> {
		// FIXME: add sane bounds, use weights, check errors(?)
		let _ = pallet_proposal::Module::<T>::execute_proposal(proposal_hash, 0xFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
		ProposalVotes::<T>::remove_prefix(proposal_hash);

		Ok(())
	}

	/// Reduce a member's points
	/// The member is removed from the council if this reduction leads leq 0
	/// points
	pub fn do_slash(account_id: &T::AccountId, reason: SlashReason) -> Result<(usize, bool), dispatch::DispatchError> {
		let amount = reason.get_penalty();
		let mut status = None;
		let members: Vec<CouncilMember<T>> = Members::<T>::get()
			.into_iter()
			.filter_map(|mut m| {
				if &m.account_id == account_id {
					Self::deposit_event(RawEvent::Slashed(account_id.clone(), reason));
					if m.points > amount {
						m.points = m.points.saturating_sub(amount);
						Some(m)
					} else {
						Self::deposit_event(RawEvent::LeftCouncil(account_id.clone(), LeftCouncilReason::SlashedOut));
						status = Some(SessionStatus::Outdated);
						None
					}
				} else {
					Some(m)
				}
			})
			.collect();
		let member_count = members.len();
		Members::<T>::set(members);
		if let Some(status) = status {
			Status::set(status);
			Ok((member_count, true))
		} else {
			Ok((member_count, false))
		}
	}

	pub fn execute_proposals() {
		let now = frame_system::Module::<T>::block_number();
		let proposals = CouncilProposals::<T>::get();
		let proposal_count = proposals.len();

		let remaining_proposals: Vec<CouncilProposal<T>> = proposals
			.into_iter()
			.filter(
				|CouncilProposal {
				     closing_block,
				     proposal_hash,
				 }| {
					if closing_block <= &now {
						let res = Self::do_tally_proposal(*proposal_hash);
						if res.is_err() {
							debug::error!("🏛 error while tally proposal. (proposal_hash: {:?})", proposal_hash);
						}
						false
					} else {
						true
					}
				},
			)
			.collect();

		if proposal_count != remaining_proposals.len() {
			CouncilProposals::<T>::set(remaining_proposals);
		}
	}

	pub fn get_current_era() -> (
		<T as frame_system::Config>::BlockNumber,
		<T as frame_system::Config>::BlockNumber,
	) {
		let duration = T::GetSessionDuration::get();
		let now = frame_system::Module::<T>::block_number();
		let era_start = now - (now % duration);

		(era_start, era_start.saturating_add(duration))
	}
}

impl<T: Config> SessionManager<T::ValidatorId> for Module<T> {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		let session_status = Status::get();
		let members = Members::<T>::get();

		if session_status == SessionStatus::Outdated && !members.is_empty() {
			debug::info!("🏛 Council members become validators.");
			let new_validators = members.iter().map(|m| m.validator_id.clone()).collect();
			debug::trace!("🏛 New members: {:#?}", new_validators);
			Members::<T>::set(members);
			Status::set(SessionStatus::UpToDate);
			Some(new_validators)
		} else if session_status == SessionStatus::Outdated {
			debug::warn!("🏛 No Council members! Keep old validator set.");
			None
		} else {
			None
		}
	}

	fn end_session(_end_index: SessionIndex) {}
	fn start_session(_start_index: SessionIndex) {}
}

impl<T: Config> ShouldEndSession<T::BlockNumber> for Module<T>
where
	T::BlockNumber: From<u32>,
{
	fn should_end_session(now: T::BlockNumber) -> bool {
		let session_status = Status::get();
		let duration = T::GetSessionDuration::get();
		let should_end = session_status == SessionStatus::Outdated && (now % duration).is_zero();

		debug::trace!("🏛 Next rotation? {:?}", should_end);
		should_end
	}
}

impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Module<T> {
	fn estimate_next_session_rotation(now: T::BlockNumber) -> Option<T::BlockNumber> {
		match Status::get() {
			SessionStatus::Outdated => {
				let duration = T::GetSessionDuration::get();
				let remaining_blocks = duration - (now % duration);

				let rotation_at = now + remaining_blocks;
				debug::trace!("🏛 Next session rotation at: {:?}", rotation_at);
				Some(rotation_at)
			}
			SessionStatus::UpToDate => None,
		}
	}

	fn weight(_now: T::BlockNumber) -> dispatch::Weight {
		T::DbWeight::get().reads(1)
	}
}

impl<T: Config> Convert<T::AccountId, Option<T::ValidatorId>> for Module<T> {
	fn convert(account: T::AccountId) -> Option<T::ValidatorId> {
		let members = Members::<T>::get();
		if let Ok(i) = members.binary_search_by_key(&&account, |a| &a.account_id) {
			Some(members[i].validator_id.clone())
		} else {
			None
		}
	}
}

impl<T: Config> BondedVote<T::AccountId, CurrencyIdOf<T>, BalanceOf<T>> for Module<T> {
	/// Update voting weight of a user after changing the bonded amount
	/// # <weight>
	/// - Time complexity: O(A * U), where A is number of active applications, U
	///   number of applicants user voted for
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
		for CouncilProposal {
			proposal_hash: hash, ..
		} in <CouncilProposals<T>>::get().into_iter()
		{
			if let Some(UserVote { approve, .. }) = UserVotes::<T>::get(hash, (controller, currency_id)) {
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
					UserVote {
						amount: *amount_new,
						approve,
					},
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
