#![cfg_attr(not(feature = "std"), no_std)]

use core::slice::Iter;
use frame_support::{
	pallet_prelude::{Decode, DispatchError, Encode, MaxEncodedLen, TypeInfo},
	BoundedVec, RuntimeDebug,
};
use sp_std::vec::Vec;

/// The various roles that a member can hold.
#[derive(Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
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
pub enum Big4 {
	Deloitte,
	PwC,
	EY,
	KPMG,
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
	pub issuer: Big4,
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
	fn get_roles_of(who: &AccountId) -> Vec<&MemberRole>;
}
