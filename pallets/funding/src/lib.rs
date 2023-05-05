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

//! # Funding Pallet
//!
//! Polimec's main business logic. It allows users to create, evaluate, and fund projects.
//!
//! It rewards project evaluators and contributors with `Contribution Tokens`. These tokens
//! can be redeemed for a project's native tokens, after their parachain is deployed on mainnet.
//! ## 👷 Work in progress 🏗️
//! Expect major changes between PRs
//!
//! ## Overview
//! The official logic for Polimec's blockchain can be found at our [whitepaper](https://polimec.link/whitepaper).
//!
//! There are 3 types of users in Polimec:
//! - **Issuers**: They create projects and are responsible for their success.
//! - **Evaluators**: They evaluate projects and are rewarded for their work.
//! - **Contributors**: They contribute financially to projects and are rewarded on the basis of their contribution
//!
//! A contributor, depending on their investor profile, can participate in different rounds of a project's funding.
//!
//! There are 3 types of contributors:
//! - **Institutional**
//! - **Professional**
//! - **Retail**
//!
//! Basic flow of a project's lifecycle:
//!
//!
//! | Step                      | Description                                                                                                                                                                                                                                                                                                                                                                                                 | Resulting Project State                                             |
//! |---------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------|
//! | Creation                  | Issuer creates a project with the [`create()`](Pallet::create) extrinsic.                                                                                                                                                                                                                                                                                                                                   | [`Application`](ProjectStatus::Application)                         |
//! | Evaluation Start          | Issuer starts the evaluation round with the [`start_evaluation()`](Pallet::start_evaluation) extrinsic.                                                                                                                                                                                                                                                                                                     | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
//! | Evaluation Submissions    | Evaluators assess the project information, and if they think it is good enough to get funding, they bond Polimec's native token PLMC with [`bond_evaluation()`](Pallet::bond_evaluation)                                                                                                                                                                                                                    | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
//! | Evaluation End            | Evaluation round ends automatically after the [`Config::EvaluationDuration`] has passed. This is achieved by the [`on_initialize()`](Pallet::on_initialize) function.                                                                                                                                                                                                                                       | [`AuctionInitializePeriod`](ProjectStatus::AuctionInitializePeriod) |
//! | Auction Start             | Issuer starts the auction round within the [`Config::AuctionInitializePeriodDuration`], by calling the extrinsic [`start_auction()`](Pallet::start_auction)                                                                                                                                                                                                                                                 | [`AuctionRound(English)`](ProjectStatus::AuctionRound)              |
//! | Bid Submissions           | Institutional and Professional users can place bids with [`bid()`](Pallet::bid) by choosing their desired token price, amount, and multiplier (for vesting). Their bids are guaranteed to be considered                                                                                                                                                                                                     | [`AuctionRound(English)`](ProjectStatus::AuctionRound)              |                                                                                                                                                                                                                |                                                                     |
//! | Candle Auction Transition | After the [`Config::EnglishAuctionDuration`] has passed, the auction goes into candle mode thanks to [`on_initialize()`](Pallet::on_initialize)                                                                                                                                                                                                                                                             | [`AuctionRound(Candle)`](ProjectStatus::AuctionRound)               |
//! | Bid Submissions           | Institutional and Professional users can continue bidding, but this time their bids will only be considered, if they managed to fall before the random ending block calculated at the end of the auction.                                                                                                                                                                                                   | [`AuctionRound(Candle)`](ProjectStatus::AuctionRound)               |
//! | Community Funding Start   | After the [`Config::CandleAuctionDuration`] has passed, the auction automatically. A final token price for the next rounds is calculated based on the accepted bids.                                                                                                                                                                                                                                        | [`CommunityRound`](ProjectStatus::CommunityRound)                   |
//! | Funding Submissions       | Retail investors can call the [`contribute()`](Pallet::contribute) extrinsic to buy tokens at the set price.                                                                                                                                                                                                                                                                                                | [`CommunityRound`](ProjectStatus::CommunityRound)                   |
//! | Remainder Funding Start   | After the [`Config::CommunityFundingDuration`] has passed, the project is now open to token purchases from any user type                                                                                                                                                                                                                                                                                    | [`RemainderRound`](ProjectStatus::RemainderRound)                   |
//! | Funding End               | If all tokens were sold, or after the [`Config::RemainderFundingDuration`] has passed, the project automatically ends, and it is calculated if it reached its desired funding or not.                                                                                                                                                                                                                       | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
//! | Evaluator Rewards         | If the funding was successful, evaluators can claim their contribution token rewards with the [`TBD`]() extrinsic. If it failed, evaluators can either call the [`failed_evaluation_unbond_for()`](Pallet::failed_evaluation_unbond_for) extrinsic, or wait for the [`on_idle()`](Pallet::on_initialize) function, to return their funds                                                                    | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
//! | Bidder Rewards            | If the funding was successful, bidders will call [`vested_contribution_token_bid_mint_for()`](Pallet::vested_contribution_token_bid_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_bid_unbond_for()`](Pallet::vested_plmc_bid_unbond_for) to unbond their PLMC, based on their current vesting schedule.                                                                        | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
//! | Buyer Rewards             | If the funding was successful, users who bought tokens on the Community or Remainder round, can call [`vested_contribution_token_purchase_mint_for()`](Pallet::vested_contribution_token_purchase_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_purchase_unbond_for()`](Pallet::vested_plmc_purchase_unbond_for) to unbond their PLMC, based on their current vesting schedule | [`FundingEnded`](ProjectStatus::FundingEnded)                       |
//!
//! ## Interface
//! All users who wish to participate need to have a valid credential, given to them on the KILT parachain, by a KYC/AML provider.
//! ### Extrinsics
//! * [`create`](Pallet::create) : Creates a new project.
//! * [`edit_metadata`](Pallet::edit_metadata) : Submit a new Hash of the project metadata.
//! * [`start_evaluation`](Pallet::start_evaluation) : Start the Evaluation round of a project.
//! * [`start_auction`](Pallet::start_auction) : Start the English Auction round of a project.
//! * [`bond_evaluation`](Pallet::bond_evaluation) : Bond PLMC on a project in the evaluation stage. A sort of "bet" that you think the project will be funded
//! * [`failed_evaluation_unbond_for`](Pallet::failed_evaluation_unbond_for) : Unbond the PLMC bonded on a project's evaluation round for any user, if the project failed the evaluation.
//! * [`bid`](Pallet::bid) : Perform a bid during the English or Candle Auction Round.
//! * [`contribute`](Pallet::contribute) : Buy contribution tokens if a project during the Community or Remainder round
//! * [`vested_plmc_bid_unbond_for`](Pallet::vested_plmc_bid_unbond_for) : Unbond the PLMC bonded on a project's English or Candle Auction Round for any user, based on their vesting schedule.
//! * [`vested_plmc_purchase_unbond_for`](Pallet::vested_plmc_purchase_unbond_for) : Unbond the PLMC bonded on a project's Community or Remainder Round for any user, based on their vesting schedule.
//! * [`vested_contribution_token_bid_mint_for`](Pallet::vested_contribution_token_bid_mint_for) : Mint the contribution tokens for a user who participated in the English or Candle Auction Round, based on their vesting schedule.
//! * [`vested_contribution_token_purchase_mint_for`](Pallet::vested_contribution_token_purchase_mint_for) : Mint the contribution tokens for a user who participated in the Community or Remainder Round, based on their vesting schedule.
//!
//! ### Storage Items
//! * [`NextProjectId`] : Increasing counter to get the next id to assign to a project.
//! * [`NextBidId`]: Increasing counter to get the next id to assign to a bid.
//! * [`Nonce`]: Increasing counter to be used in random number generation.
//! * [`Images`]: Map of the hash of some metadata to the user who owns it. Avoids storing the same image twice, and keeps track of ownership for a future project data access due to regulatory compliance.
//! * [`Projects`]: Map of the assigned id, to the main information of a project.
//! * [`ProjectsIssuers`]: Map of a project id, to its issuer account.
//! * [`ProjectsInfo`]: Map of a project id, to some additional information required for ensuring correctness of the protocol.
//! * [`ProjectsToUpdate`]: Map of a block number, to a vector of project ids. Used to keep track of projects that need to be updated in on_initialize.
//! * [`AuctionsInfo`]: Double map linking a project-user to the bids they made.
//! * [`EvaluationBonds`]: Double map linking a project-user to the PLMC they bonded in the evaluation round.
//! * [`BiddingBonds`]: Double map linking a project-user to the PLMC they bonded in the English or Candle Auction round.
//! * [`ContributingBonds`]: Double map linking a project-user to the PLMC they bonded in the Community or Remainder round.
//! * [`Contributions`]: Double map linking a project-user to the contribution tokens they bought in the Community or Remainder round.
//!
//! ## Usage
//! You can circumvent the extrinsics by calling the do_* functions that they call directly.
//! This is useful if you need to make use of this pallet's functionalities in a pallet of your own, and you don't want to pay the transaction fees twice.
//! ### Example: A retail user buying tokens for a project in the community round
//! ```
//! pub use pallet::*;
//!
//! #[frame_support::pallet(dev_mode)]
//! pub mod pallet {
//! 	use super::*;
//! 	use frame_support::pallet_prelude::*;
//! 	use frame_system::pallet_prelude::*;
//!
//! 	#[pallet::pallet]
//! 	pub struct Pallet<T>(_);
//!
//! 	#[pallet::config]
//! 	pub trait Config: frame_system::Config + pallet_funding::Config {}
//!
//! 	#[pallet::call]
//! 	impl<T: Config> Pallet<T> {
//! 		/// Buy tokens for a project in the community round if it achieved at least 500k USDT funding
//! 		#[pallet::weight(0)]
//! 		pub fn buy_if_popular(
//! 			origin: OriginFor<T>,
//! 			project_id: <T as pallet_funding::Config>::ProjectIdParameter,
//! 			amount: <T as pallet_funding::Config>::CurrencyBalance
//! 		) -> DispatchResult {
//! 			let retail_user = ensure_signed(origin)?;
//! 			let project_id: <T as pallet_funding::Config>::ProjectIdentifier = project_id.into();
//! 			// Check project is in the community round
//! 			let project_info = pallet_funding::Pallet::<T>::project_info(project_id).ok_or(Error::<T>::ProjectNotFound)?;
//! 			ensure!(project_info.project_status == pallet_funding::ProjectStatus::CommunityRound, "Project is not in the community round");
//!
//! 			// Calculate how much funding was done already
//! 			let project_contributions: <T as pallet_funding::Config>::CurrencyBalance = pallet_funding::Contributions::<T>::iter_prefix_values(project_id)
//! 				.flatten()
//! 				.fold(
//! 					0u64.into(),
//! 					|total_tokens_bought, contribution| {
//! 						total_tokens_bought + contribution.contribution_amount
//! 					}
//! 				);
//!
//! 			ensure!(project_contributions >= 500_000_0_000_000_000u64.into(), "Project did not achieve at least 500k USDT funding");
//!
//! 			// Buy tokens with the default multiplier
//! 			<pallet_funding::Pallet<T>>::do_contribute(retail_user, project_id, amount, None)?;
//!
//! 			Ok(())
//! 		}
//! 	}
//!
//! 	#[pallet::error]
//! 	pub enum Error<T> {
//! 		ProjectNotFound,
//! 	}
//! }
//! ```
//!
//! ## Credentials
//! The pallet will only allow users with certain credential types, to execute certain extrinsics.:
//!
//!
//! | Extrinsic                                     | Issuer | Retail Investor | Professional Investor | Institutional Investor |
//! |-----------------------------------------------|--------|-----------------|-----------------------|------------------------|
//! | `create`                                      | X      |                 |                       |                        |
//! | `edit_metadata`                               | X      |                 |                       |                        |
//! | `start_evaluation`                            | X      |                 |                       |                        |
//! | `start_auction`                               | X      |                 |                       |                        |
//! | `bond_evaluation`                             |        | X               | X                     | X                      |
//! | `failed_evaluation_unbond_for`                |        | X               | X                     | X                      |
//! | `bid`                                         |        |                 | X                     | X                      |
//! | `contribute`                                  |        | X               | X*                    | X*                     |
//! | `vested_plmc_bid_unbond_for`                  |        |                 | X                     | X                      |
//! | `vested_plmc_purchase_unbond_for`             |        | X               | X                     | X                      |
//! | `vested_contribution_token_bid_mint_for`      |        |                 | X                     | X                      |
//! | `vested_contribution_token_purchase_mint_for` |        | X               | X                     | X                      |
//!
//! _* They can call contribute only if the project is on the remainder round._
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
pub mod tests;

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
		Currency as CurrencyT, Get, NamedReservableCurrency, Randomness, ReservableCurrency,
	},
	BoundedVec, PalletId, Parameter,
};

use sp_arithmetic::traits::{One, Saturating};

use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv},
	FixedPointNumber, FixedPointOperand, FixedU128,
};
use sp_std::prelude::*;

type BalanceOf<T> = <T as Config>::CurrencyBalance;

type ProjectOf<T> = Project<
	BoundedVec<u8, <T as Config>::StringLimit>,
	BalanceOf<T>,
	<T as frame_system::Config>::Hash,
>;

type ProjectInfoOf<T> = ProjectInfo<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>;

type BidInfoOf<T> = BidInfo<
	<T as Config>::BidId,
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
	pub trait Config: frame_system::Config {
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
		type BiddingCurrency: ReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;

		/// Unique identifier for any bid in the system.
		type BidId: Parameter + Copy + Saturating + One + Default;

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

		/// The time window (expressed in number of blocks) that an issuer has to start the auction round.
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

		/// The length (expressed in number of blocks) of the Remainder Round.
		#[pallet::constant]
		type RemainderFundingDuration: Get<Self::BlockNumber>;

		/// `PalletId` for the funding pallet. An appropriate value could be
		/// `PalletId(*b"py/cfund")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How many projects should we update in on_initialize each block
		#[pallet::constant]
		type MaxProjectsToUpdatePerBlock: Get<u32>;

		/// The maximum number of bids per user per project
		#[pallet::constant]
		type MaximumBidsPerUser: Get<u32>;

		/// The maximum number of bids per user per project
		#[pallet::constant]
		type MaxContributionsPerUser: Get<u32>;

		/// The maximum number of bids per user
		#[pallet::constant]
		type ContributionVesting: Get<u32>;

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
	#[pallet::getter(fn next_bid_id)]
	/// A global counter for indexing the bids
	/// OnEmpty in this case is GetDefault, so 0.
	pub type NextBidId<T: Config> = StorageValue<_, T::BidId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// A global counter used in the randomness generation
	// TODO: PLMC-155. Remove it after using the Randomness from BABE's VRF: https://github.com/PureStake/moonbeam/issues/1391
	// 	Or use the randomness from Moonbeam.
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn images)]
	/// A StorageMap containing all the hashes of the project metadata uploaded by the users.
	/// TODO: PLMC-156. The metadata should be stored on IPFS/offchain database, and the hash of the metadata should be stored here.
	pub type Images<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn projects)]
	/// A StorageMap containing the primary project information of projects
	pub type Projects<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn project_issuer)]
	/// StorageMap to get the issuer of a project
	pub type ProjectsIssuers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn project_info)]
	/// StorageMap containing additional information for the projects, relevant for correctness of the protocol
	pub type ProjectsInfo<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectInfoOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn projects_to_update)]
	/// A map to know in which block to update which active projects using on_initialize.
	pub type ProjectsToUpdate<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<T::ProjectIdentifier, T::MaxProjectsToUpdatePerBlock>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn auctions_info)]
	/// StorageMap containing the bids for each project and user
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
	/// Keep track of the PLMC bonds made to each project by each evaluator
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
	/// Keep track of the PLMC bonds made to each project by each bidder
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
	/// Keep track of the PLMC bonds made to each project by each contributor
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
	/// Contributions made during the Community and Remainder round. i.e token buys
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
		/// A project was created.
		Created { project_id: T::ProjectIdentifier },
		/// The metadata of a project was modified.
		MetadataEdited { project_id: T::ProjectIdentifier },
		/// The evaluation phase of a project started.
		EvaluationStarted { project_id: T::ProjectIdentifier },
		/// The evaluation phase of a project ended without reaching the minimum threshold of evaluation bonds.
		EvaluationFailed { project_id: T::ProjectIdentifier },
		/// The period an issuer has to start the auction phase of the project.
		AuctionInitializePeriod {
			project_id: T::ProjectIdentifier,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
		},
		/// The auction round of a project started.
		EnglishAuctionStarted { project_id: T::ProjectIdentifier, when: T::BlockNumber },
		/// The candle auction part of the auction started for a project
		CandleAuctionStarted { project_id: T::ProjectIdentifier, when: T::BlockNumber },
		/// The auction round of a project ended.
		AuctionEnded { project_id: T::ProjectIdentifier },
		/// A `bonder` bonded an `amount` of PLMC for `project_id`.
		FundsBonded { project_id: T::ProjectIdentifier, amount: BalanceOf<T>, bonder: T::AccountId },
		/// Someone paid for the release of a user's PLMC bond for a project.
		BondReleased {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			bonder: T::AccountId,
			releaser: T::AccountId,
		},
		/// A bid was made for a project
		Bid {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			price: BalanceOf<T>,
			multiplier: BalanceOf<T>,
		},
		/// A contribution was made for a project. i.e token purchase
		Contribution {
			project_id: T::ProjectIdentifier,
			contributor: T::AccountId,
			amount: BalanceOf<T>,
			multiplier: BalanceOf<T>,
		},
		/// A project is now in its community funding round
		CommunityFundingStarted { project_id: T::ProjectIdentifier },
		/// A project is now in the remainder funding round
		RemainderFundingStarted { project_id: T::ProjectIdentifier },
		/// A project has now finished funding
		FundingEnded { project_id: T::ProjectIdentifier },
		/// Something was not properly initialized. Most likely due to dev error manually calling do_* functions or updating storage
		TransitionError { project_id: T::ProjectIdentifier, error: DispatchError },
		/// Something terribly wrong happened where the bond could not be unbonded. Most likely a programming error
		FailedEvaluationUnbondFailed { error: DispatchError },
		/// Contribution tokens were minted to a user
		ContributionTokenMinted {
			caller: T::AccountId,
			project_id: T::ProjectIdentifier,
			contributor: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Something in storage has a state which should never be possible at this point. Programming error
		ImpossibleState,
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
		/// The Evaluation Round of the project has ended without reaching the minimum threshold
		EvaluationFailed,
		/// The issuer cannot contribute to their own project during the Funding Round
		ContributionToThemselves,
		/// Only the issuer can start the Evaluation Round
		NotAllowed,
		/// The Metadata Hash of the project was not found
		MetadataNotProvided,
		/// The Auction Round of the project has not started yet
		AuctionNotStarted,
		/// You cannot edit the metadata of a project that already passed the Evaluation Round
		Frozen,
		/// The bid is too low
		BidTooLow,
		/// The user has not enough balance to perform the action
		InsufficientBalance,
		// TODO: PLMC-133 Check after the introduction of the cross-chain identity pallet by KILT
		NotAuthorized,
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
		/// Checks for other projects not copying metadata of others
		MetadataAlreadyExists,
		/// The specified issuer does not exist
		ProjectIssuerNotFound,
		/// The specified project info does not exist
		ProjectInfoNotFound,
		/// Tried to finish an evaluation before its target end block
		EvaluationPeriodNotEnded,
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
		/// Creates a project and assigns it to the `issuer` account.
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

		/// Change the metadata hash of a project
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

		/// Starts the evaluation round of a project. It needs to be called by the project issuer.
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

		/// Starts the auction round for a project. From the next block forward, any professional or
		/// institutional user can set bids for a token_amount/token_price pair.
		/// Any bids from this point until the candle_auction starts, will be considered as valid.
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

		/// Bond PLMC for a project in the evaluation stage
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

		/// Bid for a project in the Auction round
		#[pallet::weight(T::WeightInfo::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			#[pallet::compact] amount: BalanceOf<T>,
			#[pallet::compact] price: BalanceOf<T>,
			multiplier: Option<BalanceOf<T>>,
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

		/// Buy tokens in the Community or Remainder round at the price set in the Auction Round
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

		/// Unbond some plmc from a contribution, after a step in the vesting period has passed.
		pub fn vested_plmc_bid_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			bidder: T::AccountId,
		) -> DispatchResult {
			// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_plmc_bid_unbond_for(claimer, project_id, bidder)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		/// Mint contribution tokens after a step in the vesting period for a successful bid.
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
		/// Unbond some plmc from a contribution, after a step in the vesting period has passed.
		pub fn vested_plmc_purchase_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdParameter,
			purchaser: T::AccountId,
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			let project_id = project_id.into();

			Self::do_vested_plmc_purchase_unbond_for(claimer, project_id, purchaser)
		}

		// TODO: PLMC-157. Manage the fact that the CTs may not be claimed by those entitled
		/// Mint contribution tokens after a step in the vesting period for a contribution.
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
				let maybe_project_info = ProjectsInfo::<T>::get(project_id);
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
					_ => {},
				}
			}
			// TODO: PLMC-127. Set a proper weight
			Weight::from_ref_time(0)
		}

		fn on_idle(_now: T::BlockNumber, max_weight: Weight) -> Weight {
			let pallet_account: T::AccountId =
				<T as Config>::PalletId::get().into_account_truncating();

			let mut remaining_weight = max_weight;

			// Unbond the plmc from failed evaluation projects
			let unbond_results = ProjectsInfo::<T>::iter()
				// Retrieve failed evaluation projects
				.filter_map(|(project_id, info)| {
					if let ProjectStatus::EvaluationFailed = info.project_status {
						Some(project_id)
					} else {
						None
					}
				})
				// Get a flat list of bonds
				.flat_map(|project_id| {
					// get all the bonds for projects with a failed evaluation phase
					EvaluationBonds::<T>::iter_prefix(project_id)
						.map(|(_bonder, bond)| bond)
						.collect::<Vec<_>>()
				})
				// Retrieve as many as possible for the given weight
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
				// Unbond the plmc
				.map(|bond| Self::do_failed_evaluation_unbond_for(bond, pallet_account.clone()))
				.collect::<Vec<_>>();

			// Make sure no unbonding failed
			for result in unbond_results {
				if let Err(e) = result {
					Self::deposit_event(Event::<T>::FailedEvaluationUnbondFailed { error: e });
				}
			}

			// // TODO: PLMC-127. Set a proper weight
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
				metadata: Some(metadata_hash),
				..Default::default()
			};
			project
		}
	}
}

pub mod local_macros {
	/// used to unwrap storage values that can be None in places where an error cannot be returned,
	/// but an event should be emitted, and skip to the next iteration of a loop
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

	/// used to unwrap storage values that can be Err in places where an error cannot be returned,
	/// but an event should be emitted, and skip to the next iteration of a loop
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
