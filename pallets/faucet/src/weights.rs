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


//! Autogenerated weights for `pallet_faucet`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-04-09, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Justs-MacBook-Pro.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("politest-local")`, DB CACHE: `1024`

// Executed Command:
// target/release/polimec-node
// benchmark
// pallet
// --chain=politest-local
// --steps=50
// --repeat=20
// --pallet=pallet-faucet
// --no-storage-info
// --no-median-slopes
// --no-min-squares
// --extrinsic
// *
// --wasm-execution=compiled
// --heap-pages=4096
// --output=pallets/faucet/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_faucet`.
pub trait WeightInfo {
	fn claim() -> Weight;
	fn set_claiming_amount() -> Weight;
}

/// Weights for `pallet_faucet` using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Claims::Claims` (r:1 w:1)
	/// Proof: `Claims::Claims` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Claims::ClaimAmount` (r:1 w:0)
	/// Proof: `Claims::ClaimAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Vesting::Vesting` (r:1 w:1)
	/// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(481), added: 2956, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:1)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:0)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	fn claim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `288`
		//  Estimated: `4764`
		// Minimum execution time: 191_000_000 picoseconds.
		Weight::from_parts(200_000_000, 4764)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: `Claims::ClaimAmount` (r:0 w:1)
	/// Proof: `Claims::ClaimAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn set_claiming_amount() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_000_000 picoseconds.
		Weight::from_parts(5_000_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Claims::Claims` (r:1 w:1)
	/// Proof: `Claims::Claims` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Claims::ClaimAmount` (r:1 w:0)
	/// Proof: `Claims::ClaimAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Vesting::Vesting` (r:1 w:1)
	/// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(481), added: 2956, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:1)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:0)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	fn claim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `288`
		//  Estimated: `4764`
		// Minimum execution time: 191_000_000 picoseconds.
		Weight::from_parts(200_000_000, 4764)
			.saturating_add(RocksDbWeight::get().reads(7_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	/// Storage: `Claims::ClaimAmount` (r:0 w:1)
	/// Proof: `Claims::ClaimAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn set_claiming_amount() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_000_000 picoseconds.
		Weight::from_parts(5_000_000, 0)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}