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

use frame_support::{assert_noop, assert_ok, assert_storage_noop, dispatch::EncodeLike};
use frame_system::RawOrigin;
use sp_runtime::{
	traits::{BadOrigin, Identity},
	TokenError,
};

use super::{Vesting as VestingStorage, *};
use crate::{
	mock::{Balances, ExtBuilder, System, Test, Vesting},
	types::LockType,
};

/// A default existential deposit.
const ED: u64 = 256;

/// Calls vest, and asserts that there is no entry for `account`
/// in the `Vesting` storage item.
fn vest_and_assert_no_vesting<T>(account: u64, reason: LockType<u32>)
where
	u64: EncodeLike<AccountIdOf<T>>,
	LockType<u32>: EncodeLike<ReasonOf<T>>,
	T: pallet::Config,
{
	// Its ok for this to fail because the user may already have no schedules.
	let _result: Result<(), DispatchError> = Vesting::vest(Some(account).into(), LockType::Participation(0));
	assert!(!<VestingStorage<T>>::contains_key(account, reason));
}

#[test]
fn check_vesting_status() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(&1);
		let user2_free_balance = Balances::free_balance(&2);
		let user12_free_balance = Balances::free_balance(&12);
		assert_eq!(user1_free_balance, ED * 5); // Account 1 has free balance
		assert_eq!(user2_free_balance, ED * 1); // Account 2 has free balance
		assert_eq!(user12_free_balance, ED * 5); // Account 12 has free balance
		let user1_vesting_schedule = VestingInfo::new(
			ED * 5,
			128, // Vesting over 10 blocks
			0,
		);
		let user2_vesting_schedule = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 19 blocks
			10,
		);
		let user12_vesting_schedule = VestingInfo::new(
			ED * 5,
			64, // Vesting over 20 blocks
			10,
		);
		assert_eq!(Vesting::vesting(&1, LockType::Participation(0)).unwrap(), vec![user1_vesting_schedule]); // Account 1 has a vesting schedule
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![user2_vesting_schedule]); // Account 2 has a vesting schedule
		assert_eq!(Vesting::vesting(&12, LockType::Participation(0)).unwrap(), vec![user12_vesting_schedule]); // Account 12 has a vesting schedule

		// Account 1 has only 128 units vested from their illiquid ED * 5 units at block 1
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(128 * 9));
		// Account 2 has their full balance locked
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(ED * 20));
		// Account 12 has only their illiquid funds locked
		assert_eq!(Vesting::vesting_balance(&12, LockType::Participation(0)), Some(ED * 5));

		System::set_block_number(10);
		assert_eq!(System::block_number(), 10);

		// Account 1 has fully vested by block 10
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(0));
		// Account 2 has started vesting by block 10
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(ED * 20));
		// Account 12 has started vesting by block 10
		assert_eq!(Vesting::vesting_balance(&12, LockType::Participation(0)), Some(ED * 5));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(0)); // Account 1 is still fully vested, and not negative
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(0)); // Account 2 has fully vested by block 30
		assert_eq!(Vesting::vesting_balance(&12, LockType::Participation(0)), Some(0)); // Account 2 has fully vested by block 30

		// Once we unlock the funds, they are removed from storage.
		vest_and_assert_no_vesting::<Test>(1, LockType::Participation(0));
		vest_and_assert_no_vesting::<Test>(2, LockType::Participation(0));
		vest_and_assert_no_vesting::<Test>(12, LockType::Participation(0));
	});
}

#[test]
fn check_vesting_status_for_multi_schedule_account() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			10,
		);
		// Account 2 already has a vesting schedule.
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		// Account 2's free balance is the one set in Genesis inside the Balances pallet.
		let balance = Balances::balance(&2);
		assert_eq!(balance, ED * (1));
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(ED * (20)));

		// Add a 2nd schedule that is already unlocking by block #1.
		let sched1 = VestingInfo::new(
			ED * 10,
			ED, // Vesting over 10 blocks
			0,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1, LockType::Participation(0)));
		// Free balance is the one set in Genesis inside the Balances pallet
		// + the one from the vested transfer.
		// BUT NOT the one in sched0, since the vesting will start at block #10.
		let balance = Balances::balance(&2);
		assert_eq!(balance, ED * (2));
		// The most recently added schedule exists.
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1]);

		// Add a 3rd schedule.
		let sched2 = VestingInfo::new(
			ED * 30,
			ED, // Vesting over 30 blocks
			5,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched2, LockType::Participation(0)));

		System::set_block_number(9);
		assert_eq!(System::block_number(), 9);

		// Free balance is still the same.
		let balance = Balances::balance(&2);
		assert_eq!(balance, ED * (2));
		// Let's release some funds from sched1 and sched2.
		assert_ok!(Vesting::vest(Some(2).into(), LockType::Participation(0)));
		let balance = Balances::balance(&2);
		assert_eq!(balance, ED * (1 + 9 + 4)); // 1 from Genesis + 9 from sched1 + 4 from sched2

		// sched1 and sched2 are freeing funds at block #9, but sched0 is not.
		assert_eq!(
			Vesting::vesting_balance(&2, LockType::Participation(0)),
			Some(sched0.per_block() * 20 + sched1.per_block() * 1 + sched2.per_block() * 26)
		);

		System::set_block_number(20);
		// At block #20 sched1 is fully unlocked while sched2 and sched0 are partially unlocked.
		assert_eq!(
			Vesting::vesting_balance(&2, LockType::Participation(0)),
			Some(sched0.per_block() * 10 + sched2.per_block() * 15)
		);

		System::set_block_number(30);
		// At block #30 sched0 and sched1 are fully unlocked while sched2 is partially unlocked.
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(sched2.per_block() * 5));

		// At block #35 sched2 fully unlocks and thus all schedules funds are unlocked.
		System::set_block_number(35);
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(0));
		// Since we have not called any extrinsics that would unlock funds the schedules
		// are still in storage,
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1, sched2]);
		// but once we unlock the funds, they are removed from storage.
		vest_and_assert_no_vesting::<Test>(2, LockType::Participation(0));
	});
}

#[test]
fn unvested_balance_should_not_transfer() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 50); // Account 1 has free balance
									// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(45)); // Account 1 cannot send more than vested amount...
		assert_noop!(Balances::transfer_allow_death(Some(1).into(), 2, 56), TokenError::FundsUnavailable);
	});
}

#[test]
fn vested_balance_should_transfer() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 50); // Account 1 has free balance
									// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(45));
		assert_noop!(Balances::transfer_allow_death(Some(1).into(), 2, 45), TokenError::Frozen); // Account 1 free balance - ED is < 45
		assert_ok!(Vesting::vest(Some(1).into(), LockType::Participation(0)));
		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 55); // Account 1 has free balance
									// Account 1 has vested 1 unit at block 1 (plus 50 unvested)
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 45)); // After the vest it can now send the 45 UNIT
	});
}

#[test]
fn vested_balance_should_transfer_with_multi_sched() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Prep
		assert_eq!(System::block_number(), 1);
		let sched0 = VestingInfo::new(5 * ED, 128, 0);
		let user1_initial_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_initial_free_balance, 1280); // Account 1 has free balance

		// Account "13" checks
		let user13_initial_free_balance = Balances::balance(&13);
		// Amount set in Genesis
		assert_eq!(user13_initial_free_balance, 2559744);
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 1, sched0, LockType::Participation(0)));
		let user13_free_balance = Balances::balance(&13);
		assert_eq!(user13_free_balance, user13_initial_free_balance - sched0.locked());

		// Account "1" has 2 release schedule applied now: one from the Genesis and one from the transfer
		assert_eq!(Vesting::vesting(&1, LockType::Participation(0)).unwrap(), vec![sched0, sched0]);
		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, user1_initial_free_balance + (2 * sched0.per_block()));

		//
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(9 * ED));
		assert_ok!(Vesting::vest(Some(1).into(), LockType::Participation(0)));
		let user1_free_balance = Balances::balance(&1);
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, user1_free_balance - ED));
	});
}

#[test]
fn non_vested_cannot_vest() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert!(!<VestingStorage<Test>>::contains_key(4, LockType::Participation(0)));
		assert_noop!(Vesting::vest(Some(4).into(), LockType::Participation(0)), Error::<Test>::NotVesting);
	});
}

#[test]
fn vested_balance_should_transfer_using_vest_other() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 50); // Account 1 has free balance
									// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(45));
		assert_ok!(Vesting::vest_other(Some(2).into(), 1, LockType::Participation(0)));
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 55 - 10));
	});
}

#[test]
fn vested_balance_should_transfer_using_vest_other_with_multi_sched() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = VestingInfo::new(5 * ED, 128, 0);
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 1, sched0, LockType::Participation(0)));
		// Total of 10*ED of locked for all the schedules.
		assert_eq!(Vesting::vesting(&1, LockType::Participation(0)).unwrap(), vec![sched0, sched0]);

		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 1536); // Account 1 has free balance

		// Account 1 has only 256 units unlocking at block 1 (plus 1280 already free).
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(9 * ED));
		assert_ok!(Vesting::vest_other(Some(2).into(), 1, LockType::Participation(0)));
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 1536 - ED));
	});
}

#[test]
fn non_vested_cannot_vest_other() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert!(!<VestingStorage<Test>>::contains_key(4, LockType::Participation(0)));
		assert_noop!(Vesting::vest_other(Some(3).into(), 4, LockType::Participation(0)), Error::<Test>::NotVesting);
	});
}

#[test]
fn extra_balance_should_transfer() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		// Account 1 and 2 cannot send more than their free balance
		assert_noop!(Balances::transfer_allow_death(Some(2).into(), 3, 100 - 10), TokenError::FundsUnavailable);
		assert_noop!(Balances::transfer_allow_death(Some(1).into(), 3, 155 - 10), TokenError::FundsUnavailable);
		// Account 3 sends 100 units to account 1 and 2
		assert_ok!(Balances::transfer_allow_death(Some(3).into(), 1, 100));
		assert_ok!(Balances::transfer_allow_death(Some(3).into(), 2, 100));

		let user1_free_balance = Balances::free_balance(&1);
		assert_eq!(user1_free_balance, 150); // Account 1 has 100 more free balance than normal

		let user2_free_balance = Balances::free_balance(&2);
		assert_eq!(user2_free_balance, 110); // Account 2 has 100 more free balance than normal

		// Account 1 has only 5 units vested at block 1 (plus 150 unvested)
		assert_eq!(Vesting::vesting_balance(&1, LockType::Participation(0)), Some(45));
		assert_ok!(Vesting::vest(Some(1).into(), LockType::Participation(0)));
		let user1_free_balance2 = Balances::free_balance(&1);
		assert_eq!(user1_free_balance2, 155); // Account 1 has 100 more free balance than normal
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 3, 155 - 10)); // Account 1 can send extra units gained

		// Account 2 has no units vested at block 1, but gained 100
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(200));
		assert_ok!(Vesting::vest(Some(2).into(), LockType::Participation(0)));
		assert_ok!(Balances::transfer_allow_death(Some(2).into(), 3, 100 - 10)); // Account 2 can send extra
		                                                                 // units gained
	});
}

#[test]
fn liquid_funds_should_transfer_with_delayed_vesting() {
	ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
		let user12_free_balance = Balances::free_balance(&12);

		assert_eq!(user12_free_balance, 1280); // Account 12 has free balance
									   // Account 12 has liquid funds
		assert_eq!(Vesting::vesting_balance(&12, LockType::Participation(0)), Some(256 * 5));

		// Account 12 has delayed vesting
		let user12_vesting_schedule = VestingInfo::new(
			256 * 5,
			64, // Vesting over 20 blocks
			10,
		);
		assert_eq!(Vesting::vesting(&12, LockType::Participation(0)).unwrap(), vec![user12_vesting_schedule]);

		// Account 12 can still send liquid funds
		assert_ok!(Balances::transfer_allow_death(Some(12).into(), 3, 256 * 5 - 256));
	});
}

#[test]
fn vested_transfer_works() {
	ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
		let user3_free_balance = Balances::free_balance(&3);
		let user4_free_balance = Balances::free_balance(&4);
		assert_eq!(user3_free_balance, 256 * 30);
		assert_eq!(user4_free_balance, 256 * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)), None);
		// Make the schedule for the new transfer.
		let new_vesting_schedule = VestingInfo::new(
			256 * 5,
			64, // Vesting over 20 blocks
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule, LockType::Participation(0)));
		// Now account 4 should have vesting.
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)).unwrap(), vec![new_vesting_schedule]);
		// Ensure the transfer happened correctly.
		let user3_free_balance_updated = Balances::free_balance(&3);
		assert_eq!(user3_free_balance_updated, 256 * 25);
		let user4_free_balance_updated = Balances::free_balance(&4);
		assert_eq!(user4_free_balance_updated, 256 * 40);
		// Account 4 has 5 * 256 locked.
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(256 * 5));

		System::set_block_number(20);
		assert_eq!(System::block_number(), 20);

		// Account 4 has 5 * 64 units vested by block 20.
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(10 * 64));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		// Account 4 has fully vested,
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn vested_transfer_correctly_fails() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user2_free_balance = Balances::free_balance(&2);
		let user4_free_balance = Balances::free_balance(&4);
		assert_eq!(user2_free_balance, ED * 1);
		assert_eq!(user4_free_balance, ED * 40);

		// Account 2 should already have a vesting schedule.
		let user2_vesting_schedule = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![user2_vesting_schedule]);

		// Fails due to too low transfer amount.
		let new_vesting_schedule_too_low = VestingInfo::new(<Test as Config>::MinVestedTransfer::get() - 1, 64, 10);
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule_too_low, LockType::Participation(0)),
			Error::<Test>::AmountLow,
		);

		// `per_block` is 0, which would result in a schedule with infinite duration.
		let schedule_per_block_0 = VestingInfo::new(<Test as Config>::MinVestedTransfer::get(), 0, 10);
		assert_noop!(
			Vesting::vested_transfer(Some(13).into(), 4, schedule_per_block_0, LockType::Participation(0)),
			Error::<Test>::InvalidScheduleParams,
		);

		// `locked` is 0.
		let schedule_locked_0 = VestingInfo::new(0, 1, 10);
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 4, schedule_locked_0, LockType::Participation(0)),
			Error::<Test>::AmountLow,
		);

		// Free balance has not changed.
		assert_eq!(user2_free_balance, Balances::free_balance(&2));
		assert_eq!(user4_free_balance, Balances::free_balance(&4));
		// Account 4 has no schedules.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn vested_transfer_allows_max_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user_4_free_balance = Balances::free_balance(&4);
		let max_schedules = <Test as Config>::MAX_VESTING_SCHEDULES;
		let sched = VestingInfo::new(
			<Test as Config>::MinVestedTransfer::get(),
			1, // Vest over 2 * 256 blocks.
			10,
		);

		// Add max amount schedules to user 4.
		for _ in 0..max_schedules {
			assert_ok!(Vesting::vested_transfer(Some(13).into(), 4, sched, LockType::Participation(0)));
		}

		// The schedules count towards vesting balance
		let transferred_amount = <Test as Config>::MinVestedTransfer::get() * max_schedules as u64;
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(transferred_amount));
		// BUT NOT the free balance.
		assert_eq!(Balances::free_balance(&4), user_4_free_balance);

		// Cannot insert a 4th vesting schedule when `MaxVestingSchedules` === 3,
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 4, sched, LockType::Participation(0)),
			Error::<Test>::AtMaxVestingSchedules,
		);
		// so the free balance does not change.
		assert_eq!(Balances::free_balance(&4), user_4_free_balance);

		// Account 4 has fully vested when all the schedules end,
		System::set_block_number(<Test as Config>::MinVestedTransfer::get() + sched.starting_block());
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn force_vested_transfer_works() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user3_free_balance = Balances::free_balance(&3);
		let user4_free_balance = Balances::free_balance(&4);
		assert_eq!(user3_free_balance, ED * 30);
		assert_eq!(user4_free_balance, ED * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)), None);
		// Make the schedule for the new transfer.
		let new_vesting_schedule = VestingInfo::new(
			ED * 5,
			64, // Vesting over 20 blocks
			10,
		);

		assert_noop!(
			Vesting::force_vested_transfer(Some(4).into(), 3, 4, new_vesting_schedule, LockType::Participation(0)),
			BadOrigin
		);
		assert_ok!(Vesting::force_vested_transfer(
			RawOrigin::Root.into(),
			3,
			4,
			new_vesting_schedule,
			LockType::Participation(0)
		));
		// Now account 4 should have vesting.
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)).unwrap()[0], new_vesting_schedule);
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)).unwrap().len(), 1);
		// Ensure the transfer happened correctly.
		let user3_free_balance_updated = Balances::free_balance(&3);
		assert_eq!(user3_free_balance_updated, ED * 25);
		let user4_free_balance_updated = Balances::free_balance(&4);
		assert_eq!(user4_free_balance_updated, ED * 40);
		// Account 4 has 5 * ED locked.
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(ED * 5));

		System::set_block_number(20);
		assert_eq!(System::block_number(), 20);

		// Account 4 has 5 * 64 units vested by block 20.
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(10 * 64));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		// Account 4 has fully vested,
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn force_vested_transfer_correctly_fails() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user2_free_balance = Balances::free_balance(&2);
		let user4_free_balance = Balances::free_balance(&4);
		assert_eq!(user2_free_balance, ED * 1);
		assert_eq!(user4_free_balance, ED * 40);
		// Account 2 should already have a vesting schedule.
		let user2_vesting_schedule = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![user2_vesting_schedule]);

		// Too low transfer amount.
		let new_vesting_schedule_too_low = VestingInfo::new(<Test as Config>::MinVestedTransfer::get() - 1, 64, 10);
		assert_noop!(
			Vesting::force_vested_transfer(
				RawOrigin::Root.into(),
				3,
				4,
				new_vesting_schedule_too_low,
				LockType::Participation(0)
			),
			Error::<Test>::AmountLow,
		);

		// `per_block` is 0.
		let schedule_per_block_0 = VestingInfo::new(<Test as Config>::MinVestedTransfer::get(), 0, 10);
		assert_noop!(
			Vesting::force_vested_transfer(
				RawOrigin::Root.into(),
				13,
				4,
				schedule_per_block_0,
				LockType::Participation(0)
			),
			Error::<Test>::InvalidScheduleParams,
		);

		// `locked` is 0.
		let schedule_locked_0 = VestingInfo::new(0, 1, 10);
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 4, schedule_locked_0, LockType::Participation(0)),
			Error::<Test>::AmountLow,
		);

		// Verify no currency transfer happened.
		assert_eq!(user2_free_balance, Balances::free_balance(&2));
		assert_eq!(user4_free_balance, Balances::free_balance(&4));
		// Account 4 has no schedules.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn force_vested_transfer_allows_max_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user_4_free_balance = Balances::free_balance(&4);
		let max_schedules = <Test as Config>::MAX_VESTING_SCHEDULES;
		let sched = VestingInfo::new(
			<Test as Config>::MinVestedTransfer::get(),
			1, // Vest over 2 * 256 blocks.
			10,
		);

		// Add max amount schedules to user 4.
		for _ in 0..max_schedules {
			assert_ok!(Vesting::force_vested_transfer(
				RawOrigin::Root.into(),
				13,
				4,
				sched,
				LockType::Participation(0)
			));
		}

		// The schedules count towards vesting balance.
		let transferred_amount = <Test as Config>::MinVestedTransfer::get() * max_schedules as u64;
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(transferred_amount));
		// but NOT free balance.
		assert_eq!(Balances::free_balance(&4), user_4_free_balance);

		// Cannot insert a 4th vesting schedule when `MaxVestingSchedules` === 3
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 4, sched, LockType::Participation(0)),
			Error::<Test>::AtMaxVestingSchedules,
		);
		// so the free balance does not change.
		assert_eq!(Balances::free_balance(&4), user_4_free_balance);

		// Account 4 has fully vested when all the schedules end,
		System::set_block_number(<Test as Config>::MinVestedTransfer::get() + 10);
		assert_eq!(Vesting::vesting_balance(&4, LockType::Participation(0)), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4, LockType::Participation(0));
	});
}

#[test]
fn merge_schedules_that_have_not_started() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);
		assert_eq!(Balances::balance(&2), ED);

		// Add a schedule that is identical to the one that already exists.
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched0, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched0]);
		assert_eq!(Balances::balance(&2), ED);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));

		// Since we merged identical schedules, the new schedule finishes at the same
		// time as the original, just with double the amount.
		let sched1 = VestingInfo::new(
			sched0.locked() * 2,
			sched0.per_block() * 2,
			10, // Starts at the block the schedules are merged/
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched1]);

		assert_eq!(Balances::balance(&2), ED);
	});
}

#[test]
fn merge_ongoing_schedules() {
	// Merging two schedules that have started will vest both before merging.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 10,
			ED,                          // Vest over 10 blocks.
			sched0.starting_block() + 5, // Start at block 15.
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1]);

		// Got to half way through the second schedule where both schedules are actively vesting.
		let cur_block = 20;
		System::set_block_number(cur_block);

		// Account 2 has no usable balances prior to the merge because they have not unlocked
		// with `vest` yet.
		assert_eq!(Balances::balance(&2), ED);

		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));

		// Merging schedules un-vests all pre-existing schedules prior to merging, which is
		// reflected in account 2's updated usable balance.
		let sched0_vested_now = sched0.per_block() * (cur_block - sched0.starting_block());
		let sched1_vested_now = sched1.per_block() * (cur_block - sched1.starting_block());
		assert_eq!(Balances::balance(&2), ED + sched0_vested_now + sched1_vested_now);

		// The locked amount is the sum of what both schedules have locked at the current block.
		let sched2_locked =
			sched1.locked_at::<Identity>(cur_block).saturating_add(sched0.locked_at::<Identity>(cur_block));
		// End block of the new schedule is the greater of either merged schedule.
		let sched2_end = sched1.ending_block_as_balance::<Identity>().max(sched0.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end - cur_block;
		// Based off the new schedules total locked and its duration, we can calculate the
		// amount to unlock per block.
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = VestingInfo::new(sched2_locked, sched2_per_block, cur_block);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched2]);

		// And just to double check, we assert the new merged schedule we be cleaned up as expected.
		System::set_block_number(30);
		vest_and_assert_no_vesting::<Test>(2, LockType::Participation(0));
	});
}

#[test]
fn merging_shifts_other_schedules_index() {
	// Schedules being merged are filtered out, schedules to the right of any merged
	// schedule shift left and the merged schedule is always last.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = VestingInfo::new(
			ED * 10,
			ED, // Vesting over 10 blocks.
			10,
		);
		let sched1 = VestingInfo::new(
			ED * 11,
			ED, // Vesting over 11 blocks.
			11,
		);
		let sched2 = VestingInfo::new(
			ED * 12,
			ED, // Vesting over 12 blocks.
			12,
		);

		// Account 3 starts out with no schedules,
		assert_eq!(Vesting::vesting(&3, LockType::Participation(0)), None);
		// and some free balance.
		let free_balance = Balances::balance(&3);

		let cur_block = 1;
		assert_eq!(System::block_number(), cur_block);

		// Transfer the above 3 schedules to account 3.
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched0, LockType::Participation(0)));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched1, LockType::Participation(0)));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched2, LockType::Participation(0)));

		// With no schedules vested or merged they are in the order they are created
		assert_eq!(Vesting::vesting(&3, LockType::Participation(0)).unwrap(), vec![sched0, sched1, sched2]);
		// and the free balance has not changed.
		assert_eq!(free_balance, Balances::balance(&3));

		assert_ok!(Vesting::merge_schedules(Some(3).into(), 0, 2, LockType::Participation(0)));

		// Create the merged schedule of sched0 & sched2.
		// The merged schedule will have the max possible starting block,
		let sched3_start = sched1.starting_block().max(sched2.starting_block());
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched3_locked = sched2.locked_at::<Identity>(cur_block) + sched0.locked_at::<Identity>(cur_block);
		// and will end at the max possible block.
		let sched3_end = sched2.ending_block_as_balance::<Identity>().max(sched0.ending_block_as_balance::<Identity>());
		let sched3_duration = sched3_end - sched3_start;
		let sched3_per_block = sched3_locked / sched3_duration;
		let sched3 = VestingInfo::new(sched3_locked, sched3_per_block, sched3_start);

		// The not touched schedule moves left and the new merged schedule is appended.
		assert_eq!(Vesting::vesting(&3, LockType::Participation(0)).unwrap(), vec![sched1, sched3]);
		// The usable balance hasn't changed since none of the schedules have started.
		assert_eq!(Balances::balance(&3), free_balance);
	});
}

#[test]
fn merge_ongoing_and_yet_to_be_started_schedules() {
	// Merge an ongoing schedule that has had `vest` called and a schedule that has not already
	// started.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		// Fast forward to half way through the life of sched1.
		let mut cur_block = (sched0.starting_block() + sched0.ending_block_as_balance::<Identity>()) / 2;
		assert_eq!(cur_block, 20);
		System::set_block_number(cur_block);

		// Prior to vesting there is ED usable balance.
		let mut balance = ED;
		assert_eq!(Balances::balance(&2), balance);
		// Vest the current schedules (which is just sched0 now).
		Vesting::vest(Some(2).into(), LockType::Participation(0)).unwrap();

		// After vesting the usable balance increases by the unlocked amount.
		let sched0_vested_now = sched0.locked() - sched0.locked_at::<Identity>(cur_block);
		balance += sched0_vested_now;
		assert_eq!(Balances::balance(&2), balance);

		// Go forward a block.
		cur_block += 1;
		System::set_block_number(cur_block);

		// And add a schedule that starts after this block, but before sched0 finishes.
		let sched1 = VestingInfo::new(
			ED * 10,
			1, // Vesting over 256 * 10 (2560) blocks
			cur_block + 1,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1, LockType::Participation(0)));

		// Merge the schedules before sched1 starts.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));
		// After merging, the usable balance only changes by the amount sched0 vested since we
		// last called `vest` (which is just 1 block). The usable balance is not affected by
		// sched1 because it has not started yet.
		balance += sched0.per_block();
		assert_eq!(Balances::balance(&2), balance);

		// The resulting schedule will have the later starting block of the two,
		let sched2_start = sched1.starting_block();
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched2_locked = sched0.locked_at::<Identity>(cur_block) + sched1.locked_at::<Identity>(cur_block);
		// and will end at the max possible block.
		let sched2_end = sched0.ending_block_as_balance::<Identity>().max(sched1.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end - sched2_start;
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = VestingInfo::new(sched2_locked, sched2_per_block, sched2_start);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched2]);
	});
}

#[test]
fn merge_finished_and_ongoing_schedules() {
	// If a schedule finishes by the current block we treat the ongoing schedule,
	// without any alterations, as the merged one.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vesting over 20 blocks.
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 40,
			ED, // Vesting over 40 blocks.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1, LockType::Participation(0)));

		// Transfer a 3rd schedule, so we can demonstrate how schedule indices change.
		// (We are not merging this schedule.)
		let sched2 = VestingInfo::new(
			ED * 30,
			ED, // Vesting over 30 blocks.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched2, LockType::Participation(0)));

		// The schedules are in expected order prior to merging.
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1, sched2]);

		// Fast forward to sched0's end block.
		let cur_block = sched0.ending_block_as_balance::<Identity>();
		System::set_block_number(cur_block);
		assert_eq!(System::block_number(), 30);

		// Prior to `merge_schedules` and with no vest/vest_other called the user has no usable
		// balance.
		assert_eq!(Balances::balance(&2), ED);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));

		// sched2 is now the first, since sched0 & sched1 get filtered out while "merging".
		// sched1 gets treated like the new merged schedule by getting pushed onto back
		// of the vesting schedules vec. Note: sched0 finished at the current block.
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched2, sched1]);

		// sched0 has finished, so its funds are fully unlocked.
		let sched0_unlocked_now = sched0.locked();
		// The remaining schedules are ongoing, so their funds are partially unlocked.
		let sched1_unlocked_now = sched1.locked() - sched1.locked_at::<Identity>(cur_block);
		let sched2_unlocked_now = sched2.locked() - sched2.locked_at::<Identity>(cur_block);

		// Since merging also vests all the schedules, the users usable balance after merging
		// includes all pre-existing schedules unlocked through the current block, including
		// schedules not merged.
		assert_eq!(Balances::balance(&2), ED + sched0_unlocked_now + sched1_unlocked_now + sched2_unlocked_now);
	});
}

#[test]
fn merge_finishing_schedules_does_not_create_a_new_one() {
	// If both schedules finish by the current block we don't create new one
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		// Create sched1 and transfer it to account 2.
		let sched1 = VestingInfo::new(
			ED * 30,
			ED, // 30 block duration.
			10,
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched1, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1]);

		let all_scheds_end =
			sched0.ending_block_as_balance::<Identity>().max(sched1.ending_block_as_balance::<Identity>());

		assert_eq!(all_scheds_end, 40);
		System::set_block_number(all_scheds_end);

		// Prior to merge_schedules and with no vest/vest_other called the user has no usable
		// balance.
		assert_eq!(Balances::balance(&2), ED);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));
		// The user no longer has any more vesting schedules because they both ended at the
		// block they where merged,
		assert!(!<VestingStorage<Test>>::contains_key(&2, LockType::Participation(0)));
		// and their usable balance has increased by the total amount locked in the merged
		// schedules.
		assert_eq!(Balances::balance(&2), ED + sched0.locked() + sched1.locked());
	});
}

#[test]
fn merge_finished_and_yet_to_be_started_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule from the Genesis.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10, // Ends at block 30
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		let sched1 = VestingInfo::new(
			ED * 30,
			ED * 2, // 30 block duration.
			35,
		);
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched1, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1]);

		let sched2 = VestingInfo::new(
			ED * 40,
			ED, // 40 block duration.
			30,
		);
		// Add a 3rd schedule to demonstrate how sched1 shifts.
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched2, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched1, sched2]);

		assert_eq!(
			Vesting::vesting_balance(&2, LockType::Participation(0)).unwrap(),
			Balances::balance_on_hold(&LockType::Participation(0), &2)
		);
		assert_eq!(
			Vesting::vesting_balance(&2, LockType::Participation(0)),
			Some(sched0.locked() + sched1.locked() + sched2.locked())
		);

		System::set_block_number(30);

		// At block 30, sched0 has finished unlocking while sched1 and sched2 are still fully
		// locked,
		assert_eq!(Vesting::vesting_balance(&2, LockType::Participation(0)), Some(sched1.locked() + sched2.locked()));
		// but since we have not vested usable balance is still 0.
		assert_eq!(Balances::balance(&2), ED);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)));

		// sched0 is removed since it finished, and sched1 is removed and then pushed on the back
		// because it is treated as the merged schedule
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched2, sched1]);

		// The usable balance is updated because merging fully unlocked sched0.
		assert_eq!(Balances::balance(&2), ED + sched0.locked());
	});
}

#[test]
fn merge_schedules_throws_proper_errors() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // 20 block duration.
			10,
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);

		// Account 2 only has 1 vesting schedule.
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// Account 4 has 0 vesting schedules.
		assert_eq!(Vesting::vesting(&4, LockType::Participation(0)), None);
		assert_noop!(
			Vesting::merge_schedules(Some(4).into(), 0, 1, LockType::Participation(0)),
			Error::<Test>::NotVesting
		);

		// There are enough schedules to merge but an index is non-existent.
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched0, LockType::Participation(0)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0, sched0]);
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 2, LockType::Participation(0)),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// It is a storage noop with no errors if the indexes are the same.
		assert_storage_noop!(Vesting::merge_schedules(Some(2).into(), 0, 0, LockType::Participation(0)).unwrap());
	});
}

#[test]
fn merge_vesting_handles_per_block_0() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = VestingInfo::new(
			ED, 0, // Vesting over 256 blocks.
			1,
		);
		assert_eq!(sched0.ending_block_as_balance::<Identity>(), 257);
		let sched1 = VestingInfo::new(
			ED * 2,
			0, // Vesting over 512 blocks.
			10,
		);
		assert_eq!(sched1.ending_block_as_balance::<Identity>(), 512u64 + 10);

		let merged = VestingInfo::new(764, 1, 10);
		assert_eq!(Vesting::merge_vesting_info(5, sched0, sched1), Some(merged));
	});
}

#[test]
fn vesting_info_validate_works() {
	let min_transfer = <Test as Config>::MinVestedTransfer::get();
	// Does not check for min transfer.
	assert_eq!(VestingInfo::new(min_transfer - 1, 1u64, 10u64).is_valid(), true);

	// `locked` cannot be 0.
	assert_eq!(VestingInfo::new(0, 1u64, 10u64).is_valid(), false);

	// `per_block` cannot be 0.
	assert_eq!(VestingInfo::new(min_transfer + 1, 0u64, 10u64).is_valid(), false);

	// With valid inputs it does not error.
	assert_eq!(VestingInfo::new(min_transfer, 1u64, 10u64).is_valid(), true);
}

#[test]
fn vesting_info_ending_block_as_balance_works() {
	// Treats `per_block` 0 as 1.
	let per_block_0 = VestingInfo::new(256u32, 0u32, 10u32);
	assert_eq!(per_block_0.ending_block_as_balance::<Identity>(), 256 + 10);

	// `per_block >= locked` always results in a schedule ending the block after it starts
	let per_block_gt_locked = VestingInfo::new(256u32, 256 * 2u32, 10u32);
	assert_eq!(per_block_gt_locked.ending_block_as_balance::<Identity>(), 1 + per_block_gt_locked.starting_block());
	let per_block_eq_locked = VestingInfo::new(256u32, 256u32, 10u32);
	assert_eq!(
		per_block_gt_locked.ending_block_as_balance::<Identity>(),
		per_block_eq_locked.ending_block_as_balance::<Identity>()
	);

	// Correctly calcs end if `locked % per_block != 0`. (We need a block to unlock the remainder).
	let imperfect_per_block = VestingInfo::new(256u32, 250u32, 10u32);
	assert_eq!(imperfect_per_block.ending_block_as_balance::<Identity>(), imperfect_per_block.starting_block() + 2u32,);
	assert_eq!(imperfect_per_block.locked_at::<Identity>(imperfect_per_block.ending_block_as_balance::<Identity>()), 0);
}

#[test]
fn per_block_works() {
	let per_block_0 = VestingInfo::new(256u32, 0u32, 10u32);
	assert_eq!(per_block_0.per_block(), 1u32);
	assert_eq!(per_block_0.raw_per_block(), 0u32);

	let per_block_1 = VestingInfo::new(256u32, 1u32, 10u32);
	assert_eq!(per_block_1.per_block(), 1u32);
	assert_eq!(per_block_1.raw_per_block(), 1u32);
}

// When an accounts free balance + schedule.locked is less than ED, the vested transfer will fail.
#[test]
fn vested_transfer_less_than_existential_deposit_fails() {
	ExtBuilder::default().existential_deposit(4 * ED).build().execute_with(|| {
		// MinVestedTransfer is less the ED.
		assert!(<Test as Config>::Currency::minimum_balance() > <Test as Config>::MinVestedTransfer::get());

		let sched = VestingInfo::new(<Test as Config>::MinVestedTransfer::get() as u64, 1u64, 10u64);
		// The new account balance with the schedule's locked amount would be less than ED.
		assert!(Balances::free_balance(&99) + sched.locked() < <Test as Config>::Currency::minimum_balance());

		// vested_transfer fails.
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 99, sched, LockType::Participation(0)),
			TokenError::CannotCreateHold,
		);
		// force_vested_transfer fails.
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 99, sched, LockType::Participation(0)),
			TokenError::CannotCreateHold,
		);
	});
}

// TODO
#[test]
fn set_release_schedule() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		// Initial Status
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 30 * ED); // 7680 ED
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 0);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 0);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Hold 15 ED
		assert_ok!(Balances::hold(&LockType::Participation(0), &3, 15 * ED));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 15 * ED);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 15 * ED);
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 15 * ED);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Set release schedule to release the locked amount, starting from now, one ED per block.
		let user3_vesting_schedule = VestingInfo::new(user_3_on_hold_balance, ED, System::block_number());
		assert_ok!(Vesting::set_release_schedule(
			&3,
			user3_vesting_schedule.locked,
			user3_vesting_schedule.per_block(),
			user3_vesting_schedule.starting_block,
			LockType::Participation(0)
		));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(15 * ED));

		// 1 ED can be "released", we need to call vest to release it.
		System::set_block_number(2);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(14 * ED));
		assert_ok!(Vesting::vest(Some(3).into(), LockType::Participation(0)));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 16 * ED);

		// 2 ED can be "released"
		System::set_block_number(3);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(13 * ED));
		assert_ok!(Vesting::vest(Some(3).into(), LockType::Participation(0)));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 17 * ED);

		// Go to the end of the schedule
		System::set_block_number(16);
		assert_eq!(System::block_number(), 16);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(0));
		vest_and_assert_no_vesting::<Test>(3, LockType::Participation(0));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 30 * ED);
	});
}

// TODO
#[test]
fn cannot_release_different_reason() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		// Initial Status
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 30 * ED); // 7680 ED
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 0);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 0);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Hold 15 ED
		assert_ok!(Balances::hold(&LockType::Participation(0), &3, 15 * ED));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 15 * ED);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 15 * ED);
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 15 * ED);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Set release schedule to release the locked amount, starting from now, one ED per block.
		let user3_vesting_schedule = VestingInfo::new(user_3_on_hold_balance, ED, System::block_number());
		assert_ok!(Vesting::set_release_schedule(
			&3,
			user3_vesting_schedule.locked,
			user3_vesting_schedule.per_block(),
			user3_vesting_schedule.starting_block,
			LockType::Participation(0)
		));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(15 * ED));

		// 1 ED can be "released", we need to call vest to release it.
		System::set_block_number(2);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(14 * ED));
		assert_ok!(Vesting::vest(Some(3).into(), LockType::Participation(0)));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 16 * ED);
	});
}

// TODO
#[test]
fn multile_holds_release_schedule() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		// Initial Status
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 30 * ED); // 7680 ED
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 0);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 0);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Hold 15 ED
		assert_ok!(Balances::hold(&LockType::Participation(0), &3, 15 * ED));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 15 * ED);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(0), &3);
		assert_eq!(user_3_on_hold_balance, 15 * ED);
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 15 * ED);
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);

		// Set release schedule to release the locked amount, starting from now, one ED per block.
		let user3_vesting_schedule = VestingInfo::new(user_3_on_hold_balance, ED, 1);
		assert_ok!(Vesting::set_release_schedule(
			&3,
			user3_vesting_schedule.locked,
			user3_vesting_schedule.per_block,
			user3_vesting_schedule.starting_block,
			LockType::Participation(0)
		));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(15 * ED));

		// Hold 7 ED more
		assert_ok!(Balances::hold(&LockType::Participation(1), &3, 7 * ED));
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 8 * ED);
		let user_3_on_hold_balance = Balances::balance_on_hold(&LockType::Participation(1), &3);
		assert_eq!(user_3_on_hold_balance, 7 * ED);
		let user_3_reserved_balance = Balances::reserved_balance(&3);
		assert_eq!(user_3_reserved_balance, 22 * ED);

		// Set release schedule to release the locked amount, starting from now, one ED per block.
		let user3_vesting_schedule = VestingInfo::new(user_3_on_hold_balance, ED, System::block_number() + 21);
		assert_ok!(Vesting::set_release_schedule(
			&3,
			user3_vesting_schedule.locked,
			user3_vesting_schedule.per_block,
			user3_vesting_schedule.starting_block,
			LockType::Participation(1)
		));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), Some(15 * ED));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(1)), Some(7 * ED));

		System::set_block_number(16);
		assert_ok!(Vesting::vest(Some(3).into(), LockType::Participation(0)));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(0)), None);
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 23 * ED);

		System::set_block_number(100);
		assert_ok!(Vesting::vest(Some(3).into(), LockType::Participation(1)));
		assert_eq!(Vesting::vesting_balance(&3, LockType::Participation(1)), None);
		let user_3_free_balance = Balances::free_balance(&3);
		assert_eq!(user_3_free_balance, 30 * ED);
	});
}

#[test]
fn merge_schedules_different_reason() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = VestingInfo::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			10,
		);
		assert_eq!(Balances::balance(&2), ED);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(0)).unwrap(), vec![sched0]);


		// Add a schedule that is identical to the one that already exists.
		assert_ok!(Vesting::vested_transfer(Some(14).into(), 2, sched0, LockType::Participation(1)));
		assert_ok!(Vesting::vested_transfer(Some(14).into(), 2, sched0, LockType::Participation(1)));
		assert_eq!(Vesting::vesting(&2, LockType::Participation(1)).unwrap(), vec![sched0, sched0]);
		assert_eq!(Balances::balance(&2), ED);
		assert_noop!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(0)), Error::<Test>::ScheduleIndexOutOfBounds);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1, LockType::Participation(1)));

		// Since we merged identical schedules, the new schedule finishes at the same
		// time as the original, just with double the amount.
		let sched1 = VestingInfo::new(
			sched0.locked() * 2,
			sched0.per_block() * 2,
			10, // Starts at the block the schedules are merged/
		);
		assert_eq!(Vesting::vesting(&2, LockType::Participation(1)).unwrap(), vec![sched1]);

		assert_eq!(Balances::balance(&2), ED);
	});
}