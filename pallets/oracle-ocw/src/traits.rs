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

use crate::types::{AssetResponse, OpenCloseVolume};

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
use super::*;

use sp_runtime::{
    FixedU128,
    Saturating,
    offchain::{
        http::{self, Request, PendingRequest, Response},
        Duration,
    }
};

pub(crate) trait FetchPrice {

    fn get_moving_average(assets: Vec<AssetName>, timeout: u64) -> Vec<(AssetName, FixedU128)> {

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(timeout));
        let asset_requests = assets.into_iter().filter_map(|asset| {
            let url = Self::get_url(asset);
            let request = http::Request::get(url);
            
            if let Ok(req) = request.deadline(deadline).send() {
                return Some(AssetRequest {asset, id: req.id});
            }
            None
        }).collect::<Vec<AssetRequest>>();
        
        let request: Vec<PendingRequest> = asset_requests.iter().map(|r| PendingRequest{id: r.id}).collect();
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(timeout));
        let maybe_responses = PendingRequest::try_wait_all(request, deadline);
        maybe_responses.into_iter().zip(asset_requests.into_iter().map(|r| r.asset)).filter_map(|(maybe_response, asset)| {
            if let Ok(Ok(response)) = maybe_response {
                if response.code != 200 {
                    return None
                }
                return Some((asset, response))
            }
            None
        }).filter_map(|(asset, response)| {
            let body = response.body().collect::<Vec<u8>>();
            if let Ok(body_str) = sp_std::str::from_utf8(&body) {
                if let Some(ocv_data) = Self::parse_body(body_str) {
                    return Some((asset, ocv_data))
                }
            }
            None
        })
        .filter_map(|(asset, ocv_data)| {
            let (w_price_sum, total_vol) = ocv_data.into_iter().fold((FixedU128::zero(), FixedU128::zero()), |(w_price_sum, vol_sum), ocv| { 
                (w_price_sum + ocv.vwp(), vol_sum.saturating_add(ocv.volume))
            });
            if total_vol.is_zero() {
                return None
            }
            Some((asset, w_price_sum.div(total_vol)))
        }).collect::<Vec<(AssetName, FixedU128)>>()
    }

    fn parse_body(body: &str) -> Option<Vec<OpenCloseVolume>>;

    fn get_url(name: AssetName) -> &'static str;
}
