
//! Autogenerated weights for `pallet_collective`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-04-10, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-23-147`, CPU: `AMD EPYC 9R14`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-local")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polimec-node
// benchmark
// pallet
// --chain=polimec-local
// --steps=50
// --repeat=20
// --pallet=pallet_collective
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=runtimes/polimec/src/weights/pallet_collective.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_collective`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collective::WeightInfo for WeightInfo<T> {
	/// Storage: `TechnicalCommittee::Members` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Voting` (r:7 w:7)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Prime` (r:0 w:1)
	/// Proof: `TechnicalCommittee::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[0, 20]`.
	/// The range of component `n` is `[0, 20]`.
	/// The range of component `p` is `[0, 7]`.
	/// The range of component `m` is `[0, 5]`.
	/// The range of component `n` is `[0, 5]`.
	/// The range of component `p` is `[0, 7]`.
	fn set_members(m: u32, n: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + m * (257 ±0) + p * (631 ±0)`
		//  Estimated: `3936 + m * (200 ±2) + p * (2248 ±3)`
		// Minimum execution time: 5_980_000 picoseconds.
		Weight::from_parts(6_160_000, 0)
			.saturating_add(Weight::from_parts(0, 3936))
			// Standard Error: 16_215
			.saturating_add(Weight::from_parts(799_756, 0).saturating_mul(m.into()))
			// Standard Error: 16_215
			.saturating_add(Weight::from_parts(284_876, 0).saturating_mul(n.into()))
			// Standard Error: 26_088
			.saturating_add(Weight::from_parts(3_073_457, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(p.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
			.saturating_add(Weight::from_parts(0, 200).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 2248).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 20]`.
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 5]`.
	fn execute(b: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `32 + m * (32 ±0)`
		//  Estimated: `1517 + m * (32 ±0)`
		// Minimum execution time: 8_910_000 picoseconds.
		Weight::from_parts(9_240_806, 0)
			.saturating_add(Weight::from_parts(0, 1517))
			// Standard Error: 8
			.saturating_add(Weight::from_parts(1_405, 0).saturating_mul(b.into()))
			// Standard Error: 373
			.saturating_add(Weight::from_parts(20_528, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:0)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 20]`.
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[1, 5]`.
	fn propose_execute(b: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `32 + m * (32 ±0)`
		//  Estimated: `3497 + m * (32 ±0)`
		// Minimum execution time: 10_810_000 picoseconds.
		Weight::from_parts(10_815_523, 0)
			.saturating_add(Weight::from_parts(0, 3497))
			// Standard Error: 11
			.saturating_add(Weight::from_parts(1_575, 0).saturating_mul(b.into()))
			// Standard Error: 527
			.saturating_add(Weight::from_parts(48_832, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalCount` (r:1 w:1)
	/// Proof: `TechnicalCommittee::ProposalCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Voting` (r:0 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[2, 20]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[2, 5]`.
	/// The range of component `p` is `[1, 7]`.
	fn propose_proposed(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `97 + m * (32 ±0) + p * (56 ±0)`
		//  Estimated: `3549 + m * (33 ±0) + p * (61 ±0)`
		// Minimum execution time: 14_590_000 picoseconds.
		Weight::from_parts(14_023_632, 0)
			.saturating_add(Weight::from_parts(0, 3549))
			// Standard Error: 33
			.saturating_add(Weight::from_parts(1_820, 0).saturating_mul(b.into()))
			// Standard Error: 1_298
			.saturating_add(Weight::from_parts(38_582, 0).saturating_mul(m.into()))
			// Standard Error: 5_204
			.saturating_add(Weight::from_parts(448_594, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 33).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 61).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Voting` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[5, 20]`.
	/// The range of component `m` is `[5, 5]`.
	fn vote(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `266 + m * (64 ±0)`
		//  Estimated: `3731 + m * (64 ±0)`
		// Minimum execution time: 12_170_000 picoseconds.
		Weight::from_parts(12_559_668, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			// Standard Error: 1_191
			.saturating_add(Weight::from_parts(65_573, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 64).saturating_mul(m.into()))
	}
	/// Storage: `TechnicalCommittee::Voting` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[4, 20]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `m` is `[4, 5]`.
	/// The range of component `p` is `[1, 7]`.
	fn close_early_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `118 + m * (64 ±0) + p * (56 ±0)`
		//  Estimated: `3605 + m * (64 ±0) + p * (54 ±0)`
		// Minimum execution time: 16_010_000 picoseconds.
		Weight::from_parts(16_573_137, 0)
			.saturating_add(Weight::from_parts(0, 3605))
			// Standard Error: 1_105
			.saturating_add(Weight::from_parts(53_023, 0).saturating_mul(m.into()))
			// Standard Error: 3_510
			.saturating_add(Weight::from_parts(259_873, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 64).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Voting` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 20]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 5]`.
	/// The range of component `p` is `[1, 7]`.
	fn close_early_approved(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `152 + b * (1 ±0) + m * (64 ±0) + p * (84 ±0)`
		//  Estimated: `3671 + b * (1 ±0) + m * (65 ±0) + p * (81 ±0)`
		// Minimum execution time: 23_330_000 picoseconds.
		Weight::from_parts(23_377_861, 0)
			.saturating_add(Weight::from_parts(0, 3671))
			// Standard Error: 32
			.saturating_add(Weight::from_parts(603, 0).saturating_mul(b.into()))
			// Standard Error: 1_270
			.saturating_add(Weight::from_parts(68_142, 0).saturating_mul(m.into()))
			// Standard Error: 4_950
			.saturating_add(Weight::from_parts(424_662, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 65).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 81).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Voting` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Prime` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[4, 20]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `m` is `[4, 5]`.
	/// The range of component `p` is `[1, 7]`.
	fn close_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `193 + m * (50 ±0) + p * (56 ±0)`
		//  Estimated: `3699 + m * (50 ±0) + p * (52 ±0)`
		// Minimum execution time: 17_450_000 picoseconds.
		Weight::from_parts(18_016_519, 0)
			.saturating_add(Weight::from_parts(0, 3699))
			// Standard Error: 1_107
			.saturating_add(Weight::from_parts(54_069, 0).saturating_mul(m.into()))
			// Standard Error: 3_517
			.saturating_add(Weight::from_parts(252_754, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 50).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 52).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Voting` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Members` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Prime` (r:1 w:0)
	/// Proof: `TechnicalCommittee::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 20]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `b` is `[2, 1024]`.
	/// The range of component `m` is `[4, 5]`.
	/// The range of component `p` is `[1, 7]`.
	fn close_approved(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `172 + b * (1 ±0) + m * (64 ±0) + p * (84 ±0)`
		//  Estimated: `3691 + b * (1 ±0) + m * (65 ±0) + p * (81 ±0)`
		// Minimum execution time: 24_800_000 picoseconds.
		Weight::from_parts(25_037_977, 0)
			.saturating_add(Weight::from_parts(0, 3691))
			// Standard Error: 34
			.saturating_add(Weight::from_parts(529, 0).saturating_mul(b.into()))
			// Standard Error: 1_374
			.saturating_add(Weight::from_parts(65_348, 0).saturating_mul(m.into()))
			// Standard Error: 5_356
			.saturating_add(Weight::from_parts(409_844, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 65).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 81).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Voting` (r:0 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::ProposalOf` (r:0 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `p` is `[1, 7]`.
	fn disapprove_proposal(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `188 + p * (32 ±0)`
		//  Estimated: `1672 + p * (32 ±0)`
		// Minimum execution time: 9_570_000 picoseconds.
		Weight::from_parts(9_844_721, 0)
			.saturating_add(Weight::from_parts(0, 1672))
			// Standard Error: 1_770
			.saturating_add(Weight::from_parts(204_509, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(p.into()))
	}
}
