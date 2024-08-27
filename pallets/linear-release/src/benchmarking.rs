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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as PalletLinearRelease;

use frame_benchmarking::v2::*;
use frame_support::{
	assert_ok,
	traits::{tokens::Preservation::Expendable, OriginTrait},
};
use frame_system::{pallet_prelude::BlockNumberFor, Pallet as System, RawOrigin};
use sp_runtime::traits::{CheckedDiv, CheckedMul};
const SEED: u32 = 0;

fn add_holds<T: Config>(who: &T::AccountId, n: u32) {
	for _id in 0..n {
		let locked = 256u32;
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = T::Currency::hold(&reason, who, locked.into());
	}
}

fn add_vesting_schedules<T: Config>(
	target: AccountIdOf<T>,
	n: u32,
	reason: ReasonOf<T>,
) -> Result<BalanceOf<T>, &'static str> {
	let min_transfer = T::MinVestedTransfer::get();
	let locked = min_transfer.checked_mul(&10u32.into()).unwrap();
	// Schedule has a duration of 10.
	let per_block = min_transfer;
	let starting_block = 1u32;

	let source: T::AccountId = account("source", 0, SEED);
	T::Currency::set_balance(&source, T::Currency::minimum_balance().saturating_add(locked));
	System::<T>::set_block_number(BlockNumberFor::<T>::zero());

	let mut total_locked: BalanceOf<T> = Zero::zero();
	for _ in 0..n {
		total_locked += locked;

		let schedule = VestingInfo::new(locked, per_block, starting_block.into());

		assert_ok!(PalletLinearRelease::<T>::do_vested_transfer(source.clone(), target.clone(), schedule, reason));

		// Top up to guarantee we can always transfer another schedule.
		T::Currency::set_balance(&source, T::Currency::minimum_balance().saturating_add(locked));
	}

	Ok(total_locked)
}

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent> + crate::Config,
    <T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
)]
mod benches {
	use super::*;

	// Implements a test for each benchmark. Execute with:
	// `cargo test -p pallet-linear-release --features runtime-benchmarks`.
	impl_benchmark_test_suite!(
		PalletLinearRelease,
		crate::mock::ExtBuilder::default().existential_deposit(256).build(),
		crate::mock::Test
	);

	#[benchmark]
	fn vest_locked(l: Linear<0, 9>, s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::set_balance(&caller, T::Currency::minimum_balance().saturating_add(1024u32.into()));

		add_holds::<T>(&caller, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = add_vesting_schedules::<T>(caller.clone(), s, reason)?;

		// At block zero, everything is vested.
		assert_eq!(System::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule not added",
		);

		#[extrinsic_call]
		vest(RawOrigin::Signed(caller.clone()), reason);

		// Nothing happened since everything is still vested.
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule was removed",
		);

		Ok(())
	}

	#[benchmark]
	fn vest_unlocked(l: Linear<0, 9>, s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::set_balance(&caller, T::Currency::minimum_balance().saturating_add(1024u32.into()));

		add_holds::<T>(&caller, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let held = add_vesting_schedules::<T>(caller.clone(), s, reason)?;

		// At block 21, everything is unlocked.
		System::<T>::set_block_number(21u32.into());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(held),
			"Vesting schedule still active",
		);

		#[extrinsic_call]
		vest(RawOrigin::Signed(caller.clone()), reason);

		// Vesting schedule is removed!
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			None,
			"Vesting schedule was not removed",
		);

		Ok(())
	}

	#[benchmark]
	fn vest_other_locked(
		l: Linear<0, 9>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>,
	) -> Result<(), BenchmarkError> {
		let other: T::AccountId = account("other", 0, SEED);
		let caller: T::AccountId = whitelisted_caller();

		T::Currency::set_balance(&other, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&other, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = add_vesting_schedules::<T>(other.clone(), s, reason)?;

		// At block zero, everything is vested.
		assert_eq!(System::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&other, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule not added",
		);

		#[extrinsic_call]
		vest_other(RawOrigin::Signed(caller.clone()), other.clone(), reason);

		// Nothing happened since everything is still vested.
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&other, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule was removed",
		);
		Ok(())
	}

	#[benchmark]
	fn vest_other_unlocked(
		l: Linear<0, 9>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>,
	) -> Result<(), BenchmarkError> {
		let other: T::AccountId = account("other", 0, SEED);
		let caller: T::AccountId = whitelisted_caller();

		T::Currency::set_balance(&other, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&other, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let held = add_vesting_schedules::<T>(other.clone(), s, reason)?;

		// At block 21 everything is unlocked.
		System::<T>::set_block_number(21u32.into());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&other, reason),
			Some(held),
			"Vesting schedule still active",
		);

		#[extrinsic_call]
		vest_other(RawOrigin::Signed(caller.clone()), other.clone(), reason);

		// Vesting schedule is removed.
		assert_eq!(PalletLinearRelease::<T>::vesting_balance(&other, reason), None, "Vesting schedule was removed",);
		Ok(())
	}

	#[benchmark]
	fn vested_transfer(l: Linear<0, 9>, s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::set_balance(&caller, T::Currency::minimum_balance() + T::MinVestedTransfer::get());

		let target: T::AccountId = account("target", 0, SEED);
		// Give target existing locks
		T::Currency::set_balance(&target, T::Currency::minimum_balance() + (256 * l).into());
		add_holds::<T>(&target, l);

		// Add one vesting schedules.
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let mut expected_balance = add_vesting_schedules::<T>(target.clone(), s, reason)?;

		let transfer_amount = T::MinVestedTransfer::get();
		let per_block = transfer_amount.checked_div(&10u32.into()).unwrap();
		expected_balance += transfer_amount;

		let vesting_schedule = VestingInfo::new(transfer_amount, per_block, 1u32.into());

		#[extrinsic_call]
		vested_transfer(RawOrigin::Signed(caller.clone()), target.clone(), vesting_schedule, reason);

		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&target, reason),
			Some(BalanceOf::<T>::zero()),
			"Lock not correctly updated",
		);
		Ok(())
	}

	#[benchmark]
	fn force_vested_transfer(
		l: Linear<0, 9>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>,
	) -> Result<(), BenchmarkError> {
		let source: T::AccountId = account("source", 0, SEED);
		T::Currency::set_balance(&source, T::Currency::minimum_balance().saturating_add(1024u32.into()));

		let target: T::AccountId = account("target", 0, SEED);
		// Give target existing locks
		T::Currency::set_balance(&target, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&target, l);

		// Add one less than max vesting schedules
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let mut expected_balance = add_vesting_schedules::<T>(target.clone(), s, reason)?;

		let transfer_amount = T::MinVestedTransfer::get();
		let per_block = transfer_amount.checked_div(&10u32.into()).unwrap();
		expected_balance += transfer_amount;

		let vesting_schedule = VestingInfo::new(transfer_amount, per_block, 1u32.into());

		#[extrinsic_call]
		force_vested_transfer(RawOrigin::Root, source.clone(), target.clone(), vesting_schedule, reason);

		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&target, reason),
			Some(BalanceOf::<T>::zero()),
			"Lock not correctly updated",
		);

		Ok(())
	}

	#[benchmark]
	fn not_unlocking_merge_schedules(
		l: Linear<0, 9>,
		s: Linear<2, T::MAX_VESTING_SCHEDULES>,
	) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = account("caller", 0, SEED);
		// Give target existing locks.
		T::Currency::set_balance(&caller, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&caller, l);
		// Add max vesting schedules.
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = add_vesting_schedules::<T>(caller.clone(), s, reason)?;

		// Schedules are not vesting at block 0.
		assert_eq!(System::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting balance should equal sum locked of all schedules",
		);
		assert_eq!(
			PalletLinearRelease::<T>::vesting(&caller, reason).unwrap().len(),
			s as usize,
			"There should be exactly max vesting schedules"
		);

		#[extrinsic_call]
		merge_schedules(RawOrigin::Signed(caller.clone()), 0, s - 1, reason);

		let expected_schedule = VestingInfo::new(
			T::MinVestedTransfer::get() * 10u32.into() * 2u32.into(),
			T::MinVestedTransfer::get() * 2u32.into(),
			1u32.into(),
		);
		let expected_index = (s - 2) as usize;
		assert_eq!(PalletLinearRelease::<T>::vesting(&caller, reason).unwrap()[expected_index], expected_schedule);
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting balance should equal total locked of all schedules",
		);
		assert_eq!(
			PalletLinearRelease::<T>::vesting(&caller, reason).unwrap().len(),
			(s - 1) as usize,
			"Schedule count should reduce by 1"
		);

		Ok(())
	}

	#[benchmark]
	fn unlocking_merge_schedules(
		l: Linear<0, 9>,
		s: Linear<2, T::MAX_VESTING_SCHEDULES>,
	) -> Result<(), BenchmarkError> {
		// Destination used just for currency transfers in asserts.
		let test_dest: T::AccountId = account("test_dest", 0, SEED);

		let caller: T::AccountId = account("caller", 0, SEED);
		// Give target other locks.
		T::Currency::set_balance(&caller, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&caller, l);
		// Add max vesting schedules.
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let held = add_vesting_schedules::<T>(caller.clone(), s, reason)?;

		// Go to about half way through all the schedules duration. (They all start at 1, and have a duration of 10 or 11).
		System::<T>::set_block_number(6u32.into());
		// We expect half the original locked balance (+ any remainder that vests on the last block).
		let expected_balance = held / 2u32.into();
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(expected_balance),
			"Vesting balance should reflect that we are half way through all schedules duration",
		);
		assert_eq!(
			PalletLinearRelease::<T>::vesting(&caller, reason).unwrap().len(),
			s as usize,
			"There should be exactly max vesting schedules"
		);
		// The balance is not actually transferable because it has not been unlocked.
		assert!(T::Currency::transfer(&caller, &test_dest, expected_balance, Expendable).is_err());

		#[extrinsic_call]
		merge_schedules(RawOrigin::Signed(caller.clone()), 0, s - 1, reason);

		let expected_schedule = VestingInfo::new(
			T::MinVestedTransfer::get() * 2u32.into() * 5u32.into(),
			T::MinVestedTransfer::get() * 2u32.into(),
			6u32.into(),
		);
		let expected_index = (s - 2) as usize;
		assert_eq!(
			PalletLinearRelease::<T>::vesting(&caller, reason).unwrap()[expected_index],
			expected_schedule,
			"New schedule is properly created and placed"
		);
		assert_eq!(PalletLinearRelease::<T>::vesting(&caller, reason).unwrap()[expected_index], expected_schedule);
		assert_eq!(
			PalletLinearRelease::<T>::vesting(&caller, reason).unwrap().len(),
			(s - 1) as usize,
			"Schedule count should reduce by 1"
		);
		// Since merge unlocks all schedules we can now transfer the balance.
		assert_ok!(T::Currency::transfer(&caller, &test_dest, expected_balance, Expendable));

		Ok(())
	}

	#[benchmark]
	fn vest_all(l: Linear<0, 9>, s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>) -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::set_balance(&caller, T::Currency::minimum_balance().saturating_add(1024u32.into()));

		add_holds::<T>(&caller, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = add_vesting_schedules::<T>(caller.clone(), s, reason)?;
		// At block zero, everything is vested.
		assert_eq!(System::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&caller, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule not added",
		);

		#[extrinsic_call]
		vest_all(RawOrigin::Signed(caller.clone()));

		for _ in 0..l {
			let reason: ReasonOf<T> = T::BenchmarkReason::get();
			// Nothing happened since everything is still vested.
			assert_eq!(
				PalletLinearRelease::<T>::vesting_balance(&caller, reason),
				Some(BalanceOf::<T>::zero()),
				"Vesting schedule was removed",
			);
		}

		Ok(())
	}

	#[benchmark]
	fn vest_all_other(l: Linear<0, 9>, s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>) -> Result<(), BenchmarkError> {
		let other: T::AccountId = account("other", 0, SEED);
		let caller: T::AccountId = whitelisted_caller();

		T::Currency::set_balance(&other, T::Currency::minimum_balance().saturating_add(1024u32.into()));
		add_holds::<T>(&other, l);
		let reason: ReasonOf<T> = T::BenchmarkReason::get();
		let _ = add_vesting_schedules::<T>(other.clone(), s, reason)?;

		// At block zero, everything is vested.
		assert_eq!(System::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&other, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule not added",
		);

		#[extrinsic_call]
		vest_all_other(RawOrigin::Signed(caller.clone()), other.clone());

		// Nothing happened since everything is still vested.
		assert_eq!(
			PalletLinearRelease::<T>::vesting_balance(&other, reason),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule was removed",
		);
		Ok(())
	}
}
