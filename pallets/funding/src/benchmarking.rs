// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! Benchmarking setup for Funding pallet

#![cfg(feature = "runtime-benchmarks")]

#[allow(unused)]
use crate::Pallet as PolimecFunding;

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};
use frame_support::{
	assert_ok,
	traits::Hooks,
};

const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}
"#;

fn metadata_as_vec() -> frame_benchmarking::Vec<u8> {
	METADATA.as_bytes().to_vec()
}

#[allow(unused)]
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[allow(unused)]
fn get_events<T: Config>() -> frame_benchmarking::Vec<<T as frame_system::Config>::RuntimeEvent> {
	frame_system::Pallet::<T>::events()
		.into_iter()
		.map(|r| r.event)
		.collect::<frame_benchmarking::Vec<_>>()
}

fn create_default_project<T: Config>(
	id: Option<u32>,
) -> (T::ProjectIdParameter, T::AccountId, ProjectOf<T>) {
	let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	let project_id_parameter = id.unwrap_or(0);
	let project_id = T::BenchmarkHelper::create_project_id_parameter(project_id_parameter);
	let metadata_hash = store_and_return_metadata_hash::<T>();
	let project = T::BenchmarkHelper::create_dummy_project(metadata_hash);
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
	let bounded_metadata = BoundedVec::try_from(metadata_as_vec()).unwrap();
	assert!(
		PolimecFunding::<T>::note_image(SystemOrigin::Signed(issuer).into(), bounded_metadata,)
			.is_ok()
	);
	// TODO: PLMC-140. Get the hash from the Noted event
	T::Hashing::hash(METADATA.as_bytes())
}

pub fn run_to_block<T: Config>(n: T::BlockNumber) {
	let max_weight = T::BlockWeights::get().max_block;
	while frame_system::Pallet::<T>::block_number() < n {
		crate::Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
		frame_system::Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_idle(frame_system::Pallet::<T>::block_number(), max_weight);
		frame_system::Pallet::<T>::set_block_number(
			frame_system::Pallet::<T>::block_number() + One::one(),
		);
		frame_system::Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_idle(frame_system::Pallet::<T>::block_number(), max_weight);
	}
}

benchmarks! {
	note_image {
		let bounded_metadata = BoundedVec::try_from(metadata_as_vec()).unwrap();
		let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	}: _(SystemOrigin::Signed(issuer), bounded_metadata)
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
		let project_info = PolimecFunding::<T>::project_info(project_id.into()).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::Application);
	}

	start_evaluation {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project_id)

	bond {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 2_000_000_000_000_u64.into());
		let _ = PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer).into(), project_id.clone());
	}: _(SystemOrigin::Signed(evaluator), project_id, 10_000_000_000_u64.into())

	edit_metadata {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		let bounded_metadata = BoundedVec::try_from(metadata_as_vec()).unwrap();
		assert!(
			PolimecFunding::<T>::note_image(SystemOrigin::Signed(issuer.clone()).into(), bounded_metadata).is_ok()
		);
		let hash = T::Hashing::hash(METADATA.as_bytes());
	}: _(SystemOrigin::Signed(issuer), project_id, hash)

	start_auction {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 2_000_000_000_000_u64.into());
		assert!(
			PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 10_000_000_000_u64.into()).is_ok()
		);
		// Move at the end of the Evaluation Round
		run_to_block::<T>(System::<T>::block_number() + 30_u32.into());
	}: _(SystemOrigin::Signed(issuer), project_id)

	bid {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 2_000_000_000_000_u64.into());
		assert!(
			PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 10_000_000_000_u64.into()).is_ok()
		);
		// Move in the middle of the Auction Round
		run_to_block::<T>(System::<T>::block_number() + 40_u32.into());
		let bidder: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&bidder, 2_000_000_000_000_u64.into());
	}: _(SystemOrigin::Signed(bidder), project_id, 100u64.into(), 2_000_000_000_u64.into(), None)

	contribute {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 2_000_000_000_000_u64.into());
		assert!(
			PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 10_000_000_000_u64.into()).is_ok()
		);
		// Move in the middle of the Community Round
		run_to_block::<T>(System::<T>::block_number() + 55_u32.into());
		let contributor: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&contributor,  20_000_000_000_u64.into());
	}: _(SystemOrigin::Signed(contributor), project_id, 2_000_000_000_u64.into())

	claim_contribution_tokens {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 20_000_000_000_000_u64.into());
		assert!(
			PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 10_000_000_000_u64.into()).is_ok()
		);
		// Move in the middle of the Auction Round
		run_to_block::<T>(System::<T>::block_number() + 40_u32.into());
		let bidder: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		assert!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder).into(), project_id.clone(), 100u64.into(), 2_000_000_000_u64.into(), None).is_ok()
		);

		// Move in the middle of the Community Round
		run_to_block::<T>(System::<T>::block_number() + 15_u32.into());
		let claimer: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		assert!(
			PolimecFunding::<T>::contribute(SystemOrigin::Signed(claimer.clone()).into(), project_id.clone(),  2_000_000_000_u64.into()).is_ok()
		);
		// Move at the end of the Community Round
		run_to_block::<T>(System::<T>::block_number() + 20_u32.into());
	}: _(SystemOrigin::Signed(claimer), project_id)

	on_initialize {
		let p = T::ActiveProjectsLimit::get();
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 2_000_000_000_000_u64.into());
		// Create 100 projects
		for i in 0 .. p {
			let (project_id, issuer) = create_default_minted_project::<T>(Some(i));
			assert!(
				PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
			);
			assert!(
				PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator.clone()).into(), project_id, 1_000_u64.into()).is_ok()
			);
		}
		// Move at the end of the Evaluation Round
		System::<T>::set_block_number(System::<T>::block_number() + 29_u32.into());
		// TODO: PLMC-139. Benchmark the hook when computing the Funding Round results
	} : {
		PolimecFunding::<T>::on_initialize(System::<T>::block_number());
	}
	verify {
		let p = T::ActiveProjectsLimit::get();
		for i in 0 .. p {
			let project_id = T::BenchmarkHelper::create_project_id_parameter(i);
			let project_info = PolimecFunding::<T>::project_info(project_id.into()).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationEnded);
		}
	}

	calculate_weighted_price {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&evaluator, 500_000__0_000_000_000_u64.into()); // 100k tokens

		// minimum value is a million tokens. 10% of that needs to be bonded
		assert!(
			PolimecFunding::<T>::bond(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);
		let bidder_1: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::Currency::make_free_balance_be(&bidder_1, 500_000__0_000_000_000_u64.into()); // 100k tokens

		let bidder_2: T::AccountId = account::<T::AccountId>("Charlie", 1, 1);
		T::Currency::make_free_balance_be(&bidder_2, 500_000__0_000_000_000_u64.into()); // 100k tokens

		let bidder_3: T::AccountId = account::<T::AccountId>("Dave", 1, 1);
		T::Currency::make_free_balance_be(&bidder_3, 500_000__0_000_000_000_u64.into()); // 100k tokens

		// Move to the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 1_u32.into());
		assert_ok!(PolimecFunding::<T>::start_auction(SystemOrigin::Signed(issuer).into(), project_id.clone()));


		let mut project_info = <pallet::Pallet<T> as Store>::ProjectsInfo::get(project_id.clone().into()).unwrap();
		let fundraising_target = project_info.fundraising_target;


		assert!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_1).into(), project_id.clone(), 10_000u64.into(), 15__0_000_000_000_u64.into(), None).is_ok()
		);
		assert!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_2).into(), project_id.clone(), 20_000u64.into(), 20__0_000_000_000_u64.into(), None).is_ok()
		);
		assert!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_3).into(), project_id.clone(), 20_000u64.into(), 10__0_000_000_000_u64.into(), None).is_ok()
		);

		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EnglishAuctionDuration::get() + 5_u32.into());

		let random_ending_point =  System::<T>::block_number() - 2_u32.into();

		let mut weighted_average_price = 0u32.into();

	}: {
		weighted_average_price = crate::Pallet::<T>::calculate_weighted_average_price(project_id.into(), random_ending_point, fundraising_target).unwrap();
	}
	verify {
		// TODO: PLMC-147 test locking and unlocking of funds
		assert_eq!(weighted_average_price, 16__3_333_333_200_u64.into());
	}

	impl_benchmark_test_suite!(PolimecFunding, crate::mock::new_test_ext(), crate::mock::Test);
}
