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
use super::*;
use core::{str::FromStr, ops::Mul};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::Deserialize;
use sp_core::offchain::HttpRequestId as RequestId;
use sp_runtime::{Saturating, FixedPointNumber};
use heapless::{Vec as HVec, LinearMap};
use sp_std::vec::Vec;
use substrate_fixed::{types::U100F28, traits::ToFixed};


#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord, Encode, Decode, TypeInfo)]
pub enum AssetName {
	USDT,
	USDC,
	DOT,
	PLMC,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AssetRequest {
	pub asset: AssetName,
	pub id: RequestId,
}

#[derive(Debug)]
pub(crate) struct OpenCloseVolume {
	pub open: FixedU128,
	pub close: FixedU128,
	pub volume: FixedU128,
}

impl OpenCloseVolume {
	pub fn vwp(&self) -> FixedU128 {
		let avg_price = self.open.saturating_add(self.close) / FixedU128::from_u32(2u32);
		self.volume.saturating_mul(avg_price)
	}

	pub fn from_u100f28(open: U100F28, close: U100F28, volume: U100F28) -> Self {
		let open_as_fixedu128_inner: u128 = open.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		let close_as_fixedu128_inner: u128 = close.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		let volume_as_fixedu128_inner: u128 = volume.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		OpenCloseVolume {
			open: FixedU128::from_inner(open_as_fixedu128_inner),
			close: FixedU128::from_inner(close_as_fixedu128_inner),
			volume: FixedU128::from_inner(volume_as_fixedu128_inner),
		}
	}

	pub fn from_f64(open: f64, close: f64, volume: f64) -> Result<Self, ()> {
		let open = open.checked_to_fixed::<U100F28>().ok_or(())?;
		let close = close.checked_to_fixed::<U100F28>().ok_or(())?;
		let volume = volume.checked_to_fixed::<U100F28>().ok_or(())?;
		Ok(Self::from_u100f28(open, close, volume))
	}

	pub fn from_str(open: &str, close: &str, volume: &str) -> Result<Self, <U100F28 as FromStr>::Err> {
		let open = U100F28::from_str(open)?;
		let close = U100F28::from_str(close)?;
		let volume = U100F28::from_str(volume)?;
		Ok(Self::from_u100f28(open, close, volume))
	}
}



fn deserialize_hloc_kraken<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let data = HVec::<(u64, &str, &str, &str, &str, &str, &str, u64), 100>::deserialize(deserializer)?;
	let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
	for row in data.into_iter() {
		let ocv = OpenCloseVolume::from_str(row.1, row.4, row.6)
			.map_err(|_| serde::de::Error::custom("Error parsing float"))?;
		result.push(ocv);
	}
	Ok(result)
}

#[derive(Default, Deserialize)]
struct KrakenResult {
	#[serde(alias = "USDTZUSD")]
	#[serde(alias = "DOTUSD")]
	#[serde(alias = "USDCUSD")]
	#[serde(deserialize_with = "deserialize_hloc_kraken")]
	data: Vec<OpenCloseVolume>,
	#[serde(skip)]
	_last: Vec<u8>,
}
#[derive(Deserialize)]
struct KrakenResponse {
	#[serde(skip)]
	_error: Vec<u8>,
	#[serde(default)]
	result: KrakenResult,
}
pub(crate) struct KrakenFetcher;
impl FetchPrice for KrakenFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<KrakenResponse>(body);
		if let Err(e) = maybe_response {
			panic!("Error parsing response: {}", e);
		}
		let response = maybe_response.ok()?;
		Some(response.0.result.data.into_iter().rev().take(10).collect())
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://api.kraken.com/0/public/OHLC?pair=USDTZUSD",
			AssetName::DOT => "https://api.kraken.com/0/public/OHLC?pair=DOTUSD",
			AssetName::USDC => "https://api.kraken.com/0/public/OHLC?pair=USDCUSD",
			_ => "",
		}
	}
}

pub(crate) struct BitFinexFetcher;
impl FetchPrice for BitFinexFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<HVec<(u64, f64, f64, f64, f64, f64), 10>>(body);
		if let Err(e) = maybe_response {
			panic!("Error parsing response: {:?}", e);
		}
		let response = maybe_response.ok()?;

		let data: Vec<OpenCloseVolume> = response.0.into_iter()
			.filter_map(|r|
				OpenCloseVolume::from_f64(r.1, r.2, r.5).ok()
			).collect();
		if data.len() < 10 {
			return None
		}
		Some(data)
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtUSTUSD/hist?limit=10",
			AssetName::DOT => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtDOTUSD/hist?limit=10",
			AssetName::USDC => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtUDCUSD/hist?limit=10",
			_ => "",
		}
	}
}

fn deserialize_hloc_bitstamp<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let data = HVec::<LinearMap<&str, &str, 6>, 10>::deserialize(deserializer)?;
	let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
	let open_str  = "open";
	let close_str= "close";
	let volume_str = "volume";
	for row in data.into_iter() {
		if !row.contains_key(&open_str) && !row.contains_key(&close_str) && !row.contains_key(&volume_str) {
			return Err(serde::de::Error::custom("Row does not contain required data"))
		}
		let open = *row.get(&open_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let close = *row.get(&close_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let volume = *row.get(&volume_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let ocv = OpenCloseVolume::from_str(open, close, volume)
			.map_err(|_| serde::de::Error::custom("Error parsing float"))?;
		result.push(ocv);
	}
	Ok(result)
}

#[derive(Deserialize, Default)]
struct BitStampResult {
	#[serde(deserialize_with = "deserialize_hloc_bitstamp")]
	ohlc: Vec<OpenCloseVolume>,
	#[serde(skip)]
	_pair: Vec<u8>,
}

#[derive(Deserialize, Default)]
struct BitStampResponse {
	#[serde(default)]
	data: BitStampResult,
}

pub(crate) struct BitStampFetcher;
impl FetchPrice for BitStampFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<BitStampResponse>(body);
		if let Err(e) = maybe_response {
			panic!("Error parsing response: {:?}", e);
		}
		let response = maybe_response.ok()?;
		if response.0.data.ohlc.len() < 10 {
			return None
		}

		Some(response.0.data.ohlc.into_iter().rev().collect())
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://www.bitstamp.net/api/v2/ohlc/usdtusd/?step=60&limit=10",
			AssetName::DOT => "https://www.bitstamp.net/api/v2/ohlc/dotusd/?step=60&limit=10",
			AssetName::USDC => "https://www.bitstamp.net/api/v2/ohlc/usdcusd/?step=60&limit=10",
			_ => "",
		}
	}
}

pub(crate) struct CoinbaseFetcher;
impl FetchPrice for CoinbaseFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<HVec<(u64, f64, f64, f64, f64, f64), 10>>(body);
		if let Err(e) = maybe_response {
			panic!("Error parsing response: {:?}", e);
		}
		let response = maybe_response.ok()?;
		if response.0.len() < 10 {
			return None
		}

		let data: Vec<OpenCloseVolume> = response.0.into_iter().take(10)
			.filter_map(|r|
				OpenCloseVolume::from_f64(r.3, r.4, r.5).ok()
			).collect();
		if data.len() < 10 {
			return None
		}
		Some(data)
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://api.exchange.coinbase.com/products/USDT-USD/candles?granularity=60",
			AssetName::DOT => "https://api.exchange.coinbase.com/products/DOT-USD/candles?granularity=60",
			AssetName::USDC => "",
			_ => "",
		}
	}
}
