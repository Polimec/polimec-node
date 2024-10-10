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

use crate::{
	currency::{deposit, PLMC},
	Balance,
};
use core::marker::PhantomData;
use frame_support::{parameter_types, PalletId};
use orml_traits::DataProvider;
use parachains_common::DAYS;
use polimec_common::{credentials::Cid, ProvideAssetPrice};
use sp_arithmetic::FixedPointNumber;

parameter_types! {
	pub const AssetDeposit: Balance = 10  * PLMC;
	pub const AssetsStringLimit: u32 = 50;
	/// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
	pub const AssetAccountDeposit: Balance = deposit(1, 18);
	pub const ZeroDeposit: Balance = 0;
}

pub struct OraclePriceProvider<AssetId, Price, Oracle>(PhantomData<(AssetId, Price, Oracle)>);

impl<AssetId, Price, Oracle> ProvideAssetPrice for OraclePriceProvider<AssetId, Price, Oracle>
where
	Price: FixedPointNumber,
	Oracle: DataProvider<AssetId, Price>,
{
	type AssetId = AssetId;
	type Price = Price;

	fn get_price(asset_id: AssetId) -> Option<Price> {
		Oracle::get(&asset_id)
	}
}

// Dispenser Related constants
parameter_types! {
	pub const InitialDispenseAmount: Balance = 700 * PLMC;
	pub const FreeDispenseAmount: Balance = 5 * PLMC;
	pub const DispenserId: PalletId = PalletId(*b"plmc/fct");
	pub const DispenserLockPeriod: u32 = DAYS * 365 * 2; // 2 years
	pub const DispenserVestPeriod: u32 = DAYS * 365 * 2; // 2 years
	pub DispenserWhitelistedPolicy: Cid = (*b"QmVdGSxuWcamYEmYJjR3gvZucqQpp4Jnf6tqJABHwKZVo3").to_vec().try_into().unwrap();
}
