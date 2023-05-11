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

use sp_std::{borrow::Borrow, marker::PhantomData};
use xcm::latest::{AssetId::Concrete, Fungibility::Fungible, MultiAsset, MultiLocation};
use xcm_executor::traits::{Convert, Error as MatchError, MatchesFungibles};

pub struct AsAssetMultiLocation<AssetId, AssetIdInfoGetter>(
	PhantomData<(AssetId, AssetIdInfoGetter)>,
);
impl<AssetId, AssetIdInfoGetter> xcm_executor::traits::Convert<MultiLocation, AssetId>
	for AsAssetMultiLocation<AssetId, AssetIdInfoGetter>
where
	AssetId: Clone,
	AssetIdInfoGetter: AssetMultiLocationGetter<AssetId>,
{
	fn convert_ref(asset_multi_location: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
		AssetIdInfoGetter::get_asset_id(asset_multi_location.borrow().clone()).ok_or(())
	}

	fn reverse_ref(asset_id: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
		AssetIdInfoGetter::get_asset_multi_location(asset_id.borrow().clone()).ok_or(())
	}
}

pub trait AssetMultiLocationGetter<AssetId> {
	fn get_asset_multi_location(asset_id: AssetId) -> Option<MultiLocation>;
	fn get_asset_id(asset_multi_location: MultiLocation) -> Option<AssetId>;
}

pub struct ConvertedRegisteredAssetId<AssetId, Balance, ConvertAssetId, ConvertBalance>(
	PhantomData<(AssetId, Balance, ConvertAssetId, ConvertBalance)>,
);
impl<
		AssetId: Clone,
		Balance: Clone,
		ConvertAssetId: Convert<MultiLocation, AssetId>,
		ConvertBalance: Convert<u128, Balance>,
	> MatchesFungibles<AssetId, Balance>
	for ConvertedRegisteredAssetId<AssetId, Balance, ConvertAssetId, ConvertBalance>
{
	fn matches_fungibles(a: &MultiAsset) -> Result<(AssetId, Balance), MatchError> {
		let (amount, id) = match (&a.fun, &a.id) {
			(Fungible(ref amount), Concrete(ref id)) => (amount, id),
			_ => return Err(MatchError::AssetNotHandled),
		};
		let what = ConvertAssetId::convert_ref(id).map_err(|_| MatchError::AssetNotHandled)?;
		let amount = ConvertBalance::convert_ref(amount)
			.map_err(|_| MatchError::AmountToBalanceConversionFailed)?;
		Ok((what, amount))
	}
}
