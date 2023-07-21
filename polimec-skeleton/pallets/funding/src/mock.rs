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
use sp_std::collections::btree_map::BTreeMap;

use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU32},
	PalletId,
};
use frame_system as system;
use frame_system::EnsureRoot;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, ConvertInto},
	BuildStorage,
};
use system::EnsureSigned;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
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
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Balances: pallet_balances,
		FundingModule: pallet_funding,
		Credentials: pallet_credentials,
		Vesting: pallet_linear_release,
		LocalAssets: pallet_assets::<Instance1>::{Pallet, Call, Storage, Event<T>},
		StatemintAssets: pallet_assets::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},
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
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
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

impl pallet_assets::Config<StatemintAssetsInstance> for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type RemoveItemsLimit = ConstU32<1000>;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
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
parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

impl system::Config for TestRuntime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}
impl pallet_balances::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type ReserveIdentifier = LockType<u32>;
	type HoldIdentifier = LockType<u32>;
	type FreezeIdentifier = ();
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = frame_support::traits::ConstU32<1024>;
	type MaxHolds = ConstU32<1024>;
	type MaxFreezes = ();
}

impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}

impl pallet_credentials::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureSigned<AccountId>;
	type RemoveOrigin = EnsureSigned<AccountId>;
	type SwapOrigin = EnsureSigned<AccountId>;
	type ResetOrigin = EnsureSigned<AccountId>;
	type PrimeOrigin = EnsureSigned<AccountId>;
	type MembershipInitialized = ();
	type MembershipChanged = ();
}
pub const HOURS: BlockNumber = 300u64;

// REMARK: In the production configuration we use DAYS instead of HOURS.
parameter_types! {
	pub const EvaluationDuration: BlockNumber = (28 * HOURS) as BlockNumber;
	pub const AuctionInitializePeriodDuration: BlockNumber = (7 * HOURS) as BlockNumber;
	pub const EnglishAuctionDuration: BlockNumber = (2 * HOURS) as BlockNumber;
	pub const CandleAuctionDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const CommunityRoundDuration: BlockNumber = (5 * HOURS) as BlockNumber;
	pub const RemainderFundingDuration: BlockNumber = (1 * HOURS) as BlockNumber;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
	pub PriceMap: BTreeMap<AssetId, FixedU128> = BTreeMap::from_iter(vec![
		(0u32, FixedU128::from_float(69f64)), // DOT
		(420u32, FixedU128::from_float(0.97f64)), // USDC
		(1984u32, FixedU128::from_float(0.95f64)), // USDT
		(2069u32, FixedU128::from_float(8.4f64)), // PLMC
	]);
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * US_DOLLAR),
		(Percent::from_percent(8), 5_000_000 * US_DOLLAR),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
}

use frame_support::traits::WithdrawReasons;

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}
impl pallet_linear_release::Config for TestRuntime {
	type Balance = Balance;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();
	type Reason = LockType<u32>;
	const MAX_VESTING_SCHEDULES: u32 = 3;
}

impl pallet_funding::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type ProjectIdentifier = Identifier;
	type Multiplier = Multiplier<TestRuntime>;
	type Balance = Balance;
	type Price = FixedU128;
	type NativeCurrency = Balances;
	type FundingCurrency = StatemintAssets;
	type ContributionTokenCurrency = LocalAssets;
	type PriceProvider = ConstPriceProvider<AssetId, FixedU128, PriceMap>;
	type StorageItemId = u128;
	type Randomness = RandomnessCollectiveFlip;
	type HandleMembers = Credentials;
	type StringLimit = ConstU32<64>;
	type PreImageLimit = ConstU32<1024>;
	type EvaluationDuration = EvaluationDuration;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type CandleAuctionDuration = CandleAuctionDuration;
	type CommunityFundingDuration = CommunityRoundDuration;
	type RemainderFundingDuration = RemainderFundingDuration;
	type PalletId = FundingPalletId;
	type MaxProjectsToUpdatePerBlock = ConstU32<100>;
	type MaxEvaluationsPerUser = ConstU32<4>;
	// Low value to simplify the tests
	type MaxBidsPerUser = ConstU32<4>;
	type MaxContributionsPerUser = ConstU32<4>;
	type ContributionVesting = ConstU32<4>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type WeightInfo = ();
	type FeeBrackets = FeeBrackets;
	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
	type Vesting = Vesting;
}

// Build genesis storage according to the mock runtime.
// TODO: PLMC-161. Add some mocks projects at Genesis to simplify the tests
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<TestRuntime>()
		.unwrap();

	GenesisConfig {
		balances: BalancesConfig {
			balances: vec![(
				<TestRuntime as Config>::PalletId::get().into_account_truncating(),
				<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get(),
			)],
		},
		credentials: CredentialsConfig {
			issuers: vec![1, 16558220937623665250],
			retails: vec![2],
			professionals: vec![2, 3],
			institutionals: vec![4],
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
