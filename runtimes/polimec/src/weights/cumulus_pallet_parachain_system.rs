
//! Autogenerated weights for `cumulus_pallet_parachain_system`
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
// --pallet=cumulus_pallet_parachain_system
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/cumulus_pallet_parachain_system.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `cumulus_pallet_parachain_system`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> cumulus_pallet_parachain_system::WeightInfo for WeightInfo<T> {
	/// Storage: `ParachainSystem::LastDmqMqcHead` (r:1 w:1)
	/// Proof: `ParachainSystem::LastDmqMqcHead` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(5), added: 500, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::ProcessedDownwardMessages` (r:0 w:1)
	/// Proof: `ParachainSystem::ProcessedDownwardMessages` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1000)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65585), added: 68060, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 1000]`.
	fn enqueue_inbound_downward_messages(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `12`
		//  Estimated: `3517`
		// Minimum execution time: 2_230_000 picoseconds.
		Weight::from_parts(2_340_000, 0)
			.saturating_add(Weight::from_parts(0, 3517))
			// Standard Error: 21_241
			.saturating_add(Weight::from_parts(181_643_985, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
	}
}
