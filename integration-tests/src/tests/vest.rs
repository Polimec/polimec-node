use crate::*;
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::fungible::InspectHold;
use pallet_funding::LockType;
use pallet_linear_release::VestingInfo;
use polimec_parachain_runtime::{Balances, LinearVesting, ParachainStaking, RuntimeOrigin};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;

#[test]
fn can_start_vesting() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		let bob = Polimec::account_id_of(BOB);

		let sched1 = VestingInfo::new(
			PLMC * 5,
			PLMC, // Vesting over 5 blocks
			1,
		);
		assert_eq!(Balances::balance(&alice), 420 * PLMC);
		assert_eq!(Balances::balance(&bob), 420 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &alice), 0 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &bob), 0 * PLMC);

		assert_ok!(LinearVesting::vested_transfer(
			RuntimeOrigin::signed(alice.clone()),
			bob.clone(),
			sched1,
			LockType::Participation(0)
		));

		assert_eq!(Balances::balance(&alice), 415 * PLMC);
		assert_eq!(Balances::balance(&bob), 420 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &alice), 0 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &bob), 5 * PLMC);
	})
}

#[test]
fn todo_cannot_create_hold() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		let bob = Polimec::account_id_of(BOB);

		let sched1 = VestingInfo::new(
			PLMC * 20,
			PLMC, // Vesting over 20 blocks
			1,
		);

		let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &new_account), 0 * PLMC);

		// TODO: It looks like this is not working since the NEW_ACCOUNT has no balance yet.
		assert_ok!(LinearVesting::vested_transfer(
			RuntimeOrigin::signed(alice.clone()),
			new_account.clone(),
			sched1,
			LockType::Participation(0)
		));

		assert_eq!(Balances::balance(&alice), 415 * PLMC);
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &new_account), 0 * PLMC);
	})
}

#[test]
fn vested_can_stake() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		let bob = Polimec::account_id_of(BOB);
		let COLL_1 = get_account_id_from_seed::<sr25519::Public>("COLL_1");

		let sched1 = VestingInfo::new(
			PLMC * 20,
			PLMC, // Vesting over 20 blocks
			1,
		);

		let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &new_account), 0 * PLMC);

		// TEMP FIX:
		assert_ok!(Balances::transfer_allow_death(
			RuntimeOrigin::signed(alice.clone()),
			sp_runtime::MultiAddress::Id(new_account.clone()),
			PLMC * 51,
		));

		assert_ok!(LinearVesting::vested_transfer(
			RuntimeOrigin::signed(alice.clone()),
			new_account.clone(),
			sched1,
			LockType::Participation(0)
		));

		assert_eq!(Balances::balance(&alice), 349 * PLMC);
		assert_eq!(Balances::balance(&new_account), 51 * PLMC);
		assert_eq!(Balances::balance_on_hold(&LockType::Participation(0), &new_account), 20 * PLMC);

		// Stake 20 PLMC from "new_account" to "COLL_1"
		assert_ok!(ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), COLL_1, 70 * PLMC, 0, 0));
	})
}
