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

// TODO: To be removed after "The Merge"

// -->
use core::slice::Iter;
use frame_support::{pallet_prelude::*, traits::tokens::fungible, BoundedVec, RuntimeDebug};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;

/// The various roles that a member can hold.
#[derive(Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
pub enum MemberRole {
	Issuer,
	Retail,
	Professional,
	Institutional,
}

impl MemberRole {
	pub fn iterator() -> Iter<'static, MemberRole> {
		static ROLES: [MemberRole; 4] =
			[MemberRole::Issuer, MemberRole::Retail, MemberRole::Professional, MemberRole::Institutional];
		ROLES.iter()
	}
}

/// The various attesters on KILT.
#[derive(Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum Issuers {
	IssuerOne,
	IssuerTwo,
	IssuerThree,
	IssuerFour,
}

#[derive(Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum Country {
	Switzerland,
	UnitedStates,
}

// TODO: Set this at runtime
type MaxDomicile = frame_support::traits::ConstU32<255>;

/// A basic "credential" representation
#[derive(Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct Credential {
	pub issuer: Issuers,
	pub role: MemberRole,
	pub domicile: BoundedVec<u8, MaxDomicile>,
	pub country: Country,
	// TODO: Find a way to handle the date of birth
	pub date_of_birth: u32,
}

pub trait PolimecMembers<AccountId> {
	fn is_in(role: &MemberRole, who: &AccountId) -> bool;
	fn add_member(role: &MemberRole, who: &AccountId) -> Result<(), DispatchError>;
	fn initialize_members(role: &MemberRole, members: &[AccountId]);
	fn get_members_of(role: &MemberRole) -> Vec<AccountId>;
	fn get_roles_of(who: &AccountId) -> Vec<MemberRole>;
}

// <--

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
	fn vesting_balance(
		who: &AccountId,
		reason: Reason,
	) -> Option<<Self::Currency as fungible::Inspect<AccountId>>::Balance>;

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
