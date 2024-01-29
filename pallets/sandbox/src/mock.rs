// // // Polimec Blockchain – https://www.polimec.org/
// // // Copyright (C) Polimec 2022. All rights reserved.
//
// // // The Polimec Blockchain is free software: you can redistribute it and/or modify
// // // it under the terms of the GNU General Public License as published by
// // // the Free Software Foundation, either version 3 of the License, or
// // // (at your option) any later version.
//
// // // The Polimec Blockchain is distributed in the hope that it will be useful,
// // // but WITHOUT ANY WARRANTY; without even the implied warranty of
// // // MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// // // GNU General Public License for more details.
//
// // // You should have received a copy of the GNU General Public License
// // // along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// use sp_std::collections::btree_map::BTreeMap;
// use frame_support::{
// 	parameter_types,
// 	traits::{AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32},
// 	PalletId,
// };
// use frame_system as system;
// use parachains_common::HOURS;
// use sp_arithmetic::{FixedU128, Percent};
// use sp_core::H256;
// use sp_runtime::{
// 	testing::Header,
// 	traits::{BlakeTwo256, IdentityLookup},
// 	BuildStorage,
// };
// use system::EnsureSigned;
//
// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<TestRuntime>;
// type Block = frame_system::mocking::MockBlock<TestRuntime>;
//
// pub type AccountId = u64;
// pub type Balance = u128;
// pub type BlockNumber = u64;
// pub type Identifier = u32;
// pub type AssetId = u32;
// pub const PLMC: u128 = 10_000_000_000_u128;
// const US_DOLLAR: u128 = 1_0_000_000_000u128;
//
// // Configure a mock runtime to test the pallet.
// frame_support::construct_runtime!(
// 	pub enum TestRuntime where
// 		Block = Block,
// 		NodeBlock = Block,
// 		UncheckedExtrinsic = UncheckedExtrinsic,
// 	{
// 		System: frame_system,
// 		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
// 		Assets: pallet_assets,
// 		Balances: pallet_balances,
// 		PolimecFunding: pallet_funding,
// 		Sandbox: crate,
// 	}
// );
//
// impl crate::Config for TestRuntime {}
//
// parameter_types! {
// 	pub const BlockHashCount: u32 = 250;
// }
//
// impl system::Config for TestRuntime {
// 	type AccountData = pallet_balances::AccountData<Balance>;
// 	type AccountId = AccountId;
// 	type BaseCallFilter = frame_support::traits::Everything;
// 	type BlockHashCount = BlockHashCount;
// 	type BlockLength = ();
// 	type BlockNumber = BlockNumber;
// 	type BlockWeights = ();
// 	type DbWeight = ();
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type Header = Header;
// 	type Index = u64;
// 	type Lookup = IdentityLookup<AccountId>;
// 	type MaxConsumers = frame_support::traits::ConstU32<16>;
// 	type OnKilledAccount = ();
// 	type OnNewAccount = ();
// 	type OnSetCode = ();
// 	type PalletInfo = PalletInfo;
// 	type RuntimeCall = RuntimeCall;
// 	type RuntimeEvent = RuntimeEvent;
// 	type RuntimeOrigin = RuntimeOrigin;
// 	type SS58Prefix = ConstU16<42>;
// 	type SystemWeightInfo = ();
// 	type Version = ();
// }
//
// parameter_types! {
// 	pub static ExistentialDeposit: Balance = 1;
// }
//
// impl pallet_balances::Config for TestRuntime {
// 	type AccountStore = System;
// 	type Balance = Balance;
// 	type DustRemoval = ();
// 	type ExistentialDeposit = ExistentialDeposit;
// 	type FreezeIdentifier = pallet_funding::types::LockType<u32>;
// 	type HoldIdentifier = pallet_funding::types::LockType<u32>;
// 	type MaxFreezes = ();
// 	type MaxHolds = ();
// 	type MaxLocks = frame_support::traits::ConstU32<1024>;
// 	type MaxReserves = frame_support::traits::ConstU32<1024>;
// 	type ReserveIdentifier = pallet_funding::types::LockType<u32>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }
//
// impl pallet_insecure_randomness_collective_flip::Config for TestRuntime {}
//
// impl pallet_assets::Config for TestRuntime {
// 	type ApprovalDeposit = ConstU128<1>;
// 	type AssetAccountDeposit = ConstU128<10>;
// 	type AssetDeposit = ConstU128<1>;
// 	type AssetId = Identifier;
// 	type AssetIdParameter = Identifier;
// 	type Balance = Balance;
// 	#[cfg(feature = "runtime-benchmarks")]
// 	type BenchmarkHelper = ();
// 	type CallbackHandle = ();
// 	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
// 	type Currency = Balances;
// 	type Extra = ();
// 	type ForceOrigin = frame_system::EnsureRoot<u64>;
// 	type Freezer = ();
// 	type MetadataDepositBase = ConstU128<1>;
// 	type MetadataDepositPerByte = ConstU128<1>;
// 	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
// 	type RuntimeEvent = RuntimeEvent;
// 	type StringLimit = ConstU32<50>;
// 	type WeightInfo = ();
// }
//
// // REMARK: In the production configuration we use DAYS instead of HOURS.
// parameter_types! {
// 	pub const EvaluationDuration: BlockNumber = (28 * HOURS) as BlockNumber;
// 	pub const AuctionInitializePeriodDuration: BlockNumber = (7 * HOURS) as BlockNumber;
// 	pub const EnglishAuctionDuration: BlockNumber = (2 * HOURS) as BlockNumber;
// 	pub const CandleAuctionDuration: BlockNumber = (3 * HOURS) as BlockNumber;
// 	pub const CommunityRoundDuration: BlockNumber = (5 * HOURS) as BlockNumber;
// 	pub const RemainderFundingDuration: BlockNumber = (1 * HOURS) as BlockNumber;
// 	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
// 	pub const ManualAcceptanceDuration: BlockNumber = (3 * HOURS) as BlockNumber;
// 	pub const SuccessToSettlementTime: BlockNumber =(4 * HOURS) as BlockNumber;
// 	pub PriceMap: BTreeMap<AssetId, FixedU128> = BTreeMap::from_iter(vec![
// 		(0u32, FixedU128::from_float(69f64)), // DOT
// 		(420u32, FixedU128::from_float(0.97f64)), // USDC
// 		(1984u32, FixedU128::from_float(0.95f64)), // USDT
// 		(2069u32, FixedU128::from_float(8.4f64)), // PLMC
// 	]);
// 	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
// 		(Percent::from_percent(10), 1_000_000 * US_DOLLAR),
// 		(Percent::from_percent(8), 5_000_000 * US_DOLLAR),
// 		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
// 	];
// 	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
// }
//
// impl pallet_funding::Config for TestRuntime {
// 	type AuctionInitializePeriodDuration = AuctionInitializePeriodDuration;
// 	type Balance = Balance;
// 	#[cfg(feature = "runtime-benchmarks")]
// 	type BenchmarkHelper = ();
// 	type CandleAuctionDuration = CandleAuctionDuration;
// 	type CommunityFundingDuration = CommunityRoundDuration;
// 	type ContributionTokenCurrency = ();
// 	type ContributionVesting = ConstU32<4>;
// 	type EnglishAuctionDuration = EnglishAuctionDuration;
// 	type EvaluationDuration = EvaluationDuration;
// 	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
// 	type FeeBrackets = FeeBrackets;
// 	type FundingCurrency = ();
// 	type ManualAcceptanceDuration = ManualAcceptanceDuration;
// 	// Low values to simplify the tests
// 	type MaxBidsPerUser = ConstU32<4>;
// 	type MaxContributionsPerUser = ConstU32<4>;
// 	type MaxEvaluationsPerUser = ConstU32<4>;
// 	type MaxProjectsToUpdatePerBlock = ConstU32<100>;
// 	type Multiplier = ();
// 	type NativeCurrency = Balances;
// 	type PalletId = FundingPalletId;
// 	type PreImageLimit = ConstU32<1024>;
// 	type Price = FixedU128;
// 	type PriceProvider = ();
// 	type ProjectIdentifier = Identifier;
// 	type Randomness = RandomnessCollectiveFlip;
// 	type RemainderFundingDuration = RemainderFundingDuration;
// 	type RuntimeEvent = RuntimeEvent;
// 	type StringLimit = ConstU32<64>;
// 	type SuccessToSettlementTime = SuccessToSettlementTime;
// 	type Vesting = ();
// 	type WeightInfo = ();
// }
//
// // Build genesis storage according to the mock runtime.
// #[allow(dead_code)]
// pub fn new_test_ext() -> sp_io::TestExternalities {
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();
//
// 	GenesisConfig { balances: BalancesConfig { balances: vec![] }, ..Default::default() }
// 		.assimilate_storage(&mut t)
// 		.unwrap();
//
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	// In order to emit events the block number must be more than 0
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }
//
// pub fn hashed(data: impl AsRef<[u8]>) -> H256 {
// 	<BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
// }
