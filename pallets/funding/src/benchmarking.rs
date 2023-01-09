//! Benchmarking setup for pallet-funding
#![cfg(feature = "runtime-benchmarks")]

#[allow(unused)]
use crate::Pallet as PolimecFunding;

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};

#[allow(unused)]
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn create_default_project<T: Config>(id: Option<u32>) -> (
	T::ProjectIdentifier,
	T::AccountId,
	Project<T::AccountId, BoundedVec<u8, T::StringLimit>, BalanceOf<T>>,
) {
	let project_id_parameter = id.unwrap_or(0);
	let project_id = T::BenchmarkHelper::create_project_id_parameter(project_id_parameter);
	let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	let project = T::BenchmarkHelper::create_dummy_project(issuer.clone());
	(project_id, issuer, project)
}

fn create_default_minted_project<T: Config>(id: Option<u32>) -> (T::ProjectIdentifier, T::AccountId) {
	let (project_id, issuer, project) = create_default_project::<T>(id);
	assert!(
		PolimecFunding::<T>::create(SystemOrigin::Signed(issuer.clone()).into(), project,).is_ok()
	);
	(project_id, issuer)
}

benchmarks! {
	create {
		let (_, issuer, project) = create_default_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project)
	verify {
		// assert_last_event::<T>(Event::ProjectCreated(0).into());
		let project_id = T::BenchmarkHelper::create_project_id_parameter(1);
		let project_info = PolimecFunding::<T>::project_info(project_id);
		assert_eq!(project_info.project_status, ProjectStatus::Application);
	}

	start_evaluation {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project_id)

	on_finalize {
		/* code to set the initial state */
		let p = T::ActiveProjectsLimit::get();
		// Create 100 projects
		for i in 0 .. p {
			let (project_id, issuer) = create_default_minted_project::<T>(Some(i));
			assert!(
				PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id).is_ok()
			);
		}
		let block_number = System::<T>::block_number();
		// Move at the end of the evaluation period
		System::<T>::set_block_number(block_number + 29_u32.into());
		let block_number = System::<T>::block_number();
	} : {
		 /* code to test the function benchmarked */
		PolimecFunding::<T>::on_finalize(block_number);
	}

	verify {
		/* optional verification */
		let p = T::ActiveProjectsLimit::get();
		for i in 0 .. p {
			let project_id = T::BenchmarkHelper::create_project_id_parameter(i);
			let project_info = PolimecFunding::<T>::project_info(project_id);
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationEnded);
		}

	}

	impl_benchmark_test_suite!(PolimecFunding, crate::mock::new_test_ext(), crate::mock::Test);
}
