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

use crate::{deposit, Balance, PLMC};
use frame_support::parameter_types;
use parachains_common::AccountId;

parameter_types! {
	/// The basic deposit to create an identity.
	pub const BasicDeposit: Balance = 20 * PLMC;
	/// Deposit for each additional field.
	pub const ByteDeposit: Balance = deposit(0, 1);
	/// The deposit needed to create a registrar.
	pub const PendingUsernameExpiration: u32 = 0;
	/// The deposit needed to create a sub-account.
	/// We do not allow sub-accounts so can be 0.
	/// Should be set to a non-zero value if sub-accounts are allowed.
	pub const SubAccountDeposit: Balance = 0;
	/// Max number of additional fields that can be created.
	pub const MaxAdditionalFields: u32 = 100;
	/// Max number of registrars that can be set.
	pub const MaxRegistrars: u32 = 3;
}

#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
	/// Max number of sub-accounts that can be created.
	/// We do not allow sub-accounts so set to 0.
	pub const MaxSubAccounts: u32 = 0;
	/// Max length of username suffix.
	pub const MaxSuffixLength: u32 = 0;
	/// Max length of username.
	pub const MaxUsernameLength: u32 = 0;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const MaxSubAccounts: u32 = 1;
	pub const MaxSuffixLength: u32 = 24;
	pub const MaxUsernameLength: u32 = 32;
}

#[cfg(not(feature = "runtime-benchmarks"))]
pub type UsernameAuthorityOrigin = frame_system::EnsureNever<AccountId>;

#[cfg(feature = "runtime-benchmarks")]
pub type UsernameAuthorityOrigin = frame_system::EnsureRoot<AccountId>;
