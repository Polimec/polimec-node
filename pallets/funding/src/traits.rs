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

use crate::{BalanceOf, Config, ProjectId};
use frame_support::weights::Weight;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_arithmetic::{
	traits::{CheckedDiv, CheckedMul},
	FixedPointNumber,
};
use sp_runtime::DispatchError;

pub trait BondingRequirementCalculation {
	fn calculate_bonding_requirement<T: Config>(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait VestingDurationCalculation {
	fn calculate_vesting_duration<T: Config>(&self) -> BlockNumberFor<T>;
}

pub trait ProvideAssetPrice {
	type AssetId;
	type Price: FixedPointNumber;
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
		let abs_diff_unit = 10u128.pow(abs_diff);
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

pub trait DoRemainingOperation<T: Config> {
	fn has_remaining_operations(&self) -> bool;

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError>;
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub trait SetPrices {
	fn set_prices();
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
impl SetPrices for () {
	fn set_prices() {}
}
