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

//! Tests for Funding pallet.
use super::*;
use crate::{
	instantiator::{async_features::create_multiple_projects_at, *},
	mock::*,
	traits::{ProvideAssetPrice, VestingDurationCalculation},
	CurrencyMetadata, Error, ProjectMetadata, TicketSize,
};
use assert_matches2::assert_matches;
use defaults::*;
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold},
		Get,
	},
};
use itertools::Itertools;
use parachains_common::DAYS;
use polimec_common::{credentials::*, ReleaseSchedule};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_runtime::{BuildStorage, TokenError};
use sp_std::{cell::RefCell, marker::PhantomData};
use std::{cmp::min, iter::zip, ops::Not};

type MockInstantiator =
	Instantiator<TestRuntime, <TestRuntime as crate::Config>::AllPalletsWithoutSystem, RuntimeEvent>;

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
const EVALUATOR_4: AccountId = 23;
const EVALUATOR_5: AccountId = 24;
const BIDDER_1: AccountId = 30;
const BIDDER_2: AccountId = 31;
const BIDDER_3: AccountId = 32;
const BIDDER_4: AccountId = 33;
const BIDDER_5: AccountId = 34;
const BIDDER_6: AccountId = 35;
const BUYER_1: AccountId = 40;
const BUYER_2: AccountId = 41;
const BUYER_3: AccountId = 42;
const BUYER_4: AccountId = 43;
const BUYER_5: AccountId = 44;
const BUYER_6: AccountId = 45;
const BUYER_7: AccountId = 46;
const BUYER_8: AccountId = 47;
const BUYER_9: AccountId = 48;

const ASSET_UNIT: u128 = 10_u128.pow(10u32);

const USDT_FOREIGN_ID: crate::mock::AssetId = 1984u32;
const USDT_UNIT: u128 = 1_0_000_000_000_u128;

pub mod defaults {
	use super::*;

	pub fn default_token_information() -> CurrencyMetadata<BoundedVec<u8, StringLimitOf<TestRuntime>>> {
		CurrencyMetadata {
			name: BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap(),
			symbol: BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap(),
			decimals: ASSET_DECIMALS,
		}
	}
	pub fn default_project_metadata(nonce: u64, issuer: AccountId) -> ProjectMetadataOf<TestRuntime> {
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
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: issuer,
			offchain_information_hash: Some(metadata_hash),
		}
	}

	pub fn knowledge_hub_project(nonce: u64) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		let project_metadata = ProjectMetadataOf::<TestRuntime> {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
		};
		project_metadata
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
			UserToUSDBalance::new(EVALUATOR_1, 500_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_2, 250_000 * PLMC),
			UserToUSDBalance::new(EVALUATOR_3, 320_000 * PLMC),
		]
	}

	pub fn knowledge_hub_evaluations() -> Vec<UserToUSDBalance<TestRuntime>> {
		vec![
			UserToUSDBalance::new(EVALUATOR_1, 75_000 * USDT_UNIT),
			UserToUSDBalance::new(EVALUATOR_2, 65_000 * USDT_UNIT),
			UserToUSDBalance::new(EVALUATOR_3, 60_000 * USDT_UNIT),
		]
	}

	pub fn default_failing_evaluations() -> Vec<UserToUSDBalance<TestRuntime>> {
		vec![UserToUSDBalance::new(EVALUATOR_1, 3_000 * PLMC), UserToUSDBalance::new(EVALUATOR_2, 1_000 * PLMC)]
	}

	pub fn default_bids() -> Vec<BidParams<TestRuntime>> {
		vec![
			BidParams::new(BIDDER_1, 400_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 50_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn knowledge_hub_bids() -> Vec<BidParams<TestRuntime>> {
		// This should reflect the bidding currency, which currently is USDT
		vec![
			BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 20_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_3, 20_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_4, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_5, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_6, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_community_buys() -> Vec<ContributionParams<TestRuntime>> {
		vec![
			ContributionParams::new(BUYER_1, 50_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 130_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 30_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_4, 210_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_5, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_remainder_buys() -> Vec<ContributionParams<TestRuntime>> {
		vec![
			ContributionParams::new(EVALUATOR_2, 20_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BIDDER_1, 30_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn knowledge_hub_buys() -> Vec<ContributionParams<TestRuntime>> {
		vec![
			ContributionParams::new(BUYER_1, 4_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_2, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_3, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_4, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_5, 30_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_6, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BUYER_7, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_weights() -> Vec<u8> {
		vec![20u8, 15u8, 10u8, 25u8, 30u8]
	}

	pub fn default_evaluators() -> Vec<AccountId> {
		vec![EVALUATOR_1, EVALUATOR_2, EVALUATOR_3, EVALUATOR_4, EVALUATOR_5]
	}
	pub fn default_bidders() -> Vec<AccountId> {
		vec![BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5]
	}
	pub fn default_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}
	pub fn default_bidder_multipliers() -> Vec<u8> {
		vec![20u8, 3u8, 15u8, 13u8, 9u8]
	}
	pub fn default_community_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 5u8, 3u8, 1u8, 2u8]
	}
	pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 10u8, 3u8, 2u8, 4u8]
	}

	pub fn default_community_contributors() -> Vec<AccountId> {
		vec![BUYER_1, BUYER_2, BUYER_3, BUYER_4, BUYER_5]
	}

	pub fn default_remainder_contributors() -> Vec<AccountId> {
		vec![EVALUATOR_1, BIDDER_3, BUYER_4, BUYER_6, BIDDER_6]
	}

	pub fn project_from_funding_reached(instantiator: &mut MockInstantiator, percent: u64) -> ProjectId {
		let project_metadata = default_project_metadata(instantiator.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let usd_to_reach = Perquintill::from_percent(percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
			default_community_contributors(),
			default_multipliers(),
		);
		instantiator.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![])
	}
}

// only functionalities that happen in the CREATION period of a project
mod creation {
	use super::*;
	use polimec_common::credentials::InvestorType;
	use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};

	#[test]
	fn create_extrinsic() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
		inst.mint_plmc_to(default_plmc_balances());
		let jwt = get_mock_jwt(ISSUER, InvestorType::Institutional, generate_did_from_account(ISSUER));
		assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::create(
			RuntimeOrigin::signed(ISSUER),
			jwt,
			project_metadata
		)));
	}

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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);

		inst.create_evaluating_project(project_metadata, issuer);
	}

	#[test]
	fn multiple_creations() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		for _ in 0..512 {
			let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
			inst.create_evaluating_project(project_metadata, issuer);
			inst.advance_time(1u64).unwrap();
		}
	}

	#[test]
	fn project_id_autoincrement_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_1 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project_2 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project_3 = default_project_metadata(inst.get_new_nonce(), issuer);

		let created_project_1_id = inst.create_evaluating_project(project_1, ISSUER);
		let created_project_2_id = inst.create_evaluating_project(project_2, ISSUER);
		let created_project_3_id = inst.create_evaluating_project(project_3, ISSUER);

		assert_eq!(created_project_1_id, 0);
		assert_eq!(created_project_2_id, 1);
		assert_eq!(created_project_3_id, 2);
	}

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 100_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: 0_u128.into(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.mint_plmc_to(default_plmc_balances());
		let project_err = inst.execute(|| Pallet::<TestRuntime>::do_create(&ISSUER, wrong_project).unwrap_err());
		assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into());
	}

	#[test]
	fn ticket_sizes_validity_check() {
		let correct_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 100_000_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Default::default(),
			minimum_price: 10_u128.into(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let mut counter: u8 = 0u8;
		let mut with_different_metadata = |mut project: ProjectMetadataOf<TestRuntime>| {
			let mut binding = project.offchain_information_hash.unwrap();
			let h256_bytes = binding.as_fixed_bytes_mut();
			h256_bytes[0] = counter;
			counter += 1u8;
			project.offchain_information_hash = Some(binding);
			project
		};

		// min in bidding below 5k
		let mut wrong_project_1 = correct_project.clone();
		wrong_project_1.bidding_ticket_sizes.professional = TicketSize::new(Some(4999 * US_DOLLAR), None);

		let mut wrong_project_2 = correct_project.clone();
		wrong_project_2.bidding_ticket_sizes.institutional = TicketSize::new(Some(4999 * US_DOLLAR), None);

		let mut wrong_project_3 = correct_project.clone();
		wrong_project_3.bidding_ticket_sizes.professional = TicketSize::new(Some(3000 * US_DOLLAR), None);
		wrong_project_3.bidding_ticket_sizes.institutional = TicketSize::new(Some(0 * US_DOLLAR), None);

		let mut wrong_project_4 = correct_project.clone();
		wrong_project_4.bidding_ticket_sizes.professional = TicketSize::new(None, None);
		wrong_project_4.bidding_ticket_sizes.institutional = TicketSize::new(None, None);

		// min higher than max
		let mut wrong_project_5 = correct_project.clone();
		wrong_project_5.bidding_ticket_sizes.professional =
			TicketSize::new(Some(5000 * US_DOLLAR), Some(4990 * US_DOLLAR));

		let mut wrong_project_6 = correct_project.clone();
		wrong_project_6.bidding_ticket_sizes.institutional =
			TicketSize::new(Some(6000 * US_DOLLAR), Some(5500 * US_DOLLAR));

		let wrong_projects = vec![
			wrong_project_1.clone(),
			wrong_project_2,
			wrong_project_3.clone(),
			wrong_project_4,
			wrong_project_5,
			wrong_project_6,
		];

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		inst.mint_plmc_to(default_plmc_balances());

		let test_1 = with_different_metadata(wrong_project_1);
		let test_2 = with_different_metadata(wrong_project_3);
		assert!(test_1.offchain_information_hash != test_2.offchain_information_hash);

		for project in wrong_projects {
			let project_err = inst
				.execute(|| Pallet::<TestRuntime>::do_create(&ISSUER, with_different_metadata(project)).unwrap_err());
			assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
		}
	}

	#[test]
	fn issuer_cannot_pay_for_escrow_ed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let ed = MockInstantiator::get_ed();

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(ISSUER, ed)]);
		let project_err = inst.execute(|| Pallet::<TestRuntime>::do_create(&ISSUER, project_metadata).unwrap_err());
		assert_eq!(project_err, Error::<TestRuntime>::NotEnoughFundsForEscrowCreation.into());
	}

	#[test]
	fn multiple_funding_currencies() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let mut counter: u8 = 1u8;
		let mut with_different_metadata = |mut project: ProjectMetadataOf<TestRuntime>| {
			let mut binding = project.offchain_information_hash.unwrap();
			let h256_bytes = binding.as_fixed_bytes_mut();
			h256_bytes[0] = counter;
			counter += 1u8;
			project.offchain_information_hash = Some(binding);
			project
		};
		let default_project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);

		let mut one_currency_1 = default_project_metadata.clone();
		one_currency_1.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

		let mut one_currency_2 = default_project_metadata.clone();
		one_currency_2.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

		let mut one_currency_3 = default_project_metadata.clone();
		one_currency_3.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut two_currencies_1 = default_project_metadata.clone();
		two_currencies_1.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC].try_into().unwrap();

		let mut two_currencies_2 = default_project_metadata.clone();
		two_currencies_2.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut two_currencies_3 = default_project_metadata.clone();
		two_currencies_3.participation_currencies =
			vec![AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut three_currencies = default_project_metadata.clone();
		three_currencies.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

		let projects = vec![
			one_currency_1.clone(),
			one_currency_2.clone(),
			one_currency_3,
			two_currencies_1,
			two_currencies_2,
			two_currencies_3,
			three_currencies,
		];

		inst.mint_plmc_to(default_plmc_balances());

		let test_1 = with_different_metadata(one_currency_1);
		let test_2 = with_different_metadata(one_currency_2);
		assert!(test_1.offchain_information_hash != test_2.offchain_information_hash);

		for project in projects {
			let project_metadata_new = with_different_metadata(project);
			assert_ok!(inst.execute(|| { Pallet::<TestRuntime>::do_create(&ISSUER, project_metadata_new) }));
		}

		let mut wrong_project_1 = default_project_metadata.clone();
		wrong_project_1.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT].try_into().unwrap();

		let mut wrong_project_2 = default_project_metadata.clone();
		wrong_project_2.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT]
				.try_into()
				.unwrap();

		let mut wrong_project_3 = default_project_metadata.clone();
		wrong_project_3.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::USDT]
				.try_into()
				.unwrap();

		let mut wrong_project_4 = default_project_metadata.clone();
		wrong_project_4.participation_currencies =
			vec![AcceptedFundingAsset::DOT, AcceptedFundingAsset::DOT, AcceptedFundingAsset::USDC].try_into().unwrap();

		let wrong_projects = vec![wrong_project_1, wrong_project_2, wrong_project_3, wrong_project_4];
		for project in wrong_projects {
			let project_err = inst
				.execute(|| Pallet::<TestRuntime>::do_create(&ISSUER, with_different_metadata(project)).unwrap_err());
			assert_eq!(project_err, Error::<TestRuntime>::ParticipationCurrenciesError.into());
		}
	}
}

// only functionalities that happen in the EVALUATION period of a project
mod evaluation {
	use super::*;

	#[test]
	fn evaluation_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		inst.create_auctioning_project(project_metadata, issuer, evaluations);
	}

	#[test]
	fn multiple_evaluating_projects() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project1 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project2 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project3 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project4 = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		inst.create_auctioning_project(project1, issuer, evaluations.clone());
		inst.create_auctioning_project(project2, issuer, evaluations.clone());
		inst.create_auctioning_project(project3, issuer, evaluations.clone());
		inst.create_auctioning_project(project4, issuer, evaluations);
	}

	#[test]
	fn rewards_are_paid_full_funding() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let project_metadata = knowledge_hub_project(0);
		let evaluations = knowledge_hub_evaluations();
		let bids = knowledge_hub_bids();
		let contributions = knowledge_hub_buys();

		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		inst.advance_time(10).unwrap();

		let actual_reward_balances = inst.execute(|| {
			vec![
				(EVALUATOR_1, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_1)),
				(EVALUATOR_2, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_2)),
				(EVALUATOR_3, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_3)),
			]
		});
		let expected_ct_rewards = vec![
			(EVALUATOR_1, 1_332_4_500_000_000),
			(EVALUATOR_2, 917_9_100_000_000),
			(EVALUATOR_3, 710_6_400_000_000),
		];

		for (real, desired) in zip(actual_reward_balances.iter(), expected_ct_rewards.iter()) {
			assert_close_enough!(real.1, desired.1, Perquintill::from_float(0.01));
		}
	}

	#[test]
	fn round_fails_after_not_enough_bonds() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let now = inst.current_block();
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_failing_evaluations();
		let plmc_eval_deposits: Vec<UserToPLMCBalance<_>> =
			MockInstantiator::calculate_evaluation_plmc_spent(evaluations);
		let plmc_existential_deposits = plmc_eval_deposits.accounts().existential_deposits();
		let ct_account_deposits = plmc_eval_deposits.accounts().ct_account_deposits();

		let expected_evaluator_balances = MockInstantiator::generic_map_operation(
			vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone(), ct_account_deposits.clone()],
			MergeOperation::Add,
		);

		inst.mint_plmc_to(plmc_eval_deposits.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(ct_account_deposits.clone());

		let project_id = inst.create_evaluating_project(project_metadata, issuer);

		let evaluation_end = inst
			.get_project_details(project_id)
			.phase_transition_points
			.evaluation
			.end
			.expect("Evaluation round end block should be set");

		inst.bond_for_users(project_id, default_failing_evaluations()).expect("Bonding should work");

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_reserved_plmc_assertions(plmc_eval_deposits, HoldReason::Evaluation(project_id).into());

		inst.advance_time(evaluation_end - now + 1).unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationFailed);

		// Check that on_idle has unlocked the failed bonds
		inst.advance_time(10).unwrap();
		inst.do_free_plmc_assertions(expected_evaluator_balances);
	}

	#[test]
	fn evaluation_fails_on_insufficient_balance() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let insufficient_eval_deposits = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone())
			.iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance::new(*account, plmc_amount / 2))
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let plmc_existential_deposits = insufficient_eval_deposits.accounts().existential_deposits();

		inst.mint_plmc_to(insufficient_eval_deposits);
		inst.mint_plmc_to(plmc_existential_deposits);

		let project_id = inst.create_evaluating_project(project_metadata, issuer);

		let dispatch_error = inst.bond_for_users(project_id, evaluations);
		assert_err!(dispatch_error, TokenError::FundsUnavailable)
	}

	#[test]
	fn evaluation_ct_account_deposits_are_returned_on_evaluation_failed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER);
		let evaluation_success_threshold = <TestRuntime as Config>::EvaluationSuccessThreshold::get();
		let evaluation_min_success_amount = evaluation_success_threshold *
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		let evaluation_fail_amount = evaluation_min_success_amount - 100 * ASSET_UNIT;
		let evaluator_bond = evaluation_fail_amount / 4;
		let evaluations = vec![
			(EVALUATOR_1, evaluator_bond).into(),
			(EVALUATOR_1, evaluator_bond).into(),
			(EVALUATOR_2, evaluator_bond).into(),
			(EVALUATOR_3, evaluator_bond).into(),
		];
		let deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(project_id);
		inst.do_free_plmc_assertions(vec![
			(EVALUATOR_1, 0u128).into(),
			(EVALUATOR_2, 0u128).into(),
			(EVALUATOR_3, 0u128).into(),
		]);
		inst.do_reserved_plmc_assertions(
			vec![(EVALUATOR_1, 0u128).into(), (EVALUATOR_2, 0u128).into(), (EVALUATOR_3, 0u128).into()],
			HoldReason::FutureDeposit(project_id).into(),
		);

		let required_plmc_bonds = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits = required_plmc_bonds.accounts().existential_deposits();
		let plmc_ct_account_deposits = required_plmc_bonds.accounts().ct_account_deposits();

		inst.mint_plmc_to(required_plmc_bonds.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		let _ = inst.bond_for_users(project_id, evaluations);

		inst.do_free_plmc_assertions(vec![
			(EVALUATOR_1, MockInstantiator::get_ed()).into(),
			(EVALUATOR_2, MockInstantiator::get_ed()).into(),
			(EVALUATOR_3, MockInstantiator::get_ed()).into(),
		]);
		inst.do_reserved_plmc_assertions(
			vec![
				(EVALUATOR_1, deposit_required).into(),
				(EVALUATOR_2, deposit_required).into(),
				(EVALUATOR_3, deposit_required).into(),
			],
			HoldReason::FutureDeposit(project_id).into(),
		);

		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationFailed);

		let final_plmc_amounts = MockInstantiator::generic_map_operation(
			vec![required_plmc_bonds, plmc_existential_deposits, plmc_ct_account_deposits],
			MergeOperation::Add,
		);
		inst.do_free_plmc_assertions(final_plmc_amounts);
		inst.do_reserved_plmc_assertions(
			vec![(EVALUATOR_1, 0u128).into(), (EVALUATOR_2, 0u128).into(), (EVALUATOR_3, 0u128).into()],
			HoldReason::FutureDeposit(project_id).into(),
		);
	}

	#[test]
	fn cannot_evaluate_more_than_project_limit() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let evaluations = (0u32..<TestRuntime as Config>::MaxEvaluationsPerProject::get())
			.map(|i| UserToUSDBalance::<TestRuntime>::new(i as u32 + 420u32, (10u128 * ASSET_UNIT).into()))
			.collect_vec();
		let failing_evaluation = UserToUSDBalance::new(EVALUATOR_1, 1000 * ASSET_UNIT);

		let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER);

		let plmc_for_evaluating = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits = evaluations.accounts().existential_deposits();
		let plmc_ct_account_deposits = evaluations.accounts().ct_account_deposits();

		inst.mint_plmc_to(plmc_for_evaluating.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		inst.bond_for_users(project_id, evaluations.clone()).unwrap();

		let plmc_for_failing_evaluating =
			MockInstantiator::calculate_evaluation_plmc_spent(vec![failing_evaluation.clone()]);
		let plmc_existential_deposits = plmc_for_failing_evaluating.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_for_failing_evaluating.accounts().ct_account_deposits();

		inst.mint_plmc_to(plmc_for_failing_evaluating.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		assert_err!(
			inst.bond_for_users(project_id, vec![failing_evaluation]),
			Error::<TestRuntime>::TooManyEvaluationsForProject
		);
	}
}

// only functionalities that happen in the AUCTION period of a project
mod auction {
	use super::*;
	use crate::instantiator::async_features::create_multiple_projects_at;
	use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};

	#[test]
	fn auction_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let _project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
	}

	#[test]
	fn multiple_auction_projects_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project1 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project2 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project3 = default_project_metadata(inst.get_new_nonce(), issuer);
		let project4 = default_project_metadata(inst.get_new_nonce(), issuer);
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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_bidder = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let evaluator_bid = BidParams::new(evaluator_bidder, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_bidder, evaluation_amount).into());

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

		let already_bonded_plmc =
			MockInstantiator::calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0]
				.plmc_amount;

		let usable_evaluation_plmc =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;

		let necessary_plmc_for_bid = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&vec![evaluator_bid.clone()],
			project_metadata.minimum_price,
		)[0]
		.plmc_amount;

		let necessary_usdt_for_bid = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
			&vec![evaluator_bid.clone()],
			project_metadata.minimum_price,
		);

		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_bidder,
			necessary_plmc_for_bid - usable_evaluation_plmc,
		)]);

		inst.mint_foreign_asset_to(necessary_usdt_for_bid);

		inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();
	}

	#[test]
	fn price_calculation() {
		// From the knowledge hub: https://hub.polimec.org/learn/calculation-example#auction-round-calculation-example
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		const ADAM: u32 = 60;
		const TOM: u32 = 61;
		const SOFIA: u32 = 62;
		const FRED: u32 = 63;
		const ANNA: u32 = 64;
		const DAMIAN: u32 = 65;

		let accounts = vec![ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];

		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, 0));
		let project_metadata = ProjectMetadata {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
		};

		// overfund with plmc
		let plmc_fundings = accounts
			.iter()
			.map(|acc| UserToPLMCBalance { account: acc.clone(), plmc_amount: PLMC * 1_000_000 })
			.collect_vec();
		let usdt_fundings = accounts
			.iter()
			.map(|acc| UserToForeignAssets {
				account: acc.clone(),
				asset_amount: US_DOLLAR * 1_000_000,
				asset_id: AcceptedFundingAsset::USDT.to_assethub_id(),
			})
			.collect_vec();
		inst.mint_plmc_to(plmc_fundings);
		inst.mint_foreign_asset_to(usdt_fundings);

		let project_id = inst.create_auctioning_project(project_metadata, ISSUER, default_evaluations());

		let bids = vec![
			(ADAM, 10_000 * ASSET_UNIT).into(),
			(TOM, 20_000 * ASSET_UNIT).into(),
			(SOFIA, 20_000 * ASSET_UNIT).into(),
			(FRED, 10_000 * ASSET_UNIT).into(),
			(ANNA, 5_000 * ASSET_UNIT).into(),
			(DAMIAN, 5_000 * ASSET_UNIT).into(),
		];

		inst.bid_for_users(project_id, bids).unwrap();

		inst.start_community_funding(project_id).unwrap();

		let token_price =
			inst.get_project_details(project_id).weighted_average_price.unwrap().saturating_mul_int(ASSET_UNIT);

		let desired_price = PriceOf::<TestRuntime>::from_float(11.1818f64).saturating_mul_int(ASSET_UNIT);

		assert_close_enough!(token_price, desired_price, Perquintill::from_float(0.01));
	}

	#[test]
	fn only_candle_bids_before_random_block_get_included() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let mut project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		project_metadata.total_allocation_size = 1_000_000 * ASSET_UNIT;
		let evaluations = MockInstantiator::generate_successful_evaluations(
			project_metadata.clone(),
			default_evaluators(),
			default_weights(),
		);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);
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

		let bid_info = BidParams::new(0, 500u128 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let plmc_necessary_funding = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&vec![bid_info.clone()],
			project_metadata.minimum_price,
		)[0]
		.plmc_amount;

		let foreign_asset_necessary_funding =
			MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_info.clone()],
				project_metadata.minimum_price,
			)[0]
			.asset_amount;

		let mut bids_made: Vec<BidParams<TestRuntime>> = vec![];
		let starting_bid_block = inst.current_block();
		let blocks_to_bid = inst.current_block()..candle_end_block;

		let mut bidding_account = 1000;

		// Do one candle bid for each block until the end of candle auction with a new user
		for _block in blocks_to_bid {
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::Candle));
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(bidding_account, plmc_necessary_funding * 10)]);
			inst.mint_plmc_to(vec![bidding_account].existential_deposits());
			inst.mint_plmc_to(vec![bidding_account].ct_account_deposits());

			inst.mint_foreign_asset_to(vec![UserToForeignAssets::new(
				bidding_account,
				foreign_asset_necessary_funding * 10,
				bid_info.asset.to_assethub_id(),
			)]);
			let bids: Vec<BidParams<_>> = vec![BidParams {
				bidder: bidding_account,
				amount: bid_info.amount,
				multiplier: bid_info.multiplier,
				asset: bid_info.asset,
			}];
			inst.bid_for_users(project_id, bids.clone()).unwrap();

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
				original_ct_usd_price: None,
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
		let project_id = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		let ct_acount_deposits = required_plmc.accounts().ct_account_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.mint_plmc_to(ct_acount_deposits);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn issuer_can_start_auction_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		let ct_acount_deposits = required_plmc.accounts().ct_account_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.mint_plmc_to(ct_acount_deposits);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(1).unwrap();
		inst.execute(|| Pallet::<TestRuntime>::do_english_auction(ISSUER, project_id)).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn stranger_cannot_start_auction_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc = required_plmc.accounts().existential_deposits();
		let ct_acount_deposits = required_plmc.accounts().ct_account_deposits();
		inst.mint_plmc_to(required_plmc);
		inst.mint_plmc_to(ed_plmc);
		inst.mint_plmc_to(ct_acount_deposits);
		inst.bond_for_users(project_id, evaluations).unwrap();
		inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
		inst.advance_time(1).unwrap();

		for account in 6000..6010 {
			inst.execute(|| {
				let response = Pallet::<TestRuntime>::do_english_auction(account, project_id);
				assert_noop!(response, Error::<TestRuntime>::NotAllowed);
			});
		}
	}

	#[test]
	fn bidder_was_evaluator() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let evaluations = default_evaluations();
		let mut bids = default_bids();
		let evaluator = evaluations[0].account;
		bids.push(BidParams::new(evaluator, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT));
		let _ = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
	}

	#[test]
	fn bids_at_higher_price_than_weighted_average_use_average() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids: Vec<BidParams<_>> = MockInstantiator::generate_bids_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);

		let second_bucket_bid = (BIDDER_6, 500 * ASSET_UNIT).into();
		bids.push(second_bucket_bid);

		let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
		let bidder_5_bid =
			inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, BIDDER_6)).next().unwrap());
		let wabgp = inst.get_project_details(project_id).weighted_average_price.unwrap();
		assert_eq!(bidder_5_bid.original_ct_usd_price.to_float(), 11.0);
		assert_eq!(bidder_5_bid.final_ct_usd_price, wabgp);
	}

	#[test]
	fn unsuccessful_bids_dont_get_vest_schedule() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let auction_token_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		let mut bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(80) * project_metadata.minimum_price.saturating_mul_int(auction_token_allocation),
			project_metadata.minimum_price,
			vec![60, 40],
			vec![BIDDER_1, BIDDER_2],
			vec![1u8, 1u8],
		);

		let available_tokens =
			auction_token_allocation.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));

		let rejected_bid = vec![BidParams::new(BIDDER_5, available_tokens, 1u8, AcceptedFundingAsset::USDT)];
		let accepted_bid = vec![BidParams::new(BIDDER_4, available_tokens, 1u8, AcceptedFundingAsset::USDT)];
		bids.extend(rejected_bid.clone());
		bids.extend(accepted_bid.clone());

		let community_contributions = default_community_buys();

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

		let bidders_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);
		let bidders_existential_deposits = bidders_plmc.accounts().existential_deposits();
		let bidders_ct_account_deposits = bidders_plmc.accounts().ct_account_deposits();
		inst.mint_plmc_to(bidders_plmc.clone());
		inst.mint_plmc_to(bidders_existential_deposits);
		inst.mint_plmc_to(bidders_ct_account_deposits);

		let bidders_funding_assets =
			MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
		inst.mint_foreign_asset_to(bidders_funding_assets);

		inst.bid_for_users(project_id, bids).unwrap();

		inst.start_community_funding(project_id).unwrap();

		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors_plmc =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		let contributors_existential_deposits = contributors_plmc.accounts().existential_deposits();
		let contributors_ct_account_deposits = contributors_plmc.accounts().ct_account_deposits();
		inst.mint_plmc_to(contributors_plmc.clone());
		inst.mint_plmc_to(contributors_existential_deposits);
		inst.mint_plmc_to(contributors_ct_account_deposits);

		let contributors_funding_assets =
			MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
		inst.mint_foreign_asset_to(contributors_funding_assets);

		inst.contribute_for_users(project_id, community_contributions).unwrap();
		inst.start_remainder_or_end_funding(project_id).unwrap();
		inst.finish_funding(project_id).unwrap();

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let plmc_locked_for_accepted_bid =
			MockInstantiator::calculate_auction_plmc_charged_with_given_price(&accepted_bid, final_price);
		let plmc_locked_for_rejected_bid =
			MockInstantiator::calculate_auction_plmc_charged_with_given_price(&rejected_bid, final_price);

		let UserToPLMCBalance { account: accepted_user, plmc_amount: accepted_plmc_amount } =
			plmc_locked_for_accepted_bid[0];
		let schedule = inst.execute(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&accepted_user,
				HoldReason::Participation(project_id).into(),
			)
		});
		assert_close_enough!(schedule.unwrap(), accepted_plmc_amount, Perquintill::from_float(0.01));

		let UserToPLMCBalance { account: rejected_user, .. } = plmc_locked_for_rejected_bid[0];
		let schedule_exists = inst
			.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&rejected_user,
					HoldReason::Participation(project_id).into(),
				)
			})
			.is_some();
		assert!(!schedule_exists);
	}

	// We use the already tested instantiator functions to calculate the correct post-wap returns
	#[test]
	fn refund_on_partial_acceptance_and_price_above_wap_and_ct_sold_out_bids() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();

		let bid_1 = BidParams::new(BIDDER_1, 5000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let bid_2 = BidParams::new(BIDDER_2, 40_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let bid_3 = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let bid_4 = BidParams::new(BIDDER_3, 6000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let bid_5 = BidParams::new(BIDDER_4, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		// post bucketing, the bids look like this:
		// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_1, 5k) - (BIDDER_1, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
		// | -------------------- 1USD ----------------------|---- 1.1 USD ---|---- 1.2 USD ----|----------- 1.3 USD -------------|
		// post wap ~ 1.0557252:
		// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

		let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

		let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);
		let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);

		let plmc_existential_amounts = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_amounts.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());

		inst.bid_for_users(project_id, bids.clone()).unwrap();

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
			UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
		]);
		inst.do_reserved_plmc_assertions(plmc_fundings.clone(), HoldReason::Participation(project_id).into());
		inst.do_bid_transferred_foreign_asset_assertions(usdt_fundings.clone(), project_id);

		inst.start_community_funding(project_id).unwrap();

		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let returned_auction_plmc =
			MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
		let returned_funding_assets =
			MockInstantiator::calculate_auction_funding_asset_returned_from_all_bids_made(&bids, project_metadata, wap);

		let expected_free_plmc = MockInstantiator::generic_map_operation(
			vec![returned_auction_plmc.clone(), plmc_existential_amounts],
			MergeOperation::Add,
		);
		let expected_free_funding_assets =
			MockInstantiator::generic_map_operation(vec![returned_funding_assets.clone()], MergeOperation::Add);
		dbg!(&expected_free_plmc);
		let expected_reserved_plmc = MockInstantiator::generic_map_operation(
			vec![plmc_fundings.clone(), returned_auction_plmc],
			MergeOperation::Subtract,
		);
		let expected_held_funding_assets = MockInstantiator::generic_map_operation(
			vec![usdt_fundings.clone(), returned_funding_assets],
			MergeOperation::Subtract,
		);

		inst.do_free_plmc_assertions(expected_free_plmc);

		inst.do_reserved_plmc_assertions(expected_reserved_plmc, HoldReason::Participation(project_id).into());

		inst.do_free_foreign_asset_assertions(expected_free_funding_assets);
		inst.do_bid_transferred_foreign_asset_assertions(expected_held_funding_assets, project_id);
	}

	#[test]
	fn cannot_start_auction_before_evaluation_finishes() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		inst.execute(|| {
			assert_noop!(
				PolimecFunding::do_english_auction(ISSUER, project_id),
				Error::<TestRuntime>::EvaluationPeriodNotEnded
			);
		});
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		let did = generate_did_from_account(ISSUER);
		let investor_type = InvestorType::Institutional;
		inst.execute(|| {
			assert_noop!(
				PolimecFunding::do_bid(
					&BIDDER_2,
					0,
					1,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					did,
					investor_type
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn cannot_bid_more_than_project_limit_count() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(100.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};
		let evaluations =
			MockInstantiator::generate_successful_evaluations(project_metadata.clone(), vec![EVALUATOR_1], vec![100u8]);
		let bids = (0u32..<TestRuntime as Config>::MaxBidsPerProject::get())
			.map(|i| (i as u32 + 420u32, 50 * ASSET_UNIT).into())
			.collect_vec();
		let failing_bid = BidParams::<TestRuntime>::new(BIDDER_1, 50 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER, evaluations);

		let plmc_for_bidding = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&bids.clone(),
			project_metadata.minimum_price,
		);
		let plmc_existential_deposits = bids.accounts().existential_deposits();
		let plmc_ct_account_deposits = bids.accounts().ct_account_deposits();
		let usdt_for_bidding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
			&bids.clone(),
			project_metadata.minimum_price,
		);

		inst.mint_plmc_to(plmc_for_bidding.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_bidding.clone());

		inst.bid_for_users(project_id, bids.clone()).unwrap();

		let plmc_for_failing_bid = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&vec![failing_bid.clone()],
			project_metadata.minimum_price,
		);
		let plmc_existential_deposits = plmc_for_failing_bid.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_for_failing_bid.accounts().ct_account_deposits();
		let usdt_for_bidding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
			&vec![failing_bid.clone()],
			project_metadata.minimum_price,
		);

		inst.mint_plmc_to(plmc_for_failing_bid.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_bidding.clone());

		assert_err!(inst.bid_for_users(project_id, vec![failing_bid]), Error::<TestRuntime>::TooManyBidsForProject);
	}

	#[test]
	fn contribute_does_not_work() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_evaluating_project(default_project_metadata(0, ISSUER), ISSUER);
		let did = generate_did_from_account(ISSUER);
		let investor_type = InvestorType::Retail;
		inst.execute(|| {
			assert_noop!(
				PolimecFunding::do_community_contribute(
					&BIDDER_1,
					project_id,
					100,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					did,
					investor_type
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn bid_with_asset_not_accepted() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id =
			inst.create_auctioning_project(default_project_metadata(0, ISSUER), ISSUER, default_evaluations());
		let bids = vec![BidParams::<TestRuntime>::new(BIDDER_1, 10_000, 1u8, AcceptedFundingAsset::USDC)];

		let did = generate_did_from_account(ISSUER);
		let investor_type = InvestorType::Institutional;

		let outcome = inst.execute(|| {
			Pallet::<TestRuntime>::do_bid(
				&bids[0].bidder,
				project_id,
				bids[0].amount,
				bids[0].multiplier,
				bids[0].asset,
				did,
				investor_type,
			)
		});
		frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
	}

	#[test]
	fn no_bids_made() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let project_id = inst.create_auctioning_project(project_metadata, issuer, evaluations);

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
	fn after_random_end_bid_gets_refunded() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER, default_evaluations());

		let (bid_in, bid_out) = (default_bids()[0].clone(), default_bids()[1].clone());

		let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&vec![bid_in.clone(), bid_out.clone()],
			project_metadata.minimum_price,
		);
		let plmc_existential_amounts = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();

		let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
			&vec![bid_in.clone(), bid_out.clone()],
			project_metadata.minimum_price,
		);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_amounts.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());

		inst.bid_for_users(project_id, vec![bid_in]).unwrap();
		inst.advance_time(
			<TestRuntime as Config>::EnglishAuctionDuration::get() +
				<TestRuntime as Config>::CandleAuctionDuration::get() -
				1,
		)
		.unwrap();

		inst.bid_for_users(project_id, vec![bid_out]).unwrap();

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
			UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
		]);
		inst.do_reserved_plmc_assertions(
			vec![
				UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount),
				UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount),
			],
			HoldReason::Participation(project_id).into(),
		);
		inst.do_bid_transferred_foreign_asset_assertions(
			vec![
				UserToForeignAssets::<TestRuntime>::new(
					BIDDER_1,
					usdt_fundings[0].asset_amount,
					AcceptedFundingAsset::USDT.to_assethub_id(),
				),
				UserToForeignAssets::<TestRuntime>::new(
					BIDDER_2,
					usdt_fundings[1].asset_amount,
					AcceptedFundingAsset::USDT.to_assethub_id(),
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
			HoldReason::Participation(project_id).into(),
		);

		inst.do_bid_transferred_foreign_asset_assertions(
			vec![
				UserToForeignAssets::<TestRuntime>::new(
					BIDDER_1,
					usdt_fundings[0].asset_amount,
					AcceptedFundingAsset::USDT.to_assethub_id(),
				),
				UserToForeignAssets::<TestRuntime>::new(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_assethub_id()),
			],
			project_id,
		);
	}

	#[test]
	fn auction_gets_percentage_of_ct_total_allocation() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let evaluations = default_evaluations();
		let auction_percentage = project_metadata.auction_round_allocation_percentage;
		let total_allocation = project_metadata.total_allocation_size;

		let auction_allocation = auction_percentage * total_allocation;

		let bids = vec![(BIDDER_1, auction_allocation).into()];
		let project_id =
			inst.create_community_contributing_project(project_metadata.clone(), ISSUER, evaluations.clone(), bids);
		let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
		let bid_info = inst.execute(|| bid_infos.next().unwrap());
		assert!(inst.execute(|| bid_infos.next().is_none()));
		assert_eq!(bid_info.final_ct_amount, auction_allocation);

		let project_metadata = default_project_metadata(1, ISSUER);
		let bids = vec![(BIDDER_1, auction_allocation).into(), (BIDDER_1, 1000 * ASSET_UNIT).into()];
		let project_id =
			inst.create_community_contributing_project(project_metadata.clone(), ISSUER, evaluations.clone(), bids);
		let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
		let bid_info_1 = inst.execute(|| bid_infos.next().unwrap());
		let bid_info_2 = inst.execute(|| bid_infos.next().unwrap());
		assert!(inst.execute(|| bid_infos.next().is_none()));
		assert_eq!(
			bid_info_1.final_ct_amount + bid_info_2.final_ct_amount,
			auction_allocation,
			"Should not be able to buy more than auction allocation"
		);
	}

	#[test]
	fn per_credential_type_ticket_size_minimums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};
		let evaluations = default_evaluations();

		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER, evaluations.clone());

		inst.mint_plmc_to(vec![(BIDDER_1, 50_000 * ASSET_UNIT).into(), (BIDDER_2, 50_000 * ASSET_UNIT).into()]);

		inst.mint_foreign_asset_to(vec![(BIDDER_1, 50_000 * US_DOLLAR).into(), (BIDDER_2, 50_000 * US_DOLLAR).into()]);

		// bid below 800 CT (8k USD) should fail for professionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_bid(
					&BIDDER_1,
					project_id,
					799 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BIDDER_1),
					InvestorType::Professional
				),
				Error::<TestRuntime>::BidTooLow
			);
		});
		// bid below 2000 CT (20k USD) should fail for institutionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_bid(
					&BIDDER_2,
					project_id,
					1999 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BIDDER_1),
					InvestorType::Institutional
				),
				Error::<TestRuntime>::BidTooLow
			);
		});
	}

	#[test]
	fn per_credential_type_ticket_size_maximums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(80u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), Some(100_000 * US_DOLLAR)),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), Some(500_000 * US_DOLLAR)),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
				professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
				institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};
		let evaluations = default_evaluations();

		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER, evaluations.clone());

		inst.mint_plmc_to(vec![
			(BIDDER_1, 500_000 * ASSET_UNIT).into(),
			(BIDDER_2, 500_000 * ASSET_UNIT).into(),
			(BIDDER_3, 500_000 * ASSET_UNIT).into(),
			(BIDDER_4, 500_000 * ASSET_UNIT).into(),
		]);

		inst.mint_foreign_asset_to(vec![
			(BIDDER_1, 500_000 * US_DOLLAR).into(),
			(BIDDER_2, 500_000 * US_DOLLAR).into(),
			(BIDDER_3, 500_000 * US_DOLLAR).into(),
			(BIDDER_4, 500_000 * US_DOLLAR).into(),
		]);

		let bidder_1_jwt = get_mock_jwt(BIDDER_1, InvestorType::Professional, generate_did_from_account(BIDDER_1));
		let bidder_2_jwt_same_did =
			get_mock_jwt(BIDDER_2, InvestorType::Professional, generate_did_from_account(BIDDER_1));
		// total bids with same DID above 10k CT (100k USD) should fail for professionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_1),
				bidder_1_jwt,
				project_id,
				8000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_2),
					bidder_2_jwt_same_did.clone(),
					project_id,
					3000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::BidTooHigh
			);
		});
		// bidding 10k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_2),
				bidder_2_jwt_same_did,
				project_id,
				2000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});

		let bidder_3_jwt = get_mock_jwt(BIDDER_3, InvestorType::Institutional, generate_did_from_account(BIDDER_3));
		let bidder_4_jwt_same_did =
			get_mock_jwt(BIDDER_4, InvestorType::Institutional, generate_did_from_account(BIDDER_3));
		// total bids with same DID above 50k CT (500k USD) should fail for institutionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_3),
				bidder_3_jwt,
				project_id,
				40_000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_4),
					bidder_4_jwt_same_did.clone(),
					project_id,
					11_000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				Error::<TestRuntime>::BidTooHigh
			);
		});
		// bidding 50k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_4),
				bidder_4_jwt_same_did,
				project_id,
				10_000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
	}

	#[test]
	fn bid_with_multiple_currencies() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let mut project_metadata_usdt = default_project_metadata(0, ISSUER);
		project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

		let mut project_metadata_all = default_project_metadata(1, ISSUER);
		project_metadata_all.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut project_metadata_usdc = default_project_metadata(2, ISSUER);
		project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

		let mut project_metadata_dot = default_project_metadata(3, ISSUER);
		project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

		let evaluations = default_evaluations();

		let projects = vec![
			TestProjectParams {
				expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
				metadata: project_metadata_all.clone(),
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
				metadata: project_metadata_usdt,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
				metadata: project_metadata_usdc,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
				metadata: project_metadata_dot,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
		];
		let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

		let project_id_all = project_ids[0];
		let project_id_usdt = project_ids[1];
		let project_id_usdc = project_ids[2];
		let project_id_dot = project_ids[3];

		let usdt_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let usdc_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
		let dot_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

		let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
			&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
			project_metadata_all.minimum_price,
		);
		let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();

		let plmc_all_mints = MockInstantiator::generic_map_operation(
			vec![plmc_fundings, plmc_existential_deposits, plmc_ct_account_deposits],
			MergeOperation::Add,
		);
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());

		let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
			&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
			project_metadata_all.minimum_price,
		);
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());

		assert_ok!(inst.bid_for_users(project_id_all, vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()]));

		assert_ok!(inst.bid_for_users(project_id_usdt, vec![usdt_bid.clone()]));
		assert_err!(
			inst.bid_for_users(project_id_usdt, vec![usdc_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.bid_for_users(project_id_usdt, vec![dot_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.bid_for_users(project_id_usdc, vec![usdt_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.bid_for_users(project_id_usdc, vec![usdc_bid.clone()]));
		assert_err!(
			inst.bid_for_users(project_id_usdc, vec![dot_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.bid_for_users(project_id_dot, vec![usdt_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.bid_for_users(project_id_dot, vec![usdc_bid.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.bid_for_users(project_id_dot, vec![dot_bid.clone()]));
	}
}

// only functionalities that happen in the COMMUNITY FUNDING period of a project
mod community_contribution {
	use super::*;
	pub const HOURS: BlockNumber = 300u64;

	#[test]
	fn community_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_remainder_contributing_project(
			default_project_metadata(0, ISSUER),
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
		let project1 = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let project2 = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let project3 = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let project4 = default_project_metadata(inst.get_new_nonce(), ISSUER);
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
		let metadata = default_project_metadata(0, ISSUER);
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let project_id = inst.create_community_contributing_project(metadata, issuer, evaluations, bids);

		const BOB: AccountId = 42;
		let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributions = vec![
			ContributionParams::new(BOB, 3 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			ContributionParams::new(BOB, 4 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
		let plmc_existential_deposit = plmc_funding.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_funding.accounts().ct_account_deposits();
		let foreign_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		inst.mint_plmc_to(plmc_funding);
		inst.mint_plmc_to(plmc_existential_deposit);
		inst.mint_plmc_to(plmc_ct_account_deposits);
		inst.mint_foreign_asset_to(foreign_funding);

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
			BidParams::new_with_defaults(BIDDER_1, 40_000 * ASSET_UNIT),
			BidParams::new_with_defaults(BIDDER_2, 10_000 * ASSET_UNIT),
		];
		let project_id = inst.create_community_contributing_project(
			default_project_metadata(0, ISSUER),
			ISSUER,
			default_evaluations(),
			bids,
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();
		let foreign_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();
		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
			BOB,
			0_u128,
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)]);
		inst.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], HoldReason::Participation(project_id).into());
		inst.do_contribution_transferred_foreign_asset_assertions(foreign_asset_fundings, project_id);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_overbuy() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let bids = vec![
			BidParams::new(BIDDER_1, 40_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let project_id = inst.create_community_contributing_project(
			default_project_metadata(0, ISSUER),
			ISSUER,
			default_evaluations(),
			bids,
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;

		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();
		let mut foreign_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).plmc_amount;
		let _remaining_plmc: BalanceOf<TestRuntime> =
			plmc_fundings.iter().fold(0_u128, |acc, item| acc + item.plmc_amount);

		let actual_funding_transferred = foreign_asset_fundings.swap_remove(0).asset_amount;
		let remaining_foreign_assets: BalanceOf<TestRuntime> =
			foreign_asset_fundings.iter().fold(0_u128, |acc, item| acc + item.asset_amount);

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
			BOB,
			remaining_foreign_assets,
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)]);
		inst.do_reserved_plmc_assertions(
			vec![UserToPLMCBalance::new(BOB, reserved_plmc)],
			HoldReason::Participation(project_id).into(),
		);
		inst.do_contribution_transferred_foreign_asset_assertions(
			vec![UserToForeignAssets::<TestRuntime>::new(
				BOB,
				actual_funding_transferred,
				AcceptedFundingAsset::USDT.to_assethub_id(),
			)],
			project_id,
		);
	}

	#[test]
	fn contribution_errors_if_limit_is_reached() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_community_contributing_project(
			default_project_metadata(0, ISSUER),
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
		let plmc_existential_deposits = plmc_funding.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_funding.accounts().ct_account_deposits();

		let foreign_funding =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		inst.mint_plmc_to(plmc_funding.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		inst.mint_foreign_asset_to(foreign_funding.clone());

		// Reach up to the limit of contributions for a user-project
		assert!(inst.contribute_for_users(project_id, contributions).is_ok());

		// Try to contribute again, but it should fail because the limit of contributions for a user-project was reached.
		let over_limit_contribution =
			ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT);
		assert!(inst.contribute_for_users(project_id, vec![over_limit_contribution]).is_err());

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let contributor_post_buy_plmc_balance =
			inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&CONTRIBUTOR));
		let contributor_post_buy_foreign_asset_balance =
			inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_FOREIGN_ID, CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, MockInstantiator::get_ed());
		assert_eq!(contributor_post_buy_foreign_asset_balance, 0);

		let plmc_bond_stored = inst.execute(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&HoldReason::Participation(project_id.into()).into(),
				&CONTRIBUTOR,
			)
		});
		let foreign_asset_contributions_stored = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, MockInstantiator::sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(
			foreign_asset_contributions_stored,
			MockInstantiator::sum_foreign_mappings(vec![foreign_funding.clone()])
		);
	}

	#[test]
	fn retail_contributor_was_evaluator() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
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
		inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

		inst.contribute_for_users(project_id, vec![contribution]).unwrap();
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let bids = default_bids();

		let evaluator_contributor = 69;
		let evaluation_usd_amount = 400 * US_DOLLAR;
		let contribution_ct_amount =
			project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(evaluation_usd_amount) -
				1 * ASSET_UNIT;

		let evaluation: UserToUSDBalance<TestRuntime> = (evaluator_contributor, evaluation_usd_amount).into();
		let contribution: ContributionParams<TestRuntime> = (evaluator_contributor, contribution_ct_amount).into();

		evaluations.push(evaluation.clone());

		let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);

		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let plmc_evaluation_amount =
			MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()])[0].plmc_amount;
		let plmc_contribution_amount =
			MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;

		let evaluation_plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;

		assert!(
			plmc_contribution_amount > evaluation_plmc_available_for_participating,
			"contribution should want to use slash reserve"
		);

		assert!(
			plmc_contribution_amount < plmc_evaluation_amount,
			"contribution should want to succeed by just using the slash reserve"
		);

		let necessary_usdt_for_contribution =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);
		inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

		assert_matches!(inst.contribute_for_users(project_id, vec![contribution]), Err(_));
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_success() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let bids = default_bids();

		let evaluator_contributor = 69;
		let evaluation_usd_amount = 400 * US_DOLLAR;

		// We want to contribute in PLMC, less than what we used for evaluating, but more than what we have due to slash reserve
		let contribution_ct_amount =
			project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(evaluation_usd_amount) -
				1 * ASSET_UNIT;

		let evaluation: UserToUSDBalance<TestRuntime> = (evaluator_contributor, evaluation_usd_amount).into();
		let contribution: ContributionParams<TestRuntime> = (evaluator_contributor, contribution_ct_amount).into();

		evaluations.push(evaluation.clone());

		let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);

		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let plmc_evaluation_amount = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation])[0].plmc_amount;
		let plmc_contribution_amount =
			MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;

		let evaluation_plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;

		assert!(
			plmc_contribution_amount > evaluation_plmc_available_for_participating,
			"contribution should want to use slash reserve"
		);

		assert!(
			plmc_contribution_amount < plmc_evaluation_amount,
			"contribution should want to succeed by just using the slash reserve"
		);

		let necessary_usdt_for_contribution =
			MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

		// we mint what we would have taken from the reserve, to try and make the call pass
		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			evaluator_contributor,
			plmc_contribution_amount - evaluation_plmc_available_for_participating,
		)]);
		inst.mint_foreign_asset_to(necessary_usdt_for_contribution);
		inst.contribute_for_users(project_id, vec![contribution]).unwrap();

		let evaluation_locked = inst
			.get_reserved_plmc_balances_for(vec![evaluator_contributor], HoldReason::Evaluation(project_id).into())[0]
			.plmc_amount;
		let participation_locked = inst
			.get_reserved_plmc_balances_for(vec![evaluator_contributor], HoldReason::Participation(project_id).into())[0]
			.plmc_amount;

		assert_eq!(evaluation_locked, <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount);
		assert_eq!(participation_locked, plmc_contribution_amount);
	}

	#[test]
	fn round_has_total_ct_allocation_minus_auction_sold() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let evaluations = default_evaluations();
		let bids = default_bids();

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER,
			evaluations.clone(),
			bids.clone(),
		);
		let project_details = inst.get_project_details(project_id);
		let bid_ct_sold: BalanceOf<TestRuntime> = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.fold(Zero::zero(), |acc, bid| acc + bid.final_ct_amount)
		});
		assert_eq!(project_details.remaining_contribution_tokens, project_metadata.total_allocation_size - bid_ct_sold);

		let contributions = vec![(BUYER_1, project_details.remaining_contribution_tokens).into()];

		let plmc_contribution_funding = MockInstantiator::calculate_contributed_plmc_spent(
			contributions.clone(),
			project_details.weighted_average_price.unwrap(),
		);
		let plmc_existential_deposits = plmc_contribution_funding.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_contribution_funding.accounts().ct_account_deposits();
		inst.mint_plmc_to(plmc_contribution_funding.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		let foreign_asset_contribution_funding = MockInstantiator::calculate_contributed_funding_asset_spent(
			contributions.clone(),
			project_details.weighted_average_price.unwrap(),
		);
		inst.mint_foreign_asset_to(foreign_asset_contribution_funding.clone());

		inst.contribute_for_users(project_id, contributions).unwrap();

		assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);
	}

	#[test]
	fn per_credential_type_ticket_size_minimums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(Some(10 * US_DOLLAR), None),
				professional: TicketSize::new(Some(100_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(200_000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);

		inst.mint_plmc_to(vec![
			(BUYER_1, 50_000 * ASSET_UNIT).into(),
			(BUYER_2, 50_000 * ASSET_UNIT).into(),
			(BUYER_3, 50_000 * ASSET_UNIT).into(),
		]);

		inst.mint_foreign_asset_to(vec![
			(BUYER_1, 50_000 * US_DOLLAR).into(),
			(BUYER_2, 50_000 * US_DOLLAR).into(),
			(BUYER_3, 50_000 * US_DOLLAR).into(),
		]);

		// contribution below 1 CT (10 USD) should fail for retail
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_community_contribute(
					&BUYER_1,
					project_id,
					ASSET_UNIT / 2,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_1),
					InvestorType::Retail
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});
		// contribution below 10_000 CT (100k USD) should fail for professionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_community_contribute(
					&BUYER_2,
					project_id,
					9_999,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_2),
					InvestorType::Professional
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});

		// contribution below 20_000 CT (200k USD) should fail for professionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_community_contribute(
					&BUYER_2,
					project_id,
					19_999,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_2),
					InvestorType::Institutional
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});
	}

	#[test]
	fn per_credential_type_ticket_size_maximums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
				professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
				institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);

		inst.mint_plmc_to(vec![
			(BUYER_1, 500_000 * ASSET_UNIT).into(),
			(BUYER_2, 500_000 * ASSET_UNIT).into(),
			(BUYER_3, 500_000 * ASSET_UNIT).into(),
			(BUYER_4, 500_000 * ASSET_UNIT).into(),
			(BUYER_5, 500_000 * ASSET_UNIT).into(),
			(BUYER_6, 500_000 * ASSET_UNIT).into(),
		]);

		inst.mint_foreign_asset_to(vec![
			(BUYER_1, 500_000 * US_DOLLAR).into(),
			(BUYER_2, 500_000 * US_DOLLAR).into(),
			(BUYER_3, 500_000 * US_DOLLAR).into(),
			(BUYER_4, 500_000 * US_DOLLAR).into(),
			(BUYER_5, 500_000 * US_DOLLAR).into(),
			(BUYER_6, 500_000 * US_DOLLAR).into(),
		]);

		let buyer_1_jwt = get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1));
		let buyer_2_jwt_same_did = get_mock_jwt(BUYER_2, InvestorType::Retail, generate_did_from_account(BUYER_1));
		// total contributions with same DID above 10k CT (100k USD) should fail for retail
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_1),
				buyer_1_jwt,
				project_id,
				9000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_2),
					buyer_2_jwt_same_did.clone(),
					project_id,
					1001 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 2k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_2),
				buyer_2_jwt_same_did,
				project_id,
				1000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});

		let buyer_3_jwt = get_mock_jwt(BUYER_3, InvestorType::Professional, generate_did_from_account(BUYER_3));
		let buyer_4_jwt_same_did =
			get_mock_jwt(BUYER_4, InvestorType::Professional, generate_did_from_account(BUYER_3));
		// total contributions with same DID above 2k CT (20k USD) should fail for professionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_3),
				buyer_3_jwt,
				project_id,
				1800 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_4),
					buyer_4_jwt_same_did.clone(),
					project_id,
					201 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 2k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_4),
				buyer_4_jwt_same_did,
				project_id,
				200 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});

		let buyer_5_jwt = get_mock_jwt(BUYER_5, InvestorType::Institutional, generate_did_from_account(BUYER_5));
		let buyer_6_jwt_same_did =
			get_mock_jwt(BUYER_6, InvestorType::Institutional, generate_did_from_account(BUYER_5));
		// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_5),
				buyer_5_jwt,
				project_id,
				4690 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_6),
					buyer_6_jwt_same_did.clone(),
					project_id,
					311 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 5k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_6),
				buyer_6_jwt_same_did,
				project_id,
				310 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			));
		});
	}

	#[test]
	fn contribute_with_multiple_currencies() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let mut project_metadata_usdt = default_project_metadata(0, ISSUER);
		project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

		let mut project_metadata_all = default_project_metadata(1, ISSUER);
		project_metadata_all.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut project_metadata_usdc = default_project_metadata(2, ISSUER);
		project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

		let mut project_metadata_dot = default_project_metadata(3, ISSUER);
		project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

		let evaluations = default_evaluations();

		let usdt_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::USDT;
				b
			})
			.collect::<Vec<_>>();

		let usdc_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::USDC;
				b
			})
			.collect::<Vec<_>>();

		let dot_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::DOT;
				b
			})
			.collect::<Vec<_>>();

		let projects = vec![
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: project_metadata_all.clone(),
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdt_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: project_metadata_usdt,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdt_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: project_metadata_usdc,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdc_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: project_metadata_dot,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: dot_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
		];
		let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

		let project_id_all = project_ids[0];
		let project_id_usdt = project_ids[1];
		let project_id_usdc = project_ids[2];
		let project_id_dot = project_ids[3];

		let usdt_contribution = ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let usdc_contribution = ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
		let dot_contribution = ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

		let wap = inst.get_project_details(project_id_all).weighted_average_price.unwrap();

		let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
			wap,
		);
		let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();

		let plmc_all_mints = MockInstantiator::generic_map_operation(
			vec![plmc_fundings, plmc_existential_deposits, plmc_ct_account_deposits],
			MergeOperation::Add,
		);
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());

		let usdt_fundings = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
			wap,
		);
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());

		assert_ok!(inst.contribute_for_users(
			project_id_all,
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()]
		));

		assert_ok!(inst.contribute_for_users(project_id_usdt, vec![usdt_contribution.clone()]));
		assert_err!(
			inst.contribute_for_users(project_id_usdt, vec![usdc_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.contribute_for_users(project_id_usdt, vec![dot_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.contribute_for_users(project_id_usdc, vec![usdt_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.contribute_for_users(project_id_usdc, vec![usdc_contribution.clone()]));
		assert_err!(
			inst.contribute_for_users(project_id_usdc, vec![dot_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.contribute_for_users(project_id_dot, vec![usdt_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.contribute_for_users(project_id_dot, vec![usdc_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.contribute_for_users(project_id_dot, vec![dot_contribution.clone()]));
	}
}

// only functionalities that happen in the REMAINDER FUNDING period of a project
mod remainder_contribution {
	use super::*;
	use crate::instantiator::async_features::create_multiple_projects_at;

	#[test]
	fn remainder_round_works() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let _ = inst.create_finished_project(
			default_project_metadata(inst.get_new_nonce(), ISSUER),
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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let mut evaluations = default_evaluations();
		let community_contributions = default_community_buys();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let remainder_contribution =
			ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer,
			evaluations,
			bids,
			community_contributions,
		);
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
		inst.mint_foreign_asset_to(necessary_usdt_for_buy);

		inst.contribute_for_users(project_id, vec![remainder_contribution]).unwrap();
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_exact() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_remainder_contributing_project(
			default_project_metadata(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let plmc_existential_deposits = contributions.accounts().existential_deposits();
		let plmc_ct_account_deposits = contributions.accounts().ct_account_deposits();
		let foreign_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
			BOB,
			0_u128,
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)]);
		inst.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], HoldReason::Participation(project_id).into());
		inst.do_contribution_transferred_foreign_asset_assertions(foreign_asset_fundings, project_id);
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_overbuy() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_remainder_contributing_project(
			default_project_metadata(0, ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
		const BOB: AccountId = 808;

		let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;

		let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

		let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let plmc_existential_deposits = contributions.accounts().existential_deposits();
		let plmc_ct_account_deposits = contributions.accounts().ct_account_deposits();
		let mut foreign_asset_fundings =
			MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		inst.mint_plmc_to(plmc_fundings.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

		// Buy remaining CTs
		inst.contribute_for_users(project_id, contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		inst.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			inst.get_project_details(project_id).remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).plmc_amount;
		let _remaining_plmc: BalanceOf<TestRuntime> =
			plmc_fundings.iter().fold(Zero::zero(), |acc, item| item.plmc_amount + acc);

		let actual_funding_transferred = foreign_asset_fundings.swap_remove(0).asset_amount;
		let remaining_foreign_assets: BalanceOf<TestRuntime> =
			foreign_asset_fundings.iter().fold(Zero::zero(), |acc, item| item.asset_amount + acc);

		inst.do_free_plmc_assertions(plmc_existential_deposits);
		inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
			BOB,
			remaining_foreign_assets,
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)]);
		inst.do_reserved_plmc_assertions(
			vec![UserToPLMCBalance::new(BOB, reserved_plmc)],
			HoldReason::Participation(project_id).into(),
		);
		inst.do_contribution_transferred_foreign_asset_assertions(
			vec![UserToForeignAssets::new(
				BOB,
				actual_funding_transferred,
				AcceptedFundingAsset::USDT.to_assethub_id(),
			)],
			project_id,
		);
	}

	#[test]
	fn round_has_total_ct_allocation_minus_auction_sold() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let evaluations = default_evaluations();
		let bids = default_bids();

		let project_id = inst.create_remainder_contributing_project(
			project_metadata.clone(),
			ISSUER,
			evaluations.clone(),
			bids.clone(),
			vec![],
		);
		let project_details = inst.get_project_details(project_id);
		let bid_ct_sold: BalanceOf<TestRuntime> = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.fold(Zero::zero(), |acc, bid| acc + bid.final_ct_amount)
		});
		assert_eq!(project_details.remaining_contribution_tokens, project_metadata.total_allocation_size - bid_ct_sold);

		let contributions = vec![(BUYER_1, project_details.remaining_contribution_tokens).into()];

		let plmc_contribution_funding = MockInstantiator::calculate_contributed_plmc_spent(
			contributions.clone(),
			project_details.weighted_average_price.unwrap(),
		);
		let plmc_existential_deposits = plmc_contribution_funding.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_contribution_funding.accounts().ct_account_deposits();
		inst.mint_plmc_to(plmc_contribution_funding.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		let foreign_asset_contribution_funding = MockInstantiator::calculate_contributed_funding_asset_spent(
			contributions.clone(),
			project_details.weighted_average_price.unwrap(),
		);
		inst.mint_foreign_asset_to(foreign_asset_contribution_funding.clone());

		inst.contribute_for_users(project_id, contributions).unwrap();

		assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);
	}

	#[test]
	fn per_credential_type_ticket_size_minimums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(8000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(Some(10 * US_DOLLAR), None),
				professional: TicketSize::new(Some(100_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(200_000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let project_id = inst.create_remainder_contributing_project(
			project_metadata.clone(),
			ISSUER,
			default_evaluations(),
			default_bids(),
			vec![],
		);

		inst.mint_plmc_to(vec![
			(BUYER_4, 50_000 * ASSET_UNIT).into(),
			(BUYER_5, 50_000 * ASSET_UNIT).into(),
			(BUYER_6, 50_000 * ASSET_UNIT).into(),
		]);

		inst.mint_foreign_asset_to(vec![
			(BUYER_4, 50_000 * US_DOLLAR).into(),
			(BUYER_5, 50_000 * US_DOLLAR).into(),
			(BUYER_6, 50_000 * US_DOLLAR).into(),
		]);

		// contribution below 1 CT (10 USD) should fail for retail
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_4,
					project_id,
					ASSET_UNIT / 2,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_4),
					InvestorType::Retail
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});
		// contribution below 10_000 CT (100k USD) should fail for professionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_5,
					project_id,
					9_999,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_5),
					InvestorType::Professional
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});

		// contribution below 20_000 CT (200k USD) should fail for professionals
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_6,
					project_id,
					19_999,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_6),
					InvestorType::Institutional
				),
				Error::<TestRuntime>::ContributionTooLow
			);
		});
	}

	#[test]
	fn per_credential_type_ticket_size_maximums() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, Some(300_000 * US_DOLLAR)),
				professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
				institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};

		let project_id = inst.create_remainder_contributing_project(
			project_metadata.clone(),
			ISSUER,
			default_evaluations(),
			default_bids(),
			vec![],
		);

		inst.mint_plmc_to(vec![
			(BUYER_4, 500_000 * ASSET_UNIT).into(),
			(BUYER_5, 500_000 * ASSET_UNIT).into(),
			(BUYER_6, 500_000 * ASSET_UNIT).into(),
			(BUYER_7, 500_000 * ASSET_UNIT).into(),
			(BUYER_8, 500_000 * ASSET_UNIT).into(),
			(BUYER_9, 500_000 * ASSET_UNIT).into(),
		]);

		inst.mint_foreign_asset_to(vec![
			(BUYER_4, 500_000 * US_DOLLAR).into(),
			(BUYER_5, 500_000 * US_DOLLAR).into(),
			(BUYER_6, 500_000 * US_DOLLAR).into(),
			(BUYER_7, 500_000 * US_DOLLAR).into(),
			(BUYER_8, 500_000 * US_DOLLAR).into(),
			(BUYER_9, 500_000 * US_DOLLAR).into(),
		]);

		// total contributions with same DID above 30k CT (300k USD) should fail for retail
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_4,
				project_id,
				28_000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				generate_did_from_account(BUYER_4),
				InvestorType::Retail
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_5,
					project_id,
					2001 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_4),
					InvestorType::Retail
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 2k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_5,
				project_id,
				2000 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				// note we use the same did as bidder 1, on a different account
				generate_did_from_account(BUYER_4),
				InvestorType::Retail
			));
		});

		// total contributions with same DID above 2k CT (20k USD) should fail for professionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_6,
				project_id,
				1800 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				generate_did_from_account(BUYER_6),
				InvestorType::Professional
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_7,
					project_id,
					201 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_6),
					InvestorType::Professional
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 2k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_7,
				project_id,
				200 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				// note we use the same did as bidder 1, on a different account
				generate_did_from_account(BUYER_6),
				InvestorType::Professional
			));
		});

		// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_8,
				project_id,
				4690 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				generate_did_from_account(BUYER_8),
				InvestorType::Institutional
			));
		});
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_9,
					project_id,
					311 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 3, on a different account
					generate_did_from_account(BUYER_8),
					InvestorType::Institutional
				),
				Error::<TestRuntime>::ContributionTooHigh
			);
		});
		// bidding 5k total works
		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
				&BUYER_9,
				project_id,
				310 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
				// note we use the same did as bidder 3, on a different account
				generate_did_from_account(BUYER_8),
				InvestorType::Institutional
			));
		});
	}

	#[test]
	fn contribute_with_multiple_currencies() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let mut project_metadata_usdt = default_project_metadata(0, ISSUER);
		project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

		let mut project_metadata_all = default_project_metadata(1, ISSUER);
		project_metadata_all.participation_currencies =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

		let mut project_metadata_usdc = default_project_metadata(2, ISSUER);
		project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

		let mut project_metadata_dot = default_project_metadata(3, ISSUER);
		project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

		let evaluations = default_evaluations();

		let usdt_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::USDT;
				b
			})
			.collect::<Vec<_>>();

		let usdc_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::USDC;
				b
			})
			.collect::<Vec<_>>();

		let dot_bids = default_bids()
			.into_iter()
			.map(|mut b| {
				b.asset = AcceptedFundingAsset::DOT;
				b
			})
			.collect::<Vec<_>>();

		let projects = vec![
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: project_metadata_all.clone(),
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdt_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: project_metadata_usdt,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdt_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: project_metadata_usdc,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: usdc_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: project_metadata_dot,
				issuer: ISSUER,
				evaluations: evaluations.clone(),
				bids: dot_bids.clone(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
		];
		let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

		let project_id_all = project_ids[0];
		let project_id_usdt = project_ids[1];
		let project_id_usdc = project_ids[2];
		let project_id_dot = project_ids[3];

		let usdt_contribution = ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		let usdc_contribution = ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
		let dot_contribution = ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

		let wap = inst.get_project_details(project_id_all).weighted_average_price.unwrap();

		let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
			wap,
		);
		let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
		let plmc_ct_account_deposits = plmc_fundings.accounts().ct_account_deposits();

		let plmc_all_mints = MockInstantiator::generic_map_operation(
			vec![plmc_fundings, plmc_existential_deposits, plmc_ct_account_deposits],
			MergeOperation::Add,
		);
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());
		inst.mint_plmc_to(plmc_all_mints.clone());

		let usdt_fundings = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
			wap,
		);
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());
		inst.mint_foreign_asset_to(usdt_fundings.clone());

		assert_ok!(inst.contribute_for_users(
			project_id_all,
			vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()]
		));

		assert_ok!(inst.contribute_for_users(project_id_usdt, vec![usdt_contribution.clone()]));
		assert_err!(
			inst.contribute_for_users(project_id_usdt, vec![usdc_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.contribute_for_users(project_id_usdt, vec![dot_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.contribute_for_users(project_id_usdc, vec![usdt_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.contribute_for_users(project_id_usdc, vec![usdc_contribution.clone()]));
		assert_err!(
			inst.contribute_for_users(project_id_usdc, vec![dot_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);

		assert_err!(
			inst.contribute_for_users(project_id_dot, vec![usdt_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_err!(
			inst.contribute_for_users(project_id_dot, vec![usdc_contribution.clone()]),
			Error::<TestRuntime>::FundingAssetNotAccepted
		);
		assert_ok!(inst.contribute_for_users(project_id_dot, vec![dot_contribution.clone()]));
	}
}

// only functionalities that happen after the REMAINDER FUNDING period of a project, and before the CT Migration
mod funding_end {
	use super::*;

	#[test]
	fn automatic_fail_less_eq_33_percent() {
		for funding_percent in (1..=33).step_by(5) {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids = MockInstantiator::generate_bids_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				vec![100u8],
				vec![BIDDER_1],
				vec![10u8],
			);
			let contributions = MockInstantiator::generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_community_contributors(),
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
			let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
				default_community_contributors(),
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
			let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
				default_community_contributors(),
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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
			default_community_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let project_id = project_id;
		inst.execute(|| {
			PolimecFunding::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::AcceptFunding)
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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
			default_community_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let project_id = project_id;
		inst.execute(|| {
			PolimecFunding::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::RejectFunding)
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
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
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
			default_community_contributors(),
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
			.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			inst,
			PolimecFunding::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::AcceptFunding)
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

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_funding_rejected() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = project_from_funding_reached(&mut inst, 56u64);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		let old_evaluation_locked_plmc = inst
			.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<UserToPLMCBalance<_>>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			inst,
			PolimecFunding::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::RejectFunding)
		);
		inst.advance_time(1u64).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
		let mut expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
			vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
			MergeOperation::Add,
		);
		let ct_deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(project_id);
		expected_evaluator_free_balances
			.iter_mut()
			.for_each(|UserToPLMCBalance { plmc_amount, .. }| *plmc_amount += ct_deposit_required);

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_failed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = project_from_funding_reached(&mut inst, 24u64);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

		let old_evaluation_locked_plmc = inst
			.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
			.into_iter()
			.filter(|item| item.plmc_amount > Zero::zero())
			.collect::<Vec<_>>();

		let evaluators = old_evaluation_locked_plmc.accounts();

		let old_participation_locked_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
		let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
		let mut expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
			vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
			MergeOperation::Add,
		);
		let ct_deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(project_id);
		expected_evaluator_free_balances
			.iter_mut()
			.for_each(|UserToPLMCBalance { plmc_amount, .. }| *plmc_amount += ct_deposit_required);

		let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn ct_minted_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10u64).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluators = evaluations.accounts();
		let evaluator_ct_amounts = evaluators
			.iter()
			.map(|account| {
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_evaluator_ct_rewarded = evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

				(account, total_evaluator_ct_rewarded)
			})
			.collect_vec();

		let bidders = bids.accounts();
		let bidder_ct_amounts = bidders
			.iter()
			.map(|account| {
				let bids = inst.execute(|| {
					Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

				(account, total_bidder_ct_rewarded)
			})
			.collect_vec();

		let community_accounts = community_contributions.accounts();
		let remainder_accounts = remainder_contributions.accounts();
		let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
		let contributor_ct_amounts = all_contributors
			.map(|account| {
				let contributions = inst.execute(|| {
					Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_contributor_ct_rewarded =
					contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

				(account, total_contributor_ct_rewarded)
			})
			.collect_vec();

		let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
			vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
			|item| item.0,
			Zero::zero(),
			|item, accumulator| accumulator + item.1,
		);

		for (account, amount) in all_ct_expectations {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		// do_end_funding
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let evaluators = evaluations.accounts();
		let evaluator_ct_amounts = evaluators
			.iter()
			.map(|account| {
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for evaluation in evaluations.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::evaluation_reward_payout_for(
							RuntimeOrigin::signed(evaluation.evaluator),
							project_id,
							evaluation.evaluator,
							evaluation.id,
						));
					});
				}
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_evaluator_ct_rewarded = evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

				(account, total_evaluator_ct_rewarded)
			})
			.collect_vec();

		let bidders = bids.accounts();
		let bidder_ct_amounts = bidders
			.iter()
			.map(|account| {
				let bids = inst.execute(|| {
					Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for bid in bids.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::bid_ct_mint_for(
							RuntimeOrigin::signed(bid.bidder),
							project_id,
							bid.bidder,
							bid.id,
						));
					});
				}

				let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

				(account, total_bidder_ct_rewarded)
			})
			.collect_vec();

		let community_accounts = community_contributions.accounts();
		let remainder_accounts = remainder_contributions.accounts();
		let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
		let contributor_ct_amounts = all_contributors
			.map(|account| {
				let contributions = inst.execute(|| {
					Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for contribution in contributions.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::contribution_ct_mint_for(
							RuntimeOrigin::signed(contribution.contributor),
							project_id,
							contribution.contributor,
							contribution.id,
						));
					});
				}

				let total_contributor_ct_rewarded =
					contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

				(account, total_contributor_ct_rewarded)
			})
			.collect_vec();

		let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
			vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
			|item| item.0,
			Zero::zero(),
			|item, accumulator| accumulator + item.1,
		);

		for (account, amount) in all_ct_expectations {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
			assert_eq!(minted, amount, "Account: {}", account);
		}

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	}

	#[test]
	fn cannot_mint_ct_twice_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		// do_end_funding
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let evaluators = evaluations.accounts();
		let evaluator_ct_amounts = evaluators
			.iter()
			.map(|account| {
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for evaluation in evaluations.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::evaluation_reward_payout_for(
							RuntimeOrigin::signed(evaluation.evaluator),
							project_id,
							evaluation.evaluator,
							evaluation.id,
						));
						assert_noop!(
							Pallet::<TestRuntime>::evaluation_reward_payout_for(
								RuntimeOrigin::signed(evaluation.evaluator),
								project_id,
								evaluation.evaluator,
								evaluation.id,
							),
							Error::<TestRuntime>::NotAllowed
						);
					});
				}
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_evaluator_ct_rewarded = evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

				(account, total_evaluator_ct_rewarded)
			})
			.collect_vec();

		let bidders = bids.accounts();
		let bidder_ct_amounts = bidders
			.iter()
			.map(|account| {
				let bids = inst.execute(|| {
					Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for bid in bids.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::bid_ct_mint_for(
							RuntimeOrigin::signed(bid.bidder),
							project_id,
							bid.bidder,
							bid.id,
						));
					});
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
					});
				}

				let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

				(account, total_bidder_ct_rewarded)
			})
			.collect_vec();

		let community_accounts = community_contributions.accounts();
		let remainder_accounts = remainder_contributions.accounts();
		let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
		let contributor_ct_amounts = all_contributors
			.map(|account| {
				let contributions = inst.execute(|| {
					Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for contribution in contributions.iter() {
					inst.execute(|| {
						assert_ok!(Pallet::<TestRuntime>::contribution_ct_mint_for(
							RuntimeOrigin::signed(contribution.contributor),
							project_id,
							contribution.contributor,
							contribution.id,
						));
					});
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

				let total_contributor_ct_rewarded =
					contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

				(account, total_contributor_ct_rewarded)
			})
			.collect_vec();

		let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
			vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
			|item| item.0,
			Zero::zero(),
			|item, accumulator| accumulator + item.1,
		);

		for (account, amount) in all_ct_expectations {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
			assert_eq!(minted, amount, "Account: {}", account);
		}

		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	}

	#[test]
	fn cannot_mint_ct_manually_after_automatic_mint() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let details = inst.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		inst.advance_time(1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluators = evaluations.accounts();
		let evaluator_ct_amounts = evaluators
			.iter()
			.map(|account| {
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for evaluation in evaluations.iter() {
					inst.execute(|| {
						assert_noop!(
							Pallet::<TestRuntime>::evaluation_reward_payout_for(
								RuntimeOrigin::signed(evaluation.evaluator),
								project_id,
								evaluation.evaluator,
								evaluation.id,
							),
							Error::<TestRuntime>::NotAllowed
						);
					});
				}
				let evaluations = inst.execute(|| {
					Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				let total_evaluator_ct_rewarded = evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

				(account, total_evaluator_ct_rewarded)
			})
			.collect_vec();

		let bidders = bids.accounts();
		let bidder_ct_amounts = bidders
			.iter()
			.map(|account| {
				let bids = inst.execute(|| {
					Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for bid in bids.iter() {
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
					});
				}

				let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

				(account, total_bidder_ct_rewarded)
			})
			.collect_vec();

		let community_accounts = community_contributions.accounts();
		let remainder_accounts = remainder_contributions.accounts();
		let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
		let contributor_ct_amounts = all_contributors
			.map(|account| {
				let contributions = inst.execute(|| {
					Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
				});
				for contribution in contributions.iter() {
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

				let total_contributor_ct_rewarded =
					contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

				(account, total_contributor_ct_rewarded)
			})
			.collect_vec();

		let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
			vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
			|item| item.0,
			Zero::zero(),
			|item, accumulator| accumulator + item.1,
		);

		for (account, amount) in all_ct_expectations {
			let minted =
				inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
			assert_eq!(minted, amount, "Account: {}", account);
		}
	}

	#[test]
	fn multiplier_gets_correct_vesting_duration() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = vec![
			BidParams::new(BIDDER_1, 325_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_2, 75_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
			BidParams::new(BIDDER_3, 50_000 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
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

	#[test]
	fn funding_assets_are_paid_manually_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
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
					UserToForeignAssets::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_assethub_id(),
					)
				})
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
		});
		let final_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToForeignAssets::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_assethub_id(),
					)
				})
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
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
			inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
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
	fn funding_assets_are_paid_automatically_to_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
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
					UserToForeignAssets::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_assethub_id(),
					)
				})
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
		});
		let final_contribution_payouts = inst.execute(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id,))
				.map(|contribution| {
					UserToForeignAssets::new(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_assethub_id(),
					)
				})
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.asset_amount)
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let prev_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
			final_bid_payouts[0].asset_id,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.asset_amount;

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		inst.advance_time(1u64).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let post_issuer_funding_balance =
			inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

		let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
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
	fn funding_assets_are_released_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);

		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(auction_allocation),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
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
					UserToForeignAssets::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_assethub_id(),
					)
				})
				.sorted_by_key(|bid| bid.account)
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
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
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let all_participants = all_expected_payouts.accounts();
		let prev_participants_funding_balances =
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

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
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_participants_funding_balances =
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
		let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
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
	fn funding_assets_are_released_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(auction_allocation),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
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
					UserToForeignAssets::<TestRuntime>::new(
						bid.bidder,
						bid.funding_asset_amount_locked,
						bid.funding_asset.to_assethub_id(),
					)
				})
				.sorted_by_key(|item| item.account)
				.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
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
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let all_participants = all_expected_payouts.accounts();
		let prev_participants_funding_balances =
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

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
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
		let post_participants_funding_balances =
			inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
		let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
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
	fn plmc_bonded_is_returned_automatically_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(auction_allocation),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let expected_evaluators_and_contributors_payouts =
			MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				evaluations.clone(),
				remainder_contributions.clone(),
				final_price,
				true,
			);
		let expected_bid_payouts =
			MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, final_price);
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_evaluators_and_contributors_payouts.clone(),
				expected_bid_payouts,
				expected_community_contribution_payouts,
			],
			MergeOperation::Add,
		);
		println!("all expected payouts {:?}", all_expected_payouts);
		for payout in all_expected_payouts.clone() {
			let evaluation_hold = inst.execute(|| {
				<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
					&HoldReason::Evaluation(project_id).into(),
					&payout.account,
				)
			});
			let participation_hold = inst.execute(|| {
				<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
					&HoldReason::Participation(project_id).into(),
					&payout.account,
				)
			});
			println!("account {:?} has evaluation hold {:?}", payout.account, evaluation_hold);
			println!("account {:?} has participation hold {:?}", payout.account, participation_hold);
		}
		let deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(project_id);
		let all_expected_payouts = all_expected_payouts
			.into_iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| {
				UserToPLMCBalance::new(account, plmc_amount + deposit_required)
			})
			.collect::<Vec<_>>();

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

		let participants = all_participants_plmc_deltas.accounts();
		for participant in participants {
			let future_deposit_reserved = inst.execute(||{<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), &participant)});
			println!("participant {:?} has future deposit reserved {:?}", participant, future_deposit_reserved);
		}
		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_participants_plmc_deltas, all_expected_payouts);
	}

	#[test]
	fn plmc_bonded_is_returned_manually_on_funding_fail() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(auction_allocation),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			project_metadata.minimum_price.saturating_mul_int(
				Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
			),
			project_metadata.minimum_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let expected_evaluators_and_contributors_payouts =
			MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				evaluations.clone(),
				remainder_contributions.clone(),
				final_price,
				true,
			);
		let expected_bid_payouts =
			MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, final_price);
		let expected_community_contribution_payouts =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		let all_expected_payouts = MockInstantiator::generic_map_operation(
			vec![
				expected_evaluators_and_contributors_payouts.clone(),
				expected_bid_payouts,
				expected_community_contribution_payouts,
			],
			MergeOperation::Add,
		);
		println!("all expected payouts {:?}", all_expected_payouts);
		for payout in all_expected_payouts.clone() {
			let evaluation_hold = inst.execute(|| {
				<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
					&HoldReason::Evaluation(project_id).into(),
					&payout.account,
				)
			});
			let participation_hold = inst.execute(|| {
				<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
					&HoldReason::Participation(project_id).into(),
					&payout.account,
				)
			});
			println!("account {:?} has evaluation hold {:?}", payout.account, evaluation_hold);
			println!("account {:?} has participation hold {:?}", payout.account, participation_hold);
		}
		let _deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(project_id);

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
		let participants = all_participants_plmc_deltas.accounts();
		for participant in participants {
			let future_deposit_reserved = inst.execute(||{<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), &participant)});
			println!("participant {:?} has future deposit reserved {:?}", participant, future_deposit_reserved);
		}
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);
		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_participants_plmc_deltas, all_expected_payouts);
	}

	// i.e consumer increase bug fixed with touch on pallet-assets
	#[test]
	fn no_limit_on_project_contributions_per_user() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let project = |x| TestProjectParams {
			expected_state: ProjectStatus::FundingSuccessful,
			metadata: default_project_metadata(x, ISSUER),
			evaluations: default_evaluations(),
			bids: default_bids(),
			community_contributions: default_community_buys(),
			remainder_contributions: default_remainder_buys(),
			issuer: ISSUER,
		};
		let projects = (0..20).into_iter().map(|x| project(x)).collect_vec();
		async_features::create_multiple_projects_at(inst, projects);
	}

	#[test]
	fn evaluation_plmc_unbonded_after_funding_success() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let evaluations = default_evaluations();
		let evaluators = evaluations.accounts();

		let project_id = inst.create_remainder_contributing_project(
			default_project_metadata(inst.get_new_nonce(), ISSUER),
			ISSUER,
			evaluations.clone(),
			default_bids(),
			default_community_buys(),
		);

		let prev_reserved_plmc =
			inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Evaluation(project_id).into());

		let prev_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

		inst.finish_funding(project_id).unwrap();
		inst.advance_time(<TestRuntime as Config>::ManualAcceptanceDuration::get() + 1).unwrap();
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		inst.advance_time(10).unwrap();
		let post_unbond_amounts: Vec<UserToPLMCBalance<_>> = prev_reserved_plmc
			.iter()
			.map(|UserToPLMCBalance { account, .. }| UserToPLMCBalance::new(*account, Zero::zero()))
			.collect();

		inst.do_reserved_plmc_assertions(post_unbond_amounts.clone(), HoldReason::Evaluation(project_id).into());
		inst.do_reserved_plmc_assertions(post_unbond_amounts, HoldReason::Participation(project_id).into());

		let post_free_plmc = inst.get_free_plmc_balances_for(evaluators);

		let increased_amounts =
			MockInstantiator::generic_map_operation(vec![post_free_plmc, prev_free_plmc], MergeOperation::Subtract);

		assert_eq!(increased_amounts, MockInstantiator::calculate_evaluation_plmc_spent(evaluations))
	}

	#[test]
	fn plmc_vesting_schedule_starts_automatically() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let mut bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
		bids =
			stored_bids.into_iter().map(|bid| BidParams::new_with_defaults(bid.bidder, bid.final_ct_amount)).collect();
		let auction_locked_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, price);
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
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&account,
					HoldReason::Participation(project_id).into(),
				)
			});

			assert_eq!(schedule.unwrap(), plmc_amount);
		}
	}

	#[test]
	fn plmc_vesting_schedule_starts_manually() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = inst.get_project_details(project_id);
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let stored_successful_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
				.collect::<Vec<_>>()
		});
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for bid in stored_successful_bids.clone() {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::do_start_bid_vesting_schedule_for(&bid.bidder, project_id, &bid.bidder, bid.id,)
			);
		}
		for contribution in stored_contributions {
			call_and_is_ok!(
				inst,
				Pallet::<TestRuntime>::do_start_contribution_vesting_schedule_for(
					&contribution.contributor,
					project_id,
					&contribution.contributor,
					contribution.id,
				)
			);
		}

		let auction_locked_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, price);
		let community_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(community_contributions, price);
		let remainder_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(remainder_contributions, price);
		let all_plmc_locks = MockInstantiator::generic_map_operation(
			vec![auction_locked_plmc, community_locked_plmc, remainder_locked_plmc],
			MergeOperation::Add,
		);

		for UserToPLMCBalance { account, plmc_amount } in all_plmc_locks {
			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&account,
					HoldReason::Participation(project_id).into(),
				)
			});

			assert_eq!(schedule.unwrap(), plmc_amount);
		}
	}

	#[test]
	fn plmc_vesting_full_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
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

		let stored_successful_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
				.collect::<Vec<_>>()
		});

		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		let total_bid_plmc_in_vesting: Vec<UserToPLMCBalance<TestRuntime>> = stored_successful_bids
			.iter()
			.map(|bid| (bid.bidder, bid.plmc_vesting_info.unwrap().total_amount).into())
			.collect_vec();

		let total_contribution_plmc_in_vesting: Vec<UserToPLMCBalance<TestRuntime>> = stored_contributions
			.iter()
			.map(|contribution| (contribution.contributor, contribution.plmc_vesting_info.unwrap().total_amount).into())
			.collect_vec();

		let total_participant_plmc_in_vesting = MockInstantiator::generic_map_operation(
			vec![total_bid_plmc_in_vesting, total_contribution_plmc_in_vesting],
			MergeOperation::Add,
		);

		inst.advance_time((10 * DAYS).into()).unwrap();

		for UserToPLMCBalance { account, plmc_amount } in total_participant_plmc_in_vesting {
			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&account));

			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(account, project_id, account)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&account));
			assert_eq!(plmc_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	fn plmc_vesting_partial_amount() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER;
		let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let project_id = inst.create_finished_project(
			project_metadata,
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
		let stored_successful_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_prefix_values((project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
				.collect::<Vec<_>>()
		});
		let stored_contributions =
			inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		let bidder_vesting =
			stored_successful_bids.iter().map(|bid| (bid.bidder, bid.plmc_vesting_info.unwrap())).collect_vec();
		let contributor_vesting = stored_contributions
			.iter()
			.map(|contribution| (contribution.contributor, contribution.plmc_vesting_info.unwrap()))
			.collect_vec();

		let participant_vesting_infos: Vec<(AccountIdOf<TestRuntime>, Vec<VestingInfoOf<TestRuntime>>)> =
			MockInstantiator::generic_map_merge_reduce(
				vec![bidder_vesting, contributor_vesting],
				|map| map.0,
				Vec::new(),
				|map, mut vestings| {
					vestings.push(map.1);
					vestings
				},
			);

		let now = inst.current_block();
		for (participant, vesting_infos) in participant_vesting_infos {
			let vested_amount = vesting_infos.into_iter().fold(0u128, |acc, vesting_info| {
				acc + vesting_info.amount_per_block * min(vesting_info.duration, now - vest_start_block) as u128
			});

			let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&participant));

			inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(participant, project_id, participant)).unwrap();

			let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&participant));
			assert_eq!(vested_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	fn ct_account_deposits_are_returned() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(0, ISSUER);
		let automatic_fail_funding_percent = Percent::from_percent(30);
		let deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
			ProjectId,
			AccountIdOf<TestRuntime>,
		>>::deposit_required(0);

		let _remainder_contributors = vec![EVALUATOR_1, BIDDER_3, BUYER_4, BUYER_6, BIDDER_6];

		let funding_target = project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids = MockInstantiator::generate_bids_from_total_usd(
			automatic_fail_funding_percent * funding_target / 3,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);

		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			automatic_fail_funding_percent * funding_target / 3,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);

		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			automatic_fail_funding_percent * funding_target / 3,
			project_metadata.minimum_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);

		let zero_balances = remainder_contributions
			.clone()
			.accounts()
			.into_iter()
			.map(|acc| UserToPLMCBalance::new(acc, 0u128))
			.collect_vec();
		inst.do_free_plmc_assertions(zero_balances.clone());
		inst.do_reserved_plmc_assertions(zero_balances.clone(), HoldReason::FutureDeposit(0).into());

		let project_id = inst.create_finished_project(
			project_metadata,
			ISSUER,
			default_evaluations(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let bidder_plmc_bonds = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, wap);
		let community_contributor_plmc_bonds =
			MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), wap);
		let evaluators_and_contributors_plmc_bonds =
			MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				default_evaluations(),
				remainder_contributions,
				wap,
				true,
			);

		let mut expected_final_plmc_balances = MockInstantiator::generic_map_operation(
			vec![bidder_plmc_bonds, community_contributor_plmc_bonds, evaluators_and_contributors_plmc_bonds],
			MergeOperation::Add,
		);
		expected_final_plmc_balances.iter_mut().for_each(|UserToPLMCBalance { account: _, plmc_amount }| {
			*plmc_amount += deposit_required;
		});

		let prev_balances = inst.get_free_plmc_balances_for(expected_final_plmc_balances.accounts());
		let ct_deposit_balances = expected_final_plmc_balances
			.accounts()
			.into_iter()
			.map(|acc| UserToPLMCBalance::new(acc, deposit_required))
			.collect_vec();
		inst.do_reserved_plmc_assertions(ct_deposit_balances, HoldReason::FutureDeposit(project_id).into());

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_balances = inst.get_free_plmc_balances_for(expected_final_plmc_balances.accounts());

		let plmc_deltas =
			MockInstantiator::generic_map_operation(vec![post_balances, prev_balances], MergeOperation::Subtract);

		assert_eq!(plmc_deltas, expected_final_plmc_balances);
		inst.do_reserved_plmc_assertions(zero_balances, HoldReason::FutureDeposit(project_id).into());
	}
}

// only functionalities related to the CT Migration
mod ct_migration {
	use super::*;
	use frame_support::assert_err;

	#[test]
	fn para_id_for_project_can_be_set_by_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_finished_project(
			default_project_metadata(inst.get_new_nonce(), ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			default_remainder_buys(),
		);

		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_set_para_id_for_project(
				&ISSUER,
				project_id,
				ParaId::from(2006u32),
			));
		});
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.parachain_id, Some(ParaId::from(2006u32)));
	}

	#[test]
	fn para_id_for_project_cannot_be_set_by_anyone_but_issuer() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_finished_project(
			default_project_metadata(inst.get_new_nonce(), ISSUER),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			default_remainder_buys(),
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		inst.execute(|| {
			assert_err!(
				crate::Pallet::<TestRuntime>::do_set_para_id_for_project(
					&EVALUATOR_1,
					project_id,
					ParaId::from(2006u32),
				),
				Error::<TestRuntime>::NotAllowed
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::do_set_para_id_for_project(&BIDDER_1, project_id, ParaId::from(2006u32),),
				Error::<TestRuntime>::NotAllowed
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::do_set_para_id_for_project(&BUYER_1, project_id, ParaId::from(2006u32),),
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

// check that functions created to facilitate testing return the expected results
mod helper_functions {
	use super::*;

	#[test]
	fn calculate_evaluation_plmc_spent() {
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1u32;
		const USD_AMOUNT_1: u128 = 150_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 17_857_1_428_571_428_u128;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2u32;
		const USD_AMOUNT_2: u128 = 50_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 5_952_3_809_523_809_u128;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3u32;
		const USD_AMOUNT_3: u128 = 75_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 8_928_5_714_285_714_u128;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4u32;
		const USD_AMOUNT_4: u128 = 100_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 11_9_047_619_047_u128;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5u32;
		const USD_AMOUNT_5: u128 = 123_7_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 14_7_261_904_761_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap(),
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
	fn calculate_auction_plmc_returned() {
		const CT_AMOUNT_1: u128 = 5000 * ASSET_UNIT;
		const CT_AMOUNT_2: u128 = 40_000 * ASSET_UNIT;
		const CT_AMOUNT_3: u128 = 10_000 * ASSET_UNIT;
		const CT_AMOUNT_4: u128 = 6000 * ASSET_UNIT;
		const CT_AMOUNT_5: u128 = 2000 * ASSET_UNIT;

		let bid_1 = BidParams::new(BIDDER_1, CT_AMOUNT_1, 1u8, AcceptedFundingAsset::USDT);
		let bid_2 = BidParams::new(BIDDER_2, CT_AMOUNT_2, 1u8, AcceptedFundingAsset::USDT);
		let bid_3 = BidParams::new(BIDDER_1, CT_AMOUNT_3, 1u8, AcceptedFundingAsset::USDT);
		let bid_4 = BidParams::new(BIDDER_3, CT_AMOUNT_4, 1u8, AcceptedFundingAsset::USDT);
		let bid_5 = BidParams::new(BIDDER_4, CT_AMOUNT_5, 1u8, AcceptedFundingAsset::USDT);

		// post bucketing, the bids look like this:
		// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_1, 5k) - (BIDDER_1, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
		// | -------------------- 1USD ----------------------|---- 1.1 USD ---|---- 1.2 USD ----|----------- 1.3 USD -------------|
		// post wap ~ 1.0557252:
		// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

		const ORIGINAL_PLMC_CHARGED_BIDDER_1: u128 = 18_452_3_809_523_790;
		const ORIGINAL_PLMC_CHARGED_BIDDER_2: u128 = 47_619_0_476_190_470;
		const ORIGINAL_PLMC_CHARGED_BIDDER_3: u128 = 86_90_4_761_904_760;
		const ORIGINAL_PLMC_CHARGED_BIDDER_4: u128 = 30_95_2_380_952_380;

		const FINAL_PLMC_CHARGED_BIDDER_1: u128 = 12_236_4_594_692_840;
		const FINAL_PLMC_CHARGED_BIDDER_2: u128 = 38_095_2_380_952_380;
		const FINAL_PLMC_CHARGED_BIDDER_3: u128 = 75_40_8_942_202_840;
		const FINAL_PLMC_CHARGED_BIDDER_4: u128 = 2_513_6_314_067_610;

		let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(hashed(METADATA)),
		};
		let plmc_charged = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);
		dbg!(plmc_charged);
		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER,
			default_evaluations(),
			bids.clone(),
		);

		let stored_bids = inst.execute(|| {
			Bids::<TestRuntime>::iter_values().into_iter().sorted_by(|b1, b2| b1.id.cmp(&b2.id)).collect_vec()
		});
		dbg!(stored_bids);
		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
		dbg!(wap);

		let expected_returns = vec![
			ORIGINAL_PLMC_CHARGED_BIDDER_1 - FINAL_PLMC_CHARGED_BIDDER_1,
			ORIGINAL_PLMC_CHARGED_BIDDER_2 - FINAL_PLMC_CHARGED_BIDDER_2,
			ORIGINAL_PLMC_CHARGED_BIDDER_3 - FINAL_PLMC_CHARGED_BIDDER_3,
			ORIGINAL_PLMC_CHARGED_BIDDER_4 - FINAL_PLMC_CHARGED_BIDDER_4,
		];
		dbg!(&expected_returns);

		let mut returned_plmc_mappings =
			MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
		returned_plmc_mappings.sort_by(|b1, b2| b1.account.cmp(&b2.account));

		let returned_plmc_balances = returned_plmc_mappings.into_iter().map(|map| map.plmc_amount).collect_vec();
		dbg!(&returned_plmc_balances);

		for (expected, calculated) in zip(expected_returns, returned_plmc_balances) {
			assert_close_enough!(expected, calculated, Perquintill::from_float(0.01));
		}
	}

	#[test]
	fn calculate_contributed_plmc_spent() {
		const PLMC_PRICE: f64 = 8.4f64;
		const CT_PRICE: f64 = 16.32f64;

		const CONTRIBUTOR_1: AccountIdOf<TestRuntime> = 1u32;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 1_958_4_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 233_1_428_571_428_u128;

		const CONTRIBUTOR_2: AccountIdOf<TestRuntime> = 2u32;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 81_975_3_600_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 4_879_4_857_142_857_u128;

		const CONTRIBUTOR_3: AccountIdOf<TestRuntime> = 3u32;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 326_400_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_285_7_142_857_142_u128;

		const CONTRIBUTOR_4: AccountIdOf<TestRuntime> = 4u32;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_4: u128 = 16_320_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 77_714_2_857_142_857_u128;

		const CONTRIBUTOR_5: AccountIdOf<TestRuntime> = 5u32;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_5: u128 = 2_0_122_562_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_239_554_285_u128;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap(),
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
}

// logic of small functions that extrinsics use to process data or interact with storage
mod inner_functions {
	use super::*;

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
}

// test the parallel instantiation of projects
mod async_tests {
	use super::*;
	use instantiator::async_features::*;

	#[test]
	fn prototype_2() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let project_params = vec![
			TestProjectParams {
				expected_state: ProjectStatus::Application,
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: vec![],
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::EvaluationRound,
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: vec![],
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: default_evaluations(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: default_community_buys(),
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::FundingSuccessful,
				metadata: default_project_metadata(inst.get_new_nonce(), ISSUER),
				issuer: ISSUER,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: default_community_buys(),
				remainder_contributions: default_remainder_buys(),
			},
		];

		let (project_ids, mut inst) = create_multiple_projects_at(inst, project_params);

		dbg!(inst.get_project_details(project_ids[0]).status);
		dbg!(inst.get_project_details(project_ids[1]).status);
		dbg!(inst.get_project_details(project_ids[2]).status);
		dbg!(inst.get_project_details(project_ids[3]).status);
		dbg!(inst.get_project_details(project_ids[4]).status);
		dbg!(inst.get_project_details(project_ids[5]).status);

		assert_eq!(inst.get_project_details(project_ids[0]).status, ProjectStatus::Application);
		assert_eq!(inst.get_project_details(project_ids[1]).status, ProjectStatus::EvaluationRound);
		assert_eq!(inst.get_project_details(project_ids[2]).status, ProjectStatus::AuctionRound(AuctionPhase::English));
		assert_eq!(inst.get_project_details(project_ids[3]).status, ProjectStatus::CommunityRound);
		assert_eq!(inst.get_project_details(project_ids[4]).status, ProjectStatus::RemainderRound);
		assert_eq!(inst.get_project_details(project_ids[5]).status, ProjectStatus::FundingSuccessful);
	}

	#[test]
	fn genesis_parallel_instantiaton() {
		let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// only used to generate some values, and not for chain interactions
		let funding_percent = 93u64;
		let project_metadata = default_project_metadata(0u64, ISSUER.into());
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(30u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(20u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);
		mock::RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: vec![(
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get(),
				)],
			},
			foreign_assets: ForeignAssetsConfig {
				assets: vec![(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				)],
				metadata: vec![],
				accounts: vec![],
			},
			polimec_funding: PolimecFundingConfig {
				starting_projects: vec![
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::FundingSuccessful,
						metadata: default_project_metadata(0u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: community_contributions.clone(),
						remainder_contributions: remainder_contributions.clone(),
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::RemainderRound,
						metadata: default_project_metadata(1u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: community_contributions.clone(),
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::CommunityRound,
						metadata: default_project_metadata(2u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
						metadata: default_project_metadata(3u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: evaluations.clone(),
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::EvaluationRound,
						metadata: default_project_metadata(4u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: vec![],
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::Application,
						metadata: default_project_metadata(5u64, ISSUER.into()),
						issuer: ISSUER.into(),
						evaluations: vec![],
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
				],
				phantom: PhantomData,
			},

			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let ext = sp_io::TestExternalities::new(t);
		let mut inst = MockInstantiator::new(Some(RefCell::new(ext)));

		dbg!(inst.get_project_details(0).status);
		dbg!(inst.get_project_details(1).status);
		dbg!(inst.get_project_details(2).status);
		dbg!(inst.get_project_details(3).status);
		dbg!(inst.get_project_details(4).status);
		dbg!(inst.get_project_details(5).status);

		assert_eq!(inst.get_project_details(5).status, ProjectStatus::Application);
		assert_eq!(inst.get_project_details(4).status, ProjectStatus::EvaluationRound);
		assert_eq!(inst.get_project_details(3).status, ProjectStatus::AuctionRound(AuctionPhase::English));
		assert_eq!(inst.get_project_details(2).status, ProjectStatus::CommunityRound);
		assert_eq!(inst.get_project_details(1).status, ProjectStatus::RemainderRound);
		assert_eq!(inst.get_project_details(0).status, ProjectStatus::FundingSuccessful);
	}

	#[test]
	fn starting_auction_round_with_bids() {
		let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// only used to generate some values, and not for chain interactions
		let mut project_metadata = default_project_metadata(0u64, ISSUER.into());
		let evaluations = default_evaluations();
		let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
		let min_bid = project_metadata.bidding_ticket_sizes.institutional.usd_minimum_per_participation.unwrap();
		let auction_allocation_percentage = project_metadata.auction_round_allocation_percentage;
		let auction_ct_required = min_bid.saturating_mul(max_bids_per_project as u128);
		let total_allocation_required = auction_allocation_percentage.saturating_reciprocal_mul(auction_ct_required);
		project_metadata.total_allocation_size = total_allocation_required;

		let max_bids = (0u32..max_bids_per_project)
			.map(|i| {
				instantiator::BidParams::<TestRuntime>::new(
					(i + 69).into(),
					project_metadata.bidding_ticket_sizes.institutional.usd_minimum_per_participation.unwrap(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		mock::RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: vec![(
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get(),
				)],
			},
			foreign_assets: ForeignAssetsConfig {
				assets: vec![(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				)],
				metadata: vec![],
				accounts: vec![],
			},
			polimec_funding: PolimecFundingConfig {
				starting_projects: vec![TestProjectParams::<TestRuntime> {
					expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
					metadata: default_project_metadata(0u64, ISSUER.into()),
					issuer: ISSUER.into(),
					evaluations: evaluations.clone(),
					bids: max_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				}],
				phantom: PhantomData,
			},

			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let ext = sp_io::TestExternalities::new(t);
		let mut inst = MockInstantiator::new(Some(RefCell::new(ext)));

		assert_eq!(inst.get_project_details(0).status, ProjectStatus::AuctionRound(AuctionPhase::English));
		let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
		let total_bids_count = inst.execute(|| Bids::<TestRuntime>::iter_values().collect_vec().len());
		assert_eq!(total_bids_count, max_bids_per_project as usize);
	}
}
