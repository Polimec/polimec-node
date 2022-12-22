//! Benchmarking setup for pallet-funding
#![cfg(feature = "runtime-benchmarks")]

#[allow(unused)]
use crate::Pallet as PolimecFunding;

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin as SystemOrigin;

// fn default_project_id<T: Config>() -> T::ProjectIdentifier {
// 	T::BenchmarkHelper::create_project_id_parameter(0)
// }

#[allow(unused)]
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn create_default_project<T: Config>() -> (
	// T::ProjectIdentifier,
	T::AccountId,
	Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
) {
	// let project_id = default_project_id::<T>();
	let caller: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	let project = T::BenchmarkHelper::create_dummy_project(caller.clone());
	(caller, project)
}

benchmarks! {
	create {
		let (issuer, project) = create_default_project::<T>();
	}: _(SystemOrigin::Signed(issuer), project)
	verify {
		// assert_last_event::<T>(Event::ProjectCreated(0).into());
	}

	impl_benchmark_test_suite!(PolimecFunding, crate::mock::new_test_ext(), crate::mock::Test);
}
