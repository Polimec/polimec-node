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

use core::slice::Iter;
use frame_support::{
	pallet_prelude::{Decode, DispatchError, Encode, MaxEncodedLen, TypeInfo},
	BoundedVec, RuntimeDebug,
};
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
		static ROLES: [MemberRole; 4] = [
			MemberRole::Issuer,
			MemberRole::Retail,
			MemberRole::Professional,
			MemberRole::Institutional,
		];
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
