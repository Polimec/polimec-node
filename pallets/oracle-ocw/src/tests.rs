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
	types::{AssetName, BitFinexFetcher, BitStampFetcher, CoinbaseFetcher, KrakenFetcher, MexcFetcher, XTFetcher},
};
use parity_scale_codec::Decode;
use polimec_common::do_request;
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
						10 => assert_close_enough(price, FixedU128::from_float(6.138485575453039783)),
						1984 => assert_close_enough(price, FixedU128::from_float(1.000154206100002620)),
						1337 => assert_close_enough(price, FixedU128::from_float(1.000093378020633965)),
						3344 => assert_close_enough(price, FixedU128::from_float(0.414564170729477207)),
						_ => panic!("Unexpected asset"),
					}
				},
			_ => panic!("Unexpected call"),
		}
	});
}

fn test_fetcher_against_real_api<F: FetchPrice>() {
	for asset in vec![AssetName::DOT, AssetName::USDC, AssetName::USDT, AssetName::PLMC] {
		let url = F::get_url(asset);
		if url == "" {
			continue;
		}
		let body = do_request(url);
		let data = F::parse_body(&body);
		assert!(data.is_some());
	}
}

#[test]
fn test_coinbase_against_real_api() {
	test_fetcher_against_real_api::<CoinbaseFetcher>();
}

#[test]
fn test_kraken_against_real_api() {
	test_fetcher_against_real_api::<KrakenFetcher>();
}

#[test]
fn test_bitfinex_against_real_api() {
	test_fetcher_against_real_api::<BitFinexFetcher>();
}

#[test]
fn test_bitstamp_against_real_api() {
	test_fetcher_against_real_api::<BitStampFetcher>();
}

#[test]
fn test_xt_against_real_api() {
	test_fetcher_against_real_api::<XTFetcher>();
}

#[test]
fn test_mexc_against_real_api() {
	test_fetcher_against_real_api::<MexcFetcher>();
}
