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

//! Test environment for Funding pallet.

use super::*;
use crate as pallet_funding;
use frame_support::{
	construct_runtime,
	pallet_prelude::Weight,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU32, ConstU64, WithdrawReasons},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use polimec_common::credentials::EnsureInvestor;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	BuildStorage,
};
use sp_std::collections::btree_map::BTreeMap;
use std::cell::RefCell;
use system::EnsureSigned;

type Block = frame_system::mocking::MockBlock<TestRuntime>;

pub type AccountId = u32;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Identifier = u32;
pub type Price = FixedU128;

pub const PLMC: u128 = 10_000_000_000_u128;
pub const MILLI_PLMC: Balance = 10u128.pow(7);
pub const MICRO_PLMC: Balance = 10u128.pow(4);
pub const EXISTENTIAL_DEPOSIT: Balance = 10 * MILLI_PLMC;

const US_DOLLAR: u128 = 1_0_000_000_000u128;

pub type ContributionTokensInstance = pallet_assets::Instance1;
pub type ForeignAssetsInstance = pallet_assets::Instance2;

pub type AssetId = u32;
pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * MICRO_PLMC + (bytes as Balance) * 6 * MICRO_PLMC
}

pub const fn free_deposit() -> Balance {
	0 * MICRO_PLMC
}

use crate::traits::ProvideAssetPrice;
use frame_support::traits::{Everything, OriginTrait};
use frame_system::RawOrigin as SystemRawOrigin;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::traits::{ConvertBack, Get, TryConvert};
use xcm_builder::{EnsureXcmOrigin, FixedWeightBounds, ParentIsPreset, SiblingParachainConvertsVia};
use xcm_executor::traits::XcmAssetTransfers;

pub struct SignedToAccountIndex<RuntimeOrigin, AccountId, Network>(PhantomData<(RuntimeOrigin, AccountId, Network)>);

impl<RuntimeOrigin: OriginTrait + Clone, AccountId: Into<u32>, Network: Get<Option<NetworkId>>>
	TryConvert<RuntimeOrigin, MultiLocation> for SignedToAccountIndex<RuntimeOrigin, AccountId, Network>
where
	RuntimeOrigin::PalletsOrigin:
		From<SystemRawOrigin<AccountId>> + TryInto<SystemRawOrigin<AccountId>, Error = RuntimeOrigin::PalletsOrigin>,
{
	fn try_convert(o: RuntimeOrigin) -> Result<MultiLocation, RuntimeOrigin> {
		o.try_with_caller(|caller| match caller.try_into() {
			Ok(SystemRawOrigin::Signed(who)) =>
				Ok(Junction::AccountIndex64 { network: Network::get(), index: Into::<u32>::into(who).into() }.into()),
			Ok(other) => Err(other.into()),
			Err(other) => Err(other),
		})
	}
}
pub type LocalOriginToLocation = SignedToAccountIndex<RuntimeOrigin, AccountId, RelayNetwork>;
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
);

parameter_types! {
	pub UniversalLocation: InteriorMultiLocation = (
		GlobalConsensus(Polkadot),
		 Parachain(3344u32),
	).into();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;

	pub const HereLocation: MultiLocation = MultiLocation::here();
}

pub struct MockPrepared;
impl PreparedMessage for MockPrepared {
	fn weight_of(&self) -> Weight {
		Weight::zero()
	}
}

pub struct MockXcmExecutor;
impl XcmAssetTransfers for MockXcmExecutor {
	type AssetTransactor = ();
	type IsReserve = ();
	type IsTeleporter = ();
}

impl ExecuteXcm<RuntimeCall> for MockXcmExecutor {
	type Prepared = MockPrepared;

	fn prepare(_message: Xcm<RuntimeCall>) -> core::result::Result<Self::Prepared, Xcm<RuntimeCall>> {
		Ok(MockPrepared)
	}

	fn execute(
		_origin: impl Into<MultiLocation>,
		_pre: Self::Prepared,
		_id: &mut XcmHash,
		_weight_credit: Weight,
	) -> Outcome {
		Outcome::Complete(Weight::zero())
	}

	fn charge_fees(_location: impl Into<MultiLocation>, _fees: MultiAssets) -> XcmResult {
		Ok(())
	}
}

impl pallet_xcm::Config for TestRuntime {
	type AdminOrigin = EnsureRoot<AccountId>;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type SovereignAccountOf = LocationToAccountId;
	type TrustedLockers = ();
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	// TODO: change back to `Nothing` once we add the xcm functionalities into a pallet
	type XcmExecuteFilter = Everything;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = MockXcmExecutor;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = ();
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

parameter_types! {
	pub const AssetDeposit: Balance = PLMC; // 1 UNIT deposit to create asset
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const ZeroAssetAccountDeposit: Balance = free_deposit();
	pub const AssetsStringLimit: u32 = 50;
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = free_deposit();
	pub const MetadataDepositPerByte: Balance = free_deposit();
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;

}
impl pallet_assets::Config<ContributionTokensInstance> for TestRuntime {
	type ApprovalDeposit = ApprovalDeposit;
	type AssetAccountDeposit = ZeroAssetAccountDeposit;
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
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type RemoveItemsLimit = ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
}

impl pallet_assets::Config<ForeignAssetsInstance> for TestRuntime {
	type ApprovalDeposit = ApprovalDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
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
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type RemoveItemsLimit = ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
}
parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

impl system::Config for TestRuntime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type Block = Block;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockWeights = ();
	type DbWeight = ();
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Lookup = IdentityLookup<AccountId>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = u64;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type PalletInfo = PalletInfo;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeTask = RuntimeTask;
	type SS58Prefix = ConstU16<42>;
	type SystemWeightInfo = ();
	type Version = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}
impl pallet_balances::Config for TestRuntime {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type MaxHolds = ConstU32<1024>;
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = frame_support::traits::ConstU32<1024>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
}

impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}

impl pallet_timestamp::Config for TestRuntime {
	type MinimumPeriod = ConstU64<5>;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

pub const HOURS: BlockNumber = 300u64;

// REMARK: In the production configuration we use DAYS instead of HOURS.
parameter_types! {
	pub const EvaluationDuration: BlockNumber = (28 * HOURS) as BlockNumber;
	pub const AuctionInitializePeriodDuration: BlockNumber = (7 * HOURS) as BlockNumber;
	pub const AuctionOpeningDuration: BlockNumber = (2 * HOURS) as BlockNumber;
	pub const AuctionClosingDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const CommunityRoundDuration: BlockNumber = (5 * HOURS) as BlockNumber;
	pub const RemainderFundingDuration: BlockNumber = (1 * HOURS) as BlockNumber;
	pub const ManualAcceptanceDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const SuccessToSettlementTime: BlockNumber =(4 * HOURS) as BlockNumber;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * US_DOLLAR),
		(Percent::from_percent(8), 5_000_000 * US_DOLLAR),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
	pub ProtocolGrowthTreasuryAccount: AccountId = AccountId::from(696969u32);
	pub ContributionTreasury: AccountId = AccountId::from(4204204206u32);
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub PolimecReceiverInfo: xcm::v3::PalletInfo = xcm::v3::PalletInfo::new(
		51, "PolimecReceiver".into(), "polimec_receiver".into(), 0, 1, 0
	).unwrap();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub BenchmarkReason: RuntimeHoldReason = RuntimeHoldReason::PolimecFunding(crate::HoldReason::Participation(0));
}
impl pallet_linear_release::Config for TestRuntime {
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkReason = BenchmarkReason;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 32;
}

parameter_types! {
	pub MaxMessageSizeThresholds: (u32, u32) = (50000, 102_400);
	pub MaxCapacityThresholds: (u32, u32) = (8, 1000);
	pub RequiredMaxCapacity: u32 = 8;
	pub RequiredMaxMessageSize: u32 = 102_400;
	pub VerifierPublicKey: [u8; 32] = [
		32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253,
		195, 18, 202, 111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
	];
}

pub struct DummyConverter;
impl sp_runtime::traits::Convert<AccountId, [u8; 32]> for DummyConverter {
	fn convert(a: AccountId) -> [u8; 32] {
		let mut account: [u8; 32] = [0u8; 32];
		account[0..4].copy_from_slice(a.to_le_bytes().as_slice());
		account
	}
}
impl ConvertBack<AccountId, [u8; 32]> for DummyConverter {
	fn convert_back(bytes: [u8; 32]) -> AccountId {
		let account: [u8; 4] = bytes[0..3].try_into().unwrap();
		u32::from_le_bytes(account)
	}
}
thread_local! {
	pub static PRICE_MAP: RefCell<BTreeMap<AssetId, FixedU128>> = RefCell::new(BTreeMap::from_iter(vec![
		(0u32, FixedU128::from_float(69f64)), // DOT
		(1337u32, FixedU128::from_float(0.97f64)), // USDC
		(1984u32, FixedU128::from_float(1.0f64)), // USDT
		(3344u32, FixedU128::from_float(8.4f64)), // PLMC
	]));
}
pub struct ConstPriceProvider;
impl ProvideAssetPrice for ConstPriceProvider {
	type AssetId = AssetId;
	type Price = Price;

	fn get_price(asset_id: AssetId) -> Option<Price> {
		PRICE_MAP.with(|price_map| price_map.borrow().get(&asset_id).cloned())
	}
}

impl ConstPriceProvider {
	pub fn set_price(asset_id: AssetId, price: Price) {
		PRICE_MAP.with(|price_map| {
			price_map.borrow_mut().insert(asset_id, price);
		});
	}
}
impl Config for TestRuntime {
	type AccountId32Conversion = DummyConverter;
	type AllPalletsWithoutSystem =
		(Balances, ContributionTokens, ForeignAssets, PolimecFunding, Vesting, RandomnessCollectiveFlip);
	type AuctionClosingDuration = AuctionClosingDuration;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type AuctionOpeningDuration = AuctionOpeningDuration;
	type Balance = Balance;
	type BlockNumber = BlockNumber;
	type BlockNumberToBalance = ConvertInto;
	type CommunityFundingDuration = CommunityRoundDuration;
	type ContributionTokenCurrency = ContributionTokens;
	type ContributionTreasury = ContributionTreasury;
	type DaysToBlocks = DaysToBlocks;
	type EvaluationDuration = EvaluationDuration;
	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
	type EvaluatorSlash = EvaluatorSlash;
	type FeeBrackets = FeeBrackets;
	type FundingCurrency = ForeignAssets;
	type InvestorOrigin = EnsureInvestor<TestRuntime>;
	type ManualAcceptanceDuration = ManualAcceptanceDuration;
	type MaxBidsPerProject = ConstU32<1024>;
	type MaxBidsPerUser = ConstU32<25>;
	type MaxCapacityThresholds = MaxCapacityThresholds;
	type MaxContributionsPerUser = ConstU32<25>;
	type MaxEvaluationsPerProject = ConstU32<1024>;
	type MaxEvaluationsPerUser = ConstU32<4>;
	type MaxMessageSizeThresholds = MaxMessageSizeThresholds;
	type MaxProjectsToUpdateInsertionAttempts = ConstU32<100>;
	type MaxProjectsToUpdatePerBlock = ConstU32<1>;
	type Multiplier = Multiplier;
	type NativeCurrency = Balances;
	type PalletId = FundingPalletId;
	type PolimecReceiverInfo = PolimecReceiverInfo;
	type PreImageLimit = ConstU32<1024>;
	type Price = FixedU128;
	type PriceProvider = ConstPriceProvider;
	type ProtocolGrowthTreasury = ProtocolGrowthTreasuryAccount;
	type Randomness = RandomnessCollectiveFlip;
	type RemainderFundingDuration = RemainderFundingDuration;
	type RequiredMaxCapacity = RequiredMaxCapacity;
	type RequiredMaxMessageSize = RequiredMaxMessageSize;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeOrigin = RuntimeOrigin;
	#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
	type SetPrices = ();
	type StringLimit = ConstU32<64>;
	type SuccessToSettlementTime = SuccessToSettlementTime;
	type VerifierPublicKey = VerifierPublicKey;
	type Vesting = Vesting;
	type WeightInfo = weights::SubstrateWeight<TestRuntime>;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum TestRuntime
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Balances: pallet_balances,
		Vesting: pallet_linear_release,
		ContributionTokens: pallet_assets::<Instance1>::{Pallet, Call, Storage, Event<T>},
		ForeignAssets: pallet_assets::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},
		PolkadotXcm: pallet_xcm,
		PolimecFunding: pallet_funding::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason}  = 52,
	}
);

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();
	let ed = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
	RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				(<TestRuntime as Config>::PalletId::get().into_account_truncating(), ed),
				(<TestRuntime as Config>::ContributionTreasury::get(), ed),
				(<TestRuntime as Config>::ProtocolGrowthTreasury::get(), ed),
			],
		},
		foreign_assets: ForeignAssetsConfig {
			assets: vec![
				(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				),
				(
					AcceptedFundingAsset::USDC.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				),
				(
					AcceptedFundingAsset::DOT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				),
			],
			metadata: vec![],
			accounts: vec![],
		},
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	// In order to emit events the block number must be more than 0
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

pub fn hashed(data: impl AsRef<[u8]>) -> H256 {
	<BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
}
