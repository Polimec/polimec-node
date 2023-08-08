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

impl<T: Config> Pallet<T> {
	// Create a new `VestingInfo`, based off of two other `VestingInfo`s.
	// NOTE: We assume both schedules have had funds unlocked up through the current block.
	pub fn merge_vesting_info(
		now: BlockNumberFor<T>,
		schedule1: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
		schedule2: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
	) -> Option<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>> {
		let schedule1_ending_block = schedule1.ending_block_as_balance::<T::BlockNumberToBalance>();
		let schedule2_ending_block = schedule2.ending_block_as_balance::<T::BlockNumberToBalance>();
		let now_as_balance = T::BlockNumberToBalance::convert(now);

		// Check if one or both schedules have ended.
		match (schedule1_ending_block <= now_as_balance, schedule2_ending_block <= now_as_balance) {
			// If both schedules have ended, we don't merge and exit early.
			(true, true) => return None,
			// If one schedule has ended, we treat the one that has not ended as the new
			// merged schedule.
			(true, false) => return Some(schedule2),
			(false, true) => return Some(schedule1),
			// If neither schedule has ended don't exit early.
			_ => {},
		}

		let locked = schedule1
			.locked_at::<T::BlockNumberToBalance>(now)
			.saturating_add(schedule2.locked_at::<T::BlockNumberToBalance>(now));
		// This shouldn't happen because we know at least one ending block is greater than now,
		// thus at least a schedule a some locked balance.
		debug_assert!(!locked.is_zero(), "merge_vesting_info validation checks failed to catch a locked of 0");

		let ending_block = schedule1_ending_block.max(schedule2_ending_block);
		let starting_block = now.max(schedule1.starting_block()).max(schedule2.starting_block());

		let per_block = {
			let duration =
				ending_block.saturating_sub(T::BlockNumberToBalance::convert(starting_block)).max(One::one());
			(locked / duration).max(One::one())
		};

		let schedule = VestingInfo::new(locked, per_block, starting_block);
		debug_assert!(schedule.is_valid(), "merge_vesting_info schedule validation check failed");

		Some(schedule)
	}

	// Execute a vested transfer from `source` to `target` with the given `schedule`.
	pub fn do_vested_transfer(
		source: AccountIdOf<T>,
		target: AccountIdOf<T>,
		schedule: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
		reason: ReasonOf<T>,
	) -> DispatchResult {
		// Validate user inputs.
		ensure!(schedule.locked() >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);
		if !schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into())
		};

		// Check we can add to this account prior to any storage writes.
		Self::can_add_release_schedule(
			&target,
			schedule.locked(),
			schedule.per_block(),
			schedule.starting_block(),
			reason,
		)?;

		let amount_transferred = T::Currency::transfer_and_hold(
			&reason,
			&source,
			&target,
			schedule.locked(),
			Precision::Exact,
			frame_support::traits::tokens::Preservation::Expendable,
			frame_support::traits::tokens::Fortitude::Polite,
		)?;

		Self::deposit_event(Event::<T>::VestingTransferred { to: target.clone(), amount: amount_transferred });

		// We can't let this fail because the currency transfer has already happened.
		let res = Self::add_release_schedule(
			&target,
			amount_transferred,
			schedule.per_block(),
			schedule.starting_block(),
			reason,
		);
		debug_assert!(res.is_ok(), "{:#?}", res.err());

		Ok(())
	}

	/// Iterate through the schedules to track the current locked amount and
	/// filter out completed and specified schedules.
	///
	/// Returns a tuple that consists of:
	/// - Vec of vesting schedules, where completed schedules and those specified
	///   by filter are removed. (Note the vec is not checked for respecting
	///   bounded length.)
	/// - The amount locked at the current block number based on the given schedules.
	///
	/// NOTE: the amount locked does not include any schedules that are filtered out via `action`.
	pub fn report_schedule_updates(
		schedules: Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>,
		action: VestingAction,
	) -> (Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>, BalanceOf<T>) {
		let now = <frame_system::Pallet<T>>::block_number();

		let mut total_releasable: BalanceOf<T> = Zero::zero();
		let filtered_schedules = action
			.pick_schedules::<T>(schedules)
			.filter(|schedule| {
				let locked_now = schedule.locked_at::<T::BlockNumberToBalance>(now);
				let keep = !locked_now.is_zero();
				if keep {
					total_releasable.saturating_accrue(locked_now);
				}
				keep
			})
			.collect::<Vec<_>>();

		(filtered_schedules, total_releasable)
	}

	/// Write an accounts updated vesting lock to storage.
	pub fn write_release(
		who: &T::AccountId,
		total_held_now: BalanceOf<T>,
		reason: ReasonOf<T>,
	) -> Result<(), DispatchError> {
		if total_held_now.is_zero() {
			T::Currency::release(
				&reason,
				who,
				T::Currency::balance_on_hold(&reason, who),
				frame_support::traits::tokens::Precision::BestEffort,
			)?;
			Self::deposit_event(Event::<T>::VestingCompleted { account: who.clone() });
		} else {
			let already_held = T::Currency::balance_on_hold(&reason, who);
			let to_release = already_held.saturating_sub(total_held_now);
			T::Currency::release(&reason, who, to_release, Precision::BestEffort)?;
			Self::deposit_event(Event::<T>::VestingUpdated { account: who.clone(), unvested: total_held_now });
		};

		Ok(())
	}

	/// Write an accounts updated vesting schedules to storage.
	pub fn write_vesting_schedule(
		who: &T::AccountId,
		schedules: Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>,
		reason: ReasonOf<T>,
	) -> Result<(), DispatchError> {
		let schedules: BoundedVec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>, MaxVestingSchedulesGet<T>> =
			schedules.try_into().map_err(|_| Error::<T>::AtMaxVestingSchedules)?;

		if schedules.len() == 0 {
			Vesting::<T>::remove(who, reason);
		} else {
			Vesting::<T>::insert(who, reason, schedules)
		}

		Ok(())
	}

	/// Unlock any vested funds of `who`.
	pub fn do_vest(who: T::AccountId, reason: ReasonOf<T>) -> DispatchResult {
		let schedules = Self::vesting(&who, reason).ok_or(Error::<T>::NotVesting)?;

		let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;
		Self::write_vesting_schedule(&who, schedules, reason)?;
		Self::write_release(&who, locked_now, reason)?;

		Ok(())
	}

	/// Execute a `VestingAction` against the given `schedules`. Returns the updated schedules
	/// and locked amount.
	pub fn exec_action(
		schedules: Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>,
		action: VestingAction,
	) -> Result<(Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>, BalanceOf<T>), DispatchError> {
		let (schedules, locked_now) = match action {
			VestingAction::Merge { index1: idx1, index2: idx2 } => {
				// The schedule index is based off of the schedule ordering prior to filtering out
				// any schedules that may be ending at this block.
				let schedule1 = *schedules.get(idx1).ok_or(Error::<T>::ScheduleIndexOutOfBounds)?;
				let schedule2 = *schedules.get(idx2).ok_or(Error::<T>::ScheduleIndexOutOfBounds)?;

				// The length of `schedules` decreases by 2 here since we filter out 2 schedules.
				// Thus we know below that we can push the new merged schedule without error
				// (assuming initial state was valid).
				let (mut schedules, mut locked_now) = Self::report_schedule_updates(schedules.to_vec(), action);

				let now = <frame_system::Pallet<T>>::block_number();
				if let Some(new_schedule) = Self::merge_vesting_info(now, schedule1, schedule2) {
					// Merging created a new schedule so we:
					// 1) need to add it to the accounts vesting schedule collection,
					schedules.push(new_schedule);
					// (we use `locked_at` in case this is a schedule that started in the past)
					let new_schedule_locked = new_schedule.locked_at::<T::BlockNumberToBalance>(now);
					// and 2) update the locked amount to reflect the schedule we just added.
					locked_now = locked_now.saturating_add(new_schedule_locked);
				} // In the None case there was no new schedule to account for.

				(schedules, locked_now)
			},
			_ => Self::report_schedule_updates(schedules.to_vec(), action),
		};

		debug_assert!(
			locked_now > Zero::zero() && !schedules.is_empty() || locked_now == Zero::zero() && schedules.is_empty()
		);

		Ok((schedules, locked_now))
	}
}

impl<T: Config> ReleaseSchedule<AccountIdOf<T>, ReasonOf<T>> for Pallet<T> {
	type Currency = T::Currency;
	type Moment = BlockNumberFor<T>;

	/// Get the amount that is possible to vest (i.e release) at this block.
	fn vesting_balance(who: &T::AccountId, reason: ReasonOf<T>) -> Option<BalanceOf<T>> {
		if let Some(v) = Self::vesting(who, reason) {
			let now = <frame_system::Pallet<T>>::block_number();
			let total_releasable = v.iter().fold(Zero::zero(), |total, schedule| {
				schedule.releaseble_at::<T::BlockNumberToBalance>(now).saturating_add(total)
			});
			Some(total_releasable)
		} else {
			None
		}
	}

	fn total_scheduled_amount(who: &T::AccountId, reason: ReasonOf<T>) -> Option<BalanceOf<T>> {
		if let Some(v) = Self::vesting(who, reason) {
			let total = v.iter().fold(Zero::zero(), |total, schedule| {
				schedule.locked.saturating_add(total)
			});
			Some(total)
		} else {
			None
		}
	}

	/// Adds a vesting schedule to a given account.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// On success, a linearly reducing amount of funds will be locked. In order to realise any
	/// reduction of the lock over time as it diminishes, the account owner must use `vest` or
	/// `vest_other`.
	///
	/// Is a no-op if the amount to be vested is zero.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn add_release_schedule(
		who: &T::AccountId,
		locked: BalanceOf<T>,
		per_block: BalanceOf<T>,
		starting_block: BlockNumberFor<T>,
		reason: ReasonOf<T>,
	) -> DispatchResult {
		if locked.is_zero() {
			return Ok(())
		}

		let vesting_schedule = VestingInfo::new(locked, per_block, starting_block);
		// Check for `per_block` or `locked` of 0.
		if !vesting_schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into())
		};

		let mut schedules = Self::vesting(who, reason).unwrap_or_default();

		// NOTE: we must push the new schedule so that `exec_action`
		// will give the correct new locked amount.
		ensure!(schedules.try_push(vesting_schedule).is_ok(), Error::<T>::AtMaxVestingSchedules);

		let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;

		Self::write_vesting_schedule(who, schedules, reason)?;
		Self::write_release(who, locked_now, reason)?;
		Ok(())
	}

	// Ensure we can call `add_vesting_schedule` without error. This should always
	// be called prior to `add_vesting_schedule`.
	fn can_add_release_schedule(
		who: &T::AccountId,
		locked: BalanceOf<T>,
		per_block: BalanceOf<T>,
		starting_block: BlockNumberFor<T>,
		reason: ReasonOf<T>,
	) -> DispatchResult {
		// Check for `per_block` or `locked` of 0.
		if !VestingInfo::new(locked, per_block, starting_block).is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into())
		}

		ensure!(
			(Vesting::<T>::decode_len(who, reason).unwrap_or_default() as u32) < T::MAX_VESTING_SCHEDULES,
			Error::<T>::AtMaxVestingSchedules
		);

		Ok(())
	}

	fn set_release_schedule(
		who: &T::AccountId,
		locked: <Self::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance,
		per_block: <Self::Currency as frame_support::traits::fungible::Inspect<T::AccountId>>::Balance,
		starting_block: Self::Moment,
		reason: ReasonOf<T>,
	) -> DispatchResult {
		if locked.is_zero() {
			return Ok(())
		}

		let vesting_schedule = VestingInfo::new(locked, per_block, starting_block);
		// Check for `per_block` or `locked` of 0.
		if !vesting_schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into())
		};

		let mut schedules = Self::vesting(who, reason).unwrap_or_default();

		// NOTE: we must push the new schedule so that `exec_action`
		// will give the correct new locked amount.
		ensure!(schedules.try_push(vesting_schedule).is_ok(), Error::<T>::AtMaxVestingSchedules);

		let (schedules, _) = Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;

		Self::write_vesting_schedule(who, schedules, reason)?;
		Ok(())
	}

	/// Remove a vesting schedule for a given account.
	fn remove_vesting_schedule(who: &T::AccountId, schedule_index: u32, reason: ReasonOf<T>) -> DispatchResult {
		let schedules = Self::vesting(who, reason).ok_or(Error::<T>::NotVesting)?;
		let remove_action = VestingAction::Remove { index: schedule_index as usize };

		let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), remove_action)?;

		Self::write_vesting_schedule(who, schedules, reason)?;
		Self::write_release(who, locked_now, reason)?;
		Ok(())
	}

	fn vest(who: AccountIdOf<T>, reason: ReasonOf<T>) -> DispatchResult {
		Self::do_vest(who, reason)
	}
}
