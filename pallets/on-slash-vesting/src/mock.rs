use frame_support::{
	__private::RuntimeDebug,
	derive_impl, parameter_types,
	sp_runtime::{traits::IdentityLookup, BuildStorage},
	traits::{VariantCount, WithdrawReasons},
};
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::traits::ConvertInto;

frame_support::construct_runtime!(
	pub enum TestRuntime {
		System: frame_system = 0,
		Balances: pallet_balances = 1,
		Vesting: pallet_vesting = 2,
	}
);
type Block = frame_system::mocking::MockBlock<TestRuntime>;

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

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for TestRuntime {
	type AccountData = pallet_balances::AccountData<u128>;
	type AccountId = u64;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 10;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub static ExistentialDeposit: u128 = 10u128.pow(7);
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for TestRuntime {
	type AccountStore = System;
	type Balance = u128;
	type ExistentialDeposit = ExistentialDeposit;
	type RuntimeHoldReason = MockRuntimeHoldReason;
}

impl pallet_vesting::Config for TestRuntime {
	type BlockNumberProvider = System;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 6;
}

#[derive(Default)]
pub struct ExtBuilder {
	pub existential_deposit: u128,
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();
		pallet_balances::GenesisConfig::<TestRuntime> {
			balances: vec![
				(1, self.existential_deposit),
				(2, self.existential_deposit),
				(3, self.existential_deposit),
				(4, self.existential_deposit),
			],
			dev_accounts: None,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		sp_io::TestExternalities::new(t)
	}
}
