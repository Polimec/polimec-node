use frame_support::{
	derive_impl,
	pallet_prelude::RuntimeDebug,
	traits::{AsEnsureOriginWithArg, VariantCount, WithdrawReasons},
	PalletId,
};
use frame_system::{mocking::MockBlock, GenesisConfig};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use polimec_common::ProvideAssetPrice;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::{
	app_crypto::sp_core::MaxEncodedLen,
	traits::{parameter_types, ConstU64, ConstU8, Identity, IdentityLookup},
	BuildStorage, FixedU128, Perbill,
};
use std::{cell::RefCell, collections::BTreeMap};
use xcm::v4::{Junction::Parachain, Location, Parent};

pub const NATIVE_DECIMALS: u8 = 10;
pub const NATIVE_UNIT: u64 = 1 * 10u64.pow(NATIVE_DECIMALS as u32);
pub const MILLI_NATIVE_UNIT: u64 = NATIVE_UNIT / 1_000;

pub fn mock_fee_asset_id() -> Location {
	(Parent, Parachain(0)).into()
}
pub const MOCK_FEE_ASSET_DECIMALS: u8 = 6;
pub const MOCK_FEE_ASSET_UNIT: u64 = 1 * 10u64.pow(MOCK_FEE_ASSET_DECIMALS as u32);

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod test_runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct TestRuntime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;

	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances;

	#[runtime::pallet_index(2)]
	pub type Assets = pallet_assets;

	#[runtime::pallet_index(3)]
	pub type LinearRelease = pallet_linear_release;

	#[runtime::pallet_index(4)]
	pub type ProxyBonding = crate;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for TestRuntime {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = u64;
	type Block = MockBlock<TestRuntime>;
	type Lookup = IdentityLookup<Self::AccountId>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for TestRuntime {
	type AccountStore = System;
	type ExistentialDeposit = ConstU64<MILLI_NATIVE_UNIT>;
	type RuntimeHoldReason = MockRuntimeHoldReason;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct PalletAssetsBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<Location> for PalletAssetsBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> Location {
		(Parent, Parachain(id)).into()
	}
}
#[derive_impl(pallet_assets::config_preludes::TestDefaultConfig)]
impl pallet_assets::Config for TestRuntime {
	type AssetId = Location;
	type AssetIdParameter = Location;
	type Balance = <TestRuntime as pallet_balances::Config>::Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = PalletAssetsBenchmarkHelper;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<u64>>;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type Freezer = ();
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub BenchmarkReason: MockRuntimeHoldReason = MockRuntimeHoldReason::Reason;
}
impl pallet_linear_release::Config for TestRuntime {
	type Balance = <TestRuntime as pallet_balances::Config>::Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkReason = BenchmarkReason;
	type BlockNumberProvider = System;
	type BlockNumberToBalance = Identity;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = MockRuntimeHoldReason;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 10;
}

parameter_types! {
	pub FeePercentage: Perbill = Perbill::from_percent(5);
	pub Treasury: u64 = 420u64;
	pub FeeRecipient: u64 = 69u64;
	pub RootId: PalletId = PalletId(*b"treasury");
}

thread_local! {
	pub static PRICE_MAP: RefCell<BTreeMap<Location, FixedU128>> = RefCell::new(BTreeMap::from_iter(vec![
		(Location::here(), FixedU128::from_float(0.5f64)), // Native Token
		((Parent, Parachain(0)).into(), FixedU128::from_float(1f64)), // Fee Asset
	]));
}
pub struct ConstPriceProvider;
impl ProvideAssetPrice for ConstPriceProvider {
	type AssetId = Location;
	type Price = FixedU128;

	fn get_price(asset_id: Location) -> Option<FixedU128> {
		PRICE_MAP.with(|price_map| price_map.borrow().get(&asset_id).cloned())
	}
}

impl ConstPriceProvider {
	pub fn set_price(asset_id: Location, price: FixedU128) {
		PRICE_MAP.with(|price_map| {
			price_map.borrow_mut().insert(asset_id, price);
		});
	}
}

#[derive(
	Encode,
	Decode,
	Copy,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
	Ord,
	PartialOrd,
	Serialize,
	Deserialize,
	DecodeWithMemTracking,
)]
pub enum MockRuntimeHoldReason {
	Reason,
	Reason2,
}
impl VariantCount for MockRuntimeHoldReason {
	const VARIANT_COUNT: u32 = 2;
}

parameter_types! {
	pub HereLocationGetter: Location = Location::here();
}
impl crate::Config for TestRuntime {
	type BondingToken = Balances;
	type BondingTokenDecimals = ConstU8<NATIVE_DECIMALS>;
	type BondingTokenId = HereLocationGetter;
	type FeePercentage = FeePercentage;
	type FeeRecipient = FeeRecipient;
	type FeeToken = Assets;
	type Id = PalletId;
	type PriceProvider = ConstPriceProvider;
	type RootId = RootId;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = MockRuntimeHoldReason;
	type Treasury = Treasury;
	type UsdDecimals = ConstU8<MOCK_FEE_ASSET_DECIMALS>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = GenesisConfig::<TestRuntime>::default().build_storage().unwrap();
	RuntimeGenesisConfig {
		assets: AssetsConfig {
			assets: vec![(mock_fee_asset_id(), 1, true, 100)],
			metadata: vec![(
				mock_fee_asset_id(),
				"Tether USD".to_string().into_bytes(),
				"USDT".to_string().into_bytes(),
				MOCK_FEE_ASSET_DECIMALS,
			)],
			accounts: vec![],
			next_asset_id: None,
		},
		..Default::default()
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	sp_io::TestExternalities::new(storage)
}
