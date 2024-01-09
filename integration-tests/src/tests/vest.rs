use crate::{polimec_base::ED, *};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::fungible::{InspectHold, Mutate};
use macros::generate_accounts;
use pallet_funding::assert_close_enough;
use pallet_vesting::VestingInfo;
use polimec_base_runtime::{Balances, ParachainStaking, RuntimeOrigin, Vesting};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128, Perquintill};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;

generate_accounts!(PEPE, CARLOS,);

#[test]
fn base_vested_can_stake() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);
		let coll_1 = get_account_id_from_seed::<sr25519::Public>("COLL_1");
		let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");

		// Initially the NEW_ACCOUNT has no PLMC
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);

		// Stake 60 PLMC from "new_account" to "COLL_1", it should fail since the account has no PLMC
		assert_noop!(
			ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), coll_1.clone(), 60 * PLMC, 0, 0),
			pallet_parachain_staking::Error::<polimec_parachain_runtime::Runtime>::InsufficientBalance
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
	PolimecBase::execute_with(|| {
		let coll_1 = get_account_id_from_seed::<sr25519::Public>("COLL_1");
		Balances::set_balance(&PEPE.into(), 2_020 * PLMC + ED * 2);
		Balances::set_balance(&CARLOS.into(), 0);

		// Vesting schedule for PEPE of 20k PLMC + ED, which should have start date before it is applied
		let vesting_schedule = VestingInfo::new(2_020 * PLMC, 10 * PLMC, 0);

		assert_eq!(Balances::free_balance(&CARLOS.into()), 0);
		// We need some free balance at the time of the vested transfer
		// Otherwise the user will never have free balance to pay for the "vest" extrinsic
		PolimecSystem::set_block_number(1u32);

		// The actual vested transfer
		assert_ok!(Vesting::vested_transfer(
			RuntimeOrigin::signed(PEPE.into()),
			sp_runtime::MultiAddress::Id(CARLOS.into()),
			vesting_schedule
		));

		// Vested transfer didnt start with the full amount locked, since start date was befire execution
		assert_eq!(Balances::usable_balance(&CARLOS.into()), 10 * PLMC);

		let carlos_acc: PolimecAccountId = CARLOS.into();

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
		PolimecSystem::set_block_number(2u32);

		// This should pass if the fee is correctly deducted with the new fee struct
		assert_ok!(Vesting::vest(RuntimeOrigin::signed(CARLOS.into())));

		let usable_balance = Balances::usable_balance(&CARLOS.into());
		// we expect the real value to be at minimum 99% of the expected value, due to fees paid
		assert_close_enough!(usable_balance, 20 * PLMC, Perquintill::from_percent(1));

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