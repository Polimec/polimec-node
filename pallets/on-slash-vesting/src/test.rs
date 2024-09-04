extern crate alloc;
use super::{mock::*, *};
use frame_support::{
	assert_ok,
	traits::tokens::fungible::{BalancedHold, Inspect, Mutate, MutateHold},
};
use mock::{Balances as PalletBalances, System as PalletSystem, Vesting as PalletVesting};
use pallet_balances::AccountData;
use pallet_vesting::VestingInfo;

#[test]
fn one_schedule() {
	ExtBuilder { existential_deposit: 1 }.build().execute_with(|| {
		<PalletBalances as Mutate<u64>>::set_balance(&1, 0);
		<PalletBalances as Mutate<u64>>::set_balance(&2, 100);
		let vesting_info = VestingInfo::new(100, 10, 1);
		assert_ok!(PalletVesting::vested_transfer(RuntimeOrigin::signed(2), 1, vesting_info));
		assert_ok!(<PalletBalances as MutateHold<u64>>::hold(&MockRuntimeHoldReason::Reason, &1u64, 30u128));

		assert_eq!(PalletBalances::usable_balance(1), 0);

		PalletSystem::set_block_number(3);
		// Unlock 20
		assert_ok!(PalletVesting::vest(RuntimeOrigin::signed(1)));
		assert_eq!(PalletBalances::usable_balance(1), 20);
		dbg!(<pallet_vesting::Vesting<TestRuntime>>::get(1));

		// Slash 30
		<PalletBalances as BalancedHold<u64>>::slash(&MockRuntimeHoldReason::Reason, &1u64, 30u128);
		<PalletVesting as OnSlash<u64, u128>>::on_slash(&1, 30);

		// After calling on_slash, the previously unlocked 20 should be available again
		assert_eq!(PalletBalances::usable_balance(1), 20);
	});
}

#[test]
fn multiple_schedules() {
	ExtBuilder { existential_deposit: 1 }.build().execute_with(|| {
		<PalletBalances as Mutate<u64>>::set_balance(&1, 0);
		<PalletBalances as Mutate<u64>>::set_balance(&2, 100);
		<PalletBalances as Mutate<u64>>::mint_into(&2, 130).unwrap();
		<PalletBalances as Mutate<u64>>::mint_into(&2, 75).unwrap();
		<PalletBalances as Mutate<u64>>::mint_into(&2, 200).unwrap();

		// Duration 10 blocks
		let vesting_info_1 = VestingInfo::new(100, 10, 1);
		// Duration 2 blocks
		let vesting_info_2 = VestingInfo::new(130, 65, 1);
		// Duration 15 blocks
		let vesting_info_3 = VestingInfo::new(75, 5, 1);
		// Duration 10 blocks
		let vesting_info_4 = VestingInfo::new(200, 20, 1);

		assert_ok!(PalletVesting::vested_transfer(RuntimeOrigin::signed(2), 1, vesting_info_1));
		assert_ok!(PalletVesting::vested_transfer(RuntimeOrigin::signed(2), 1, vesting_info_2));
		assert_ok!(PalletVesting::vested_transfer(RuntimeOrigin::signed(2), 1, vesting_info_3));
		assert_ok!(PalletVesting::vested_transfer(RuntimeOrigin::signed(2), 1, vesting_info_4));

		assert_ok!(<PalletBalances as MutateHold<u64>>::hold(&MockRuntimeHoldReason::Reason, &1u64, 100u128));
		assert_eq!(PalletBalances::usable_balance(1), 0);
		// see account data
		dbg!(PalletSystem::account(1).data);

		PalletSystem::set_block_number(3);

		// Unlock 10*2 + 65*2 + 5*2 + 20*2 = 200
		assert_ok!(PalletVesting::vest(RuntimeOrigin::signed(1)));
		assert_eq!(PalletBalances::usable_balance(1), 200);

		<PalletBalances as BalancedHold<u64>>::slash(&MockRuntimeHoldReason::Reason, &1u64, 65u128);
		<PalletVesting as OnSlash<u64, u128>>::on_slash(&1, 65);

		let schedules = <pallet_vesting::Vesting<TestRuntime>>::get(1).unwrap().to_vec();

		// One schedule was fully vested before the slash, the other got the full amount reduced after the slash
		assert_eq!(schedules, vec![VestingInfo::new(15, 1, 3), VestingInfo::new(95, 11, 3),]);

		assert_eq!(
			PalletSystem::account(1).data,
			AccountData { free: 405, reserved: 35, frozen: 110, flags: Default::default() }
		);

		// What part of the frozen restriction applies to the free balance after applying it to the slash
		let untouchable = 110 - 35;
		assert_eq!(PalletBalances::usable_balance(1), 405 - untouchable);
	});
}
