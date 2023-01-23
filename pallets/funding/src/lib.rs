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
//! ### Terminology
//!
//! ### Goals
//!
//! ## Interface
//!
//! ### Permissionless Functions
//!
//! ### Permissioned Functions
//!
//! ### Privileged Functions
//!
//!
//! Please refer to the [`Call`] enum and its associated variants for documentation on each
//! function.
//!
//! ### Public Functions
//!
//! * `create` - .
//! * `edit_metadata` - .
//!
//! Please refer to the [`Pallet`] struct for details on publicly available functions.
//!

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// This recursion limit is needed because we have too many benchmarks and benchmarking will fail if
// we add more without this limit.
#![cfg_attr(feature = "runtime-benchmarks", recursion_limit = "512")]
// Nightly only feature. It allows us to combine traits into a single trait.
#![feature(trait_alias)]

use codec::MaxEncodedLen;
pub use pallet::*;

pub mod types;
pub use types::*;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod functions;

use frame_support::{
	pallet_prelude::{Member, ValueQuery},
	traits::{
		tokens::{
			fungibles::{metadata::Mutate as MetadataMutate, Create, InspectMetadata, Mutate},
			Balance,
		},
		Currency, Get, LockIdentifier, LockableCurrency, Randomness, ReservableCurrency,
		WithdrawReasons,
	},
	BoundedVec, PalletId, Parameter,
};
use polimec_traits::{MemberRole, PolimecMembers};
use sp_arithmetic::traits::{Saturating, Zero};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, Hash},
	FixedPointNumber, FixedPointOperand, FixedU128, Perbill,
};
use sp_std::{ops::AddAssign, prelude::*};
/// The balance type of this pallet.
type BalanceOf<T> = <T as Config>::CurrencyBalance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type ProjectOf<T> = Project<
	BoundedVec<u8, <T as Config>::StringLimit>,
	BalanceOf<T>,
	<T as frame_system::Config>::Hash,
>;
type BidInfoOf<T> = BidInfo<
	BalanceOf<T>,
	<T as frame_system::Config>::AccountId,
	<T as frame_system::Config>::BlockNumber,
>;

// TODO: Add multiple locks
const LOCKING_ID: LockIdentifier = *b"evaluate";

pub trait Identifiable =
	Member + Parameter + Copy + MaxEncodedLen + Default + AddAssign + From<u32>;
// TODO: + MaybeSerializeDeserialize: Maybe needed for JSON serialization @ Genesis: https://github.com/paritytech/substrate/issues/12738#issuecomment-1320921201

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	/// The module configuration trait.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Global identifier for the projects.
		type ProjectIdentifier: Identifiable;

		/// Wrapper around `Self::ProjectIdentifier` to use in dispatchable call signatures. Allows the use
		/// of compact encoding in instances of the pallet, which will prevent breaking changes
		/// resulting from the removal of `HasCompact` from `Self::ProjectIdentifier`.
		///
		/// This type includes the `From<Self::ProjectIdentifier>` bound, since tightly coupled pallets may
		/// want to convert an `ProjectIdentifier` into a parameter for calling dispatchable functions
		/// directly.
		type ProjectIdParameter: Parameter
			+ Copy
			+ From<Self::ProjectIdentifier>
			+ Into<Self::ProjectIdentifier>
			+ From<u32>
			+ MaxEncodedLen;

		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to
		/// `From<u64>`.
		type CurrencyBalance: Balance + From<u64> + FixedPointOperand;

		/// The bonding balance.
		type Currency: LockableCurrency<
			Self::AccountId,
			Moment = Self::BlockNumber,
			Balance = BalanceOf<Self>,
		>;

		/// The bidding balance.
		type BiddingCurrency: ReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// Something that provides the members of the Polimec
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

		#[pallet::constant]
		type EvaluationDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type EnglishAuctionDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type CandleAuctionDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type CommunityRoundDuration: Get<Self::BlockNumber>;

		/// `PalletId` for the funding pallet. An appropriate value could be
		/// `PalletId(*b"py/cfund")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The maximum number of "active" (In Evaluation or Funding Round) projects
		#[pallet::constant]
		type ActiveProjectsLimit: Get<u32>;

		/// The maximum number of bids per project
		#[pallet::constant]
		type MaximumBidsPerProject: Get<u32>;

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
	// TODO: Remove it after using the Randomness from BABE's VRF
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn images)]
	/// A StorageMap containing all the images uploaded by the users
	pub type Images<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// A StorageMap containing all the the projects that applied for a request for funds
	pub type Projects<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn project_issuer)]
	/// StorageMap (k: ProjectIdentifier, v: T::AccountId) to "reverse lookup" the project issuer so
	/// the users doesn't need to specify each time the project issuer
	pub type ProjectsIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn project_info)]
	/// StorageMap(k1: ProjectIdentifier, v:ProjectInfo) containing all the the information for the projects
	pub type ProjectsInfo<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		ProjectInfo<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn projects_active)]
	/// A BoundedVec to list all the "active" Projects
	/// A Project is active if its status is {EvaluationRound, EvaluationEnded, AuctionRound(AuctionPhase), CommunityRound, FundingEnded}
	pub type ProjectsActive<T: Config> =
		StorageValue<_, BoundedVec<T::ProjectIdentifier, T::ActiveProjectsLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn auctions_info)]
	/// Save the bids for each project and when they were made
	pub type AuctionsInfo<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		BoundedVec<BidInfoOf<T>, T::MaximumBidsPerProject>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bonds)]
	/// StorageDoubleMap (k1: ProjectIdentifier, k2: T::AccountId, v: BalanceOf<T>) to store the bonds for each project during the Evaluation Round
	pub type Bonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
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
		ContributionInfo<BalanceOf<T>>,
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
		/// The evaluation phase of `project_id` was started.
		EvaluationStarted {
			project_id: T::ProjectIdentifier,
		},
		/// The evaluation phase of `project_id` was ended.
		EvaluationEnded {
			project_id: T::ProjectIdentifier,
		},
		/// The auction round of `project_id` started at block `when`.
		AuctionStarted {
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
		},
		/// A `bidder` bid an `amount` at `market_cap` for `project_id` with a `multiplier`.
		Bid {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			market_cap: BalanceOf<T>,
			multiplier: u8,
		},
		/// A bid  made by a `bidder` of `amount` at `market_cap` for `project_id` with a `multiplier` is returned.
		BidReturned {
			project_id: T::ProjectIdentifier,
			bidder: T::AccountId,
			amount: BalanceOf<T>,
			market_cap: BalanceOf<T>,
			multiplier: u8,
		},
		Noted {
			hash: T::Hash,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		PriceTooLow,
		ParticipantsSizeError,
		TicketSizeError,
		ProjectNotExists,
		EvaluationAlreadyStarted,
		ContributionToThemselves,
		NotAllowed,
		EvaluationNotStarted,
		AuctionAlreadyStarted,
		AuctionNotStarted,
		Frozen,
		BondTooLow,
		BondTooHigh,
		InsufficientBalance,
		TooManyActiveProjects,
		NotAuthorized,
		AlreadyClaimed,
		CannotClaimYet,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Validate a preimage on-chain and store the image.
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		// TODO: Use BounmdedVec instead of Vec
		pub fn note_image(origin: OriginFor<T>, bytes: Vec<u8>) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			ensure!(
				T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
				Error::<T>::NotAuthorized
			);

			Self::note_bytes(bytes.into(), &issuer)?;

			Ok(())
		}
		/// Start the "Funding Application" round
		/// Project applies for funding, providing all required information.
		#[pallet::weight(T::WeightInfo::create())]
		pub fn create(origin: OriginFor<T>, project: ProjectOf<T>) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			ensure!(
				T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
				Error::<T>::NotAuthorized
			);
			ensure!(Images::<T>::contains_key(project.metadata), Error::<T>::ProjectNotExists);

			match project.validity_check() {
				Err(error) => match error {
					ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
					ValidityError::ParticipantsSizeError =>
						Err(Error::<T>::ParticipantsSizeError.into()),
					ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
				},
				Ok(()) => {
					let project_id = NextProjectId::<T>::get();
					Self::do_create(project_id, &issuer, project)
				},
			}
		}

		/// Edit the `project_metadata` of a `project_id` if "Evaluation Round" is not yet started
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn edit_metadata(
			origin: OriginFor<T>,
			project_metadata_hash: T::Hash,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(ProjectsIssuers::<T>::contains_key(project_id), Error::<T>::ProjectNotExists);
			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);
			ensure!(Images::<T>::contains_key(project_metadata_hash), Error::<T>::ProjectNotExists);
			ensure!(!ProjectsInfo::<T>::get(project_id).is_frozen, Error::<T>::Frozen);

			Projects::<T>::try_mutate(project_id, |maybe_project| -> DispatchResult {
				let project = maybe_project.as_mut().ok_or(Error::<T>::ProjectNotExists)?;
				project.metadata = project_metadata_hash;
				Self::deposit_event(Event::MetadataEdited { project_id });
				Ok(())
			})?;
			Ok(())
		}

		/// Start the "Evaluation Round" of a `project_id`
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn start_evaluation(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(ProjectsIssuers::<T>::contains_key(project_id), Error::<T>::ProjectNotExists);
			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);
			ensure!(
				ProjectsInfo::<T>::get(project_id).project_status == ProjectStatus::Application,
				Error::<T>::EvaluationAlreadyStarted
			);
			Self::do_start_evaluation(project_id)
		}

		/// Evaluators can bond `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn bond(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let project_id = project_id.into();

			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;
			ensure!(from != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_id);
			ensure!(
				project_info.project_status == ProjectStatus::EvaluationRound,
				Error::<T>::EvaluationNotStarted
			);
			ensure!(T::Currency::free_balance(&from) > amount, Error::<T>::InsufficientBalance);
			let project = Projects::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Take the value given by the issuer or use the minimum balance any single account may have.
			let minimum_amount =
				project.ticket_size.minimum.unwrap_or_else(T::Currency::minimum_balance);

			// Take the value given by the issuer or use the total amount of issuance in the system.
			let maximum_amount =
				project.ticket_size.maximum.unwrap_or_else(T::Currency::total_issuance);
			ensure!(amount >= minimum_amount, Error::<T>::BondTooLow);
			ensure!(amount <= maximum_amount, Error::<T>::BondTooHigh);

			// TODO: Unlock the PLMC when it's the right time
			// Check if the user has already bonded
			Bonds::<T>::try_mutate(project_id, &from, |maybe_bond| {
				match maybe_bond {
					Some(bond) => {
						// If the user has already bonded, add the new amount to the old one
						let new_bond = bond.checked_add(&amount).unwrap();
						*maybe_bond = Some(new_bond);
						T::Currency::set_lock(LOCKING_ID, &from, new_bond, WithdrawReasons::all());
					},
					None => {
						// If the user has not bonded yet, create a new bond
						*maybe_bond = Some(amount);
						T::Currency::set_lock(LOCKING_ID, &from, amount, WithdrawReasons::all());
					},
				}
				Ok(())
			})
		}

		/// Start the "Funding Round" of a `project_id`
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn start_auction(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(ProjectsIssuers::<T>::contains_key(project_id), Error::<T>::ProjectNotExists);
			ensure!(
				T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
				Error::<T>::NotAuthorized
			);
			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);
			let project_info = ProjectsInfo::<T>::get(project_id);
			ensure!(
				project_info.project_status == ProjectStatus::EvaluationEnded,
				Error::<T>::EvaluationNotStarted
			);
			Self::do_start_auction(project_id)
		}
		/// Place a bid in the "Auction Round"
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn bid(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] price: BalanceOf<T>,
			#[pallet::compact] market_cap: BalanceOf<T>,
			multiplier: Option<u8>,
			// TODO: Add a parameter to specify the currency to use, should be equal to the currency
			// specified in `participation_currencies`
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(
				T::HandleMembers::is_in(&MemberRole::Professional, &bidder) ||
					T::HandleMembers::is_in(&MemberRole::Institutional, &bidder),
				Error::<T>::NotAuthorized
			);

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the bidder is not the project_issuer
			ensure!(bidder != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_id);
			let project = Projects::<T>::get(project_id)
				.expect("Project exists, already checked in previous ensure");

			// Make sure Auction Round is started
			ensure!(
				matches!(project_info.project_status, ProjectStatus::AuctionRound(_)),
				Error::<T>::AuctionNotStarted
			);

			// Make sure the bidder can actually perform the bid
			let free_balance_of = T::Currency::free_balance(&bidder);
			ensure!(free_balance_of >= price, Error::<T>::InsufficientBalance);

			// Make sure the bid amount is greater than the minimum_price specified by the issuer
			ensure!(price >= project.minimum_price, Error::<T>::BondTooLow);

			let now = <frame_system::Pallet<T>>::block_number();
			let multiplier = multiplier.unwrap_or(1);
			let bid = BidInfo::new(
				market_cap,
				price,
				project.fundraising_target,
				now,
				bidder.clone(),
				multiplier,
			);

			// TODO: If it's better to save te bids ordered, we can use smt like this
			// let mut bids = AuctionsInfo::<T>::get(project_id);
			// let index = bids.partition_point(|x| x < &bid);
			// bids.try_insert(index, bid);

			match AuctionsInfo::<T>::try_append(project_id, &bid) {
				Ok(_) => {
					// Reserve the new bid
					T::BiddingCurrency::reserve(&bidder, price)?;
					// TODO: Send an XCM message to Statemine to transfer amount * multiplier USDT to the PalletId Account
					// Alternative TODO: The user should have the specified currency (e.g: USDT) already on Polimec
					Self::deposit_event(Event::<T>::Bid {
						project_id,
						amount: price,
						market_cap,
						multiplier,
					});
				},
				Err(_) => {
					// TODO: Check the best strategy to handle the case where the vector is full
					// Maybe it-s better to keep the vector sorted so we always know the lowest bid
					let mut bids = AuctionsInfo::<T>::get(project_id);

					// Get the lowest bid and its index
					let (index, lowest_bid) = bids
							.iter()
							.enumerate()
							.min()
							.expect("This code runs only if the vector is full, so there is always a minimum; qed");
					// Make sure the new bid is greater than the lowest bid
					if &bid > lowest_bid {
						// Reserve the new bid
						T::BiddingCurrency::reserve(&bidder, price)?;
						// Unreserve the lowest bid
						T::BiddingCurrency::unreserve(&lowest_bid.bidder, lowest_bid.amount);
						// Remove the lowest bid from the AuctionsInfo
						bids.remove(index);
						// Add the new bid to the AuctionsInfo, this should never fail since we just removed an element
						bids.try_push(bid)
							.expect("We removed an element, so there is always space");
						AuctionsInfo::<T>::set(project_id, bids);
						// TODO: Send an XCM message to Statemine to transfer amount * multiplier USDT to the PalletId Account
						Self::deposit_event(Event::<T>::Bid {
							project_id,
							amount: price,
							market_cap,
							multiplier,
						});
					} else {
						// New bid is lower than the lowest bid, return Error
						Err(Error::<T>::BondTooLow)?
					}
				},
			};

			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Contribute to the "Community Round"
		pub fn contribute(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let contributor = ensure_signed(origin)?;
			let project_id = project_id.into();

			// TODO: Add the "Retail before, Institutional and Professionals after, if there are still tokens" logic
			ensure!(
				T::HandleMembers::is_in(&MemberRole::Retail, &contributor),
				Error::<T>::NotAuthorized
			);

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the contributor is not the project_issuer
			ensure!(contributor != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_id);
			let project = Projects::<T>::get(project_id)
				.expect("Project exists, already checked in previous ensure");

			// Make sure Community Round is started
			ensure!(
				project_info.project_status == ProjectStatus::CommunityRound,
				Error::<T>::AuctionNotStarted
			);

			// Make sure the contributor can actually perform the bid
			let free_balance_of = T::Currency::free_balance(&contributor);
			ensure!(free_balance_of > amount, Error::<T>::InsufficientBalance);

			// Make sure the bid amount is greater than the minimum_price specified by the issuer
			ensure!(free_balance_of > project.minimum_price, Error::<T>::BondTooLow);

			let fund_account = T::fund_account_id(project_id);
			// TODO: Use USDT on Statemine (via XCM) instead of PLMC
			// TODO: Check the logic
			T::Currency::transfer(
				&contributor,
				&fund_account,
				amount,
				// TODO: Take the ExistenceRequirement as parameter
				frame_support::traits::ExistenceRequirement::KeepAlive,
			)?;

			Contributions::<T>::get(project_id, &contributor)
				.map(|mut contribution| {
					contribution.amount += amount;
					Contributions::<T>::insert(project_id, &contributor, contribution)
				})
				.unwrap_or_else(|| {
					let contribution = ContributionInfo { amount, can_claim: true };
					Contributions::<T>::insert(project_id, &contributor, contribution)
				});

			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		// TODO: Manage the fact that the CTs may not be claimed by those entitled
		pub fn claim_contribution_tokens(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(ProjectsIssuers::<T>::contains_key(project_id), Error::<T>::ProjectNotExists);

			// TODO: Check the right credential status
			// ensure!(
			// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
			// 	Error::<T>::NotAuthorized
			// );

			let project = Projects::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;
			let project_info = ProjectsInfo::<T>::get(project_id);
			ensure!(
				project_info.project_status == ProjectStatus::ReadyToLaunch,
				Error::<T>::CannotClaimYet
			);
			// TODO: Set a reasonable default value
			let final_price = project_info.final_price.unwrap_or(1_000_000_000_0_u64.into());

			Contributions::<T>::try_mutate(
				project_id,
				claimer.clone(),
				|maybe_contribution| -> DispatchResult {
					let mut contribution =
						maybe_contribution.as_mut().ok_or(Error::<T>::ProjectNotExists)?;
					ensure!(contribution.can_claim, Error::<T>::AlreadyClaimed);
					let token_decimals = project.token_information.decimals;
					Self::do_claim_contribution_tokens(
						project_id,
						claimer,
						contribution.amount,
						final_price,
						token_decimals,
					)?;
					contribution.can_claim = false;
					Ok(())
				},
			)?;

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// TODO: Critical: Found a way to perform less iterations on the storage
			for project_id in ProjectsActive::<T>::get().iter() {
				let project_info = ProjectsInfo::<T>::get(project_id);
				match project_info.project_status {
					// Check if we need to start the Funding Round
					// EvaluationEnded -> AuctionRound
					ProjectStatus::EvaluationEnded => {
						let evaluation_period_ends = project_info
							.evaluation_period_ends
							.expect("In EvaluationEnded there always exist evaluation_period_ends");
						Self::handle_auction_start(project_id, now, evaluation_period_ends);
					},
					// Check if we need to move to the Candle Phase of the Auction Round
					// AuctionRound(AuctionPhase::English) -> AuctionRound(AuctionPhase::Candle)
					ProjectStatus::AuctionRound(AuctionPhase::English) => {
						let english_ending_block = project_info
							.auction_metadata
							.expect("In AuctionRound there always exist auction_metadata")
							.english_ending_block;
						Self::handle_auction_candle(project_id, now, english_ending_block);
					},
					// Check if we need to move from the Auction Round of the Community Round
					// AuctionRound(AuctionPhase::Candle) -> CommunityRound
					ProjectStatus::AuctionRound(AuctionPhase::Candle) => {
						let auction_metadata = project_info
							.auction_metadata
							.expect("In AuctionRound there always exist auction_metadata");
						let candle_ending_block = auction_metadata.candle_ending_block;
						let english_ending_block = auction_metadata.english_ending_block;
						Self::handle_community_start(
							project_id,
							now,
							candle_ending_block,
							english_ending_block,
						);
					},
					// Check if Evaluation Round have to end, if true, end it
					// EvaluationRound -> EvaluationEnded
					ProjectStatus::EvaluationRound => {
						let evaluation_period_ends = project_info
							.evaluation_period_ends
							.expect("In EvaluationRound there always exist evaluation_period_ends");
						Self::handle_evaluation_end(project_id, now, evaluation_period_ends);
					},
					// Check if we need to end the Fundind Round
					// CommunityRound -> FundingEnded
					ProjectStatus::CommunityRound => {
						let community_ending_block = project_info
							.auction_metadata
							.expect("In CommunityRound there always exist auction_metadata")
							.community_ending_block;
						Self::handle_community_end(*project_id, now, community_ending_block);
					},
					_ => (),
				}
			}
			// TODO: Set a proper weight
			Weight::from_ref_time(0)
		}

		/// Cleanup the `active_projects` BoundedVec
		fn on_idle(now: T::BlockNumber, _max_weight: Weight) -> Weight {
			for project_id in ProjectsActive::<T>::get().iter() {
				let project_info = ProjectsInfo::<T>::get(project_id);
				if project_info.project_status == ProjectStatus::FundingEnded {
					Self::handle_fuding_end(project_id, now);
				}
			}
			// TODO: Set a proper weight
			Weight::from_ref_time(0)
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
				minimum_price: 1u8.into(),
				ticket_size: TicketSize { minimum: Some(1u8.into()), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				metadata: metadata_hash,
				..Default::default()
			};
			project
		}
	}
}

pub trait ConfigHelper: Config {
	fn fund_account_id(index: Self::ProjectIdentifier) -> AccountIdOf<Self>;
}

impl<T: Config> ConfigHelper for T {
	#[inline(always)]
	fn fund_account_id(index: T::ProjectIdentifier) -> AccountIdOf<Self> {
		Self::PalletId::get().into_sub_account_truncating(index)
	}
}
