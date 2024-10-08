
//! Autogenerated weights for `pallet_collective`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 39.0.0
//! DATE: 2024-08-30, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-23-147`, CPU: `AMD EPYC 9R14`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-paseo-local")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polimec-node
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
		// Minimum execution time: 5_620_000 picoseconds.
		Weight::from_parts(5_860_000, 0)
			.saturating_add(Weight::from_parts(0, 3936))
			// Standard Error: 22_794
			.saturating_add(Weight::from_parts(875_074, 0).saturating_mul(m.into()))
			// Standard Error: 22_794
			.saturating_add(Weight::from_parts(713_464, 0).saturating_mul(n.into()))
			// Standard Error: 36_672
			.saturating_add(Weight::from_parts(2_964_030, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 8_360_000 picoseconds.
		Weight::from_parts(8_295_856, 0)
			.saturating_add(Weight::from_parts(0, 1517))
			// Standard Error: 13
			.saturating_add(Weight::from_parts(1_586, 0).saturating_mul(b.into()))
			// Standard Error: 633
			.saturating_add(Weight::from_parts(66_946, 0).saturating_mul(m.into()))
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
		// Minimum execution time: 9_880_000 picoseconds.
		Weight::from_parts(9_678_790, 0)
			.saturating_add(Weight::from_parts(0, 3497))
			// Standard Error: 19
			.saturating_add(Weight::from_parts(1_773, 0).saturating_mul(b.into()))
			// Standard Error: 908
			.saturating_add(Weight::from_parts(103_728, 0).saturating_mul(m.into()))
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
		// Minimum execution time: 13_780_000 picoseconds.
		Weight::from_parts(13_288_015, 0)
			.saturating_add(Weight::from_parts(0, 3549))
			// Standard Error: 32
			.saturating_add(Weight::from_parts(1_617, 0).saturating_mul(b.into()))
			// Standard Error: 1_254
			.saturating_add(Weight::from_parts(51_020, 0).saturating_mul(m.into()))
			// Standard Error: 5_028
			.saturating_add(Weight::from_parts(414_126, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 11_300_000 picoseconds.
		Weight::from_parts(11_691_017, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			// Standard Error: 1_120
			.saturating_add(Weight::from_parts(66_446, 0).saturating_mul(m.into()))
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
		// Minimum execution time: 15_070_000 picoseconds.
		Weight::from_parts(15_447_314, 0)
			.saturating_add(Weight::from_parts(0, 3605))
			// Standard Error: 1_136
			.saturating_add(Weight::from_parts(59_370, 0).saturating_mul(m.into()))
			// Standard Error: 3_609
			.saturating_add(Weight::from_parts(241_061, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 21_920_000 picoseconds.
		Weight::from_parts(22_104_910, 0)
			.saturating_add(Weight::from_parts(0, 3671))
			// Standard Error: 32
			.saturating_add(Weight::from_parts(322, 0).saturating_mul(b.into()))
			// Standard Error: 1_300
			.saturating_add(Weight::from_parts(80_414, 0).saturating_mul(m.into()))
			// Standard Error: 5_068
			.saturating_add(Weight::from_parts(387_014, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 16_221_000 picoseconds.
		Weight::from_parts(16_841_593, 0)
			.saturating_add(Weight::from_parts(0, 3699))
			// Standard Error: 1_099
			.saturating_add(Weight::from_parts(56_126, 0).saturating_mul(m.into()))
			// Standard Error: 3_491
			.saturating_add(Weight::from_parts(261_482, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 23_300_000 picoseconds.
		Weight::from_parts(23_700_355, 0)
			.saturating_add(Weight::from_parts(0, 3691))
			// Standard Error: 31
			.saturating_add(Weight::from_parts(396, 0).saturating_mul(b.into()))
			// Standard Error: 1_254
			.saturating_add(Weight::from_parts(72_419, 0).saturating_mul(m.into()))
			// Standard Error: 4_886
			.saturating_add(Weight::from_parts(381_866, 0).saturating_mul(p.into()))
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
		// Minimum execution time: 8_920_000 picoseconds.
		Weight::from_parts(9_182_590, 0)
			.saturating_add(Weight::from_parts(0, 1672))
			// Standard Error: 1_649
			.saturating_add(Weight::from_parts(201_678, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(p.into()))
	}
}
