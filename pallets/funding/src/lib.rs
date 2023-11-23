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
//! | Funding End               | If all tokens were sold, or after the [`Config::RemainderFundingDuration`] has passed, the project automatically ends, and it is calculated if it reached its desired funding or not.                                                                                                                                                                                                                       | [`FundingEnded`](ProjectStatus::FundingSuccessful)                       |
//! | Evaluator Rewards         | If the funding was successful, evaluators can claim their contribution token rewards with the [`TBD`]() extrinsic. If it failed, evaluators can either call the [`failed_evaluation_unbond_for()`](Pallet::failed_evaluation_unbond_for) extrinsic, or wait for the [`on_idle()`](Pallet::on_initialize) function, to return their funds                                                                    | [`FundingEnded`](ProjectStatus::FundingSuccessful)                       |
//! | Bidder Rewards            | If the funding was successful, bidders will call [`vested_contribution_token_bid_mint_for()`](Pallet::vested_contribution_token_bid_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_bid_unbond_for()`](Pallet::vested_plmc_bid_unbond_for) to unbond their PLMC, based on their current vesting schedule.                                                                        | [`FundingEnded`](ProjectStatus::FundingSuccessful)                       |
//! | Buyer Rewards             | If the funding was successful, users who bought tokens on the Community or Remainder round, can call [`vested_contribution_token_purchase_mint_for()`](Pallet::vested_contribution_token_purchase_mint_for) to mint the contribution tokens they are owed, and [`vested_plmc_purchase_unbond_for()`](Pallet::vested_plmc_purchase_unbond_for) to unbond their PLMC, based on their current vesting schedule | [`FundingEnded`](ProjectStatus::FundingSuccessful)                       |
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
//! * [`ProjectsMetadata`]: Map of the assigned id, to the main information of a project.
//! * [`ProjectsIssuers`]: Map of a project id, to its issuer account.
//! * [`ProjectsDetails`]: Map of a project id, to some additional information required for ensuring correctness of the protocol.
//! * [`ProjectsToUpdate`]: Map of a block number, to a vector of project ids. Used to keep track of projects that need to be updated in on_initialize.
//! * [`Bids`]: Double map linking a project-user to the bids they made.
//! * [`Evaluations`]: Double map linking a project-user to the PLMC they bonded in the evaluation round.
//! * [`Contributions`]: Double map linking a project-user to the contribution tokens they bought in the Community or Remainder round.
//!
//! ## Usage
//! You can circumvent the extrinsics by calling the do_* functions that they call directly.
//! This is useful if you need to make use of this pallet's functionalities in a pallet of your own, and you don't want to pay the transaction fees twice.
//! ### Example: A retail user buying tokens for a project in the community round
//! ```
//! #[frame_support::pallet(dev_mode)]
//! pub mod pallet {
//!     use super::*;
//!     use frame_support::pallet_prelude::*;
//!     use frame_system::pallet_prelude::*;
//!    	use pallet_funding::{AcceptedFundingAsset, MultiplierOf};
//!
//!     #[pallet::pallet]
//!     pub struct Pallet<T>(_);
//!
//!     #[pallet::config]
//!     pub trait Config: frame_system::Config + pallet_funding::Config {}
//!
//!     #[pallet::call]
//!     impl<T: Config> Pallet<T> {
//! 		/// Buy tokens for a project in the community round if it achieved at least 500k USDT funding
//! 		#[pallet::weight(0)]
//! 		pub fn buy_if_popular(
//! 			origin: OriginFor<T>,
//! 			project_id: <T as pallet_funding::Config>::ProjectIdentifier,
//! 			amount: <T as pallet_funding::Config>::Balance
//! 		) -> DispatchResultWithPostInfo {
//! 			let retail_user = ensure_signed(origin)?;
//! 			let project_id: <T as pallet_funding::Config>::ProjectIdentifier = project_id.into();
//! 			// Check project is in the community round
//! 			let project_details = pallet_funding::Pallet::<T>::project_details(project_id).ok_or(Error::<T>::ProjectNotFound)?;
//! 			ensure!(project_details.status == pallet_funding::ProjectStatus::CommunityRound, "Project is not in the community round");
//!
//! 			// Calculate how much funding was done already
//! 			let project_contributions: <T as pallet_funding::Config>::Balance = pallet_funding::Contributions::<T>::iter_prefix_values((project_id,))
//! 				.fold(
//! 					0u64.into(),
//! 					|total_tokens_bought, contribution| {
//! 						total_tokens_bought + contribution.usd_contribution_amount
//! 					}
//! 				);
//!
//! 			ensure!(project_contributions >= 500_000_0_000_000_000u64.into(), "Project did not achieve at least 500k USDT funding");
//!    			let multiplier: MultiplierOf<T> = 1u8.try_into().map_err(|_| Error::<T>::ProjectNotFound)?;
//!    			// Buy tokens with the default multiplier
//!    			pallet_funding::Pallet::<T>::do_contribute(&retail_user, project_id, amount, multiplier, AcceptedFundingAsset::USDT)
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

use frame_support::{
	traits::{
		tokens::{fungible, fungibles, Balance},
		Randomness,
	},
	BoundedVec, PalletId,
};
pub use pallet::*;
use polkadot_parachain::primitives::Id as ParaId;
use sp_arithmetic::traits::{One, Saturating};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, FixedPointOperand, FixedU128};
use sp_std::{marker::PhantomData, prelude::*};
pub use types::*;
use xcm::v3::{opaque::Instruction, prelude::*, SendXcm};

pub use crate::weights::WeightInfo;

pub mod functions;
pub mod types;
pub mod weights;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod impls;
#[cfg(any(feature = "runtime-benchmarks", test, all(feature = "testing-node", feature = "std")))]
pub mod instantiator;
pub mod traits;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
pub type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
pub type MultiplierOf<T> = <T as Config>::Multiplier;

pub type BalanceOf<T> = <T as Config>::Balance;
pub type PriceOf<T> = <T as Config>::Price;
pub type StringLimitOf<T> = <T as Config>::StringLimit;
pub type HashOf<T> = <T as frame_system::Config>::Hash;
pub type AssetIdOf<T> =
	<<T as Config>::FundingCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
pub type RewardInfoOf<T> = RewardInfo<BalanceOf<T>>;
pub type EvaluatorsOutcomeOf<T> = EvaluatorsOutcome<BalanceOf<T>>;

pub type ProjectMetadataOf<T> =
	ProjectMetadata<BoundedVec<u8, StringLimitOf<T>>, BalanceOf<T>, PriceOf<T>, AccountIdOf<T>, HashOf<T>>;
pub type ProjectDetailsOf<T> =
	ProjectDetails<AccountIdOf<T>, BlockNumberOf<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;
pub type EvaluationRoundInfoOf<T> = EvaluationRoundInfo<BalanceOf<T>>;
pub type VestingInfoOf<T> = VestingInfo<BlockNumberOf<T>, BalanceOf<T>>;
pub type EvaluationInfoOf<T> = EvaluationInfo<u32, ProjectIdOf<T>, AccountIdOf<T>, BalanceOf<T>, BlockNumberOf<T>>;
pub type BidInfoOf<T> = BidInfo<
	u32,
	ProjectIdOf<T>,
	BalanceOf<T>,
	PriceOf<T>,
	AccountIdOf<T>,
	BlockNumberOf<T>,
	MultiplierOf<T>,
	VestingInfoOf<T>,
>;
pub type ContributionInfoOf<T> =
	ContributionInfo<u32, ProjectIdOf<T>, AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, VestingInfoOf<T>>;

pub type MigrationOriginOf<T> = MigrationOrigin<AccountIdOf<T>, u32>;
pub type ProjectMigrationOriginsOf<T> =
	ProjectMigrationOrigins<ProjectIdOf<T>, BoundedVec<MigrationOriginOf<T>, MaxMigrationsPerXcm<T>>>;

pub type BucketOf<T> = Bucket<BalanceOf<T>, PriceOf<T>>;
pub type BondTypeOf<T> = LockType<ProjectIdOf<T>>;
pub type WeightInfoOf<T> = <T as Config>::WeightInfo;

pub const PLMC_STATEMINT_ID: u32 = 2069;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::Percent;
	use sp_runtime::traits::Convert;

	use local_macros::*;

	use crate::traits::{BondingRequirementCalculation, ProvideStatemintPrice, VestingDurationCalculation};

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_balances::Config<Balance = BalanceOf<Self>> + pallet_xcm::Config
	{
		type AllPalletsWithoutSystem: frame_support::traits::OnFinalize<BlockNumberOf<Self>>
			+ frame_support::traits::OnIdle<BlockNumberOf<Self>>
			+ frame_support::traits::OnInitialize<BlockNumberOf<Self>>;

		type RuntimeEvent: From<Event<Self>>
			+ TryInto<Event<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>
			+ Parameter
			+ Member;

		// TODO: our local BlockNumber should be removed once we move onto using Moment for time tracking
		type BlockNumber: IsType<<Self as frame_system::Config>::BlockNumber> + Into<u64>;

		type AccountId32Conversion: Convert<Self::AccountId, [u8; 32]>;

		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		type RuntimeCall: Parameter + IsType<<Self as frame_system::Config>::RuntimeCall> + From<Call<Self>>;

		/// Global identifier for the projects.
		type ProjectIdentifier: Parameter + Copy + Default + One + Saturating + From<u32> + Ord + MaxEncodedLen;
		// TODO: PLMC-153 + MaybeSerializeDeserialize: Maybe needed for JSON serialization @ Genesis: https://github.com/paritytech/substrate/issues/12738#issuecomment-1320921201

		/// Multiplier that decides how much PLMC needs to be bonded for a token buy/bid
		type Multiplier: Parameter
			+ BondingRequirementCalculation
			+ VestingDurationCalculation
			+ Default
			+ Copy
			+ TryFrom<u8>
			+ MaxEncodedLen
			+ MaybeSerializeDeserialize;

		/// The inner balance type we will use for all of our outer currency types. (e.g native, funding, CTs)
		type Balance: Balance + From<u64> + FixedPointOperand + MaybeSerializeDeserialize + Into<u128>;

		/// Represents the value of something in USD
		type Price: FixedPointNumber + Parameter + Copy + MaxEncodedLen + MaybeSerializeDeserialize;

		/// The chains native currency
		type NativeCurrency: fungible::InspectHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::MutateHold<AccountIdOf<Self>, Balance = BalanceOf<Self>, Reason = BondTypeOf<Self>>
			+ fungible::BalancedHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// The currency used for funding projects in bids and contributions
		// type FundingCurrency: ReservableCurrency<AccountIdOf<Self, Balance = BalanceOf<Self>>;
		type FundingCurrency: fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = BalanceOf<Self>, AssetId = u32>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// The currency used for minting contribution tokens as fungible assets (i.e pallet-assets)
		type ContributionTokenCurrency: fungibles::Create<AccountIdOf<Self>, AssetId = Self::ProjectIdentifier, Balance = BalanceOf<Self>>
			+ fungibles::Destroy<AccountIdOf<Self>, AssetId = Self::ProjectIdentifier, Balance = BalanceOf<Self>>
			+ fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungibles::roles::Inspect<AccountIdOf<Self>>;

		type PriceProvider: ProvideStatemintPrice<AssetId = u32, Price = Self::Price>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, <Self as frame_system::Config>::BlockNumber>;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// The maximum size of a preimage allowed, expressed in bytes.
		#[pallet::constant]
		type PreImageLimit: Get<u32>;

		/// The length (expressed in number of blocks) of the evaluation period.
		#[pallet::constant]
		type EvaluationDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// The time window (expressed in number of blocks) that an issuer has to start the auction round.
		#[pallet::constant]
		type AuctionInitializePeriodDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// The length (expressed in number of blocks) of the Auction Round, English period.
		#[pallet::constant]
		type EnglishAuctionDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// The length (expressed in number of blocks) of the Auction Round, Candle period.
		#[pallet::constant]
		type CandleAuctionDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// The length (expressed in number of blocks) of the Community Round.
		#[pallet::constant]
		type CommunityFundingDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// The length (expressed in number of blocks) of the Remainder Round.
		#[pallet::constant]
		type RemainderFundingDuration: Get<<Self as frame_system::Config>::BlockNumber>;

		/// `PalletId` for the funding pallet. An appropriate value could be
		/// `PalletId(*b"py/cfund")`
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// How many projects should we update in on_initialize each block
		#[pallet::constant]
		type MaxProjectsToUpdatePerBlock: Get<u32>;

		/// How many distinct evaluations per user per project
		type MaxEvaluationsPerUser: Get<u32>;

		/// The maximum number of bids per user per project
		#[pallet::constant]
		type MaxBidsPerUser: Get<u32>;

		/// The maximum number of bids per user per project
		#[pallet::constant]
		type MaxContributionsPerUser: Get<u32>;

		/// The maximum number of bids per user
		#[pallet::constant]
		type ContributionVesting: Get<u32>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: weights::WeightInfo;

		type FeeBrackets: Get<Vec<(Percent, <Self as Config>::Balance)>>;

		type EvaluationSuccessThreshold: Get<Percent>;

		type Vesting: polimec_traits::ReleaseSchedule<
			AccountIdOf<Self>,
			BondTypeOf<Self>,
			Currency = Self::NativeCurrency,
			Moment = BlockNumberOf<Self>,
		>;
		/// For now we expect 3 days until the project is automatically accepted. Timeline decided by MiCA regulations.
		type ManualAcceptanceDuration: Get<<Self as frame_system::Config>::BlockNumber>;
		/// For now we expect 4 days from acceptance to settlement due to MiCA regulations.
		type SuccessToSettlementTime: Get<<Self as frame_system::Config>::BlockNumber>;

		type EvaluatorSlash: Get<Percent>;

		type TreasuryAccount: Get<AccountIdOf<Self>>;

		/// Convert 24 hours as FixedU128, to the corresponding amount of blocks in the same type as frame_system
		type DaysToBlocks: Convert<FixedU128, BlockNumberOf<Self>>;

		type BlockNumberToBalance: Convert<BlockNumberOf<Self>, BalanceOf<Self>>;

		type PolimecReceiverInfo: Get<PalletInfo>;

		/// Range of max_message_size values for the hrmp config where we accept the incoming channel request
		type MaxMessageSizeThresholds: Get<(u32, u32)>;
		/// Range of max_capacity_thresholds values for the hrmp config where we accept the incoming channel request
		type MaxCapacityThresholds: Get<(u32, u32)>;
		/// max_capacity config required for the channel from polimec to the project
		type RequiredMaxCapacity: Get<u32>;
		/// max_message_size config required for the channel from polimec to the project
		type RequiredMaxMessageSize: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn next_project_id)]
	/// A global counter for indexing the projects
	/// OnEmpty in this case is GetDefault, so 0.
	pub type NextProjectId<T: Config> = StorageValue<_, T::ProjectIdentifier, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_evaluation_id)]
	pub type NextEvaluationId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_bid_id)]
	pub type NextBidId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_contribution_id)]
	pub type NextContributionId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// A global counter used in the randomness generation
	// TODO: PLMC-155. Remove it after using the Randomness from BABE's VRF: https://github.com/PureStake/moonbeam/issues/1391
	// 	Or use the randomness from Moonbeam.
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn images)]
	/// A StorageMap containing all the hashes of the project metadata uploaded by the users.
	pub type Images<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, AccountIdOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn projects_metadata)]
	/// A StorageMap containing the primary project information of projects
	pub type ProjectsMetadata<T: Config> = StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, ProjectMetadataOf<T>>;

	#[pallet::storage]
	/// A StorageMap containing the primary project information of projects
	pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, T::ProjectIdentifier, BucketOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn project_details)]
	/// StorageMap containing additional information for the projects, relevant for correctness of the protocol
	pub type ProjectsDetails<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::ProjectIdentifier,
		ProjectDetails<AccountIdOf<T>, BlockNumberOf<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn projects_to_update)]
	/// A map to know in which block to update which active projects using on_initialize.
	pub type ProjectsToUpdate<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		<T as frame_system::Config>::BlockNumber,
		BoundedVec<(T::ProjectIdentifier, UpdateType), T::MaxProjectsToUpdatePerBlock>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn evaluations)]
	/// Keep track of the PLMC bonds made to each project by each evaluator
	pub type Evaluations<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::ProjectIdentifier>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		EvaluationInfoOf<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn bids)]
	/// StorageMap containing the bids for each project and user
	pub type Bids<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::ProjectIdentifier>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		BidInfoOf<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn contributions)]
	/// Contributions made during the Community and Remainder round. i.e token buys
	pub type Contributions<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, T::ProjectIdentifier>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		ContributionInfoOf<T>,
	>;

	#[pallet::storage]
	/// Migrations sent and awaiting for confirmation
	pub type UnconfirmedMigrations<T: Config> = StorageMap<_, Blake2_128Concat, QueryId, ProjectMigrationOriginsOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A project was created.
		ProjectCreated {
			project_id: T::ProjectIdentifier,
			issuer: T::AccountId,
		},
		/// The metadata of a project was modified.
		MetadataEdited {
			project_id: T::ProjectIdentifier,
		},
		/// The evaluation phase of a project started.
		EvaluationStarted {
			project_id: T::ProjectIdentifier,
		},
		/// The evaluation phase of a project ended without reaching the minimum threshold of evaluation bonds.
		EvaluationFailed {
			project_id: T::ProjectIdentifier,
		},
		/// The period an issuer has to start the auction phase of the project.
		AuctionInitializePeriod {
			project_id: T::ProjectIdentifier,
			start_block: <T as frame_system::Config>::BlockNumber,
			end_block: <T as frame_system::Config>::BlockNumber,
		},
		/// The auction round of a project started.
		EnglishAuctionStarted {
			project_id: T::ProjectIdentifier,
			when: <T as frame_system::Config>::BlockNumber,
		},
		/// The candle auction part of the auction started for a project
		CandleAuctionStarted {
			project_id: T::ProjectIdentifier,
			when: <T as frame_system::Config>::BlockNumber,
		},
		/// The auction round of a project ended.
		AuctionFailed {
			project_id: T::ProjectIdentifier,
		},
		/// A `bonder` bonded an `amount` of PLMC for `project_id`.
		FundsBonded {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			bonder: AccountIdOf<T>,
		},
		/// Someone paid for the release of a user's PLMC bond for a project.
		BondReleased {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			bonder: AccountIdOf<T>,
			releaser: AccountIdOf<T>,
		},
		/// A bid was made for a project
		Bid {
			project_id: T::ProjectIdentifier,
			amount: BalanceOf<T>,
			price: T::Price,
			multiplier: MultiplierOf<T>,
		},
		/// A contribution was made for a project. i.e token purchase
		Contribution {
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
		},
		/// A project is now in its community funding round
		CommunityFundingStarted {
			project_id: T::ProjectIdentifier,
		},
		/// A project is now in the remainder funding round
		RemainderFundingStarted {
			project_id: T::ProjectIdentifier,
		},
		/// A project has now finished funding
		FundingEnded {
			project_id: T::ProjectIdentifier,
			outcome: FundingOutcome,
		},
		/// Something was not properly initialized. Most likely due to dev error manually calling do_* functions or updating storage
		TransitionError {
			project_id: T::ProjectIdentifier,
			error: DispatchError,
		},
		/// Something terribly wrong happened where the bond could not be unbonded. Most likely a programming error
		EvaluationUnbondFailed {
			project_id: ProjectIdOf<T>,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		/// Contribution tokens were minted to a user
		ContributionTokenMinted {
			releaser: AccountIdOf<T>,
			project_id: T::ProjectIdentifier,
			claimer: AccountIdOf<T>,
			amount: BalanceOf<T>,
		},
		/// A transfer of tokens failed, but because it was done inside on_initialize it cannot be solved.
		TransferError {
			error: DispatchError,
		},
		EvaluationRewardFailed {
			project_id: ProjectIdOf<T>,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		EvaluationSlashFailed {
			project_id: ProjectIdOf<T>,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ReleaseBidFundsFailed {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		BidUnbondFailed {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ReleaseContributionFundsFailed {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ContributionUnbondFailed {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		PayoutContributionFundsFailed {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		PayoutBidFundsFailed {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		EvaluationRewarded {
			project_id: ProjectIdOf<T>,
			evaluator: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		EvaluationSlashed {
			project_id: ProjectIdOf<T>,
			evaluator: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		CTMintFailed {
			project_id: ProjectIdOf<T>,
			claimer: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		StartBidderVestingScheduleFailed {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		StartContributionVestingScheduleFailed {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		BidPlmcVestingScheduled {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionPlmcVestingScheduled {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ParticipantPlmcVested {
			project_id: ProjectIdOf<T>,
			participant: AccountIdOf<T>,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		BidFundingPaidOut {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionFundingPaidOut {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		BidFundingReleased {
			project_id: ProjectIdOf<T>,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionFundingReleased {
			project_id: ProjectIdOf<T>,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ProjectOutcomeDecided {
			project_id: ProjectIdOf<T>,
			decision: FundingOutcomeDecision,
		},
		ProjectParaIdSet {
			project_id: T::ProjectIdentifier,
			para_id: ParaId,
			caller: T::AccountId,
		},
		/// A channel was accepted from a parachain to Polimec belonging to a project. A request has been sent to the relay for a Polimec->project channel
		HrmpChannelAccepted {
			project_id: T::ProjectIdentifier,
			para_id: ParaId,
		},
		/// A channel was established from Polimec to a project. The relay has notified us of their acceptance of our request
		HrmpChannelEstablished {
			project_id: T::ProjectIdentifier,
			para_id: ParaId,
		},
		/// Started a migration readiness check
		MigrationReadinessCheckStarted {
			project_id: T::ProjectIdentifier,
			caller: T::AccountId,
		},
		MigrationCheckResponseAccepted {
			project_id: ProjectIdOf<T>,
			query_id: QueryId,
			response: Response,
		},
		MigrationCheckResponseRejected {
			project_id: ProjectIdOf<T>,
			query_id: QueryId,
			response: Response,
		},
		MigrationStarted {
			project_id: T::ProjectIdentifier,
		},
		UserMigrationSent {
			project_id: ProjectIdOf<T>,
			caller: AccountIdOf<T>,
			participant: AccountIdOf<T>,
		},
		MigrationsConfirmed {
			project_id: ProjectIdOf<T>,
			query_id: QueryId,
		},
		MigrationsFailed {
			project_id: ProjectIdOf<T>,
			query_id: QueryId,
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
		/// Tried to move the project to ReadyToLaunch round, but it was not in FundingEnded round before
		ProjectNotInFundingEndedRound,
		/// Tried to start an auction before the initialization period
		TooEarlyForEnglishAuctionStart,
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
		/// The specified project details does not exist
		ProjectDetailsNotFound,
		/// Tried to finish an evaluation before its target end block
		EvaluationPeriodNotEnded,
		/// Tried to access field that is not set
		FieldIsNone,
		/// Checked math failed
		BadMath,
		/// Tried to retrieve a bid but it does not exist
		BidNotFound,
		/// Tried to contribute but its too low to be accepted
		ContributionTooLow,
		/// Contribution is higher than the limit set by the issuer
		ContributionTooHigh,
		/// Tried to delete a project from the update store but it is not there to begin with.
		ProjectNotInUpdateStore,
		/// The provided asset is not accepted by the project issuer
		FundingAssetNotAccepted,
		/// Could not get the price in USD for PLMC
		PLMCPriceNotAvailable,
		/// Could not get the price in USD for the provided asset
		PriceNotFound,
		/// Bond is either lower than the minimum set by the issuer, or the vec is full and can't replace an old one with a lower value
		EvaluationBondTooLow,
		/// Tried to do an operation on an evaluation that does not exist
		EvaluationNotFound,
		/// Tried to do an operation on a finalizer that already finished
		FinalizerFinished,
		///
		ContributionNotFound,
		/// Tried to start a migration check but the bidirectional channel is not yet open
		CommsNotEstablished,
		XcmFailed,
		// Tried to convert one type into another and failed. i.e try_into failed
		BadConversion,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a project and assigns it to the `issuer` account.
		#[pallet::call_index(0)]
		#[pallet::weight(WeightInfoOf::<T>::create())]
		pub fn create(origin: OriginFor<T>, project: ProjectMetadataOf<T>) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			log::trace!(target: "pallet_funding::test", "in create");
			Self::do_create(&issuer, project)
		}

		/// Change the metadata hash of a project
		#[pallet::call_index(1)]
		#[pallet::weight(WeightInfoOf::<T>::edit_metadata())]
		pub fn edit_metadata(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			project_metadata_hash: T::Hash,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			Self::do_edit_metadata(issuer, project_id, project_metadata_hash)
		}

		/// Starts the evaluation round of a project. It needs to be called by the project issuer.
		#[pallet::call_index(2)]
		#[pallet::weight(WeightInfoOf::<T>::start_evaluation())]
		pub fn start_evaluation(origin: OriginFor<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			Self::do_evaluation_start(issuer, project_id)
		}

		/// Starts the auction round for a project. From the next block forward, any professional or
		/// institutional user can set bids for a token_amount/token_price pair.
		/// Any bids from this point until the candle_auction starts, will be considered as valid.
		#[pallet::call_index(3)]
		#[pallet::weight(WeightInfoOf::<T>::start_auction())]
		pub fn start_auction(origin: OriginFor<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
			let issuer = ensure_signed(origin)?;
			Self::do_english_auction(issuer, project_id)
		}

		/// Bond PLMC for a project in the evaluation stage
		#[pallet::call_index(4)]
		#[pallet::weight(WeightInfoOf::<T>::bond_evaluation())]
		pub fn bond_evaluation(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			#[pallet::compact] usd_amount: BalanceOf<T>,
		) -> DispatchResult {
			let evaluator = ensure_signed(origin)?;
			Self::do_evaluate(&evaluator, project_id, usd_amount)
		}

		/// Bid for a project in the Auction round
		#[pallet::call_index(5)]
		#[pallet::weight(WeightInfoOf::<T>::bid())]
		pub fn bid(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
			multiplier: T::Multiplier,
			asset: AcceptedFundingAsset,
		) -> DispatchResult {
			let bidder = ensure_signed(origin)?;
			Self::do_bid(&bidder, project_id, amount, multiplier, asset)
		}

		/// Buy tokens in the Community or Remainder round at the price set in the Auction Round
		#[pallet::call_index(6)]
		#[pallet::weight(WeightInfoOf::<T>::contribute())]
		pub fn contribute(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			#[pallet::compact] amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
			asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let contributor = ensure_signed(origin)?;
			Self::do_contribute(&contributor, project_id, amount, multiplier, asset)
		}

		/// Release evaluation-bonded PLMC when a project finishes its funding round.
		#[pallet::call_index(7)]
		#[pallet::weight(WeightInfoOf::<T>::evaluation_unbond_for())]
		pub fn evaluation_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			evaluator: AccountIdOf<T>,
			bond_id: u32,
		) -> DispatchResult {
			let releaser = ensure_signed(origin)?;
			Self::do_evaluation_unbond_for(&releaser, project_id, &evaluator, bond_id)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(WeightInfoOf::<T>::evaluation_slash_for())]
		pub fn evaluation_slash_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			evaluator: AccountIdOf<T>,
			bond_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_evaluation_slash_for(&caller, project_id, &evaluator, bond_id)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(WeightInfoOf::<T>::evaluation_reward_payout_for())]
		pub fn evaluation_reward_payout_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			evaluator: AccountIdOf<T>,
			bond_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_evaluation_reward_payout_for(&caller, project_id, &evaluator, bond_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(WeightInfoOf::<T>::bid_ct_mint_for())]
		pub fn bid_ct_mint_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_bid_ct_mint_for(&caller, project_id, &bidder, bid_id)
		}

		#[pallet::call_index(11)]
		#[pallet::weight(WeightInfoOf::<T>::contribution_ct_mint_for())]
		pub fn contribution_ct_mint_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_contribution_ct_mint_for(&caller, project_id, &contributor, contribution_id)
		}

		#[pallet::call_index(12)]
		#[pallet::weight(WeightInfoOf::<T>::start_bid_vesting_schedule_for())]
		pub fn start_bid_vesting_schedule_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_start_bid_vesting_schedule_for(&caller, project_id, &bidder, bid_id)
		}

		#[pallet::call_index(13)]
		#[pallet::weight(WeightInfoOf::<T>::start_contribution_vesting_schedule_for())]
		pub fn start_contribution_vesting_schedule_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_start_contribution_vesting_schedule_for(&caller, project_id, &contributor, contribution_id)
		}

		#[pallet::call_index(14)]
		#[pallet::weight(WeightInfoOf::<T>::payout_bid_funds_for())]
		pub fn payout_bid_funds_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_payout_bid_funds_for(&caller, project_id, &bidder, bid_id)
		}

		#[pallet::call_index(15)]
		#[pallet::weight(WeightInfoOf::<T>::payout_contribution_funds_for())]
		pub fn payout_contribution_funds_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_payout_contribution_funds_for(&caller, project_id, &contributor, contribution_id)
		}

		#[pallet::call_index(16)]
		#[pallet::weight(WeightInfoOf::<T>::decide_project_outcome())]
		pub fn decide_project_outcome(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			outcome: FundingOutcomeDecision,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_decide_project_outcome(caller, project_id, outcome)
		}

		#[pallet::call_index(17)]
		#[pallet::weight(WeightInfoOf::<T>::release_bid_funds_for())]
		pub fn release_bid_funds_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_release_bid_funds_for(&caller, project_id, &bidder, bid_id)
		}

		#[pallet::call_index(18)]
		#[pallet::weight(WeightInfoOf::<T>::bid_unbond_for())]
		pub fn bid_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_bid_unbond_for(&caller, project_id, &bidder, bid_id)
		}

		#[pallet::call_index(19)]
		#[pallet::weight(WeightInfoOf::<T>::release_contribution_funds_for())]
		pub fn release_contribution_funds_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_release_contribution_funds_for(&caller, project_id, &contributor, contribution_id)
		}

		#[pallet::call_index(20)]
		#[pallet::weight(WeightInfoOf::<T>::contribution_unbond_for())]
		pub fn contribution_unbond_for(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_contribution_unbond_for(&caller, project_id, &contributor, contribution_id)
		}

		#[pallet::call_index(21)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn set_para_id_for_project(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			para_id: ParaId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_set_para_id_for_project(&caller, project_id, para_id)
		}

		#[pallet::call_index(22)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn start_migration_readiness_check(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_start_migration_readiness_check(&caller, project_id)
		}

		/// Called only by other chains through a query response xcm message
		#[pallet::call_index(23)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn migration_check_response(
			origin: OriginFor<T>,
			query_id: xcm::v3::QueryId,
			response: xcm::v3::Response,
		) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_migration_check_response(location, query_id, response)
		}

		#[pallet::call_index(24)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn start_migration(origin: OriginFor<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_start_migration(&caller, project_id)
		}

		#[pallet::call_index(25)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn migrate_one_participant(
			origin: OriginFor<T>,
			project_id: T::ProjectIdentifier,
			participant: AccountIdOf<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_migrate_one_participant(caller, project_id, participant)
		}

		#[pallet::call_index(26)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn confirm_migrations(origin: OriginFor<T>, query_id: QueryId, response: Response) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_confirm_migrations(location, query_id, response)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: <T as frame_system::Config>::BlockNumber) -> Weight {
			// Get the projects that need to be updated on this block and update them
			for (project_id, update_type) in ProjectsToUpdate::<T>::take(now) {
				match update_type {
					// EvaluationRound -> AuctionInitializePeriod | EvaluationFailed
					UpdateType::EvaluationEnd => {
						unwrap_result_or_skip!(Self::do_evaluation_end(project_id), project_id);
					},

					// AuctionInitializePeriod -> AuctionRound(AuctionPhase::English)
					// Only if it wasn't first handled by user extrinsic
					UpdateType::EnglishAuctionStart => {
						unwrap_result_or_skip!(
							Self::do_english_auction(T::PalletId::get().into_account_truncating(), project_id),
							project_id
						);
					},

					// AuctionRound(AuctionPhase::English) -> AuctionRound(AuctionPhase::Candle)
					UpdateType::CandleAuctionStart => {
						unwrap_result_or_skip!(Self::do_candle_auction(project_id), project_id);
					},

					// AuctionRound(AuctionPhase::Candle) -> CommunityRound
					UpdateType::CommunityFundingStart => {
						unwrap_result_or_skip!(Self::do_community_funding(project_id), project_id);
					},

					// CommunityRound -> RemainderRound
					UpdateType::RemainderFundingStart => {
						unwrap_result_or_skip!(Self::do_remainder_funding(project_id), project_id)
					},

					// CommunityRound || RemainderRound -> FundingEnded
					UpdateType::FundingEnd => {
						unwrap_result_or_skip!(Self::do_end_funding(project_id), project_id)
					},

					UpdateType::ProjectDecision(decision) => {
						unwrap_result_or_skip!(Self::do_project_decision(project_id, decision), project_id)
					},

					UpdateType::StartSettlement => {
						unwrap_result_or_skip!(Self::do_start_settlement(project_id), project_id)
					},
				}
			}
			// TODO: PLMC-127. Set a proper weight
			Weight::from_parts(0, 0)
		}

		fn on_idle(_now: <T as frame_system::Config>::BlockNumber, max_weight: Weight) -> Weight {
			let mut remaining_weight = max_weight;

			let projects_needing_cleanup = ProjectsDetails::<T>::iter()
				.filter_map(|(project_id, info)| match info.cleanup {
					cleaner if cleaner.has_remaining_operations() => Some((project_id, cleaner)),
					_ => None,
				})
				.collect::<Vec<_>>();

			let projects_amount = projects_needing_cleanup.len() as u64;
			if projects_amount == 0 {
				return max_weight
			}

			let mut max_weight_per_project = remaining_weight.saturating_div(projects_amount);

			for (remaining_projects, (project_id, mut cleaner)) in
				projects_needing_cleanup.into_iter().enumerate().rev()
			{
				// TODO: Create this benchmark
				// let mut consumed_weight = WeightInfoOf::<T>::insert_cleaned_project();
				let mut consumed_weight = Weight::from_parts(6_034_000, 0);
				while !consumed_weight.any_gt(max_weight_per_project) {
					if let Ok(weight) = cleaner.do_one_operation::<T>(project_id) {
						consumed_weight.saturating_accrue(weight);
					} else {
						break
					}
				}

				let mut details =
					if let Some(details) = ProjectsDetails::<T>::get(project_id) { details } else { continue };
				details.cleanup = cleaner;
				ProjectsDetails::<T>::insert(project_id, details);

				remaining_weight = remaining_weight.saturating_sub(consumed_weight);
				if remaining_projects > 0 {
					max_weight_per_project = remaining_weight.saturating_div(remaining_projects as u64);
				}
			}

			max_weight.saturating_sub(remaining_weight)
		}
	}

	#[cfg(all(feature = "testing-node", feature = "std"))]
	use frame_support::traits::{OnFinalize, OnIdle, OnInitialize, OriginTrait};
	use pallet_xcm::ensure_response;

	#[cfg(all(feature = "testing-node", feature = "std"))]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[cfg_attr(
		feature = "std",
		serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))
	)]
	#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
	pub struct TestProject<T: Config> {
		pub expected_state: ProjectStatus,
		pub metadata: ProjectMetadataOf<T>,
		pub issuer: AccountIdOf<T>,
		pub evaluations: Vec<instantiator::UserToUSDBalance<T>>,
		pub bids: Vec<instantiator::BidParams<T>>,
		pub community_contributions: Vec<instantiator::ContributionParams<T>>,
		pub remainder_contributions: Vec<instantiator::ContributionParams<T>>,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		#[cfg(all(feature = "testing-node", feature = "std"))]
		pub starting_projects: Vec<TestProject<T>>,
		pub phantom: PhantomData<T>,
	}

	#[cfg(all(feature = "testing-node", feature = "std"))]
	impl<T: Config> Default for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		T::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
	{
		fn default() -> Self {
			Self { starting_projects: vec![], phantom: PhantomData }
		}
	}

	#[cfg(not(all(feature = "testing-node", feature = "std")))]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	#[cfg(not(all(feature = "testing-node", feature = "std")))]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {}
	}

	#[cfg(all(feature = "testing-node", feature = "std"))]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as Config>::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		// AccountIdOf<T>: Into<<instantiator::RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		fn build(&self) {
			{
				type GenesisInstantiator<T> =
					instantiator::Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;
				let mut inst = GenesisInstantiator::<T>::new(None);

				for test_project in self.starting_projects.clone() {
					inst.create_project_at(
						test_project.expected_state,
						test_project.metadata,
						test_project.issuer,
						test_project.evaluations,
						test_project.bids,
						test_project.community_contributions,
						test_project.remainder_contributions,
					);
				}
			}
		}
	}

	// #[cfg(feature = "std")]
	#[cfg(all(feature = "testing-node", feature = "std"))]
	impl<T: Config> GenesisConfig<T>
	where
		T: Config
			+ frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent>
			+ pallet_balances::Config<Balance = BalanceOf<T>>,
		T::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		AccountIdOf<T>: Into<<instantiator::RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build_storage(&self) -> Result<sp_runtime::Storage, String> {
			<Self as GenesisBuild<T>>::build_storage(self)
		}

		/// Direct implementation of `GenesisBuild::assimilate_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn assimilate_storage(&self, storage: &mut sp_runtime::Storage) -> Result<(), String> {
			<Self as GenesisBuild<T>>::assimilate_storage(self, storage)
		}
	}
}

pub mod xcm_executor_impl {
	use super::*;

	pub struct HrmpHandler<T: Config>(PhantomData<T>);
	impl<T: Config> polimec_xcm_executor::HrmpHandler for HrmpHandler<T> {
		fn handle_channel_open_request(message: Instruction) -> XcmResult {
			<Pallet<T>>::do_handle_channel_open_request(message)
		}

		fn handle_channel_accepted(message: Instruction) -> XcmResult {
			<Pallet<T>>::do_handle_channel_accepted(message)
		}
	}
}

pub mod local_macros {
	/// used to unwrap storage values that can be None in places where an error cannot be returned,
	/// but an event should be emitted, and skip to the next iteration of a loop
	#[allow(unused_macros)]
	macro_rules! unwrap_option_or_skip {
		($option:expr, $project_id:expr) => {
			match $option {
				Some(val) => val,
				None => {
					Self::deposit_event(Event::TransitionError {
						project_id: $project_id,
						error: Error::<T>::FieldIsNone.into(),
					});
					continue
				},
			}
		};
	}

	/// used to unwrap storage values that can be Err in places where an error cannot be returned,
	/// but an event should be emitted, and skip to the next iteration of a loop
	macro_rules! unwrap_result_or_skip {
		($option:expr, $project_id:expr) => {
			match $option {
				Ok(val) => val,
				Err(err) => {
					Self::deposit_event(Event::TransitionError { project_id: $project_id, error: err });
					continue
				},
			}
		};
	}
	pub(crate) use unwrap_result_or_skip;
}
