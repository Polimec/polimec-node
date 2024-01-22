
//! Autogenerated weights for `pallet_funding`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2024-01-22, STEPS: `10`, REPEAT: `5`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Juans-MBP.home`, CPU: `<UNKNOWN>`
//! EXECUTION: ``, WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-rococo-local")`, DB CACHE: 1024

// Executed Command:
// target/debug/polimec-parachain-node
// benchmark
// pallet
// --chain=polimec-rococo-local
// --steps=10
// --repeat=5
// --pallet=pallet_funding
// --extrinsic=start_evaluation,start_auction_manually
// --output
// ./debug_weight_gen.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_funding`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_funding::WeightInfo for WeightInfo<T> {
	/// Storage: `PolimecFunding::ProjectsMetadata` (r:1 w:0)
	/// Proof: `PolimecFunding::ProjectsMetadata` (`max_values`: None, `max_size`: Some(334), added: 2809, mode: `MaxEncodedLen`)
	/// Storage: `PolimecFunding::ProjectsDetails` (r:1 w:1)
	/// Proof: `PolimecFunding::ProjectsDetails` (`max_values`: None, `max_size`: Some(349), added: 2824, mode: `MaxEncodedLen`)
	/// Storage: `PolimecFunding::ProjectsToUpdate` (r:100 w:1)
	/// Proof: `PolimecFunding::ProjectsToUpdate` (`max_values`: None, `max_size`: Some(622), added: 3097, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[1, 99]`.
	fn start_evaluation(x: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `560 + x * (529 ±0)`
		//  Estimated: `4087 + x * (3097 ±0)`
		// Minimum execution time: 313_000_000 picoseconds.
		Weight::from_parts(277_026_432, 0)
			.saturating_add(Weight::from_parts(0, 4087))
			// Standard Error: 177_230
			.saturating_add(Weight::from_parts(36_411_429, 0).saturating_mul(x.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 3097).saturating_mul(x.into()))
	}
	/// Storage: `PolimecFunding::ProjectsDetails` (r:1 w:1)
	/// Proof: `PolimecFunding::ProjectsDetails` (`max_values`: None, `max_size`: Some(349), added: 2824, mode: `MaxEncodedLen`)
	/// Storage: `PolimecFunding::ProjectsToUpdate` (r:4554 w:2)
	/// Proof: `PolimecFunding::ProjectsToUpdate` (`max_values`: None, `max_size`: Some(622), added: 3097, mode: `MaxEncodedLen`)
	/// The range of component `x` is `[1, 99]`.
	/// The range of component `y` is `[1, 10000]`.
	/// The range of component `z` is `[1, 100]`.
	fn start_auction_manually(x: u32, y: u32, _z: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + x * (8678 ±0) + y * (249 ±0) + z * (22472 ±0)`
		//  Estimated: `316884 + x * (20887 ±12_728) + y * (1199 ±125)`
		// Minimum execution time: 5_710_000_000 picoseconds.
		Weight::from_parts(5_718_000_000, 0)
			.saturating_add(Weight::from_parts(0, 316884))
			// Standard Error: 281_211_660
			.saturating_add(Weight::from_parts(385_812_210, 0).saturating_mul(x.into()))
			// Standard Error: 2_780_420
			.saturating_add(Weight::from_parts(25_829_204, 0).saturating_mul(y.into()))
			.saturating_add(T::DbWeight::get().reads(103))
			.saturating_add(T::DbWeight::get().reads((7_u64).saturating_mul(x.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 20887).saturating_mul(x.into()))
			.saturating_add(Weight::from_parts(0, 1199).saturating_mul(y.into()))
	}
}
