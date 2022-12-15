use crate::{mock::*, Error, Project, Weight};
use frame_support::{
	assert_noop, assert_ok,
	traits::{OnFinalize, OnIdle, OnInitialize},
};

pub fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

pub fn run_to_block(n: BlockNumber) {
	while System::block_number() < n {
		FundingModule::on_finalize(System::block_number());
		Balances::on_finalize(System::block_number());
		FundingModule::on_idle(System::block_number(), Weight::from_ref_time(10000000));
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		FundingModule::on_initialize(System::block_number());
		Balances::on_initialize(System::block_number());
		FundingModule::on_idle(System::block_number(), Weight::from_ref_time(10000000));
	}
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 3;

mod creation_round {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::assert_noop;

	#[test]
	fn create_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_eq!(
				last_event(),
				RuntimeEvent::FundingModule(crate::Event::Created { project_id: 0 })
			);
		})
	}

	#[test]
	fn only_issuer_can_create() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(BOB), project),
				Error::<Test>::NotAuthorized
			);
		})
	}

	#[test]
	fn project_id_autoincremenet_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project.clone()));
			assert_eq!(
				last_event(),
				RuntimeEvent::FundingModule(crate::Event::Created { project_id: 0 })
			);
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_eq!(
				last_event(),
				RuntimeEvent::FundingModule(crate::Event::Created { project_id: 1 })
			);
		})
	}

	#[test]
	fn price_too_low() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), project),
				Error::<Test>::PriceTooLow
			);
		})
	}

	#[test]
	fn participants_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), project),
				Error::<Test>::ParticipantsSizeError
			);
		})
	}

	#[test]
	fn ticket_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}

	// #[test]
	// #[ignore = "ATM only the first error will be thrown"]
	// fn multiple_field_error() {
	// 	new_test_ext().execute_with(|| {
	// 		let project = Project {
	// 			minimum_price: 0,
	// 			ticket_size: TicketSize { minimum: None, maximum: None },
	// 			participants_size: ParticipantsSize { minimum: None, maximum: None },
	// 			..Default::default()
	// 		};

	// 		assert_noop!(
	// 			FundingModule::create(RuntimeOrigin::signed(ALICE), project),
	// 			Error::<Test>::TicketSizeError
	// 		);
	// 	})
	// }
}

mod evaluation_round {
	use super::*;
	use crate::{ParticipantsSize, ProjectStatus, TicketSize};

	#[test]
	fn start_evaluation_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
		})
	}

	#[test]
	fn evaluation_stops_after_28_days() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			let ed = FundingModule::project_info(0);
			assert!(ed.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let ed = FundingModule::project_info(0);
			assert!(ed.project_status == ProjectStatus::EvaluationRound);
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			let ed = FundingModule::project_info(0);
			assert!(ed.project_status == ProjectStatus::EvaluationEnded);
		})
	}

	#[test]
	fn basic_bond_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
		})
	}

	#[test]
	fn multiple_bond_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
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
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));

			assert_noop!(
				FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 1024 * PLMC),
				Error::<Test>::InsufficientBalance
			);
		})
	}
}

mod auction_round {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::assert_noop;

	#[test]
	fn start_auction_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
		})
	}

	#[test]
	fn cannot_start_auction_before_evaluation() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_noop!(
				FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0),
				Error::<Test>::EvaluationNotStarted
			);
		})
	}

	#[test]
	fn bid_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));

			let free_balance = Balances::free_balance(&CHARLIE);

			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1, None));
			let bids = FundingModule::auctions_info(0);
			assert!(bids
				.iter()
				.any(|(when, bid)| *when == block_number + 29 &&
					bid.amount == 100 && bid.market_cap == 1));
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
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
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
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			assert_noop!(
				FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100),
				Error::<Test>::AuctionNotStarted
			);
		})
	}
}

mod community_round {
	#[test]
	fn contribute_works() {}
}

mod flow {
	use super::*;
	use crate::{AuctionPhase, ParticipantsSize, ProjectStatus, TicketSize};

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			// Create a new project
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::Application);

			// Start the Evaluation Round
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 1);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));

			// Evaluation Round ends automatically
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::EvaluationEnded);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 1, 100, None));

			// Second phase of Funding Round: 2) Candle Auction Round
			let block_number = System::block_number();
			run_to_block(block_number + 10);
			let project_info = FundingModule::project_info(0);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 2, 200, None));

			// Third phase of Funding Round: 3) Community Round
			let block_number = System::block_number();
			run_to_block(block_number + 5);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::CommunityRound);
			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100));

			// Funding Round ends
			let block_number = System::block_number();
			run_to_block(block_number + 11);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::ReadyToLaunch);
			// Project is no longer "active"
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 0);
		})
	}

	#[test]
	fn check_final_price() {
		new_test_ext().execute_with(|| {
			// Prologue
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				total_allocation_size: 100000,
				fundraising_target: 101 * PLMC,
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::Application);
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let active_projects = FundingModule::projects_active();
			assert!(active_projects.len() == 1);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::EvaluationRound);
			assert_ok!(FundingModule::bond(RuntimeOrigin::signed(BOB), 0, 128));
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.project_status == ProjectStatus::EvaluationEnded);

			// Start the Funding Round: 1) English Auction Round
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English)
			);
			assert_ok!(FundingModule::bid(
				RuntimeOrigin::signed(BOB),
				0,
				19 * PLMC,
				17 * PLMC,
				None
			));

			// Second phase of Funding Round: 2) Candle Auction Round
			let block_number = System::block_number();
			run_to_block(block_number + 10);
			let project_info = FundingModule::project_info(0);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(
				RuntimeOrigin::signed(CHARLIE),
				0,
				74 * PLMC,
				2 * PLMC,
				None
			));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(3), 0, 16 * PLMC, 35 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(4), 0, 15 * PLMC, 20 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(4), 0, 12 * PLMC, 55 * PLMC, None));

			let block_number = System::block_number();
			run_to_block(block_number + 10);
			let project_info = FundingModule::project_info(0);
			assert!(project_info.final_price != Some(0));
		})
	}

	#[test]
	fn bids_overflow() {
		new_test_ext().execute_with(|| {
			// Prologue
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				total_allocation_size: 100000,
				fundraising_target: 101 * PLMC,
				..Default::default()
			};
			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let block_number = System::block_number();
			run_to_block(block_number + 29);
			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
			// Second phase of Funding Round: 2) Candle Auction Round
			let block_number = System::block_number();
			run_to_block(block_number + 10);
			let project_info = FundingModule::project_info(0);
			assert!(
				project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, PLMC, 2 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(3), 0, 2 * PLMC, 3 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(4), 0, 4 * PLMC, 5 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(4), 0, 6 * PLMC, 7 * PLMC, None));
			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(4), 0, 10 * PLMC, 7 * PLMC, None));
			let bids = FundingModule::auctions_info(0);
			assert!(bids.len() == 4);
			assert!(bids[0].1.amount == 2 * PLMC);
			assert!(bids[1].1.amount == 4 * PLMC);
			assert!(bids[2].1.amount == 6 * PLMC);
			assert!(bids[3].1.amount == 10 * PLMC);
		})
	}
}

// mod final_price {
// 	use crate::BidInfo;
// 	use sp_std::cmp::Reverse;

// 	use super::*;

// 	#[test]
// 	fn check() {
// 		new_test_ext().execute_with(|| {
// 			let total_allocation_size = 101 * PLMC;
// 			let mut bids: Vec<BidInfo<u128, u64>> = vec![
// 				BidInfo::new(10 * PLMC, 10 * PLMC, 1, total_allocation_size),
// 				BidInfo::new(12 * PLMC, 55 * PLMC, 2, total_allocation_size),
// 				BidInfo::new(15 * PLMC, 20 * PLMC, 3, total_allocation_size),
// 				BidInfo::new(16 * PLMC, 35 * PLMC, 4, total_allocation_size),
// 				BidInfo::new(19 * PLMC, 17 * PLMC, 5, total_allocation_size),
// 				BidInfo::new(1 * PLMC, 28 * PLMC, 6, total_allocation_size),
// 				BidInfo::new(5 * PLMC, 10 * PLMC, 7, total_allocation_size),
// 				BidInfo::new(74 * PLMC, 1 * PLMC, 8, total_allocation_size),
// 				BidInfo::new(3 * PLMC, 23 * PLMC, 9, total_allocation_size),
// 			];
// 			bids.sort_by_key(|bid| Reverse(bid.market_cap));
// 			let value = FundingModule::final_price_logic(bids, total_allocation_size);
// 			assert!(value.is_ok());
// 			let inner_value = value.unwrap();
// 			println!("inner_value: {:#?}", inner_value);
// 			// assert!(inner_value == 248019801985);
// 		})
// 	}
// }
