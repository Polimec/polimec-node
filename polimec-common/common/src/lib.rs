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

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, traits::tokens::fungible};
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul},
	FixedPointNumber, RuntimeDebug,
};
pub use xcm::v4::{opaque::Xcm, Assets, Location, QueryId, SendError, SendResult, SendXcm, XcmHash};

pub mod assets;
pub mod credentials;

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: u32 = 60_000 / (MILLISECS_PER_BLOCK as u32);
pub const HOURS: u32 = MINUTES * 60;
pub const DAYS: u32 = HOURS * 24;

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
	fn vest(
		who: AccountId,
		reason: Reason,
	) -> Result<<Self::Currency as fungible::Inspect<AccountId>>::Balance, DispatchError>;

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

	fn remove_all_vesting_schedules(who: &AccountId, reason: Reason) -> DispatchResult;
}

pub mod migration_types {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use xcm::v4::Junction;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct MigrationOrigin {
		pub user: Junction,
		pub participation_type: ParticipationType,
	}
	impl PartialOrd for MigrationOrigin {
		fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
			Some(self.cmp(other))
		}
	}
	impl Ord for MigrationOrigin {
		fn cmp(&self, other: &Self) -> core::cmp::Ordering {
			if self.user == other.user {
				self.participation_type.cmp(&other.participation_type)
			} else {
				self.user.cmp(&other.user)
			}
		}
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum ParticipationType {
		Evaluation,
		Bid,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct MigrationInfo {
		pub contribution_token_amount: u128,
		pub vesting_time: u64,
	}
	impl From<(u128, u64)> for MigrationInfo {
		fn from((contribution_token_amount, vesting_time): (u128, u64)) -> Self {
			Self { contribution_token_amount, vesting_time }
		}
	}

	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		Ord,
		PartialOrd,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		DecodeWithMemTracking,
	)]
	pub enum MigrationStatus {
		NotStarted,
		Confirmed,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Migration {
		pub origin: MigrationOrigin,
		pub info: MigrationInfo,
	}

	impl Migration {
		pub const fn new(origin: MigrationOrigin, info: MigrationInfo) -> Self {
			Self { origin, info }
		}
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, Default)]
	pub struct Migrations(Vec<Migration>);
	impl FromIterator<Migration> for Migrations {
		fn from_iter<T: IntoIterator<Item = Migration>>(iter: T) -> Self {
			Migrations::from(iter.into_iter().collect::<Vec<_>>())
		}
	}

	impl Migrations {
		pub const fn new() -> Self {
			Self(Vec::new())
		}

		pub fn inner(self) -> Vec<Migration> {
			self.0
		}

		pub fn push(&mut self, migration: Migration) {
			self.0.push(migration)
		}

		pub const fn from(migrations: Vec<Migration>) -> Self {
			Self(migrations)
		}

		pub fn contains(&self, migration: &Migration) -> bool {
			self.0.contains(migration)
		}

		pub fn len(&self) -> usize {
			self.0.len()
		}

		pub fn is_empty(&self) -> bool {
			self.0.is_empty()
		}

		pub fn origins(&self) -> Vec<MigrationOrigin> {
			self.0.iter().map(|migration| migration.origin.clone()).collect()
		}

		pub fn infos(&self) -> Vec<MigrationInfo> {
			self.0.iter().map(|migration| migration.info.clone()).collect()
		}

		pub fn total_ct_amount(&self) -> u128 {
			self.0.iter().map(|migration| migration.info.contribution_token_amount).sum()
		}

		pub fn biggest_vesting_time(&self) -> u64 {
			self.0.iter().map(|migration| migration.info.vesting_time).max().unwrap_or(0)
		}
	}
}

pub const USD_DECIMALS: u8 = 6;
pub const USD_UNIT: u128 = 10u128.pow(USD_DECIMALS as u32);
pub const PLMC_DECIMALS: u8 = 10;
pub const PLMC_UNIT: u128 = 10u128.pow(PLMC_DECIMALS as u32);

pub trait ProvideAssetPrice {
	type AssetId;
	type Price: FixedPointNumber;

	/// Gets the nominal price of a given `asset_id`.
	///
	/// The nominal price is typically expressed in a standard currency (e.g., USD)
	/// per single whole unit of the asset (e.g., 2.5 USD for 1 DOT).
	/// This price does not yet account for the differing decimal precisions
	/// of the asset and the pricing currency.
	///
	/// # Arguments
	///
	/// * `asset_id`: The identifier of the asset for which to fetch the price.
	///
	/// # Returns
	///
	/// Returns `Some(Self::Price)` if the price is available, containing the nominal
	/// price as a `FixedPointNumber`.
	/// Returns `None` if the price for the given `asset_id` is not available or
	/// cannot be determined.
	fn get_price(asset_id: &Self::AssetId) -> Option<Self::Price>;

	/// Calculates a "decimals-aware" price from a nominal `original_price`.
	///
	/// The `original_price` is the commonly quoted price, e.g., "Asset X price is 2.5 USD".
	/// This function adjusts this `original_price` to account for the differing
	/// decimal precisions used by the asset and the pricing currency (assumed to be USD here).
	///
	/// The resulting "decimals-aware" price is a `FixedPointNumber` designed for direct
	/// calculation:
	/// `asset_amount_in_smallest_units * decimals_aware_price = usd_amount_in_smallest_units`
	///
	/// ### Example:
	///
	/// Given:
	/// - `original_price` = 2.5 (representing 2.5 USD for 1 whole unit of the asset).
	/// - `usd_decimals` = 6 (meaning 1 USD = 10^6 smallest USD units).
	/// - `asset_decimals` = 8 (meaning 1 Asset = 10^8 smallest asset units).
	///
	/// The `original_price` (2.5 USD / 1 Asset) is equivalent to:
	/// `(2.5 * 10^usd_decimals)` smallest USD units / `(1 * 10^asset_decimals)` smallest asset units
	/// `(2.5 * 10^6)` smallest USD units / `(1 * 10^8)` smallest asset units.
	///
	/// This function calculates the `decimals_aware_price` as:
	/// `original_price * (10^usd_decimals / 10^asset_decimals)`
	/// which simplifies to `original_price * 10^(usd_decimals - asset_decimals)` or
	/// `original_price / 10^(asset_decimals - usd_decimals)`.
	///
	/// For the example values: `2.5 / 10^(8 - 6) = 2.5 / 100 = 0.025`.
	///
	/// So, if you have 20 whole units of the asset:
	/// - Amount in smallest asset units = `20 * 10^asset_decimals = 20 * 10^8`.
	/// - Equivalent USD in smallest units = `(20 * 10^8) * 0.025 = 50 * 10^6`.
	/// - This is `50 * 10^6 / 10^6 = 50` USD.
	///
	/// # Arguments
	///
	/// * `original_price`: The nominal price of the asset (e.g., USD per whole asset unit).
	/// * `usd_decimals`: The number of decimal places for the USD (or pricing) currency.
	/// * `asset_decimals`: The number of decimal places for the asset.
	///
	/// # Returns
	///
	/// Returns `Some(Self::Price)` containing the decimals-aware price if the calculation
	/// is successful.
	/// Returns `None` if any intermediate calculation (like power or fixed-point
	/// conversion) fails, for example, due to overflow.
	fn calculate_decimals_aware_price(
		original_price: Self::Price,
		usd_decimals: u8,
		asset_decimals: u8,
	) -> Option<Self::Price> {
		let diff_exponent_abs: u32 = usd_decimals.abs_diff(asset_decimals).into();
		let scaling_factor_int = 10u128.checked_pow(diff_exponent_abs)?;

		// Convert the integer scaling factor to the fixed-point type.
		// This represents 10^|usd_decimals - asset_decimals|.
		let scaling_factor_fixed = Self::Price::checked_from_rational(scaling_factor_int, 1u128)?;

		if usd_decimals >= asset_decimals {
			// Equivalent to original_price * 10^(usd_decimals - asset_decimals)
			original_price.checked_mul(&scaling_factor_fixed)
		} else {
			// Equivalent to original_price / 10^(asset_decimals - usd_decimals)
			original_price.checked_div(&scaling_factor_fixed)
		}
	}

	/// Converts a "decimals-aware" price back to its nominal price.
	///
	/// This is the inverse operation of `calculate_decimals_aware_price`.
	/// Given a `decimals_aware_price` (which relates smallest units of an asset to
	/// smallest units of a pricing currency like USD), and the respective decimal counts,
	/// this function calculates the nominal price (e.g., USD per whole asset unit).
	///
	/// # Arguments
	///
	/// * `decimals_aware_price`: The price adjusted for decimal differences, typically
	///   obtained from `calculate_decimals_aware_price`.
	/// * `usd_decimals`: The number of decimal places for the USD (or pricing) currency.
	/// * `asset_decimals`: The number of decimal places for the asset.
	///
	/// # Returns
	///
	/// Returns `Some(Self::Price)` containing the nominal price if the calculation
	/// is successful.
	/// Returns `None` if any intermediate calculation fails (e.g., due to overflow).
	fn convert_back_to_normal_price(
		decimals_aware_price: Self::Price,
		usd_decimals: u8,
		asset_decimals: u8,
	) -> Option<Self::Price> {
		let abs_diff: u32 = asset_decimals.abs_diff(usd_decimals).into();
		let abs_diff_unit = 10u128.checked_pow(abs_diff)?;
		// We are pretty sure this is going to be representable because the number size is not the size of the asset decimals, but the difference between the asset and usd decimals
		let abs_diff_fixed = Self::Price::checked_from_rational(abs_diff_unit, 1)?;
		if usd_decimals > asset_decimals {
			decimals_aware_price.checked_div(&abs_diff_fixed)
		} else {
			decimals_aware_price.checked_mul(&abs_diff_fixed)
		}
	}

	/// Fetches the nominal price of an asset and then calculates its "decimals-aware" price.
	///
	/// This is a convenience method that combines `get_price` and
	/// `calculate_decimals_aware_price`.
	///
	/// # Arguments
	///
	/// * `asset_id`: The identifier of the asset.
	/// * `asset_decimals`: The number of decimal places for the asset.
	///
	/// # Returns
	///
	/// Returns `Some(Self::Price)` containing the decimals-aware price if both fetching
	/// the nominal price and the subsequent calculation are successful.
	/// Returns `None` if the nominal price cannot be fetched or if the decimals-aware
	/// calculation fails.
	fn get_decimals_aware_price(asset_id: &Self::AssetId, asset_decimals: u8) -> Option<Self::Price> {
		let original_price = Self::get_price(asset_id)?;
		Self::calculate_decimals_aware_price(original_price, USD_DECIMALS, asset_decimals)
	}
}

/// Rounds a given `u128` number to the nearest multiple of a `step`.
///
/// This is useful for rounding to a specific level of precision, e.g.,
/// rounding a token amount to the nearest 0.01 tokens.
///
/// Returns `None` if the `step` is zero or if any calculation results in an overflow.
///
/// # Arguments
///
/// * `number`: The number to round.
/// * `step`: The granularity to round to. The final result will be a multiple of this.
///
/// # Example
///
/// // Round 147 to the nearest 10 -> 150
/// assert_eq!(round_to_nearest(147, 10), Some(150));
/// // Round 142 to the nearest 10 -> 140
/// assert_eq!(round_to_nearest(142, 10), Some(140));
/// // Round 145 to the nearest 10 -> 150 (rounds half up)
/// assert_eq!(round_to_nearest(145, 10), Some(150));
pub fn round_to_nearest(number: u128, step: u128) -> Option<u128> {
	if step == 0 {
		return None;
	}

	let half_step = step.checked_div(2)?;

	let quotient = number.checked_div(step)?;
	let remainder = number.checked_rem(step)?;

	if remainder >= half_step {
		// Round up
		let next_quotient = quotient.checked_add(1)?;
		next_quotient.checked_mul(step)
	} else {
		// Round down
		quotient.checked_mul(step)
	}
}
