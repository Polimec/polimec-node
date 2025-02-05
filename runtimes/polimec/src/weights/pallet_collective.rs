
//! Autogenerated weights for `pallet_collective`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 46.0.0
//! DATE: 2025-02-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `MacBook-Pro-2.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-paseo-local")`, DB CACHE: 1024

// Executed Command:
// ./polimec-node
// benchmark
// pallet
// --chain=polimec-paseo-local
// --wasm-execution=compiled
// --pallet=pallet_collective
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/pallet_collective.rs

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
		// Minimum execution time: 7_000_000 picoseconds.
		Weight::from_parts(1_285_793, 0)
			.saturating_add(Weight::from_parts(0, 3936))
			// Standard Error: 19_805
			.saturating_add(Weight::from_parts(931_284, 0).saturating_mul(m.into()))
			// Standard Error: 19_805
			.saturating_add(Weight::from_parts(324_594, 0).saturating_mul(n.into()))
			// Standard Error: 58_140
			.saturating_add(Weight::from_parts(4_856_452, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_757_480, 0)
			.saturating_add(Weight::from_parts(0, 1517))
			// Standard Error: 152
			.saturating_add(Weight::from_parts(1_247, 0).saturating_mul(b.into()))
			// Standard Error: 6_972
			.saturating_add(Weight::from_parts(65_755, 0).saturating_mul(m.into()))
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
	fn propose_execute(_b: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `32 + m * (32 ±0)`
		//  Estimated: `3497 + m * (32 ±0)`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_943_533, 0)
			.saturating_add(Weight::from_parts(0, 3497))
			// Standard Error: 14_659
			.saturating_add(Weight::from_parts(70_320, 0).saturating_mul(m.into()))
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
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(18_763_926, 0)
			.saturating_add(Weight::from_parts(0, 3549))
			// Standard Error: 82
			.saturating_add(Weight::from_parts(851, 0).saturating_mul(b.into()))
			// Standard Error: 3_175
			.saturating_add(Weight::from_parts(47_169, 0).saturating_mul(m.into()))
			// Standard Error: 12_729
			.saturating_add(Weight::from_parts(524_890, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 15_000_000 picoseconds.
		Weight::from_parts(16_654_057, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			// Standard Error: 4_302
			.saturating_add(Weight::from_parts(43_471, 0).saturating_mul(m.into()))
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
		// Minimum execution time: 20_000_000 picoseconds.
		Weight::from_parts(23_665_121, 0)
			.saturating_add(Weight::from_parts(0, 3605))
			// Standard Error: 13_226
			.saturating_add(Weight::from_parts(458_112, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 31_000_000 picoseconds.
		Weight::from_parts(32_886_622, 0)
			.saturating_add(Weight::from_parts(0, 3671))
			// Standard Error: 3_989
			.saturating_add(Weight::from_parts(70_793, 0).saturating_mul(m.into()))
			// Standard Error: 15_545
			.saturating_add(Weight::from_parts(628_281, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 25_000_000 picoseconds.
		Weight::from_parts(26_300_938, 0)
			.saturating_add(Weight::from_parts(0, 3699))
			// Standard Error: 3_641
			.saturating_add(Weight::from_parts(47_992, 0).saturating_mul(m.into()))
			// Standard Error: 11_563
			.saturating_add(Weight::from_parts(337_673, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 33_000_000 picoseconds.
		Weight::from_parts(36_060_788, 0)
			.saturating_add(Weight::from_parts(0, 3691))
			// Standard Error: 5_140
			.saturating_add(Weight::from_parts(141_321, 0).saturating_mul(m.into()))
			// Standard Error: 20_029
			.saturating_add(Weight::from_parts(516_733, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 12_000_000 picoseconds.
		Weight::from_parts(12_737_329, 0)
			.saturating_add(Weight::from_parts(0, 1672))
			// Standard Error: 8_628
			.saturating_add(Weight::from_parts(197_917, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:1)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::CostOf` (r:1 w:0)
	/// Proof: `TechnicalCommittee::CostOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Proposals` (r:1 w:1)
	/// Proof: `TechnicalCommittee::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::Voting` (r:0 w:1)
	/// Proof: `TechnicalCommittee::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `d` is `[0, 1]`.
	/// The range of component `p` is `[1, 7]`.
	/// The range of component `d` is `[0, 1]`.
	/// The range of component `p` is `[1, 7]`.
	fn kill(d: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1210 + p * (58 ±0)`
		//  Estimated: `4677 + d * (18 ±0) + p * (58 ±0)`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(16_326_329, 0)
			.saturating_add(Weight::from_parts(0, 4677))
			// Standard Error: 63_770
			.saturating_add(Weight::from_parts(1_528_763, 0).saturating_mul(d.into()))
			// Standard Error: 14_843
			.saturating_add(Weight::from_parts(674_810, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 18).saturating_mul(d.into()))
			.saturating_add(Weight::from_parts(0, 58).saturating_mul(p.into()))
	}
	/// Storage: `TechnicalCommittee::ProposalOf` (r:1 w:0)
	/// Proof: `TechnicalCommittee::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TechnicalCommittee::CostOf` (r:1 w:0)
	/// Proof: `TechnicalCommittee::CostOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn release_proposal_cost() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `337`
		//  Estimated: `3802`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(10_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3802))
			.saturating_add(T::DbWeight::get().reads(2))
	}
}
