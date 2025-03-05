use super::*;
use crate::{
	functions::runtime_api::{ExtrinsicHelpers, Leaderboards, ProjectInformation, UserInformation},
	instantiator::*,
	mock::*,
	traits::VestingDurationCalculation,
	CurrencyMetadata, Error,
	ParticipationMode::*,
	ProjectMetadata, TicketSize,
};
use defaults::*;
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{
		fungible::{InspectFreeze, MutateFreeze, MutateHold},
		fungibles::{metadata::Inspect as MetadataInspect, Inspect, Mutate},
	},
};
use itertools::Itertools;
use pallet_balances::AccountData;
use parachains_common::DAYS;
use polimec_common::{
	assets::{
		AcceptedFundingAsset,
		AcceptedFundingAsset::{DOT, ETH, USDC, USDT},
	},
	ProvideAssetPrice, USD_DECIMALS, USD_UNIT,
};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt, get_mock_jwt_with_cid};
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_runtime::{bounded_vec, traits::Convert, PerThing, TokenError};
use sp_std::cell::RefCell;
use std::{
	collections::{BTreeSet, HashSet},
	iter::zip,
};
use InvestorType::{self, *};

#[path = "1_application.rs"]
mod application;
#[path = "3_auction.rs"]
mod auction;
#[path = "6_ct_migration.rs"]
mod ct_migration;
#[path = "2_evaluation.rs"]
mod evaluation;
#[path = "4_funding_end.rs"]
mod funding_end;
mod misc;
mod runtime_api;
#[path = "5_settlement.rs"]
mod settlement;

pub type MockInstantiator =
	Instantiator<TestRuntime, <TestRuntime as crate::Config>::AllPalletsWithoutSystem, RuntimeEvent>;
pub const CT_DECIMALS: u8 = 15;
pub const CT_UNIT: u128 = 10_u128.pow(CT_DECIMALS as u32);
pub const USDT_UNIT: u128 = USD_UNIT;

const IPFS_CID: &str = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";
const ISSUER_1: AccountId = 11;
const ISSUER_2: AccountId = 12;
const ISSUER_3: AccountId = 13;
const ISSUER_4: AccountId = 14;
const ISSUER_5: AccountId = 15;
const ISSUER_6: AccountId = 16;
const ISSUER_7: AccountId = 17;
const EVALUATOR_1: AccountId = 21;
const EVALUATOR_2: AccountId = 22;
const EVALUATOR_3: AccountId = 23;
const EVALUATOR_4: AccountId = 24;
const EVALUATOR_5: AccountId = 25;
const BIDDER_1: AccountId = 31;
const BIDDER_2: AccountId = 32;
const BIDDER_3: AccountId = 33;
const BIDDER_4: AccountId = 34;
const BIDDER_5: AccountId = 35;
const BIDDER_6: AccountId = 36;

const fn default_accounts() -> [AccountId; 18] {
	[
		ISSUER_1,
		ISSUER_2,
		ISSUER_3,
		ISSUER_4,
		ISSUER_5,
		ISSUER_6,
		ISSUER_7,
		EVALUATOR_1,
		EVALUATOR_2,
		EVALUATOR_3,
		EVALUATOR_4,
		EVALUATOR_5,
		BIDDER_1,
		BIDDER_2,
		BIDDER_3,
		BIDDER_4,
		BIDDER_5,
		BIDDER_6,
	]
}

pub mod defaults {
	use super::*;
	use polimec_common::assets::AcceptedFundingAsset::{DOT, ETH, USDC, USDT};

	pub fn default_token_information() -> CurrencyMetadata<BoundedVec<u8, StringLimitOf<TestRuntime>>> {
		CurrencyMetadata { name: bounded_name(), symbol: bounded_symbol(), decimals: CT_DECIMALS }
	}
	pub fn default_project_metadata(issuer: AccountId) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = bounded_name();
		let bounded_symbol = bounded_symbol();
		let metadata_hash = ipfs_hash();
		let base_price = PriceOf::<TestRuntime>::from_float(10.0);
		let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
			base_price,
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap();
		ProjectMetadata {
			token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
			mainnet_token_max_supply: 8_000_000 * CT_UNIT,
			total_allocation_size: 500_000 * CT_UNIT,
			minimum_price: decimal_aware_price,
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(100 * USD_UNIT, None),
				institutional: TicketSize::new(100 * USD_UNIT, None),
				retail: TicketSize::new(100 * USD_UNIT, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![USDT, USDC, DOT, ETH].try_into().unwrap(),
			funding_destination_account: issuer,
			policy_ipfs_cid: Some(metadata_hash),
			participants_account_type: ParticipantsAccountType::Polkadot,
		}
	}

	pub fn bounded_name() -> BoundedVec<u8, sp_core::ConstU32<64>> {
		BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap()
	}
	pub fn bounded_symbol() -> BoundedVec<u8, sp_core::ConstU32<64>> {
		BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap()
	}
	pub fn ipfs_hash() -> BoundedVec<u8, sp_core::ConstU32<96>> {
		BoundedVec::try_from(IPFS_CID.as_bytes().to_vec()).unwrap()
	}

	pub fn default_plmc_balances() -> Vec<UserToPLMCBalance<TestRuntime>> {
		let accounts = default_accounts().to_vec();
		accounts.iter().map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: PLMC * 1_000_000 }).collect()
	}

	pub fn default_usdt_balances() -> Vec<UserToFundingAsset<TestRuntime>> {
		let accounts = default_accounts().to_vec();
		accounts
			.iter()
			.map(|acc| UserToFundingAsset { account: *acc, asset_amount: 1_000_000 * USD_UNIT, asset_id: USDT.id() })
			.collect()
	}

	pub fn project_from_funding_reached(instantiator: &mut MockInstantiator, percent: u64) -> ProjectId {
		let project_metadata = default_project_metadata(ISSUER_1);
		let usd_to_reach = Perquintill::from_percent(percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = instantiator.generate_successful_evaluations(project_metadata.clone(), 5);
		let bids = instantiator.generate_bids_from_total_usd(project_metadata.clone(), usd_to_reach, 5);

		instantiator.create_finished_project(project_metadata, ISSUER_1, None, evaluations, bids)
	}

	pub fn default_bids_from_ct_percent(percent: u8) -> Vec<BidParams<TestRuntime>> {
		// Used only to generate values, not for chain interactions
		let inst = MockInstantiator::new(None);
		let project_metadata = default_project_metadata(ISSUER_1);
		inst.generate_bids_from_total_ct_percent(project_metadata, percent, 10)
	}
}

pub fn create_project_with_funding_percentage(percentage: u8, start_settlement: bool) -> (MockInstantiator, ProjectId) {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let mut project_metadata = default_project_metadata(ISSUER_1);
	project_metadata.total_allocation_size = 1_000_000 * CT_UNIT;
	let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
	let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), percentage, 30);

	let project_id = inst.create_finished_project(project_metadata, ISSUER_1, None, evaluations, bids);

	if start_settlement {
		assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(_)));
	}

	(inst, project_id)
}

pub fn create_finished_project_with_usd_raised(
	mut inst: MockInstantiator,
	usd_raised: Balance,
	usd_target: Balance,
) -> (MockInstantiator, ProjectId) {
	let mut test_inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = inst.get_new_nonce();
	let mut project_metadata = default_project_metadata(issuer);
	project_metadata.total_allocation_size =
		project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(usd_target);

	let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

	let bids;
	if usd_raised <= usd_target {
		bids = inst.generate_bids_from_total_usd(project_metadata.clone(), usd_raised, 5);
	} else {
		// This function generates new projects, so we need to use the test instance
		bids = test_inst.generate_bids_from_higher_usd_than_target(project_metadata.clone(), usd_raised);
	}

	let project_id = inst.create_finished_project(project_metadata, issuer, None, evaluations, bids);

	let project_details = inst.get_project_details(project_id);

	// We are happy if the amount raised is 99.999% of what we wanted
	assert_close_enough!(project_details.funding_amount_reached_usd, usd_raised, Perquintill::from_float(0.999));
	assert_eq!(project_details.fundraising_target_usd, usd_target);

	(inst, project_id)
}

macro_rules! polkadot_junction {
    // Case 1: Explicit `[u8; 32]` literal with 32 values
    ([ $($byte:literal),* ]) => {{
        let id: [u8; 32] = [$($byte),*];
        Junction::AccountId32 {
            network: Some(NetworkId::Polkadot),
            id,
        }
    }};

    // Case 2: Repeated syntax `[value; 32]`
    ([ $byte:literal ; 32 ]) => {{
        let id: [u8; 32] = [$byte; 32];
        Junction::AccountId32 {
            network: Some(NetworkId::Polkadot),
            id,
        }
    }};

    // Case 3: Variable or expression
    ($account:expr) => {{
        let id: [u8; 32] = <TestRuntime as Config>::AccountId32Conversion::convert($account);
        Junction::AccountId32 {
            network: Some(NetworkId::Polkadot),
            id,
        }
    }};
}
use polkadot_junction;
