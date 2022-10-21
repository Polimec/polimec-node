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
	traits::{Currency, Get, LockIdentifier, LockableCurrency, WithdrawReasons},
	PalletId,
};
use sp_runtime::traits::{AccountIdConversion, CheckedAdd, Zero};

/// The balance type of this pallet.
pub type BalanceOf<T> = <T as Config>::CurrencyBalance;

/// Identifier for the collection of item.
pub type ProjectIdentifier = u32;

const LOCKING_ID: LockIdentifier = *b"evaluate";

#[frame_support::pallet]
pub mod pallet {

	use super::*;
	use frame_support::pallet_prelude::{ValueQuery, *};
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// The bonding balance.
		type Currency: LockableCurrency<
			Self::AccountId,
			Moment = Self::BlockNumber,
			Balance = Self::CurrencyBalance,
		>;

		/// Just the `Currency::Balance` type; we have this item to allow us to constrain it to
		/// `From<u64>`.
		type CurrencyBalance: sp_runtime::traits::AtLeast32BitUnsigned
			+ codec::FullCodec
			+ Copy
			+ MaybeSerializeDeserialize
			+ sp_std::fmt::Debug
			+ Default
			+ From<u64>
			+ TypeInfo
			+ MaxEncodedLen;

		#[pallet::constant]
		type EvaluationDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type AuctionDuration: Get<Self::BlockNumber>;

		/// `PalletId` for the funding pallet. An appropriate value could be
		/// `PalletId(*b"py/cfund")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		// TODO: Should be helpful for allowing the calls only by the user in the set of
		// { Issuer, Retail, Professional, Institutional }
		// Project creation is only allowed if the origin attempting it and the
		// collection are in this set.
		// type CreateOrigin: EnsureOriginWithArg<
		//	Self::Origin,
		//	Self::CollectionId,
		//	Success = Self::AccountId,
		//>;

		// type ForceOrigin: EnsureOrigin<Self::Origin>;

		// Weight information for extrinsic in this pallet.
		// type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn project_ids)]
	/// A global counter for indexing the projects
	/// OnEmpty in this case is GetDefault, so 0.
	pub type ProjectId<T: Config> = StorageValue<_, ProjectIdentifier, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// A DoubleMap containing all the the projects that applied for a request for funds
	pub type Projects<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
	>;

	/// TODO: We can add a StorageMap (k: ProjectIdentifier, v: T::AccountId) to
	/// "reverse lookup" the project issuer so the users doesn't need to specify each time the
	/// project issuer
	#[pallet::storage]
	#[pallet::getter(fn project_issuer)]
	pub type ProjectsIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, ProjectIdentifier, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn projects_info)]
	/// A DoubleMap containing all the the information for the projects
	pub type ProjectsInfo<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		ProjectInfo<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn evaluations)]
	/// Projects in the evaluation phase
	pub type Evaluations<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		EvaluationMetadata<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn auctions)]
	/// Projects in the auction phase
	pub type Auctions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		AuctionMetadata<T::BlockNumber, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bonds)]
	/// Bonds during the Evaluation Phase
	pub type Bonds<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		BondingLedger<T::AccountId, BalanceOf<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn contributions)]
	/// Contribution during the Community Phase
	pub type Contributions<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		ProjectIdentifier,
		BondingLedger<T::AccountId, BalanceOf<T>>,
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
		AuctionEnded {
			project_id: ProjectIdentifier,
			issuer: T::AccountId,
			// when: T::BlockNumber,
		},
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
		EvaluationNotStarted,
		AuctionAlreadyStarted,
		AuctionNotStarted,
		Frozen,
		BondTooLow,
		BondTooHigh,
		InsufficientBalance,
		TooSoon,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Funding Application" phase
		pub fn create(
			origin: OriginFor<T>,
			project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let issuer = ensure_signed(origin)?;

			match project.validity_check() {
				Err(error) => match error {
					ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
					ValidityError::ParticipantsSizeError =>
						Err(Error::<T>::ParticipantsSizeError.into()),
					ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
				},
				Ok(()) => {
					let project_id = ProjectId::<T>::get();
					Self::do_create(issuer, project, project_id)
				},
			}
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Edit the public metadata of a project if "Evaluation Phase" (or one of the following
		/// phases) is not yet started
		pub fn edit_metadata(
			origin: OriginFor<T>,
			project_metadata: ProjectMetadata<BoundedVec<u8, T::StringLimit>>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(
				Projects::<T>::contains_key(issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			ensure!(
				!ProjectsInfo::<T>::get(issuer.clone(), project_id).is_frozen,
				Error::<T>::Frozen
			);
			Projects::<T>::mutate(issuer.clone(), project_id, |project| {
				project.as_mut().unwrap().metadata = project_metadata;
				Self::deposit_event(Event::<T>::MetadataEdited {
					project_id,
					issuer: issuer.clone(),
				});
			});
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Evaluation Phase"
		pub fn start_evaluation(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(
				Projects::<T>::contains_key(issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			ensure!(
				ProjectsInfo::<T>::get(issuer.clone(), project_id).evaluation_status ==
					EvaluationStatus::NotYetStarted,
				Error::<T>::EvaluationAlreadyStarted
			);
			Self::do_start_evaluation(issuer, project_id)
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond their PLMC to evaluate a Project
		pub fn bond(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;
			ensure!(from != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_issuer.clone(), project_id);
			let project = Projects::<T>::get(project_issuer.clone(), project_id);
			ensure!(
				project_info.evaluation_status == EvaluationStatus::Started,
				Error::<T>::EvaluationNotStarted
			);
			ensure!(T::Currency::free_balance(&from) > amount, Error::<T>::InsufficientBalance);

			let minimum_amount = project
				.as_ref()
				.expect("Project exists")
				.ticket_size
				.minimum
				// Take the value given by the issuer or use the minimum balance any single account
				// may have.
				.unwrap_or_else(T::Currency::minimum_balance);

			let maximum_amount = project
				.as_ref()
				.expect("Project exists")
				.ticket_size
				.maximum
				// Take the value given by the issuer or use the total amount of issuance in the
				// system.
				.unwrap_or_else(T::Currency::total_issuance);
			ensure!(amount >= minimum_amount, Error::<T>::BondTooLow);
			ensure!(amount <= maximum_amount, Error::<T>::BondTooHigh);

			T::Currency::set_lock(LOCKING_ID, &from, amount, WithdrawReasons::all());
			Bonds::<T>::insert(
				project_issuer.clone(),
				project_id,
				BondingLedger { stash: from.clone(), amount_bonded: amount },
			);
			Evaluations::<T>::mutate(project_issuer.clone(), project_id, |project| {
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

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Evaluators can bond more of their PLMC to evaluate a Project
		pub fn rebond(
			_origin: OriginFor<T>,
			_project_id: ProjectIdentifier,
			#[pallet::compact] _amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Start the "Auction Round"
		pub fn start_auction(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			ensure!(
				Projects::<T>::contains_key(issuer.clone(), project_id),
				Error::<T>::ProjectNotExists
			);
			let project_info = ProjectsInfo::<T>::get(issuer.clone(), project_id);
			ensure!(
				project_info.evaluation_status == EvaluationStatus::Ended,
				Error::<T>::EvaluationNotStarted
			);
			ensure!(
				project_info.auction_status == AuctionStatus::NotYetStarted,
				Error::<T>::AuctionAlreadyStarted
			);
			let evaluation_detail = Evaluations::<T>::get(issuer.clone(), project_id);
			ensure!(
				<frame_system::Pallet<T>>::block_number() >=
					evaluation_detail.evaluation_period_ends,
				Error::<T>::TooSoon
			);

			Self::do_start_auction(issuer, project_id)
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		/// Place a bid in the "Auction Round"
		pub fn bid(
			origin: OriginFor<T>,
			project_id: ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
			// Add a parameter to specify the currency to use, should be equal to the currency
			// specified in `participation_currencies`
			// TODO: In future participation_currencies will became an array of currencies, so the
			// currency to use should be IN the `participation_currencies` vector/set
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the bidder is not the project_issuer
			ensure!(bidder != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_issuer.clone(), project_id);
			let project = Projects::<T>::get(project_issuer, project_id)
				.expect("Project exists, already checked in previous ensure");

			// Make sure Auction Round is started
			ensure!(
				project_info.auction_status == AuctionStatus::Started(AuctionPhase::English),
				Error::<T>::AuctionNotStarted
			);

			// Make sure the bidder can actually perform the bid
			let free_balance_of = T::Currency::free_balance(&bidder);
			ensure!(free_balance_of > amount, Error::<T>::InsufficientBalance);

			// Make sure the bid amount is greater than the minimum_price specified by the issuer
			ensure!(free_balance_of > project.minimum_price, Error::<T>::BondTooLow);

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
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

			// Make sure project exists
			let project_issuer =
				ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectNotExists)?;

			// Make sure the contributor is not the project_issuer
			ensure!(contributor != project_issuer, Error::<T>::ContributionToThemselves);

			let project_info = ProjectsInfo::<T>::get(project_issuer.clone(), project_id);
			let project = Projects::<T>::get(project_issuer.clone(), project_id)
				.expect("Project exists, already checked in previous ensure");

			// Make sure Community Round is started
			ensure!(
				project_info.auction_status == AuctionStatus::Started(AuctionPhase::Candle),
				Error::<T>::AuctionNotStarted
			);

			// Make sure the contributor can actually perform the bid
			let free_balance_of = T::Currency::free_balance(&contributor);
			ensure!(free_balance_of > amount, Error::<T>::InsufficientBalance);

			// Make sure the bid amount is greater than the minimum_price specified by the issuer
			ensure!(free_balance_of > project.minimum_price, Error::<T>::BondTooLow);

			let fund_account = Self::fund_account_id(project_id);
			// TODO: Use the currency chosen by the Issuer
			T::Currency::transfer(
				&contributor,
				&fund_account,
				amount,
				// TODO: Take the ExistenceRequirement as parameter
				frame_support::traits::ExistenceRequirement::KeepAlive,
			)?;

			Contributions::<T>::insert(
				project_issuer,
				project_id,
				BondingLedger { stash: contributor, amount_bonded: amount },
			);

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// TODO; Check if we can move any part of the logic into the "on_idle" hook
			// Maybe we can use that space block to re-order/cleanup the storage

			for (project_issuer, project_id, evaluation_detail) in Evaluations::<T>::iter() {
				// Stop the evaluation period
				let project_info = ProjectsInfo::<T>::get(project_issuer.clone(), project_id);
				if now >= evaluation_detail.evaluation_period_ends &&
					project_info.evaluation_status == EvaluationStatus::Started
				{
					ProjectsInfo::<T>::mutate(project_issuer.clone(), project_id, |project_info| {
						project_info.evaluation_status = EvaluationStatus::Ended;
					});
				}
				// If more than 7 days are passed from the end of the evaluation, start the auction
				if evaluation_detail.evaluation_period_ends + T::AuctionDuration::get() <= now &&
					project_info.evaluation_status == EvaluationStatus::Ended &&
					project_info.auction_status == AuctionStatus::NotYetStarted
				{
					Auctions::<T>::mutate(project_issuer.clone(), project_id, |auction| {
						auction.starting_block = now;
					});
					ProjectsInfo::<T>::mutate(project_issuer.clone(), project_id, |project_info| {
						project_info.auction_status = AuctionStatus::Started(AuctionPhase::English);
					});
					Self::deposit_event(Event::<T>::AuctionStarted {
						project_id,
						issuer: project_issuer.clone(),
						when: now,
					});
				}
				// TODO: CHECK if it's ok to remove a project from the Evaluations storage
				// this is done in order to maintain stable the number of operations to do in the
				// hook
				Evaluations::<T>::remove(project_issuer, project_id);
			}
			// TODO: Check why return 0 as Weight
			0
		}
	}
}

use frame_support::{pallet_prelude::DispatchError, BoundedVec};

impl<T: Config> Pallet<T> {
	pub fn fund_account_id(index: ProjectIdentifier) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(index)
	}

	pub fn do_create(
		issuer: T::AccountId,
		project: Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
		project_id: ProjectIdentifier,
	) -> Result<(), DispatchError> {
		Projects::<T>::insert(issuer.clone(), project_id, project);
		ProjectsIssuers::<T>::insert(project_id, issuer.clone());

		let project_info = ProjectInfo {
			is_frozen: false,
			final_price: None,
			created_at: <frame_system::Pallet<T>>::block_number(),
			evaluation_status: EvaluationStatus::NotYetStarted,
			auction_status: AuctionStatus::NotYetStarted,
		};
		ProjectsInfo::<T>::insert(issuer.clone(), project_id, project_info);

		let evaluation_metadata = EvaluationMetadata {
			// TODO: I REALLY don't like to initialize this value using the current block number,
			// probably an Option<T::BlockNumber> should be a better choice, but at the moment it
			// would complicate the code in many other functions
			evaluation_period_ends: <frame_system::Pallet<T>>::block_number(),
			amount_bonded: BalanceOf::<T>::zero(),
		};
		Evaluations::<T>::insert(issuer.clone(), project_id, evaluation_metadata);

		ProjectId::<T>::mutate(|n| *n += 1);

		Self::deposit_event(Event::<T>::Created { project_id, issuer });
		Ok(())
	}

	pub fn do_start_evaluation(
		who: T::AccountId,
		project_id: ProjectIdentifier,
	) -> Result<(), DispatchError> {
		ProjectsInfo::<T>::mutate(who.clone(), project_id, |project_info| {
			project_info.is_frozen = true;
			project_info.evaluation_status = EvaluationStatus::Started;
		});
		Evaluations::<T>::mutate(who.clone(), project_id, |project_metadata| {
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			project_metadata.evaluation_period_ends =
				current_block_number + T::EvaluationDuration::get();
		});
		Self::deposit_event(Event::<T>::EvaluationStarted { project_id, issuer: who.clone() });

		let auction_metadata = AuctionMetadata { ..Default::default() };
		Auctions::<T>::insert(who, project_id, auction_metadata);
		Ok(())
	}

	pub fn do_start_auction(
		who: T::AccountId,
		project_id: ProjectIdentifier,
	) -> Result<(), DispatchError> {
		ProjectsInfo::<T>::mutate(who.clone(), project_id, |project_info| {
			project_info.is_frozen = true;
			project_info.auction_status = AuctionStatus::Started(AuctionPhase::English);
		});
		Auctions::<T>::mutate(who.clone(), project_id, |project| {
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			project.starting_block = current_block_number;
			Self::deposit_event(Event::<T>::AuctionStarted {
				project_id,
				issuer: who,
				when: current_block_number,
			});
		});
		Ok(())
	}
}
