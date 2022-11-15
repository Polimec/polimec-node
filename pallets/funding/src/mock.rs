use crate as pallet_funding;
use frame_support::{pallet_prelude::ConstU32, parameter_types, traits::ConstU16, PalletId};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage,
};
use system::EnsureSigned;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip,
		Balances: pallet_balances,
		FundingModule: pallet_funding,
		Credentials: pallet_credentials
	}
);

parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
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

impl pallet_balances::Config for Test {
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_randomness_collective_flip::Config for Test {}

impl pallet_credentials::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureSigned<AccountId>;
	type RemoveOrigin = EnsureSigned<AccountId>;
	type SwapOrigin = EnsureSigned<AccountId>;
	type ResetOrigin = EnsureSigned<AccountId>;
	type PrimeOrigin = EnsureSigned<AccountId>;
	type MembershipInitialized = ();
	type MembershipChanged = ();
	type MaxMembersCount = ConstU32<255>;
}

parameter_types! {
	pub const EvaluationDuration: BlockNumber = 28;
	pub const EnglishAuctionDuration: BlockNumber = 10;
	pub const CandleAuctionDuration: BlockNumber = 5;
	pub const CommunityRoundDuration: BlockNumber = 10;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
}

impl pallet_funding::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = ConstU32<64>;
	type Currency = Balances;
	type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
	type EvaluationDuration = EvaluationDuration;
	type EnglishAuctionDuration = EnglishAuctionDuration;
	type CandleAuctionDuration = CandleAuctionDuration;
	type PalletId = FundingPalletId;
	type ActiveProjectsLimit = ConstU32<100>;
	type CommunityRoundDuration = CommunityRoundDuration;
	type Randomness = RandomnessCollectiveFlip;
	type HandleMembers = Credentials;
}

// Build genesis storage according to the mock runtime.
// TODO: Add some mocks projects at Genesis to simplify the tests
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	GenesisConfig {
		balances: BalancesConfig {
			balances: vec![(1, 512), (2, 512), (3, 512), (4, 512), (5, 512)],
		},
		credentials: CredentialsConfig {
			issuers: vec![1],
			retails: vec![2],
			professionals: vec![3],
			institutionals: vec![4],
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
