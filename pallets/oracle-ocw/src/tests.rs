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

use crate::{
	mock::*,
	traits::FetchPrice,
	types::{BitFinexFetcher, BitStampFetcher, CoinbaseFetcher, KrakenFetcher},
};
use parity_scale_codec::Decode;
use sp_runtime::FixedU128;

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
			RuntimeCall::Oracle(orml_oracle::Call::feed_values { values }) =>
				for (asset, price) in values {
					match asset {
						0 => assert_close_enough(price, FixedU128::from_float(6.138485575453039783)),
						1984 => assert_close_enough(price, FixedU128::from_float(1.000154206100002620)),
						420 => assert_close_enough(price, FixedU128::from_float(1.000093378020633965)),
						_ => panic!("Unexpected asset"),
					}
				},
			_ => panic!("Unexpected call"),
		}
	});
}

#[test]
fn kraken_parser() {
	for (_, body_str) in KRAKEN_RESPONSES.iter() {
		let body = std::str::from_utf8(body_str).unwrap();
		let data = KrakenFetcher::parse_body(body);
		assert!(data.is_some());
	}
}

#[test]
fn bitfinex_parser() {
	for (_, body_str) in BITFINEX_RESPONSES.iter() {
		let body = std::str::from_utf8(body_str).unwrap();
		let data = BitFinexFetcher::parse_body(body);
		assert!(data.is_some());
	}
}

#[test]
fn bitstamp_parser() {
	for (_, body_str) in BITSTAMP_RESPONSES.iter() {
		let body = std::str::from_utf8(body_str).unwrap();
		let data = BitStampFetcher::parse_body(body);
		assert!(data.is_some());
	}
}

#[test]
fn coinbase_parser() {
	for (_, body_str) in COINBASE_RESPONSES.iter() {
		let body = std::str::from_utf8(body_str).unwrap();
		let data = CoinbaseFetcher::parse_body(body);
		assert!(data.is_some());
	}
}
