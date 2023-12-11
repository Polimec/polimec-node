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

// If you feel like getting in touch with us, you ca ,n do so at info@polimec.org

//! Tests for Funding pallet.
use assert_matches2::assert_matches;
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold},
		Get,
	},
};
use itertools::Itertools;
use parachains_common::DAYS;
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_std::{cell::RefCell, marker::PhantomData};
use std::{cmp::min, iter::zip};

use defaults::*;
use polimec_traits::ReleaseSchedule;

use crate::{
	instantiator::*,
	mock::*,
	traits::{ProvideStatemintPrice, VestingDurationCalculation},
	CurrencyMetadata, Error, ParticipantsSize, ProjectMetadata, TicketSize,
	UpdateType::{CommunityFundingStart, RemainderFundingStart},
};

use mock::TestRuntime;

use super::*;

type MockInstantiator = Instantiator<TestRuntime, AllPalletsWithoutSystem, RuntimeEvent>;

const METADATA: &str = r#"METADATA
            {
                "whitepaper":"ipfs_url",
                "team_description":"ipfs_url",
                "tokenomics":"ipfs_url",
                "roadmap":"ipfs_url",
                "usage_of_founds":"ipfs_url"
            }"#;
const ASSET_DECIMALS: u8 = 10;
const ISSUER: AccountId = 10;
const EVALUATOR_1: AccountId = 20;
const EVALUATOR_2: AccountId = 21;
const EVALUATOR_3: AccountId = 22;
const BIDDER_1: AccountId = 30;
const BIDDER_2: AccountId = 31;
const BIDDER_3: AccountId = 32;
const BIDDER_4: AccountId = 33;
const BIDDER_5: AccountId = 34;
const BUYER_1: AccountId = 40;
const BUYER_2: AccountId = 41;
const BUYER_3: AccountId = 42;
const BUYER_4: AccountId = 43;
const BUYER_5: AccountId = 44;
const BUYER_6: AccountId = 45;
const BUYER_7: AccountId = 46;

const ASSET_UNIT: u128 = 10_u128.pow(10u32);

const USDT_STATEMINT_ID: crate::mock::AssetId = 1984u32;
const USDT_UNIT: u128 = 10_000_000_000_u128;

pub const US_DOLLAR: u128 = 1_0_000_000_000;

pub mod defaults {
	use super::*;

	pub fn default_project(nonce: u64, issuer: AccountId) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		ProjectMetadata {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: (50_000 * ASSET_UNIT, 50_000 * ASSET_UNIT),
			minimum_price: PriceOf::<TestRuntime>::from_float(1.0),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: AcceptedFundingAsset::USDT,
			funding_destination_account: issuer,
			offchain_information_hash: Some(metadata_hash),
		}
	}

	pub fn default_plmc_balances() -> Vec<UserToPLMCBalance<TestRuntime>> {
		vec![
			UserToPLMCBalance::new(ISSUER, 20_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_1, 35_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_2, 60_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_3, 100_000 * PLMC),
			UserToPLMCBalance::new(BIDDER_1, 500_000 * PLMC),
			UserToPLMCBalance::new(BIDDER_2, 300_000 * PLMC),
			UserToPLMCBalance::new(BUYER_1, 30_000 * PLMC),
			UserToPLMCBalance::new(BUYER_2, 30_000 * PLMC),
		]
	}

	pub fn default_evaluations() -> Vec<UserToUSDBalance<TestRuntime>> {
		vec![
			UserToUSDBalance::new(EVALUATOR_1, 50_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_2, 25_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_3, 32_000 * PLMC),
		]
	}

	pub fn default_failing_evaluations() -> Vec<UserToUSDBalance<TestRuntime>> {
		vec![UserToUSDBalance::new(EVALUATOR_1, 3_000 * PLMC), UserToUSDBalance::new(EVALUATOR_2, 1_000 * PLMC)]
	}

	pub fn default_bids() -> Vec<BidParams<TestRuntime>> {
		// This should reflect the bidding currency, which currently is USDT
		vec![
			BidParams::new(BIDDER_1, 40_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 5_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_community_buys() -> Vec<ContributionParams<TestRuntime>> {
		vec![
			ContributionParams::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_remainder_buys() -> Vec<ContributionParams<TestRuntime>> {
		vec![
			ContributionParams::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_weights() -> Vec<u8> {
		vec![20u8, 15u8, 10u8, 25u8, 30u8]
	}

	pub fn default_bidders() -> Vec<AccountId> {
		vec![BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5]
	}
	pub fn default_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}

	pub fn default_contributors() -> Vec<AccountId> {
		vec![BUYER_1, BUYER_2, BUYER_3, BUYER_4, BUYER_5]
	}

	pub fn project_from_funding_reached(instantiator: &mut MockInstantiator, percent: u64) -> ProjectIdOf<TestRuntime> {
		let project_metadata = default_project(instantiator.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let usd_to_reach = Perquintill::from_percent(percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * usd_to_reach,
			min_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(50u8) * usd_to_reach,
			min_price,
			default_weights(),
			default_contributors(),
			default_multipliers(),
		);
		instantiator.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![])
	}
}

mod creation_round_success {
	use super::*;

	#[test]
	fn basic_plmc_transfer_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		inst.mint_plmc_to(default_plmc_balances());

		inst.execute(|| {
			assert_ok!(Balances::transfer(RuntimeOrigin::signed(EVALUATOR_1), EVALUATOR_2, PLMC));
		});
	}

	#[test]
	fn creation_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);

		inst.create_evaluating_project(project, issuer);
	}

	#[test]
	fn multiple_creation_rounds() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		for _ in 0..512 {
			let project = default_project(inst.get_new_nonce(), issuer);
			inst.create_evaluating_project(project, issuer);
		}
	}

	#[test]
	fn project_id_autoincrement_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_1 = default_project(inst.get_new_nonce(), issuer);
		let project_2 = default_project(inst.get_new_nonce(), issuer);
		let project_3 = default_project(inst.get_new_nonce(), issuer);

		let created_project_1_id = inst.create_evaluating_project(project_1, ISSUER);
		let created_project_2_id = inst.create_evaluating_project(project_2, ISSUER);
		let created_project_3_id = inst.create_evaluating_project(project_3, ISSUER);

		assert_eq!(created_project_1_id, 0);
		assert_eq!(created_project_2_id, 1);
		assert_eq!(created_project_3_id, 2);
	}
}

mod creation_round_failure {
	use super::*;

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 0_u128.into(),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.mint_plmc_to(default_plmc_balances());
		let project_err =
			inst.execute(|| Pallet::<TestRuntime>::create(RuntimeOrigin::signed(ISSUER), wrong_project).unwrap_err());
		assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into());
	}

	#[test]
	fn participants_size_error() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 1_u128.into(),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: None, maximum: None },
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.mint_plmc_to(default_plmc_balances());

		let project_err =
			inst.execute(|| Pallet::<TestRuntime>::create(RuntimeOrigin::signed(ISSUER), wrong_project).unwrap_err());
		assert_eq!(project_err, Error::<TestRuntime>::ParticipantsSizeError.into());
	}

	#[test]
	fn ticket_size_error() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 1_u128.into(),
			ticket_size: TicketSize { minimum: None, maximum: None },
			participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.mint_plmc_to(default_plmc_balances());

		let project_err =
			inst.execute(|| Pallet::<TestRuntime>::create(RuntimeOrigin::signed(ISSUER), wrong_project).unwrap_err());
		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
	}
}

mod evaluation_round_success {
	use super::*;

	#[test]
	fn evaluation_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		inst.create_auctioning_project(project, issuer, evaluations);
	}

	#[test]
	fn multiple_evaluation_projects() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project1 = default_project(inst.get_new_nonce(), issuer);
		let project2 = default_project(inst.get_new_nonce(), issuer);
		let project3 = default_project(inst.get_new_nonce(), issuer);
		let project4 = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		inst.create_auctioning_project(project1, issuer, evaluations.clone());
		inst.create_auctioning_project(project2, issuer, evaluations.clone());
		inst.create_auctioning_project(project3, issuer, evaluations.clone());
		inst.create_auctioning_project(project4, issuer, evaluations);
	}

	#[test]
	fn rewards_are_paid_full_funding() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, 420));
		let project_metadata = ProjectMetadataOf::<TestRuntime> {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: (50_000 * ASSET_UNIT, 50_000 * ASSET_UNIT),
			minimum_price: PriceOf::<TestRuntime>::from_float(1.0),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: AcceptedFundingAsset::USDT,
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
		};

		// all values taken from the knowledge hub
		let evaluations: Vec<UserToUSDBalance<TestRuntime>> = default_evaluations();

		let bids: Vec<BidParams<TestRuntime>> = vec![
			BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 20_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_4, 20_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let contributions: Vec<ContributionParams<_>> = vec![
			ContributionParams::new(BUYER_1, 4_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_4, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_5, 30_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_6, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_7, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let (project_id, _) = inst.create_community_contributing_project(project_metadata, ISSUER, evaluations, bids);
		let details = inst.get_project_details(project_id);
		let ct_price = details.weighted_average_price.unwrap();
		let plmc_deposits = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let existential_deposits = plmc_deposits.accounts().existential_deposits();
		let funding_deposits =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_deposits);
		inst.mint_plmc_to(existential_deposits);
		inst.mint_statemint_asset_to(funding_deposits);

		inst.contribute_for_users(project_id, contributions).unwrap();
		inst.finish_funding(project_id).unwrap();
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10).unwrap();
		let actual_reward_balances = inst.execute(|| {
			vec![
				(EVALUATOR_1, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_1)),
				(EVALUATOR_2, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_2)),
				(EVALUATOR_3, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_3)),
			]
		});
		let expected_ct_rewards =
			vec![(EVALUATOR_1, 17214953271028), (EVALUATOR_2, 5607476635514), (EVALUATOR_3, 6_379_471_698_137)];

		for (real, desired) in zip(actual_reward_balances.iter(), expected_ct_rewards.iter()) {
			assert_eq!(real.0, desired.0, "bad accounts order");
			// 0.01 parts of a Perbill
			assert_close_enough!(real.1, desired.1, Perquintill::from_parts(10_000_000u64));
		}
	}

	#[test]
	fn plmc_unbonded_after_funding_success() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let evaluations = default_evaluations();
		let evaluators = evaluations.accounts();

		let (project_id, _) = inst.create_remainder_contributing_project(
			default_project(inst.get_new_nonce(), ISSUER),
			ISSUER,
			evaluations.clone(),
			default_bids(),
			default_community_buys(),
		);

		let prev_reserved_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Evaluation(project_id));

		let prev_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		inst.finish_funding(project_id).unwrap();
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10).unwrap();
		let post_unbond_amounts: Vec<UserToPLMCBalance<_>> = prev_reserved_plmc
			.iter()
			.map(|UserToPLMCBalance { account, .. }| UserToPLMCBalance::new(*account, Zero::zero()))
			.collect();

		inst.do_reserved_plmc_assertions(post_unbond_amounts.clone(), LockType::Evaluation(project_id));
		inst.do_reserved_plmc_assertions(post_unbond_amounts, LockType::Participation(project_id));

		let post_free_plmc = inst.get_free_plmc_balances_for(evaluators);

		let increased_amounts =
			MockInstantiator::generic_map_operation(vec![post_free_plmc, prev_free_plmc], MergeOperation::Subtract);

		assert_eq!(increased_amounts, MockInstantiator::calculate_evaluation_plmc_spent(evaluations))
	}

	#[test]
	fn plmc_unbonded_after_funding_failure() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let evaluations = default_evaluations();
		let evaluators = evaluations.accounts();

		let (project_id, _) = inst.create_remainder_contributing_project(
			default_project(inst.get_new_nonce(), ISSUER),
			ISSUER,
			evaluations.clone(),
			vec![BidParams::new(BUYER_1, 1000 * ASSET_UNIT, 10u128.into(), 1u8, AcceptedFundingAsset::USDT)],
			vec![ContributionParams::new(BUYER_1, 1000 * US_DOLLAR, 1u8, AcceptedFundingAsset::USDT)],
		);

		let prev_reserved_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Evaluation(project_id));
		let prev_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		inst.finish_funding(project_id).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.advance_time(10).unwrap();

		let post_unbond_amounts: Vec<UserToPLMCBalance<_>> = prev_reserved_plmc
			.iter()
			.map(|UserToPLMCBalance { account, .. }| UserToPLMCBalance::new(*account, Zero::zero()))
			.collect();

		inst.do_reserved_plmc_assertions(post_unbond_amounts.clone(), LockType::Evaluation(project_id));
		inst.do_reserved_plmc_assertions(post_unbond_amounts, LockType::Participation(project_id));

		let post_free_plmc = inst.get_free_plmc_balances_for(evaluators);

		let increased_amounts =
			MockInstantiator::generic_map_operation(vec![post_free_plmc, prev_free_plmc], MergeOperation::Subtract);

		assert_eq!(
			increased_amounts,
			MockInstantiator::slash_evaluator_balances(MockInstantiator::calculate_evaluation_plmc_spent(evaluations))
		)
	}
}

mod evaluation_round_failure {
	use super::*;
	use frame_support::assert_err;
	use sp_runtime::TokenError;
	#[test]
	fn not_enough_bonds() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let now = inst.current_block();
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_failing_evaluations();
		let plmc_eval_deposits: Vec<UserToPLMCBalance<_>> =
			MockInstantiator::calculate_evaluation_plmc_spent(evaluations);
		let plmc_existential_deposits = plmc_eval_deposits.accounts().existential_deposits();
		let expected_evaluator_balances = MockInstantiator::generic_map_operation(
			vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		inst.mint_plmc_to(plmc_eval_deposits.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());

		let project_id = inst.create_evaluating_project(project, issuer);

		let evaluation_end = inst
			.get_project_details(project_id)
			.phase_transition_points
			.evaluation
			.end
			.expect("Evaluation round end block should be set");

		inst.bond_for_users(project_id, default_failing_evaluations()).expect("Bonding should work");

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_reserved_plmc_assertions(plmc_eval_deposits, LockType::Evaluation(project_id));

		inst.advance_time(evaluation_end - now + 1).unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationFailed);

		// Check that on_idle has unlocked the failed bonds
		inst.advance_time(10).unwrap();
		inst.do_free_plmc_assertions(expected_evaluator_balances);
	}

	#[test]
	fn insufficient_balance() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let insufficient_eval_deposits = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone())
			.iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance::new(*account, plmc_amount / 2))
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let plmc_existential_deposits = insufficient_eval_deposits.accounts().existential_deposits();

		inst.mint_plmc_to(insufficient_eval_deposits);
		inst.mint_plmc_to(plmc_existential_deposits);

		let project_id = inst.create_evaluating_project(project, issuer);

		let dispatch_error = inst.bond_for_users(project_id, evaluations);
		assert_err!(dispatch_error, TokenError::FundsUnavailable)
	}
}

mod auction_round_success {
	use super::*;

	#[test]
	fn auction_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let _project_id = inst.create_community_contributing_project(project, issuer, evaluations, bids);
	}

	#[test]
	fn multiple_auction_projects_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project1 = default_project(inst.get_new_nonce(), issuer);
		let project2 = default_project(inst.get_new_nonce(), issuer);
		let project3 = default_project(inst.get_new_nonce(), issuer);
		let project4 = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();

		inst.create_community_contributing_project(project1, issuer, evaluations.clone(), bids.clone());
		inst.create_community_contributing_project(project2, issuer, evaluations.clone(), bids.clone());
		inst.create_community_contributing_project(project3, issuer, evaluations.clone(), bids.clone());
		inst.create_community_contributing_project(project4, issuer, evaluations, bids);
	}

	#[test]
	fn evaluation_bond_counts_towards_bid() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_bidder = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let evaluator_bid =
			BidParams::new(evaluator_bidder, 600 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_bidder, evaluation_amount));

		let project_id = inst.create_auctioning_project(project, issuer, evaluations);

		let already_bonded_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
			evaluator_bidder,
			evaluation_amount,
		)])[0]
			.plmc_amount;
		let usable_evaluation_plmc =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_bid =
			MockInstantiator::calculate_auction_plmc_spent(&vec![evaluator_bid.clone()], None)[0].plmc_amount;
		let necessary_usdt_for_bid =
			MockInstantiator::calculate_auction_funding_asset_spent(&vec![evaluator_bid.clone()], None);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_bidder,
			necessary_plmc_for_bid - usable_evaluation_plmc,
		)]);
		inst.mint_statemint_asset_to(necessary_usdt_for_bid);

		inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();
	}

	#[test]
	fn evaluation_bond_counts_towards_bid_vec_full() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_bidder = 69;
		let evaluator_bid =
			BidParams::new(evaluator_bidder, 600 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT);

		let mut bids = Vec::new();
		for _ in 0..<TestRuntime as Config>::MaxBidsPerUser::get() {
			bids.push(BidParams::new(evaluator_bidder, 100 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT));
		}

		let fill_necessary_plmc_for_bids = MockInstantiator::calculate_auction_plmc_spent(&bids.clone(), None);
		let fill_necessary_usdt_for_bids = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);

		let bid_necessary_plmc = MockInstantiator::calculate_auction_plmc_spent(&vec![evaluator_bid.clone()], None);
		let bid_necessary_usdt =
			MockInstantiator::calculate_auction_funding_asset_spent(&vec![evaluator_bid.clone()], None);

		let evaluation_bond =
			MockInstantiator::sum_balance_mappings(vec![fill_necessary_plmc_for_bids, bid_necessary_plmc]);
		let plmc_available_for_participation =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push(UserToUSDBalance::new(evaluator_bidder, evaluation_usd_amount));

		let project_id = inst.create_auctioning_project(project, issuer, evaluations);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_bidder,
			evaluation_bond - plmc_available_for_participation,
		)]);
		inst.mint_statemint_asset_to(fill_necessary_usdt_for_bids);
		inst.mint_statemint_asset_to(bid_necessary_usdt);

		inst.bid_for_users(project_id, bids).unwrap();
		inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();

		let evaluation_bonded = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_bidder,
			)
		});
		assert_close_enough!(
			evaluation_bonded,
			<TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond,
			Perquintill::from_parts(1_000_000_000)
		);
	}

	#[test]
	fn price_calculation_1() {
		// TODO: Update this test to use the knowledge hub values (when they are available)
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER, default_evaluations());
		let bids = vec![BidParams::new(
			100,
			project_metadata.total_allocation_size.0,
			15.into(),
			1u8,
			AcceptedFundingAsset::USDT,
		)];
		let statemint_funding = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);
		let plmc_funding = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		let ed_funding = plmc_funding.accounts().existential_deposits();

		inst.mint_plmc_to(ed_funding);
		inst.mint_plmc_to(plmc_funding);
		inst.mint_statemint_asset_to(statemint_funding);

		inst.bid_for_users(project_id, bids).unwrap();

		inst.start_community_funding(project_id).unwrap();
		// let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		// let price_in_10_decimals = token_price.checked_mul_int(1_0_000_000_000_u128).unwrap();
		// let price_in_12_decimals = token_price.checked_mul_int(1_000_000_000_000_u128).unwrap();
		// assert_eq!(price_in_10_decimals, 16_3_333_333_333_u128);
		// assert_eq!(price_in_12_decimals, 16_333_333_333_333_u128);
	}

	#[test]
	fn price_calculation_2() {
		// From the knowledge hub
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
		let project_id = inst.create_auctioning_project(project_metadata, ISSUER, default_evaluations());
		let bids = vec![
			BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 40_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_3, 35_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let statemint_funding = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);
		let plmc_funding = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		let ed_funding = plmc_funding.accounts().existential_deposits();

		inst.mint_plmc_to(ed_funding);
		inst.mint_plmc_to(plmc_funding);
		inst.mint_statemint_asset_to(statemint_funding);

		inst.bid_for_users(project_id, bids).unwrap();

		inst.start_community_funding(project_id).unwrap();
		let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap().to_float();

		assert_eq!(token_price, 1.283606557377049);
	}

	#[test]
	fn only_candle_bids_before_random_block_get_included() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let project_id = inst.create_auctioning_project(project, issuer, evaluations);
		let english_end_block = inst
			.get_project_details(project_id)
			.phase_transition_points
			.english_auction
			.end()
			.expect("Auction start point should exist");
		// The block following the end of the english auction, is used to transition the project into candle auction.
		// We move past that transition, into the start of the candle auction.
		let now = inst.current_block();
		inst.advance_time(english_end_block - now + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::Candle));

		let candle_end_block = inst
			.get_project_details(project_id)
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle auction end point should exist");

		let mut bidding_account = 1000;
		let bid_info =
			BidParams::new(0, 50u128, PriceOf::<TestRuntime>::from_float(1.0), 1u8, AcceptedFundingAsset::USDT);
		let plmc_necessary_funding =
			MockInstantiator::calculate_auction_plmc_spent(&vec![bid_info.clone()], None)[0].plmc_amount;
		let statemint_asset_necessary_funding =
			MockInstantiator::calculate_auction_funding_asset_spent(&vec![bid_info.clone()], None)[0].asset_amount;

		let mut bids_made: Vec<BidParams<TestRuntime>> = vec![];
		let starting_bid_block = inst.current_block();
		let blocks_to_bid = inst.current_block()..candle_end_block;

		// Do one candle bid for each block until the end of candle auction with a new user
		for _block in blocks_to_bid {
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::Candle));
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(bidding_account, MockInstantiator::get_ed())]);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(bidding_account, plmc_necessary_funding)]);
			inst.mint_statemint_asset_to(vec![UserToStatemintAsset::new(
				bidding_account,
				statemint_asset_necessary_funding,
				bid_info.asset.to_statemint_id(),
			)]);
			let bids: Vec<BidParams<_>> = vec![BidParams {
				bidder: bidding_account,
				amount: bid_info.amount,
				price: bid_info.price,
				multiplier: bid_info.multiplier,
				asset: bid_info.asset,
			}];
			inst.bid_for_users(project_id, bids.clone()).expect("Candle Bidding should not fail");

			bids_made.push(bids[0].clone());
			bidding_account += 1;

			inst.advance_time(1).unwrap();
		}
		let now = inst.current_block();
		inst.advance_time(candle_end_block - now + 1).unwrap();

		let random_end = inst
			.get_project_details(project_id)
			.phase_transition_points
			.random_candle_ending
			.expect("Random auction end point should exist");

		let split = (random_end - starting_bid_block + 1) as usize;
		let excluded_bids = bids_made.split_off(split);
		let included_bids = bids_made;
		let _weighted_price =
			inst.get_project_details(project_id).weighted_average_price.expect("Weighted price should exist");

		for bid in included_bids {
			let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, bid.bidder)));
			let desired_bid: BidInfoFilter<TestRuntime> = BidInfoFilter {
				project_id: Some(project_id),
				bidder: Some(bid.bidder),
				original_ct_amount: Some(bid.amount),
				original_ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Accepted),
				..Default::default()
			};

			assert!(
				inst.execute(|| stored_bids.any(|bid| desired_bid.matches_bid(&bid))),
				"Stored bid does not match the given filter"
			)
		}

		for bid in excluded_bids {
			let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, bid.bidder)));
			let desired_bid: BidInfoFilter<TestRuntime> = BidInfoFilter {
				project_id: Some(project_id),
				bidder: Some(bid.bidder),
				original_ct_amount: Some(bid.amount),
				original_ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Rejected(RejectionReason::AfterCandleEnd)),
				..Default::default()
			};
			assert!(
				inst.execute(|| stored_bids.any(|bid| desired_bid.matches_bid(&bid))),
				"Stored bid does not match the given filter"
			);
		}
	}

	#[test]
	fn pallet_can_start_auction_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn issuer_can_start_auction_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(1).unwrap();

		inst.execute(|| Pallet::<TestRuntime>::start_auction(RuntimeOrigin::signed(ISSUER), project_id)).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn stranger_cannot_start_auction_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(1).unwrap();

		for account in 6000..6010 {
			inst.execute(|| {
				let response = Pallet::<TestRuntime>::start_auction(RuntimeOrigin::signed(account), project_id);
				assert_noop!(response, Error::<TestRuntime>::NotAllowed);
			});
		}
	}

	#[test]
	fn bidder_was_evaluator() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), ISSUER);
		let evaluations = default_evaluations();
		let mut bids = default_bids();
		let evaluator = evaluations[0].account;
		bids.push(BidParams::new(evaluator, 150 * ASSET_UNIT, 21_u128.into(), 1u8, AcceptedFundingAsset::USDT));
		let _ = inst.create_community_contributing_project(project, issuer, evaluations, bids);
	}

	#[test]
	fn bids_at_higher_price_than_weighted_average_use_average() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids: Vec<BidParams<_>> = vec![
			BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 0.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 20_000 * ASSET_UNIT, 0.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_4, 20_000 * ASSET_UNIT, 0.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_5, 5_000 * ASSET_UNIT, 0.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);
		let bidder_5_bid =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, BIDDER_5)).next().unwrap());
		let wabgp = inst.get_project_details(project_id).weighted_average_price.unwrap();
		assert_eq!(bidder_5_bid.original_ct_usd_price.to_float(), 1.1);
		assert_eq!(bidder_5_bid.final_ct_usd_price, wabgp);
	}

	#[test]
	fn ct_minted_for_bids_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_bids],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_bids_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for bid in stored_bids.clone() {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for bid in stored_bids.clone() {
			inst.execute(|| {
				Pallet::<TestRuntime>::bid_ct_mint_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
				.unwrap()
			});
		}

		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_bids],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for bid in stored_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::bid_ct_mint_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
				.unwrap();

				assert_noop!(
					Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_bids.clone()],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}

		for bid in stored_bids {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			})
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let mut bids = default_bids();
		let new_bids = vec![BidParams::new(
			BIDDER_4,
			500 * US_DOLLAR,
			FixedU128::from_float(1.1),
			1u8,
			AcceptedFundingAsset::USDT,
		)];
		bids.extend(new_bids.clone());

		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let final_price = details.weighted_average_price.unwrap();
		let plmc_locked_for_bids = MockInstantiator::calculate_auction_plmc_spent(&new_bids, Some(final_price));

		for UserToPLMCBalance { account, plmc_amount } in plmc_locked_for_bids {
			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(&account, LockType::Participation(project_id))
			});

			assert_close_enough!(schedule.unwrap(), plmc_amount, Perquintill::from_parts(10_000_000_000u64));
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		for bid in stored_bids {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::start_bid_vesting_schedule_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			);

			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&bid.bidder,
					LockType::Participation(project_id),
				)
			});

			let bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, bid.bidder, bid.id)).unwrap());
			assert_eq!(schedule.unwrap(), bid.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		inst.advance_time((10 * DAYS).into()).unwrap();

		for bid in stored_bids {
			let vesting_info = bid.plmc_vesting_info.unwrap();
			let locked_amount = vesting_info.total_amount;

			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));

			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(bid.bidder, project_id, bid.bidder)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));
			assert_eq!(locked_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = vec![
			BidParams::new(BIDDER_1, 49_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(15u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for bid in stored_bids {
			let vesting_info = bid.plmc_vesting_info.unwrap();

			let now = inst.current_block();

			let blocks_vested = min(vesting_info.duration, now - vest_start_block);
			let vested_amount = vesting_info.amount_per_block * blocks_vested as u128;

			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));

			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(bid.bidder, project_id, bid.bidder)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));
			assert_eq!(vested_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn unsuccessful_bids_dont_get_vest_schedule() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids = vec![
			BidParams::new(BIDDER_1, 30000 * ASSET_UNIT, 1_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 15000 * ASSET_UNIT, 1_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let available_tokens =
			project.total_allocation_size.0.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));

		let unused_price = FixedU128::from_float(1.0);
		let rejected_bid =
			vec![BidParams::new(BIDDER_5, available_tokens, unused_price, 1u8, AcceptedFundingAsset::USDT)];
		let unused_price = FixedU128::from_float(1.1);
		let accepted_bid =
			vec![BidParams::new(BIDDER_4, available_tokens, unused_price, 1u8, AcceptedFundingAsset::USDT)];
		bids.extend(rejected_bid.clone());
		bids.extend(accepted_bid.clone());

		let community_contributions = default_community_buys();

		let project_id = inst.create_auctioning_project(project, issuer, evaluations);

		let mut bidders_plmc = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		bidders_plmc
			.iter_mut()
			.for_each(|UserToPLMCBalance { account: _, plmc_amount }| *plmc_amount += MockInstantiator::get_ed());
		inst.mint_plmc_to(bidders_plmc.clone());

		let bidders_funding_assets = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);
		inst.mint_statemint_asset_to(bidders_funding_assets);

		inst.bid_for_users(project_id, bids).unwrap();

		inst.start_community_funding(project_id).unwrap();
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let mut contributors_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		contributors_plmc
			.iter_mut()
			.for_each(|UserToPLMCBalance { account: _, plmc_amount }| *plmc_amount += MockInstantiator::get_ed());
		inst.mint_plmc_to(contributors_plmc.clone());

		let contributors_funding_assets =
			MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
		inst.mint_statemint_asset_to(contributors_funding_assets);

		inst.contribute_for_users(project_id, community_contributions).unwrap();
		inst.finish_funding(project_id).unwrap();

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let plmc_locked_for_accepted_bid =
			MockInstantiator::calculate_auction_plmc_spent(&accepted_bid, Some(final_price));
		let plmc_locked_for_rejected_bid =
			MockInstantiator::calculate_auction_plmc_spent(&rejected_bid, Some(final_price));

		let UserToPLMCBalance { account: accepted_user, plmc_amount: accepted_plmc_amount } =
			plmc_locked_for_accepted_bid[0];
		let schedule = inst.execute(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&accepted_user,
				LockType::Participation(project_id),
			)
		});
		assert_eq!(schedule.unwrap(), accepted_plmc_amount);

		let UserToPLMCBalance { account: rejected_user, .. } = plmc_locked_for_rejected_bid[0];
		let schedule_exists = inst
			.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&rejected_user,
					LockType::Participation(project_id),
				)
			})
			.is_some();
		assert!(!schedule_exists);
	}

	#[test]
	pub fn bid_funding_assets_are_paid_automatically_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta, total_expected_bid_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn bid_funding_assets_are_paid_manually_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let final_winning_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			final_bid_payouts.iter().map(|item| item.account).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta, total_expected_bid_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn bid_funding_assets_are_released_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		bids.remove(0);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();

		inst.advance_time(1).unwrap();
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
	}

	#[test]
	pub fn bid_funding_assets_are_released_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		bids.remove(0);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let final_winning_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
	}

	#[test]
	pub fn bid_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);

		let prev_bidders_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_bidders_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());

		let mut delta_bidders_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_bidders_plmc_balances, prev_bidders_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_bidders_plmc_balances.sort_by_key(|item| item.account);

		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		plmc_locked_for_bids.sort_by_key(|item| item.account);

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
	}

	#[test]
	pub fn bid_plmc_bonded_is_returned_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let final_winning_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let prev_bidders_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				),
				Pallet::<TestRuntime>::bid_unbond_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			);
		}

		let post_bidders_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());

		let mut delta_bidders_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_bidders_plmc_balances, prev_bidders_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_bidders_plmc_balances.sort_by_key(|item| item.account);

		let details = inst.get_project_details(project_id);
		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		plmc_locked_for_bids.sort_by_key(|item| item.account);

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
	}
}

mod auction_round_failure {
	use super::*;

	#[test]
	fn cannot_start_auction_before_evaluation_finishes() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		inst.execute(|| {
			assert_noop!(
				FundingModule::start_auction(RuntimeOrigin::signed(ISSUER), project_id),
				Error::<TestRuntime>::EvaluationPeriodNotEnded
			);
		});
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		inst.execute(|| {
			assert_noop!(
				FundingModule::bid(
					RuntimeOrigin::signed(BIDDER_2),
					0,
					1,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn contribute_does_not_work() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project(0, ISSUER), ISSUER);
		inst.execute(|| {
			assert_noop!(
				FundingModule::contribute(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					100,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn bids_overflow() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_auctioning_project(default_project(0, ISSUER), ISSUER, default_evaluations());
		const DAVE: AccountId = 42;
		let bids: Vec<BidParams<_>> = vec![
			BidParams::new(DAVE, 10_000 * USDT_UNIT, 2_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 20k
			BidParams::new(DAVE, 12_000 * USDT_UNIT, 8_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 96k
			BidParams::new(DAVE, 15_000 * USDT_UNIT, 5_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 75k
			// Bid with lowest PLMC bonded gets dropped
			BidParams::new(DAVE, 1_000 * USDT_UNIT, 7_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 7k
			BidParams::new(DAVE, 20_000 * USDT_UNIT, 5_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 100k
		];

		let mut plmc_fundings = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		// Existential deposit on DAVE
		plmc_fundings.push(UserToPLMCBalance::new(DAVE, MockInstantiator::get_ed()));

		let statemint_asset_fundings = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);

		// Fund enough for all PLMC bonds for the bids (multiplier of 1)
		inst.mint_plmc_to(plmc_fundings);

		// Fund enough for all bids
		inst.mint_statemint_asset_to(statemint_asset_fundings);

		inst.bid_for_users(project_id, bids).expect("Bids should pass");

		inst.execute(|| {
			let mut stored_bids = Bids::<TestRuntime>::iter_prefix_values((project_id, DAVE)).collect::<Vec<_>>();
			assert_eq!(stored_bids.len(), 4);
			stored_bids.sort();
			assert_eq!(stored_bids[0].original_ct_usd_price.to_float(), 1.0);
			assert_eq!(stored_bids[1].original_ct_usd_price.to_float(), 1.0);
			assert_eq!(stored_bids[2].original_ct_usd_price.to_float(), 1.1);
			assert_eq!(stored_bids[3].original_ct_usd_price.to_float(), 1.2);
		});
	}

	#[test]
	fn bid_with_asset_not_accepted() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_auctioning_project(default_project(0, ISSUER), ISSUER, default_evaluations());
		let bids = vec![
			BidParams::new(BIDDER_1, 10_000, 2_u128.into(), 1u8, AcceptedFundingAsset::USDC),
			BidParams::new(BIDDER_2, 13_000, 3_u128.into(), 2u8, AcceptedFundingAsset::USDC),
		];
		let outcome = inst.bid_for_users(project_id, bids);
		frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
	}

	#[test]
	fn no_bids_made() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let project_id = inst.create_auctioning_project(project, issuer, evaluations);

		let details = inst.get_project_details(project_id);
		let english_end = details.phase_transition_points.english_auction.end().unwrap();
		let now = inst.current_block();
		inst.advance_time(english_end - now + 2).unwrap();

		let details = inst.get_project_details(project_id);
		let candle_end = details.phase_transition_points.candle_auction.end().unwrap();
		let now = inst.current_block();
		inst.advance_time(candle_end - now + 2).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingFailed);
	}

	#[test]
	fn after_ct_soldout_bid_gets_refunded() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_auctioning_project(default_project(0, ISSUER), ISSUER, default_evaluations());
		let metadata = inst.get_project_metadata(project_id);
		let max_cts_for_bids = metadata.total_allocation_size.0;

		let glutton_bid_1 = BidParams::new(
			BIDDER_1,
			max_cts_for_bids - 5_000 * ASSET_UNIT,
			FixedU128::from_float(1.0),
			1u8,
			AcceptedFundingAsset::USDT,
		);
		let rejected_bid =
			BidParams::new(BIDDER_2, 5_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT);
		let glutton_bid_2 =
			BidParams::new(BIDDER_1, 5_000 * ASSET_UNIT, FixedU128::from_float(1.1), 1u8, AcceptedFundingAsset::USDT);
		let bids = inst.simulate_bids_with_bucket(
			vec![glutton_bid_1.clone(), rejected_bid.clone(), glutton_bid_2.clone()],
			project_id,
		);
		let mut plmc_fundings = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		plmc_fundings.push(UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()));
		plmc_fundings.push(UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()));
		let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_spent(&bids, None);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(usdt_fundings.clone());

		inst.bid_for_users(project_id, vec![glutton_bid_1, rejected_bid, glutton_bid_2]).expect("Bids should pass");

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
			UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
		]);
		inst.do_reserved_plmc_assertions(
			vec![
				UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount + plmc_fundings[2].plmc_amount),
				UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount),
			],
			LockType::Participation(project_id),
		);
		inst.do_bid_transferred_statemint_asset_assertions(
			vec![
				UserToStatemintAsset::<TestRuntime>::new(
					BIDDER_1,
					usdt_fundings[0].asset_amount + usdt_fundings[2].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
				UserToStatemintAsset::<TestRuntime>::new(
					BIDDER_2,
					usdt_fundings[1].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
			],
			project_id,
		);

		inst.start_community_funding(project_id).unwrap();

		let weighted_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let plmc_fundings_after_round = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(weighted_price));
		let usdt_fundings_after_round =
			MockInstantiator::calculate_auction_funding_asset_spent(&bids, Some(weighted_price));

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(
				BIDDER_1,
				MockInstantiator::get_ed() + (plmc_fundings[2].plmc_amount - plmc_fundings_after_round[2].plmc_amount),
			),
			UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount + MockInstantiator::get_ed()),
		]);

		inst.do_reserved_plmc_assertions(
			vec![
				UserToPLMCBalance::new(
					BIDDER_1,
					plmc_fundings_after_round[0].plmc_amount + plmc_fundings_after_round[2].plmc_amount,
				),
				UserToPLMCBalance::new(BIDDER_2, 0),
			],
			LockType::Participation(project_id),
		);

		inst.do_bid_transferred_statemint_asset_assertions(
			vec![
				UserToStatemintAsset::new(
					BIDDER_1,
					usdt_fundings_after_round[0].asset_amount + usdt_fundings_after_round[2].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
				UserToStatemintAsset::new(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);
	}

	#[test]
	fn after_random_end_bid_gets_refunded() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_auctioning_project(default_project(0, ISSUER), ISSUER, default_evaluations());

		let (bid_in, bid_out) = (default_bids()[0].clone(), default_bids()[1].clone());

		let mut plmc_fundings =
			MockInstantiator::calculate_auction_plmc_spent(&vec![bid_in.clone(), bid_out.clone()], None);
		plmc_fundings.push(UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()));
		plmc_fundings.push(UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()));

		let usdt_fundings =
			MockInstantiator::calculate_auction_funding_asset_spent(&vec![bid_in.clone(), bid_out.clone()], None);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(usdt_fundings.clone());

		inst.bid_for_users(project_id, vec![bid_in]).expect("Bids should pass");
		inst.advance_time(
			<TestRuntime as Config>::EnglishAuctionDuration::get() +
				<TestRuntime as Config>::CandleAuctionDuration::get() -
				1,
		)
		.unwrap();

		inst.bid_for_users(project_id, vec![bid_out]).expect("Bids should pass");

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
			UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
		]);
		inst.do_reserved_plmc_assertions(
			vec![
				UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount),
				UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount),
			],
			LockType::Participation(project_id),
		);
		inst.do_bid_transferred_statemint_asset_assertions(
			vec![
				UserToStatemintAsset::<TestRuntime>::new(
					BIDDER_1,
					usdt_fundings[0].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
				UserToStatemintAsset::<TestRuntime>::new(
					BIDDER_2,
					usdt_fundings[1].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
			],
			project_id,
		);
		inst.start_community_funding(project_id).unwrap();
		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
			UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount + MockInstantiator::get_ed()),
		]);

		inst.do_reserved_plmc_assertions(
			vec![UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount), UserToPLMCBalance::new(BIDDER_2, 0)],
			LockType::Participation(project_id),
		);

		inst.do_bid_transferred_statemint_asset_assertions(
			vec![
				UserToStatemintAsset::<TestRuntime>::new(
					BIDDER_1,
					usdt_fundings[0].asset_amount,
					AcceptedFundingAsset::USDT.to_statemint_id(),
				),
				UserToStatemintAsset::<TestRuntime>::new(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);
	}
}

mod community_round_success {
	use super::*;

	pub const HOURS: BlockNumber = 300u64;

	#[test]
	fn community_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_remainder_contributing_project(
			default_project(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
	}

	#[test]
	fn multiple_contribution_projects_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project1 = default_project(inst.get_new_nonce(), ISSUER);
		let project2 = default_project(inst.get_new_nonce(), ISSUER);
		let project3 = default_project(inst.get_new_nonce(), ISSUER);
		let project4 = default_project(inst.get_new_nonce(), ISSUER);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_buys = default_community_buys();

		inst.create_remainder_contributing_project(
			project1,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		inst.create_remainder_contributing_project(
			project2,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		inst.create_remainder_contributing_project(
			project3,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		inst.create_remainder_contributing_project(project4, issuer, evaluations, bids, community_buys);
	}

	#[test]
	fn contribute_multiple_times_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let metadata = default_project(0, ISSUER);
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let (project_id, _) = inst.create_community_contributing_project(metadata, issuer, evaluations, bids);

		const BOB: AccountId = 42;
		let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributions = vec![
			ContributionParams::new(BOB, 3 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BOB, 4 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let mut plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
		plmc_funding.push(UserToPLMCBalance::new(BOB, MockInstantiator::get_ed()));
		let statemint_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		inst.mint_plmc_to(plmc_funding);
		inst.mint_statemint_asset_to(statemint_funding);

		inst.contribute_for_users(project_id, vec![contributions[0].clone()])
			.expect("The Buyer should be able to buy multiple times");
		inst.advance_time(HOURS as BlockNumber).unwrap();

		inst.contribute_for_users(project_id, vec![contributions[1].clone()])
			.expect("The Buyer should be able to buy multiple times");

		let bob_total_contributions: BalanceOf<TestRuntime> = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, BOB)).map(|c| c.funding_asset_amount).sum()
		});

		let total_contributed = MockInstantiator::calculate_contributed_funding_asset_spent(contributions, token_price)
			.iter()
			.map(|item| item.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		assert_eq!(bob_total_contributions, total_contributed);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_exact() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let bids = vec![
			BidParams::from(BIDDER_1, 40_000 * ASSET_UNIT, FixedU128::from_float(1.0)),
			BidParams::from(BIDDER_2, 10_000 * ASSET_UNIT, FixedU128::from_float(1.0)),
		];
		let (project_id, _) =
			inst.create_community_contributing_project(default_project(0, ISSUER), ISSUER, default_evaluations(), bids);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct.1, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push(UserToPLMCBalance::new(BOB, MockInstantiator::get_ed()));
		let statemint_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();
		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens.1,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		inst.do_free_plmc_assertions(vec![plmc_fundings[1].clone()]);
		inst.do_free_statemint_asset_assertions(vec![UserToStatemintAsset::<TestRuntime>::new(
			BOB,
			0_u128,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		inst.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], LockType::Participation(project_id));
		inst.do_contribution_transferred_statemint_asset_assertions(statemint_asset_fundings, project_id);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_overbuy() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let bids = vec![
			BidParams::new(BIDDER_1, 40_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 10_000 * ASSET_UNIT, FixedU128::from_float(1.0), 1u8, AcceptedFundingAsset::USDT),
		];
		let (project_id, _) =
			inst.create_community_contributing_project(default_project(0, ISSUER), ISSUER, default_evaluations(), bids);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;

		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct.1, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push(UserToPLMCBalance::new(BOB, MockInstantiator::get_ed()));
		let mut statemint_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens.1,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).plmc_amount;
		let remaining_plmc: BalanceOf<TestRuntime> =
			plmc_fundings.iter().fold(0_u128, |acc, item| acc + item.plmc_amount);

		let actual_funding_transferred = statemint_asset_fundings.swap_remove(0).asset_amount;
		let remaining_statemint_assets: BalanceOf<TestRuntime> =
			statemint_asset_fundings.iter().fold(0_u128, |acc, item| acc + item.asset_amount);

		inst.do_free_plmc_assertions(vec![UserToPLMCBalance::new(BOB, remaining_plmc)]);
		inst.do_free_statemint_asset_assertions(vec![UserToStatemintAsset::<TestRuntime>::new(
			BOB,
			remaining_statemint_assets,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		inst.do_reserved_plmc_assertions(
			vec![UserToPLMCBalance::new(BOB, reserved_plmc)],
			LockType::Participation(project_id),
		);
		inst.do_contribution_transferred_statemint_asset_assertions(
			vec![UserToStatemintAsset::<TestRuntime>::new(
				BOB,
				actual_funding_transferred,
				AcceptedFundingAsset::USDT.to_statemint_id(),
			)],
			project_id,
		);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_same_mult_diff_ct() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, _) = inst.create_community_contributing_project(
			default_project(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

		let project_details = inst.get_project_details(project_id);
		let token_price = project_details.weighted_average_price.unwrap();

		// Create a contribution vector that will reach the limit of contributions for a user-project
		let token_amount: BalanceOf<TestRuntime> = ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: Vec<ContributionParams<_>> = range
			.map(|_| ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT))
			.collect();

		let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
		let ed_funding = vec![UserToPLMCBalance::new(CONTRIBUTOR, MockInstantiator::get_ed())];
		let statemint_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		inst.mint_plmc_to(plmc_funding.clone());
		inst.mint_plmc_to(ed_funding);
		inst.mint_statemint_asset_to(statemint_funding.clone());

		// Reach the limit of contributions for a user-project
		inst.contribute_for_users(project_id, contributions).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let contributor_post_buy_plmc_balance =
			inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&CONTRIBUTOR));
		let contributor_post_buy_statemint_asset_balance =
			inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, MockInstantiator::get_ed());
		assert_eq!(contributor_post_buy_statemint_asset_balance, 0);

		let plmc_bond_stored = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, MockInstantiator::sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(
			statemint_asset_contributions_stored,
			MockInstantiator::sum_statemint_mappings(vec![statemint_funding.clone()])
		);

		let new_token_amount: BalanceOf<TestRuntime> = 2 * ASSET_UNIT;
		let new_contribution =
			vec![ContributionParams::new(CONTRIBUTOR, new_token_amount, 1u8, AcceptedFundingAsset::USDT)];

		let new_plmc_funding =
			MockInstantiator::calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		inst.mint_plmc_to(new_plmc_funding.clone());
		inst.mint_statemint_asset_to(new_statemint_funding.clone());

		inst.contribute_for_users(project_id, new_contribution).unwrap();

		let contributor_post_return_plmc_balance =
			inst.execute(|| <TestRuntime as Config>::NativeCurrency::free_balance(CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, CONTRIBUTOR));

		assert_eq!(
			contributor_post_return_plmc_balance,
			contributor_post_buy_plmc_balance + plmc_funding[0].plmc_amount
		);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding[0].asset_amount
		);

		let new_plmc_bond_stored = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + MockInstantiator::sum_balance_mappings(vec![new_plmc_funding]) -
				plmc_funding[0].plmc_amount
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored +
				MockInstantiator::sum_statemint_mappings(vec![new_statemint_funding]) -
				statemint_funding[0].asset_amount
		);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_diff_mult_same_ct() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, _) = inst.create_community_contributing_project(
			default_project(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

		let project_details = inst.get_project_details(project_id);
		let token_price = project_details.weighted_average_price.unwrap();

		// Create a contribution vector that will reach the limit of contributions for a user-project
		let token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: Vec<ContributionParams<_>> = range
			.map(|_| ContributionParams::new(CONTRIBUTOR, token_amount, 3u8, AcceptedFundingAsset::USDT))
			.collect();

		let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
		let ed_funding = vec![UserToPLMCBalance::new(CONTRIBUTOR, MockInstantiator::get_ed())];
		let statemint_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		inst.mint_plmc_to(plmc_funding.clone());
		inst.mint_plmc_to(ed_funding);
		inst.mint_statemint_asset_to(statemint_funding.clone());

		// Reach the limit of contributions for a user-project
		inst.contribute_for_users(project_id, contributions).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let contributor_post_buy_plmc_balance =
			inst.execute(|| <TestRuntime as Config>::NativeCurrency::free_balance(CONTRIBUTOR));
		let contributor_post_buy_statemint_asset_balance =
			inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, MockInstantiator::get_ed());
		assert_eq!(contributor_post_buy_statemint_asset_balance, 0);

		let plmc_bond_stored = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, MockInstantiator::sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(
			statemint_asset_contributions_stored,
			MockInstantiator::sum_statemint_mappings(vec![statemint_funding.clone()])
		);

		let new_token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let new_contribution =
			vec![ContributionParams::new(CONTRIBUTOR, new_token_amount, 1u8, AcceptedFundingAsset::USDT)];

		let new_plmc_funding =
			MockInstantiator::calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		inst.mint_plmc_to(new_plmc_funding.clone());
		inst.mint_statemint_asset_to(new_statemint_funding.clone());

		inst.contribute_for_users(project_id, new_contribution).unwrap();

		let contributor_post_return_plmc_balance =
			inst.execute(|| <TestRuntime as Config>::NativeCurrency::free_balance(CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, CONTRIBUTOR));

		assert_eq!(
			contributor_post_return_plmc_balance,
			contributor_post_buy_plmc_balance + plmc_funding[0].plmc_amount
		);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding[0].asset_amount
		);

		let new_plmc_bond_stored = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + MockInstantiator::sum_balance_mappings(vec![new_plmc_funding]) -
				plmc_funding[0].plmc_amount
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored +
				MockInstantiator::sum_statemint_mappings(vec![new_statemint_funding]) -
				statemint_funding[0].asset_amount
		);
	}

	#[test]
	fn retail_contributor_was_evaluator() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);
		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let already_bonded_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
			evaluator_contributor,
			evaluation_amount,
		)])[0]
			.plmc_amount;
		let plmc_available_for_participating =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_contribution =
			MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;
		let necessary_usdt_for_contribution =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			necessary_plmc_for_contribution - plmc_available_for_participating,
		)]);
		inst.mint_statemint_asset_to(necessary_usdt_for_contribution);

		inst.contribute_for_users(project_id, vec![contribution]).unwrap();
	}

	#[test]
	fn retail_contributor_was_evaluator_vec_full() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let bids = default_bids();
		let evaluator_contributor = 69;
		let overflow_contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let mut fill_contributions = Vec::new();
		for _i in 0..<TestRuntime as Config>::MaxContributionsPerUser::get() {
			fill_contributions.push(ContributionParams::new(
				evaluator_contributor,
				10 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			));
		}

		let expected_price = MockInstantiator::calculate_price_from_test_bids(bids.clone());
		let fill_necessary_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(fill_contributions.clone(), expected_price);
		let fill_necessary_usdt =
			MockInstantiator::calculate_contributed_funding_asset_spent(fill_contributions.clone(), expected_price);

		let overflow_necessary_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(vec![overflow_contribution.clone()], expected_price);
		let overflow_necessary_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![overflow_contribution.clone()],
			expected_price,
		);

		let evaluation_bond =
			MockInstantiator::sum_balance_mappings(vec![fill_necessary_plmc, overflow_necessary_plmc]);
		let plmc_available_for_participating =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_usd_amount));

		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			evaluation_bond - plmc_available_for_participating,
		)]);
		inst.mint_statemint_asset_to(fill_necessary_usdt);
		inst.mint_statemint_asset_to(overflow_necessary_usdt);

		inst.contribute_for_users(project_id, fill_contributions).unwrap();
		inst.contribute_for_users(project_id, vec![overflow_contribution]).unwrap();

		let evaluation_bonded = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_contributor,
			)
		});
		assert_eq!(evaluation_bonded, <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond);
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			ContributionParams::new(evaluator_contributor, 396 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);
		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let necessary_plmc_for_contribution =
			MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;
		let plmc_evaluation_amount = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
			evaluator_contributor,
			evaluation_amount,
		)])[0]
			.plmc_amount;
		let plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;
		assert!(
			necessary_plmc_for_contribution > plmc_available_for_participating &&
				necessary_plmc_for_contribution < plmc_evaluation_amount
		);
		println!(
			"Plmc contr: {:?}, plmc eval: {:?}, plmc avail: {:?}",
			necessary_plmc_for_contribution, plmc_evaluation_amount, plmc_available_for_participating
		);
		let necessary_usdt_for_contribution =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

		inst.mint_statemint_asset_to(necessary_usdt_for_contribution);

		assert_matches!(inst.contribute_for_users(project_id, vec![contribution]), Err(_));
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_success() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			ContributionParams::new(evaluator_contributor, 396 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);

		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let necessary_plmc_for_contribution =
			MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;
		let plmc_evaluation_amount = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
			evaluator_contributor,
			evaluation_amount,
		)])[0]
			.plmc_amount;
		let plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;
		assert!(
			necessary_plmc_for_contribution > plmc_available_for_participating &&
				necessary_plmc_for_contribution < plmc_evaluation_amount
		);
		let necessary_usdt_for_contribution =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			necessary_plmc_for_contribution - plmc_available_for_participating,
		)]);
		inst.mint_statemint_asset_to(necessary_usdt_for_contribution);

		inst.contribute_for_users(project_id, vec![contribution]).unwrap();
		let evaluation_locked = inst
			.get_reserved_plmc_balances_for(vec![evaluator_contributor], LockType::Evaluation(project_id))[0]
			.plmc_amount;
		let participation_locked = inst
			.get_reserved_plmc_balances_for(vec![evaluator_contributor], LockType::Participation(project_id))[0]
			.plmc_amount;

		assert_eq!(evaluation_locked, <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount);
		assert_eq!(participation_locked, necessary_plmc_for_contribution);
	}

	#[test]
	fn ct_minted_for_community_buys_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_community_buys =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_community_buys.len(), community_contributions.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_community_buys],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_community_buys_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for contribution in stored_contributions.clone() {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions.clone() {
			inst.execute(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap()
			});
		}

		assert_eq!(stored_contributions.len(), community_contributions.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_contributions],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap();

				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_contributions.len(), community_contributions.len());
		let user_ct_amounts = MockInstantiator::generic_map_merge_reduce(
			vec![stored_contributions.clone()],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}

		for contribution in stored_contributions {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			})
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contribution_locked_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions, price);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		for UserToPLMCBalance { account: user, plmc_amount: amount } in contribution_locked_plmc {
			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(&user, LockType::Participation(project_id))
			});

			assert_eq!(schedule.unwrap(), amount);
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		for contribution in contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::start_contribution_vesting_schedule_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			);

			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&contribution.contributor,
					LockType::Participation(project_id),
				)
			});

			let contribution = inst.execute(|| {
				Contributions::<TestRuntime>::get((project_id, contribution.contributor, contribution.id)).unwrap()
			});
			assert_eq!(schedule.unwrap(), contribution.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		inst.advance_time((10 * DAYS).into()).unwrap();

		for contribution in stored_contributions {
			let vesting_info = contribution.plmc_vesting_info.unwrap();
			let locked_amount = vesting_info.total_amount;

			let prev_free_balance =
				inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));

			inst.execute(|| {
				Pallet::<TestRuntime>::do_vest_plmc_for(contribution.contributor, project_id, contribution.contributor)
			})
			.unwrap();

			let post_free_balance =
				inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));
			assert_eq!(locked_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = vec![
			BidParams::new(BIDDER_1, 49_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(15u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for contribution in stored_contributions {
			let vesting_info = contribution.plmc_vesting_info.unwrap();

			let now = inst.current_block();
			let blocks_vested = min(vesting_info.duration, now - vest_start_block);
			let vested_amount = vesting_info.amount_per_block * blocks_vested as u128;

			let prev_free_balance =
				inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));

			inst.execute(|| {
				Pallet::<TestRuntime>::do_vest_plmc_for(contribution.contributor, project_id, contribution.contributor)
			})
			.unwrap();

			let post_free_balance =
				inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));
			assert_eq!(vested_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::<TestRuntime>::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].asset_id,
			final_contribution_payouts.iter().map(|item| item.account).collect::<Vec<_>>(),
		);

		let prev_total_contributor_balance =
			prev_contributors_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].asset_id,
			final_contribution_payouts.iter().map(|item| item.account).collect::<Vec<_>>(),
		);

		let post_total_contributor_balance =
			post_contributors_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_contributor_balance, 0u128);
		assert_eq!(post_total_contributor_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_winning_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let final_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::<TestRuntime>::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].asset_id,
			final_contribution_payouts.iter().map(|item| item.account).collect::<Vec<_>>(),
		);

		let prev_total_contributor_balance =
			prev_contributors_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}
		for contribution in final_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			})
			.unwrap();
		}
		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].asset_id,
			final_contribution_payouts.iter().map(|item| item.account).collect::<Vec<_>>(),
		);

		let post_total_contributor_balance =
			post_contributors_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_contributor_balance, 0u128);
		assert_eq!(post_total_contributor_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}
}

mod community_round_failure {
	use super::*;

	#[test]
	pub fn bid_and_community_contribution_funding_assets_are_released_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 2,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let expected_community_contribution_payouts = MockInstantiator::calculate_contributed_funding_asset_spent(
			community_contributions.clone(),
			inst.get_project_details(project_id).weighted_average_price.unwrap(),
		);

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let prev_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			community_contributions.iter().map(|test_contribution| test_contribution.contributor).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			community_contributions.iter().map(|test_contribution| test_contribution.contributor).collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let contributors_funding_delta = MockInstantiator::generic_map_operation(
			vec![post_contributors_funding_balances, prev_contributors_funding_balances],
			MergeOperation::Subtract,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
		assert_eq!(contributors_funding_delta, expected_community_contribution_payouts)
	}

	#[test]
	pub fn bid_and_community_contribution_funding_assets_are_released_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 2,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let final_winning_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToStatemintAsset<_>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let expected_community_contribution_payouts = MockInstantiator::calculate_contributed_funding_asset_spent(
			community_contributions.clone(),
			inst.get_project_details(project_id).weighted_average_price.unwrap(),
		);

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let prev_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let prev_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			community_contributions.iter().map(|test_contribution| test_contribution.contributor).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}

		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		for contribution in stored_contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_bidders_funding_balances =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, final_bid_payouts.accounts());
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|item| item.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let post_contributors_funding_balances = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			community_contributions.iter().map(|test_contribution| test_contribution.contributor).collect::<Vec<_>>(),
		);
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let contributors_funding_delta = MockInstantiator::generic_map_operation(
			vec![post_contributors_funding_balances, prev_contributors_funding_balances],
			MergeOperation::Subtract,
		);
		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
		assert_eq!(contributors_funding_delta, expected_community_contribution_payouts)
	}

	#[test]
	pub fn bid_and_community_contribution_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 2,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions,
		);

		let prev_bidders_plmc_balances =
			inst.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder).collect::<Vec<_>>());
		let prev_contributors_plmc_balances = inst.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor).collect::<Vec<_>>(),
		);

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);
		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_bidders_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());
		let post_contributors_plmc_balances = inst.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor).collect::<Vec<_>>(),
		);

		let mut delta_bidders_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_bidders_plmc_balances, prev_bidders_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_bidders_plmc_balances.sort_by_key(|item| item.account);

		let mut delta_contributors_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_contributors_plmc_balances, prev_contributors_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_contributors_plmc_balances.sort_by_key(|item| item.account);

		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		plmc_locked_for_bids.sort_by_key(|item| item.account);
		let mut plmc_locked_for_contributions =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions, final_price);
		plmc_locked_for_contributions.sort_by_key(|item| item.account);

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
		assert_eq!(delta_contributors_plmc_balances, plmc_locked_for_contributions);
	}

	#[test]
	pub fn bid_and_community_contribution_plmc_bonded_is_returned_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions,
		);
		let final_winning_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let prev_bidders_plmc_balances =
			inst.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder).collect::<Vec<_>>());
		let prev_contributors_plmc_balances = inst.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor).collect::<Vec<_>>(),
		);
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				),
				Pallet::<TestRuntime>::bid_unbond_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			);
		}

		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		for contribution in stored_contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				),
				Pallet::<TestRuntime>::contribution_unbond_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_bidders_plmc_balances =
			inst.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder).collect::<Vec<_>>());
		let post_contributors_plmc_balances = inst.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor).collect::<Vec<_>>(),
		);

		let mut delta_bidders_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_bidders_plmc_balances, prev_bidders_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_bidders_plmc_balances.sort_by_key(|item| item.account);
		let mut delta_contributors_plmc_balances = MockInstantiator::generic_map_operation(
			vec![post_contributors_plmc_balances, prev_contributors_plmc_balances],
			MergeOperation::Subtract,
		);
		delta_contributors_plmc_balances.sort_by_key(|item| item.account);

		let details = inst.get_project_details(project_id);
		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		plmc_locked_for_bids.sort_by_key(|item| item.account);
		let mut plmc_locked_for_contributions =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions, final_price);
		plmc_locked_for_contributions.sort_by_key(|item| item.account);

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
		assert_eq!(delta_contributors_plmc_balances, plmc_locked_for_contributions);
	}
}

mod remainder_round_success {
	use super::*;

	#[test]
	fn remainder_round_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_finished_project(
			default_project(inst.get_new_nonce(), ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			default_remainder_buys(),
		);
	}

	#[test]
	fn remainder_contributor_was_evaluator() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let community_contributions = default_community_buys();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let remainder_contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let (project_id, _) =
			inst.create_remainder_contributing_project(project, issuer, evaluations, bids, community_contributions);
		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let already_bonded_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
			evaluator_contributor,
			evaluation_amount,
		)])[0]
			.plmc_amount;
		let plmc_available_for_contribution =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_buy =
			MockInstantiator::calculate_contributed_plmc_spent(vec![remainder_contribution.clone()], ct_price)[0]
				.plmc_amount;
		let necessary_usdt_for_buy =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![remainder_contribution.clone()], ct_price);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			necessary_plmc_for_buy - plmc_available_for_contribution,
		)]);
		inst.mint_statemint_asset_to(necessary_usdt_for_buy);

		inst.contribute_for_users(project_id, vec![remainder_contribution]).unwrap();
	}

	#[test]
	fn remainder_contributor_was_evaluator_vec_full() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let bids = default_bids();
		let evaluator_contributor = 69;
		let overflow_contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let mut fill_contributions = Vec::new();
		for _i in 0..<TestRuntime as Config>::MaxContributionsPerUser::get() {
			fill_contributions.push(ContributionParams::new(
				evaluator_contributor,
				10 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			));
		}

		let expected_price = MockInstantiator::calculate_price_from_test_bids(bids.clone());
		let fill_necessary_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(fill_contributions.clone(), expected_price);
		let fill_necessary_usdt_for_bids =
			MockInstantiator::calculate_contributed_funding_asset_spent(fill_contributions.clone(), expected_price);

		let overflow_necessary_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(vec![overflow_contribution.clone()], expected_price);
		let overflow_necessary_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![overflow_contribution.clone()],
			expected_price,
		);

		let evaluation_bond =
			MockInstantiator::sum_balance_mappings(vec![fill_necessary_plmc, overflow_necessary_plmc]);
		let plmc_available_for_participating =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_usd_amount));

		let (project_id, _) =
			inst.create_remainder_contributing_project(project, issuer, evaluations, bids, default_community_buys());

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			evaluation_bond - plmc_available_for_participating,
		)]);
		inst.mint_statemint_asset_to(fill_necessary_usdt_for_bids);
		inst.mint_statemint_asset_to(overflow_necessary_usdt);

		inst.contribute_for_users(project_id, fill_contributions).unwrap();
		inst.contribute_for_users(project_id, vec![overflow_contribution]).unwrap();

		let evaluation_bonded = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_contributor,
			)
		});
		assert_eq!(evaluation_bonded, <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond);
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_exact() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, _) = inst.create_remainder_contributing_project(
			default_project(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions =
			vec![ContributionParams::new(BOB, remaining_ct.0 + remaining_ct.1, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push(UserToPLMCBalance::new(BOB, MockInstantiator::get_ed()));
		let statemint_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens.0 +
				inst.get_project_details(project_id).remaining_contribution_tokens.1,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		inst.do_free_plmc_assertions(vec![plmc_fundings[1].clone()]);
		inst.do_free_statemint_asset_assertions(vec![UserToStatemintAsset::<TestRuntime>::new(
			BOB,
			0_u128,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		inst.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], LockType::Participation(project_id));
		inst.do_contribution_transferred_statemint_asset_assertions(statemint_asset_fundings, project_id);
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_overbuy() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, _) = inst.create_remainder_contributing_project(
			default_project(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens.0 +
			inst.get_project_details(project_id).remaining_contribution_tokens.1;

		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push(UserToPLMCBalance::new(BOB, MockInstantiator::get_ed()));
		let mut statemint_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens.0 +
				inst.get_project_details(project_id).remaining_contribution_tokens.1,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).plmc_amount;
		let remaining_plmc: BalanceOf<TestRuntime> =
			plmc_fundings.iter().fold(Zero::zero(), |acc, item| item.plmc_amount + acc);

		let actual_funding_transferred = statemint_asset_fundings.swap_remove(0).asset_amount;
		let remaining_statemint_assets: BalanceOf<TestRuntime> =
			statemint_asset_fundings.iter().fold(Zero::zero(), |acc, item| item.asset_amount + acc);

		inst.do_free_plmc_assertions(vec![UserToPLMCBalance::new(BOB, remaining_plmc)]);
		inst.do_free_statemint_asset_assertions(vec![UserToStatemintAsset::<TestRuntime>::new(
			BOB,
			remaining_statemint_assets,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		inst.do_reserved_plmc_assertions(
			vec![UserToPLMCBalance::new(BOB, reserved_plmc)],
			LockType::Participation(project_id),
		);
		inst.do_contribution_transferred_statemint_asset_assertions(
			vec![UserToStatemintAsset::new(
				BOB,
				actual_funding_transferred,
				AcceptedFundingAsset::USDT.to_statemint_id(),
			)],
			project_id,
		);
	}

	#[test]
	fn ct_minted_for_remainder_buys_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&mut inst,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 40_000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_community_buys_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = vec![
			UserToUSDBalance::new(EVALUATOR_1, 50_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_2, 25_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_3, 32_000 * PLMC),
		];
		let bids = vec![BidParams::new(BIDDER_1, 50000 * ASSET_UNIT, 1_u128.into(), 1u8, AcceptedFundingAsset::USDT)];
		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			ContributionParams::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = inst.execute(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution, buyer_contribution, bidder_contribution]
		});
		for contribution in stored_contributions.clone() {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap()
			});
		}

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&mut inst,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 50000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = inst.execute(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution, buyer_contribution, bidder_contribution]
		});
		for contribution in stored_contributions.clone() {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap();

				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&mut inst,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 40000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = inst.execute(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution, buyer_contribution, bidder_contribution]
		});
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&mut inst,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 40000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}

		for contribution in stored_contributions {
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
		bids = stored_bids
			.into_iter()
			.map(|bid| BidParams::from(bid.bidder, bid.final_ct_amount, bid.final_ct_usd_price))
			.collect();
		let auction_locked_plmc = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(price));
		let community_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(community_contributions, price);
		let remainder_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(remainder_contributions, price);
		let all_plmc_locks = MockInstantiator::generic_map_operation(
			vec![auction_locked_plmc, community_locked_plmc, remainder_locked_plmc],
			MergeOperation::Add,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		for UserToPLMCBalance { account, plmc_amount } in all_plmc_locks {
			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(&account, LockType::Participation(project_id))
			});

			assert_eq!(schedule.unwrap(), plmc_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		for contribution in contributions {
			let prev_scheduled = inst
				.execute(|| {
					<TestRuntime as Config>::Vesting::total_scheduled_amount(
						&contribution.contributor,
						LockType::Participation(project_id),
					)
				})
				.unwrap_or(Zero::zero());

			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::start_contribution_vesting_schedule_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			);

			let post_scheduled = inst
				.execute(|| {
					<TestRuntime as Config>::Vesting::total_scheduled_amount(
						&contribution.contributor,
						LockType::Participation(project_id),
					)
				})
				.unwrap();

			let new_scheduled = post_scheduled - prev_scheduled;

			let contribution = inst.execute(|| {
				Contributions::<TestRuntime>::get((project_id, contribution.contributor, contribution.id)).unwrap()
			});
			assert_eq!(new_scheduled, contribution.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		let bid_plmc_balances =
			stored_bids.into_iter().map(|b| (b.bidder, b.plmc_vesting_info.unwrap().total_amount)).collect::<Vec<_>>();
		let contributed_plmc_balances = stored_contributions
			.into_iter()
			.map(|c| (c.contributor, c.plmc_vesting_info.unwrap().total_amount))
			.collect::<Vec<_>>();

		let merged_plmc_balances = MockInstantiator::generic_map_merge_reduce(
			vec![contributed_plmc_balances, bid_plmc_balances],
			|(account, _amount)| *account,
			BalanceOf::<TestRuntime>::zero(),
			|(_account, amount), total| total + amount,
		);
		inst.advance_time((DAYS + 1u32).into()).unwrap();

		for (contributor, plmc_amount) in merged_plmc_balances {
			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(contributor, project_id, contributor)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			assert_eq!(plmc_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(15u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();

		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		let now = inst.current_block();

		let bid_plmc_balances = stored_bids
			.into_iter()
			.map(|b| {
				(b.bidder, {
					let blocks_vested = min(b.plmc_vesting_info.unwrap().duration, now - vest_start_block);
					b.plmc_vesting_info.unwrap().amount_per_block * blocks_vested as u128
				})
			})
			.collect::<Vec<_>>();
		let contributed_plmc_balances = stored_contributions
			.into_iter()
			.map(|c| {
				(c.contributor, {
					let blocks_vested = min(c.plmc_vesting_info.unwrap().duration, now - vest_start_block);
					c.plmc_vesting_info.unwrap().amount_per_block * blocks_vested as u128
				})
			})
			.collect::<Vec<_>>();

		let merged_plmc_balances = MockInstantiator::generic_map_merge_reduce(
			vec![contributed_plmc_balances, bid_plmc_balances],
			|(account, _amount)| *account,
			BalanceOf::<TestRuntime>::zero(),
			|(_account, amount), total| total + amount,
		);

		for (contributor, amount) in merged_plmc_balances {
			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));

			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(contributor, project_id, contributor)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			assert_eq!(amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn remainder_contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = vec![];
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn community_contribution_remainder_contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn remainder_contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = vec![];
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_winning_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let final_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}
		for contribution in final_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			})
			.unwrap();
		}
		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn remainder_contribution_community_contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);

		let final_winning_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| {
					UserToStatemintAsset::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let final_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToStatemintAsset::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				)
			})
			.unwrap();
		}
		for contribution in final_contributions {
			inst.execute(|| {
				Pallet::<TestRuntime>::payout_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			})
			.unwrap();
		}
		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;
		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}
}

mod remainder_round_failure {
	use super::*;

	#[test]
	pub fn bids_and_community_and_remainder_contribution_funding_assets_are_released_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 4,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let expected_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|bid| bid.account)
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions, final_price);
		let expected_remainder_contribution_payouts =
			MockInstantiator::calculate_contributed_funding_asset_spent(remainder_contributions, final_price);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			MergeOperation::Add,
		);

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let all_participants = all_expected_payouts.accounts();
		let prev_participants_funding_balances =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_participants_funding_balances =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			expected_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let all_participants_funding_delta = MockInstantiator::generic_map_operation(
			vec![prev_participants_funding_balances, post_participants_funding_balances],
			MergeOperation::Add,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(all_expected_payouts, all_participants_funding_delta);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_funding_assets_are_released_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 4,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let expected_bid_payouts = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|bid| {
					UserToStatemintAsset::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_statemint_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToStatemintAsset<TestRuntime>>>()
		});
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions, final_price);
		let expected_remainder_contribution_payouts =
			MockInstantiator::calculate_contributed_funding_asset_spent(remainder_contributions, final_price);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			MergeOperation::Add,
		);

		let prev_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let all_participants = all_expected_payouts.accounts();
		let prev_participants_funding_balances =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let stored_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for bid in stored_bids {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				)
			)
		}

		for contribution in stored_contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_issuer_funding_balance =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_participants_funding_balances =
			inst.get_free_statemint_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
		let post_project_pot_funding_balance = inst.get_free_statemint_asset_balances_for(
			expected_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		let all_participants_funding_delta = MockInstantiator::generic_map_operation(
			vec![prev_participants_funding_balances, post_participants_funding_balances],
			MergeOperation::Add,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(all_expected_payouts, all_participants_funding_delta);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = vec![UserToUSDBalance::new(EVALUATOR_1, 50_000 * US_DOLLAR)];
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 5,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let expected_evaluator_contributor_return =
			MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				vec![UserToUSDBalance::new(EVALUATOR_1, 50_000 * US_DOLLAR)],
				vec![ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT)],
				final_price,
				true,
			);
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
		let bids = stored_bids
			.into_iter()
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.map(|bid| BidParams::from(bid.bidder, bid.final_ct_amount, bid.final_ct_usd_price))
			.collect_vec();
		let expected_bid_payouts = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions, final_price);
		let expected_remainder_contribution_payouts = MockInstantiator::calculate_contributed_plmc_spent(
			vec![
				ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
				ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			],
			final_price,
		);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_evaluator_contributor_return,
				expected_bid_payouts,
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			MergeOperation::Add,
		);

		let prev_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;

		let all_participants = all_expected_payouts.accounts();
		let prev_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants.clone());

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
		let post_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants);

		let all_participants_plmc_deltas = MockInstantiator::generic_map_operation(
			vec![post_participants_plmc_balances, prev_participants_plmc_balances],
			MergeOperation::Subtract,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_expected_payouts, all_participants_plmc_deltas);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_plmc_bonded_is_returned_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = vec![
			UserToUSDBalance::new(EVALUATOR_1, 50_000 * US_DOLLAR),
			UserToUSDBalance::new(EVALUATOR_2, 25_000 * US_DOLLAR),
			UserToUSDBalance::new(EVALUATOR_3, 32_000 * US_DOLLAR),
		];
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project.total_allocation_size.0 / 4,
			project.minimum_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);

		let community_contributions = vec![
			ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 73 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions,
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let expected_evaluator_contributor_return =
			MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				vec![UserToUSDBalance::new(EVALUATOR_1, 50_000 * US_DOLLAR)],
				vec![ContributionParams::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT)],
				final_price,
				true,
			);
		let expected_bid_payouts = MockInstantiator::calculate_auction_plmc_spent(&bids, Some(final_price));
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions, final_price);
		let expected_remainder_contribution_payouts = MockInstantiator::calculate_contributed_plmc_spent(
			vec![
				ContributionParams::new(BIDDER_1, 13_400 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
				ContributionParams::new(BUYER_1, 42 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			],
			final_price,
		);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_evaluator_contributor_return,
				expected_bid_payouts,
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			MergeOperation::Add,
		);

		let prev_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
		let all_participants = all_expected_payouts.accounts();
		let prev_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants.clone());

		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_evaluations =
			inst.execute(|| Evaluations::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let stored_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for evaluation in stored_evaluations {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::evaluation_slash_for(
					RuntimeOrigin::signed(evaluation.evaluator),
					project_id,
					evaluation.evaluator,
					evaluation.id,
				),
				Pallet::<TestRuntime>::evaluation_unbond_for(
					RuntimeOrigin::signed(evaluation.evaluator),
					project_id,
					evaluation.evaluator,
					evaluation.id,
				)
			)
		}

		for bid in stored_bids {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					bid.bidder,
					bid.id,
				),
				Pallet::<TestRuntime>::bid_unbond_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
			)
		}

		for contribution in stored_contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				),
				Pallet::<TestRuntime>::contribution_unbond_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
		let post_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants);

		let all_participants_plmc_deltas = MockInstantiator::generic_map_operation(
			vec![post_participants_plmc_balances, prev_participants_plmc_balances],
			MergeOperation::Subtract,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_expected_payouts, all_participants_plmc_deltas);
	}
}

mod funding_end {
	use super::*;

	#[test]
	fn automatic_fail_less_eq_33_percent() {
		for funding_percent in (1..=33).step_by(5) {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
			let evaluations = default_evaluations();
			let bids = MockInstantiator::generate_bids_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let contributions = MockInstantiator::generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_contributors(),
				default_multipliers(),
			);
			let project_id =
				inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		}
	}

	#[test]
	fn automatic_success_bigger_eq_90_percent() {
		for funding_percent in (90..=100).step_by(2) {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
			let evaluations = default_evaluations();
			let bids = MockInstantiator::generate_bids_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let contributions = MockInstantiator::generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_contributors(),
				default_multipliers(),
			);
			let project_id =
				inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		}
	}

	#[test]
	fn manual_outcome_above33_to_below90() {
		for funding_percent in (34..90).step_by(5) {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
			let evaluations = default_evaluations();
			let bids = MockInstantiator::generate_bids_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let contributions = MockInstantiator::generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_contributors(),
				default_multipliers(),
			);
			let project_id =
				inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);
		}
	}

	#[test]
	fn manual_acceptance() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let project_id = project_id;
		inst.execute(|| {
			FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::AcceptFunding)
		})
		.unwrap();

		inst.advance_time(1u64).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		assert_matches!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(_)));
		inst.test_ct_created_for(project_id);

		inst.advance_time(10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn manual_rejection() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let project_id = project_id;
		inst.execute(|| {
			FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::RejectFunding)
		})
		.unwrap();

		inst.advance_time(1u64).unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		inst.test_ct_not_created_for(project_id);

		inst.advance_time(10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn automatic_acceptance_on_manual_decision_after_time_delta() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size.0).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_bidders(),
			default_multipliers(),
		);
		let contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let project_id = project_id;
		inst.advance_time(1u64 + <TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		inst.test_ct_created_for(project_id);

		inst.advance_time(10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn evaluators_get_slashed_funding_accepted() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = project_from_funding_reached(&mut inst, 43u64);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let old_evaluation_locked_plmc = inst
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			inst,
			FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::AcceptFunding)
		);
		inst.advance_time(1u64).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
			vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
			MergeOperation::Add,
		);

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators);

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_funding_rejected() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = project_from_funding_reached(&mut inst, 56u64);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let old_evaluation_locked_plmc = inst
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			inst,
			FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::RejectFunding)
		);
		inst.advance_time(1u64).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
			vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
			MergeOperation::Add,
		);

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators);

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_failed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = project_from_funding_reached(&mut inst, 24u64);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

		let old_evaluation_locked_plmc = inst
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<_>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
			vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
			MergeOperation::Add,
		);

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators);

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn multiplier_gets_correct_vesting_duration() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project = default_project(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = vec![
			BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 20_000 * ASSET_UNIT, 1.into(), 2u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_3, 20_000 * ASSET_UNIT, 11.into(), 3u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let project_id = inst.create_finished_project(
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let mut stored_bids =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		stored_bids.sort_by_key(|bid| bid.bidder);
		let one_week_in_blocks = DAYS * 7;
		assert_eq!(stored_bids[0].plmc_vesting_info.unwrap().duration, 1u64);
		assert_eq!(
			stored_bids[1].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(2167, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[2].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(4334, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
	}
}

mod ct_migration {
	use super::*;
	use frame_support::assert_err;

	#[test]
	fn para_id_for_project_can_be_set_by_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_finished_project(
			default_project(inst.get_new_nonce(), ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			vec![],
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::set_para_id_for_project(
				RuntimeOrigin::signed(ISSUER),
				project_id,
				ParaId::from(2006u32)
			));
		});
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.parachain_id, Some(ParaId::from(2006u32)));
	}

	#[test]
	fn para_id_for_project_cannot_be_set_by_anyone_but_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_finished_project(
			default_project(inst.get_new_nonce(), ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			vec![],
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		inst.execute(|| {
			assert_err!(
				crate::Pallet::<TestRuntime>::set_para_id_for_project(
					RuntimeOrigin::signed(EVALUATOR_1),
					project_id,
					ParaId::from(2006u32)
				),
				Error::<TestRuntime>::NotAllowed
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::set_para_id_for_project(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					ParaId::from(2006u32)
				),
				Error::<TestRuntime>::NotAllowed
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::set_para_id_for_project(
					RuntimeOrigin::signed(BUYER_1),
					project_id,
					ParaId::from(2006u32)
				),
				Error::<TestRuntime>::NotAllowed
			);
		});
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.parachain_id, None);
	}

	#[test]
	fn check_migrations_per_xcm() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.execute(|| dbg!(Pallet::<TestRuntime>::migrations_per_xcm_message_allowed()));
	}
}

mod test_helper_functions {
	use super::*;

	#[test]
	fn calculate_evaluation_plmc_spent() {
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1u64;
		const USD_AMOUNT_1: u128 = 150_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 17_857_1_428_571_428_u128;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2u64;
		const USD_AMOUNT_2: u128 = 50_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 5_952_3_809_523_809_u128;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3u64;
		const USD_AMOUNT_3: u128 = 75_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 8_928_5_714_285_714_u128;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4u64;
		const USD_AMOUNT_4: u128 = 100_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 11_9_047_619_047_u128;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5u64;
		const USD_AMOUNT_5: u128 = 123_7_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 14_7_261_904_761_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let evaluations = vec![
			UserToUSDBalance::new(EVALUATOR_1, USD_AMOUNT_1),
			UserToUSDBalance::new(EVALUATOR_2, USD_AMOUNT_2),
			UserToUSDBalance::new(EVALUATOR_3, USD_AMOUNT_3),
			UserToUSDBalance::new(EVALUATOR_4, USD_AMOUNT_4),
			UserToUSDBalance::new(EVALUATOR_5, USD_AMOUNT_5),
		];

		let expected_plmc_spent = vec![
			UserToPLMCBalance::new(EVALUATOR_1, EXPECTED_PLMC_AMOUNT_1),
			UserToPLMCBalance::new(EVALUATOR_2, EXPECTED_PLMC_AMOUNT_2),
			UserToPLMCBalance::new(EVALUATOR_3, EXPECTED_PLMC_AMOUNT_3),
			UserToPLMCBalance::new(EVALUATOR_4, EXPECTED_PLMC_AMOUNT_4),
			UserToPLMCBalance::new(EVALUATOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = MockInstantiator::calculate_evaluation_plmc_spent(evaluations);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_auction_plmc_spent() {
		const BIDDER_1: AccountIdOf<TestRuntime> = 1u64;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const PRICE_PER_TOKEN_1: f64 = 0.3f64;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 36_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 4_2_857_142_857_u128;

		const BIDDER_2: AccountIdOf<TestRuntime> = 2u64;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const PRICE_PER_TOKEN_2: f64 = 13f64;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 65_299_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 3_886_8_452_380_952_u128;

		const BIDDER_3: AccountIdOf<TestRuntime> = 3u64;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const PRICE_PER_TOKEN_3: f64 = 20f64;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 400_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_801_1_204_481_792_u128;

		const BIDDER_4: AccountIdOf<TestRuntime> = 4u64;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const PRICE_PER_TOKEN_4: f64 = 5.52f64;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_USD_4: u128 = 5_520_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 26_285_7_142_857_142_u128;

		const BIDDER_5: AccountIdOf<TestRuntime> = 5u64;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const PRICE_PER_TOKEN_5: f64 = 11.34f64;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_USD_5: u128 = 1_3_982_220_000_u128;
		// TODO: Is this due to rounding errors?
		// Should be in reality 0.0166455, but we get 0.0166454999. i.e error of 0.0000000001 PLMC
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_166_454_999_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let bids = vec![
			BidParams::new(
				BIDDER_1,
				TOKEN_AMOUNT_1,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_1),
				MULTIPLIER_1,
				AcceptedFundingAsset::USDT,
			),
			BidParams::new(
				BIDDER_2,
				TOKEN_AMOUNT_2,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_2),
				MULTIPLIER_2,
				AcceptedFundingAsset::USDT,
			),
			BidParams::new(
				BIDDER_3,
				TOKEN_AMOUNT_3,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_3),
				MULTIPLIER_3,
				AcceptedFundingAsset::USDT,
			),
			BidParams::new(
				BIDDER_4,
				TOKEN_AMOUNT_4,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_4),
				MULTIPLIER_4,
				AcceptedFundingAsset::USDT,
			),
			BidParams::new(
				BIDDER_5,
				TOKEN_AMOUNT_5,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_5),
				MULTIPLIER_5,
				AcceptedFundingAsset::USDT,
			),
		];

		let expected_plmc_spent = vec![
			UserToPLMCBalance::new(BIDDER_1, EXPECTED_PLMC_AMOUNT_1),
			UserToPLMCBalance::new(BIDDER_2, EXPECTED_PLMC_AMOUNT_2),
			UserToPLMCBalance::new(BIDDER_3, EXPECTED_PLMC_AMOUNT_3),
			UserToPLMCBalance::new(BIDDER_4, EXPECTED_PLMC_AMOUNT_4),
			UserToPLMCBalance::new(BIDDER_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = MockInstantiator::calculate_auction_plmc_spent(&bids, None);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_contributed_plmc_spent() {
		const PLMC_PRICE: f64 = 8.4f64;
		const CT_PRICE: f64 = 16.32f64;

		const CONTRIBUTOR_1: AccountIdOf<TestRuntime> = 1u64;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 1_958_4_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 233_1_428_571_428_u128;

		const CONTRIBUTOR_2: AccountIdOf<TestRuntime> = 2u64;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 81_975_3_600_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 4_879_4_857_142_857_u128;

		const CONTRIBUTOR_3: AccountIdOf<TestRuntime> = 3u64;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 326_400_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_285_7_142_857_142_u128;

		const CONTRIBUTOR_4: AccountIdOf<TestRuntime> = 4u64;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_4: u128 = 16_320_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 77_714_2_857_142_857_u128;

		const CONTRIBUTOR_5: AccountIdOf<TestRuntime> = 5u64;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_5: u128 = 2_0_122_562_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_239_554_285_u128;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let contributions = vec![
			ContributionParams::new(CONTRIBUTOR_1, TOKEN_AMOUNT_1, MULTIPLIER_1, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_2, TOKEN_AMOUNT_2, MULTIPLIER_2, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_3, TOKEN_AMOUNT_3, MULTIPLIER_3, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_4, TOKEN_AMOUNT_4, MULTIPLIER_4, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_5, TOKEN_AMOUNT_5, MULTIPLIER_5, AcceptedFundingAsset::USDT),
		];

		let expected_plmc_spent = vec![
			UserToPLMCBalance::new(CONTRIBUTOR_1, EXPECTED_PLMC_AMOUNT_1),
			UserToPLMCBalance::new(CONTRIBUTOR_2, EXPECTED_PLMC_AMOUNT_2),
			UserToPLMCBalance::new(CONTRIBUTOR_3, EXPECTED_PLMC_AMOUNT_3),
			UserToPLMCBalance::new(CONTRIBUTOR_4, EXPECTED_PLMC_AMOUNT_4),
			UserToPLMCBalance::new(CONTRIBUTOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = MockInstantiator::calculate_contributed_plmc_spent(
			contributions,
			PriceOf::<TestRuntime>::from_float(CT_PRICE),
		);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_price_from_test_bids() {
		let bids = vec![
			BidParams::new(100, 10_000_0_000_000_000, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(200, 20_000_0_000_000_000, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(300, 20_000_0_000_000_000, 10.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let price = MockInstantiator::calculate_price_from_test_bids(bids);
		let price_in_10_decimals = price.checked_mul_int(1_0_000_000_000_u128).unwrap();

		assert_eq!(price_in_10_decimals, 16_3_333_333_333_u128);
	}
}

mod misc_features {
	use super::*;

	#[test]
	fn remove_from_update_store_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let now = inst.current_block();
		inst.execute(|| {
			FundingModule::add_to_update_store(now + 10u64, (&42u32, CommunityFundingStart));
			FundingModule::add_to_update_store(now + 20u64, (&69u32, RemainderFundingStart));
			FundingModule::add_to_update_store(now + 5u64, (&404u32, RemainderFundingStart));
		});
		inst.advance_time(2u64).unwrap();
		inst.execute(|| {
			let stored = ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored.len(), 3, "There should be 3 blocks scheduled for updating");

			FundingModule::remove_from_update_store(&69u32).unwrap();

			let stored = ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored[2], vec![], "Vector should be empty for that block after deletion");
		});
	}

	#[test]
	fn calculate_vesting_duration() {
		let default_multiplier = MultiplierOf::<TestRuntime>::default();
		let default_multiplier_duration = default_multiplier.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(default_multiplier_duration, 1u64);

		let multiplier_1 = MultiplierOf::<TestRuntime>::new(1u8).unwrap();
		let multiplier_1_duration = multiplier_1.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_1_duration, 1u64);

		let multiplier_2 = MultiplierOf::<TestRuntime>::new(2u8).unwrap();
		let multiplier_2_duration = multiplier_2.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_2_duration, FixedU128::from_rational(2167, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_3 = MultiplierOf::<TestRuntime>::new(3u8).unwrap();
		let multiplier_3_duration = multiplier_3.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_3_duration, FixedU128::from_rational(4334, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_19 = MultiplierOf::<TestRuntime>::new(19u8).unwrap();
		let multiplier_19_duration = multiplier_19.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_19_duration, FixedU128::from_rational(39006, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_20 = MultiplierOf::<TestRuntime>::new(20u8).unwrap();
		let multiplier_20_duration = multiplier_20.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_20_duration, FixedU128::from_rational(41173, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_24 = MultiplierOf::<TestRuntime>::new(24u8).unwrap();
		let multiplier_24_duration = multiplier_24.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_24_duration, FixedU128::from_rational(49841, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_25 = MultiplierOf::<TestRuntime>::new(25u8).unwrap();
		let multiplier_25_duration = multiplier_25.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_25_duration, FixedU128::from_rational(52008, 1000).saturating_mul_int((DAYS * 7) as u64));
	}

	#[test]
	fn sandbox() {
		assert!(true);
	}
}
