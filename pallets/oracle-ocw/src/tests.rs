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

#![cfg(test)]

use crate::{mock::*, types::*, traits::*};
use sp_runtime::RuntimeAppPublic;
use frame_support::{assert_noop, assert_ok};


#[test]
fn call_offchain_worker() {
	let (mut ext, offchain_state) = new_test_ext_with_offchain_storage();
	price_oracle_response(&mut offchain_state.write());
	ext.execute_with(|| {
		run_to_block(1);
	});
}
