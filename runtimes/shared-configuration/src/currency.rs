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

use crate::Balance;
use frame_support::parameter_types;
use pallet_funding::AcceptedFundingAsset;
use pallet_oracle_ocw::types::AssetName;
use parachains_common::AssetIdForTrustBackedAssets as AssetId;
use sp_runtime::{traits::Convert, FixedU128};

/// One PLMC
pub const PLMC: Balance = 10u128.pow(10);

/// 0.001 PLMC
pub const MILLI_PLMC: Balance = 10u128.pow(7);

/// 0.000_001 PLMC
pub const MICRO_PLMC: Balance = 10u128.pow(4);

pub const EXISTENTIAL_DEPOSIT: Balance = 10 * MILLI_PLMC;

/// Deposit that must be provided for each occupied storage item.
pub const DEPOSIT_STORAGE_ITEM: Balance = 56 * MILLI_PLMC;
/// Deposit that must be provided for each occupied storage byte.
pub const DEPOSIT_STORAGE_BYTE: Balance = 100 * MICRO_PLMC;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * DEPOSIT_STORAGE_ITEM + (bytes as Balance) * DEPOSIT_STORAGE_BYTE) / 100
}

#[inline(always)]
pub const fn free_deposit() -> Balance {
	deposit(0, 0)
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICRO_PLMC;
	pub const DepositBase: Balance = DEPOSIT_STORAGE_ITEM;
	pub const DepositFactor: Balance = DEPOSIT_STORAGE_BYTE;
	pub const MaxSignatories: u32 = 64;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

// Required for the treasury payout benchmark, as it does a transfer under the normal ED.
#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

pub mod vesting {
	use frame_support::traits::WithdrawReasons;

	use super::*;

	parameter_types! {
		pub const MinVestedTransfer: Balance = PLMC;
		pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
			WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	}
}

pub type Price = FixedU128;

pub type Moment = u64;

pub struct AssetPriceConverter;
impl Convert<(AssetName, FixedU128), (AssetId, Price)> for AssetPriceConverter {
	fn convert((asset, price): (AssetName, FixedU128)) -> (AssetId, Price) {
		match asset {
			AssetName::DOT => (AcceptedFundingAsset::DOT.to_assethub_id(), price),
			AssetName::USDC => (AcceptedFundingAsset::USDC.to_assethub_id(), price),
			AssetName::USDT => (AcceptedFundingAsset::USDT.to_assethub_id(), price),
			AssetName::PLMC => (pallet_funding::PLMC_FOREIGN_ID, price),
		}
	}
}
