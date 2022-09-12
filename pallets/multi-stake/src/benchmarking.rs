//! Benchmarking setup for pallet-proposal
// TODO: Proper Benchmarking

use super::*;

#[allow(unused)]
use crate::Pallet as Template; // TODO: Change pallet name
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	do_something {
		// TODO: Prepare the env
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), s)
	verify {
		// TODO: call a function/storage in the pallet
		assert_eq!(Something::<T>::get(), Some(s));
	}

	impl_benchmark_test_suite!(Template, crate::mock::new_test_ext(), crate::mock::Test);
}
