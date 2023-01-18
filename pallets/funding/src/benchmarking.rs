//! Benchmarking setup for pallet-funding
#![cfg(feature = "runtime-benchmarks")]

#[allow(unused)]
use crate::Pallet as PolimecFunding;

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};

const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}
"#;

#[allow(unused)]
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[allow(unused)]
fn get_events<T: Config>() -> frame_benchmarking::Vec<<T as frame_system::Config>::RuntimeEvent> {
	frame_system::Pallet::<T>::events()
		.into_iter()
		.map(|r| r.event)
		.collect::<Vec<_>>()
}

fn create_default_project<T: Config>(
	id: Option<u32>,
) -> (T::ProjectIdParameter, T::AccountId, ProjectOf<T>) {
	let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	let project_id_parameter = id.unwrap_or(0);
	let project_id = T::BenchmarkHelper::create_project_id_parameter(project_id_parameter);
	let metadata_hash = store_and_return_metadata_hash::<T>();
	let project = T::BenchmarkHelper::create_dummy_project(issuer.clone(), metadata_hash);
	(project_id, issuer, project)
}

fn create_default_minted_project<T: Config>(
	id: Option<u32>,
) -> (T::ProjectIdParameter, T::AccountId) {
	let (project_id, issuer, project) = create_default_project::<T>(id);
	assert!(
		PolimecFunding::<T>::create(SystemOrigin::Signed(issuer.clone()).into(), project,).is_ok()
	);
	(project_id, issuer)
}

fn store_and_return_metadata_hash<T: Config>() -> T::Hash {
	let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	assert!(PolimecFunding::<T>::note_image(
		SystemOrigin::Signed(issuer.clone()).into(),
		METADATA.into(),
	)
	.is_ok());
	// TODO: Get the hash from the Noted event
	T::Hashing::hash(METADATA.as_bytes())
}

benchmarks! {
	note_image {
		let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	}: _(SystemOrigin::Signed(issuer), METADATA.into())
	verify {
		let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
		let hash = T::Hashing::hash(METADATA.as_bytes());
		// Let it panic if the image is not found
		let image_issuer = PolimecFunding::<T>::images(hash).unwrap();
		assert_eq!(issuer, image_issuer);
	}

	create {
		let (_, issuer, project) = create_default_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project)
	verify {
		// assert_last_event::<T>(Event::ProjectCreated(0).into());
		let project_id = T::BenchmarkHelper::create_project_id_parameter(1);
		let project_info = PolimecFunding::<T>::project_info(project_id.into());
		assert_eq!(project_info.project_status, ProjectStatus::Application);
	}

	start_evaluation {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project_id)

	on_finalize {
		let p = T::ActiveProjectsLimit::get();
		// Create 100 projects
		for i in 0 .. p {
			let (project_id, issuer) = create_default_minted_project::<T>(Some(i));
			assert!(
				PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id).is_ok()
			);
		}
		// Move at the end of the Evaluation Round
		System::<T>::set_block_number(System::<T>::block_number() + 29_u32.into());
	} : {
		PolimecFunding::<T>::on_finalize(System::<T>::block_number());
	}
	verify {
		let p = T::ActiveProjectsLimit::get();
		for i in 0 .. p {
			let project_id = T::BenchmarkHelper::create_project_id_parameter(i);
			let project_info = PolimecFunding::<T>::project_info(project_id.into());
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationEnded);
		}

	}

	// claim_contribution_tokens {
	// }: _(SystemOrigin::Signed(issuer), project_id)
	// verify {
	// }

	impl_benchmark_test_suite!(PolimecFunding, crate::mock::new_test_ext(), crate::mock::Test);
}
