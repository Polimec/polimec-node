#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod weights;
pub mod xcm_config;

use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
use frame_support::{
	construct_runtime,
	dispatch::DispatchClass,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU32, ConstU64, ConstU8, EitherOfDiverse, Everything},
	weights::{ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot, EnsureSigned,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedU128,
};
pub use sp_runtime::{MultiAddress, Perbill, Permill};
use sp_std::collections::btree_map::BTreeMap;
use xcm_config::{RelayLocation, XcmConfig, XcmOriginToTransactDispatchOrigin};

// Polkadot imports
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};

use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};

// XCM Imports
use xcm::latest::prelude::BodyId;
use xcm_executor::XcmExecutor;

use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub use parachains_common::Signature;

/// Balance of an account.
pub use parachains_common::Balance;

/// Index of a transaction in the chain.
pub use parachains_common::Index;

/// A hash of some data used by the chain.
pub use parachains_common::Hash;

pub use parachains_common::AccountId as PolimecAccountId;

/// An index to a block.
pub use parachains_common::BlockNumber;

/// The address format for describing accounts.
pub type Address = MultiAddress<PolimecAccountId, ()>;

/// Block header type as expected by this runtime.
// pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub use parachains_common::Header;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
// pub use parachains_common::Block;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<PolimecAccountId, RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLIUNIT:
		// in our template, we map to 1/10 of that, or 1/10 MILLIUNIT
		let p = MILLI_PLMC / 10;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub use parachains_common::opaque;

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("template-parachain"),
	impl_name: create_runtime_str!("template-parachain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub use parachains_common::MILLISECS_PER_BLOCK;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub use parachains_common::SLOT_DURATION;

// Time is measured by number of blocks.
pub use parachains_common::DAYS;
pub use parachains_common::HOURS;
pub use parachains_common::MINUTES;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
pub use parachains_common::AVERAGE_ON_INITIALIZE_RATIO;

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
pub use parachains_common::NORMAL_DISPATCH_RATIO;

/// We allow for 0.5 of a second of compute with a 12 second average block time.
pub use parachains_common::MAXIMUM_BLOCK_WEIGHT;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 41;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = PolimecAccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<PolimecAccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = Everything;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = ();
	type HoldIdentifier = BondTypeOf<Runtime>;
	type FreezeIdentifier = ();
	type MaxHolds = MaxLocks;
	type MaxFreezes = MaxReserves;
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICRO_PLMC;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	type ExecuteOverweightOrigin = EnsureRoot<PolimecAccountId>;
	type ControllerOrigin = EnsureRoot<PolimecAccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
	type PriceForSiblingDelivery = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<PolimecAccountId>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MaxCandidates: u32 = 1000;
	pub const MinCandidates: u32 = 5;
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const MaxInvulnerables: u32 = 100;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin =
	EitherOfDiverse<EnsureRoot<PolimecAccountId>, EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = MaxCandidates;
	type MinCandidates = MinCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

/// One PLMC
pub const PLMC: Balance = 10u128.pow(10);

/// 0.001 PLMC
pub const MILLI_PLMC: Balance = 10u128.pow(7);

/// 0.000_001 PLMC
pub const MICRO_PLMC: Balance = 10u128.pow(4);

pub const EXISTENTIAL_DEPOSIT: Balance = 10 * MILLI_PLMC;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * MICRO_PLMC + (bytes as Balance) * 6 * MICRO_PLMC
}

pub const fn free_deposit() -> Balance {
	0
}

parameter_types! {
	pub const AssetDeposit: Balance = PLMC; // 1 UNIT deposit to create asset
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = free_deposit();
	pub const MetadataDepositPerByte: Balance = free_deposit();
	pub const AssetsPalletId: PalletId = PalletId(*b"assetsid");
}

pub type LocalAssetsInstance = pallet_assets::Instance1;
pub type StatemintAssetsInstance = pallet_assets::Instance2;

use pallet_funding::BondTypeOf;
pub use parachains_common::AssetIdForTrustBackedAssets as AssetId;

impl pallet_assets::Config<LocalAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<PolimecAccountId>>;
	type ForceOrigin = EnsureRoot<PolimecAccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_assets::Config<StatemintAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<PolimecAccountId>>;
	type ForceOrigin = EnsureRoot<PolimecAccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

#[cfg(feature = "fast-gov")]
pub const EVALUATION_DURATION: BlockNumber = 28;
#[cfg(not(feature = "fast-gov"))]
pub const EVALUATION_DURATION: BlockNumber = 28 * DAYS;

#[cfg(feature = "fast-gov")]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 7;
#[cfg(not(feature = "fast-gov"))]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 2 * DAYS;

#[cfg(feature = "fast-gov")]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 5;
#[cfg(not(feature = "fast-gov"))]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 3 * DAYS;

#[cfg(feature = "fast-gov")]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 5 * DAYS;

#[cfg(feature = "fast-gov")]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = 1 * DAYS;

#[cfg(feature = "fast-gov")]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 365;
#[cfg(not(feature = "fast-gov"))]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 365 * DAYS;

parameter_types! {
	pub const EvaluationDuration: BlockNumber = EVALUATION_DURATION;
	pub const AuctionInitializePeriodDuration: BlockNumber = AUCTION_INITIALIZE_PERIOD_DURATION;
	pub const EnglishAuctionDuration: BlockNumber = ENGLISH_AUCTION_DURATION;
	pub const CandleAuctionDuration: BlockNumber = CANDLE_AUCTION_DURATION;
	pub const CommunityFundingDuration: BlockNumber = COMMUNITY_FUNDING_DURATION;
	pub const RemainderFundingDuration: BlockNumber = REMAINDER_FUNDING_DURATION;
	pub const ContributionVestingDuration: BlockNumber = CONTRIBUTION_VESTING_DURATION;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
	pub PriceMap: BTreeMap<AssetId, FixedU128> = BTreeMap::from_iter(vec![
		(0u32, FixedU128::from_rational(69, 1)), // DOT
		(420u32, FixedU128::from_rational(97, 100)), // USDC
		(1984u32, FixedU128::from_rational(95, 100)), // USDT
		(2069u32, FixedU128::from_rational(840, 100)), // PLMC
	]);
}

impl pallet_funding::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ProjectIdentifier = u32;
	type Multiplier = pallet_funding::types::Multiplier<Self>;
	type Balance = Balance;
	type Price = FixedU128;
	type NativeCurrency = Balances;
	type FundingCurrency = StatemintAssets;
	type ContributionTokenCurrency = LocalAssets;
	type PriceProvider = pallet_funding::types::ConstPriceProvider<AssetId, FixedU128, PriceMap>;
	type StorageItemId = u128;
	type Randomness = Random;
	type HandleMembers = Credentials;
	type StringLimit = ConstU32<64>;
	type PreImageLimit = ConstU32<1024>;
	type EvaluationDuration = EvaluationDuration;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type CandleAuctionDuration = CandleAuctionDuration;
	type CommunityFundingDuration = CommunityFundingDuration;
	type RemainderFundingDuration = RemainderFundingDuration;
	type PalletId = FundingPalletId;
	type MaxProjectsToUpdatePerBlock = ConstU32<100>;
	type MaxEvaluationsPerUser = ();
	type MaxBidsPerUser = ConstU32<256>;

	type MaxContributionsPerUser = ConstU32<256>;
	type ContributionVesting = ContributionVestingDuration;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type WeightInfo = ();
}

impl pallet_credentials::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRoot<PolimecAccountId>;
	type RemoveOrigin = EnsureRoot<PolimecAccountId>;
	type SwapOrigin = EnsureRoot<PolimecAccountId>;
	type ResetOrigin = EnsureRoot<PolimecAccountId>;
	type PrimeOrigin = EnsureRoot<PolimecAccountId>;
	type MembershipInitialized = ();
	type MembershipChanged = ();
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		Timestamp: pallet_timestamp = 2,
		ParachainInfo: parachain_info = 3,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,
		LocalAssets: pallet_assets::<Instance1>::{Pallet, Call, Storage, Event<T>} = 13,
		StatemintAssets: pallet_assets::<Instance2>::{Pallet, Call, Storage, Event<T>} = 14,

		// Collator support. The order of these 4 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		DmpQueue: cumulus_pallet_dmp_queue = 33,

		// Polimec
		Random: pallet_insecure_randomness_collective_flip = 40,
		Funding: pallet_funding = 41,
		Credentials: pallet_credentials = 42,
	}
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_session, SessionBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_collator_selection, CollatorSelection]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
	);
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, PolimecAccountId, Index> for Runtime {
		fn account_nonce(account: PolimecAccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
			impl cumulus_pallet_session_benchmarking::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block, relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data = cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
			relay_chain_slot,
			sp_std::time::Duration::from_secs(6),
		)
		.create_inherent_data()
		.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}