
//! Autogenerated weights for `frame_system`
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
// --pallet=frame_system
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/frame_system.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `frame_system`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> frame_system::WeightInfo for WeightInfo<T> {
	/// The range of component `b` is `[0, 3932160]`.
	fn remark(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 1_300_000 picoseconds.
		Weight::from_parts(2_087_279, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 0
			.saturating_add(Weight::from_parts(271, 0).saturating_mul(b.into()))
	}
	/// The range of component `b` is `[0, 3932160]`.
	fn remark_with_event(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 3_340_000 picoseconds.
		Weight::from_parts(223_332_150, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 8
			.saturating_add(Weight::from_parts(1_342, 0).saturating_mul(b.into()))
	}
	/// Storage: `System::Digest` (r:1 w:1)
	/// Proof: `System::Digest` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: UNKNOWN KEY `0x3a686561707061676573` (r:0 w:1)
	/// Proof: UNKNOWN KEY `0x3a686561707061676573` (r:0 w:1)
	fn set_heap_pages() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `1485`
		// Minimum execution time: 2_120_000 picoseconds.
		Weight::from_parts(2_340_000, 0)
			.saturating_add(Weight::from_parts(0, 1485))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `ParachainSystem::ValidationData` (r:1 w:0)
	/// Proof: `ParachainSystem::ValidationData` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::UpgradeRestrictionSignal` (r:1 w:0)
	/// Proof: `ParachainSystem::UpgradeRestrictionSignal` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::PendingValidationCode` (r:1 w:1)
	/// Proof: `ParachainSystem::PendingValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `ParachainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::NewValidationCode` (r:0 w:1)
	/// Proof: `ParachainSystem::NewValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::DidSetValidationCode` (r:0 w:1)
	/// Proof: `ParachainSystem::DidSetValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_code() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `127`
		//  Estimated: `1612`
		// Minimum execution time: 67_659_989_000 picoseconds.
		Weight::from_parts(69_358_616_000, 0)
			.saturating_add(Weight::from_parts(0, 1612))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Skipped::Metadata` (r:0 w:0)
	/// Proof: `Skipped::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `i` is `[0, 1000]`.
	fn set_storage(i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 1_300_000 picoseconds.
		Weight::from_parts(1_370_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 1_449
			.saturating_add(Weight::from_parts(640_025, 0).saturating_mul(i.into()))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(i.into())))
	}
	/// Storage: `Skipped::Metadata` (r:0 w:0)
	/// Proof: `Skipped::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `i` is `[0, 1000]`.
	fn kill_storage(i: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 1_280_000 picoseconds.
		Weight::from_parts(1_390_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 802
			.saturating_add(Weight::from_parts(488_042, 0).saturating_mul(i.into()))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(i.into())))
	}
	/// Storage: `Skipped::Metadata` (r:0 w:0)
	/// Proof: `Skipped::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `p` is `[0, 1000]`.
	fn kill_prefix(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `61 + p * (69 ±0)`
		//  Estimated: `74 + p * (70 ±0)`
		// Minimum execution time: 2_720_000 picoseconds.
		Weight::from_parts(2_840_000, 0)
			.saturating_add(Weight::from_parts(0, 74))
			// Standard Error: 1_296
			.saturating_add(Weight::from_parts(986_797, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 70).saturating_mul(p.into()))
	}
	/// Storage: `System::AuthorizedUpgrade` (r:0 w:1)
	/// Proof: `System::AuthorizedUpgrade` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	fn authorize_upgrade() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_600_000 picoseconds.
		Weight::from_parts(6_670_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `System::AuthorizedUpgrade` (r:1 w:1)
	/// Proof: `System::AuthorizedUpgrade` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::ValidationData` (r:1 w:0)
	/// Proof: `ParachainSystem::ValidationData` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::UpgradeRestrictionSignal` (r:1 w:0)
	/// Proof: `ParachainSystem::UpgradeRestrictionSignal` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::PendingValidationCode` (r:1 w:1)
	/// Proof: `ParachainSystem::PendingValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::HostConfiguration` (r:1 w:0)
	/// Proof: `ParachainSystem::HostConfiguration` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::NewValidationCode` (r:0 w:1)
	/// Proof: `ParachainSystem::NewValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::DidSetValidationCode` (r:0 w:1)
	/// Proof: `ParachainSystem::DidSetValidationCode` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn apply_authorized_upgrade() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `149`
		//  Estimated: `1634`
		// Minimum execution time: 73_564_255_000 picoseconds.
		Weight::from_parts(76_229_229_000, 0)
			.saturating_add(Weight::from_parts(0, 1634))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
}
