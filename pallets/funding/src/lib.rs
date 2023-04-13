// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! # Funding Pallet
//!
//! A simple, secure module for handling projects funding.
//!
//! ## Overview
//!
//! To use it in your runtime, you need to implement the funding [`Config`].
//!
//! The supported dispatchable functions are documented in the [`Call`] enum.
//!
//! ## Interface
//!
//! ### Permissioned Functions, callable only by credentialized users
//!
//! * `note_image` : Save on-chin the Hash of the project metadata.
//! * `create` : Create a new project.
//! * `bond_evaluation` : Bond PLMC on a project's evaluation round.
//! * `failed_evaluation_unbond_for` : Unbond the PLMC bonded on a project's evaluation round for any user, if the project failed the evaluation.
//! * `bid` : Perform a bid during the Auction Round.
//! * `contribute` : Contribute to a project during the Community Round.
//! * `claim_contribution_tokens` : Claim the Contribution Tokens if you contributed to a project during the Funding Round.
//!
//! ### Privileged Functions, callable only by the project's Issuer
//!
//! * `edit_metadata` : Submit a new Hash of the project metadata.
//! * `start_evaluation` : Start the Evaluation Round of a project.
//! * `start_auction` : Start the Funding Round of a project.
//!

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// This recursion limit is needed because we have too many benchmarks and benchmarking will fail if
// we add more without this limit.
#![cfg_attr(feature = "runtime-benchmarks", recursion_limit = "512")]

pub use pallet::*;

pub mod types;
pub use types::*;

pub mod weights;

mod functions;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[allow(unused_imports)]
use polimec_traits::{MemberRole, PolimecMembers};

pub use crate::weights::WeightInfo;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::ValueQuery,
	traits::{
		tokens::{
			fungibles::{metadata::Mutate as MetadataMutate, Create, InspectMetadata, Mutate},
			Balance,
		},
		Currency as CurrencyT, Get, LockIdentifier, NamedReservableCurrency, Randomness,
		ReservableCurrency,
	},
	BoundedVec, PalletId, Parameter,
};
use parachains_common::Block;
use sp_arithmetic::traits::{One, Saturating, Zero};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, Hash},
	FixedPointNumber, FixedPointOperand, FixedU128,
};
use sp_std::{cmp::Reverse, prelude::*};
/// The balance type of this pallet.
type BalanceOf<T> = <T as Config>::CurrencyBalance;

/// The project type of this pallet.
type ProjectOf<T> = Project<
	BoundedVec<u8, <T as Config>::StringLimit>,
	BalanceOf<T>,
	<T as frame_system::Config>::Hash,
>;

type ProjectInfoOf<T> = ProjectInfo<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>;

/// The bid type of this pallet.
type BidInfoOf<T> = BidInfo<
	<T as Config>::ProjectIdentifier,
	BalanceOf<T>,
	<T as frame_system::Config>::AccountId,
	<T as frame_system::Config>::BlockNumber,
	Vesting<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>,
	Vesting<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>,
>;

type ContributionInfoOf<T> = ContributionInfo<
	BalanceOf<T>,
	Vesting<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>,
	Vesting<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>,
>;

// TODO: PLMC-151. Add multiple locks
// 	Review the use of locks after:
// 	- https://github.com/paritytech/substrate/issues/12918
// 	- https://github.com/paritytech/substrate/pull/12951
const LOCKING_ID: LockIdentifier = *b"evaluate";

// TODO: PLMC-152. Remove `dev_mode` attribute when extrinsics API are stable
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use local_macros::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Global identifier for the projects.
		type ProjectIdentifier: Parameter + Copy + Default + One + Saturating;
		// TODO: PLMC-153 + MaybeSerializeDeserialize: Maybe needed for JSON serialization @ Genesis: https://github.com/paritytech/substrate/issues/12738#issuecomment-1320921201

		/// Wrapper around `Self::ProjectIdentifier` to use in dispatchable call signatures. Allows the use
		/// of compact encoding in instances of the pallet, which will prevent breaking changes
		/// resulting from the removal of `HasCompact` from `Self::ProjectIdentifier`.
		///
		/// This type includes the `From<Self::ProjectIdentifier>` bound, since tightly coupled pallets may
		/// want to convert an `ProjectIdentifier` into a parameter for calling dispatchable functions
		/// directly.
		type ProjectIdParameter: Parameter
			+ From<Self::ProjectIdentifier>
			+ Into<Self::ProjectIdentifier>
			// TODO: PLMC-154 Used only in benchmarks, is there a way to bound this trait under #[cfg(feature = "runtime-benchmarks")]?
			+ From<u32>
			+ MaxEncodedLen;

		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to `From<u64>`.
		type CurrencyBalance: Balance + From<u64> + FixedPointOperand;

		/// The bonding balance.
		type Currency: NamedReservableCurrency<
			Self::AccountId,
			Balance = BalanceOf<Self>,
			ReserveIdentifier = BondType,
		>;

		/// The bidding balance.
		// type BiddingCurrency: Transfer<Self::AccountId>;
		type BiddingCurrency: ReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// Something that provides the members of Polimec
		type HandleMembers: PolimecMembers<Self::AccountId>;

		/// Something that provides the ability to create, mint and burn fungible assets.
		type Assets: Create<
				Self::AccountId,
				AssetId = Self::ProjectIdentifier,
				Balance = Self::CurrencyBalance,
			> + Mutate<Self::AccountId>
			+ MetadataMutate<Self::AccountId>
			+ InspectMetadata<Self::AccountId>;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// The maximum size of a preimage allowed, expressed in bytes.
		#[pallet::constant]
		type PreImageLimit: Get<u32>;

		/// The length (expressed in number of blocks) of the evaluation period.
		#[pallet::constant]
		type EvaluationDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type AuctionInitializePeriodDuration: Get<Self::BlockNumber>;

		/// The length (expressed in number of blocks) of the Auction Round, English period.
		#[pallet::constant]
		type EnglishAuctionDuration: Get<Self::BlockNumber>;

		/// The length (expressed in number of blocks) of the Auction Round, Candle period.
		#[pallet::constant]
		type CandleAuctionDuration: Get<Self::BlockNumber>;

		/// The length (expressed in number of blocks) of the Community Round.
		#[pallet::constant]
		type CommunityFundingDuration: Get<Self::BlockNumber>;

		/// The length (expressed in number of blocks) of the Funding Round.
		#[pallet::constant]
		type RemainderFundingDuration: Get<Self::BlockNumber>;

		/// `PalletId` for the funding pallet. An appropriate value could be
		/// `PalletId(*b"py/cfund")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The maximum number of "active" (In Evaluation or Funding Round) projects
		#[pallet::constant]
		type MaxProjectsToUpdatePerBlock: Get<u32>;

		/// The maximum number of bids per user
		#[pallet::constant]
		type MaximumBidsPerUser: Get<u32>;

		/// The maximum number of bids per user
		#[pallet::constant]
		type MaxContributionsPerUser: Get<u32>;

		/// Helper trait for benchmarks.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: BenchmarkHelper<Self>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn project_ids)]
	/// A global counter for indexing the projects
	/// OnEmpty in this case is GetDefault, so 0.
	pub type NextProjectId<T: Config> = StorageValue<_, T::ProjectIdentifier, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// A global counter used in the randomness generation
	// TODO: PLMC-155. Remove it after using the Randomness from BABE's VRF: https://github.com/PureStake/moonbeam/issues/1391
	// 	Or use the randomness from Moonbeam.
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn images)]
	/// A StorageMap containing all the images of the project metadata uploaded by the users.
	/// TODO: PLMC-156. The metadata should be stored on IPFS/offchain database, and the hash of the metadata should be stored here.
	pub type Images<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// A StorageMap containing all the the projects that applied for a request for funds
	pub type Projects<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn project_issuer)]
	/// StorageMap to "reverse lookup" the project issuer
	pub type ProjectsIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn project_info)]
	/// StorageMap containing all the the information for the projects
	pub type ProjectsInfo<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectInfoOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn projects_to_update)]
	/// A map for in which block to update which active projects.
	/// A Project is in need of an update at some point, if its status is {EvaluationRound, EvaluationEnded, AuctionRound(AuctionPhase), CommunityRound, FundingEnded}
	pub type ProjectsToUpdate<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<T::ProjectIdentifier, T::MaxProjectsToUpdatePerBlock>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn auctions_info)]
	/// StorageMap containing the bids for each project
	pub type AuctionsInfo<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<BidInfoOf<T>, T::MaximumBidsPerUser>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn evaluation_bonds)]
	/// Keep track of the bonds made to each project
	pub type EvaluationBonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		EvaluationBond<T::ProjectIdentifier, T::AccountId, BalanceOf<T>, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bidding_bonds)]
	/// Keep track of the bonds made to each project
	pub type BiddingBonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BiddingBond<T::ProjectIdentifier, T::AccountId, BalanceOf<T>, T::BlockNumber>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn contributing_bonds)]
	/// Keep track of the bonds made to each project
	pub type ContributingBonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		ContributingBond<T::ProjectIdentifier, T::AccountId, BalanceOf<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn contributions)]
	/// Contributions made during the Community Round
	pub type Contributions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ContributionInfoOf<T>, T::MaxContributionsPerUser>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A `project_id` was created.
		Created {
			project_id: T::ProjectIdentifier,
		},
		/// The metadata of `project_id` was modified.
		MetadataEdited {
			project_id: T::ProjectIdentifier,
		},
		/// The evaluation phase of `project_id` started.
		EvaluationStarted {
			project_id: T::ProjectIdentifier,
		},
		/// The evaluation phase of `project_id` ended without reaching the minimum threshold.
		EvaluationFailed {
			project_id: T::ProjectIdentifier,
		},
		/// The period an issuer has, to start the auction phase of the project.
		AuctionInitializePeriod {
			project_id: T::ProjectIdentifier,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
		},
		/// The auction round of `project_id` started at block `when`.
		EnglishAuctionStarted {
			project_id: T::ProjectIdentifier,
			when: T::BlockNumber,
		},
		CandleAuctionStarted {
			project_id: T::ProjectIdentifier,
			when: T::BlockNumber,
		},
		/// The auction round of `project_id` ended  at block `when`.
		AuctionEnded {
			project_id: T::ProjectIdentifier,
		},
		/// A `bonder` bonded an `amount` of PLMC for `project_id`.
		FundsBonded {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			bonder: T::AccountId,
		},
		/// Someone released the bond of a `bonder` for `project_id`, because the Evaluation round failed.
		BondReleased {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			bonder: T::AccountId,
			releaser: T::AccountId,
		},
		/// A `bidder` bid an `amount` at `market_cap` for `project_id` with a `multiplier`.
		Bid {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			price: BalanceOf<T>,
			multiplier: u32,
		},
		Contribution {
			project_id: T::ProjectIdentifier,
			contributor: T::AccountId,
			amount: BalanceOf<T>,
			multiplier: u32,
		},
		/// A bid  made by a `bidder` of `amount` at `market_cap` for `project_id` with a `multiplier` is returned.
		BidReturned {
			project_id: T::ProjectIdentifier,
			bidder: T::AccountId,
			amount: BalanceOf<T>,
			price: BalanceOf<T>,
			multiplier: u8,
		},
		CommunityFundingStarted {
			project_id: T::ProjectIdentifier,
		},
		RemainderFundingStarted {
			project_id: T::ProjectIdentifier,
		},
		FundingEnded {
			project_id: T::ProjectIdentifier,
		},
		Noted {
			hash: T::Hash,
		},
		/// Something was not properly initialized. Most likely due to dev error manually calling do_* functions or updating storage
		TransitionError {
			project_id: T::ProjectIdentifier,
			error: DispatchError,
		},
		/// Something terribly wrong happened where the bond could not be unbonded. Most likely a programming error
		FailedEvaluationUnbondFailed {
			error: DispatchError,
		},
		ContributionTokenMinted {
			caller: T::AccountId,
			project_id: T::ProjectIdentifier,
			contributor: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The price provided in the `create` call is too low
		PriceTooLow,
		/// The participation size provided in the `create` call is too low
		ParticipantsSizeError,
		/// The ticket size provided in the `create` call is too low
		TicketSizeError,
		/// The specified project does not exist
		ProjectNotFound,
		/// The Evaluation Round of the project has not started yet
		EvaluationNotStarted,
		/// The Evaluation Round of the project has already started
		EvaluationAlreadyStarted,
		/// The Evaluation Round of the project has ended without reaching the minimum threshold
		EvaluationFailed,
		/// The issuer cannot contribute to their own project during the Funding Round
		ContributionToThemselves,
		/// Only the issuer can start the Evaluation Round
		NotAllowed,
		/// The Metadata Hash of the project was not found
		MetadataNotProvided,
		/// The Auction Round of the project has already started
		AuctionAlreadyStarted,
		/// The Auction Round of the project has not started yet
		AuctionNotStarted,
		/// You cannot edit the metadata of a project that already passed the Evaluation Round
		Frozen,
		/// The bid is too low
		BidTooLow,
		/// The user has not enough balance to perform the action
		InsufficientBalance,
		/// There are too many active projects
		TooManyActiveProjects,
		// TODO: PLMC-133 Check after the introduction of the cross-chain identity pallet by KILT
		NotAuthorized,
		/// Contribution Tokens are already claimed
		AlreadyClaimed,
		/// The Funding Round of the project has not ended yet
		CannotClaimYet,
		/// No bids were made for the project at the time of the auction close
		NoBidsFound,
		/// Tried to freeze the project to start the Evaluation Round, but the project is already frozen
		ProjectAlreadyFrozen,
		/// Tried to move the project from Application to Evaluation round, but the project is not in ApplicationRound
		ProjectNotInApplicationRound,
		/// Tried to move the project from Evaluation to EvaluationEnded round, but the project is not in EvaluationRound
		ProjectNotInEvaluationRound,
		/// Tried to move the project from Evaluation to Auction round, but the project is not in EvaluationEndedRound
		ProjectNotInEvaluationEndedRound,
		/// Tried to move the project from AuctionInitializePeriod to EnglishAuctionRound, but the project is not in AuctionInitializePeriodRound
		ProjectNotInAuctionInitializePeriodRound,
		/// Tried to move the project to CandleAuction, but it was not in EnglishAuctionRound before
		ProjectNotInEnglishAuctionRound,
		/// Tried to move the project to Community round, but it was not in CandleAuctionRound before
		ProjectNotInCandleAuctionRound,
		/// Tried to move the project to RemainderRound, but it was not in CommunityRound before
		ProjectNotInCommunityRound,
		/// Tried to move the project to FundingEndedRound, but it was not in RemainderRound before
		ProjectNotInRemainderRound,
		/// Tried to move the project to ReadyToLaunch round, but it was not in FundingEnded round before
		ProjectNotInFundingEndedRound,
		/// Tried to start an auction before the initialization period
		TooEarlyForEnglishAuctionStart,
		/// Tried to start an auction after the initialization period
		TooLateForEnglishAuctionStart,
		/// Tried to move the project to CandleAuctionRound, but its too early for that
		TooEarlyForCandleAuctionStart,
		/// Tried to move the project to CommunityRound, but its too early for that
		TooEarlyForCommunityRoundStart,
		/// Tried to move the project to RemainderRound, but its too early for that
		TooEarlyForRemainderRoundStart,
		/// Tried to move to project to FundingEnded round, but its too early for that
		TooEarlyForFundingEnd,
		/// Tried to access auction metadata, but it was not correctly initialized.
		AuctionMetadataNotFound,
		/// Checks for other projects not copying metadata of others
		MetadataAlreadyExists,
		/// Ending block for the candle auction is not set
		EndingBlockNotSet,
		/// The specified issuer does not exist
		ProjectIssuerNotFound,
		/// The specified project info does not exist
		ProjectInfoNotFound,
		/// The Project was not correctly created. Most likely due to dev error manually calling do_* functions or updating storage
		ProjectNotCorrectlyCreated,
		/// Tried to finish an evaluation before its target end block
		EvaluationPeriodNotEnded,
		/// Tried to finish the english auction before its target end block
		EnglishAuctionPeriodNotEnded,
		/// Tried to access field that is not set
		FieldIsNone,
		/// Tried to create the contribution token after the remaining round but it failed
		AssetCreationFailed,
		/// Tried to update the metadata of the contribution token but it failed
		AssetMetadataUpdateFailed,
		/// Tried to do an operation assuming the evaluation failed, when in fact it did not
		EvaluationNotFailed,
		/// Tried to unbond PLMC after unsuccessful evaluation, but specified bond does not exist.
		BondNotFound,
		/// Checked math failed
		BadMath,
		/// Tried to bond PLMC for bidding, but that phase has already ended
		TooLateForBidBonding,
		/// Tried to withdraw funds that were vesting, but it was too early
		NextVestingWithdrawalNotReached,
		/// Tried to retrieve a bid but it does not exist
		BidNotFound,
		/// Tried to append a new bid to storage but too many bids were already made
		TooManyBids,
		/// Tried to append a new contribution to storage but too many were made for that user
		TooManyContributions,
		/// Tried to bond PLMC for contributing in the community or remainder round, but remainder round ended already
		TooLateForContributingBonding,
		/// Tried to contribute but its too low to be accepted
		ContributionTooLow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Start the "Funding Application" round
		/// Project applies for funding, providing all required information.
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(origin: OriginFor<T>, project: ProjectOf<T>) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			// TODO: PLMC-133 Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
			// ensure!(
			// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
			// 	Error::<T>::NotAuthorized
			// );

			Self::do_create(issuer, project)
		}

		/// Edit the `project_metadata` of a `project_id` if "Evaluation Round" is not yet started
		#[pallet::weight(T::WeightInfo::edit_metadata())]
		pub fn edit_metadata(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			project_metadata_hash: T::Hash,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_edit_metadata(issuer, project_id, project_metadata_hash)
		}

		/// Start the "Evaluation Round" of a `project_id`
		#[pallet::weight(T::WeightInfo::start_evaluation())]
		pub fn start_evaluation(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
			// ensure!(
			// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
			// 	Error::<T>::NotAuthorized
			// );

			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);

			Self::do_evaluation_start(project_id)
		}

		/// Start the "Evaluation Round" of a `project_id`
		#[pallet::weight(T::WeightInfo::start_auction())]
		pub fn start_auction(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
			// ensure!(
			// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
			// 	Error::<T>::NotAuthorized
			// );

			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);

			Self::do_english_auction(project_id)
		}

		/// Evaluators can bond `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond_evaluation(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let project_id = project_id.into();
			Self::do_evaluation_bond(from, project_id, amount)
		}

		/// Release the bonded PLMC for an evaluator if the project assigned to it is in the EvaluationFailed phase
		#[pallet::weight(T::WeightInfo::failed_evaluation_unbond_for())]
		pub fn failed_evaluation_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			bonder: T::AccountId,
		) -> DispatchResult {
			let releaser = ensure_signed(origin)?;
			let bond = EvaluationBonds::<T>::get(project_id.into(), bonder)
				.ok_or(Error::<T>::BondNotFound)?;
			Self::do_failed_evaluation_unbond_for(bond, releaser)
		}

		/// Place a bid in the "Auction Round"
		#[pallet::weight(T::WeightInfo::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
			#[pallet::compact] price: BalanceOf<T>,
			multiplier: Option<u32>,
			// TODO: PLMC-158 Add a parameter to specify the currency to use, should be equal to the currency
			// specified in `participation_currencies`
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;
			let project_id = project_id.into();

			// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
			// ensure!(
			// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
			// 	Error::<T>::NotAuthorized
			// );
			Self::do_bid(bidder, project_id, amount, price, multiplier)
		}

		/// Contribute to the "Community Round"
		#[pallet::weight(T::WeightInfo::contribute())]
		pub fn contribute(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let contributor = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_contribute(contributor, project_id, amount, None)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		pub fn vested_plmc_bid_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			bidder: T::AccountId,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_plmc_bid_unbond_for(claimer, project_id, bidder)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		pub fn vested_contribution_token_bid_mint_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			bidder: T::AccountId,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_contribution_token_bid_mint_for(claimer, project_id, bidder)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		pub fn vested_plmc_purchase_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			purchaser: T::AccountId,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_contribution_token_purchase_mint_for(claimer, project_id, purchaser)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		pub fn vested_contribution_token_purchase_mint_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			purchaser: T::AccountId,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_contribution_token_purchase_mint_for(claimer, project_id, purchaser)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// Get the projects that need to be updated on this block and update them
			for project_id in ProjectsToUpdate::<T>::take(now) {
				let maybe_project_info = ProjectsInfo::<T>::get(project_id.clone());
				let project_info = unwrap_option_or_skip!(maybe_project_info, project_id);

				match project_info.project_status {
					// Application -> EvaluationRound
					// Handled by user extrinsic

					// EvaluationRound -> AuctionInitializePeriod | EvaluationFailed
					ProjectStatus::EvaluationRound => {
						unwrap_result_or_skip!(Self::do_evaluation_end(project_id), project_id);
					},

					// AuctionInitializePeriod -> AuctionRound(AuctionPhase::English)
					// Handled by user extrinsic

					// AuctionRound(AuctionPhase::English) -> AuctionRound(AuctionPhase::Candle)
					ProjectStatus::AuctionRound(AuctionPhase::English) => {
						unwrap_result_or_skip!(Self::do_candle_auction(project_id), project_id);
					},

					// AuctionRound(AuctionPhase::Candle) -> CommunityRound
					ProjectStatus::AuctionRound(AuctionPhase::Candle) => {
						unwrap_result_or_skip!(Self::do_community_funding(project_id), project_id);
					},

					// CommunityRound -> RemainderRound
					ProjectStatus::CommunityRound => {
						unwrap_result_or_skip!(Self::do_remainder_funding(project_id), project_id)
					},

					// RemainderRound -> FundingEnded
					ProjectStatus::RemainderRound => {
						unwrap_result_or_skip!(Self::do_end_funding(project_id), project_id)
					},

					// FundingEnded -> ReadyToLaunch
					// Handled by user extrinsic
					_ => (),
				}
			}
			// TODO: PLMC-127. Set a proper weight
			Weight::from_ref_time(0)
		}

		/// Cleanup the `active_projects` BoundedVec
		fn on_idle(_now: T::BlockNumber, max_weight: Weight) -> Weight {
			let pallet_account: T::AccountId =
				<T as Config>::PalletId::get().into_account_truncating();

			let mut remaining_weight = max_weight.clone();
			let unbond_results = ProjectsInfo::<T>::iter()
				.filter_map(|(project_id, info)| {
					if let ProjectStatus::EvaluationFailed = info.project_status {
						Some(project_id)
					} else {
						None
					}
				})
				.flat_map(|project_id| {
					// get all the bonds for projects with a failed evaluation phase
					EvaluationBonds::<T>::iter_prefix(project_id)
						.map(|(bonder, bond)| bond)
						.collect::<Vec<_>>()
				})
				.take_while(|_| {
					if let Some(new_weight) =
						remaining_weight.checked_sub(&T::WeightInfo::failed_evaluation_unbond_for())
					{
						remaining_weight = new_weight;
						true
					} else {
						false
					}
				})
				.map(|bond| Self::do_failed_evaluation_unbond_for(bond, pallet_account.clone()))
				.collect::<Vec<_>>();

			for result in unbond_results {
				if let Err(e) = result {
					Self::deposit_event(Event::<T>::FailedEvaluationUnbondFailed { error: e });
				}
			}

			// // TODO: PLMC-127. Set a proper weightK
			max_weight.saturating_sub(remaining_weight)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub trait BenchmarkHelper<T: Config> {
		fn create_project_id_parameter(id: u32) -> T::ProjectIdParameter;
		fn create_dummy_project(metadata_hash: T::Hash) -> ProjectOf<T>;
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> BenchmarkHelper<T> for () {
		fn create_project_id_parameter(id: u32) -> T::ProjectIdParameter {
			id.into()
		}
		fn create_dummy_project(metadata_hash: T::Hash) -> ProjectOf<T> {
			let project: ProjectOf<T> = Project {
				total_allocation_size: 1_000_000u64.into(),
				minimum_price: 1__0_000_000_000_u64.into(),
				ticket_size: TicketSize { minimum: Some(1u8.into()), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				metadata: metadata_hash,
				..Default::default()
			};
			project
		}
	}
}

pub mod local_macros {
	// used to unwrap storage values that can be None in places where an error cannot be returned,
	// but an event should be emitted, and optionally a skip to the next iteration of a loop
	macro_rules! unwrap_option_or_skip {
		($option:expr, $project_id:expr) => {
			match $option {
				Some(val) => val,
				None => {
					Self::deposit_event(Event::<T>::TransitionError {
						project_id: $project_id,
						error: Error::<T>::FieldIsNone.into(),
					});
					continue
				},
			}
		};
	}
	pub(crate) use unwrap_option_or_skip;

	macro_rules! unwrap_result_or_skip {
		($option:expr, $project_id:expr) => {
			match $option {
				Ok(val) => val,
				Err(err) => {
					Self::deposit_event(Event::<T>::TransitionError {
						project_id: $project_id,
						error: err,
					});
					continue
				},
			}
		};
	}
	pub(crate) use unwrap_result_or_skip;
}
