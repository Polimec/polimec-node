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

use frame_support::{
	pallet_prelude::ValueQuery,
	traits::{
		tokens::Balance, Currency, Get, LockIdentifier, LockableCurrency, Randomness,
		ReservableCurrency, WithdrawReasons,
	},
	PalletId,
};
use sp_runtime::traits::AccountIdConversion;

use sp_arithmetic::traits::{CheckedAdd, Saturating, Zero};

use polimec_traits::{MemberRole, PolimecMembers};

/// The balance type of this pallet.
pub type BalanceOf<T> = <T as Config>::CurrencyBalance;

/// Identifier for the collection of item.
pub type ProjectIdentifier = u32;

// TODO: Add multiple locks
const LOCKING_ID: LockIdentifier = *b"evaluate";

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

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// The bonding balance.
		type Currency: LockableCurrency<
			Self::AccountId,
			Moment = Self::BlockNumber,
			Balance = Self::CurrencyBalance,
		>;

		/// The bidding balance.
		type BiddingCurrency: ReservableCurrency<Self::AccountId, Balance = Self::CurrencyBalance>;

		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to
		/// `From<u64>`.
		type CurrencyBalance: Balance + From<u64>;

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

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

		// Weight information for extrinsic in this pallet.
		// type WeightInfo: WeightInfo;

		type HandleMembers: PolimecMembers<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn project_ids)]
	/// A global counter for indexing the projects
	/// OnEmpty in this case is GetDefault, so 0.
	pub type ProjectId<T: Config> = StorageValue<_, ProjectIdentifier, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// A global counter used in the randomness generation
	// TODO: Remove it after using the Randomness from BABE's VRF
	/// OnEmpty in this case is GetDefault, so 0.
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// A DoubleMap containing all the the projects that applied for a request for funds
	pub type Projects<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn project_issuer)]
	/// StorageMap (k: ProjectIdentifier, v: T::AccountId) to "reverse lookup" the project issuer so
	/// the users doesn't need to specify each time the project issuer
	pub type ProjectsIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, ProjectIdentifier, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn project_info)]
	/// A DoubleMap containing all the the information for the projects
	pub type ProjectsInfo<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		ProjectInfo<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn projects_active)]
	/// A BoundedVec to list all the "active" Projects
	/// A Project is active if its status is {EvaluationRound, EvaluationEnded, AuctionRound(AuctionPhase), CommunityRound, FundingEnded}
	pub type ProjectsActive<T: Config> =
		StorageValue<_, BoundedVec<ProjectIdentifier, T::ActiveProjectsLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn evaluations)]
	/// Projects in the Evaluation Round
	pub type Evaluations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		EvaluationMetadata<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn auctions)]
	/// Projects in the Auction Round
	pub type Auctions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		AuctionMetadata<T::BlockNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn auctions_info)]
	/// Bids during the Auction Round
	pub type AuctionsInfo<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BidInfo<BalanceOf<T>, T::BlockNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bonds)]
	/// Bonds during the Evaluation Phase
	pub type Bonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn contributions)]
	/// Contributions during the Community Phase
	pub type Contributions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectIdentifier,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A `project_id` was created.
		Created { project_id: ProjectIdentifier, issuer: T::AccountId },
		/// The metadata of `project_id` was modified by `issuer`.
		MetadataEdited { project_id: ProjectIdentifier, issuer: T::AccountId },
		/// The evaluation phase of `project_id` was started by `issuer`.
		EvaluationStarted { project_id: ProjectIdentifier, issuer: T::AccountId },
		/// The evaluation phase of `project_id` was ended by `issuer`.
		EvaluationEnded { project_id: ProjectIdentifier, issuer: T::AccountId },

		/// The auction round of `project_id` started by `issuer` at block `when`.
		AuctionStarted { project_id: ProjectIdentifier, issuer: T::AccountId, when: T::BlockNumber },
		/// The auction round of `project_id` ended by `issuer` at block `when`.
		AuctionEnded { project_id: ProjectIdentifier, issuer: T::AccountId },
		/// The auction round of `project_id` ended by `issuer` at block `when`.
		FundsBonded {
			project_id: ProjectIdentifier,
			issuer: T::AccountId,
			bonder: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		PriceTooLow,
		ParticipantsSizeError,
		TicketSizeError,
		ProjectIdInUse,
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
		TooSoon,
		TooManyActiveProjects,
		NotAuthorized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Funding Application" round
		/// Project applies for funding, providing all required information.
		pub fn create(
			origin: OriginFor<T>,
			project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			ensure!(
				T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
				Error::<T>::NotAuthorized
			);

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
			project_metadata: ProjectMetadata<BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(project_id, &issuer), Error::<T>::ProjectNotExists);
			ensure!(!ProjectsInfo::<T>::get(project_id, &issuer).is_frozen, Error::<T>::Frozen);
			Projects::<T>::mutate(project_id, &issuer, |project| {
				project.as_mut().unwrap().metadata = project_metadata;
			});
			Self::deposit_event(Event::<T>::MetadataEdited { project_id, issuer });
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Evaluation Round"
		pub fn start_evaluation(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(project_id, &issuer), Error::<T>::ProjectNotExists);
			ensure!(
				ProjectsInfo::<T>::get(project_id, &issuer).project_status ==
					ProjectStatus::Application,
				Error::<T>::EvaluationAlreadyStarted
			);
			Self::do_start_evaluation(project_id, &issuer)
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
		pub fn bond(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;
			ensure!(from != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_id, &project_issuer);
			let project = Projects::<T>::get(project_id, &project_issuer)
				.ok_or(Error::<T>::ProjectNotExists)?;
			ensure!(
				project_info.project_status == ProjectStatus::EvaluationRound,
				Error::<T>::EvaluationNotStarted
			);
			ensure!(T::Currency::free_balance(&from) > amount, Error::<T>::InsufficientBalance);

			// Take the value given by the issuer or use the minimum balance any single account may have.
			let minimum_amount =
				project.ticket_size.minimum.unwrap_or_else(T::Currency::minimum_balance);

			// Take the value given by the issuer or use the total amount of issuance in the system.
			let maximum_amount =
				project.ticket_size.maximum.unwrap_or_else(T::Currency::total_issuance);
			ensure!(amount >= minimum_amount, Error::<T>::BondTooLow);
			ensure!(amount <= maximum_amount, Error::<T>::BondTooHigh);

			T::Currency::set_lock(LOCKING_ID, &from, amount, WithdrawReasons::all());
			Bonds::<T>::insert(project_id, &from, amount);
			Evaluations::<T>::mutate(project_id, &project_issuer, |project| {
				project.amount_bonded =
					project.amount_bonded.checked_add(&amount).unwrap_or(project.amount_bonded)
			});
			Self::deposit_event(Event::<T>::FundsBonded {
				project_id,
				issuer: project_issuer,
				bonder: from,
				amount,
			});
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond more `amount` PLMC to evaluate a `project_id` in the "Evaluation Round"
		pub fn rebond(
			_origin: OriginFor<T>,
			_project_id: ProjectIdentifier,
			#[pallet::compact] _amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Funding Round"
		pub fn start_auction(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(Projects::<T>::contains_key(project_id, &issuer), Error::<T>::ProjectNotExists);
			let project_info = ProjectsInfo::<T>::get(project_id, &issuer);
			ensure!(
				project_info.project_status != ProjectStatus::AuctionRound(AuctionPhase::English),
				Error::<T>::AuctionAlreadyStarted
			);
			ensure!(
				project_info.project_status == ProjectStatus::EvaluationEnded,
				Error::<T>::EvaluationNotStarted
			);
			Self::do_start_auction(project_id, &issuer)
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Place a bid in the "Auction Round"
		// TODO: This function currently to simplify uses PLMC as the currency, and not the currency
		// expressed by the project issuer at the project creation stage. This will have to change
		// when XCM is implemented.
		pub fn bid(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
			#[pallet::compact] price: BalanceOf<T>,
			#[pallet::compact] market_cap: BalanceOf<T>,
			// TODO: Add a parameter to specify the currency to use, should be equal to the currency
			// specified in `participation_currencies`
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;

			ensure!(
				T::HandleMembers::is_in(&MemberRole::Professional, &bidder,) ||
					T::HandleMembers::is_in(&MemberRole::Institutional, &bidder),
				Error::<T>::NotAuthorized
			);

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the bidder is not the project_issuer
			ensure!(bidder != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_id, &project_issuer);
			let project = Projects::<T>::get(project_id, &project_issuer)
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

			T::BiddingCurrency::reserve(&bidder, price)
				.map_err(|_| "Bidder can't afford to reserve the amount requested")?;
			let now = <frame_system::Pallet<T>>::block_number();
			let bid_info = BidInfo::new(market_cap, price, now, project.fundraising_target);

			AuctionsInfo::<T>::insert(project_id, bidder, bid_info);

			Ok(())
		}

		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		/// Contribute to the "Community Round"
		pub fn contribute(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
			// Add a parameter to specify the currency to use, should be equal to the currency
			// specified in `participation_currencies`
			// TODO: In future participation_currencies will became an array of currencies, so the
			// currency to use should be in the `participation_currencies` vector/set
		) -> DispatchResult {
			let contributor = ensure_signed(origin)?;

			// TODO: Add the "Retail before, Institutional and Professionals after, if there are still tokens" logic

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the contributor is not the project_issuer
			ensure!(contributor != project_issuer, Error::<T>::ContributionToThemselves);

			ensure!(!Auctions::<T>::contains_key(project_id, &contributor), Error::<T>::NotAllowed);

			let project_info = ProjectsInfo::<T>::get(project_id, &project_issuer);
			let project = Projects::<T>::get(project_id, &project_issuer)
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

			let fund_account = Self::fund_account_id(project_id);
			// TODO: Use the currency chosen by the Issuer
			// TODO: Check the logic
			T::Currency::transfer(
				&contributor,
				&fund_account,
				amount,
				// TODO: Take the ExistenceRequirement as parameter
				frame_support::traits::ExistenceRequirement::KeepAlive,
			)?;

			Contributions::<T>::insert(project_id, contributor, amount);

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			for project_id in ProjectsActive::<T>::get().iter() {
				let project_issuer =
					ProjectsIssuers::<T>::get(project_id).expect("The project issuer is set");
				let project_info = ProjectsInfo::<T>::get(project_id, &project_issuer);
				match project_info.project_status {
					// Check if Evaluation Round have to end, if true, end it
					// EvaluationRound -> EvaluationEnded
					ProjectStatus::EvaluationRound => {
						Self::handle_evaluation_end(project_id, &project_issuer, now);
					},
					// Check if more than 7 days passed since the end of evaluation, if true, start the Funding Round
					// EvaluationEnded -> AuctionRound
					ProjectStatus::EvaluationEnded => {
						Self::handle_auction_start(project_id, &project_issuer, now);
					},
					// Check if we need to move to the Candle Phase of the Auction Round
					// AuctionRound(AuctionPhase::English) -> AuctionRound(AuctionPhase::Candle)
					ProjectStatus::AuctionRound(AuctionPhase::English) => {
						Self::handle_auction_candle(project_id, &project_issuer, now);
					},
					// Check if we need to move from the Auction Round of the Community Round
					// AuctionRound(AuctionPhase::Candle) -> CommunityRound
					ProjectStatus::AuctionRound(AuctionPhase::Candle) => {
						Self::handle_community_start(project_id, &project_issuer, now);
					},
					// Check if we need to end the Fundind Round
					// CommunityRound -> FundingEnded
					ProjectStatus::CommunityRound => {
						Self::handle_community_end(project_id, &project_issuer, now);
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
				let project_issuer =
					ProjectsIssuers::<T>::get(project_id).expect("The project issuer is set");
				let project_info = ProjectsInfo::<T>::get(project_id, &project_issuer);
				if project_info.project_status == ProjectStatus::FundingEnded {
					Self::handle_fuding_end(project_id, &project_issuer, now);
				}
			}
			Weight::from_ref_time(0)
		}
	}
}

use frame_support::{pallet_prelude::*, BoundedVec};
use sp_arithmetic::Perquintill;
use sp_std::{cmp::Reverse, vec::Vec};

impl<T: Config> Pallet<T> {
	pub fn fund_account_id(index: ProjectIdentifier) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(index)
	}

	pub fn do_create(
		project_id: ProjectIdentifier,
		issuer: &T::AccountId,
		project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
	) -> Result<(), DispatchError> {
		let project_info = ProjectInfo {
			is_frozen: false,
			final_price: None,
			created_at: <frame_system::Pallet<T>>::block_number(),
			project_status: ProjectStatus::Application,
			auction_round_end: None,
		};

		ProjectsInfo::<T>::insert(project_id, issuer, project_info);
		Projects::<T>::insert(project_id, issuer, project);
		ProjectsIssuers::<T>::insert(project_id, issuer);
		ProjectId::<T>::mutate(|n| *n += 1);

		Self::deposit_event(Event::<T>::Created { project_id, issuer: issuer.clone() });
		Ok(())
	}

	pub fn do_start_evaluation(
		project_id: ProjectIdentifier,
		who: &T::AccountId,
	) -> Result<(), DispatchError> {
		let evaluation_metadata = EvaluationMetadata {
			evaluation_period_ends: <frame_system::Pallet<T>>::block_number() +
				T::EvaluationDuration::get(),
			amount_bonded: BalanceOf::<T>::zero(),
		};
		Evaluations::<T>::insert(project_id, who, evaluation_metadata);
		ProjectsInfo::<T>::mutate(project_id, who, |project_info| {
			project_info.is_frozen = true;
			project_info.project_status = ProjectStatus::EvaluationRound;
		});
		ProjectsActive::<T>::try_append(project_id)
			.map_err(|()| Error::<T>::TooManyActiveProjects)?;

		Self::deposit_event(Event::<T>::EvaluationStarted { project_id, issuer: who.clone() });
		Ok(())
	}

	pub fn do_start_auction(
		project_id: ProjectIdentifier,
		who: &T::AccountId,
	) -> Result<(), DispatchError> {
		ProjectsInfo::<T>::mutate(project_id, who, |project_info| {
			project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
		});

		let current_block_number = <frame_system::Pallet<T>>::block_number();
		let english_ending_block = current_block_number + T::EnglishAuctionDuration::get();
		let candle_ending_block = english_ending_block + T::CandleAuctionDuration::get();
		let community_ending_block = candle_ending_block + T::CommunityRoundDuration::get();

		let auction_metadata = AuctionMetadata {
			starting_block: current_block_number,
			english_ending_block,
			candle_ending_block,
			community_ending_block,
		};
		Auctions::<T>::insert(project_id, who, auction_metadata);

		Self::deposit_event(Event::<T>::AuctionStarted {
			project_id,
			issuer: who.clone(),
			when: current_block_number,
		});
		Ok(())
	}

	pub fn handle_evaluation_end(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		now: T::BlockNumber,
	) {
		let evaluation_detail = Evaluations::<T>::get(project_id, project_issuer);
		if now >= evaluation_detail.evaluation_period_ends {
			ProjectsInfo::<T>::mutate(project_id, project_issuer, |project_info| {
				project_info.project_status = ProjectStatus::EvaluationEnded;
			});
			Self::deposit_event(Event::<T>::EvaluationEnded {
				project_id: *project_id,
				issuer: project_issuer.clone(),
			});
		}
	}

	pub fn handle_auction_start(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		now: T::BlockNumber,
	) {
		let evaluation_detail = Evaluations::<T>::get(project_id, project_issuer);
		if evaluation_detail.evaluation_period_ends + T::EnglishAuctionDuration::get() <= now {
			// TODO: Unused error, more tests needed
			// TODO: Here the start_auction is "free", check the Weight
			let _ = Self::do_start_auction(*project_id, project_issuer);
		}
	}

	pub fn handle_auction_candle(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		now: T::BlockNumber,
	) {
		let auction_detail = Auctions::<T>::get(project_id, project_issuer);
		if now >= auction_detail.english_ending_block {
			ProjectsInfo::<T>::mutate(project_id, project_issuer, |project_info| {
				project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
			});
		}
	}

	pub fn handle_community_start(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		now: T::BlockNumber,
	) {
		let auction_detail = Auctions::<T>::get(project_id, project_issuer);
		if now >= auction_detail.candle_ending_block {
			let project =
				Projects::<T>::get(project_id, project_issuer).expect("meaningful message");
			ProjectsInfo::<T>::mutate(project_id, project_issuer.clone(), |project_info| {
				project_info.project_status = ProjectStatus::CommunityRound;
				project_info.final_price = Some(
					Self::calculate_final_price(*project_id, project.fundraising_target)
						.expect("placeholder_function"),
				);
				project_info.auction_round_end = Some(Self::select_random_block(
					auction_detail.english_ending_block + 1_u8.into(),
					auction_detail.candle_ending_block,
				));
			});
		}
	}

	pub fn handle_community_end(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		now: T::BlockNumber,
	) {
		let auction_detail = Auctions::<T>::get(project_id, project_issuer);
		if now >= auction_detail.community_ending_block {
			ProjectsInfo::<T>::mutate(project_id, project_issuer.clone(), |project_info| {
				project_info.project_status = ProjectStatus::FundingEnded;
			});

			// TODO: Mint the "Contribution Tokens"
			// TODO: Assign the CTs to the participants of the Funding Round
		}
	}

	pub fn handle_fuding_end(
		project_id: &ProjectIdentifier,
		project_issuer: &T::AccountId,
		_now: T::BlockNumber,
	) {
		// Project identified by project_id is no longer "active"
		ProjectsActive::<T>::mutate(|active_projects| {
			if let Some(pos) = active_projects.iter().position(|x| x == project_id) {
				active_projects.remove(pos);
			}
		});

		ProjectsInfo::<T>::mutate(project_id, project_issuer.clone(), |project_info| {
			project_info.project_status = ProjectStatus::ReadyToLaunch;
		});
	}

	pub fn calculate_final_price(
		project_id: ProjectIdentifier,
		total_allocation_size: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let mut bids: Vec<BidInfo<BalanceOf<T>, T::BlockNumber>> =
			AuctionsInfo::<T>::iter_prefix_values(project_id).collect();
		bids.sort_by_key(|bid| Reverse(bid.market_cap));
		Self::final_price_logic(bids, total_allocation_size)
	}

	pub fn final_price_logic(
		mut bids: Vec<BidInfo<<T as Config>::CurrencyBalance, T::BlockNumber>>,
		total_allocation_size: <T as Config>::CurrencyBalance,
	) -> Result<<T as Config>::CurrencyBalance, DispatchError> {
		let mut fundraising_amount = BalanceOf::<T>::zero();
		let mut final_price = BalanceOf::<T>::zero();
		for (idx, bid) in bids.iter_mut().enumerate() {
			let old_amount = fundraising_amount;
			fundraising_amount += bid.amount;
			if fundraising_amount > total_allocation_size {
				bid.amount = total_allocation_size.saturating_sub(old_amount);
				bid.ratio = Perquintill::from_rational(bid.amount, total_allocation_size);
				bids.truncate(idx + 1);
				break
			}
		}
		for bid in bids {
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
}
