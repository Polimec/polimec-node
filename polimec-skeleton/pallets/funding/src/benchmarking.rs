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

use super::USDT_STATEMINT_ID;
#[allow(unused)]
use crate::Pallet as PolimecFunding;

type PolimecSystem<T> = frame_system::Pallet<T>;

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_support::{
	assert_ok,
	traits::{
		fungible::Mutate,
		fungibles::{Inspect, Mutate as FungiblesMutate},
		Hooks,
	},
};
use frame_system::{Pallet as System, RawOrigin as SystemOrigin};
use sp_runtime::traits::Hash;

const METADATA: &str = r#"
{
    "whitepaper":"ipfs_url",
    "team_description":"ipfs_url",
    "tokenomics":"ipfs_url",
    "roadmap":"ipfs_url",
    "usage_of_founds":"ipfs_url"
}
"#;

const EDIT_METADATA: &str = r#"
{
    "whitepaper":"new_ipfs_url",
    "team_description":"new_ipfs_url",
    "tokenomics":"new_ipfs_url",
    "roadmap":"new_ipfs_url",
    "usage_of_founds":"new_ipfs_url"
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
		.collect::<frame_benchmarking::Vec<_>>()
}

fn create_default_project<T: Config>(id: Option<u32>) -> (T::ProjectIdParameter, T::AccountId, ProjectMetadataOf<T>) {
	let issuer: T::AccountId = account::<T::AccountId>("Alice", 1, 1);
	let project_id_parameter = id.unwrap_or(0);
	let project_id = T::BenchmarkHelper::create_project_id_parameter(project_id_parameter);
	let metadata_hash = T::Hashing::hash_of(&METADATA);
	let project = T::BenchmarkHelper::create_dummy_project(metadata_hash);
	(project_id, issuer, project)
}

fn create_default_minted_project<T: Config>(id: Option<u32>) -> (T::ProjectIdParameter, T::AccountId) {
	let (project_id, issuer, project) = create_default_project::<T>(id);
	assert!(PolimecFunding::<T>::create(SystemOrigin::Signed(issuer.clone()).into(), project).is_ok());
	(project_id, issuer)
}

pub fn run_to_block<T: Config>(n: T::BlockNumber) {
	let max_weight = T::BlockWeights::get().max_block;
	while frame_system::Pallet::<T>::block_number() < n {
		crate::Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
		frame_system::Pallet::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_idle(frame_system::Pallet::<T>::block_number(), max_weight);
		frame_system::Pallet::<T>::set_block_number(frame_system::Pallet::<T>::block_number() + One::one());
		frame_system::Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_initialize(frame_system::Pallet::<T>::block_number());
		crate::Pallet::<T>::on_idle(frame_system::Pallet::<T>::block_number(), max_weight);
	}
}

benchmarks! {
	create {
		let (_, issuer, project) = create_default_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project)
	verify {
		// assert_last_event::<T>(Event::ProjectCreated(0).into());
		let project_id = T::BenchmarkHelper::create_project_id_parameter(0);
		let project_details = PolimecFunding::<T>::project_details(project_id.into()).unwrap();
		assert_eq!(project_details.project_status, ProjectStatus::Application);
		assert!(!project_details.is_frozen);

	}

	edit_metadata {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		let hash = T::Hashing::hash_of(&EDIT_METADATA);
		let project_details = PolimecFunding::<T>::project_details(project_id.clone().into()).unwrap();
	}: _(SystemOrigin::Signed(issuer), project_id, hash)
	verify {
		let project_id = T::BenchmarkHelper::create_project_id_parameter(0);
		let project_details = PolimecFunding::<T>::project_details(project_id.into()).unwrap();
		assert_eq!(project_details.project_status, ProjectStatus::Application);
	}

	start_evaluation {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
	}: _(SystemOrigin::Signed(issuer), project_id)

	bond_evaluation {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 2_000_000_000_000_u64.into());
		let _ = PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer).into(), project_id.clone());
	}: _(SystemOrigin::Signed(evaluator), project_id, 10_000_000_000_u64.into())

	start_auction {
		// Create and register a project
		let (project_id, issuer) = create_default_minted_project::<T>(None);

		// Start the evaluation round
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);

		// Create evaluator account
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 500_000__0_000_000_000_u64.into()); // 500k tokens
		// Bond minimum amount (currently 10% of 1MM tokens)
		assert!(
			PolimecFunding::<T>::bond_evaluation(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);

		// Move to a block valid for starting the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 2_u32.into());

	}: _(SystemOrigin::Signed(issuer), project_id)

	bid {
		// Create and register a project
		let (project_id, issuer) = create_default_minted_project::<T>(None);

		// Start the evaluation round
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);

		// Create evaluator account
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 500_000__0_000_000_000_u64.into()); // 500k tokens
		// Bond minimum amount (currently 10% of 1MM tokens)
		assert!(
			PolimecFunding::<T>::bond_evaluation(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);

		// Move to a block valid for starting the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 2_u32.into());

		// Fund bid accounts
		let bidder_1: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens

		// Start the Auction round
		assert_ok!(PolimecFunding::<T>::start_auction(SystemOrigin::Signed(issuer).into(), project_id.clone()));

	}: _(SystemOrigin::Signed(bidder_1.clone()), project_id.clone(), 10_000_u64.into(), 15__0_000_000_000_u64.into(), None)
	verify {
		let project_auctions = AuctionsInfo::<T>::get(project_id.clone().into(), bidder_1.clone()).unwrap();
		assert_eq!(project_auctions.len(), 1);
		assert_eq!(project_auctions[0].ct_amount, 10_000_u64.into());
		assert_eq!(project_auctions[0].ct_usd_price, 15__0_000_000_000_u64.into());
		let events = PolimecSystem::<T>::events();
		assert!(events.iter().any(|r| {
			let expected_event: <T as Config>::RuntimeEvent = Event::<T>::Bid {
				project_id: project_id.clone().into(),
				amount: 10_000_u64.into(),
				price: 15__0_000_000_000_u64.into(),
				multiplier: Default::default(),
			}.into();
			matches!(
				r.event.clone(),
				expected_event
			)
		}));
	}


	contribute {
		// create and register a project
		let (project_id, issuer) = create_default_minted_project::<T>(None);

		// Start the evaluation round
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);

		// have an evaluator bond the minimum amount to proceed to the auction round
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 500_000__0_000_000_000_u64.into());
		assert!(
			PolimecFunding::<T>::bond_evaluation(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);

		// Move to a block valid for starting the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 2_u32.into());

		// fund bid accounts
		let bidder_1: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens

		let bidder_2: T::AccountId = account::<T::AccountId>("Charlie", 1, 1);
		T::NativeCurrency::set_balance(&bidder_2, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_2, 500_000__0_000_000_000_u64.into()); // 500k tokens

		let bidder_3: T::AccountId = account::<T::AccountId>("Dave", 1, 1);
		T::NativeCurrency::set_balance(&bidder_3, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_3, 500_000__0_000_000_000_u64.into()); // 500k tokens

		// Start the Auction round
		assert_ok!(PolimecFunding::<T>::start_auction(SystemOrigin::Signed(issuer).into(), project_id.clone()));

		run_to_block::<T>(System::<T>::block_number() + 1u32.into());

		// Place bids
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_1).into(), project_id.clone(), 300u64.into(), 1__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_2).into(), project_id.clone(), 400u64.into(), 1__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_3).into(), project_id.clone(), 500u64.into(), 1__0_000_000_000_u64.into(), None)
		);

		// Move past the Auction limit block
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EnglishAuctionDuration::get() + <T as Config>::CandleAuctionDuration::get() + 1u32.into());

		let project_details = PolimecFunding::<T>::project_details(project_id.clone().into()).unwrap();

		// Create contributor account
		let contributor: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&contributor,  500_000__0_000_000_000_u64.into());
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &contributor,  500_000__0_000_000_000_u64.into());

	}: _(SystemOrigin::Signed(contributor), project_id, 1000_u64.into(), Default::default())

	vested_contribution_token_purchase_mint_for {
		// Create and register a project
		let (project_id, issuer) = create_default_minted_project::<T>(None);

		// Start the evaluation round
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);

		// Have an evaluator bond the minimum amount to proceed to the auction round
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 500_000__0_000_000_000_u64.into()); // 500k tokens
		assert!(
			PolimecFunding::<T>::bond_evaluation(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);

		// Move to a block valid for starting the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 2_u32.into());

		// fund bid accounts
		let bidder_1: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_1, 500_000__0_000_000_000_u64.into()); // 500k tokens

		let bidder_2: T::AccountId = account::<T::AccountId>("Charlie", 1, 1);
		T::NativeCurrency::set_balance(&bidder_2, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_2, 500_000__0_000_000_000_u64.into()); // 500k tokens

		let bidder_3: T::AccountId = account::<T::AccountId>("Dave", 1, 1);
		T::NativeCurrency::set_balance(&bidder_3, 500_000__0_000_000_000_u64.into()); // 500k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_3, 500_000__0_000_000_000_u64.into()); // 500k tokens

		// Start the Auction round
		assert_ok!(PolimecFunding::<T>::start_auction(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()));
		// Place bids
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_1).into(), project_id.clone(), 100u64.into(), 15__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_2).into(), project_id.clone(), 200u64.into(), 20__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_3).into(), project_id.clone(), 300u64.into(), 10__0_000_000_000_u64.into(), None)
		);

		// Move past the Auction limit block
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EnglishAuctionDuration::get() + <T as Config>::CandleAuctionDuration::get() + 2u32.into());

		// Create contributor account
		let contributor: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&contributor,  500_000__0_000_000_000_u64.into());
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &contributor,  500_000__0_000_000_000_u64.into());

		run_to_block::<T>(System::<T>::block_number() + 1u32.into());

		// The contributor wants to buy 2000 CT
		assert_ok!(
			PolimecFunding::<T>::contribute(SystemOrigin::Signed(contributor.clone()).into(), project_id.clone(), 2000_u64.into(), Default::default())
		);
		// Move to the end of the funding round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::CommunityFundingDuration::get() + <T as Config>::RemainderFundingDuration::get() + 1u32.into());

	}: _(SystemOrigin::Signed(contributor.clone()), project_id.clone(), contributor.clone())
	verify {
		let transfered_ct_to_contributor = T::ContributionTokenCurrency::balance(project_id.clone().into(), &contributor);
		assert_eq!(transfered_ct_to_contributor, 2000_u64.into());
	}

	calculate_weighted_price {
		let (project_id, issuer) = create_default_minted_project::<T>(None);
		assert!(
			PolimecFunding::<T>::start_evaluation(SystemOrigin::Signed(issuer.clone()).into(), project_id.clone()).is_ok()
		);
		let evaluator: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&evaluator, 500_000__0_000_000_000_u64.into()); // 100k tokens

		// minimum value is a million tokens. 10% of that needs to be bonded
		assert!(
			PolimecFunding::<T>::bond_evaluation(SystemOrigin::Signed(evaluator).into(), project_id.clone(), 100_000__0_000_000_000_u64.into()).is_ok()
		);
		let bidder_1: T::AccountId = account::<T::AccountId>("Bob", 1, 1);
		T::NativeCurrency::set_balance(&bidder_1, 500_000__0_000_000_000_u64.into()); // 100k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_1, 500_000__0_000_000_000_u64.into()); // 100k tokens

		let bidder_2: T::AccountId = account::<T::AccountId>("Charlie", 1, 1);
		T::NativeCurrency::set_balance(&bidder_2, 500_000__0_000_000_000_u64.into()); // 100k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_2, 500_000__0_000_000_000_u64.into()); // 100k tokens

		let bidder_3: T::AccountId = account::<T::AccountId>("Dave", 1, 1);
		T::NativeCurrency::set_balance(&bidder_3, 500_000__0_000_000_000_u64.into()); // 100k tokens
		T::FundingCurrency::set_balance(USDT_STATEMINT_ID, &bidder_3, 500_000__0_000_000_000_u64.into()); // 100k tokens

		// Move to the Auction Round
		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EvaluationDuration::get() + 2_u32.into());
		assert_ok!(PolimecFunding::<T>::start_auction(SystemOrigin::Signed(issuer).into(), project_id.clone()));

		let project_details = ProjectsDetails::<T>::get(project_id.clone().into()).unwrap();
		let fundraising_target = project_details.fundraising_target;

		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_1).into(), project_id.clone(), 100u64.into(), 15__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_2).into(), project_id.clone(), 200u64.into(), 20__0_000_000_000_u64.into(), None)
		);
		assert_ok!(
			PolimecFunding::<T>::bid(SystemOrigin::Signed(bidder_3).into(), project_id.clone(), 300u64.into(), 10__0_000_000_000_u64.into(), None)
		);

		run_to_block::<T>(System::<T>::block_number() + <T as Config>::EnglishAuctionDuration::get() + 5_u32.into());

		let random_ending_point =  System::<T>::block_number() - 2_u32.into();

	}: {
		crate::Pallet::<T>::calculate_weighted_average_price(project_id.clone().into(), random_ending_point, fundraising_target).unwrap();
	}
	verify {
		let project_details = ProjectsDetails::<T>::get(project_id.clone().into()).unwrap();
		let weighted_average_price = project_details.weighted_average_price.unwrap();
		assert_eq!(weighted_average_price, 15__5_882_352_800_u64.into());
	}

	impl_benchmark_test_suite!(PolimecFunding, crate::mock::new_test_ext(), crate::mock::TestRuntime);
}
