//! THIS SHOULD BE AUTO-GENERATED, BUT FOR NOW IT'S JUST A REMINDER TO
//! AUTO-GENERATE THIS!

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_collective.
pub trait WeightInfo {
	fn execute(_b: u32) -> Weight;
}

/// Weights for pallet_collective using the Substrate node and recommended
/// hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn execute(b: u32) -> Weight {
		(31_147_000 as Weight)
			.saturating_add((4_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn execute(b: u32) -> Weight {
		(31_147_000 as Weight)
			.saturating_add((4_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
	}
}
