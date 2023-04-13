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

use std::cell::{RefMut, RefCell};
use super::*;
use crate::{mock::*, CurrencyMetadata, Error, ParticipantsSize, Project, TicketSize};
use frame_support::{
	assert_noop, assert_ok,
	traits::{tokens::fungibles::Inspect, ConstU32, Hooks, Get},
	weights::Weight,
};
use sp_core::parameter_types;
use sp_io::TestExternalities;
use sp_runtime::DispatchError;
use crate::mock::FundingModule;

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;

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
	frame_system::Pallet::<TestRuntime>::events().pop().expect("Event expected").event
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

fn default_project() -> Project<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
	let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
	Project {
		total_allocation_size: 1000,
		minimum_price: 1_u128,
		ticket_size: TicketSize { minimum: Some(1), maximum: None },
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		metadata: Some(hashed(METADATA)),
		token_information: CurrencyMetadata {
			name: bounded_name,
			symbol: bounded_symbol,
			decimals: ASSET_DECIMALS,
		},
		..Default::default()
	}
}

fn create_on_chain_project() {
	let project = default_project();
	let _ = FundingModule::create(RuntimeOrigin::signed(ALICE), project);
}

trait TestInstance {}
trait ProjectInstance {
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime>;
	fn get_test_environment(&self) -> &TestEnvironment;
	fn do_project_assertions(&self, project_assertions: impl Fn(ProjectIdOf<TestRuntime>, RefMut<TestExternalities>) -> ()) {
		let project_id = self.get_project_id();
		let test_env = self.get_test_environment();
		project_assertions(project_id,test_env.ext_env.borrow_mut());
	}
}

// Initial instance of a test
#[derive(Debug)]
struct TestEnvironment {
	ext_env: RefCell<sp_io::TestExternalities>,
}
impl TestEnvironment {
	fn new() -> Self {
		Self {
			ext_env: RefCell::new(new_test_ext()),
		}
	}
	fn create_project(&self, creator: mock::AccountId, project: ProjectOf<TestRuntime>) -> Result<CreatedProject, DispatchError> {
		// Create project in the externalities environment of this struct instance
		self.ext_env.borrow_mut().execute_with(|| {
			FundingModule::create(RuntimeOrigin::signed(creator), project)
		})?;

		// Retrieve the project_id from the events
		let project_id = self.ext_env.borrow_mut().execute_with(|| {
			Ok(frame_system::Pallet::<TestRuntime>::events().iter().filter_map(|event| {
				match event.event {
					RuntimeEvent::FundingModule(crate::Event::Created{ project_id }) => Some(project_id),
					_ => None
				}
			}).last().expect("Project created event expected").clone())
		});

		Ok(CreatedProject {
			test_env: self,
			project_id
		})
	}
}

#[derive(Debug)]
struct CreatedProject<'a> {
	test_env: &'a TestEnvironment,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for CreatedProject<'a> {
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}

	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
}
impl<'a> CreatedProject<'a> {
	fn start_evaluation(&self, caller: mock::AccountId) -> Result<EvaluatingProject, DispatchError> {
		let project_id = self.get_project_id();
		let test_env = self.get_test_environment();
		test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::start_evaluation(RuntimeOrigin::signed(caller), project_id)
		})?;

		Ok(EvaluatingProject {
			test_env,
			project_id,
		})
	}
}

#[derive(Debug)]
struct EvaluatingProject<'a> {
	test_env: &'a TestEnvironment,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for EvaluatingProject<'a> {
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}

	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
}
impl<'a> EvaluatingProject<'a> {}


#[cfg(test)]
mod creation_round_success {
	use super::*;

	pub fn default_creation_assertions(project_id: ProjectIdOf<TestRuntime>, mut test_ext: RefMut<TestExternalities> ) {
		test_ext.execute_with(|| {
			let project = FundingModule::projects(project_id).expect("Project should exist");
			let project_info = FundingModule::project_info(project_id).expect("Project info should exist");
			let default_project = default_project();
			assert_eq!(project, default_project);
			assert_eq!(project_info.project_status, ProjectStatus::Application);
		});
	}

	#[test]
	fn create_works() {
		let mut test_env = TestEnvironment::new();
		let project_1 = test_env.create_project(ALICE, default_project()).unwrap();
		project_1.do_project_assertions(default_creation_assertions);
	}



	#[test]
	fn project_id_autoincrement_works() {
		let mut test_env = TestEnvironment::new();
		let project_1 = test_env.create_project(ALICE, default_project()).unwrap();

		let mut project_2_info = default_project();
		project_2_info.metadata = Some(hashed("metadata2"));

		let mut project_3_info = default_project();
		project_3_info.metadata = Some(hashed("metadata3"));

		let project_2 = test_env.create_project(ALICE, project_2_info).unwrap();
		let project_3 = test_env.create_project(ALICE, project_3_info).unwrap();
		assert_eq!(project_1.project_id, 0);
		assert_eq!(project_2.project_id, 1);
		assert_eq!(project_3.project_id, 2);
	}


}

#[cfg(test)]
mod creation_round_failure {
	use super::*;

	#[test]
	#[ignore]
	fn only_with_credential_can_create() {
		new_test_ext().execute_with(|| {
			let project = default_project();
			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(BOB), project),
				Error::<TestRuntime>::NotAuthorized
			);
		})
	}

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectOf<TestRuntime> = Project {
			minimum_price: 0,
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			metadata: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();

		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
		assert_eq!(
			project_err,
			Error::<TestRuntime>::PriceTooLow.into(),
		);
	}

	#[test]
	fn participants_size_error() {
		let wrong_project: ProjectOf<TestRuntime> = Project {
			minimum_price: 1,
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: None, maximum: None },
			metadata: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();
		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
		assert_eq!(
			project_err,
			Error::<TestRuntime>::ParticipantsSizeError.into(),
		);
	}

	#[test]
	fn ticket_size_error() {
		let wrong_project: ProjectOf<TestRuntime> = Project {
			minimum_price: 1,
			ticket_size: TicketSize { minimum: None, maximum: None },
			participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
			metadata: Some(hashed(METADATA)),
			..Default::default()
		};
		let test_env = TestEnvironment::new();
		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
		assert_eq!(
			project_err,
			Error::<TestRuntime>::TicketSizeError.into()
		);
	}

	#[test]
	#[ignore = "ATM only the first error will be thrown"]
	fn multiple_field_error() {
		let wrong_project: ProjectOf<TestRuntime> = Project {
			minimum_price: 0,
			ticket_size: TicketSize { minimum: None, maximum: None },
			participants_size: ParticipantsSize { minimum: None, maximum: None },
			..Default::default()
		};
		let test_env = TestEnvironment::new();
		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
		assert_eq!(
			project_err,
			Error::<TestRuntime>::TicketSizeError.into()
		);
	}
}

#[cfg(test)]
mod evaluation_round_success {
	use super::*;
	use super::creation_round_success::default_creation_assertions;

	pub fn default_evaluation_assertions(project_id: ProjectIdOf<TestRuntime>, mut test_ext: RefMut<TestExternalities> ) {
		test_ext.execute_with(|| {
			let project_info = FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
		});
	}

	#[test]
	fn start_evaluation_works() {
		let test_env = TestEnvironment::new();
		let creator = ALICE;
		let project = test_env.create_project(creator, default_project()).unwrap();
		project.do_project_assertions(default_creation_assertions);
		let project = project.start_evaluation(creator).unwrap();

			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
			let project_info = FundingModule::project_info(0).unwrap();
		})
	}

	// #[test]
	// fn evaluation_stops_with_success_after_config_duration() {
	// 	new_test_ext().execute_with(|| {
	// 		create_on_chain_project();
	// 		let ed = FundingModule::project_info(0).unwrap();
	// 		assert_eq!(ed.project_status, ProjectStatus::Application);
	// 		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
	// 		let ed = FundingModule::project_info(0).unwrap();
	// 		run_to_block(System::block_number() + 1);
	// 		assert_eq!(ed.project_status, ProjectStatus::EvaluationRound);
	// 		assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 1000));
	// 		run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
	// 		let ed = FundingModule::project_info(0).unwrap();
	// 		assert_eq!(ed.project_status, ProjectStatus::AuctionInitializePeriod);
	// 	})
	// }



// 	#[test]
// 	fn evaluation_stops_with_failure_after_config_period() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			let ed = FundingModule::project_info(0).unwrap();
// 			assert_eq!(ed.project_status, ProjectStatus::Application);
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			let ed = FundingModule::project_info(0).unwrap();
// 			assert_eq!(ed.project_status, ProjectStatus::EvaluationRound);
// 			run_to_block(System::block_number() + 1);
// 			let pre_bond_bob_free_balance = Balances::usable_balance(&BOB);
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 50));
// 			run_to_block(System::block_number() + 1);
// 			let post_bond_bob_free_balance = Balances::usable_balance(&BOB);
// 			assert_eq!(pre_bond_bob_free_balance - post_bond_bob_free_balance, 50);
// 			run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 			let ed = FundingModule::project_info(0).unwrap();
// 			assert_eq!(ed.project_status, ProjectStatus::EvaluationFailed);
// 			// assert_ok!(FundingModule::failed_evaluation_unbond_for(RuntimeOrigin::signed(BOB), 0, BOB));
// 			run_to_block(System::block_number() + 1);
// 			let post_release_bob_free_balance = Balances::usable_balance(&BOB);
// 			assert_eq!(post_release_bob_free_balance - post_bond_bob_free_balance, 50);
// 		})
// 	}
//
// 	#[test]
// 	fn basic_bond_works() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_noop!(
// 				FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128),
// 				Error::<TestRuntime>::EvaluationNotStarted
// 			);
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128));
// 		})
// 	}
//
// 	#[test]
// 	fn multiple_users_can_bond() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_noop!(
// 				FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128),
// 				Error::<TestRuntime>::EvaluationNotStarted
// 			);
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(CHARLIE), 0, 128));
//
// 			let bonds = Balances::reserved_balance_named(&BondType::Evaluation, &BOB);
// 			assert_eq!(bonds, 128);
//
// 			let bonds = Balances::reserved_balance_named(&BondType::Evaluation, &CHARLIE);
// 			assert_eq!(bonds, 128);
// 		})
// 	}
//
// 	#[test]
// 	fn cannot_bond() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
//
// 			assert_noop!(
// 				FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, u128::MAX),
// 				Error::<TestRuntime>::InsufficientBalance
// 			);
// 		})
// 	}
//
// 	#[test]
// 	fn multiple_bond_works() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_noop!(
// 				FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128),
// 				Error::<TestRuntime>::EvaluationNotStarted
// 			);
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128));
// 			let bonds = Balances::reserved_balance_named(&BondType::Evaluation, &BOB);
// 			assert_eq!(bonds, 256);
// 			let reserved_amount = Balances::reserved_balance_named(&BondType::Evaluation, &BOB);
// 			assert_eq!(reserved_amount, 256);
// 		})
// 	}
// }
//
// mod auction_round {
// 	use super::*;
//
// 	fn setup_environment() {
// 		create_on_chain_project();
// 		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 		assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 10_000));
// 		run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
// 	}
//
// 	#[test]
// 	fn start_auction_works() {
// 		new_test_ext().execute_with(|| {
// 			setup_environment();
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 		})
// 	}
//
// 	#[test]
// 	fn cannot_start_auction_before_evaluation() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_noop!(
// 				FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0),
// 				Error::<TestRuntime>::EvaluationPeriodNotEnded
// 			);
// 		})
// 	}
//
// 	#[test]
// 	fn bid_works() {
// 		new_test_ext().execute_with(|| {
// 			setup_environment();
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 			let free_balance = Balances::free_balance(&CHARLIE);
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1, None));
// 			let bids = FundingModule::auctions_info(0, CHARLIE).unwrap();
// 			assert!(bids.iter().any(|bid| bid.amount == 100 &&
// 				bid.price == 1));
// 			let free_balance_after_bid = Balances::free_balance(&CHARLIE);
//
// 			// PLMC and bidding currency is both the same right now, so 100 for bond and 100 for bid
// 			assert_eq!(free_balance_after_bid, free_balance - 200);
//
// 			// Get the reserved_balance of CHARLIE
// 			let reserved_balance = Balances::reserved_balance(&CHARLIE);
// 			assert_eq!(free_balance_after_bid, free_balance - reserved_balance);
// 		})
// 	}
//
// 	#[test]
// 	fn cannot_bid_before_auction_round() {
// 		new_test_ext().execute_with(|| {
// 			create_on_chain_project();
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			assert_noop!(
// 				FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 1, 100, None),
// 				Error::<TestRuntime>::AuctionNotStarted
// 			);
// 		})
// 	}
//
// 	#[test]
// 	fn contribute_does_not_work() {
// 		new_test_ext().execute_with(|| {
// 			setup_environment();
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 			assert_noop!(
// 				FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100),
// 				Error::<TestRuntime>::AuctionNotStarted
// 			);
// 		})
// 	}
// }
//
// mod community_round {
// 	use super::*;
//
// 	fn setup_envirnoment() {
// 		create_on_chain_project();
// 		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 		assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 10_000));
//
// 		run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
// 		assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 		run_to_block(System::block_number() + 1);
//
// 		assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1, None));
//
// 		run_to_block(
// 			System::block_number() +
// 				<TestRuntime as Config>::EnglishAuctionDuration::get() +
// 				<TestRuntime as Config>::CandleAuctionDuration::get() +
// 				1,
// 		);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::CommunityRound);
// 	}
//
// 	#[test]
// 	fn contribute_works() {
// 		new_test_ext().execute_with(|| {
// 			setup_envirnoment();
// 			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100));
//
// 			// Check that the contribution is stored
// 			let contribution_info = FundingModule::contributions(0, BOB).unwrap();
// 			assert_eq!(contribution_info[0].contribution_amount, 100);
//
// 			// Check that the funds are in the project's account Balance
// 			let project_account = Pallet::<TestRuntime>::fund_account_id(0);
// 			let project_balance = Balances::free_balance(&project_account);
// 			assert_eq!(project_balance, 100);
// 		})
// 	}
//
// 	#[test]
// 	fn contribute_multiple_times_works() {
// 		new_test_ext().execute_with(|| {
// 			setup_envirnoment();
// 			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 100));
// 			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 200));
// 			let contributions_info = FundingModule::contributions(0, BOB).unwrap();
// 			let contributions_total_amount = contributions_info.iter().fold(0, |acc, contribution| acc + contribution.contribution_amount);
// 			assert_eq!(contributions_total_amount, 300);
// 		})
// 	}
// }
//
// mod vested_contribution_token_purchase_mint_for {
// 	use super::*;
//
// 	fn setup_environment() {
// 		create_on_chain_project();
// 		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 		assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 10_000));
//
// 		run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
// 		assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
//
// 		run_to_block(System::block_number() + 1);
//
// 		assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 100, 1 * PLMC, None));
//
// 		run_to_block(
// 			System::block_number() +
// 				<TestRuntime as Config>::EnglishAuctionDuration::get() +
// 				<TestRuntime as Config>::CandleAuctionDuration::get() +
// 				1,
// 		);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.weighted_average_price, Some(PLMC));
// 		assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 99 * PLMC));
//
// 		run_to_block(
// 			System::block_number() + <TestRuntime as Config>::CommunityFundingDuration::get() + 1,
// 		);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::RemainderRound);
//
// 		run_to_block(
// 			System::block_number() + <TestRuntime as Config>::RemainderFundingDuration::get() + 1,
// 		);
// 		let project_info = FundingModule::project_info(0).unwrap();
// 		assert_eq!(project_info.project_status, ProjectStatus::FundingEnded);
// 	}
//
// 	#[test]
// 	fn it_works() {
// 		new_test_ext().execute_with(|| {
// 			setup_environment();
// 			assert_ok!(FundingModule::vested_contribution_token_purchase_mint_for(RuntimeOrigin::signed(BOB), 0, BOB));
// 			assert_eq!(Assets::balance(0, BOB), 99);
// 		})
// 	}
//
// 	// TODO: You can now that we added vesting. We should test claiming after vesting ended, and before the next vesting period
// 	// #[test]
// 	// fn cannot_claim_multiple_times() {
// 	// 	new_test_ext().execute_with(|| {
// 	// 		setup_environment();
// 	// 		assert_ok!(FundingModule::vested_contribution_token_purchase_mint_for(RuntimeOrigin::signed(BOB), 0));
// 	// 		assert_noop!(
// 	// 			FundingModule::vested_contribution_token_purchase_mint_for(RuntimeOrigin::signed(BOB), 0),
// 	// 			Error::<Test>::AlreadyClaimed
// 	// 		);
// 	// 	})
// 	// }
// }
//
// mod flow {
// 	use super::*;
// 	use crate::{AuctionPhase, ParticipantsSize, ProjectStatus, TicketSize};
//
// 	#[test]
// 	fn it_works() {
// 		new_test_ext().execute_with(|| {
// 			// Create a new project
// 			create_on_chain_project();
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::Application);
//
// 			// Start the Evaluation Round
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			let active_projects =
// 				ProjectsToUpdate::<TestRuntime>::iter_values().flatten().collect::<Vec<_>>();
// 			assert_eq!(active_projects.len(), 1);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 128));
//
// 			// Evaluation Round ends automatically
// 			run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
//
// 			// Start the Funding Round: 1) English Auction Round
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(
// 				project_info.project_status,
// 				ProjectStatus::AuctionRound(AuctionPhase::English)
// 			);
// 			run_to_block(System::block_number() + 1);
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(CHARLIE), 0, 1, 100, None));
//
// 			// Second phase of Funding Round: 2) Candle Auction Round
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::EnglishAuctionDuration::get() + 1,
// 			);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(
// 				project_info.project_status,
// 				ProjectStatus::AuctionRound(AuctionPhase::Candle)
// 			);
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 2, 200, None));
//
// 			// Third phase of Funding Round: 3) Community Round
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::CandleAuctionDuration::get() + 1,
// 			);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::CommunityRound);
// 			assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(BOB), 0, 200));
//
// 			// Fourth phase of Funding Round: 4) Remainder Round
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::CommunityFundingDuration::get() + 1,
// 			);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::RemainderRound);
//
// 			// Funding ended, claim contribution tokens
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::RemainderFundingDuration::get() + 1,
// 			);
// 			// Check if the Contribution Token is actually created
// 			assert!(Assets::asset_exists(0));
//
// 			// Check if the the metadata are set correctly
// 			let metadata_name = Assets::name(&0);
// 			assert_eq!(metadata_name, b"Contribution Token TEST".to_vec());
// 			let metadata_symbol = Assets::symbol(&0);
// 			assert_eq!(metadata_symbol, b"CTEST".to_vec());
// 			let metadata_decimals = Assets::decimals(&0);
// 			assert_eq!(metadata_decimals, ASSET_DECIMALS);
//
// 			// Check if the Contribution Token is minted correctly
// 			assert_ok!(FundingModule::vested_contribution_token_purchase_mint_for(RuntimeOrigin::signed(BOB), 0, BOB));
// 			assert_eq!(Assets::balance(0, BOB), 1);
// 		})
// 	}
//
// 	#[test]
// 	fn check_weighted_average_price() {
// 		new_test_ext().execute_with(|| {
// 			// Prologue
// 			let metadata_hash = store_and_return_metadata_hash();
// 			let project = Project {
// 				minimum_price: 10,
// 				ticket_size: TicketSize { minimum: Some(1), maximum: None },
// 				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
// 				total_allocation_size: 1_000_000,
// 				metadata: metadata_hash,
// 				..Default::default()
// 			};
//
// 			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
//
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::Application);
//
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
//
// 			let active_projects =
// 				ProjectsToUpdate::<TestRuntime>::iter_values().flatten().collect::<Vec<_>>();
// 			assert_eq!(active_projects.len(), 1);
//
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
//
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 20 * PLMC));
//
// 			run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
//
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
//
// 			// Start the Funding Round: 1) English Auction Round
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(
// 				project_info.project_status,
// 				ProjectStatus::AuctionRound(AuctionPhase::English)
// 			);
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(BOB), 0, 10_000, 15 * PLMC, None));
//
// 			// Second phase of Funding Round: 2) Candle Auction Round
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::EnglishAuctionDuration::get() + 1,
// 			);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			assert_eq!(
// 				project_info.project_status,
// 				ProjectStatus::AuctionRound(AuctionPhase::Candle)
// 			);
// 			assert_ok!(FundingModule::bid(
// 				RuntimeOrigin::signed(CHARLIE),
// 				0,
// 				20_000,
// 				20 * PLMC,
// 				None
// 			));
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 20_000, 10 * PLMC, None));
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(5), 0, 20_000, 8 * PLMC, None));
//
// 			run_to_block(
// 				System::block_number() + <TestRuntime as Config>::CandleAuctionDuration::get() + 1,
// 			);
// 			let project_info = FundingModule::project_info(0).unwrap();
// 			let price = project_info.weighted_average_price;
// 			assert!(price.is_some());
// 			assert_ne!(price.unwrap(), 0);
// 		})
// 	}
//
// 	#[test]
// 	fn bids_overflow() {
// 		new_test_ext().execute_with(|| {
// 			// Prologue
// 			let metadata_hash = store_and_return_metadata_hash();
// 			let project = Project {
// 				minimum_price: 10,
// 				ticket_size: TicketSize { minimum: Some(1), maximum: None },
// 				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
// 				total_allocation_size: 1_000_000,
// 				metadata: metadata_hash,
// 				..Default::default()
// 			};
// 			assert_ok!(FundingModule::create(RuntimeOrigin::signed(ALICE), project));
// 			assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(ALICE), 0));
// 			assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(BOB), 0, 20 * PLMC));
// 			run_to_block(System::block_number() + <TestRuntime as Config>::EvaluationDuration::get() + 2);
// 			assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(ALICE), 0));
// 			// Perform 5 bids, T::MaxBidsPerProject = 4 in the mock runtime
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 10_000, 2 * PLMC, None));
// 			assert_ok!(FundingModule::bid(
// 				RuntimeOrigin::signed(DAVE),
// 				0,
// 				13_000,
// 				3 * PLMC,
// 				None
// 			));
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 15_000, 5 * PLMC, None));
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 1_000, 7 * PLMC, None));
// 			assert_ok!(FundingModule::bid(RuntimeOrigin::signed(DAVE), 0, 20_000, 8 * PLMC, None));
// 			let bids = FundingModule::auctions_info(0, DAVE).unwrap();
// 			assert_eq!(bids.len(), 4);
// 			assert_eq!(bids[0].ticket_size, 20_000 * 8 * PLMC);
// 			assert_eq!(bids[1].ticket_size, 1_000 * 7 * PLMC);
// 			assert_eq!(bids[2].ticket_size, 15_000 * 5 * PLMC);
// 			assert_eq!(bids[3].ticket_size, 13_000 * 3 * PLMC);
// 		})
// 	}
// }
//
// mod unit_tests {
// 	use super::*;
//
// 	#[test]
// 	fn calculate_claimable_tokens_works() {
// 		new_test_ext().execute_with(|| {
// 			let contribution_amount: BalanceOf<TestRuntime> = 1000 * PLMC;
// 			let weighted_average_price: BalanceOf<TestRuntime> = 10 * PLMC;
// 			let expected_amount: FixedU128 = FixedU128::from(100);
//
// 			let amount = Pallet::<TestRuntime>::calculate_claimable_tokens(
// 				contribution_amount,
// 				weighted_average_price,
// 			);
//
// 			assert_eq!(amount, expected_amount);
// 		})
// 	}
//
// 	#[test]
// 	fn calculate_claimable_tokens_works_with_float() {
// 		new_test_ext().execute_with(|| {
// 			let contribution_amount: BalanceOf<TestRuntime> = 11 * PLMC;
// 			let weighted_average_price: BalanceOf<TestRuntime> = 4 * PLMC;
// 			let expected_amount: FixedU128 = FixedU128::from_float(2.75);
//
// 			let amount = Pallet::<TestRuntime>::calculate_claimable_tokens(
// 				contribution_amount,
// 				weighted_average_price,
// 			);
//
// 			assert_eq!(amount, expected_amount);
// 		})
// 	}
//
// 	#[test]
// 	fn calculate_claimable_tokens_works_with_small_amount() {
// 		new_test_ext().execute_with(|| {
// 			let contribution_amount: BalanceOf<TestRuntime> = 1 * PLMC;
// 			let weighted_average_price: BalanceOf<TestRuntime> = 2 * PLMC;
// 			let expected_amount: FixedU128 = FixedU128::from_float(0.5);
//
// 			let amount = Pallet::<TestRuntime>::calculate_claimable_tokens(
// 				contribution_amount,
// 				weighted_average_price,
// 			);
//
// 			assert_eq!(amount, expected_amount);
// 		})
// 	}
// }
