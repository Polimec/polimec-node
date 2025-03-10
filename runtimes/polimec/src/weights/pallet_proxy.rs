
//! Autogenerated weights for `pallet_proxy`
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
// --pallet=pallet_proxy
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/pallet_proxy.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_proxy`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_proxy::WeightInfo for WeightInfo<T> {
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `90 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 8_470_000 picoseconds.
		Weight::from_parts(9_038_214, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 881
			.saturating_add(Weight::from_parts(33_818, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn proxy_announced(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `380 + a * (68 ±0) + p * (37 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 25_120_000 picoseconds.
		Weight::from_parts(25_816_214, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 1_693
			.saturating_add(Weight::from_parts(155_324, 0).saturating_mul(a.into()))
			// Standard Error: 1_749
			.saturating_add(Weight::from_parts(24_455, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn remove_announcement(a: u32, _p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `295 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 17_380_000 picoseconds.
		Weight::from_parts(18_195_007, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 1_305
			.saturating_add(Weight::from_parts(143_599, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn reject_announcement(a: u32, _p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `295 + a * (68 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 17_070_000 picoseconds.
		Weight::from_parts(18_324_570, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 1_536
			.saturating_add(Weight::from_parts(146_521, 0).saturating_mul(a.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:0)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// Storage: `Proxy::Announcements` (r:1 w:1)
	/// Proof: `Proxy::Announcements` (`max_values`: None, `max_size`: Some(2233), added: 4708, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `a` is `[0, 31]`.
	/// The range of component `p` is `[1, 31]`.
	fn announce(a: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `312 + a * (68 ±0) + p * (37 ±0)`
		//  Estimated: `5698`
		// Minimum execution time: 23_040_000 picoseconds.
		Weight::from_parts(23_676_509, 0)
			.saturating_add(Weight::from_parts(0, 5698))
			// Standard Error: 1_669
			.saturating_add(Weight::from_parts(145_606, 0).saturating_mul(a.into()))
			// Standard Error: 1_725
			.saturating_add(Weight::from_parts(23_538, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn add_proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `90 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 16_411_000 picoseconds.
		Weight::from_parts(17_083_997, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 897
			.saturating_add(Weight::from_parts(35_794, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxy(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `90 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 16_420_000 picoseconds.
		Weight::from_parts(17_101_275, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 974
			.saturating_add(Weight::from_parts(41_707, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn remove_proxies(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `90 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 14_890_000 picoseconds.
		Weight::from_parts(15_585_058, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 910
			.saturating_add(Weight::from_parts(27_113, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 31]`.
	fn create_pure(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `102`
		//  Estimated: `4706`
		// Minimum execution time: 17_181_000 picoseconds.
		Weight::from_parts(18_051_214, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 967
			.saturating_add(Weight::from_parts(9_800, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Proxy::Proxies` (r:1 w:1)
	/// Proof: `Proxy::Proxies` (`max_values`: None, `max_size`: Some(1241), added: 3716, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[0, 30]`.
	fn kill_pure(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127 + p * (37 ±0)`
		//  Estimated: `4706`
		// Minimum execution time: 15_350_000 picoseconds.
		Weight::from_parts(16_085_216, 0)
			.saturating_add(Weight::from_parts(0, 4706))
			// Standard Error: 923
			.saturating_add(Weight::from_parts(28_579, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
