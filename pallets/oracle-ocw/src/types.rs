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
use core::{ops::Mul, str::FromStr};
use heapless::{LinearMap, Vec as HVec};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::Deserialize;
use sp_core::{offchain::HttpRequestId as RequestId, RuntimeDebug};
use sp_runtime::{FixedPointNumber, Saturating};
use sp_std::vec::Vec;
use substrate_fixed::{traits::ToFixed, types::U100F28};

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

#[derive(Debug, Clone)]
pub(crate) struct OpenCloseVolume {
	pub high: FixedU128,
	pub low: FixedU128,
	pub close: FixedU128,
	pub volume: FixedU128,
}

impl OpenCloseVolume {
	pub fn vwp(&self) -> FixedU128 {
		let avg_price = (self.high.saturating_add(self.low).saturating_add(self.close)) / FixedU128::from_u32(3u32);
		self.volume.saturating_mul(avg_price)
	}

	pub fn from_u100f28(high: U100F28, low: U100F28, close: U100F28, volume: U100F28) -> Self {
		let high_as_fixedu128_inner: u128 = high.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		let low_as_fixedu128_inner: u128 = low.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		let close_as_fixedu128_inner: u128 = close.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		let volume_as_fixedu128_inner: u128 = volume.mul(U100F28::from_num(FixedU128::accuracy())).to_num::<u128>();
		OpenCloseVolume {
			high: FixedU128::from_inner(high_as_fixedu128_inner),
			low: FixedU128::from_inner(low_as_fixedu128_inner),
			close: FixedU128::from_inner(close_as_fixedu128_inner),
			volume: FixedU128::from_inner(volume_as_fixedu128_inner),
		}
	}

	pub fn from_f64(high: f64, low: f64, close: f64, volume: f64) -> Result<Self, ()> {
		let high = high.checked_to_fixed::<U100F28>().ok_or(())?;
		let low = low.checked_to_fixed::<U100F28>().ok_or(())?;
		let close = close.checked_to_fixed::<U100F28>().ok_or(())?;
		let volume = volume.checked_to_fixed::<U100F28>().ok_or(())?;
		Ok(Self::from_u100f28(high, low, close, volume))
	}

	pub fn from_str(high: &str, low: &str, close: &str, volume: &str) -> Result<Self, <U100F28 as FromStr>::Err> {
		let high = U100F28::from_str(high)?;
		let low = U100F28::from_str(low)?;
		let close = U100F28::from_str(close)?;
		let volume = U100F28::from_str(volume)?;
		Ok(Self::from_u100f28(high, low, close, volume))
	}
}

fn deserialize_hloc_kraken<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let data = HVec::<(u64, &str, &str, &str, &str, &str, &str, u64), 720>::deserialize(deserializer)?;
	let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
	for row in data.into_iter() {
		let ocv = OpenCloseVolume::from_str(row.2, row.3, row.4, row.6)
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
			log::error!(target: LOG_TARGET, "Error parsing response for Kraken: {:?}", e);
			return None;
		}
		let response = maybe_response.ok()?;
		Some(response.0.result.data.into_iter().rev().take(NUMBER_OF_CANDLES).collect())
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://api.kraken.com/0/public/OHLC?pair=USDTZUSD&interval=1",
			AssetName::DOT => "https://api.kraken.com/0/public/OHLC?pair=DOTUSD&interval=1",
			AssetName::USDC => "https://api.kraken.com/0/public/OHLC?pair=USDCUSD&interval=1",
			_ => "",
		}
	}
}

pub(crate) struct BitFinexFetcher;
impl FetchPrice for BitFinexFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<HVec<(u64, f64, f64, f64, f64, f64), NUMBER_OF_CANDLES>>(body);
		if let Err(e) = maybe_response {
			log::error!(target: LOG_TARGET, "Error parsing response for BitFinex: {:?}", e);
			return None;
		}
		let response = maybe_response.ok()?;

		let data: Vec<OpenCloseVolume> =
			response.0.into_iter().filter_map(|r| OpenCloseVolume::from_f64(r.3, r.4, r.2, r.5).ok()).collect();
		if data.len() < NUMBER_OF_CANDLES {
			return None;
		}
		Some(data)
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtUSTUSD/hist?limit=15",
			AssetName::DOT => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtDOTUSD/hist?limit=15",
			AssetName::USDC => "https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3AtUDCUSD/hist?limit=15",
			_ => "",
		}
	}
}

fn deserialize_hloc_bitstamp<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let data = HVec::<LinearMap<&str, &str, 6>, NUMBER_OF_CANDLES>::deserialize(deserializer)?;
	let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
	let high_str = "high";
	let low_str = "low";
	let close_str = "close";
	let volume_str = "volume";
	for row in data.into_iter() {
		if !row.contains_key(&high_str) && !row.contains_key(&close_str) && !row.contains_key(&volume_str) {
			return Err(serde::de::Error::custom("Row does not contain required data"));
		}
		let high = *row.get(&high_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let low = *row.get(&low_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let close = *row.get(&close_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let volume = *row.get(&volume_str).ok_or(serde::de::Error::custom("Could not parse value to str"))?;
		let ocv = OpenCloseVolume::from_str(high, low, close, volume)
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
			log::error!(target: LOG_TARGET, "Error parsing response for Bitstamp: {:?}", e);
			return None;
		}
		let response = maybe_response.ok()?;
		if response.0.data.ohlc.len() < NUMBER_OF_CANDLES {
			return None;
		}

		Some(response.0.data.ohlc.into_iter().rev().collect())
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::USDT => "https://www.bitstamp.net/api/v2/ohlc/usdtusd/?step=60&limit=15",
			AssetName::DOT => "https://www.bitstamp.net/api/v2/ohlc/dotusd/?step=60&limit=15",
			AssetName::USDC => "https://www.bitstamp.net/api/v2/ohlc/usdcusd/?step=60&limit=15",
			_ => "",
		}
	}
}

pub(crate) struct CoinbaseFetcher;
impl FetchPrice for CoinbaseFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<HVec<(u64, f64, f64, f64, f64, f64), 1000>>(body);
		if let Err(e) = maybe_response {
			log::error!(target: LOG_TARGET, "Error parsing response for Coinbase: {:?}", e);
			return None;
		}
		let response = maybe_response.ok()?;

		let data: Vec<OpenCloseVolume> = response
			.0
			.into_iter()
			.take(NUMBER_OF_CANDLES)
			.filter_map(|r| OpenCloseVolume::from_f64(r.2, r.1, r.4, r.5).ok())
			.collect();
		if data.len() < NUMBER_OF_CANDLES {
			return None;
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

#[derive(Default, Deserialize, RuntimeDebug)]
struct XTCandle<'a> {
	t: u64,
    o: &'a str,
    c: &'a str,
    h: &'a str,
    l: &'a str,
    q: &'a str,
    v: &'a str,
}

fn deserialize_hloc_xt<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	let data = HVec::<XTCandle, 10>::deserialize(deserializer)?;
	let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
	for row in data.into_iter() {
		let ocv = OpenCloseVolume::from_str(row.h, row.l, row.c, row.v)
			.map_err(|_| serde::de::Error::custom("Error parsing float"))?;
		result.push(ocv);
	}
	Ok(result)
}

#[derive(Deserialize, RuntimeDebug)]
struct XTResponse {
	#[serde(skip)]
	_rc: u8,
	#[serde(skip)]
	_mc: Vec<u8>,
	#[serde(skip)]
	_ma: Vec<u8>,
	#[serde(deserialize_with = "deserialize_hloc_xt")]
	result: Vec<OpenCloseVolume>,
}
pub(crate) struct XTFetcher;
impl FetchPrice for XTFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<XTResponse>(body);
		if let Err(e) = maybe_response {
			log::error!(target: LOG_TARGET, "Error parsing response for XT: {:?}", e);
			return None
		}
		let response = maybe_response.ok()?;

		Some(response.0.result)
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::PLMC => "https://sapi.xt.com/v4/public/kline?symbol=plmc_usdt&interval=15m&limit=10",
			_ => "",
		}
	}
}

pub(crate) struct MexcFetcher;
impl FetchPrice for MexcFetcher {
	fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
		let maybe_response = serde_json_core::from_str::<HVec<(u64, &str, &str, &str, &str, &str, u64, &str), 10>>(body);
		if let Err(e) = maybe_response {
			dbg!(e);
			log::error!(target: LOG_TARGET, "Error parsing response for BitFinex: {:?}", e);
			return None;
		}
		let response = maybe_response.ok()?;
		let data: Vec<OpenCloseVolume> =
			response.0.into_iter().filter_map(|r| OpenCloseVolume::from_str(r.2, r.3, r.4, r.5).ok()).collect();
		if data.len() < 10 {
			return None;
		}
		Some(data)
	}

	fn get_url(name: AssetName) -> &'static str {
		match name {
			AssetName::PLMC => "https://api.mexc.com/api/v3/klines?symbol=PLMCUSDT&interval=15m&limit=10",
			_ => "",
		}
	}
}
