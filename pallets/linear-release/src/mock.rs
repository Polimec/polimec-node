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

use super::*;
use crate::{self as pallet_vesting};
use frame_support::{
	derive_impl, parameter_types,
	traits::{VariantCount, WithdrawReasons},
};
use sp_runtime::{
	traits::{Identity, IdentityLookup},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Call, Config<T>, Storage, Event<T>},
		Vesting: pallet_vesting::{Call, Storage, Event<T>}
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = u64;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub static ExistentialDeposit: u64 = 10u64.pow(7);
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig as pallet_balances::DefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
	type FreezeIdentifier = RuntimeHoldReason;
	type RuntimeHoldReason = MockRuntimeHoldReason;
}

parameter_types! {
	pub BenchmarkReason: MockRuntimeHoldReason = MockRuntimeHoldReason::Reason;

}

impl Config for Test {
	type Balance = u64;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkReason = BenchmarkReason;
	type BlockNumberProvider = System;
	type BlockNumberToBalance = Identity;
	type Currency = Balances;
	// TODO: Use the type from Balances.
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = MockRuntimeHoldReason;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 3;
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
	DecodeWithMemTracking,
)]
pub enum MockRuntimeHoldReason {
	Reason,
	Reason2,
}
impl VariantCount for MockRuntimeHoldReason {
	const VARIANT_COUNT: u32 = 2;
}

#[derive(Default)]
pub struct ExtBuilder {
	existential_deposit: u64,
	vesting_genesis_config: Option<Vec<(u64, u64, u64, u64, MockRuntimeHoldReason)>>,
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10 * self.existential_deposit),
				(2, 21 * self.existential_deposit),
				(3, 30 * self.existential_deposit),
				(4, 40 * self.existential_deposit),
				(12, 10 * self.existential_deposit),
				(13, 9999 * self.existential_deposit),
				(14, 2000 * self.existential_deposit),
			],
			dev_accounts: None,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| {
			System::set_block_number(1);
			let vesting = if let Some(vesting_config) = self.vesting_genesis_config {
				vesting_config
			} else {
				vec![
					(1, 0, 10, 5 * self.existential_deposit, MockRuntimeHoldReason::Reason),
					(2, 10, 20, self.existential_deposit, MockRuntimeHoldReason::Reason),
					(12, 10, 20, 5 * self.existential_deposit, MockRuntimeHoldReason::Reason),
				]
			};
			for &(ref who, begin, length, liquid, reason) in vesting.iter() {
				let balance = Balances::balance(who);
				assert!(!balance.is_zero(), "Currencies must be init'd before vesting");
				// Total genesis `balance` minus `liquid` equals funds locked for vesting
				let locked = balance.saturating_sub(liquid);
				let length_as_balance = Identity::convert(length);
				let per_block = locked / length_as_balance.max(sp_runtime::traits::One::one());
				let vesting_info = VestingInfo::new(locked, per_block, begin);
				if !vesting_info.is_valid() {
					panic!("Invalid VestingInfo params at genesis")
				};

				crate::pallet::Vesting::<Test>::try_append(who, reason, vesting_info)
					.expect("Too many vesting schedules at genesis.");

				Balances::hold(&reason, who, locked).map_err(|err| panic!("{:?}", err)).unwrap();
			}
		});

		ext
	}
}
