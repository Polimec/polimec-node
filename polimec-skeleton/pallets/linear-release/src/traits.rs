// TODO: Add Header

use frame_support::{pallet_prelude::DispatchResult, traits::tokens::fungible};

/// A release schedule over a fungible. This allows a particular fungible to have release limits
/// applied to it.
pub trait ReleaseSchedule<AccountId, Reason> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type Moment;

	/// The currency that this schedule applies to.
	type Currency: fungible::InspectHold<AccountId>
		+ fungible::MutateHold<AccountId>
		+ fungible::BalancedHold<AccountId>;
		

	/// Get the amount that is currently being vested and cannot be transferred out of this account.
	/// Returns `None` if the account has no vesting schedule.
	fn vesting_balance(who: &AccountId) -> Option<<Self::Currency as fungible::Inspect<AccountId>>::Balance>;

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

	/// Ser a release schedule to a given account, without locking any funds.
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
	) -> DispatchResult;

	/// Checks if `add_release_schedule` would work against `who`.
	fn can_add_release_schedule(
		who: &AccountId,
		locked: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		per_block: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		starting_block: Self::Moment,
	) -> DispatchResult;

	/// Remove a release schedule for a given account.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn remove_vesting_schedule(who: &AccountId, schedule_index: u32, reason: Reason) -> DispatchResult;
}
