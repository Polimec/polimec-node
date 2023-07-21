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

use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32},
	PalletId,
};
use frame_system as system;
use parachains_common::HOURS;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use system::EnsureSigned;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
type Block = frame_system::mocking::MockBlock<TestRuntime>;

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type Identifier = u32;
pub const PLMC: u128 = 10_000_000_000_u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum TestRuntime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Assets: pallet_assets,
		Balances: pallet_balances,
		FundingModule: pallet_funding,
		Sandbox: crate,
	}
);

impl crate::Config for TestRuntime {}

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
	pub static ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for TestRuntime {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = frame_support::traits::ConstU32<1024>;
	type ReserveIdentifier = pallet_funding::BondType;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type HoldIdentifier = pallet_funding::BondType;
	type FreezeIdentifier = pallet_funding::BondType;
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}

impl pallet_assets::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = Identifier;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type AssetDeposit = ConstU128<1>;
	type AssetAccountDeposit = ConstU128<10>;
	type MetadataDepositBase = ConstU128<1>;
	type MetadataDepositPerByte = ConstU128<1>;
	type ApprovalDeposit = ConstU128<1>;
	type StringLimit = ConstU32<50>;
	type Freezer = ();
	type WeightInfo = ();
	type Extra = ();
	type AssetIdParameter = Identifier;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type CallbackHandle = ();
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// REMARK: In the production configuration we use DAYS instead of HOURS.
parameter_types! {
	pub const EvaluationDuration: BlockNumber = (28 * HOURS) as BlockNumber;
	pub const AuctionInitializePeriodDuration: BlockNumber = (7 * HOURS) as BlockNumber;
	pub const EnglishAuctionDuration: BlockNumber = (2 * HOURS) as BlockNumber;
	pub const CandleAuctionDuration: BlockNumber = (3 * HOURS) as BlockNumber;
	pub const CommunityRoundDuration: BlockNumber = (5 * HOURS) as BlockNumber;
	pub const RemainderFundingDuration: BlockNumber = (1 * HOURS) as BlockNumber;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
}

impl pallet_funding::Config for TestRuntime {
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = ConstU32<64>;
	type ProjectIdentifier = Identifier;
	type ProjectIdParameter = Identifier;
	type BidId = u128;
	type ContributionTokenCurrency = Assets;
	type EvaluationDuration = EvaluationDuration;
	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type CandleAuctionDuration = CandleAuctionDuration;
	type RemainderFundingDuration = RemainderFundingDuration;
	type PalletId = FundingPalletId;
	type MaxProjectsToUpdatePerBlock = ConstU32<100>;
	type CommunityFundingDuration = CommunityRoundDuration;
	type Randomness = RandomnessCollectiveFlip;
	type PreImageLimit = ConstU32<1024>;
	// Low value to simplify the tests
	type MaximumBidsPerUser = ConstU32<4>;
	type MaxContributionsPerUser = ConstU32<4>;
	type ContributionVesting = ConstU32<4>;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type Multiplier = pallet_funding::types::Multiplier<TestRuntime>;
	type Balance = Balance;
	type NativeCurrency = Balances;
	type FundingCurrency = Balances;
}

// Build genesis storage according to the mock runtime.
// TODO: PLMC-161. Add some mocks projects at Genesis to simplify the tests
#[allow(dead_code)]
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

	GenesisConfig { balances: BalancesConfig { balances: vec![] }, ..Default::default() }
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
