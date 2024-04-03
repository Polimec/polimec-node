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
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold, Mutate, MutateHold},
		Get,
	},
};
use itertools::Itertools;
use parachains_common::DAYS;
use polimec_common::ReleaseSchedule;
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
const ISSUER_1: AccountId = 10;
const ISSUER_2: AccountId = 12;
const ISSUER_3: AccountId = 13;
const ISSUER_4: AccountId = 14;
const ISSUER_5: AccountId = 15;
const ISSUER_6: AccountId = 16;
const ISSUER_7: AccountId = 17;
const ISSUER_8: AccountId = 18;
const ISSUER_9: AccountId = 19;
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

#[path = "1_application.rs"]
mod application;
#[path = "3_auction.rs"]
mod auction;
#[path = "4_community.rs"]
mod community;
#[path = "8_ct_migration.rs"]
mod ct_migration;
#[path = "2_evaluation.rs"]
mod evaluation;
#[path = "6_funding_end.rs"]
mod funding_end;
mod misc;
#[path = "5_remainder.rs"]
mod remainder;
#[path = "7_settlement.rs"]
mod settlement;

pub mod defaults {
	use super::*;

	pub fn default_token_information() -> CurrencyMetadata<BoundedVec<u8, StringLimitOf<TestRuntime>>> {
		CurrencyMetadata {
			name: BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap(),
			symbol: BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap(),
			decimals: ASSET_DECIMALS,
		}
	}
	pub fn default_project_metadata(issuer: AccountId) -> ProjectMetadataOf<TestRuntime> {
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
			funding_destination_account: ISSUER_1,
			offchain_information_hash: Some(metadata_hash),
		};
		project_metadata
	}

	pub fn default_plmc_balances() -> Vec<UserToPLMCBalance<TestRuntime>> {
		vec![
			UserToPLMCBalance::new(ISSUER_1, 10_000_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_1, 10_000_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_2, 10_000_000 * PLMC),
			UserToPLMCBalance::new(EVALUATOR_3, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BIDDER_1, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BIDDER_2, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BUYER_1, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BUYER_2, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BUYER_3, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BUYER_4, 10_000_000 * PLMC),
			UserToPLMCBalance::new(BUYER_5, 10_000_000 * PLMC),
		]
	}

	pub fn default_usdt_balances() -> Vec<UserToForeignAssets<TestRuntime>> {
		vec![
			(ISSUER_1, 10_000_000 * ASSET_UNIT).into(),
			(EVALUATOR_1, 10_000_000 * ASSET_UNIT).into(),
			(EVALUATOR_2, 10_000_000 * ASSET_UNIT).into(),
			(EVALUATOR_3, 10_000_000 * ASSET_UNIT).into(),
			(BIDDER_1, 10_000_000 * ASSET_UNIT).into(),
			(BIDDER_2, 10_000_000 * ASSET_UNIT).into(),
			(BUYER_1, 10_000_000 * ASSET_UNIT).into(),
			(BUYER_2, 10_000_000 * ASSET_UNIT).into(),
			(BUYER_3, 10_000_000 * ASSET_UNIT).into(),
			(BUYER_4, 10_000_000 * ASSET_UNIT).into(),
			(BUYER_5, 10_000_000 * ASSET_UNIT).into(),
		]
	}

	pub fn default_evaluations() -> Vec<UserToUSDBalance<TestRuntime>> {
		vec![
			UserToUSDBalance::new(EVALUATOR_1, 500_000 * US_DOLLAR),
			UserToUSDBalance::new(EVALUATOR_2, 250_000 * US_DOLLAR),
			UserToUSDBalance::new(EVALUATOR_3, 320_000 * US_DOLLAR),
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
		vec![10u8, 3u8, 8u8, 1u8, 4u8]
	}
	pub fn default_community_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}
	pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}

	pub fn default_community_contributors() -> Vec<AccountId> {
		vec![BUYER_1, BUYER_2, BUYER_3, BUYER_4, BUYER_5]
	}

	pub fn default_remainder_contributors() -> Vec<AccountId> {
		vec![EVALUATOR_1, BIDDER_3, BUYER_4, BUYER_6, BIDDER_6]
	}

	pub fn default_all_participants() -> Vec<AccountId> {
		let mut accounts: Vec<AccountId> = default_evaluators()
			.iter()
			.chain(default_bidders().iter())
			.chain(default_community_contributors().iter())
			.chain(default_remainder_contributors().iter())
			.copied()
			.collect();
		accounts.sort();
		accounts.dedup();
		accounts
	}

	pub fn project_from_funding_reached(instantiator: &mut MockInstantiator, percent: u64) -> ProjectId {
		let project_metadata = default_project_metadata(ISSUER_1);
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
		instantiator.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![])
	}

	pub fn default_bids_from_ct_percent(percent: u8) -> Vec<BidParams<TestRuntime>> {
		let project_metadata = default_project_metadata(ISSUER_1);
		MockInstantiator::generate_bids_from_total_ct_percent(
			project_metadata,
			percent,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		)
	}

	pub fn default_community_contributions_from_ct_percent(percent: u8) -> Vec<ContributionParams<TestRuntime>> {
		let project_metadata = default_project_metadata(ISSUER_1);
		MockInstantiator::generate_contributions_from_total_ct_percent(
			project_metadata,
			percent,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		)
	}

	pub fn default_remainder_contributions_from_ct_percent(percent: u8) -> Vec<ContributionParams<TestRuntime>> {
		let project_metadata = default_project_metadata(ISSUER_1);
		MockInstantiator::generate_contributions_from_total_ct_percent(
			project_metadata,
			percent,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		)
	}
}

pub fn create_project_with_funding_percentage(
	percentage: u64,
	maybe_decision: Option<FundingOutcomeDecision>,
) -> (MockInstantiator, ProjectId) {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(ISSUER_1);
	let min_price = project_metadata.minimum_price;
	let percentage_funded_usd = Perquintill::from_percent(percentage) *
		(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
	let evaluations = default_evaluations();
	let bids = MockInstantiator::generate_bids_from_total_usd(
		Percent::from_percent(50u8) * percentage_funded_usd,
		min_price,
		default_weights(),
		default_bidders(),
		default_multipliers(),
	);
	let contributions = MockInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(50u8) * percentage_funded_usd,
		min_price,
		default_weights(),
		default_community_contributors(),
		default_multipliers(),
	);
	let project_id = inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);

	match inst.get_project_details(project_id).status {
		ProjectStatus::AwaitingProjectDecision => {
			assert!(percentage > 33 && percentage < 90);
			assert!(maybe_decision.is_some());
			inst.execute(|| PolimecFunding::do_decide_project_outcome(ISSUER_1, project_id, maybe_decision.unwrap()))
				.unwrap();
		},
		ProjectStatus::FundingSuccessful => {
			assert!(percentage >= 90);
		},
		ProjectStatus::FundingFailed => {
			assert!(percentage <= 33);
		},
		_ => panic!("unexpected project status"),
	};

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1u64).unwrap();
	let funding_sucessful = match percentage {
		0..=33 => false,
		34..=89 if matches!(maybe_decision, Some(FundingOutcomeDecision::RejectFunding)) => false,
		34..=89 if matches!(maybe_decision, Some(FundingOutcomeDecision::AcceptFunding)) => true,
		90..=100 => true,
		_ => panic!("unexpected percentage"),
	};

	if funding_sucessful {
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
		inst.test_ct_created_for(project_id);
	} else {
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		inst.test_ct_not_created_for(project_id);
	}
	(inst, project_id)
}
