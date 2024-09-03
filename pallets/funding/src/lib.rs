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
//! Participants get contribution tokens. Contribution tokens are tokens issued by projects which successfully raised
//! funds on Polimec. They are distributed to evaluators and participants who contributed to the project’s successful
//! funding round. Contribution tokens are transferability-locked in the wallet of participants or evaluators after
//! distribution and are automatically converted to the project’s transferable mainnet token at launch.
//!
//! ## Overview
//! The official logic for Polimec's blockchain can be found in our [knowledge hub](https://hub.polimec.org/).
//!
//! There are 3 types of users in Polimec:
//! - **Issuers**: They create projects and are responsible for their success.
//! - **Evaluators**: They are incentivized to assess projects accurately by locking their PLMC. If at least 10% of its
//! target funding (in USD) is locked in PLMC, a project is given access to the funding round. Evaluators are either
//! rewarded in contribution tokens if the project gets funded, or have their PLMC slashed otherwise.
//! - **Participants**: They contribute financially to projects by locking PLMC and paying out USDT/USDC/DOT, and are rewarded in contribution tokens.
//!
//! Users need to go through a KYC/AML by a third party in order to use the protocol. This process classifies them
//! into one of the following categories, based on their investment experience and financial status:
//! - **Institutional**: Can take the role of issuer, evaluator, and participant. Can participate in both auction and community rounds.
//! - **Professional**: Can take the role of evaluator and participant. Can participate in both auction and community rounds.
//! - **Retail**: Can take the role of evaluator and participant. Can only participate in the community round.
//!
//! Basic flow of a project's lifecycle:
//! 1) **Project Creation**: Issuer creates a project with the [`create_project`](Pallet::create_project) extrinsic.
//! 2) **Evaluation Start**: Issuer starts the evaluation round with the [`start_evaluation`](Pallet::start_evaluation) extrinsic.
//! 3) **Evaluate**: Evaluators bond PLMC to evaluate a project with the [`evaluate`](Pallet::evaluate) extrinsic.
//! 4) **Evaluation End**: Anyone can end the evaluation round with the [`end_evaluation`](Pallet::end_evaluation) extrinsic after the defined end block.
//! 5) **Auction Start**: If the project receives at least 10% of its target funding (in USD) in PLMC bonded, the auction starts immediately after `end_evaluation` is called.
//! 6) **Bid**: Professional and institutional investors can place bids on the project using the [`bid`](Pallet::bid) extrinsic. The price starts at the issuer-defined minimum, and increases by increments of 10% in price and bucket size.
//! 7) **Auction End**: Anyone can end the auction round with the [`end_auction`](Pallet::end_auction) extrinsic after the defined end block.
//! 8) **Community Round Start**: After `end_auction` is called, a weighted average price is calculated from the bids, and the community round starts.
//! 9) **Contribute**: Anyone without a winning bid can now contribute at the weighted average price with the [`contribute`](Pallet::contribute) extrinsic.
//! 10) **Remainder Round Start**: After a defined [period](<T as Config>::CommunityRoundDuration), the remainder round starts.
//! 11) **Contribute**: Participants with winning bids can also contribute at the weighted average price with the [`contribute`](Pallet::contribute) extrinsic.
//! 12) **Funding End**: Anyone can end the project with the [`end_project`](Pallet::end_project) extrinsic after the defined end block.
//! The project will now be considered Failed if it reached <=33% of its target funding in USD, and Successful otherwise.
//! 13) **Settlement Start**: Anyone can start the settlement process with the [`start_settlement`](Pallet::start_settlement) extrinsic after the defined end block.
//! 14) **Settle Evaluation**: Anyone can now settle an evaluation with the [`settle_evaluation`](Pallet::settle_evaluation) extrinsic.
//! This will unlock the PLMC bonded, and either apply a slash to the PLMC, or reward CTs to the evaluator.
//! 15) **Settle Bid**: Anyone can now settle a bid with the [`settle_bid`](Pallet::settle_bid) extrinsic.
//! This will set a vesting schedule on the PLMC bonded, and pay out the funding assets to the issuer. It will also issue refunds in case the bid failed,
//! or the price paid was higher than the weighted average price.
//! 16) **Settle Contribution**: Anyone can now settle a contribution with the [`settle_contribution`](Pallet::settle_contribution) extrinsic.
//! This will set a vesting schedule on the PLMC bonded, and pay out the funding assets to the issuer.
//! 17) **Settlement End**: Anyone can now mark the project settlement as finished by calling the [`mark_project_as_settled`](Pallet::mark_project_as_settled) extrinsic.
//! 18) **Migration Start**: Once the issuer has tokens to distribute on mainnet, he can start the migration process with the [`start_offchain`](Pallet::start_offchain_migration) extrinsic.
//! 19) **Confirm Migration**: The issuer has to mark each participant's CTs as migrated with the [`confirm_offchain_migration`](Pallet::confirm_offchain_migration) extrinsic.
//! 20) **Migration End**: Once all participants have migrated their CTs, anyone can mark the migration as finished with the [`mark_project_ct_migration_as_finished`](Pallet::mark_project_ct_migration_as_finished) extrinsic.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// Needed due to empty sections raising the warning
#![allow(unreachable_patterns)]
// This recursion limit is needed because we have too many benchmarks and benchmarking will fail if
// we add more without this limit.
#![cfg_attr(feature = "runtime-benchmarks", recursion_limit = "512")]
extern crate alloc;

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
	credentials::{Cid, Did, EnsureOriginWithCredentials, InvestorType, UntrustedToken},
	migration_types::{Migration, MigrationStatus, ParticipationType},
};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_arithmetic::traits::{One, Saturating};
use sp_runtime::{traits::AccountIdConversion, FixedPointNumber, FixedPointOperand, FixedU128};
use sp_std::{marker::PhantomData, prelude::*};
pub use types::*;
use xcm::v4::{prelude::*, SendXcm};

mod functions;
pub mod storage_migrations;
pub mod traits;
pub mod types;
pub mod weights;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

#[cfg(not(feature = "on-chain-release-build"))]
pub mod instantiator;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod runtime_api;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type ProjectId = u32;
pub type MultiplierOf<T> = <T as Config>::Multiplier;

pub type BalanceOf<T> = <T as Config>::Balance;
pub type PriceOf<T> = <T as Config>::Price;
pub type PriceProviderOf<T> = <T as Config>::PriceProvider;
pub type StringLimitOf<T> = <T as Config>::StringLimit;
pub type AssetIdOf<T> =
	<<T as Config>::FundingCurrency as fungibles::Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
pub type RewardInfoOf<T> = RewardInfo<BalanceOf<T>>;
pub type EvaluatorsOutcomeOf<T> = EvaluatorsOutcome<BalanceOf<T>>;
pub type VestingInfoOf<T> = VestingInfo<BlockNumberFor<T>, BalanceOf<T>>;

pub type TicketSizeOf<T> = TicketSize<BalanceOf<T>>;
pub type ProjectMetadataOf<T> =
	ProjectMetadata<BoundedVec<u8, StringLimitOf<T>>, BalanceOf<T>, PriceOf<T>, AccountIdOf<T>, Cid>;
pub type ProjectDetailsOf<T> =
	ProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;
pub type EvaluationRoundInfoOf<T> = EvaluationRoundInfo<BalanceOf<T>>;
pub type EvaluationInfoOf<T> = EvaluationInfo<u32, Did, ProjectId, AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>;
pub type BidInfoOf<T> =
	BidInfo<ProjectId, Did, BalanceOf<T>, PriceOf<T>, AccountIdOf<T>, BlockNumberFor<T>, MultiplierOf<T>>;

pub type ContributionInfoOf<T> =
	ContributionInfo<u32, Did, ProjectId, AccountIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, MultiplierOf<T>>;

pub type BucketOf<T> = Bucket<BalanceOf<T>, PriceOf<T>>;
pub type WeightInfoOf<T> = <T as Config>::WeightInfo;

pub const PLMC_FOREIGN_ID: u32 = 3344;
pub const PLMC_DECIMALS: u8 = 10;

#[frame_support::pallet]
pub mod pallet {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use crate::traits::{BondingRequirementCalculation, ProvideAssetPrice, VestingDurationCalculation};
	use core::ops::RangeInclusive;
	use frame_support::{
		pallet_prelude::*,
		storage::KeyPrefixIterator,
		traits::{OnFinalize, OnIdle, OnInitialize},
	};
	use frame_system::pallet_prelude::*;
	use sp_arithmetic::Percent;
	use sp_runtime::{
		traits::{Convert, ConvertBack, Get},
		Perquintill,
	};

	#[pallet::composite_enum]
	pub enum HoldReason {
		Evaluation,
		Participation,
	}

	#[pallet::pallet]
	#[pallet::storage_version(storage_migrations::STORAGE_VERSION)]
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

		/// The inner balance type we will use for all of our outer currency types. (e.g native, funding, CTs)
		type Balance: Balance + From<u64> + FixedPointOperand + MaybeSerializeDeserialize + Into<u128>;

		// TODO: our local BlockNumber should be removed once we move onto using Moment for time tracking
		/// BlockNumber used for PLMC vesting durations on this chain, and CT vesting durations on funded chains.
		type BlockNumber: IsType<BlockNumberFor<Self>> + Into<u64>;

		/// The length (expressed in number of blocks) of the Auction Round, Closing period.
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		/// The length (expressed in number of blocks) of the Community Round.
		#[pallet::constant]
		type CommunityRoundDuration: Get<BlockNumberFor<Self>>;

		/// The currency used for minting contribution tokens as fungible assets (i.e pallet-assets)
		type ContributionTokenCurrency: fungibles::Create<AccountIdOf<Self>, AssetId = ProjectId, Balance = BalanceOf<Self>>
			+ fungibles::Destroy<AccountIdOf<Self>, AssetId = ProjectId, Balance = BalanceOf<Self>>
			+ fungibles::InspectEnumerable<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				AssetsIterator = KeyPrefixIterator<AssetIdOf<Self>>,
			> + fungibles::metadata::Inspect<AccountIdOf<Self>>
			+ fungibles::metadata::Mutate<AccountIdOf<Self>>
			+ fungibles::metadata::MetadataDeposit<BalanceOf<Self>>
			+ fungibles::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungibles::roles::Inspect<AccountIdOf<Self>>
			+ AccountTouch<ProjectId, AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ ContainsPair<ProjectId, AccountIdOf<Self>>;

		/// Convert 24 hours as FixedU128, to the corresponding amount of blocks in the same type as frame_system
		type DaysToBlocks: Convert<FixedU128, BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the Auction Round.
		#[pallet::constant]
		type AuctionRoundDuration: Get<BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the evaluation period.
		#[pallet::constant]
		type EvaluationRoundDuration: Get<BlockNumberFor<Self>>;

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

		type FundingSuccessThreshold: Get<Perquintill>;

		/// Credentialized investor Origin, ensures users are of investing type Retail, or Professional, or Institutional.
		type InvestorOrigin: EnsureOriginWithCredentials<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = (AccountIdOf<Self>, Did, InvestorType, Cid),
		>;

		/// Max individual bids per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxBidsPerProject: Get<u32>;

		/// Max individual bids per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxBidsPerUser: Get<u32>;

		/// Range of max_capacity_thresholds values for the hrmp config where we accept the incoming channel request
		#[pallet::constant]
		type MaxCapacityThresholds: Get<RangeInclusive<u32>>;

		/// Max individual contributions per project per user. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxContributionsPerUser: Get<u32>;

		/// Max individual evaluations per project. Used to estimate worst case weight for price calculation
		#[pallet::constant]
		type MaxEvaluationsPerProject: Get<u32>;

		/// How many distinct evaluations per user per project
		#[pallet::constant]
		type MaxEvaluationsPerUser: Get<u32>;

		#[pallet::constant]
		type MinUsdPerEvaluation: Get<BalanceOf<Self>>;

		/// RangeInclusive of max_message_size values for the hrmp config where we accept the incoming channel request
		#[pallet::constant]
		type MaxMessageSizeThresholds: Get<RangeInclusive<u32>>;

		/// Multiplier type that decides how much PLMC needs to be bonded for a token buy/bid
		type Multiplier: Parameter
			+ BondingRequirementCalculation
			+ VestingDurationCalculation
			+ Default
			+ Copy
			+ TryFrom<u8>
			+ Into<u8>
			+ MaxEncodedLen
			+ MaybeSerializeDeserialize;

		/// The chains native currency
		type NativeCurrency: fungible::InspectHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::MutateHold<
				AccountIdOf<Self>,
				Balance = BalanceOf<Self>,
				Reason = <Self as Config>::RuntimeHoldReason,
			> + fungible::BalancedHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ fungible::Inspect<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// System account for the funding pallet. Used to derive project escrow accounts.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Type that represents the value of something in USD
		type Price: FixedPointNumber + Parameter + Copy + MaxEncodedLen + MaybeSerializeDeserialize;

		/// Method to get the price of an asset like USDT or PLMC. Likely to come from an oracle
		type PriceProvider: ProvideAssetPrice<AssetId = u32, Price = Self::Price>;

		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

		/// The length (expressed in number of blocks) of the Remainder Round.
		#[pallet::constant]
		type RemainderRoundDuration: Get<BlockNumberFor<Self>>;

		/// max_capacity config required for the channel from polimec to the project
		#[pallet::constant]
		type RequiredMaxCapacity: Get<u32>;

		/// max_message_size config required for the channel from polimec to the project
		#[pallet::constant]
		type RequiredMaxMessageSize: Get<u32>;

		/// The runtime enum constructed by the construct_runtime macro
		type RuntimeCall: Parameter + IsType<<Self as pallet_xcm::Config>::RuntimeCall> + From<Call<Self>>;

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
		#[cfg(feature = "runtime-benchmarks")]
		type SetPrices: traits::SetPrices;

		/// The maximum length of data stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		/// Account that receive the PLMC slashed from failed evaluations.
		#[pallet::constant]
		type BlockchainOperationTreasury: Get<AccountIdOf<Self>>;

		/// Treasury account holding the CT fees charged to issuers.
		#[pallet::constant]
		type ContributionTreasury: Get<AccountIdOf<Self>>;

		/// The Ed25519 Verifier Public Key of credential JWTs
		#[pallet::constant]
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
	/// A StorageMap containing the primary project information of projects
	pub type ProjectsMetadata<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, ProjectMetadataOf<T>>;

	#[pallet::storage]
	/// A StorageMap containing the primary project information of projects
	pub type Buckets<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, BucketOf<T>>;

	#[pallet::storage]
	/// StorageMap containing additional information for the projects, relevant for correctness of the protocol
	pub type ProjectsDetails<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, ProjectDetailsOf<T>>;

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
	pub type UserMigrations<T: Config> = StorageNMap<
		_,
		(NMapKey<Blake2_128Concat, ProjectId>, NMapKey<Blake2_128Concat, AccountIdOf<T>>),
		(MigrationStatus, BoundedVec<Migration, MaxParticipationsPerUser<T>>),
	>;

	/// Counts how many participants have not yet migrated their CTs. Counter goes up on each settlement, and goes
	/// down on each migration. Saves us a whole read over the full migration storage for transitioning to `ProjectStatus::CTMigrationFinished`
	#[pallet::storage]
	pub type UnmigratedCounter<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, u32, ValueQuery>;

	pub struct MaxParticipationsPerUser<T: Config>(PhantomData<T>);
	impl<T: Config> Get<u32> for MaxParticipationsPerUser<T> {
		fn get() -> u32 {
			T::MaxContributionsPerUser::get() + T::MaxBidsPerUser::get() + T::MaxEvaluationsPerUser::get()
		}
	}

	#[pallet::storage]
	pub type ActiveMigrationQueue<T: Config> = StorageMap<_, Blake2_128Concat, QueryId, (ProjectId, T::AccountId)>;

	/// A map to keep track of what issuer's did has an active project. It prevents one issuer having multiple active projects
	#[pallet::storage]
	pub type DidWithActiveProjects<T: Config> = StorageMap<_, Blake2_128Concat, Did, ProjectId, OptionQuery>;

	#[pallet::storage]
	pub type DidWithWinningBids<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, ProjectId, Blake2_128Concat, Did, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A project was created.
		ProjectCreated {
			project_id: ProjectId,
			issuer: T::AccountId,
			metadata: ProjectMetadataOf<T>,
		},
		/// An issuer removed the project before the evaluation started
		ProjectRemoved {
			project_id: ProjectId,
			issuer: T::AccountId,
		},
		/// The metadata of a project was modified.
		MetadataEdited {
			project_id: ProjectId,
			metadata: ProjectMetadataOf<T>,
		},
		/// Project transitioned to a new phase.
		ProjectPhaseTransition {
			project_id: ProjectId,
			phase: ProjectStatus<BlockNumberFor<T>>,
		},
		/// A `bonder` bonded an `amount` of PLMC for `project_id`.
		Evaluation {
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			id: u32,
			plmc_amount: BalanceOf<T>,
		},
		/// A bid was made for a project
		Bid {
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			id: u32,
			ct_amount: BalanceOf<T>,
			ct_price: T::Price,
			funding_asset: AcceptedFundingAsset,
			funding_amount: BalanceOf<T>,
			plmc_bond: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
		},
		/// A contribution was made for a project. i.e token purchase
		Contribution {
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			id: u32,
			ct_amount: BalanceOf<T>,
			funding_asset: AcceptedFundingAsset,
			funding_amount: BalanceOf<T>,
			plmc_bond: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
		},
		BidRefunded {
			project_id: ProjectId,
			account: AccountIdOf<T>,
			bid_id: u32,
			plmc_amount: BalanceOf<T>,
			funding_asset: AcceptedFundingAsset,
			funding_amount: BalanceOf<T>,
		},
		EvaluationSettled {
			project_id: ProjectId,
			account: AccountIdOf<T>,
			id: u32,
			ct_rewarded: BalanceOf<T>,
			plmc_released: BalanceOf<T>,
		},
		BidSettled {
			project_id: ProjectId,
			account: AccountIdOf<T>,
			id: u32,
			final_ct_amount: BalanceOf<T>,
			final_ct_usd_price: PriceOf<T>,
		},
		ContributionSettled {
			project_id: ProjectId,
			account: AccountIdOf<T>,
			id: u32,
			ct_amount: BalanceOf<T>,
		},
		PalletMigrationStarted {
			project_id: ProjectId,
			para_id: ParaId,
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
		MigrationStatusUpdated {
			project_id: ProjectId,
			account: AccountIdOf<T>,
			status: MigrationStatus,
		},

		CTMigrationFinished {
			project_id: ProjectId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Something in storage has a state which should never be possible at this point. Programming error.
		ImpossibleState,
		/// Action is not allowed.
		NotAllowed,
		/// Checked math failed.
		BadMath,
		/// Could not get the price in USD equivalent for an asset/PLMC.
		PriceNotFound,
		/// Tried to retrieve a evaluation, bid or contribution but it does not exist.
		ParticipationNotFound,
		/// The user investor type is not eligible for the action.
		WrongInvestorType,

		// * Project Error. Project information not found, or project has an incorrect state. *
		/// The project details were not found. Happens when the project with provided ID does
		/// not exist in the `ProjectsDetails` storage.
		ProjectDetailsNotFound,
		/// The project metadata was not found. Happens when the project with provided ID does
		/// not exist in the `ProjectsMetadata` storage.
		ProjectMetadataNotFound,
		/// The project's bucket info was not found. Happens when the project with provided ID does
		/// not exist in the `Buckets` storage.
		BucketNotFound,
		/// The project is already frozen, so cannot be frozen again. Happens when
		/// `do_start_evaluation` is called on a project that has already started the
		/// evaluation round.
		ProjectAlreadyFrozen,
		/// The project is frozen, so no changes to the metadata are allowed and the project
		/// cannot be deleted anymore.
		ProjectIsFrozen,
		/// The project's weighted average price is not set while in the community round.
		/// Should not happen in practice.
		WapNotSet,

		// * A round related error. The project did not have the correct state to execute the action. *
		/// The project is not in the correct round to execute the action.
		IncorrectRound,
		/// Too early to execute the action. The action can likely be called again at a later stage.
		TooEarlyForRound,
		/// Too late to execute the action. Round has already ended, but transition to new
		/// round has still to be executed.
		TooLateForRound,
		/// A project's transition point (block number) was not set.
		TransitionPointNotSet,

		// * Issuer related errors. E.g. the action was not executed by the issuer, or the issuer *
		/// did not have the correct state to execute an action.
		/// The action's caller is not the issuer of the project and is not allowed to execute
		/// this action.
		NotIssuer,
		/// The issuer already has an active project. The issuer can only have one active project.
		HasActiveProject,
		/// The issuer tries to participate to their own project.
		ParticipationToOwnProject,
		/// The issuer has not enough funds to cover the escrow account costs.
		IssuerNotEnoughFunds,

		// * The project's metadata is incorrect. *
		/// The minimum price per token is too low.
		PriceTooLow,
		/// The ticket sizes are not valid.
		TicketSizeError,
		/// The participation currencies are not unique.
		ParticipationCurrenciesError,
		/// The allocation size is invalid. Either zero or higher than the max supply.
		AllocationSizeError,
		/// The auction round percentage cannot be zero.
		AuctionRoundPercentageError,
		/// The funding target has to be higher than 1000 USD.
		FundingTargetTooLow,
		/// The funding target has to be lower than 1bn USD.
		FundingTargetTooHigh,
		/// The project's metadata hash is not provided while starting the evaluation round.
		CidNotProvided,
		/// The ct decimals specified for the CT is outside the 4 to 20 range.
		BadDecimals,
		// The combination of decimals and price of this project is not representable within our 6 decimals USD system,
		// and integer space of 128 bits.
		BadTokenomics,

		// * Error related to an participation action. Evaluation, bid or contribution failed. *
		/// The amount is too low.
		TooLow,
		/// The amount is too high.
		TooHigh,
		/// The participation currency is not accepted for this project.
		FundingAssetNotAccepted,
		/// The user already has the maximum number of participations in this project.
		TooManyUserParticipations,
		/// The project already has the maximum number of participations.
		TooManyProjectParticipations,
		/// The user is not allowed to use the selected multiplier.
		ForbiddenMultiplier,
		/// The user has a winning bid in the auction round and is not allowed to participate
		/// in the community round.
		UserHasWinningBid,
		/// The funds in the wallet are too low to cover the participation.
		ParticipantNotEnoughFunds,
		/// The JWT included the wrong policy for participating in this project.
		PolicyMismatch,
		/// Contribution tokens have all been sold
		ProjectSoldOut,

		//  * An error related to the migration process. *
		/// Tried to start a migration check but the bidirectional channel is not yet open
		ChannelNotOpen,
		/// The xcm execution/sending failed.
		XcmFailed,
		/// Reached limit on maximum number of migrations. In practise this should not happen,
		/// as the max migrations is set to the sum of max evaluations, bids and contributions.
		TooManyMigrations,
		/// User has no migrations to execute.
		NoMigrationsFound,
		/// User has no active migrations in the queue.
		NoActiveMigrationsFound,
		/// Wrong para_id is provided.
		WrongParaId,
		/// Migration channel is not ready for migrations.
		ChannelNotReady,
		/// Settlement for this project has not yet started.
		SettlementNotStarted,
		/// Wanted to settle as successful when it failed, or vice versa.
		WrongSettlementOutcome,
		/// User still has participations that need to be settled before migration.
		ParticipationsNotSettled,
		/// Tried to mark project as fully settled but there are participations that are not settled.
		SettlementNotComplete,
		/// Tried to mark a project's CT migration as finished but there are still migrations to be confirmed
		MigrationsStillPending,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a project and assigns it to the `issuer` account.
		#[pallet::call_index(0)]
		#[pallet::weight(WeightInfoOf::<T>::create_project())]
		pub fn create_project(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project: ProjectMetadataOf<T>,
		) -> DispatchResult {
			let (account, did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);
			Self::do_create_project(&account, project, did)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(WeightInfoOf::<T>::remove_project())]
		pub fn remove_project(origin: OriginFor<T>, jwt: UntrustedToken, project_id: ProjectId) -> DispatchResult {
			let (account, did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);
			Self::do_remove_project(account, project_id, did)
		}

		/// Change the metadata hash of a project
		#[pallet::call_index(2)]
		#[pallet::weight(WeightInfoOf::<T>::edit_project())]
		pub fn edit_project(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			new_project_metadata: ProjectMetadataOf<T>,
		) -> DispatchResult {
			let (account, _did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);
			Self::do_edit_project(account, project_id, new_project_metadata)
		}

		/// Starts the evaluation round of a project. It needs to be called by the project issuer.
		#[pallet::call_index(3)]
		#[pallet::weight(WeightInfoOf::<T>::start_evaluation())]
		pub fn start_evaluation(origin: OriginFor<T>, jwt: UntrustedToken, project_id: ProjectId) -> DispatchResult {
			let (account, _did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);
			Self::do_start_evaluation(account, project_id)
		}

		/// Bond PLMC for a project in the evaluation stage
		#[pallet::call_index(4)]
		#[pallet::weight(WeightInfoOf::<T>::evaluate(<T as Config>::MaxEvaluationsPerUser::get()))]
		pub fn evaluate(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] usd_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let (account, did, _investor_type, whitelisted_policy) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;

			Self::do_evaluate(&account, project_id, usd_amount, did, whitelisted_policy)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(WeightInfoOf::<T>::end_evaluation_failure())]
		pub fn end_evaluation(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_end_evaluation(project_id)
		}

		/// Bid for a project in the Auction round
		#[pallet::call_index(7)]
		#[pallet::weight(
			WeightInfoOf::<T>::bid(
				<T as Config>::MaxBidsPerUser::get(),
				// Assuming the current bucket is full, and has a price higher than the minimum.
				// This user is buying 100% of the bid allocation.
				// Since each bucket has 10% of the allocation, one bid can be split into a max of 10
				10
		))]
		pub fn bid(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] ct_amount: BalanceOf<T>,
			multiplier: T::Multiplier,
			funding_asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let (bidder, did, investor_type, whitelisted_policy) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			let params = DoBidParams::<T> {
				bidder,
				project_id,
				ct_amount,
				multiplier,
				funding_asset,
				did,
				investor_type,
				whitelisted_policy,
			};
			Self::do_bid(params)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(WeightInfoOf::<T>::end_auction(
			<T as Config>::MaxBidsPerProject::get() / 2,
			<T as Config>::MaxBidsPerProject::get() / 2,
		)
		.max(WeightInfoOf::<T>::end_auction(
			<T as Config>::MaxBidsPerProject::get(),
			0u32,
		))
		.max(WeightInfoOf::<T>::end_auction(
			0u32,
			<T as Config>::MaxBidsPerProject::get(),
		)))]
		pub fn end_auction(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			Self::do_end_auction(project_id)
		}

		/// Buy tokens in the Community or Remainder round at the price set in the Auction Round
		#[pallet::call_index(9)]
		#[pallet::weight(
			WeightInfoOf::<T>::contribute(T::MaxContributionsPerUser::get())
		)]
		pub fn contribute(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			#[pallet::compact] ct_amount: BalanceOf<T>,
			multiplier: MultiplierOf<T>,
			funding_asset: AcceptedFundingAsset,
		) -> DispatchResultWithPostInfo {
			let (contributor, did, investor_type, whitelisted_policy) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			let params = DoContributeParams::<T> {
				contributor,
				project_id,
				ct_amount,
				multiplier,
				funding_asset,
				did,
				investor_type,
				whitelisted_policy,
			};
			Self::do_contribute(params)
		}

		#[pallet::call_index(10)]
		#[pallet::weight(WeightInfoOf::<T>::end_funding_project_successful())]
		pub fn end_funding(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_end_funding(project_id)
		}

		#[pallet::call_index(11)]
		#[pallet::weight(WeightInfoOf::<T>::start_settlement())]
		pub fn start_settlement(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_start_settlement(project_id)
		}

		#[pallet::call_index(12)]
		#[pallet::weight(WeightInfoOf::<T>::settle_rewarded_evaluation())]
		pub fn settle_evaluation(
			origin: OriginFor<T>,
			project_id: ProjectId,
			evaluator: AccountIdOf<T>,
			evaluation_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Evaluations::<T>::get((project_id, evaluator, evaluation_id))
				.ok_or(Error::<T>::ParticipationNotFound)?;
			Self::do_settle_evaluation(bid, project_id)
		}

		#[pallet::call_index(13)]
		#[pallet::weight(WeightInfoOf::<T>::settle_accepted_bid_with_refund())]
		pub fn settle_bid(
			origin: OriginFor<T>,
			project_id: ProjectId,
			bidder: AccountIdOf<T>,
			bid_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::ParticipationNotFound)?;
			Self::do_settle_bid(bid, project_id)
		}

		#[pallet::call_index(17)]
		#[pallet::weight(WeightInfoOf::<T>::settle_contribution_project_successful())]
		pub fn settle_contribution(
			origin: OriginFor<T>,
			project_id: ProjectId,
			contributor: AccountIdOf<T>,
			contribution_id: u32,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let bid = Contributions::<T>::get((project_id, contributor, contribution_id))
				.ok_or(Error::<T>::ParticipationNotFound)?;
			Self::do_settle_contribution(bid, project_id)
		}

		#[pallet::call_index(18)]
		#[pallet::weight(WeightInfoOf::<T>::mark_project_as_settled())]
		pub fn mark_project_as_settled(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			Self::do_mark_project_as_settled(project_id)
		}

		#[pallet::call_index(19)]
		#[pallet::weight(WeightInfoOf::<T>::start_offchain_migration())]
		pub fn start_offchain_migration(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
		) -> DispatchResult {
			let (account, _did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);

			Self::do_start_offchain_migration(project_id, account)
		}

		#[pallet::call_index(20)]
		#[pallet::weight(WeightInfoOf::<T>::confirm_offchain_migration(MaxParticipationsPerUser::<T>::get()))]
		pub fn confirm_offchain_migration(
			origin: OriginFor<T>,
			project_id: ProjectId,
			participant: AccountIdOf<T>,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Self::do_confirm_offchain_migration(project_id, caller, participant)
		}

		#[pallet::call_index(21)]
		#[pallet::weight(WeightInfoOf::<T>::start_pallet_migration())]
		pub fn start_pallet_migration(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
			para_id: ParaId,
		) -> DispatchResult {
			let (account, _did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);

			Self::do_start_pallet_migration(&account, project_id, para_id)
		}

		#[pallet::call_index(22)]
		#[pallet::weight(WeightInfoOf::<T>::start_pallet_migration_readiness_check())]
		pub fn start_pallet_migration_readiness_check(
			origin: OriginFor<T>,
			jwt: UntrustedToken,
			project_id: ProjectId,
		) -> DispatchResult {
			let (account, _did, investor_type, _cid) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(investor_type == InvestorType::Institutional, Error::<T>::WrongInvestorType);
			Self::do_start_pallet_migration_readiness_check(&account, project_id)
		}

		/// Called only by other chains through a query response xcm message
		#[pallet::call_index(23)]
		#[pallet::weight(WeightInfoOf::<T>::pallet_migration_readiness_response_pallet_info()
		.max(WeightInfoOf::<T>::pallet_migration_readiness_response_holding()))]
		pub fn pallet_migration_readiness_response(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_pallet_migration_readiness_response(location, query_id, response)
		}

		#[pallet::call_index(24)]
		#[pallet::weight(WeightInfoOf::<T>::send_pallet_migration_for(MaxParticipationsPerUser::<T>::get()))]
		pub fn send_pallet_migration_for(
			origin: OriginFor<T>,
			project_id: ProjectId,
			participant: AccountIdOf<T>,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			Self::do_send_pallet_migration_for(project_id, participant)
		}

		#[pallet::call_index(25)]
		#[pallet::weight(WeightInfoOf::<T>::confirm_pallet_migrations(MaxParticipationsPerUser::<T>::get()))]
		pub fn confirm_pallet_migrations(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let location = ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;

			Self::do_confirm_pallet_migrations(location, query_id, response)
		}

		#[pallet::call_index(26)]
		#[pallet::weight(WeightInfoOf::<T>::mark_project_ct_migration_as_finished())]
		pub fn mark_project_ct_migration_as_finished(origin: OriginFor<T>, project_id: ProjectId) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			Self::do_mark_project_ct_migration_as_finished(project_id)
		}
	}
}

pub mod xcm_executor_impl {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use xcm_executor::traits::{HandleHrmpChannelAccepted, HandleHrmpNewChannelOpenRequest};

	impl<T: Config> HandleHrmpChannelAccepted for Pallet<T> {
		fn handle(recipient: u32) -> XcmResult {
			<Pallet<T>>::do_handle_channel_accepted(recipient)
		}
	}

	impl<T: Config> HandleHrmpNewChannelOpenRequest for Pallet<T> {
		fn handle(sender: u32, max_message_size: u32, max_capacity: u32) -> XcmResult {
			<Pallet<T>>::do_handle_channel_open_request(sender, max_message_size, max_capacity)
		}
	}
}
