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
use parity_scale_codec::{Encode, Decode};
use scale_info::TypeInfo;
use serde_json::Value;
use sp_std::{vec::Vec, collections::btree_map::BTreeMap};
use sp_runtime::{Saturating, offchain::http::Response};
use sp_core::offchain::HttpRequestId as RequestId;
use serde::{Deserialize};
use core::str::FromStr;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord, Encode, Decode, TypeInfo)]
pub enum AssetName {
    USDT,
    USDC,
    DOT,
    PLMC,
}

#[derive(Debug, Clone, Copy)]
pub (crate) struct AssetRequest {
    pub asset: AssetName,
    pub id: RequestId,
}

pub (crate) struct AssetResponse {
    pub asset: AssetName,
    pub response: Response,
}

pub (crate) struct OpenCloseVolume {
    pub open: FixedU128,
    pub close: FixedU128,
    pub volume: FixedU128,
}

impl OpenCloseVolume {
    pub fn vwp(&self) -> FixedU128 {
        let avg_price = self.open.saturating_add(self.close) / FixedU128::from_u32(2u32);
        self.volume.saturating_mul(avg_price)
    }

    pub fn new(open: FixedU128, close: FixedU128, volume: FixedU128) -> Self {
        OpenCloseVolume {
            open,
            close,
            volume,
        }
    }

    pub fn from_f64(open: f64, close: f64, volume: f64) -> Self {
        OpenCloseVolume {
            open: FixedU128::from_float(open),
            close: FixedU128::from_float(close),
            volume: FixedU128::from_float(volume),
        }
    }

    pub fn from_str(open: &str, close: &str, volume: &str) -> Result<Self, <f64 as FromStr>::Err> {
        let open = f64::from_str(open)?;
        let close = f64::from_str(close)?;
        let volume = f64::from_str(volume)?;
        Ok(OpenCloseVolume::from_f64(open, close, volume))
    }
}



fn deserialize_hloc_kraken<'de, D>(deserializer: D) -> Result<Vec<OpenCloseVolume>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let data = Vec::<Vec<Value>>::deserialize(deserializer)?;
    let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
    for row in data.into_iter() {
        if row.len() < 8 {
            return Err(serde::de::Error::custom("Row does not have enough data"));
        }
        let open = row[1].as_str().ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let close = row[1].as_str().ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let volume = row[1].as_str().ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let ocv = OpenCloseVolume::from_str(open, close, volume).map_err(|_| serde::de::Error::custom("Error parsing float"))?;
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
    last: String,
}
#[derive(Deserialize)]
struct KrakenResponse {
    #[serde(skip)]
    error: Vec<String>,
    #[serde(default)]
    result: KrakenResult,
}
pub (crate) struct KrakenFetcher;
impl FetchPrice for KrakenFetcher {

    fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
        let maybe_response = serde_json::from_str::<KrakenResponse>(body);
        if let Err(e) = maybe_response {
            panic!("Error parsing response: {:?}", e);
        }
        let response = maybe_response.ok()?;
        Some(response.result.data.into_iter().rev().take(10).collect())
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

#[derive(Default, Deserialize)]
struct BitFinexOHLC {
    _timestamp: u64,
    pub open: f64,
    pub close: f64,
    _high: f64,
    _low:  f64,
    pub volume: f64,
}
pub (crate) struct BitFinexFetcher;
impl FetchPrice for BitFinexFetcher {

    fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
        let maybe_response = serde_json::from_str::<Vec<BitFinexOHLC>>(body);
        if let Err(e) = maybe_response {
            panic!("Error parsing response: {:?}", e);
        }
        let response = maybe_response.ok()?;
        if response.len() < 10 {
            return None
        }

        Some(response.into_iter().map(|r| OpenCloseVolume::from_f64(r.open, r.close, r.volume)).collect())
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
    let data = Vec::<BTreeMap<String, String>>::deserialize(deserializer)?;
    let mut result = Vec::<OpenCloseVolume>::with_capacity(data.len());
    for row in data.into_iter() {
        if !row.contains_key("open") && !row.contains_key("close") && !row.contains_key("volume") {
            return Err(serde::de::Error::custom("Row does not contain required data"));
        }
        let open = row.get("open").ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let close = row.get("close").ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let volume = row.get("volume").ok_or(serde::de::Error::custom("Could not parse value to str"))?;
        let ocv = OpenCloseVolume::from_str(open, close, volume).map_err(|_| serde::de::Error::custom("Error parsing float"))?;
        result.push(ocv);
    }
    Ok(result)
}

#[derive(Deserialize, Default)]
struct BitStampResult {
    #[serde(deserialize_with = "deserialize_hloc_bitstamp")]
    ohlc: Vec<OpenCloseVolume>,
    #[serde(skip)]
    pair: String, 
}

#[derive(Deserialize, Default)]
struct BitStampResponse {
    #[serde(default)]
    data: BitStampResult,
}

pub (crate) struct BitStampFetcher;
impl FetchPrice for BitStampFetcher {

    fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
        let maybe_response = serde_json::from_str::<BitStampResponse>(body);
        if let Err(e) = maybe_response {
            panic!("Error parsing response: {:?}", e);
        }
        let response = maybe_response.ok()?;
        if response.data.ohlc.len() < 10 {
            return None
        }

        Some(response.data.ohlc.into_iter().rev().collect())
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


#[derive(Default, Deserialize)]
struct CoinbaseOHLC {
    _timestamp: u64,
    _low:  f64,
    _high: f64,
    pub open: f64,
    pub close: f64,
    pub volume: f64,
}
pub (crate) struct CoinbaseFetcher;
impl FetchPrice for CoinbaseFetcher {

    fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>> {
        let maybe_response = serde_json::from_str::<Vec<CoinbaseOHLC>>(body);
        if let Err(e) = maybe_response {
            panic!("Error parsing response: {:?}", e);
        }
        let response = maybe_response.ok()?;
        if response.len() < 10 {
            return None
        }

        Some(response.into_iter().map(|r| OpenCloseVolume::from_f64(r.open, r.close, r.volume)).collect())
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