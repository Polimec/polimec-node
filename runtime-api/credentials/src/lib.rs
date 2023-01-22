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

// If you feel like getting in touch with us, you can do so at info@polimec.org

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
