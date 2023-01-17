// TODO: Insert License Header

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

use frame_support::{
	pallet_prelude::ValueQuery,
	traits::{
		tokens::{
			fungibles::{metadata::Mutate as MetadataMutate, Create, InspectMetadata, Mutate},
			Balance,
		},
		Currency, Get, LockIdentifier, LockableCurrency, Randomness, ReservableCurrency,
		WithdrawReasons,
	},
	PalletId,
};
use polimec_traits::{MemberRole, PolimecMembers};
use sp_arithmetic::traits::{Saturating, Zero};
use sp_runtime::{
	traits::{AccountIdConversion, Hash},
	FixedPointNumber, FixedPointOperand, FixedU128, Perbill,
};
use sp_std::ops::AddAssign;

pub type ProjectOf<T> = Project<
	<T as frame_system::Config>::AccountId,
	BoundedVec<u8, <T as Config>::StringLimit>,
	BalanceOf<T>,
	<T as frame_system::Config>::Hash,
>;
/// The balance type of this pallet.
type BalanceOf<T> = <T as Config>::CurrencyBalance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

// TODO: Add multiple locks
const LOCKING_ID: LockIdentifier = *b"evaluate";

pub trait Identifiable =
	Member + Parameter + Copy + MaxEncodedLen + Default + AddAssign + From<u32>;
// TODO: + MaybeSerializeDeserialize: Maybe needed for JSON serialization @ Genesis: https://github.com/paritytech/substrate/issues/12738#issuecomment-1320921201

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
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
	pub type ProjectId<T: Config> = StorageValue<_, T::ProjectIdentifier, ValueQuery>;

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
		// TODO: Create a new type for the tuple
		BoundedVec<(T::BlockNumber, BidInfo<BalanceOf<T>, T::AccountId>), T::MaximumBidsPerProject>,
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
					let project_id = ProjectId::<T>::get();
					Self::do_create(project_id, &issuer, project)
				},
			}
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Edit the `project_metadata` of a `project_id` if "Evaluation Round" is not yet started
		pub fn edit_metadata(
			origin: OriginFor<T>,
			project_metadata_hash: T::Hash,
			project_id: T::ProjectIdParameter,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			let project_id = project_id.into();

			ensure!(ProjectsIssuers::<T>::contains_key(project_id), Error::<T>::ProjectNotExists);
			ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);
			ensure!(!ProjectsInfo::<T>::get(project_id).is_frozen, Error::<T>::Frozen);
			ensure!(Images::<T>::contains_key(project_metadata_hash), Error::<T>::ProjectNotExists);

			Projects::<T>::try_mutate(project_id, |maybe_project| -> DispatchResult {
				let project = maybe_project.as_mut().ok_or(Error::<T>::ProjectNotExists)?;
				project.metadata = project_metadata_hash;
				Self::deposit_event(Event::MetadataEdited { project_id });
				Ok(())
			})?;
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Evaluation Round" of a `project_id`
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

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
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

			T::Currency::set_lock(LOCKING_ID, &from, amount, WithdrawReasons::all());
			// TODO: Unlock the PLMC when it's the right time
			Bonds::<T>::insert(project_id, &from, amount);
			Self::deposit_event(Event::<T>::FundsBonded { project_id, amount });
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond more `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
		pub fn rebond(
			_origin: OriginFor<T>,
			_project_id: T::ProjectIdentifier,
			#[pallet::compact] _amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Funding Round" of a `project_id`
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

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Place a bid in the "Auction Round"
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
				bidder.clone(),
				multiplier,
			);
			let new_bid = (now, bid.clone());

			match AuctionsInfo::<T>::try_append(project_id, &new_bid) {
				Ok(_) => {
					// Reserve the new bid
					T::BiddingCurrency::reserve(&bidder, price)?;
					// TODO: Send an XCM message to Statemine to transfer amount * multiplier USDT to the PalletId Account
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
					let (index, (_, lowest_bid)) = bids
							.iter()
							.enumerate()
							.min_by_key(|&(_, bid)| bid)
							.expect("This code runs only if the vector is full, so there is always a minimum; qed");
					// Make sure the bid is greater than the last bid
					if bid > *lowest_bid {
						// Reserve the new bid
						T::BiddingCurrency::reserve(&bidder, price)?;
						// Unreserve the lowest bid
						T::BiddingCurrency::unreserve(&lowest_bid.bidder, lowest_bid.amount);
						// Remove the lowest bid from the AuctionsInfo
						bids.remove(index);
						// Add the new bid to the AuctionsInfo, this should never fail since we just removed an element
						bids.try_push(new_bid)
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
			let final_price = project_info
				.final_price
				.expect("The final price is set, already checked in previous ensure");

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
					_ => (),
				}
			}
			// TODO: Set a proper weight
			Weight::from_ref_time(0)
		}

		/// Cleanup the `active_projects` BoundedVec
		fn on_finalize(now: T::BlockNumber) {
			for project_id in ProjectsActive::<T>::get().iter() {
				let project_info = ProjectsInfo::<T>::get(project_id);
				match project_info.project_status {
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
		fn create_dummy_project(
			destinations_account: T::AccountId,
			metadata_hash: T::Hash,
		) -> ProjectOf<T>;
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl<T: Config> BenchmarkHelper<T> for () {
		fn create_project_id_parameter(id: u32) -> T::ProjectIdParameter {
			id.into()
		}
		fn create_dummy_project(
			destinations_account: T::AccountId,
			metadata_hash: T::Hash,
		) -> ProjectOf<T> {
			let project: ProjectOf<T> =
			// TODO: Create a default project meaingful for the benchmarking
				Project {
					minimum_price: 1u8.into(),
					ticket_size: TicketSize { minimum: Some(1u8.into()), maximum: None },
					participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
					destinations_account,
					metadata: metadata_hash,
					// ..Default::default() doesn't work: the trait `std::default::Default` is not implemented for `<T as frame_system::Config>::AccountId`
					conversion_rate: 1u8.into(),
					funding_thresholds: Default::default(),
					fundraising_target: Default::default(),
					participation_currencies: Default::default(),
					token_information: Default::default(),
					total_allocation_size: Default::default(),
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

use frame_support::{pallet_prelude::*, BoundedVec};
use sp_std::{borrow::Cow, cmp::Reverse, vec::Vec};

impl<T: Config> Pallet<T> {
	/// Store an image on chain.
	///
	/// TODO: We verify that the preimage is within the bounds of what the pallet supports.
	fn note_bytes(preimage: Cow<[u8]>, issuer: &T::AccountId) -> Result<(), DispatchError> {
		// TODO: Validate and check if the preimage is a valid JSON conforming with our needs
		// TODO: Check if we can use serde/serde_json in a no_std environment

		let hash = T::Hashing::hash(&preimage);
		Images::<T>::insert(hash, issuer);

		Self::deposit_event(Event::Noted { hash });

		Ok(())
	}

	pub fn do_create(
		project_id: T::ProjectIdentifier,
		issuer: &T::AccountId,
		project: ProjectOf<T>,
	) -> Result<(), DispatchError> {
		let project_info = ProjectInfo {
			is_frozen: false,
			final_price: None,
			created_at: <frame_system::Pallet<T>>::block_number(),
			project_status: ProjectStatus::Application,
			evaluation_period_ends: None,
			auction_metadata: None,
		};

		Projects::<T>::insert(project_id, project);
		ProjectsInfo::<T>::insert(project_id, project_info);
		ProjectsIssuers::<T>::insert(project_id, issuer);
		ProjectId::<T>::mutate(|n| *n += 1_u32.into());

		Self::deposit_event(Event::<T>::Created { project_id });
		Ok(())
	}

	pub fn do_start_evaluation(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		let evaluation_period_ends =
			<frame_system::Pallet<T>>::block_number() + T::EvaluationDuration::get();

		ProjectsActive::<T>::try_append(project_id)
			.map_err(|()| Error::<T>::TooManyActiveProjects)?;

		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.is_frozen = true;
			project_info.project_status = ProjectStatus::EvaluationRound;
			project_info.evaluation_period_ends = Some(evaluation_period_ends);
		});

		Self::deposit_event(Event::<T>::EvaluationStarted { project_id });
		Ok(())
	}

	pub fn do_start_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		let current_block_number = <frame_system::Pallet<T>>::block_number();
		let english_ending_block = current_block_number + T::EnglishAuctionDuration::get();
		let candle_ending_block = english_ending_block + T::CandleAuctionDuration::get();
		let community_ending_block = candle_ending_block + T::CommunityRoundDuration::get();

		let auction_metadata = AuctionMetadata {
			starting_block: current_block_number,
			english_ending_block,
			candle_ending_block,
			community_ending_block,
			random_ending_block: None,
		};
		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
			project_info.auction_metadata = Some(auction_metadata);
		});

		Self::deposit_event(Event::<T>::AuctionStarted { project_id, when: current_block_number });
		Ok(())
	}

	pub fn handle_evaluation_end(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
	) {
		if now >= evaluation_period_ends {
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				project_info.project_status = ProjectStatus::EvaluationEnded;
			});
			Self::deposit_event(Event::<T>::EvaluationEnded { project_id: *project_id });
		}
	}

	pub fn handle_auction_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
	) {
		if evaluation_period_ends + T::EnglishAuctionDuration::get() <= now {
			// TODO: Unused error, more tests needed
			// TODO: Here the start_auction is "free", check the Weight
			let _ = Self::do_start_auction(*project_id);
		}
	}

	pub fn handle_auction_candle(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) {
		if now >= english_ending_block {
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
			});
		}
	}

	pub fn handle_community_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		candle_ending_block: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) {
		if now >= candle_ending_block {
			// TODO: Move fundraising_target to AuctionMetadata
			let project = Projects::<T>::get(project_id).expect("Project must exist");
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				let mut auction_metadata =
					project_info.auction_metadata.as_mut().expect("Auction must exist");
				let end_block = Self::select_random_block(
					english_ending_block + 1_u8.into(),
					candle_ending_block,
				);
				project_info.project_status = ProjectStatus::CommunityRound;
				auction_metadata.random_ending_block = Some(end_block);
				project_info.final_price = Some(
					Self::calculate_final_price(*project_id, project.fundraising_target, end_block)
						.expect("placeholder_function"),
				);
			});
		}
	}

	pub fn handle_community_end(
		project_id: T::ProjectIdentifier,
		now: T::BlockNumber,
		community_ending_block: T::BlockNumber,
	) {
		if now >= community_ending_block {
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				project_info.project_status = ProjectStatus::FundingEnded;
			});
		};

		let issuer =
			ProjectsIssuers::<T>::get(project_id).expect("The issuer exists, already tested.");
		let project = Projects::<T>::get(project_id).expect("The project exists, already tested.");
		let token_information = project.token_information;

		// TODO: Unused result
		let _ = T::Assets::create(project_id, issuer.clone(), false, 1_u32.into());
		// TODO: Unused result
		let _ = T::Assets::set(
			project_id,
			&issuer,
			token_information.name.into(),
			token_information.symbol.into(),
			token_information.decimals,
		);
	}

	pub fn handle_fuding_end(project_id: &T::ProjectIdentifier, _now: T::BlockNumber) {
		// Project identified by project_id is no longer "active"
		ProjectsActive::<T>::mutate(|active_projects| {
			if let Some(pos) = active_projects.iter().position(|x| x == project_id) {
				active_projects.remove(pos);
			}
		});

		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.project_status = ProjectStatus::ReadyToLaunch;
		});
	}

	pub fn calculate_final_price(
		project_id: T::ProjectIdentifier,
		total_allocation_size: BalanceOf<T>,
		end_block: T::BlockNumber,
	) -> Result<BalanceOf<T>, DispatchError> {
		// Get all the bids that were made before the end of the candle
		// TODO: Here we are not saving the modified bids, we should do it
		// TODO: Maybe add a new storage like "FinalBids(project_id) -> Vec<(BlockNumber, BidInfo)>"
		// Or maybe we can just modify the "AuctionsInfo" storage if we are sure that we will not need the discarded bids
		let mut bids = AuctionsInfo::<T>::get(project_id);
		bids.retain(|(block, _)| block <= &end_block);
		// TODO: Unreserve the funds of the bids that were made after the end of the candle

		// Sort the bids by market cap
		// If we store the bids in a sorted way we can avoid this step

		bids.sort_by_key(|(_, bid)| Reverse(bid.market_cap));
		// Calculate the final price
		let mut fundraising_amount = BalanceOf::<T>::zero();
		let mut final_price = BalanceOf::<T>::zero();
		for (idx, (_, bid)) in bids.iter_mut().enumerate() {
			let old_amount = fundraising_amount;
			fundraising_amount += bid.amount;
			if fundraising_amount > total_allocation_size {
				bid.amount = total_allocation_size.saturating_sub(old_amount);
				bid.ratio = Perbill::from_rational(bid.amount, total_allocation_size);
				bids.truncate(idx + 1);
				// TODO: refund the rest of the amount to the bidders
				// TODO: Maybe in an on_idle hook ?
				break
			}
		}

		for (_, bid) in bids {
			let weighted_price = bid.ratio.mul_ceil(bid.market_cap);
			final_price = final_price.saturating_add(weighted_price);
		}
		Ok(final_price)
	}

	pub fn select_random_block(
		candle_starting_block: T::BlockNumber,
		candle_ending_block: T::BlockNumber,
	) -> T::BlockNumber {
		let nonce = Self::get_and_increment_nonce();
		let (random_value, _known_since) = T::Randomness::random(&nonce);
		let random_block = <T::BlockNumber>::decode(&mut random_value.as_ref())
			.expect("secure hashes should always be bigger than the block number; qed");
		let block_range = candle_ending_block - candle_starting_block;

		candle_starting_block + (random_block % block_range)
	}

	fn get_and_increment_nonce() -> Vec<u8> {
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce.wrapping_add(1));
		nonce.encode()
	}

	fn do_claim_contribution_tokens(
		project_id: T::ProjectIdentifier,
		claimer: T::AccountId,
		final_price: BalanceOf<T>,
		contribution_amount: BalanceOf<T>,
		token_decimals: u8,
	) -> Result<(), DispatchError> {
		let amount =
			Self::calculate_claimable_tokens(contribution_amount, final_price, token_decimals);
		T::Assets::mint_into(project_id, &claimer, amount)?;
		Ok(())
	}

	// This functiion is kept separate from the `do_claim_contribution_tokens` for easier testing
	fn calculate_claimable_tokens(
		contribution_amount: BalanceOf<T>,
		final_price: BalanceOf<T>,
		token_decimals: u8,
	) -> BalanceOf<T> {
		let decimals = 10_u64.saturating_pow(token_decimals.into());
		let unit: BalanceOf<T> = BalanceOf::<T>::from(decimals);
		FixedU128::saturating_from_rational(contribution_amount, final_price)
			.saturating_mul_int(unit)
	}
}
