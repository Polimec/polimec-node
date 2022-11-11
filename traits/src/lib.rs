#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::{Decode, DispatchError, Encode, MaxEncodedLen, TypeInfo},
	BoundedVec, RuntimeDebug,
};

/// The various roles that a member can hold.
#[derive(Default, Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum MemberRole {
	#[default]
	Issuer,
	Retail,
	Professional,
	Institutional,
}


/// The various attesters on KILT.
#[derive(Default, Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum Big4 {
	#[default]
	Deloitte,
	PwC,
	EY,
	KPMG,
}

#[derive(Default, Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum Country {
	#[default]
	Switzerland,
	UnitedStates,
}

// TODO: Set this at runtime
type MaxDomicile = frame_support::traits::ConstU32<255>;

/// A basic "credential" representation
#[derive(Default, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct Credential {
	// TODO: Use getter instead of pub?
	pub issuer: Big4,
	pub role: MemberRole,
	pub domicile: BoundedVec<u8, MaxDomicile>,
	pub country: Country,
	pub date_of_birth: u32,
}

pub trait PolimecMembers<AccountId> {
	fn is_in(who: &AccountId, role: &MemberRole) -> bool;
	fn add_member(who: &AccountId, role: &MemberRole) -> Result<(), DispatchError>;
	fn initialize_members(members: &[AccountId], role: &MemberRole);
}
