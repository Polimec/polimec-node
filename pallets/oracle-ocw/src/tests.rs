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


use crate::mock::*;
use sp_runtime::FixedU128;
use parity_scale_codec::Decode;


#[test]
fn call_offchain_worker() {
	let (mut ext, offchain_state, pool_state) = new_test_ext_with_offchain_storage();
	price_oracle_response(&mut offchain_state.write());
	ext.execute_with(|| {
		run_to_block(6);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature.unwrap().0, 0);

		match tx.call {
			RuntimeCall::Oracle(orml_oracle::Call::feed_values { values }) => {
				for (asset, price) in values {
					match asset {
						0 => assert_close_enough(price, FixedU128::from_float(5.519610553825360850)),
						1984 => assert_close_enough(price, FixedU128::from_float(1.000692308098215370)),
						420 => assert_close_enough(price, FixedU128::from_float(1.000198559694455204)),
						_ => panic!("Unexpected asset"),
					}
				}
				
			},
			_ => panic!("Unexpected call"),
		}
		
	});
}
