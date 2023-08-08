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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::tokens::fungible};

/// A release schedule over a fungible. This allows a particular fungible to have release limits
/// applied to it.
pub trait ReleaseSchedule<AccountId, Reason> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type Moment;

	/// The currency that this schedule applies to.
	type Currency: fungible::InspectHold<AccountId>
		+ fungible::MutateHold<AccountId>
		+ fungible::BalancedHold<AccountId>;

	/// Get the amount that is possible to vest (i.e release) at the current block
	fn vesting_balance(
		who: &AccountId,
		reason: Reason,
	) -> Option<<Self::Currency as fungible::Inspect<AccountId>>::Balance>;

	/// Get the amount that was scheduled, regardless if it was already vested or not
	fn total_scheduled_amount(
		who: &AccountId,
		reason: Reason,
	) -> Option<<Self::Currency as fungible::Inspect<AccountId>>::Balance>;



	/// Release the vested amount of the given account.
	fn vest(who: AccountId, reason: Reason) -> DispatchResult;

	/// Adds a release schedule to a given account.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// Is a no-op if the amount to be vested is zero.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn add_release_schedule(
		who: &AccountId,
		locked: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		per_block: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		starting_block: Self::Moment,
		reason: Reason,
	) -> DispatchResult;

	/// Set a release schedule to a given account, without locking any funds.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// Is a no-op if the amount to be vested is zero.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn set_release_schedule(
		who: &AccountId,
		locked: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		per_block: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		starting_block: Self::Moment,
		reason: Reason,
	) -> DispatchResult;

	/// Checks if `add_release_schedule` would work against `who`.
	fn can_add_release_schedule(
		who: &AccountId,
		locked: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		per_block: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		starting_block: Self::Moment,
		reason: Reason,
	) -> DispatchResult;

	/// Remove a release schedule for a given account.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn remove_vesting_schedule(who: &AccountId, schedule_index: u32, reason: Reason) -> DispatchResult;
}
