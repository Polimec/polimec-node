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
use sp_runtime::{
	traits::{CheckedDiv, CheckedMul},
	FixedPointNumber, RuntimeDebug,
};
use sp_std::prelude::*;
pub use xcm::v4::{opaque::Xcm, Assets, Location, QueryId, SendError, SendResult, SendXcm, XcmHash};

pub mod assets;
pub mod credentials;

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

	#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum MigrationStatus {
		NotStarted,
		Sent(QueryId),
		Confirmed,
		Failed,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct Migration {
		pub origin: MigrationOrigin,
		pub info: MigrationInfo,
	}

	impl Migration {
		pub fn new(origin: MigrationOrigin, info: MigrationInfo) -> Self {
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
		pub fn new() -> Self {
			Self(Vec::new())
		}

		pub fn inner(self) -> Vec<Migration> {
			self.0
		}

		pub fn push(&mut self, migration: Migration) {
			self.0.push(migration)
		}

		pub fn from(migrations: Vec<Migration>) -> Self {
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

pub trait ProvideAssetPrice {
	type AssetId;
	type Price: FixedPointNumber;
	/// Gets the price of an asset.
	///
	/// Returns `None` if the price is not available.
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;

	/// Prices define the relationship between USD/Asset. When to and from that asset, we need to be aware that they might
	/// have different decimals. This function calculates the relationship having in mind the decimals. For example:
	/// if the price is 2.5, our underlying USD unit has 6 decimals, and the asset has 8 decimals, the price will be
	/// calculated like so: `(2.5USD * 10^6) / (1 * 10^8) = 0.025`. And so if we want to convert 20 of the asset to USD,
	/// we would do `0.025(USD/Asset)FixedPointNumber * 20_000_000_00(Asset)u128 = 50_000_000` which is 50 USD with 6 decimals
	fn calculate_decimals_aware_price(
		original_price: Self::Price,
		usd_decimals: u8,
		asset_decimals: u8,
	) -> Option<Self::Price> {
		let usd_unit = 10u128.checked_pow(usd_decimals.into())?;
		let usd_price_with_decimals = original_price.checked_mul_int(usd_unit)?;
		let asset_unit = 10u128.checked_pow(asset_decimals.into())?;

		Self::Price::checked_from_rational(usd_price_with_decimals, asset_unit)
	}

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

	fn get_decimals_aware_price(asset_id: Self::AssetId, usd_decimals: u8, asset_decimals: u8) -> Option<Self::Price> {
		let original_price = Self::get_price(asset_id)?;
		Self::calculate_decimals_aware_price(original_price, usd_decimals, asset_decimals)
	}
}
