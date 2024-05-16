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

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	construct_runtime,
	genesis_builder_helper::{build_config, create_default_config},
	ord_parameter_types, parameter_types,
	traits::{
		fungible::{Credit, HoldConsideration, Inspect},
		tokens::{self, PayFromAccount, UnityAssetBalanceConversion},
		AsEnsureOriginWithArg, ConstU32, Contains, EitherOfDiverse, InstanceFilter, LinearStoragePrice, PrivilegeCmp,
		TransformOrigin,
	},
	weights::{ConstantMultiplier, Weight},
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSigned, EnsureSignedBy};
use pallet_democracy::GetElectorate;
use pallet_funding::DaysToBlocks;

use parachains_common::{
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AssetIdForTrustBackedAssets as AssetId,
};
use parity_scale_codec::Encode;
use polimec_common::credentials::EnsureInvestor;
use polkadot_runtime_common::{
	xcm_sender::NoPriceForMessageDelivery, BlockHashCount, CurrencyToVote, SlowAdjustingFeeUpdate,
};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, Convert, ConvertBack, ConvertInto,
		IdentifyAccount, IdentityLookup, OpaqueKeys, Verify,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature, SaturatedConversion,
};
use sp_std::{cmp::Ordering, prelude::*};
use sp_version::RuntimeVersion;

// XCM Imports
use xcm_config::XcmOriginToTransactDispatchOrigin;

#[cfg(not(feature = "runtime-benchmarks"))]
use xcm_config::XcmConfig;

pub use pallet_parachain_staking;
// Polimec Shared Imports
pub use shared_configuration::{
	assets::*, currency::*, fee::*, funding::*, governance::*, identity::*, proxy::*, staking::*, time::*, weights::*,
};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, Perbill, Permill};

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
use sp_version::NativeVersion;

use polimec_common::USD_UNIT;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub mod benchmarks;

mod custom_migrations;
mod weights;
pub mod xcm_config;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type CreditOf<T> = Credit<<T as frame_system::Config>::AccountId, pallet_balances::Pallet<T, ()>>;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

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
	// TODO: Return to parity CheckNonce implementation once
	// https://github.com/paritytech/polkadot-sdk/issues/3991 is resolved.
	pallet_dispenser::extensions::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	// TODO: Use parity's implementation once
	// https://github.com/paritytech/polkadot-sdk/pull/3993 is available.
	pallet_dispenser::extensions::SkipCheckIfFeeless<
		Runtime,
		pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
	>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;

pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
#[allow(missing_docs)]
pub mod migrations {
	use crate::{custom_migrations::init_pallet::InitializePallet, DmpQueue};
	/// Unreleased migrations. Add new ones here:
	#[allow(unused_parens)]
	pub type Unreleased = (InitializePallet<DmpQueue>);
}

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;
	use sp_runtime::{
		generic,
		traits::{BlakeTwo256, Hash as HashT},
	};

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;
	/// Opaque block hash type.
	pub type Hash = <BlakeTwo256 as HashT>::Output;
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("polimec-mainnet"),
	impl_name: create_runtime_str!("polimec-mainnet"),
	authoring_version: 1,
	spec_version: 0_007_001,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const SS58Prefix: u16 = 41;
}

pub struct BaseCallFilter;
impl Contains<RuntimeCall> for BaseCallFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match c {
			RuntimeCall::Funding(call) => {
				matches!(
					call,
					pallet_funding::Call::create_project { .. } |
						pallet_funding::Call::edit_project { .. } |
						pallet_funding::Call::remove_project { .. } |
						pallet_funding::Call::start_evaluation { .. } |
						pallet_funding::Call::evaluate { .. }
				)
			},
			_ => true,
		}
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::System(..) |
				RuntimeCall::ParachainSystem(..) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Utility(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::Proxy(..) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Vesting(pallet_vesting::Call::vest {..}) |
				RuntimeCall::Vesting(pallet_vesting::Call::vest_other {..}) |
				RuntimeCall::ParachainStaking(..) |
				RuntimeCall::Treasury(..) |
				RuntimeCall::Democracy(..) |
				RuntimeCall::Council(..) |
				RuntimeCall::TechnicalCommittee(..) |
				RuntimeCall::Elections(..) |
				RuntimeCall::Preimage(..) |
				RuntimeCall::Scheduler(..) |
				RuntimeCall::Oracle(..) |
				RuntimeCall::OracleProvidersMembership(..)
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Treasury(..) |
					RuntimeCall::Democracy(..) |
					RuntimeCall::Council(..) |
					RuntimeCall::TechnicalCommittee(..) |
					RuntimeCall::Elections(..) |
					RuntimeCall::Preimage(..) |
					RuntimeCall::Scheduler(..)
			),
			ProxyType::Staking => {
				matches!(c, RuntimeCall::ParachainStaking(..))
			},
			ProxyType::IdentityJudgement =>
				matches!(c, RuntimeCall::Identity(pallet_identity::Call::provide_judgement { .. })),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = BaseCallFilter;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// The ubiquitous task type.
	type RuntimeTask = RuntimeTask;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	/// Runtime version.
	type Version = Version;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = ParachainStaking;
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
}

pub struct DustRemovalAdapter;

impl tokens::imbalance::OnUnbalanced<CreditOf<Runtime>> for DustRemovalAdapter {
	fn on_nonzero_unbalanced(amount: CreditOf<Runtime>) {
		let treasury_account = PayMaster::get();
		let _ = <Balances as tokens::fungible::Balanced<AccountId>>::resolve(&treasury_account, amount);
	}
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = DustRemovalAdapter;
	type ExistentialDeposit = ExistentialDeposit;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = MaxReserves;
	type MaxHolds = MaxLocks;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = shared_configuration::fee::FungibleAdapter<Balances, DealWithFees<Runtime>>;
	type OperationalFeeMultiplier = frame_support::traits::ConstU8<5>;
	type RuntimeEvent = RuntimeEvent;
	type WeightToFee = WeightToFee;
}

pub type ForeignAssetsInstance = pallet_assets::Instance2;

impl pallet_assets::Config<ForeignAssetsInstance> for Runtime {
	type ApprovalDeposit = ExistentialDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type AssetDeposit = AssetDeposit;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type CallbackHandle = ();
	// TODO Check Creation Origin
	type CreateOrigin = AsEnsureOriginWithArg<EnsureRootWithSuccess<AccountId, RootOperatorAccountId>>;
	type Currency = Balances;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
}

type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
	Runtime,
	RELAY_CHAIN_SLOT_DURATION_MILLIS,
	BLOCK_PROCESSING_VELOCITY,
	UNINCLUDED_SEGMENT_CAPACITY,
>;
impl cumulus_pallet_parachain_system::Config for Runtime {
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type RuntimeEvent = RuntimeEvent;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type WeightInfo = weights::cumulus_pallet_parachain_system::WeightInfo<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

// TODO: remove after upgrading pallet-xcm.
// We need this mock for now which is used on parity's parachains that use our version of pallet-xcm.
// This is due to the channel to AssetHub not being ready at genesis, and requiring a complex setup that is not relevant for benchmarking.
pub struct MockedChannelInfo;
impl cumulus_primitives_core::GetChannelInfo for MockedChannelInfo {
	fn get_channel_status(id: ParaId) -> cumulus_primitives_core::ChannelStatus {
		if id == 1000.into() {
			return cumulus_primitives_core::ChannelStatus::Ready(usize::MAX, usize::MAX)
		}

		ParachainSystem::get_channel_status(id)
	}

	fn get_channel_info(id: ParaId) -> Option<cumulus_primitives_core::ChannelInfo> {
		if id == 1000.into() {
			return Some(cumulus_primitives_core::ChannelInfo {
				max_capacity: u32::MAX,
				max_total_size: u32::MAX,
				max_message_size: u32::MAX,
				msg_count: 0,
				total_size: 0,
			})
		}

		ParachainSystem::get_channel_info(id)
	}
}
impl cumulus_pallet_xcmp_queue::Config for Runtime {
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ChannelInfo = ParachainSystem;
	#[cfg(feature = "runtime-benchmarks")]
	type ChannelInfo = MockedChannelInfo;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type MaxInboundSuspended = sp_core::ConstU32<1_000>;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type WeightInfo = weights::cumulus_pallet_xcmp_queue::WeightInfo<Runtime>;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type DmpSink = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::cumulus_pallet_dmp_queue::WeightInfo<Runtime>;
}

parameter_types! {
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type HeapSize = sp_core::ConstU32<{ 64 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		polimec_xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type RuntimeEvent = RuntimeEvent;
	type ServiceWeight = MessageQueueServiceWeight;
	type Size = u32;
	type WeightInfo = weights::pallet_message_queue::WeightInfo<Runtime>;
}

impl pallet_session::Config for Runtime {
	type Keys = SessionKeys;
	type NextSessionRotation = ParachainStaking;
	type RuntimeEvent = RuntimeEvent;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = ParachainStaking;
	type ShouldEndSession = ParachainStaking;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type WeightInfo = weights::pallet_session::WeightInfo<Runtime>;
}

impl pallet_aura::Config for Runtime {
	type AllowMultipleBlocksPerSlot = frame_support::traits::ConstBool<false>;
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

pub struct ToTreasury;

impl tokens::imbalance::OnUnbalanced<CreditOf<Runtime>> for ToTreasury {
	fn on_nonzero_unbalanced(amount: CreditOf<Runtime>) {
		let treasury_account = Treasury::account_id();
		let _ = <Balances as tokens::fungible::Balanced<AccountId>>::resolve(&treasury_account, amount);
	}
}

parameter_types! {
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>,
	>;
	type AssetKind = ();
	type BalanceConverter = UnityAssetBalanceConversion;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = TreasuryBenchmarkHelper;
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type MaxApprovals = MaxApprovals;
	type OnSlash = Treasury;
	type PalletId = TreasuryId;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type PayoutPeriod = PayoutPeriod;
	type ProposalBond = ProposalBond;
	type ProposalBondMaximum = ();
	type ProposalBondMinimum = ProposalBondMinimum;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>,
	>;
	type RuntimeEvent = RuntimeEvent;
	type SpendFunds = ();
	type SpendOrigin = SpendOrigin;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposalWeight = MaxCollectivesProposalWeight;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SetMembersOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_collective::WeightInfo<Runtime>;
}

impl pallet_elections_phragmen::Config for Runtime {
	type Balance = Balance;
	/// How much should be locked up in order to submit one's candidacy.
	type CandidacyBond = CandidacyBond;
	type ChangeMembers = Council;
	type Currency = Balances;
	type CurrencyToVote = CurrencyToVote;
	/// Number of members to elect.
	type DesiredMembers = DesiredMembers;
	/// Number of runners_up to keep.
	type DesiredRunnersUp = DesiredRunnersUp;
	type InitializeMembers = Council;
	type LoserCandidate = ToTreasury;
	type MaxCandidates = MaxCandidates;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	/// How long each seat is kept. This defines the next block number at which
	/// an election round will happen. If set to zero, no elections are ever
	/// triggered and the module will be in passive mode.
	type TermDuration = TermDuration;
	type VotingLockPeriod = VotingLockPeriod;
	type WeightInfo = weights::pallet_elections_phragmen::WeightInfo<Runtime>;
}

pub struct Electorate;
impl GetElectorate<Balance> for Electorate {
	fn get_electorate() -> Balance {
		let total_issuance = Balances::total_issuance();
		let growth_treasury_balance = Balances::balance(&Treasury::account_id());
		let protocol_treasury_balance = Balances::balance(&PayMaster::get());
		total_issuance.saturating_sub(growth_treasury_balance).saturating_sub(protocol_treasury_balance)
	}
}

impl pallet_democracy::Config for Runtime {
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	type CooloffPeriod = CooloffPeriod;
	type Electorate = Electorate;
	type EnactmentPeriod = EnactmentPeriod;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 3, 5>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type Fungible = Balances;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type InstantOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type LaunchPeriod = LaunchPeriod;
	type MaxBlacklisted = MaxBlacklisted;
	type MaxDeposits = MaxDeposits;
	type MaxProposals = MaxProposals;
	type MaxVotes = MaxVotes;
	// Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	type PalletsOrigin = OriginCaller;
	type Preimages = Preimage;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Scheduler = Scheduler;
	type Slash = ToTreasury;
	type SubmitOrigin = EnsureSigned<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type VoteLockingPeriod = EnactmentPeriod;
	type VotingPeriod = VotingPeriod;
	type WeightInfo = weights::pallet_democracy::WeightInfo<Runtime>;
}

pub struct EqualOrGreatestRootCmp;

impl PrivilegeCmp<OriginCaller> for EqualOrGreatestRootCmp {
	fn cmp_privilege(left: &OriginCaller, right: &OriginCaller) -> Option<Ordering> {
		if left == right {
			return Some(Ordering::Equal);
		}
		match (left, right) {
			// Root is greater than anything.
			(OriginCaller::system(frame_system::RawOrigin::Root), _) => Some(Ordering::Greater),
			_ => None,
		}
	}
}

impl pallet_scheduler::Config for Runtime {
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type MaximumWeight = MaximumSchedulerWeight;
	type OriginPrivilegeCmp = EqualOrGreatestRootCmp;
	type PalletsOrigin = OriginCaller;
	type Preimages = Preimage;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
}

parameter_types! {
	pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_preimage::WeightInfo<Runtime>;
}

impl pallet_parachain_staking::Config for Runtime {
	type Balance = Balance;
	type CandidateBondLessDelay = CandidateBondLessDelay;
	type Currency = Balances;
	type DelegationBondLessDelay = DelegationBondLessDelay;
	type LeaveCandidatesDelay = LeaveCandidatesDelay;
	type LeaveDelegatorsDelay = LeaveDelegatorsDelay;
	type MaxBottomDelegationsPerCandidate = MaxBottomDelegationsPerCandidate;
	type MaxDelegationsPerDelegator = MaxDelegationsPerDelegator;
	type MaxTopDelegationsPerCandidate = MaxTopDelegationsPerCandidate;
	type MinBlocksPerRound = MinBlocksPerRound;
	type MinCandidateStk = MinCandidateStk;
	type MinDelegation = MinDelegation;
	type MinDelegatorStk = MinDelegatorStk;
	type MinSelectedCandidates = MinSelectedCandidates;
	type MonetaryGovernanceOrigin = frame_system::EnsureRoot<AccountId>;
	type OnCollatorPayout = ();
	type OnNewRound = ();
	type PayMaster = PayMaster;
	// We use the default implementation, so we leave () here.
	type PayoutCollatorReward = ();
	type RevokeDelegationDelay = RevokeDelegationDelay;
	type RewardPaymentDelay = RewardPaymentDelay;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = weights::pallet_parachain_staking::WeightInfo<Runtime>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type AddOrigin = EnsureRoot<AccountId>;
	type MaxMembers = ConstU32<50>;
	type MembershipChanged = Oracle;
	type MembershipInitialized = ();
	type PrimeOrigin = EnsureRoot<AccountId>;
	type RemoveOrigin = EnsureRoot<AccountId>;
	type ResetOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SwapOrigin = EnsureRoot<AccountId>;
	// TODO: Fix the pallet_membership benchmarks and add the WeightInfo.
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumCount: u32 = 3;
	pub const ExpiresIn: Moment = 1000 * 60; // 1 mins
	pub const MaxHasDispatchedSize: u32 = 20;
	pub RootOperatorAccountId: AccountId = AccountId::from([0xffu8; 32]);
	pub const MaxFeedValues: u32 = 4; // max 4 values allowd to feed in one call (USDT, USDC, DOT, PLMC).
}

impl orml_oracle::Config for Runtime {
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn, ()>;
	type MaxFeedValues = MaxFeedValues;
	type MaxHasDispatchedSize = MaxHasDispatchedSize;
	type Members = OracleProvidersMembership;
	type OnNewData = ();
	type OracleKey = AssetId;
	type OracleValue = Price;
	type RootOperatorAccountId = RootOperatorAccountId;
	type RuntimeEvent = RuntimeEvent;
	type Time = Timestamp;
	// TODO Add weight info
	type WeightInfo = ();
}

parameter_types! {
	pub const FetchInterval: u32 = 50;
	pub const FetchWindow: u32 = 5;
}

impl pallet_oracle_ocw::Config for Runtime {
	type AppCrypto = pallet_oracle_ocw::crypto::PolimecCrypto;
	type ConvertAssetPricePair = AssetPriceConverter;
	type FetchInterval = FetchInterval;
	type FetchWindow = FetchWindow;
	type Members = OracleProvidersMembership;
	type RuntimeEvent = RuntimeEvent;
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: <Runtime as frame_system::Config>::Nonce,
	) -> Option<(RuntimeCall, <UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload)> {
		use sp_runtime::traits::StaticLookup;
		// take the biggest period possible.
		let period = BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;

		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			// TODO: Return to parity CheckNonce implementation once
			// https://github.com/paritytech/polkadot-sdk/issues/3991 is resolved.
			pallet_dispenser::extensions::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			// TODO: Use parity's implementation once
			// https://github.com/paritytech/polkadot-sdk/pull/3993 is available.
			pallet_dispenser::extensions::SkipCheckIfFeeless::from(
				pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
			),
		);
		let raw_payload = generic::SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, extra, _) = raw_payload.deconstruct();
		let address = <Runtime as frame_system::Config>::Lookup::unlookup(account);
		Some((call, (address, signature, extra)))
	}
}

impl pallet_vesting::Config for Runtime {
	type BlockNumberProvider = System;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = shared_configuration::vesting::MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type UnvestedFundsAllowedWithdrawReasons = shared_configuration::vesting::UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;

	const MAX_VESTING_SCHEDULES: u32 = 12;
}

impl pallet_utility::Config for Runtime {
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
}

impl pallet_multisig::Config for Runtime {
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_multisig::WeightInfo<Runtime>;
}

impl pallet_proxy::Config for Runtime {
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type CallHasher = BlakeTwo256;
	type Currency = Balances;
	type MaxPending = MaxPending;
	type MaxProxies = MaxProxies;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type ProxyType = ProxyType;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = weights::pallet_proxy::WeightInfo<Runtime>;
}

impl pallet_identity::Config for Runtime {
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type Currency = Balances;
	type ForceOrigin = EnsureRoot<AccountId>;
	type IdentityInformation = pallet_identity::legacy::IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxSuffixLength = MaxSuffixLength;
	type MaxUsernameLength = MaxUsernameLength;
	type OffchainSignature = Signature;
	type PendingUsernameExpiration = PendingUsernameExpiration;
	type RegistrarOrigin = EnsureRoot<AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type Slashed = Treasury;
	type SubAccountDeposit = SubAccountDeposit;
	type UsernameAuthorityOrigin = UsernameAuthorityOrigin;
	type WeightInfo = weights::pallet_identity::WeightInfo<Runtime>;
}

pub type ContributionTokensInstance = pallet_assets::Instance1;
impl pallet_assets::Config<ContributionTokensInstance> for Runtime {
	type ApprovalDeposit = ExistentialDeposit;
	type AssetAccountDeposit = ZeroDeposit;
	type AssetDeposit = AssetDeposit;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type CallbackHandle = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Currency = Balances;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = ZeroDeposit;
	type MetadataDepositPerByte = ZeroDeposit;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub ContributionTreasuryAccount: AccountId = FundingPalletId::get().into_account_truncating();
	pub PolimecReceiverInfo: xcm::v3::PalletInfo = xcm::v3::PalletInfo::new(
		51, "PolimecReceiver".into(), "polimec_receiver".into(), 0, 1, 0
	).unwrap();
	pub MaxMessageSizeThresholds: (u32, u32) = (50000, 102_400);
	pub MaxCapacityThresholds: (u32, u32) = (8, 1000);
	pub RequiredMaxCapacity: u32 = 1000;
	pub RequiredMaxMessageSize: u32 = 102_400;
	pub MinUsdPerEvaluation: Balance = 100 * USD_UNIT;

}

// Development public key
#[cfg(any(feature = "development-settings", test))]
parameter_types! {
	pub VerifierPublicKey: [u8; 32] = [
		32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253,
		195, 18, 202, 111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
	];
}

// Production public key
#[cfg(not(any(feature = "development-settings", test)))]
parameter_types! {
	pub VerifierPublicKey: [u8; 32] = [
		83,  49,  95, 191,  98, 138,  14,  43, 234, 192, 105, 248,  11,  96, 127, 234, 192,  62,  80,
		35, 204,   0,  38, 210, 177,  72, 167, 116, 133, 127, 140, 249
	 ];
}

pub struct ConvertSelf;
impl Convert<AccountId, [u8; 32]> for ConvertSelf {
	fn convert(account_id: AccountId) -> [u8; 32] {
		account_id.into()
	}
}
impl ConvertBack<AccountId, [u8; 32]> for ConvertSelf {
	fn convert_back(bytes: [u8; 32]) -> AccountId {
		bytes.into()
	}
}

impl pallet_funding::Config for Runtime {
	type AccountId32Conversion = ConvertSelf;
	#[cfg(any(test, feature = "runtime-benchmarks", feature = "std"))]
	type AllPalletsWithoutSystem =
		(Balances, ContributionTokens, ForeignAssets, Oracle, Funding, LinearRelease, Random);
	type AuctionClosingDuration = AuctionClosingDuration;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type AuctionOpeningDuration = AuctionOpeningDuration;
	type Balance = Balance;
	type BlockNumber = BlockNumber;
	type BlockNumberToBalance = ConvertInto;
	type CommunityFundingDuration = CommunityFundingDuration;
	type ContributionTokenCurrency = ContributionTokens;
	type ContributionTreasury = ContributionTreasuryAccount;
	type DaysToBlocks = DaysToBlocks;
	type EvaluationDuration = EvaluationDuration;
	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
	type EvaluatorSlash = EvaluatorSlash;
	type FeeBrackets = FeeBrackets;
	type FundingCurrency = ForeignAssets;
	type InvestorOrigin = EnsureInvestor<Runtime>;
	type ManualAcceptanceDuration = ManualAcceptanceDuration;
	type MaxBidsPerProject = ConstU32<1024>;
	type MaxBidsPerUser = ConstU32<16>;
	type MaxCapacityThresholds = MaxCapacityThresholds;
	type MaxContributionsPerUser = ConstU32<16>;
	type MaxEvaluationsPerProject = ConstU32<1024>;
	type MaxEvaluationsPerUser = ConstU32<16>;
	type MaxMessageSizeThresholds = MaxMessageSizeThresholds;
	type MaxProjectsToUpdateInsertionAttempts = ConstU32<100>;
	type MaxProjectsToUpdatePerBlock = ConstU32<1>;
	type MinUsdPerEvaluation = MinUsdPerEvaluation;
	type Multiplier = pallet_funding::types::Multiplier;
	type NativeCurrency = Balances;
	type PalletId = FundingPalletId;
	type PolimecReceiverInfo = PolimecReceiverInfo;
	type PreImageLimit = ConstU32<1024>;
	type Price = Price;
	type PriceProvider = OraclePriceProvider<AssetId, Price, Oracle>;
	type ProtocolGrowthTreasury = TreasuryAccount;
	type Randomness = Random;
	type RemainderFundingDuration = RemainderFundingDuration;
	type RequiredMaxCapacity = RequiredMaxCapacity;
	type RequiredMaxMessageSize = RequiredMaxMessageSize;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeOrigin = RuntimeOrigin;
	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	type SetPrices = crate::benchmarks::helpers::SetOraclePrices;
	type StringLimit = ConstU32<64>;
	type SuccessToSettlementTime = SuccessToSettlementTime;
	type VerifierPublicKey = VerifierPublicKey;
	type Vesting = LinearRelease;
	type WeightInfo = weights::pallet_funding::WeightInfo<Runtime>;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub BenchmarkReason: RuntimeHoldReason = RuntimeHoldReason::Funding(pallet_funding::HoldReason::Participation(0));
}

impl pallet_linear_release::Config for Runtime {
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkReason = BenchmarkReason;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = shared_configuration::vesting::MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type UnvestedFundsAllowedWithdrawReasons = shared_configuration::vesting::UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = pallet_linear_release::weights::SubstrateWeight<Runtime>;

	const MAX_VESTING_SCHEDULES: u32 = 26;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

ord_parameter_types! {
	pub const DispenserAdminAccount: AccountId = AccountId::from(hex_literal::hex!("d85a4f58eb7dba17bc436b16f394b242271237021f7880e1ccaf36cd9a616c99"));
}

// #[test]
// fn ensure_admin_account_is_correct() {
// 	use frame_support::traits::SortedMembers;
// 	use sp_core::crypto::Ss58Codec;
// 	let acc = AccountId::from_ss58check("5BAimacvMnhBEoc2g7PaiuEhJwmMZejq6j1ZMCpDZMHGAogz").unwrap();
// 	assert_eq!(acc, DispenserAdminAccount::sorted_members()[0]);
// }

impl pallet_dispenser::Config for Runtime {
	type AdminOrigin = EnsureSignedBy<DispenserAdminAccount, AccountId>;
	type BlockNumberToBalance = ConvertInto;
	type FreeDispenseAmount = FreeDispenseAmount;
	type InitialDispenseAmount = InitialDispenseAmount;
	type InvestorOrigin = EnsureInvestor<Runtime>;
	type LockPeriod = DispenserLockPeriod;
	type PalletId = DispenserId;
	type RuntimeEvent = RuntimeEvent;
	type VerifierPublicKey = VerifierPublicKey;
	type VestPeriod = DispenserVestPeriod;
	type VestingSchedule = Vesting;
	type WeightInfo = weights::pallet_dispenser::WeightInfo<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime
	{
		// System support stuff.
		System: frame_system = 0,
		ParachainSystem: cumulus_pallet_parachain_system = 1,
		Timestamp: pallet_timestamp = 2,
		ParachainInfo: parachain_info = 3,
		// Index 4 used to be Sudo
		Utility: pallet_utility::{Pallet, Call, Event} = 5,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 6,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 7,
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 8,

		// Monetary stuff.
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,
		Vesting: pallet_vesting = 12,
		ContributionTokens: pallet_assets::<Instance1> = 13,
		ForeignAssets: pallet_assets::<Instance2> = 14,
		Dispenser: pallet_dispenser = 15,

		// Collator support. the order of these 5 are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Storage} = 20,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
		Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config<T>} = 24,
		ParachainStaking: pallet_parachain_staking::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason} = 25,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 30,
		PolkadotXcm: pallet_xcm = 31,
		CumulusXcm: cumulus_pallet_xcm = 32,
		DmpQueue: cumulus_pallet_dmp_queue = 33,
		MessageQueue: pallet_message_queue = 34,

		// Governance
		Treasury: pallet_treasury = 40,
		Democracy: pallet_democracy::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason, FreezeReason} = 41,
		Council: pallet_collective::<Instance1> = 42,
		TechnicalCommittee: pallet_collective::<Instance2> = 43,
		Elections: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason, FreezeReason} = 44,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>, HoldReason} = 45,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 46,

		Random: pallet_insecure_randomness_collective_flip = 50,

		// Oracle
		Oracle: orml_oracle::{Pallet, Call, Storage, Event<T>} = 70,
		OracleProvidersMembership: pallet_membership::<Instance1> = 71,
		OracleOffchainWorker: pallet_oracle_ocw::{Pallet, Event<T>} = 72,

		Funding: pallet_funding = 80,
		LinearRelease: pallet_linear_release = 81,
	}
);

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	frame_benchmarking::define_benchmarks!(
		// System support stuff.
		[frame_system, SystemBench::<Runtime>]
		[pallet_timestamp, Timestamp]
		[pallet_utility, Utility]
		[pallet_multisig, Multisig]
		[pallet_proxy, Proxy]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[pallet_identity, Identity]

		// Monetary stuff.
		[pallet_balances, Balances]
		[pallet_vesting, Vesting]
		[pallet_assets, ForeignAssets]
		[pallet_assets, ContributionTokens]
		[pallet_dispenser, Dispenser]

		// Collator support.
		[pallet_session, SessionBench::<Runtime>]
		[pallet_parachain_staking, ParachainStaking]

		// XCM helpers.
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		[pallet_xcm, PalletXcmExtrinsiscsBenchmark::<Runtime>]
		[cumulus_pallet_dmp_queue, DmpQueue]
		[pallet_message_queue, MessageQueue]

		// Governance
		[pallet_treasury, Treasury]
		[pallet_democracy, Democracy]
		[pallet_collective, Council]
		[pallet_collective, TechnicalCommittee]
		[pallet_elections_phragmen, Elections]
		[pallet_preimage, Preimage]
		[pallet_scheduler, Scheduler]

		// Oracle
		// [pallet_membership, OracleProvidersMembership]
		// [orml_oracle, Oracle]

		// Funding
		[pallet_funding, Funding]
		[pallet_linear_release, LinearRelease]
	);
}

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(parachains_common::SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
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

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
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
			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsiscsBenchmark;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{BenchmarkError, Benchmarking, BenchmarkBatch};

			use frame_system_benchmarking::Pallet as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			impl cumulus_pallet_session_benchmarking::Config for Runtime {}

			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
			use xcm::latest::prelude::*;
			use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsiscsBenchmark;
			/// TODO: Update these benchmarks once we enable PLMC Teleportation and upgrade pallet_xcm. New version has
			/// a better and quite different trait
			impl pallet_xcm::benchmarking::Config for Runtime {
				fn reachable_dest() -> Option<MultiLocation> {
					PolkadotXcm::force_xcm_version(
						RuntimeOrigin::root(),
						Box::new(crate::xcm_config::AssetHubLocation::get()),
						xcm::prelude::XCM_VERSION
					).unwrap();
					Some(crate::xcm_config::AssetHubLocation::get())
				}

				fn reserve_transferable_asset_and_dest() -> Option<(MultiAsset, MultiLocation)> {
					PolkadotXcm::force_xcm_version(
						RuntimeOrigin::root(),
						Box::new(crate::xcm_config::AssetHubLocation::get()),
						xcm::prelude::XCM_VERSION
					).unwrap();
					Some((
						MultiAsset {
							fun: Fungible(ExistentialDeposit::get()),
							id: Concrete(Here.into())
						},
						crate::xcm_config::AssetHubLocation::get(),
					))
				}
			}
			use frame_support::traits::WhitelistedStorageKeys;
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
