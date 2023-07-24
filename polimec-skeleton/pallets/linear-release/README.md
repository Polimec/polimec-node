<!-- markdown-link-check-disable -->
# Pallet Linear Release

A modified version of [pallet-vesting](https://github.com/paritytech/substrate/tree/polkadot-v0.9.42/frame/vesting/src). It allows to set a Release Schedule using the new `fungible::Hold` API instead of the old and deprecated `LockableCurrency`.

It implements the new `ReleaseSchedule` trait. The trait serves to apply release limits to a particular fungible and provides several key methods to interact with the release schedule for an account.

Key features of the `ReleaseSchedule` trait include:

- `vesting_balance`: This method returns the amount that is currently being vested and cannot be transferred out of an account. It returns None if the account has no vesting schedule.
- `add_release_schedule`: This method allows a release schedule to be added to a given account. If the account has reached the MaxVestingSchedules, an error is returned and nothing is updated. It's a no-op if the amount to be vested is zero. Importantly, this doesn't alter the free balance of the account.
- `set_release_schedule`: This method sets a release schedule for a given account, without locking any funds. Similar to the add_release_schedule, it returns an error if the account has MaxVestingSchedules and doesn't alter the free balance of the account.
- `can_add_release_schedule`: This method checks if a release schedule can be added to a given account.
- `remove_vesting_schedule`: This method allows for a vesting schedule to be removed from a given account. Note, this does not alter the free balance of the account.

The main differences with pallet_vesting are:

-  Use of `fungible::Hold` instead of `LockableCurrency`, this change enable a more granular control.
-  Add `set_release_schedule` to set a `ReleaseSchedule` without locking the fungible in the same transaction.

Here you can find a diff with the original pallet-vesting: https://gist.github.com/lrazovic/04efb94ecc19f08e80ac6eff48a8adb0