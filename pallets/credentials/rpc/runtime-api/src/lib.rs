#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use polimec_traits::MemberRole;

use sp_std::vec::Vec;

// Here we declare the runtime API.
// It is implemented it the `impl` block in runtime file (the `runtime-api/src/lib.rs`)
sp_api::decl_runtime_apis! {
	pub trait CredentialsApi<AccountId> where
	AccountId: Codec {
		fn is_in(role: MemberRole,  who: AccountId) -> bool;
		fn get_members_of(role: MemberRole) -> Vec<AccountId>;
		fn get_roles_of(who: AccountId) -> Vec<MemberRole>;
	}
}
