
//! Autogenerated weights for `pallet_elections_phragmen`
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
// --pallet=pallet_elections_phragmen
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/pallet_elections_phragmen.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_elections_phragmen`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_elections_phragmen::WeightInfo for WeightInfo<T> {
	/// Storage: `Elections::Candidates` (r:1 w:0)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:0)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:0)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Voting` (r:1 w:1)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[1, 8]`.
	fn vote_equal(v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `380 + v * (80 ±0)`
		//  Estimated: `4764 + v * (80 ±0)`
		// Minimum execution time: 24_160_000 picoseconds.
		Weight::from_parts(25_132_092, 0)
			.saturating_add(Weight::from_parts(0, 4764))
			// Standard Error: 5_022
			.saturating_add(Weight::from_parts(146_018, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 80).saturating_mul(v.into()))
	}
	/// Storage: `Elections::Candidates` (r:1 w:0)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:0)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:0)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Voting` (r:1 w:1)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[2, 8]`.
	fn vote_more(v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `348 + v * (80 ±0)`
		//  Estimated: `4764 + v * (80 ±0)`
		// Minimum execution time: 24_390_000 picoseconds.
		Weight::from_parts(25_141_214, 0)
			.saturating_add(Weight::from_parts(0, 4764))
			// Standard Error: 5_579
			.saturating_add(Weight::from_parts(133_567, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 80).saturating_mul(v.into()))
	}
	/// Storage: `Elections::Candidates` (r:1 w:0)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:0)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:0)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Voting` (r:1 w:1)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[2, 8]`.
	fn vote_less(v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `380 + v * (80 ±0)`
		//  Estimated: `4764 + v * (80 ±0)`
		// Minimum execution time: 24_441_000 picoseconds.
		Weight::from_parts(25_440_558, 0)
			.saturating_add(Weight::from_parts(0, 4764))
			// Standard Error: 5_911
			.saturating_add(Weight::from_parts(87_377, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 80).saturating_mul(v.into()))
	}
	/// Storage: `Elections::Voting` (r:1 w:1)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:1 w:1)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	fn remove_voter() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `646`
		//  Estimated: `4764`
		// Minimum execution time: 25_190_000 picoseconds.
		Weight::from_parts(25_910_000, 0)
			.saturating_add(Weight::from_parts(0, 4764))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Elections::Candidates` (r:1 w:1)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:0)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:0)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[1, 30]`.
	fn submit_candidacy(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2706 + c * (50 ±0)`
		//  Estimated: `4184 + c * (50 ±0)`
		// Minimum execution time: 44_970_000 picoseconds.
		Weight::from_parts(46_421_748, 0)
			.saturating_add(Weight::from_parts(0, 4184))
			// Standard Error: 2_402
			.saturating_add(Weight::from_parts(57_088, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 50).saturating_mul(c.into()))
	}
	/// Storage: `Elections::Candidates` (r:1 w:1)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[1, 30]`.
	fn renounce_candidacy_candidate(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `880 + c * (54 ±0)`
		//  Estimated: `3640 + c * (55 ±0)`
		// Minimum execution time: 36_071_000 picoseconds.
		Weight::from_parts(38_006_444, 0)
			.saturating_add(Weight::from_parts(0, 3640))
			// Standard Error: 2_738
			.saturating_add(Weight::from_parts(28_560, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 55).saturating_mul(c.into()))
	}
	/// Storage: `Elections::Members` (r:1 w:1)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `Elections::RunnersUp` (r:1 w:1)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Prime` (r:1 w:1)
	/// Proof: `Council::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Proposals` (r:1 w:0)
	/// Proof: `Council::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Members` (r:0 w:1)
	/// Proof: `Council::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn renounce_candidacy_members() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2901`
		//  Estimated: `4386`
		// Minimum execution time: 50_150_000 picoseconds.
		Weight::from_parts(51_031_000, 0)
			.saturating_add(Weight::from_parts(0, 4386))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Elections::RunnersUp` (r:1 w:1)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	fn renounce_candidacy_runners_up() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2274`
		//  Estimated: `3759`
		// Minimum execution time: 39_580_000 picoseconds.
		Weight::from_parts(40_730_000, 0)
			.saturating_add(Weight::from_parts(0, 3759))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Benchmark::Override` (r:0 w:0)
	/// Proof: `Benchmark::Override` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn remove_member_without_replacement() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 500_000_000_000 picoseconds.
		Weight::from_parts(500_000_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `Elections::Members` (r:1 w:1)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Elections::RunnersUp` (r:1 w:1)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Prime` (r:1 w:1)
	/// Proof: `Council::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Proposals` (r:1 w:0)
	/// Proof: `Council::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Members` (r:0 w:1)
	/// Proof: `Council::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn remove_member_with_replacement() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3003`
		//  Estimated: `6196`
		// Minimum execution time: 53_310_000 picoseconds.
		Weight::from_parts(54_301_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Elections::Voting` (r:101 w:100)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:0)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:0)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Candidates` (r:1 w:0)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Freezes` (r:100 w:100)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(949), added: 3424, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:100 w:100)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:100 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[100, 200]`.
	/// The range of component `d` is `[0, 100]`.
	fn clean_defunct_voters(v: u32, d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + d * (513 ±0) + v * (60 ±0)`
		//  Estimated: `12004 + d * (3774 ±0) + v * (29 ±0)`
		// Minimum execution time: 3_240_000 picoseconds.
		Weight::from_parts(3_530_000, 0)
			.saturating_add(Weight::from_parts(0, 12004))
			// Standard Error: 6_868
			.saturating_add(Weight::from_parts(309_191, 0).saturating_mul(v.into()))
			// Standard Error: 14_967
			.saturating_add(Weight::from_parts(27_821_615, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((4_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 3774).saturating_mul(d.into()))
			.saturating_add(Weight::from_parts(0, 29).saturating_mul(v.into()))
	}
	/// Storage: `Elections::Candidates` (r:1 w:1)
	/// Proof: `Elections::Candidates` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Members` (r:1 w:1)
	/// Proof: `Elections::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::RunnersUp` (r:1 w:1)
	/// Proof: `Elections::RunnersUp` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Elections::Voting` (r:201 w:0)
	/// Proof: `Elections::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Proposals` (r:1 w:0)
	/// Proof: `Council::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(175), added: 2650, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Elections::ElectionRounds` (r:1 w:1)
	/// Proof: `Elections::ElectionRounds` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Members` (r:0 w:1)
	/// Proof: `Council::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Council::Prime` (r:0 w:1)
	/// Proof: `Council::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `c` is `[1, 30]`.
	/// The range of component `v` is `[1, 200]`.
	/// The range of component `e` is `[200, 1600]`.
	fn election_phragmen(c: u32, v: u32, e: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + e * (27 ±0) + v * (326 ±0)`
		//  Estimated: `83566 + c * (206 ±0) + e * (11 ±0) + v * (2473 ±3)`
		// Minimum execution time: 240_076_000 picoseconds.
		Weight::from_parts(241_885_000, 0)
			.saturating_add(Weight::from_parts(0, 83566))
			// Standard Error: 963_121
			.saturating_add(Weight::from_parts(4_158_037, 0).saturating_mul(c.into()))
			// Standard Error: 143_847
			.saturating_add(Weight::from_parts(6_989_549, 0).saturating_mul(v.into()))
			// Standard Error: 18_946
			.saturating_add(Weight::from_parts(183_469, 0).saturating_mul(e.into()))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(v.into())))
			.saturating_add(T::DbWeight::get().writes(8))
			.saturating_add(Weight::from_parts(0, 206).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 11).saturating_mul(e.into()))
			.saturating_add(Weight::from_parts(0, 2473).saturating_mul(v.into()))
	}
}
