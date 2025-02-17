
//! Autogenerated weights for `pallet_linear_release`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 39.0.0
//! DATE: 2025-02-17, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-23-147`, CPU: `AMD EPYC 9R14`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-paseo-local")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polimec-node
// benchmark
// pallet
// --chain=polimec-paseo-local
// --wasm-execution=compiled
// --pallet=pallet_linear_release
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/pallet_linear_release.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_linear_release`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_linear_release::WeightInfo for WeightInfo<T> {
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_locked(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `153 + s * (36 ±0)`
		//  Estimated: `3640 + s * (36 ±0)`
		// Minimum execution time: 26_041_000 picoseconds.
		Weight::from_parts(27_129_723, 0)
			.saturating_add(Weight::from_parts(0, 3640))
			// Standard Error: 4_658
			.saturating_add(Weight::from_parts(30_491, 0).saturating_mul(l.into()))
			// Standard Error: 454
			.saturating_add(Weight::from_parts(62_663, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_unlocked(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `153 + s * (36 ±0)`
		//  Estimated: `3640 + s * (36 ±0)`
		// Minimum execution time: 34_491_000 picoseconds.
		Weight::from_parts(35_902_942, 0)
			.saturating_add(Weight::from_parts(0, 3640))
			// Standard Error: 5_393
			.saturating_add(Weight::from_parts(17_920, 0).saturating_mul(l.into()))
			// Standard Error: 526
			.saturating_add(Weight::from_parts(55_339, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_other_locked(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `256 + s * (36 ±0)`
		//  Estimated: `3722 + s * (36 ±0)`
		// Minimum execution time: 28_201_000 picoseconds.
		Weight::from_parts(29_476_904, 0)
			.saturating_add(Weight::from_parts(0, 3722))
			// Standard Error: 5_503
			.saturating_add(Weight::from_parts(39_365, 0).saturating_mul(l.into()))
			// Standard Error: 537
			.saturating_add(Weight::from_parts(67_137, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_other_unlocked(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `256 + s * (36 ±0)`
		//  Estimated: `3722 + s * (36 ±0)`
		// Minimum execution time: 37_350_000 picoseconds.
		Weight::from_parts(38_427_634, 0)
			.saturating_add(Weight::from_parts(0, 3722))
			// Standard Error: 5_514
			.saturating_add(Weight::from_parts(11_514, 0).saturating_mul(l.into()))
			// Standard Error: 538
			.saturating_add(Weight::from_parts(57_667, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vested_transfer(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `293 + s * (36 ±0)`
		//  Estimated: `3759 + s * (36 ±0)`
		// Minimum execution time: 57_941_000 picoseconds.
		Weight::from_parts(58_524_974, 0)
			.saturating_add(Weight::from_parts(0, 3759))
			// Standard Error: 10_889
			.saturating_add(Weight::from_parts(111_738, 0).saturating_mul(l.into()))
			// Standard Error: 1_063
			.saturating_add(Weight::from_parts(73_345, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn force_vested_transfer(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `433 + s * (36 ±0)`
		//  Estimated: `6196 + s * (36 ±0)`
		// Minimum execution time: 60_571_000 picoseconds.
		Weight::from_parts(61_818_661, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			// Standard Error: 6_938
			.saturating_add(Weight::from_parts(79_467, 0).saturating_mul(l.into()))
			// Standard Error: 677
			.saturating_add(Weight::from_parts(75_795, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[2, 100]`.
	fn not_unlocking_merge_schedules(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `257 + s * (36 ±0)`
		//  Estimated: `3721 + s * (36 ±0)`
		// Minimum execution time: 29_451_000 picoseconds.
		Weight::from_parts(30_377_943, 0)
			.saturating_add(Weight::from_parts(0, 3721))
			// Standard Error: 5_184
			.saturating_add(Weight::from_parts(21_643, 0).saturating_mul(l.into()))
			// Standard Error: 506
			.saturating_add(Weight::from_parts(67_454, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:1 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[2, 100]`.
	fn unlocking_merge_schedules(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `257 + s * (36 ±0)`
		//  Estimated: `3721 + s * (36 ±0)`
		// Minimum execution time: 39_050_000 picoseconds.
		Weight::from_parts(40_329_821, 0)
			.saturating_add(Weight::from_parts(0, 3721))
			// Standard Error: 5_587
			.saturating_add(Weight::from_parts(12_211, 0).saturating_mul(l.into()))
			// Standard Error: 545
			.saturating_add(Weight::from_parts(66_061, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:2 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_all(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `228 + s * (36 ±0)`
		//  Estimated: `6169 + s * (36 ±0)`
		// Minimum execution time: 31_350_000 picoseconds.
		Weight::from_parts(32_826_741, 0)
			.saturating_add(Weight::from_parts(0, 6169))
			// Standard Error: 6_755
			.saturating_add(Weight::from_parts(16_809, 0).saturating_mul(l.into()))
			// Standard Error: 659
			.saturating_add(Weight::from_parts(83_942, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
	/// Storage: `LinearRelease::Vesting` (r:2 w:1)
	/// Proof: `LinearRelease::Vesting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[0, 9]`.
	/// The range of component `s` is `[1, 99]`.
	fn vest_all_other(l: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `331 + s * (36 ±0)`
		//  Estimated: `6272 + s * (36 ±0)`
		// Minimum execution time: 34_231_000 picoseconds.
		Weight::from_parts(35_075_431, 0)
			.saturating_add(Weight::from_parts(0, 6272))
			// Standard Error: 8_344
			.saturating_add(Weight::from_parts(55_959, 0).saturating_mul(l.into()))
			// Standard Error: 814
			.saturating_add(Weight::from_parts(87_093, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(s.into()))
	}
}
