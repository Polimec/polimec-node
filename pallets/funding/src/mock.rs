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
use frame_support::{
	pallet_prelude::Weight,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU32, WithdrawReasons},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	BuildStorage,
};
use sp_std::collections::btree_map::BTreeMap;
use system::EnsureSigned;

type Block = frame_system::mocking::MockBlock<TestRuntime>;

// pub type AccountId = u64;
pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Identifier = u32;

pub const PLMC: u128 = 10_000_000_000_u128;
pub const MILLI_PLMC: Balance = 10u128.pow(7);
pub const MICRO_PLMC: Balance = 10u128.pow(4);
pub const EXISTENTIAL_DEPOSIT: Balance = 10 * MILLI_PLMC;

const US_DOLLAR: u128 = 1_0_000_000_000u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum TestRuntime
	{
		System: frame_system,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Balances: pallet_balances,
		FundingModule: crate::{Pallet, Call, Storage, Event<T>, Config<T>, HoldReason},
		Vesting: pallet_linear_release,
		LocalAssets: pallet_assets::<Instance1>::{Pallet, Call, Storage, Event<T>},
		StatemintAssets: pallet_assets::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},
		PolkadotXcm: pallet_xcm,
	}
);

pub type LocalAssetsInstance = pallet_assets::Instance1;
pub type StatemintAssetsInstance = pallet_assets::Instance2;

pub type AssetId = u32;
pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * MICRO_PLMC + (bytes as Balance) * 6 * MICRO_PLMC
}

pub const fn free_deposit() -> Balance {
	0 * MICRO_PLMC
}

use frame_support::traits::{Everything, OriginTrait};
use frame_system::RawOrigin as SystemRawOrigin;
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{ConvertBack, Get, TryConvert};
use xcm_builder::{EnsureXcmOrigin, FixedWeightBounds, ParentIsPreset, SiblingParachainConvertsVia};

pub struct SignedToAccountIndex<RuntimeOrigin, AccountId, Network>(PhantomData<(RuntimeOrigin, AccountId, Network)>);

impl<RuntimeOrigin: OriginTrait + Clone, AccountId: Into<u64>, Network: Get<Option<NetworkId>>>
	TryConvert<RuntimeOrigin, MultiLocation> for SignedToAccountIndex<RuntimeOrigin, AccountId, Network>
where
	RuntimeOrigin::PalletsOrigin:
		From<SystemRawOrigin<AccountId>> + TryInto<SystemRawOrigin<AccountId>, Error = RuntimeOrigin::PalletsOrigin>,
{
	fn try_convert(o: RuntimeOrigin) -> Result<MultiLocation, RuntimeOrigin> {
		o.try_with_caller(|caller| match caller.try_into() {
			Ok(SystemRawOrigin::Signed(who)) =>
				Ok(Junction::AccountIndex64 { network: Network::get(), index: who.into() }.into()),
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
#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
}
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
impl pallet_xcm::Config for TestRuntime {
	type AdminOrigin = EnsureRoot<AccountId>;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
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
	type XcmExecutor = ();
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = ();
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

parameter_types! {
	pub const AssetDeposit: Balance = PLMC; // 1 UNIT deposit to create asset
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const AssetsStringLimit: u32 = 50;
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = free_deposit();
	pub const MetadataDepositPerByte: Balance = free_deposit();
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;

}
impl pallet_assets::Config<LocalAssetsInstance> for TestRuntime {
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

impl pallet_assets::Config<StatemintAssetsInstance> for TestRuntime {
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
	type ReserveIdentifier = LockType<u32>;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
}

impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}

pub const HOURS: BlockNumber = 300u64;

// REMARK: In the production configuration we use DAYS instead of HOURS.
parameter_types! {
	pub const EvaluationDuration: BlockNumber = (28 * HOURS) as BlockNumber;
	pub const AuctionInitializePeriodDuration: BlockNumber = (7 * HOURS) as BlockNumber;
	pub const EnglishAuctionDuration: BlockNumber = (2 * HOURS) as BlockNumber;
	pub const CandleAuctionDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const CommunityRoundDuration: BlockNumber = (5 * HOURS) as BlockNumber;
	pub const RemainderFundingDuration: BlockNumber = HOURS as BlockNumber;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
	pub const ManualAcceptanceDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const SuccessToSettlementTime: BlockNumber =(4 * HOURS) as BlockNumber;
	pub PriceMap: BTreeMap<AssetId, FixedU128> = BTreeMap::from_iter(vec![
		(0u32, FixedU128::from_float(69f64)), // DOT
		(420u32, FixedU128::from_float(0.97f64)), // USDC
		(1984u32, FixedU128::from_float(1.0f64)), // USDT
		(2069u32, FixedU128::from_float(8.4f64)), // PLMC
	]);
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * US_DOLLAR),
		(Percent::from_percent(8), 5_000_000 * US_DOLLAR),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
	pub TreasuryAccount: AccountId = AccountId::from(69u64);

}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub PolimecReceiverInfo: xcm::v3::PalletInfo = xcm::v3::PalletInfo::new(
		51, "PolimecReceiver".into(), "polimec_receiver".into(), 0, 1, 0
	).unwrap();
}
impl pallet_linear_release::Config for TestRuntime {
	type Balance = Balance;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeEvent = RuntimeEvent;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 32;
}

parameter_types! {
	pub MaxMessageSizeThresholds: (u32, u32) = (50000, 102_400);
	pub MaxCapacityThresholds: (u32, u32) = (8, 1000);
	pub RequiredMaxCapacity: u32 = 8;
	pub RequiredMaxMessageSize: u32 = 102_400;

}

pub struct DummyConverter;
impl sp_runtime::traits::Convert<AccountId, [u8; 32]> for DummyConverter {
	fn convert(a: AccountId) -> [u8; 32] {
		let mut account: [u8; 32] = [0u8; 32];
		account[0..7].copy_from_slice(a.to_le_bytes().as_slice());
		account
	}
}
impl ConvertBack<AccountId, [u8; 32]> for DummyConverter {
	fn convert_back(bytes: [u8; 32]) -> AccountId {
		let account: [u8; 8] = bytes[0..7].try_into().unwrap();
		u64::from_le_bytes(account)
	}
}

impl Config for TestRuntime {
	type AccountId32Conversion = DummyConverter;
	type AllPalletsWithoutSystem = AllPalletsWithoutSystem;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type Balance = Balance;
	type BlockNumber = BlockNumber;
	type BlockNumberToBalance = ConvertInto;
	type CandleAuctionDuration = CandleAuctionDuration;
	type CommunityFundingDuration = CommunityRoundDuration;
	type ContributionTokenCurrency = LocalAssets;
	type ContributionVesting = ConstU32<4>;
	type DaysToBlocks = DaysToBlocks;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type EvaluationDuration = EvaluationDuration;
	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
	type EvaluatorSlash = EvaluatorSlash;
	type FeeBrackets = FeeBrackets;
	type FundingCurrency = StatemintAssets;
	type ManualAcceptanceDuration = ManualAcceptanceDuration;
	// Low value to simplify the tests
	type MaxBidsPerUser = ConstU32<4>;
	type MaxCapacityThresholds = MaxCapacityThresholds;
	type MaxContributionsPerUser = ConstU32<4>;
	type MaxEvaluationsPerUser = ConstU32<4>;
	type MaxMessageSizeThresholds = MaxMessageSizeThresholds;
	type MaxProjectsToUpdatePerBlock = ConstU32<100>;
	type Multiplier = Multiplier;
	type NativeCurrency = Balances;
	type PalletId = FundingPalletId;
	type PolimecReceiverInfo = PolimecReceiverInfo;
	type PreImageLimit = ConstU32<1024>;
	type Price = FixedU128;
	type PriceProvider = ConstPriceProvider<AssetId, FixedU128, PriceMap>;
	type ProjectIdentifier = Identifier;
	type Randomness = RandomnessCollectiveFlip;
	type RemainderFundingDuration = RemainderFundingDuration;
	type RequiredMaxCapacity = RequiredMaxCapacity;
	type RequiredMaxMessageSize = RequiredMaxMessageSize;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	#[cfg(feature = "runtime-benchmarks")]
	type SetPrices = ();
	type StringLimit = ConstU32<64>;
	type SuccessToSettlementTime = SuccessToSettlementTime;
	type TreasuryAccount = TreasuryAccount;
	type Vesting = Vesting;
	type WeightInfo = weights::SubstrateWeight<TestRuntime>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

	GenesisConfig {
		balances: BalancesConfig {
			balances: vec![(
				<TestRuntime as Config>::PalletId::get().into_account_truncating(),
				<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get(),
			)],
		},
		statemint_assets: StatemintAssetsConfig {
			assets: vec![(
				AcceptedFundingAsset::USDT.to_statemint_id(),
				<TestRuntime as Config>::PalletId::get().into_account_truncating(),
				false,
				10,
			)],
			metadata: vec![],
			accounts: vec![],
		},
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	// In order to emit events the block number must be more than 0
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn hashed(data: impl AsRef<[u8]>) -> H256 {
	<BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
}
