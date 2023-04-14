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

use defaults::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use super::*;
use crate::{
	mock::*,
	CurrencyMetadata, Error, ParticipantsSize, Project, TicketSize,
	mock::FundingModule
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{tokens::fungibles::Inspect, ConstU32, Get, Hooks},
	weights::Weight,
};
use sp_io::TestExternalities;
use sp_runtime::DispatchError;
use std::cell::{RefCell, RefMut};
use frame_support::traits::{OnFinalize, OnInitialize};

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
type UserToBalance = Vec<(mock::AccountId, BalanceOf<TestRuntime>)>;
// User -> token_amount, price_per_token, multiplier
type UserToBid = Vec<(AccountId, (BalanceOf<TestRuntime>, BalanceOf<TestRuntime>, Option<u32>))>;

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 4;
const EVE: AccountId = 5;
const FERDIE: AccountId = 6;
const GINA: AccountId = 7;

const ASSET_DECIMALS: u8 = 12;

const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}"#;

fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<TestRuntime>::events()
		.pop()
		.expect("Event expected")
		.event
}

/// Remove accounts from fundings_1 that are not in fundings_2
fn remove_missing_accounts_from_fundings(fundings_1: UserToBalance, fundings_2: UserToBalance) -> UserToBalance {
	let mut fundings_1 = fundings_1;
	let mut fundings_2 = fundings_2;
	fundings_1.retain(|(account, _)| {
		fundings_2.iter().find_map(|(account_2, _)| {
			if account == account_2 {
				Some(())
			} else {
				None
			}
		}).is_some()

	});
	fundings_1
}

trait TestInstance {}
trait ProjectInstance {
	fn get_test_environment(&self) -> &TestEnvironment;
	fn get_creator(&self) -> AccountId;
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime>;
	fn get_project_info(&self) -> ProjectInfoOf<TestRuntime> {
		self.get_test_environment().ext_env.borrow_mut().execute_with(|| {
			FundingModule::project_info(self.get_project_id()).expect("Project info should exist")
		})
	}
	fn do_project_assertions(
		&self,
		project_assertions: impl Fn(ProjectIdOf<TestRuntime>, RefMut<TestExternalities>) -> (),
	) {
		let project_id = self.get_project_id();
		let test_env = self.get_test_environment();
		project_assertions(project_id, test_env.ext_env.borrow_mut());
	}
}

// Initial instance of a test
#[derive(Debug)]
pub struct TestEnvironment {
	ext_env: RefCell<sp_io::TestExternalities>,
	nonce: RefCell<u64>,
}
impl TestEnvironment {
	fn new() -> Self {
		Self { ext_env: RefCell::new(new_test_ext()), nonce: RefCell::new(0u64) }
	}
	fn create_project(
		&self,
		creator: mock::AccountId,
		project: ProjectOf<TestRuntime>,
	) -> Result<CreatedProject, DispatchError> {
		// Create project in the externalities environment of this struct instance
		self.ext_env
			.borrow_mut()
			.execute_with(|| FundingModule::create(RuntimeOrigin::signed(creator), project))?;

		// Retrieve the project_id from the events
		let project_id = self.ext_env.borrow_mut().execute_with(|| {
			frame_system::Pallet::<TestRuntime>::events()
				.iter()
				.filter_map(|event| match event.event {
					RuntimeEvent::FundingModule(crate::Event::Created { project_id }) =>
						Some(project_id),
					_ => None,
				})
				.last()
				.expect("Project created event expected")
				.clone()
		});

		Ok(CreatedProject { test_env: self, creator, project_id })
	}
	fn get_free_fundings(&self) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::free_balance(&user);
				fundings.push((user, funding));
			}
			fundings
		})
	}
	fn get_reserved_fundings(&self, reserve_type: BondType) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::reserved_balance_named(&reserve_type, &user);
				fundings.push((user, funding));
			}
			fundings
		})
	}
	fn fund_accounts(&self, fundings: UserToBalance) {
		self.ext_env.borrow_mut().execute_with(|| {
			for (account, amount) in fundings {
				Balances::make_free_balance_be(&account, amount);
			}
		});
	}
	fn current_block(&self) -> BlockNumber {
		self.ext_env.borrow_mut().execute_with(|| System::block_number())
	}
	fn advance_time(&self, amount: BlockNumber) {
		self.ext_env.borrow_mut().execute_with(|| {
			for _block in 0..amount {
				<AllPalletsWithSystem as OnFinalize<u64>>::on_finalize(System::block_number());
				System::set_block_number(System::block_number() + 1);
				<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
			}
		});
	}
	fn do_free_funds_assertions(
		&self,
		correct_funds: UserToBalance
	) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				assert_eq!(Balances::free_balance(user), balance);
			});
		}
	}
	fn do_reserved_funds_assertions(
		&self,
		correct_funds: UserToBalance,
		reserve_type: BondType
	) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				assert_eq!(Balances::reserved_balance_named(&reserve_type, &user), balance);
			});
		}
	}
}

#[derive(Debug, Clone)]
pub struct CreatedProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for CreatedProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_creator(&self) -> AccountId {
		self.creator.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> CreatedProject<'a> {
	fn new_default(test_env: &'a TestEnvironment) -> Self {
		test_env.fund_accounts(default_fundings());
		let creator = default_fundings()[0].0;
		let project = test_env.create_project(creator, default_project(test_env.nonce.borrow().clone())).unwrap();
		project.do_project_assertions(default_creation_assertions);
		*test_env.nonce.borrow_mut() += 1;
		project
	}

	// Move to next project phase
	fn start_evaluation(
		self,
		caller: mock::AccountId,
	) -> Result<EvaluatingProject<'a>, DispatchError> {
		self.test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::start_evaluation(RuntimeOrigin::signed(caller), self.project_id)
		})?;

		Ok(EvaluatingProject { test_env: self.test_env, creator: self.creator, project_id: self.project_id })
	}
}

#[derive(Debug, Clone)]
struct EvaluatingProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for EvaluatingProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_creator(&self) -> AccountId {
		self.creator.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> EvaluatingProject<'a> {
	fn new_default(test_env: &'a TestEnvironment) -> Self {
		let created_project = CreatedProject::new_default(test_env);
		let creator = created_project.get_creator();
		let evaluating_project = created_project.start_evaluation(creator).unwrap();
		test_env.advance_time(1 as BlockNumber);
		evaluating_project.do_project_assertions(default_evaluation_start_assertions);
		evaluating_project
	}

	fn bond_for_users(
		&self,
		bonds: UserToBalance,
	) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, amount) in bonds {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::bond_evaluation(RuntimeOrigin::signed(account), project_id, amount)
			})?;
		}
		Ok(())
	}

	fn start_auction(self, caller: AccountId) -> Result<AuctioningProject<'a>, DispatchError> {
		self.test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::start_auction(RuntimeOrigin::signed(caller), self.project_id)?;
			Ok(AuctioningProject { test_env: self.test_env, creator: self.creator, project_id: self.project_id })
		})
	}
}

#[derive(Debug)]
struct AuctioningProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for AuctioningProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_creator(&self) -> AccountId {
		self.creator.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}

impl<'a> AuctioningProject<'a> {
	fn new_default(test_env: &'a TestEnvironment) -> Self {
		let evaluating_project = EvaluatingProject::new_default(test_env);

		// Do Evaluation bonding
		evaluating_project.bond_for_users(default_evaluation_bonds()).expect("Bonding should work");

		// Check that enough funds are reserved
		test_env.do_reserved_funds_assertions(
			default_evaluation_bonds(),
			BondType::Evaluation
		);

		// Check that free funds were reduced
		let mut free_funds = default_fundings();
		// Remove accounts that didnt bond from free_funds
		free_funds = remove_missing_accounts_from_fundings(free_funds, default_evaluation_bonds());
		free_funds = free_funds.iter().zip(default_evaluation_bonds().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			}).collect::<UserToBalance>();
		test_env.do_free_funds_assertions(free_funds.clone());

		let evaluation_end = evaluating_project.get_project_info().phase_transition_points.evaluation.end().expect("Evaluation end point should exist");
		test_env.advance_time(evaluation_end - test_env.current_block() + 1);
		evaluating_project.do_project_assertions(default_evaluation_end_assertions);

		let auctioning_project = evaluating_project.start_auction(evaluating_project.creator).unwrap();
		auctioning_project.do_project_assertions(default_auction_start_assertions);

		auctioning_project
	}

	fn bid_for_users(
		&self,
		bids: UserToBid,
	) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, (token_amount, price_per_token, multiplier)) in bids {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::bid(RuntimeOrigin::signed(account), project_id, token_amount, price_per_token, multiplier)
			})?;
		}
		Ok(())
	}
}

mod defaults {
	use super::*;

	pub fn default_project(nonce: u64) -> Project<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let mut metadata_hash = hashed(METADATA);
		let mut rng = ChaCha8Rng::seed_from_u64(nonce);
		metadata_hash.randomize_using(&mut rng);
		Project {
			total_allocation_size: 1_000_000,
			minimum_price: 1 * PLMC,
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: Default::default(),
			metadata: Some(metadata_hash),
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
		}
	}

	pub fn default_fundings() -> UserToBalance {
		vec![
			(ALICE, 20_000 * PLMC),
			(BOB, 500_000 * PLMC),
			(CHARLIE, 300_000 * PLMC),
			(DAVE, 30_000 * PLMC),
			(EVE, 35_000 * PLMC),
			(FERDIE, 60_000 * PLMC),
			(GINA, 100_000 * PLMC),
		]
	}

	pub fn default_evaluation_bonds() -> UserToBalance {
		// currently the default project needs 100_000 PLMC to be successful in the evaluation round
		// we assume we will use this bond twice
		vec![
			(EVE, 20_000 * PLMC),
			(FERDIE, 30_000 * PLMC),
			(GINA, 60_000 * PLMC),
		]
	}

	pub fn default_auction_bids() -> UserToBid {
		vec![
			(BOB, (3*PLMC, 50_000,Some(1))),
			(CHARLIE, (5*PLMC, 15_000,Some(1)))
		]
	}

	pub fn default_creation_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		mut test_ext: RefMut<TestExternalities>,
	) {
		test_ext.execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::Application);
		});
	}

	pub fn default_evaluation_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		mut test_ext: RefMut<TestExternalities>,
	) {
		test_ext.execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
		});
	}

	pub fn default_evaluation_end_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		mut test_ext: RefMut<TestExternalities>,
	) {
		test_ext.execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
		});
	}

	pub fn default_auction_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		mut test_ext: RefMut<TestExternalities>,
	) {
		test_ext.execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::AuctionRound(AuctionPhase::English));
		});
	}
}

#[cfg(test)]
mod creation_round_success {
	use super::*;

	#[test]
	fn create_works() {
		let mut test_env = TestEnvironment::new();
		let project = CreatedProject::new_default(&test_env);
	}

	#[test]
	fn project_id_autoincrement_works() {
		let mut test_env = TestEnvironment::new();

		let project_1 = CreatedProject::new_default(&test_env);
		let project_2 = CreatedProject::new_default(&test_env);
		let project_3 = CreatedProject::new_default(&test_env);

		assert_eq!(project_1.project_id, 0);
		assert_eq!(project_2.project_id, 1);
		assert_eq!(project_3.project_id, 2);
	}
}

// #[cfg(test)]
// mod creation_round_failure {
// 	use super::*;
//
// 	#[test]
// 	#[ignore]
// 	fn only_with_credential_can_create() {
// 		new_test_ext().execute_with(|| {
// 			let project = default_project();
// 			assert_noop!(
// 				FundingModule::create(RuntimeOrigin::signed(BOB), project),
// 				Error::<TestRuntime>::NotAuthorized
// 			);
// 		})
// 	}
//
// 	#[test]
// 	fn price_too_low() {
// 		let wrong_project: ProjectOf<TestRuntime> = Project {
// 			minimum_price: 0,
// 			ticket_size: TicketSize { minimum: Some(1), maximum: None },
// 			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
// 			metadata: Some(hashed(METADATA)),
// 			..Default::default()
// 		};
//
// 		let test_env = TestEnvironment::new();
// 		test_env.fund_accounts(default_fundings());
//
// 		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
// 		assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into(),);
// 	}
//
// 	#[test]
// 	fn participants_size_error() {
// 		let wrong_project: ProjectOf<TestRuntime> = Project {
// 			minimum_price: 1,
// 			ticket_size: TicketSize { minimum: Some(1), maximum: None },
// 			participants_size: ParticipantsSize { minimum: None, maximum: None },
// 			metadata: Some(hashed(METADATA)),
// 			..Default::default()
// 		};
//
// 		let test_env = TestEnvironment::new();
// 		test_env.fund_accounts(default_fundings());
//
// 		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
// 		assert_eq!(project_err, Error::<TestRuntime>::ParticipantsSizeError.into(),);
// 	}
//
// 	#[test]
// 	fn ticket_size_error() {
// 		let wrong_project: ProjectOf<TestRuntime> = Project {
// 			minimum_price: 1,
// 			ticket_size: TicketSize { minimum: None, maximum: None },
// 			participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
// 			metadata: Some(hashed(METADATA)),
// 			..Default::default()
// 		};
//
// 		let test_env = TestEnvironment::new();
// 		test_env.fund_accounts(default_fundings());
//
// 		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
// 		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
// 	}
//
// 	#[test]
// 	#[ignore = "ATM only the first error will be thrown"]
// 	fn multiple_field_error() {
// 		let wrong_project: ProjectOf<TestRuntime> = Project {
// 			minimum_price: 0,
// 			ticket_size: TicketSize { minimum: None, maximum: None },
// 			participants_size: ParticipantsSize { minimum: None, maximum: None },
// 			..Default::default()
// 		};
// 		let test_env = TestEnvironment::new();
// 		test_env.fund_accounts(default_fundings());
// 		let project_err = test_env.create_project(ALICE, wrong_project).unwrap_err();
// 		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
// 	}
// }
//
// #[cfg(test)]
// mod evaluation_round_success {
// 	use parachains_common::DAYS;
// 	use super::{creation_round_success::do_create_works, *};
//
// 	pub fn do_evaluation_works(test_env: &TestEnvironment) -> EvaluatingProject{
// 		let mut created_project = do_create_works(test_env);
//
//
// 	}
//
// 	#[test]
// 	fn evaluation_works() {
// 		let mut test_env = TestEnvironment::new();
// 		do_evaluation_works(&mut test_env);
// 	}
// }
//
// #[cfg(test)]
// mod evaluation_round_failure {
// 	use super::*;
// 	use super::creation_round_success::{ do_create_works };
// 	use super::evaluation_round_success::{ default_evaluation_start_assertions };
//
// 	#[test]
// 	fn not_enough_bonds() {
// 		let mut test_env = TestEnvironment::new();
// 		let project = do_create_works(&test_env);
//
// 		// Start evaluation round
// 		let project = project.start_evaluation(project.get_creator()).unwrap();
//
// 		// Check that project correctly transitioned round
// 		test_env.advance_time(1 as BlockNumber);
// 		project.do_project_assertions(default_evaluation_start_assertions);
//
// 		// Partially bond for evaluation
// 		project.bond_for_users(default_evaluation_bonds()).expect("Bonding should work");
//
// 		// Check that enough funds are reserved
// 		test_env.do_reserved_funds_assertions(
// 			default_evaluation_bonds(),
// 			BondType::Evaluation
// 		);
//
// 		// Check that free funds were reduced
// 		let mut free_funds = default_fundings();
// 		// Remove accounts that didnt bond from free_funds
// 		free_funds = remove_missing_accounts_from_fundings(free_funds, default_evaluation_bonds());
// 		free_funds = free_funds.iter().zip(default_evaluation_bonds().iter())
// 			.map(|(original, bonded)| {
// 				assert_eq!(original.0, bonded.0, "User should be the same");
// 				(original.0, original.1 - bonded.1)
// 			}).collect::<UserToBalance>();
// 		test_env.do_free_funds_assertions(free_funds.clone());
//
//
// 		let evaluation_end = project.get_project_info().phase_transition_points.evaluation.end().expect("Evaluation end point should exist");
// 		test_env.advance_time(evaluation_end - test_env.current_block() + 1);
// 		let project_info = project.get_project_info();
// 		assert_eq!(project_info.project_status, ProjectStatus::EvaluationFailed);
// 	}
//
// 	#[test]
// 	fn insufficient_balance_bonding() {
// 		let mut test_env = TestEnvironment::new();
// 		let project = do_create_works(&test_env);
//
// 		// Start evaluation round
// 		let project = project.start_evaluation(project.get_creator()).unwrap();
//
// 		// Check that project correctly transitioned round
// 		test_env.advance_time(1 as BlockNumber);
// 		project.do_project_assertions(default_evaluation_start_assertions);
//
// 		// Try to bond twice as much as the second user of default_fundings has
// 		let mut user_funding = default_fundings()[1];
// 		user_funding.1 *= 2;
//
// 		let dispatch_error = project.bond_for_users(vec![user_funding]).unwrap_err();
// 		assert_eq!(dispatch_error, Error::<TestRuntime>::InsufficientBalance.into())
// 	}
//
// }
//
// #[cfg(test)]
// mod auction_round_success {
// 	use super::*;
// 	use super::evaluation_round_success::do_evaluation_works;
//
// 	fn do_auction_works(test_env: &TestEnvironment) {
// 		let project = do_evaluation_works(test_env);
//
// 		// Start auction round
// 		let project = project.start_auction(project.get_creator()).unwrap();
//
// 		// Check that project correctly transitioned round
// 		test_env.advance_time(1 as BlockNumber);
// 		project.do_project_assertions(default_auction_start_assertions);
//
// 		// Get current free fundings
// 		let free_fundings = test_env.get_free_fundings();
//
// 		// Bid on english auction
// 		let mut bids = default_auction_bids();
// 		project.bid_for_users(bids).expect("Bidding should work");
// 	}
//
// 	#[test]
// 	fn auction_works() {
// 		let mut test_env = TestEnvironment::new();
// 		do_auction_works(&test_env);
// 	}
// }
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
