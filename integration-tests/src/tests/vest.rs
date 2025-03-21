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

/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use crate::{polimec::ED, *};
use cumulus_pallet_parachain_system::ValidationData;
use cumulus_primitives_core::PersistedValidationData;
use frame_support::traits::fungible::{Inspect, Mutate};
use macros::generate_accounts;
use pallet_funding::assert_close_enough;
use pallet_vesting::VestingInfo;
use polimec_runtime::{Balances, BlockchainOperationTreasury, ParachainStaking, RuntimeOrigin, Vesting, PLMC};
use sp_core::crypto::get_public_from_string_or_panic;
use sp_runtime::{Perquintill, SaturatedConversion};

generate_accounts!(PEPE, CARLOS);

fn set_relay_chain_block_number<T: cumulus_pallet_parachain_system::Config>(to: u32) {
	let mut validation_data = ValidationData::<T>::get().unwrap_or_else(||
		// PersistedValidationData does not impl default in non-std
		PersistedValidationData {
			parent_head: vec![].into(),
			relay_parent_number: Default::default(),
			max_pov_size: Default::default(),
			relay_parent_storage_root: Default::default(),
		});
	validation_data.relay_parent_number = to.saturated_into();
	ValidationData::<T>::put(validation_data)
}

#[test]
fn base_vested_can_stake() {
	PolimecNet::execute_with(|| {
		let alice = PolimecNet::account_id_of(ALICE);
		let coll_1: AccountId = get_public_from_string_or_panic::<sr25519::Public>("COLL_1").into();
		let new_account = get_public_from_string_or_panic::<sr25519::Public>("NEW_ACCOUNT").into();

		// Initially the NEW_ACCOUNT has no PLMC
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);

		// Stake 60 PLMC from "new_account" to "COLL_1", it should fail since the account has no PLMC
		assert_noop!(
			ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), coll_1.clone(), 60 * PLMC, 0, 0),
			pallet_parachain_staking::Error::<polimec_runtime::Runtime>::InsufficientBalance
		);

		// Create a vesting schedule for 60 PLMC + ED over 60 blocks (~1 PLMC per block) to NEW_ACCOUNT
		let vesting_schedule = VestingInfo::new(
			60 * PLMC + ED,
			PLMC, // Vesting over 60 blocks
			1,
		);
		// The actual vested transfer
		assert_ok!(Vesting::vested_transfer(
			RuntimeOrigin::signed(alice.clone()),
			sp_runtime::MultiAddress::Id(new_account.clone()),
			vesting_schedule
		));

		// Alice now has 360 PLMC left
		assert_eq!(Balances::balance(&alice), 360 * PLMC - ED);

		// "New Account" has 60 free PLMC, using fungible::Inspect
		assert_eq!(Balances::balance(&new_account), 60 * PLMC + ED);

		// Stake 60 PLMC from "new_account" to "COLL_1", it should go through since the account has 60 + ED free PLMC
		assert_ok!(ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), coll_1, 60 * PLMC, 0, 0));
		// "New Account" only has ED free PLMC, using fungible::Inspect, since staking applies a `Hold` (which includes frozen balance)
		assert_eq!(Balances::balance(&new_account), ED);

		// Check that the staking state is correct
		ParachainStaking::delegator_state(&new_account).map(|state| {
			assert_eq!(state.total, 60 * PLMC);
			assert_eq!(state.delegations.0.len(), 1);
		});
	})
}

// It happened that the original struct that withdrew the free, didn't consider the held balance as part of the
// total balance, so if the user had 20 free, 2000 frozen, 2000 held, then the user could only withdraw any amount over 2000.
#[test]
fn base_can_withdraw_when_free_is_below_frozen_with_hold() {
	PolimecNet::execute_with(|| {
		let coll_1 = get_public_from_string_or_panic::<sr25519::Public>("COLL_1").into();
		Balances::set_balance(&PEPE.into(), 2_020 * PLMC + ED * 2);
		Balances::set_balance(&CARLOS.into(), 0);

		// Vesting schedule for PEPE of 20k PLMC + ED, which should have start date before it is applied
		let vesting_schedule = VestingInfo::new(2_020 * PLMC, 10 * PLMC, 0);

		set_relay_chain_block_number::<PolimecRuntime>(1);

		// The actual vested transfer
		assert_ok!(Vesting::vested_transfer(
			RuntimeOrigin::signed(PEPE.into()),
			sp_runtime::MultiAddress::Id(CARLOS.into()),
			vesting_schedule
		));

		// Vested transfer didnt start with the full amount locked, since start date was before execution
		assert_eq!(Balances::usable_balance(&CARLOS.into()), 10 * PLMC);

		let carlos_acc: AccountId = CARLOS.into();

		// PEPE stakes his 20k PLMC, even if most of it is locked (frozen)
		assert_ok!(ParachainStaking::delegate(RuntimeOrigin::signed(CARLOS.into()), coll_1, 2_000 * PLMC, 0, 0));

		// Check that the staking state is correct
		ParachainStaking::delegator_state(carlos_acc).map(|state| {
			assert_eq!(state.total, 2_000 * PLMC);
			assert_eq!(state.delegations.0.len(), 1);
		});

		// Even if we still didn't vest the other 10 PLMC, the .free balance is reduced from 2020PLMC to 20PLMC when staking 2000 PLMC
		let free_balance = Balances::free_balance(&CARLOS.into());
		assert_eq!(free_balance, 20 * PLMC);

		// Transferable balance is 10 PLMC due to setting vesting schedule before execution block. Need it for fees
		assert_eq!(Balances::usable_balance(&CARLOS.into()), 10 * PLMC);

		// Be able to vest 10 more PLMC for this example description
		set_relay_chain_block_number::<PolimecRuntime>(2);

		// This should pass if the fee is correctly deducted with the new fee struct
		assert_ok!(Vesting::vest(RuntimeOrigin::signed(CARLOS.into())));

		let usable_balance = Balances::usable_balance(&CARLOS.into());
		// we expect the real value to be at minimum 99% of the expected value, due to fees paid
		assert_close_enough!(usable_balance, 20 * PLMC, Perquintill::from_float(0.99));

		// Test transfer of the usable balance out of CARLOS
		assert_ok!(Balances::transfer_allow_death(
			RuntimeOrigin::signed(CARLOS.into()),
			sp_runtime::MultiAddress::Id(PEPE.into()),
			usable_balance
		));
		assert_eq!(Balances::usable_balance(&CARLOS.into()), 0);
		assert_eq!(Balances::free_balance(&CARLOS.into()), ED);
		assert_eq!(Balances::reserved_balance(&CARLOS.into()), 2_000 * PLMC);
	})
}

// Tests the behavior of transferring the dust to the Blockchain Operation Treasury.
// When an account's balance falls below the Existential Deposit (ED) threshold following a transfer,
// the account is killed and the dust is sent to the treasury.
#[test]
fn dust_to_treasury() {
	PolimecNet::execute_with(|| {
		// Create two new accounts: a sender and a receiver.
		let sender = get_public_from_string_or_panic::<sr25519::Public>("SENDER").into();
		let receiver = get_public_from_string_or_panic::<sr25519::Public>("RECEIVER").into();

		// Set the sender's initial balance to 1 PLMC.
		let initial_sender_balance = 1 * PLMC;
		Balances::set_balance(&sender, initial_sender_balance);

		// Get the total issuance and Treasury balance before the transfer.
		let initial_total_issuance = Balances::total_issuance();
		let initial_treasury_balance = Balances::free_balance(BlockchainOperationTreasury::get());

		// Transfer funds from sender to receiver, designed to deplete the sender's balance below the ED.
		// The sender account will be killed and the dust will be sent to the treasury.
		// This happens because at the end of the transfer, the user has free_balance < ED.
		assert_ok!(Balances::transfer_allow_death(
			RuntimeOrigin::signed(sender),
			sp_runtime::MultiAddress::Id(receiver),
			initial_sender_balance - ED + 1
		));

		// Confirm the total issuance remains unchanged post-transfer.
		let post_total_issuance = Balances::total_issuance();
		assert_eq!(initial_total_issuance, post_total_issuance);

		// Verify the Treasury has received the dust from the sender's account.
		let final_treasury_balance = Balances::free_balance(BlockchainOperationTreasury::get());
		assert_eq!(initial_treasury_balance + ED - 1, final_treasury_balance);
	})
}
