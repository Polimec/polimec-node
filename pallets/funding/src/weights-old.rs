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


//! Autogenerated weights for pallet_funding
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-03-20, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `pop-os`, CPU: `13th Gen Intel(R) Core(TM) i9-13900K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// target/release/polimec-standalone-node
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_funding
// --extrinsic
// *
// --execution=wasm
// --heap-pages=4096
// --output=pallets/funding/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_funding.
pub trait WeightInfo {
    fn create() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn edit_metadata() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn start_evaluation() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn start_auction() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn bond_evaluation() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn bid() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn contribute() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn evaluation_unbond_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn evaluation_slash_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn evaluation_reward_payout_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn bid_ct_mint_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn contribution_ct_mint_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn start_bid_vesting_schedule_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn start_contribution_vesting_schedule_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn payout_bid_funds_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn payout_contribution_funds_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn decide_project_outcome() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn release_bid_funds_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn bid_unbond_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn release_contribution_funds_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn contribution_unbond_for() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
    fn insert_cleaned_project() -> Weight {
        // Minimum execution time: 5_745 nanoseconds.
        Weight::from_parts(6_034_000, 0)
    }
}

/// Weights for pallet_funding using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {}
// For backwards compatibility and tests
impl WeightInfo for () {}