use frame_support::{PalletId, traits::tokens::WithdrawReasons, derive_impl, parameter_types, ord_parameter_types};
use frame_system as system;
use frame_system::EnsureSignedBy;
use sp_runtime::{
	traits::{ConvertInto},
	BuildStorage,
};
use polimec_common::credentials::{EnsureInvestor, Retail};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Vesting: pallet_vesting,
		Faucet: crate::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}



#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = AccountId;
	type Block = Block;
}

#[derive_impl(pallet_timestamp::config_preludes::TestDefaultConfig as pallet_timestamp::DefaultConfig)]
impl pallet_timestamp::Config for Test {}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 0;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
			WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
	pub const InitialClaimAmount: u64 = 100;
	pub const LockPeriod: u64 = 10;
	pub const FaucetPalletId: PalletId = PalletId(*b"plmc/fct");
	pub const VestPeriod: u64 = 10;
	pub VerifierPublicKey: [u8; 32] = [
		32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253,
		195, 18, 202, 111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
	];
}

ord_parameter_types! {
	pub const Admin: u64 = 666;
}

impl crate::Config for Test {
	type AdminOrigin = EnsureSignedBy<Admin, AccountId>;
	type BlockNumberToBalance = ConvertInto;
	type PalletId = FaucetPalletId;
	type InitialClaimAmount = InitialClaimAmount;
	type InvestorOrigin = EnsureInvestor<Test, (), Retail>;
	type LockPeriod = LockPeriod;
	type RuntimeEvent = RuntimeEvent;
	type VerifierPublicKey = VerifierPublicKey;
	type VestPeriod = VestPeriod;
	type VestingSchedule = Vesting;
	type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
