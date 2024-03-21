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
//! | Evaluation Submissions    | Evaluators assess the project information, and if they think it is good enough to get funding, they bond Polimec's native token PLMC with [`bond_evaluation()`](Pallet::evaluate)                                                                                                                                                                                                                    | [`EvaluationRound`](ProjectStatus::EvaluationRound)                 |
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
//! * [`bond_evaluation`](Pallet::evaluate) : Bond PLMC on a project in the evaluation stage. A sort of "bet" that you think the project will be funded
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
pub use crate::weights::WeightInfo;
use frame_support::{
	traits::{
		tokens::{fungible, fungibles, Balance},
		AccountTouch, ContainsPair, Randomness,
	},
	BoundedVec, PalletId,
};
use frame_system::pallet_prelude::BlockNumberFor;
pub use pallet::*;
use pallet_xcm::ensure_response;
use polimec_common::{
	credentials::{Did, EnsureOriginWithCredentials, InvestorType, UntrustedToken},
	migration_types::*,
};
use polkadot_parachain::primitives::Id as ParaId;
use sp_arithmetic::traits::{One, Saturating};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, FixedPointOperand, FixedU128};
use sp_std::{marker::PhantomData, prelude::*};
pub use types::*;
use xcm::v3::{opaque::Instruction, prelude::*, SendXcm};

pub mod functions;
pub mod settlement;

#[cfg(test)]
pub mod mock;
pub mod types;
pub mod weights;

#[cfg(test)]
pub mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub mod instantiator;
pub mod traits;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type ProjectId = u32;
pub type MultiplierOf<T> = <T as Config>::Multiplier;

pub type BalanceOf<T> = <T as Config>::Balance;
pub type PriceOf<T> = <T as Config>::Price;
pub type StringLimitOf<T> = <T as Config>::StringLimit;
pub type HashOf<T> = <T as frame_system::Config>::Hash;
pub type AssetIdOf<T> =
	<<T as Config>::FundingCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
pub type RewardInfoOf<T> = RewardInfo<BalanceOf<T>>;
pub type EvaluatorsOutcomeOf<T> = EvaluatorsOutcome<BalanceOf<T>>;

pub type TicketSizeOf<T> = TicketSize<BalanceOf<T>>;
pub type ProjectMetadataOf<T> =
	ProjectMetadata<BoundedVec<u8, StringLimitOf<T>>, BalanceOf<T>, PriceOf<T>, AccountIdOf<T>, HashOf<T>>;
pub type ProjectDetailsOf<T> =
	ProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;
pub type EvaluationRoundInfoOf<T> = EvaluationRoundInfo<BalanceOf<T>>;
pub type VestingInfoOf<T> = VestingInfo<BlockNumberFor<T>, BalanceOf<T>>;
pub type EvaluationInfoOf<T> = EvaluationInfo<u32, ProjectId, AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>;
pub type BidInfoOf<T> =
	BidInfo<ProjectId, BalanceOf<T>, PriceOf<T>, AccountIdOf<T>, BlockNumberFor<T>, MultiplierOf<T>, VestingInfoOf<T>>;
pub type ContributionInfoOf<T> =
	ContributionInfo<u32, ProjectId, AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, VestingInfoOf<T>>;

pub type ProjectMigrationOriginsOf<T> =
	ProjectMigrationOrigins<ProjectId, BoundedVec<MigrationOrigin, MaxMigrationsPerXcm<T>>>;

pub type BucketOf<T> = Bucket<BalanceOf<T>, PriceOf<T>>;
pub type WeightInfoOf<T> = <T as Config>::WeightInfo;

pub const PLMC_FOREIGN_ID: u32 = 2069;
pub const US_DOLLAR: u128 = 1_0_000_000_000;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::traits::{BondingRequirementCalculation, ProvideAssetPrice, VestingDurationCalculation};
	use frame_support::{
		dispatch::PostDispatchInfo,
		pallet_prelude::*,
		traits::{OnFinalize, OnIdle, OnInitialize},
	};
	use frame_system::pallet_prelude::*;
	use local_macros::*;
	use pallet_xcm::Origin;
use sp_arithmetic::Percent;
	use sp_runtime::{
		traits::{Convert, ConvertBack, Get}, 
		DispatchErrorWithPostInfo
	};

	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	use crate::traits::SetPrices;

	#[pallet::composite_enum]
	pub enum HoldReason {
		Evaluation(ProjectId),
		Participation(ProjectId),
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_balances::Config<Balance = BalanceOf<Self>> + pallet_xcm::Config
	{
		/// A way to convert from and to the account type used in CT migrations
		type AccountId32Conversion: ConvertBack<Self::AccountId, [u8; 32]>;

		/// Type used for testing and benchmarks
		#[cfg(any(test, feature = "runtime-benchmarks", feature = "std"))]
		type AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<Self>>
			+ OnIdle<BlockNumberFor<Self>>
			+ OnInitialize<BlockNumberFor<Self>>;

		/// The time window (expressed in number of blocks) that an issuer has to start the auction round.
		#[pallet::constant]
		type AuctionInitializePeriodDuration: Get<BlockNumberFor<Self>>;

		/// The inner balance type we will use for all of our outer currency types. (e.g native, funding, CTs)
		type Balance: Balance + From<u64> + FixedPointOperand + MaybeSerializeDeserialize + Into<u128>;

		// TODO: our local BlockNumber should be removed once we move onto using Moment for time tracking
		/// BlockNumber used for PLMC vesting durations on this chain, and CT vesting durations on funded chains.
		type BlockNumber: IsType<BlockNumberFor<Self>> + Into<u64>;

		/// The length (expressed in number of blocks) of the Auction Round, Candle period.
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		/// The length (expressed in number of blocks) of the Auction Round, Candle period.
		#[pallet::constant]
		type CandleAuctionDuration: Get<BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the Community Round.
		#[pallet::constant]
		type CommunityFundingDuration: Get<BlockNumberFor<Self>>;

		/// The currency used for minting contribution tokens as fungible assets (i.e pallet-assets)
		type ContributionTokenCurrency: fungibles::Create<AccountIdOf<Self>, AssetId = ProjectId, Balance = BalanceOf<Self>>
			+ fungibles::Destroy<AccountIdOf<Self>, AssetId = ProjectId, Balance = BalanceOf<Self>>
			+ fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>>
			+ fungibles::metadata::MetadataDeposit<BalanceOf<Self>>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungibles::roles::Inspect<AccountIdOf<Self>>
			+ AccountTouch<ProjectId, AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ ContainsPair<ProjectId, AccountIdOf<Self>>;

		/// Convert 24 hours as FixedU128, to the corresponding amount of blocks in the same type as frame_system
		type DaysToBlocks: Convert<FixedU128, BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the Auction Round, English period.
		#[pallet::constant]
		type EnglishAuctionDuration: Get<BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the evaluation period.
		#[pallet::constant]
		type EvaluationDuration: Get<BlockNumberFor<Self>>;

		/// What percentage of the target funding amount is required to be reached in the evaluation, for it to continue to the funding round.
		#[pallet::constant]
		type EvaluationSuccessThreshold: Get<Percent>;

		/// How much an evaluation should be slashed if it the project doesn't reach a certain theshold of funding.
		#[pallet::constant]
		type EvaluatorSlash: Get<Percent>;

		/// The fee brackets for the project's funding
		#[pallet::constant]
		type FeeBrackets: Get<Vec<(Percent, <Self as Config>::Balance)>>;

		/// The currency used for funding projects in bids and contributions
		type FundingCurrency: fungibles::InspectEnumerable<AccountIdOf<Self>, Balance = BalanceOf<Self>, AssetId = u32>
			+ fungibles::metadata::Inspect<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>, AssetId = u32>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// Credentialized investor Origin, ensures users are of investing type Retail, or Professional, or Institutional.
		type InvestorOrigin: EnsureOriginWithCredentials<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = (AccountIdOf<Self>, Did, InvestorType),
		>;

		/// How long an issuer has to accept or reject the funding of a project if the funding is between two thresholds.
		#[pallet::constant]
		type ManualAcceptanceDuration: Get<BlockNumberFor<Self>>;

		/// Max individual bids per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxBidsPerProject: Get<u32>;

		/// Max individual bids per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxBidsPerUser: Get<u32>;

		/// Range of max_capacity_thresholds values for the hrmp config where we accept the incoming channel request
		#[pallet::constant]
		type MaxCapacityThresholds: Get<(u32, u32)>;

		/// Max individual contributions per project per user. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxContributionsPerUser: Get<u32>;

		/// Max individual evaluations per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxEvaluationsPerProject: Get<u32>;

		/// How many distinct evaluations per user per project
		type MaxEvaluationsPerUser: Get<u32>;

		/// Range of max_message_size values for the hrmp config where we accept the incoming channel request
		#[pallet::constant]
		type MaxMessageSizeThresholds: Get<(u32, u32)>;

		/// max iterations for trying to insert a project on the projects_to_update storage
		#[pallet::constant]
		type MaxProjectsToUpdateInsertionAttempts: Get<u32>;

		/// How many projects should we update in on_initialize each block. Likely one to reduce complexity
		#[pallet::constant]
		type MaxProjectsToUpdatePerBlock: Get<u32>;

		/// Multiplier type that decides how much PLMC needs to be bonded for a token buy/bid
		type Multiplier: Parameter
			+ BondingRequirementCalculation
			+ VestingDurationCalculation
			+ Default
			+ Copy
			+ TryFrom<u8>
			+ MaxEncodedLen
			+ MaybeSerializeDeserialize;

		/// The chains native currency
		type NativeCurrency: fungible::InspectHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::MutateHold<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				Reason = <Self as Config>::RuntimeHoldReason,
			> + fungible::BalancedHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// System account for the funding pallet. Used to derive project escrow accounts.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Pallet info of the polimec receiver pallet. Used for CT migrations
		#[pallet::constant]
		type PolimecReceiverInfo: Get<PalletInfo>;

		/// The maximum size of a preimage allowed, expressed in bytes.
		#[pallet::constant]
		type PreImageLimit: Get<u32>;

		/// Type that represents the value of something in USD
		type Price: FixedPointNumber + Parameter + Copy + MaxEncodedLen + MaybeSerializeDeserialize;

		/// Method to get the price of an asset like USDT or PLMC. Likely to come from an oracle
		type PriceProvider: ProvideAssetPrice<AssetId = u32, Price = Self::Price>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the Remainder Round.
		#[pallet::constant]
		type RemainderFundingDuration: Get<BlockNumberFor<Self>>;

		/// max_capacity config required for the channel from polimec to the project
		#[pallet::constant]
		type RequiredMaxCapacity: Get<u32>;

		/// max_message_size config required for the channel from polimec to the project
		#[pallet::constant]
		type RequiredMaxMessageSize: Get<u32>;

		/// The runtime enum constructed by the construct_runtime macro
		type RuntimeCall: Parameter + IsType<<Self as frame_system::Config>::RuntimeCall> + From<Call<Self>>;

		/// The event enum constructed by the construct_runtime macro
		type RuntimeEvent: From<Event<Self>>
			+ TryInto<Event<Self>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>
			+ Parameter
			+ Member;

		/// The hold reason enum constructed by the construct_runtime macro
		type RuntimeHoldReason: From<HoldReason>;

		/// The origin enum constructed by the construct_runtime macro
		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		/// test and benchmarking helper to set the prices of assets
		#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
		type SetPrices: SetPrices;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// How long a project has to wait after it gets successfully funded, for the settlement to start.
		#[pallet::constant]
		type SuccessToSettlementTime: Get<BlockNumberFor<Self>>;

		/// Treasury account holding PLMC at TGE.
		#[pallet::constant]
		type ProtocolGrowthTreasury: Get<AccountIdOf<Self>>;

		/// Treasury account holding the CT fees charged to issuers.
		#[pallet::constant]
		type ContributionTreasury: Get<AccountIdOf<Self>>;

		/// The Ed25519 Verifier Public Key of credential JWTs
		type VerifierPublicKey: Get<[u8; 32]>;

		/// The type used for vesting
		type Vesting: polimec_common::ReleaseSchedule<
			AccountIdOf<Self>,
			<Self as Config>::RuntimeHoldReason,
			Currency = Self::NativeCurrency,
			Moment = BlockNumberFor<Self>,
		>;

		/// Struct holding information about extrinsic weights
		type WeightInfo: weights::WeightInfo;
	}

	#[pallet::storage]
	/// A global counter for indexing the projects
	/// OnEmpty in this case is GetDefault, so 0.
	pub type NextProjectId<T: Config> = StorageValue<_, ProjectId, ValueQuery>;

	#[pallet::storage]
	pub type NextEvaluationId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type NextBidId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type NextContributionId<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type BidCounts<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, u32, ValueQuery>;

	#[pallet::storage]
	pub type EvaluationCounts<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, u32, ValueQuery>;

	#[pallet::storage]
	/// A global counter used in the randomness generation
	// TODO: PLMC-155. Remove it after using the Randomness from BABE's VRF: https://github.com/PureStake/moonbeam/issues/1391
	// 	Or use the randomness from Moonbeam.
	pub type Nonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	/// A StorageMap containing all the hashes of the project metadata uploaded by the users.
	pub type Images<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, AccountIdOf<T>>;

	#[pallet::storage]
	/// A StorageMap containing the primary project information of projects
	pub type ProjectsMetadata<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, ProjectMetadataOf<T>>;

	#[pallet::storage]
	/// A StorageMap containing the primary project information of projects
	pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, BucketOf<T>>;

	#[pallet::storage]
	/// StorageMap containing additional information for the projects, relevant for correctness of the protocol
	pub type ProjectsDetails<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, ProjectDetailsOf<T>>;

	#[pallet::storage]
	/// A map to know in which block to update which active projects using on_initialize.
	pub type ProjectsToUpdate<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		BoundedVec<(ProjectId, UpdateType), T::MaxProjectsToUpdatePerBlock>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Keep track of the PLMC bonds made to each project by each evaluator
	pub type Evaluations<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, ProjectId>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		EvaluationInfoOf<T>,
	>;

	#[pallet::storage]
	/// StorageMap containing the bids for each project and user
	pub type Bids<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, ProjectId>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		BidInfoOf<T>,
	>;

	#[pallet::storage]
	/// Contributions made during the Community and Remainder round. i.e token buys
	pub type Contributions<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, ProjectId>,
			NMapKey<Blake2_128Concat, AccountIdOf<T>>,
			NMapKey<Blake2_128Concat, u32>,
		),
		ContributionInfoOf<T>,
	>;

	#[pallet::storage]
	pub type AuctionBoughtUSD<T: Config> = StorageNMap<
		_,
		(NMapKey<Blake2_128Concat, ProjectId>, NMapKey<Blake2_128Concat, Did>),
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type ContributionBoughtUSD<T: Config> = StorageNMap<
		_,
		(NMapKey<Blake2_128Concat, ProjectId>, NMapKey<Blake2_128Concat, Did>),
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type MigrationQueue<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		ProjectId,
		Blake2_128Concat,
		T::AccountId, 
		BoundedVec<Migration, MaxParticipationsPerUser<T>>,
		ValueQuery,
	>;

	
	pub struct MaxParticipationsPerUser<T: Config>(PhantomData<T>);
	impl<T: Config> Get<u32> for MaxParticipationsPerUser<T> {
		fn get() -> u32 {
			T::MaxContributionsPerUser::get() + T::MaxBidsPerUser::get() + T::MaxEvaluationsPerUser::get()
		}
	}

	#[pallet::storage]
	/// Migrations sent and awaiting for confirmation
	pub type UnconfirmedMigrations<T: Config> = StorageMap<_, Blake2_128Concat, QueryId, ProjectMigrationOriginsOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A project was created.
		ProjectCreated {
			project_id: ProjectId,
			issuer: T::AccountId,
		},
		/// The metadata of a project was modified.
		MetadataEdited {
			project_id: ProjectId,
		},
		/// The evaluation phase of a project started.
		EvaluationStarted {
			project_id: ProjectId,
		},
		/// The evaluation phase of a project ended without reaching the minimum threshold of evaluation bonds.
		EvaluationFailed {
			project_id: ProjectId,
		},
		/// The period an issuer has to start the auction phase of the project.
		AuctionInitializePeriod {
			project_id: ProjectId,
			start_block: BlockNumberFor<T>,
			end_block: BlockNumberFor<T>,
		},
		/// The auction round of a project started.
		EnglishAuctionStarted {
			project_id: ProjectId,
			when: BlockNumberFor<T>,
		},
		/// The candle auction part of the auction started for a project
		CandleAuctionStarted {
			project_id: ProjectId,
			when: BlockNumberFor<T>,
		},
		/// The auction round of a project ended.
		AuctionFailed {
			project_id: ProjectId,
		},
		/// A `bonder` bonded an `amount` of PLMC for `project_id`.
		FundsBonded {
			project_id: ProjectId,
			amount: BalanceOf<T>,
			bonder: AccountIdOf<T>,
		},
		/// Someone paid for the release of a user's PLMC bond for a project.
		BondReleased {
			project_id: ProjectId,
			amount: BalanceOf<T>,
			bonder: AccountIdOf<T>,
			releaser: AccountIdOf<T>,
		},
		/// A bid was made for a project
		Bid {
			project_id: ProjectId,
			amount: BalanceOf<T>,
			price: T::Price,
			multiplier: MultiplierOf<T>,
		},
		/// A contribution was made for a project. i.e token purchase
		Contribution {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
		},
		/// A project is now in its community funding round
		CommunityFundingStarted {
			project_id: ProjectId,
		},
		/// A project is now in the remainder funding round
		RemainderFundingStarted {
			project_id: ProjectId,
		},
		/// A project has now finished funding
		FundingEnded {
			project_id: ProjectId,
			outcome: FundingOutcome,
		},
		/// Something was not properly initialized. Most likely due to dev error manually calling do_* functions or updating storage
		TransitionError {
			project_id: ProjectId,
			error: DispatchError,
		},
		/// Something terribly wrong happened where the bond could not be unbonded. Most likely a programming error
		EvaluationUnbondFailed {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		/// Contribution tokens were minted to a user
		ContributionTokenMinted {
			releaser: AccountIdOf<T>,
			project_id: ProjectId,
			claimer: AccountIdOf<T>,
			amount: BalanceOf<T>,
		},
		/// A transfer of tokens failed, but because it was done inside on_initialize it cannot be solved.
		TransferError {
			error: DispatchError,
		},
		EvaluationRewardFailed {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		EvaluationSlashFailed {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ReleaseBidFundsFailed {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		BidUnbondFailed {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ReleaseContributionFundsFailed {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		ContributionUnbondFailed {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		PayoutContributionFundsFailed {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		PayoutBidFundsFailed {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		EvaluationRewarded {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		EvaluationSlashed {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		CTMintFailed {
			project_id: ProjectId,
			claimer: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		StartBidderVestingScheduleFailed {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		StartContributionVestingScheduleFailed {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			error: DispatchError,
		},
		BidPlmcVestingScheduled {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionPlmcVestingScheduled {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ParticipantPlmcVested {
			project_id: ProjectId,
			participant: AccountIdOf<T>,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		BidFundingPaidOut {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionFundingPaidOut {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		BidFundingReleased {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ContributionFundingReleased {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			amount: BalanceOf<T>,
			caller: AccountIdOf<T>,
		},
		ProjectOutcomeDecided {
			project_id: ProjectId,
			decision: FundingOutcomeDecision,
		},
		ProjectParaIdSet {
			project_id: ProjectId,
			para_id: ParaId,
			caller: T::AccountId,
		},
		/// A channel was accepted from a parachain to Polimec belonging to a project. A request has been sent to the relay for a Polimec->project channel
		HrmpChannelAccepted {
			project_id: ProjectId,
			para_id: ParaId,
		},
		/// A channel was established from Polimec to a project. The relay has notified us of their acceptance of our request
		HrmpChannelEstablished {
			project_id: ProjectId,
			para_id: ParaId,
		},
		/// Started a migration readiness check
		MigrationReadinessCheckStarted {
			project_id: ProjectId,
			caller: T::AccountId,
		},
		MigrationCheckResponseAccepted {
			project_id: ProjectId,
			query_id: QueryId,
			response: Response,
		},
		MigrationCheckResponseRejected {
			project_id: ProjectId,
			query_id: QueryId,
			response: Response,
		},
		MigrationStarted {
			project_id: ProjectId,
		},
		UserMigrationSent {
			project_id: ProjectId,
			caller: AccountIdOf<T>,
			participant: AccountIdOf<T>,
		},
		MigrationsConfirmed {
			project_id: ProjectId,
			migration_origins: BoundedVec<MigrationOrigin, MaxMigrationsPerXcm<T>>,
		},
		MigrationsFailed {
			project_id: ProjectId,
			migration_origins: BoundedVec<MigrationOrigin, MaxMigrationsPerXcm<T>>,
		},
		ReleaseFutureCTDepositFailed {
			project_id: ProjectId,
			participant: AccountIdOf<T>,
			error: DispatchError,
		},
		FutureCTDepositReleased {
			project_id: ProjectId,
			participant: AccountIdOf<T>,
			caller: AccountIdOf<T>,
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
		/// The participation currencies specified are invalid
		ParticipationCurrenciesError,
		/// The specified project does not exist
		ProjectNotFound,
		/// The Evaluation Round of the project has not started yet
		EvaluationNotStarted,
		/// The Evaluation Round of the project has ended without reaching the minimum threshold
		EvaluationFailed,
		/// The issuer cannot participate to their own project
		ParticipationToThemselves,
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
		/// Bid above the ticket size limit
		BidTooHigh,
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
		/// The issuer doesn't have enough funds (ExistentialDeposit), to create the escrow account
		NotEnoughFundsForEscrowCreation,
		/// The issuer doesn't have enough funds to pay for the metadata of their contribution token
		NotEnoughFundsForCTMetadata,
		/// The issuer doesn't have enough funds to pay for the deposit of their contribution token
		NotEnoughFundsForCTDeposit,
		/// Too many attempts to insert project in to ProjectsToUpdate storage
		TooManyInsertionAttempts,
		/// Reached bid limit for this user on this project
		TooManyBidsForUser,
		/// Reached bid limit for this project
		TooManyBidsForProject,
		/// Reached evaluation limit for this project
		TooManyEvaluationsForProject,
		/// Reached contribution limit for this user on this project
		TooManyContributionsForUser,
		/// Reached the migration queue limit for a user.
		TooManyMigrations,
		/// User has no migrations to execute.
		NoMigrationsFound,
		// Participant tried to do a community contribution but it already had a winning bid on the auction round.
		UserHasWinningBids,
		// Round transition already happened.
		RoundTransitionAlreadyHappened,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a project and assigns it to the `issuer` account.
		#[pallet::call_index(0)]
		#[pallet::weight(WeightInfoOf::<T>::create())]
		pub fn create(origin: OriginFor<T>, jwt: UntrustedToken, project: ProjectMetadataOf<T>) -> DispatchResult {
			let (account, did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			log::trace!(target: "pallet_funding::test", "in create");
			Self::do_create(&account, project, did)
		}

		/// Change the metadata hash of a project
		#[pallet::call_index(1)]
		#[pallet::weight(WeightInfoOf::<T>::edit_metadata())]
		pub fn edit_metadata(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			project_metadata_hash: T::Hash,
		) -> DispatchResult {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			Self::do_edit_metadata(account, project_id, project_metadata_hash)
		}

		/// Starts the evaluation round of a project. It needs to be called by the project issuer.
		#[pallet::call_index(2)]
		#[pallet::weight(WeightInfoOf::<T>::start_evaluation(<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1))]
		pub fn start_evaluation(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
		) -> DispatchResultWithPostInfo {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			Self::do_start_evaluation(account, project_id)
		}

		/// Starts the auction round for a project. From the next block forward, any professional or
		/// institutional user can set bids for a token_amount/token_price pair.
		/// Any bids from this point until the candle_auction starts, will be considered as valid.
		#[pallet::call_index(3)]
		#[pallet::weight(WeightInfoOf::<T>::start_auction_manually(<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1))]
		pub fn start_auction(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
		) -> DispatchResultWithPostInfo {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			Self::do_english_auction(account, project_id)
		}

		/// Bond PLMC for a project in the evaluation stage
		#[pallet::call_index(4)]
		#[pallet::weight(
			WeightInfoOf::<T>::evaluation_to_limit(T::MaxEvaluationsPerUser::get() - 1)
			.max(WeightInfoOf::<T>::evaluation_over_limit())
		)]
		pub fn evaluate(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] usd_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let (account, did, _investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			Self::do_evaluate(&account, project_id, usd_amount, did)
		}

		/// Bid for a project in the Auction round
		#[pallet::call_index(5)]
		#[pallet::weight(
			WeightInfoOf::<T>::bid(
				<T as Config>::MaxBidsPerUser::get() - 1,
				// Assuming the current bucket is full, and has a price higher than the minimum.
				// This user is buying 100% of the bid allocation.
				// Since each bucket has 10% of the allocation, one bid can be split into a max of 10
				10
		))]
		pub fn bid(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] amount: BalanceOf<T>,
			multiplier: T::Multiplier,
			asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let (account, did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			Self::do_bid(&account, project_id, amount, multiplier, asset, did, investor_type)
		}

		/// Buy tokens in the Community or Remainder round at the price set in the Auction Round
		#[pallet::call_index(6)]
		#[pallet::weight(
			WeightInfoOf::<T>::contribution(T::MaxContributionsPerUser::get() - 1)
			.max(WeightInfoOf::<T>::contribution_ends_round(
			// Last contribution possible before having to remove an old lower one
			<T as Config>::MaxContributionsPerUser::get() -1,
			// Since we didn't remove any previous lower contribution, we can buy all remaining CTs and try to move to the next phase
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			))
		)]
		pub fn community_contribute(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
			asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let (account, did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			Self::do_community_contribute(&account, project_id, amount, multiplier, asset, did, investor_type)
		}

		/// Buy tokens in the Community or Remainder round at the price set in the Auction Round
		#[pallet::call_index(7)]
		#[pallet::weight(
			WeightInfoOf::<T>::contribution(T::MaxContributionsPerUser::get() - 1)
			.max(WeightInfoOf::<T>::contribution_ends_round(
			// Last contribution possible before having to remove an old lower one
			<T as Config>::MaxContributionsPerUser::get() -1,
			// Since we didn't remove any previous lower contribution, we can buy all remaining CTs and try to move to the next phase
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1
			))
		)]
		pub fn remaining_contribute(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
			asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let (account, did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			Self::do_remaining_contribute(&account, project_id, amount, multiplier, asset, did, investor_type)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(WeightInfoOf::<T>::decide_project_outcome(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1
		))]
		pub fn decide_project_outcome(
			origin: OriginFor<T>,
			project_id: ProjectId,
			outcome: FundingOutcomeDecision,
		) -> DispatchResultWithPostInfo {
			let caller = ensure_signed(origin)?;
			Self::do_decide_project_outcome(caller, project_id, outcome)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_successful_evaluation(
			origin: OriginFor<T>,
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			evaluation_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_successful_evaluation(bid, project_id)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_successful_bid (
			origin: OriginFor<T>,
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_successful_bid(bid, project_id)
		}

		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_successful_contribution(
			origin: OriginFor<T>,
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_successful_contribution(bid, project_id)
		}

		#[pallet::call_index(12)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_failed_evaluation(
			origin: OriginFor<T>,
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			evaluation_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_failed_evaluation(bid, project_id)
		}

		#[pallet::call_index(13)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_failed_bid (
			origin: OriginFor<T>,
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_failed_bid(bid, project_id)
		}

		#[pallet::call_index(14)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn settle_failed_contribution(
			origin: OriginFor<T>,
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
			Self::do_settle_failed_contribution(bid, project_id)
		}


		#[pallet::call_index(22)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn set_para_id_for_project(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			para_id: ParaId,
		) -> DispatchResult {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);

			Self::do_set_para_id_for_project(&account, project_id, para_id)
		}

		#[pallet::call_index(23)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn start_migration_readiness_check(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
		) -> DispatchResult {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			Self::do_start_migration_readiness_check(&account, project_id)
		}

		/// Called only by other chains through a query response xcm message
		#[pallet::call_index(24)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn migration_check_response(
			origin: OriginFor<T>,
			query_id: xcm::v3::QueryId,
			response: xcm::v3::Response,
		) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_migration_check_response(location, query_id, response)
		}

		#[pallet::call_index(25)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn start_migration(origin: OriginFor<T>, jwt: UntrustedToken, project_id: ProjectId) -> DispatchResult {
			let (account, _did, investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::NotAllowed);
			Self::do_start_migration(&account, project_id)
		}

		#[pallet::call_index(26)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn migrate_one_participant(
			origin: OriginFor<T>,
			project_id: ProjectId,
			participant: AccountIdOf<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			Self::do_migrate_one_participant(caller, project_id, participant)
		}

		#[pallet::call_index(27)]
		#[pallet::weight(Weight::from_parts(1000, 0))]
		pub fn confirm_migrations(origin: OriginFor<T>, query_id: QueryId, response: Response) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_confirm_migrations(location, query_id, response)
		}

		#[pallet::call_index(28)]
		#[pallet::weight(WeightInfoOf::<T>::end_evaluation_success(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
		))]
		pub fn root_do_evaluation_end(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_evaluation_end(project_id)
		}

		#[pallet::call_index(29)]
		#[pallet::weight(WeightInfoOf::<T>::start_candle_phase(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
		))]
		pub fn root_do_candle_auction(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_candle_auction(project_id)
		}

		#[pallet::call_index(30)]
		#[pallet::weight(WeightInfoOf::<T>::start_community_funding_success(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			<T as Config>::MaxBidsPerProject::get() / 2,
			<T as Config>::MaxBidsPerProject::get() / 2,
		)
		.max(WeightInfoOf::<T>::start_community_funding_success(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			<T as Config>::MaxBidsPerProject::get(),
			0u32,
		))
		.max(WeightInfoOf::<T>::start_community_funding_success(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			0u32,
			<T as Config>::MaxBidsPerProject::get(),
		)))]
		pub fn root_do_community_funding(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_community_funding(project_id)
		}

		#[pallet::call_index(31)]
		#[pallet::weight(WeightInfoOf::<T>::start_remainder_funding(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
		))]
		pub fn root_do_remainder_funding(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_remainder_funding(project_id)
		}

		#[pallet::call_index(32)]
		#[pallet::weight(WeightInfoOf::<T>::end_funding_automatically_rejected_evaluators_slashed(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			)
		.max(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_slashed(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			))
		.max(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_unchanged(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			))
		.max(WeightInfoOf::<T>::end_funding_automatically_accepted_evaluators_rewarded(
			<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
			<T as Config>::MaxEvaluationsPerProject::get(),
		)))]
		pub fn root_do_end_funding(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_end_funding(project_id)
		}

		#[pallet::call_index(33)]
		#[pallet::weight(WeightInfoOf::<T>::project_decision_accept_funding()
		.max(WeightInfoOf::<T>::project_decision_reject_funding()))]
		pub fn root_do_project_decision(
			origin: OriginFor<T>,
			project_id: ProjectId,
			decision: FundingOutcomeDecision,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_project_decision(project_id, decision)
		}

		#[pallet::call_index(34)]
		#[pallet::weight(WeightInfoOf::<T>::start_settlement_funding_success()
		.max(WeightInfoOf::<T>::start_settlement_funding_failure()))]
		pub fn root_do_start_settlement(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::do_start_settlement(project_id)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			// Get the projects that need to be updated on this block and update them
			let mut used_weight = Weight::from_parts(0, 0);
			for (project_id, update_type) in ProjectsToUpdate::<T>::take(now) {
				match update_type {
					// EvaluationRound -> AuctionInitializePeriod | EvaluationFailed
					UpdateType::EvaluationEnd => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_evaluation_end(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(WeightInfoOf::<T>::end_evaluation_success(
								<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
							)),
						);
					},

					// AuctionInitializePeriod -> AuctionRound(AuctionPhase::English)
					// Only if it wasn't first handled by user extrinsic
					UpdateType::EnglishAuctionStart => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_english_auction(T::PalletId::get().into_account_truncating(), project_id,),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(WeightInfoOf::<T>::start_auction_manually(
								<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
							)),
						);
					},

					// AuctionRound(AuctionPhase::English) -> AuctionRound(AuctionPhase::Candle)
					UpdateType::CandleAuctionStart => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_candle_auction(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(WeightInfoOf::<T>::start_candle_phase(
								<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
							)),
						);
					},

					// AuctionRound(AuctionPhase::Candle) -> CommunityRound
					UpdateType::CommunityFundingStart => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_community_funding(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(
								WeightInfoOf::<T>::start_community_funding_success(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
									<T as Config>::MaxBidsPerProject::get() / 2,
									<T as Config>::MaxBidsPerProject::get() / 2,
								)
								.max(WeightInfoOf::<T>::start_community_funding_success(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
									<T as Config>::MaxBidsPerProject::get(),
									0u32,
								))
								.max(WeightInfoOf::<T>::start_community_funding_success(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
									0u32,
									<T as Config>::MaxBidsPerProject::get(),
								)),
							),
						);
					},

					// CommunityRound -> RemainderRound
					UpdateType::RemainderFundingStart => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_remainder_funding(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(WeightInfoOf::<T>::start_remainder_funding(
								<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
							)),
						);
					},

					// CommunityRound || RemainderRound -> FundingEnded
					UpdateType::FundingEnd => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_end_funding(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(
								WeightInfoOf::<T>::end_funding_automatically_rejected_evaluators_slashed(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
								)
								.max(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_slashed(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
								))
								.max(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_unchanged(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
								))
								.max(WeightInfoOf::<T>::end_funding_automatically_accepted_evaluators_rewarded(
									<T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1,
									<T as Config>::MaxEvaluationsPerProject::get(),
								)),
							),
						);
					},

					UpdateType::ProjectDecision(decision) => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_project_decision(project_id, decision),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(
								WeightInfoOf::<T>::project_decision_accept_funding()
									.max(WeightInfoOf::<T>::project_decision_reject_funding()),
							),
						);
					},

					UpdateType::StartSettlement => {
						used_weight = used_weight.saturating_add(
							unwrap_result_or_skip!(
								Self::do_start_settlement(project_id),
								project_id,
								|e: DispatchErrorWithPostInfo<PostDispatchInfo>| { e.error }
							)
							.actual_weight
							.unwrap_or(
								WeightInfoOf::<T>::start_settlement_funding_success()
									.max(WeightInfoOf::<T>::start_settlement_funding_failure()),
							),
						);
					},
				}
			}

			used_weight
		}
	}

	#[pallet::genesis_config]
	#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
	pub struct GenesisConfig<T: Config>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		#[cfg(feature = "std")]
		pub starting_projects: Vec<instantiator::TestProjectParams<T>>,
		pub phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		fn default() -> Self {
			Self {
				#[cfg(feature = "std")]
				starting_projects: vec![],
				phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		fn build(&self) {
			#[cfg(feature = "std")]
			{
				type GenesisInstantiator<T> =
					instantiator::Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;
				let inst = GenesisInstantiator::<T>::new(None);
				<T as Config>::SetPrices::set_prices();
				instantiator::async_features::create_multiple_projects_at(inst, self.starting_projects.clone());

				frame_system::Pallet::<T>::set_block_number(0u32.into());
			}
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
					continue;
				},
			}
		};
	}

	/// used to unwrap storage values that can be Err in places where an error cannot be returned,
	/// but an event should be emitted, and skip to the next iteration of a loop
	macro_rules! unwrap_result_or_skip {
		($option:expr, $project_id:expr, $error_handler:expr) => {
			match $option {
				Ok(val) => val,
				Err(err) => {
					Self::deposit_event(Event::TransitionError { project_id: $project_id, error: $error_handler(err) });
					continue;
				},
			}
		};
	}
	pub(crate) use unwrap_result_or_skip;
}
