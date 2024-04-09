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

use frame_support::{derive_impl, ord_parameter_types, parameter_types, traits::tokens::WithdrawReasons, PalletId};
use frame_system as system;
use frame_system::EnsureSignedBy;
use polimec_common::credentials::EnsureInvestor;
use sp_runtime::{traits::ConvertInto, BuildStorage};

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
	type BlockNumberProvider = System;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

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
	type InitialClaimAmount = InitialClaimAmount;
	type InvestorOrigin = EnsureInvestor<Test>;
	type LockPeriod = LockPeriod;
	type PalletId = FaucetPalletId;
	type RuntimeEvent = RuntimeEvent;
	type VerifierPublicKey = VerifierPublicKey;
	type VestPeriod = VestPeriod;
	type VestingSchedule = Vesting;
	type WeightInfo = ();
}

pub(crate) struct ExtBuilder {
	// amount of account that can claim tokens
	claiming_accounts: u64,
}

impl Default for ExtBuilder {
	fn default() -> ExtBuilder {
		ExtBuilder { claiming_accounts: 1 }
	}
}

impl ExtBuilder {
	pub(crate) fn claiming_account(mut self, amount: u64) -> Self {
		self.claiming_accounts = amount;
		self
	}

	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = system::GenesisConfig::<Test>::default()
			.build_storage()
			.expect("Frame system builds valid default genesis config");
		let faucet_filled = vec![(
			Faucet::claiming_account(),
			self.claiming_accounts * <Test as crate::Config>::InitialClaimAmount::get(),
		)];
		pallet_balances::GenesisConfig::<Test> { balances: faucet_filled }
			.assimilate_storage(&mut t)
			.expect("Pallet balances storage can be assimilated");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
