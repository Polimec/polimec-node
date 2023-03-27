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
use crate::{mock::*, CurrencyMetadata, Error, ParticipantsSize, Project, TicketSize};
use frame_support::{
	assert_noop, assert_ok,
	traits::{tokens::fungibles::Inspect, ConstU32, Hooks},
	weights::Weight,
};

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 4;

const ASSET_DECIMALS: u8 = 12;

// const fn unit(decimals: u8) -> BalanceOf<Test> {
// 	10u128.pow(decimals as u32)
// }

const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}"#;

fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

fn run_to_block(n: BlockNumber) {
	let mut current_block = System::block_number();
	while current_block < n {
		FundingModule::on_finalize(System::block_number());
		Balances::on_finalize(System::block_number());
		FundingModule::on_idle(System::block_number(), Weight::from_ref_time(10000000));
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		FundingModule::on_initialize(System::block_number());
		Balances::on_initialize(System::block_number());
		FundingModule::on_idle(System::block_number(), Weight::from_ref_time(10000000));
		current_block = System::block_number();
	}
}

fn store_and_return_metadata_hash() -> sp_core::H256 {
	let metadata_vec = METADATA.as_bytes().to_vec();
	let metadata_bounded = BoundedVec::try_from(metadata_vec)
		.expect("len() == 150 bytes, so it is safe to unwrap since the limit is 1024.");
	let _ = FundingModule::note_image(RuntimeOrigin::signed(ALICE), metadata_bounded);
	hashed(METADATA)
}

fn create_project() -> Project<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
	let metadata_hash = store_and_return_metadata_hash();
	let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
	Project {
		total_allocation_size: 1000,
		minimum_price: 1_u128,
		ticket_size: TicketSize { minimum: Some(1), maximum: None },
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		metadata: metadata_hash,
		token_information: CurrencyMetadata {
			name: bounded_name,
			symbol: bounded_symbol,
			decimals: ASSET_DECIMALS,
		},
		..Default::default()
	}
}

fn create_on_chain_project() {
	let project = create_project();
	let _ = FundingModule::create(RuntimeOrigin::signed(ALICE), project);
}

mod creation_round {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::{assert_noop, assert_ok};

	#[test]
	fn preimage_works() {
		new_test_ext().execute_with(|| {
			let bounded_metadata = BoundedVec::try_from(METADATA.as_bytes().to_vec()).unwrap();
			assert_ok!(FundingModule::note_image(RuntimeOrigin::signed(ALICE), bounded_metadata));
			let expected_hash = hashed(METADATA);
			assert_eq!(ALICE, FundingModule::images(expected_hash).unwrap())
		})
	}

	#[test]
	fn create_works() {
		new_test_ext().execute_with(|| {
			let project = create_project();
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_eq!(
				last_event(),
				RuntimeEvent::FundingModule(crate::Event::Created { project_id: 0 })
			);
		})
	}

	#[test]
	#[ignore]
	fn only_with_credential_can_create() {
		new_test_ext().execute_with(|| {
			let project = create_project();
			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(BOB), project),
				Error::<Test>::NotAuthorized
			);
		})
	}

	#[test]
	fn project_id_autoincremenet_works() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			create_on_chain_project();
			assert_eq!(
				last_event(),
				RuntimeEvent::FundingModule(crate::Event::Created { project_id: 1 })
			);
		})
	}

	#[test]
	fn price_too_low() {
		new_test_ext().execute_with(|| {
			let metadata_hash = store_and_return_metadata_hash();
			let wrong_project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				metadata: metadata_hash,
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), wrong_project),
				Error::<Test>::PriceTooLow
			);
		})
	}

	#[test]
	fn participants_size_error() {
		new_test_ext().execute_with(|| {
			let metadata_hash = store_and_return_metadata_hash();
			let wrong_project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				metadata: metadata_hash,
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), wrong_project),
				Error::<Test>::ParticipantsSizeError
			);
		})
	}

	#[test]
	fn ticket_size_error() {
		new_test_ext().execute_with(|| {
			let metadata_hash = store_and_return_metadata_hash();
			let wrong_project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
				metadata: metadata_hash,
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), wrong_project),
				Error::<Test>::TicketSizeError
			);
		})
	}

	#[test]
	#[ignore = "ATM only the first error will be thrown"]
	fn multiple_field_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}
}

mod evaluation_round {
	use super::*;

	#[test]
	fn start_evaluation_works() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0).unwrap();
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
		})
	}

	#[test]
	fn evaluation_stops_with_success_after_28_days() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 1000));
			run_to_block(System::block_number() + 29);
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::AuctionInitializePeriod);
		})
	}

	#[test]
	fn evaluation_stops_with_failure_after_28_days() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::EvaluationRound);
			run_to_block(System::block_number() + 29);
			let ed = FundingModule::project_info(0).unwrap();
			assert!(ed.project_status == ProjectStatus::EvaluationFailed);
		})
	}

	#[test]
	fn basic_bond_works() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
		})
	}

	#[test]
	fn multiple_users_can_bond() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(CHARLIE), 0, 128));

			let bonds = FundingModule::bonds(0, BOB);
			assert_eq!(bonds.unwrap(), 128);

			let bonds = FundingModule::bonds(0, CHARLIE);
			assert_eq!(bonds.unwrap(), 128);
		})
	}

	#[test]
	fn cannot_bond() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));

			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, u128::MAX),
				Error::<Test>::InsufficientBalance
			);
		})
	}

	#[test]
	fn multiple_bond_works() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
			let bonds = FundingModule::bonds(0, BOB);
			assert_eq!(bonds.unwrap(), 256);
			let locked_amount = Balances::locks(&BOB).iter().next().unwrap().amount;
			assert_eq!(locked_amount, 256);
		})
	}
}

mod auction_round {
	use super::*;

	fn setup_envirnoment() {
		create_on_chain_project();
		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
		assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 10_000));
		run_to_block(System::block_number() + <Test as Config>::EvaluationDuration::get() + 1);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
	}

	#[test]
	fn start_auction_works() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
		})
	}

	#[test]
	fn cannot_start_auction_before_evaluation() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_noop!(
				FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0),
				Error::<Test>::EvaluationNotStarted
			);
		})
	}

	#[test]
	fn bid_works() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			let free_balance = Balances::free_balance(&CHARLIE);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1, None));
			let bids = FundingModule::auctions_info(0);
			assert!(bids.iter().any(|bid| bid.when == System::block_number() &&
				bid.amount == 100 &&
				bid.price == 1));
			let free_balance_after_bid = Balances::free_balance(&CHARLIE);

			assert!(free_balance_after_bid == free_balance - 100);

			// Get the reserved_balance of CHARLIE
			let reserved_balance = Balances::reserved_balance(&CHARLIE);
			assert!(free_balance_after_bid == free_balance - reserved_balance);
			assert!(reserved_balance == 100);
		})
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		new_test_ext().execute_with(|| {
			create_on_chain_project();
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_noop!(
				FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 1, 100, None),
				Error::<Test>::AuctionNotStarted
			);
		})
	}

	#[test]
	fn contribute_does_not_work() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			assert_noop!(
				FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100),
				Error::<Test>::AuctionNotStarted
			);
		})
	}
}

mod community_round {
	use super::*;

	fn setup_envirnoment() {
		create_on_chain_project();
		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
		assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 10_000));

		run_to_block(System::block_number() + <Test as Config>::EvaluationDuration::get() + 1);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
		assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
		assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1, None));

		run_to_block(
			System::block_number() +
				<Test as Config>::EnglishAuctionDuration::get() +
				<Test as Config>::CandleAuctionDuration::get() +
				1,
		);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::CommunityRound);
	}

	#[test]
	fn contribute_works() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100));

			// Check that the contribution is stored
			let contribution_info = FundingModule::contributions(0, BOB).unwrap();
			assert_eq!(contribution_info.amount, 100);

			// Check that the funds are in the project's account Balance
			let project_account = Pallet::<Test>::fund_account_id(0);
			let project_balance = Balances::free_balance(&project_account);
			assert_eq!(project_balance, 100);
		})
	}

	#[test]
	fn contribute_multiple_times_works() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100));
			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 200));
			let contribution_info = FundingModule::contributions(0, BOB).unwrap();
			assert_eq!(contribution_info.amount, 300);
			assert!(contribution_info.can_claim);
		})
	}
}

mod claim_contribution_tokens {
	use super::*;

	fn setup_envirnoment() {
		create_on_chain_project();
		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
		assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 10_000));

		run_to_block(System::block_number() + <Test as Config>::EvaluationDuration::get() + 1);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
		assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
		assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1 * PLMC, None));

		run_to_block(
			System::block_number() +
				<Test as Config>::EnglishAuctionDuration::get() +
				<Test as Config>::CandleAuctionDuration::get() +
				1,
		);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.weighted_average_price, Some(PLMC));
		assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 99 * PLMC));

		run_to_block(System::block_number() + <Test as Config>::CommunityRoundDuration::get() + 1);
		let project_info = FundingModule::project_info(0).unwrap();
		assert_eq!(project_info.project_status, ProjectStatus::ReadyToLaunch);
	}

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::claim_contribution_tokens(RuntimeOrigin::signed(BOB), 0));
			assert_eq!(Assets::balance(0, BOB), 99);
		})
	}

	#[test]
	fn cannot_claim_multiple_times() {
		new_test_ext().execute_with(|| {
			setup_envirnoment();
			assert_ok!(FundingModule::claim_contribution_tokens(RuntimeOrigin::signed(BOB), 0));
			assert_noop!(
				FundingModule::claim_contribution_tokens(RuntimeOrigin::signed(BOB), 0),
				Error::<Test>::AlreadyClaimed
			);
		})
	}
}

mod flow {
	use super::*;
	use crate::{AuctionPhase, ParticipantsSize, ProjectStatus, TicketSize};

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			// Create a new project
			create_on_chain_project();
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::Application);

			// Start the Evaluation Round
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert_eq!(active_projects.len(), 1);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));

			// Evaluation Round ends automatically
			run_to_block(System::block_number() + <Test as Config>::EvaluationDuration::get() + 1);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(
				project_info.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 1, 100, None));

			// Second phase of Funding Round: 2) Candle Auction Round
			run_to_block(
				System::block_number() + <Test as Config>::EnglishAuctionDuration::get() + 1,
			);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(
				project_info.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 2, 200, None));

			// Third phase of Funding Round: 3) Community Round
			run_to_block(
				System::block_number() + <Test as Config>::CandleAuctionDuration::get() + 1,
			);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::CommunityRound);
			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 200));

			// Funding Round ends
			run_to_block(
				System::block_number() + <Test as Config>::CommunityRoundDuration::get() + 1,
			);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::ReadyToLaunch);
			// Project is no longer "active"
			let active_projects = FundingModule::projects_active();
			assert_eq!(active_projects.len(), 0);

			// Check if the Contribution Token is actually created
			assert!(Assets::asset_exists(0));

			// Check if the the metadata are set correctly
			let metadata_name = Assets::name(&0);
			assert_eq!(metadata_name, b"Contribution Token TEST".to_vec());
			let metadata_symbol = Assets::symbol(&0);
			assert_eq!(metadata_symbol, b"CTEST".to_vec());
			let metadata_decimals = Assets::decimals(&0);
			assert_eq!(metadata_decimals, ASSET_DECIMALS);

			// Check if the Contribution Token is minted correctly
			assert_ok!(FundingModule::claim_contribution_tokens(RuntimeOrigin::signed(BOB), 0));
			assert_eq!(Assets::balance(0, BOB), 1);
		})
	}

	#[test]
	fn check_weighted_average_price() {
		new_test_ext().execute_with(|| {
			// Prologue
			let metadata_hash = store_and_return_metadata_hash();
			let project = Project {
				minimum_price: 10,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				total_allocation_size: 1_000_000,
				metadata: metadata_hash,
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert_eq!(active_projects.len(), 1);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 20 * PLMC));
			run_to_block(System::block_number() + <Test as Config>::EvaluationDuration::get() + 1);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(
				project_info.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(BOB), 0, 10_000, 15 * PLMC, None));

			// Second phase of Funding Round: 2) Candle Auction Round
			run_to_block(
				System::block_number() + <Test as Config>::EnglishAuctionDuration::get() + 1,
			);
			let project_info = FundingModule::project_info(0).unwrap();
			assert_eq!(
				project_info.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(
				RuntimeOrigin::signed(CHARLIE),
				0,
				20_000,
				20 * PLMC,
				None
			));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 20_000, 10 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(5), 0, 20_000, 8 * PLMC, None));

			run_to_block(
				System::block_number() + <Test as Config>::CandleAuctionDuration::get() + 1,
			);
			let project_info = FundingModule::project_info(0).unwrap();
			let price = project_info.weighted_average_price;
			assert!(price.is_some());
			assert_ne!(price.unwrap(), 0);
		})
	}

	#[test]
	fn bids_overflow() {
		new_test_ext().execute_with(|| {
			// Prologue
			let metadata_hash = store_and_return_metadata_hash();
			let project = Project {
				minimum_price: 10,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				total_allocation_size: 1_000_000,
				metadata: metadata_hash,
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 20 * PLMC));
			run_to_block(System::block_number() + 29);
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			// Perform 5 bids, T::MaxBidsPerProject = 4 in the mock runtime
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(BOB), 0, 10_000, 2 * PLMC, None));
			assert_ok!(FundingModule::bid(
				RuntimeOrigin::signed(CHARLIE),
				0,
				13_000,
				3 * PLMC,
				None
			));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 15_000, 5 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(5), 0, 1_000, 7 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(5), 0, 20_000, 8 * PLMC, None));
			let bids = FundingModule::auctions_info(0);
			assert_eq!(bids.len(), 4);
			assert_eq!(bids[0].ticket_size, 20_000 * 8 * PLMC);
			assert_eq!(bids[1].ticket_size, 1_000 * 7 * PLMC);
			assert_eq!(bids[2].ticket_size, 15_000 * 5 * PLMC);
			assert_eq!(bids[3].ticket_size, 13_000 * 3 * PLMC);
		})
	}
}

mod unit_tests {
	use super::*;

	#[test]
	fn calculate_claimable_tokens_works() {
		new_test_ext().execute_with(|| {
			let contribution_amount: BalanceOf<Test> = 1000 * PLMC;
			let weighted_average_price: BalanceOf<Test> = 10 * PLMC;
			let expected_amount: FixedU128 = FixedU128::from(100);

			let amount = Pallet::<Test>::calculate_claimable_tokens(
				contribution_amount,
				weighted_average_price,
			);

			assert_eq!(amount, expected_amount);
		})
	}

	#[test]
	fn calculate_claimable_tokens_works_with_float() {
		new_test_ext().execute_with(|| {
			let contribution_amount: BalanceOf<Test> = 11 * PLMC;
			let weighted_average_price: BalanceOf<Test> = 4 * PLMC;
			let expected_amount: FixedU128 = FixedU128::from_float(2.75);

			let amount = Pallet::<Test>::calculate_claimable_tokens(
				contribution_amount,
				weighted_average_price,
			);

			assert_eq!(amount, expected_amount);
		})
	}

	#[test]
	fn calculate_claimable_tokens_works_with_small_amount() {
		new_test_ext().execute_with(|| {
			let contribution_amount: BalanceOf<Test> = 1 * PLMC;
			let weighted_average_price: BalanceOf<Test> = 2 * PLMC;
			let expected_amount: FixedU128 = FixedU128::from_float(0.5);

			let amount = Pallet::<Test>::calculate_claimable_tokens(
				contribution_amount,
				weighted_average_price,
			);

			assert_eq!(amount, expected_amount);
		})
	}
}
