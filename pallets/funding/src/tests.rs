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
	mock::{FundingModule, *},
	CurrencyMetadata, Error, ParticipantsSize, Project, TicketSize,
};
use defaults::*;
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::Mutate, ConstU32, OnFinalize, OnIdle, OnInitialize},
	weights::Weight,
};

use sp_runtime::DispatchError;
use std::cell::RefCell;

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
type UserToBalance = Vec<(mock::AccountId, BalanceOf<TestRuntime>)>;
// User -> token_amount, price_per_token, multiplier
type UserToBid = Vec<(
	AccountId,
	(BalanceOf<TestRuntime>, BalanceOf<TestRuntime>, Option<BalanceOf<TestRuntime>>),
)>;

const ISSUER: AccountId = 1;
const EVALUATOR_1: AccountId = 2;
const EVALUATOR_2: AccountId = 3;
const EVALUATOR_3: AccountId = 4;
const BIDDER_1: AccountId = 5;
const BIDDER_2: AccountId = 6;
const BUYER_1: AccountId = 7;
const BUYER_2: AccountId = 8;

const ASSET_DECIMALS: u8 = 12;

const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}"#;

// REMARK: Uncomment if we want to test the events.
// fn last_event() -> RuntimeEvent {
// 	frame_system::Pallet::<TestRuntime>::events()
// 		.pop()
// 		.expect("Event expected")
// 		.event
// }

/// Remove accounts from fundings_1 that are not in fundings_2
fn remove_missing_accounts_from_fundings(
	fundings_1: UserToBalance,
	fundings_2: UserToBalance,
) -> UserToBalance {
	let mut fundings_1 = fundings_1;
	let fundings_2 = fundings_2;
	fundings_1.retain(|(account, _)| {
		fundings_2
			.iter()
			.find_map(|(account_2, _)| if account == account_2 { Some(()) } else { None })
			.is_some()
	});
	fundings_1
}

trait TestInstance {}
trait ProjectInstance {
	fn get_test_environment(&self) -> &TestEnvironment;
	fn get_creator(&self) -> AccountId;
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime>;
	fn get_project(&self) -> ProjectOf<TestRuntime> {
		self.get_test_environment().ext_env.borrow_mut().execute_with(|| {
			FundingModule::projects(self.get_project_id()).expect("Project info should exist")
		})
	}
	fn get_project_info(&self) -> ProjectInfoOf<TestRuntime> {
		self.get_test_environment().ext_env.borrow_mut().execute_with(|| {
			FundingModule::project_info(self.get_project_id()).expect("Project info should exist")
		})
	}
	fn do_project_assertions(
		&self,
		project_assertions: impl Fn(ProjectIdOf<TestRuntime>, &TestEnvironment) -> (),
	) {
		let project_id = self.get_project_id();
		let test_env = self.get_test_environment();
		project_assertions(project_id, test_env);
	}
}

// Initial instance of a test
#[derive(Debug)]
pub struct TestEnvironment {
	pub ext_env: RefCell<sp_io::TestExternalities>,
	pub nonce: RefCell<u64>,
}
impl TestEnvironment {
	pub fn new() -> Self {
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
	/// Returns the *free* fundings of the Users.
	fn get_free_fundings(&self) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToBalance::new();
			let user_keys: Vec<AccountId> =
				frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::free_balance(&user);
				fundings.push((user, funding));
			}
			fundings
		})
	}
	/// Returns the *reserved* fundings of the Users.
	fn get_reserved_fundings(&self, reserve_type: BondType) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToBalance::new();
			let user_keys: Vec<AccountId> =
				frame_system::Account::<TestRuntime>::iter_keys().collect();
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
				Balances::mint_into(&account, amount).expect("Minting should work");
			}
		});
	}
	fn current_block(&self) -> BlockNumber {
		self.ext_env.borrow_mut().execute_with(|| System::block_number())
	}
	fn advance_time(&self, amount: BlockNumber) {
		self.ext_env.borrow_mut().execute_with(|| {
			for _block in 0..amount {
				<AllPalletsWithoutSystem as OnFinalize<u64>>::on_finalize(System::block_number());
				<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(
					System::block_number(),
					Weight::MAX,
				);
				System::set_block_number(System::block_number() + 1);
				<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
			}
		});
	}
	fn do_free_funds_assertions(&self, correct_funds: UserToBalance) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let free = Balances::free_balance(user);
				assert_eq!(free, balance);
			});
		}
	}
	fn do_reserved_funds_assertions(&self, correct_funds: UserToBalance, reserve_type: BondType) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let reserved = Balances::reserved_balance_named(&reserve_type, &user);
				assert_eq!(reserved, balance);
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
		let project = test_env
			.create_project(creator, default_project(test_env.nonce.borrow().clone()))
			.unwrap();
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

		Ok(EvaluatingProject {
			test_env: self.test_env,
			creator: self.creator,
			project_id: self.project_id,
		})
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

	fn bond_for_users(&self, bonds: UserToBalance) -> Result<(), DispatchError> {
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
			Ok(AuctioningProject {
				test_env: self.test_env,
				creator: self.creator,
				project_id: self.project_id,
			})
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
		let creator = evaluating_project.get_creator();

		// Do Evaluation bonding
		evaluating_project
			.bond_for_users(default_evaluation_bonds())
			.expect("Bonding should work");

		// Check that enough funds are reserved
		test_env.do_reserved_funds_assertions(default_evaluation_bonds(), BondType::Evaluation);

		// Check that free funds were reduced
		let mut free_funds = default_fundings();
		// Remove accounts that didnt bond from free_funds
		free_funds = remove_missing_accounts_from_fundings(free_funds, default_evaluation_bonds());
		free_funds = free_funds
			.iter()
			.zip(default_evaluation_bonds().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();
		test_env.do_free_funds_assertions(free_funds.clone());
		let evaluation_end = evaluating_project
			.get_project_info()
			.phase_transition_points
			.evaluation
			.end()
			.expect("Evaluation end point should exist");
		test_env.advance_time(evaluation_end - test_env.current_block() + 2);
		evaluating_project.do_project_assertions(default_evaluation_end_assertions);

		let auctioning_project = evaluating_project.start_auction(creator).unwrap();
		auctioning_project.do_project_assertions(default_auction_start_assertions);

		auctioning_project
	}

	fn bid_for_users(&self, bids: UserToBid) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, (token_amount, price_per_token, multiplier)) in bids {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::bid(
					RuntimeOrigin::signed(account),
					project_id,
					token_amount,
					price_per_token,
					multiplier,
				)
			})?;
		}
		Ok(())
	}

	fn start_community_funding(self) -> CommunityFundingProject<'a> {
		let english_end = self
			.get_project_info()
			.phase_transition_points
			.english_auction
			.end()
			.expect("English end point should exist");
		self.test_env.advance_time(english_end - self.test_env.current_block() + 1);
		let candle_end = self
			.get_project_info()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");
		self.test_env.advance_time(candle_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_info().project_status, ProjectStatus::CommunityRound);
		CommunityFundingProject {
			test_env: self.test_env,
			creator: self.creator,
			project_id: self.project_id,
		}
	}
}

#[derive(Debug)]
pub struct CommunityFundingProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for CommunityFundingProject<'a> {
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
impl<'a> CommunityFundingProject<'a> {
	pub fn new_default(test_env: &'a TestEnvironment) -> Self {
		let auctioning_project = AuctioningProject::new_default(test_env);

		// Do Auction bidding
		auctioning_project
			.bid_for_users(default_auction_bids())
			.expect("Bidding should work");

		// Check our auction was properly interpreted
		test_env.advance_time(1);
		auctioning_project.do_project_assertions(default_auction_end_assertions);

		// Start community funding by moving block to after the end of candle round
		let community_funding_project = auctioning_project.start_community_funding();

		// Check the community funding round started correctly
		community_funding_project.do_project_assertions(default_community_funding_start_assertions);

		community_funding_project
	}

	fn buy_for_users(&self, buys: UserToBalance) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, ct_amount) in buys {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::contribute(RuntimeOrigin::signed(account), project_id, ct_amount)
			})?;
		}
		Ok(())
	}

	fn start_remainder_funding(self) -> RemainderFundingProject<'a> {
		let community_funding_end = self
			.get_project_info()
			.phase_transition_points
			.community
			.end()
			.expect("Community funding end point should exist");
		self.test_env
			.advance_time(community_funding_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_info().project_status, ProjectStatus::RemainderRound);
		RemainderFundingProject {
			test_env: self.test_env,
			creator: self.creator,
			project_id: self.project_id,
		}
	}
}

#[derive(Debug)]
struct RemainderFundingProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for RemainderFundingProject<'a> {
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
impl<'a> RemainderFundingProject<'a> {
	fn new_default(test_env: &'a TestEnvironment) -> Self {
		let community_funding_project = CommunityFundingProject::new_default(test_env);

		// Do community buying
		community_funding_project
			.buy_for_users(default_community_buys())
			.expect("Community buying should work");

		// Check our buys were properly interpreted
		test_env.advance_time(1);
		community_funding_project.do_project_assertions(default_community_funding_end_assertions);

		// Start remainder funding by moving block to after the end of community round
		let remainder_funding_project = community_funding_project.start_remainder_funding();

		// Check the community funding round started correctly
		remainder_funding_project.do_project_assertions(default_remainder_funding_start_assertions);

		remainder_funding_project
	}

	fn finish_project(self) -> FinishedProject<'a> {
		let remainder_funding_end = self
			.get_project_info()
			.phase_transition_points
			.remainder
			.end()
			.expect("Remainder funding end point should exist");
		self.test_env
			.advance_time(remainder_funding_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_info().project_status, ProjectStatus::FundingEnded);
		FinishedProject {
			test_env: self.test_env,
			creator: self.creator,
			project_id: self.project_id,
		}
	}
}

#[derive(Debug)]
struct FinishedProject<'a> {
	test_env: &'a TestEnvironment,
	creator: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for FinishedProject<'a> {
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
impl<'a> FinishedProject<'a> {
	fn new_default(test_env: &'a TestEnvironment) -> Self {
		let remainder_funding_project = RemainderFundingProject::new_default(test_env);

		// End project funding by moving block to after the end of remainder round
		let finished_project = remainder_funding_project.finish_project();

		// Check the community funding round started correctly
		finished_project.do_project_assertions(default_project_end_assertions);

		finished_project
	}
}

mod defaults {
	use super::*;

	pub fn default_project(
		nonce: u64,
	) -> Project<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
		let bounded_name =
			BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
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
			(ISSUER, 20_000 * PLMC),
			(EVALUATOR_1, 35_000 * PLMC),
			(EVALUATOR_2, 60_000 * PLMC),
			(EVALUATOR_3, 100_000 * PLMC),
			(BIDDER_1, 500_000 * PLMC),
			(BIDDER_2, 300_000 * PLMC),
			(BUYER_1, 30_000 * PLMC),
			(BUYER_2, 30_000 * PLMC),
		]

		// 35_000_0_000_000_000
		// 25_000_0_000_000_000
	}

	pub fn default_evaluation_bonds() -> UserToBalance {
		// currently the default project needs 100_000 PLMC to be successful in the evaluation round
		// we assume we will use this bond twice
		vec![
			(EVALUATOR_1, 20_000 * PLMC),
			(EVALUATOR_2, 30_000 * PLMC),
			(EVALUATOR_3, 60_000 * PLMC),
		]
	}

	pub fn default_failing_evaluation_bonds() -> UserToBalance {
		default_evaluation_bonds()
			.into_iter()
			.map(|(user, balance)| (user, balance / 2))
			.collect::<UserToBalance>()
	}

	pub fn default_auction_bids() -> UserToBid {
		// This should reflect the bidding currency, which currently is just PLMC
		vec![(BIDDER_1, (300, 500 * PLMC, Some(1))), (BIDDER_2, (500, 150 * PLMC, Some(1)))]
	}

	pub fn default_token_average_price() -> BalanceOf<TestRuntime> {
		383_3_333_329_500
	}

	pub fn default_auction_bids_plmc_bondings() -> UserToBalance {
		default_auction_bids()
			.into_iter()
			.map(|(user, (amount, price, mult))| (user, (amount * price / mult.unwrap_or(1))))
			.collect()
	}

	pub fn default_auction_bids_bidding_currency_reserved() -> UserToBalance {
		default_auction_bids()
			.into_iter()
			.map(|(user, (amount, price, _mult))| (user, (amount * price)))
			.collect()
	}

	pub fn default_community_buys() -> UserToBalance {
		vec![(BUYER_1, 10), (BUYER_2, 20)]
	}

	// pub fn default_community_funding_plmc_bondings() -> UserToBalance {
	// 	// for now multiplier is always 1, and since plmc and bidding currency are the same,
	// 	// we can just use the same values
	// 	vec![(BUYER_1, (100 * PLMC)), (BUYER_2, (6000 * PLMC))]
	// }

	pub fn default_creation_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::Application);
		});
	}

	pub fn default_evaluation_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::EvaluationRound);
		});
	}

	pub fn default_evaluation_end_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::AuctionInitializePeriod);
		});
	}

	pub fn default_auction_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(
				project_info.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::English)
			);
		});
	}

	pub fn default_auction_end_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		// Check that enough PLMC is bonded
		test_env
			.do_reserved_funds_assertions(default_auction_bids_plmc_bondings(), BondType::Bidding);

		// Check that the bidding currency is reserved
		test_env.ext_env.borrow_mut().execute_with(|| {
			for ((account, plmc_amount), (_, bid_amount)) in default_auction_bids_plmc_bondings()
				.into_iter()
				.zip(default_auction_bids_bidding_currency_reserved().into_iter())
			{
				let bidding_currency_reserve = Balances::reserved_balance(account);
				// Since for now bids use the same pallet as PLMC, the only reserve amount should be the plmc
				assert_eq!(
					bidding_currency_reserve,
					plmc_amount + bid_amount,
					"Bidding currency reserve should be drained"
				);
			}
		});

		// Check that free funds were reduced
		let mut free_funds = default_fundings();
		// Remove accounts that didnt bond from free_funds
		free_funds =
			remove_missing_accounts_from_fundings(free_funds, default_auction_bids_plmc_bondings());
		// Subtract plmc bonded bidding funds
		free_funds = free_funds
			.iter()
			.zip(default_auction_bids_plmc_bondings().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();
		// Subtract bidding currency reserve
		free_funds = free_funds
			.iter()
			.zip(default_auction_bids_bidding_currency_reserved().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();

		test_env.do_free_funds_assertions(free_funds.clone());
	}

	pub fn default_community_funding_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		// Bids that reserved bidding currency, should have that drained from their account on community round, and transfered to the pallet account
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (account, _amount) in default_auction_bids_plmc_bondings() {
				let _bidding_currency_reserve = mock::Balances::reserved_balance(account);
				// Since for now bids use the same pallet as PLMC, the only reserve amount should be the plmc
				// TODO: draining of bid reserves is not implemented yet
				// assert_eq!(bidding_currency_reserve, amount, "Bidding currency reserve should be drained");
			}
		});

		// PLMC should still be reserved, since its only a bond
		test_env
			.do_reserved_funds_assertions(default_auction_bids_plmc_bondings(), BondType::Bidding);

		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::CommunityRound);

			// Check correct weighted_average_price
			let token_price =
				project_info.weighted_average_price.expect("Token price should exist");
			assert_eq!(
				token_price,
				default_token_average_price(),
				"Weighted average token price is incorrect"
			);
		});

		// Check status of bids
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_bids = crate::pallet::AuctionsInfo::<TestRuntime>::iter_prefix(project_id)
				.collect::<Vec<_>>();
			let project_info = FundingModule::project_info(project_id).unwrap();
			assert!(
				matches!(project_info.weighted_average_price, Some(_)),
				"Weighted average price should exist"
			);
			assert!(project_bids
				.into_iter()
				.find(|(_bidder, bids)| {
					!bids.iter().all(|bid| bid.status == BidStatus::Accepted)
				})
				.is_none());
		});
	}

	pub fn default_community_funding_end_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		let token_price = test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			project_info.weighted_average_price.expect("Token price should exist")
		});
		let expected_plmc_bondings = default_community_buys()
			.iter()
			.map(|(account, amount)| {
				let plmc_amount = amount * token_price;
				(*account, plmc_amount)
			})
			.collect::<UserToBalance>();

		// Check that enough PLMC is bonded
		test_env
			.do_reserved_funds_assertions(expected_plmc_bondings.clone(), BondType::Contributing);

		// Check that free funds were reduced
		let mut free_funds = default_fundings();
		// Remove accounts that didnt bond from free_funds
		free_funds = remove_missing_accounts_from_fundings(free_funds, default_community_buys());
		// Subtract the amount spent on the buys from the free funds
		free_funds = free_funds
			.iter()
			.zip(expected_plmc_bondings.clone().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();
		// Subtract the amount reserved for the PLMC bonding, since we use the same pallet for now
		free_funds = free_funds
			.iter()
			.zip(expected_plmc_bondings.iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();

		test_env.do_free_funds_assertions(free_funds.clone());
	}

	pub fn default_remainder_funding_start_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::RemainderRound);
		});
	}

	pub fn default_project_end_assertions(
		project_id: ProjectIdOf<TestRuntime>,
		test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_info =
				FundingModule::project_info(project_id).expect("Project info should exist");
			assert_eq!(project_info.project_status, ProjectStatus::FundingEnded);
		});
	}
}

#[cfg(test)]
mod helper_functions {
	use super::*;
	#[test]
	fn remove_from_update_store_works() {
		let test_env = TestEnvironment::new();
		let now = test_env.current_block();
		test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::add_to_update_store(now + 10u64, &42u32).unwrap();
			FundingModule::add_to_update_store(now + 20u64, &69u32).unwrap();
			FundingModule::add_to_update_store(now + 5u64, &404u32).unwrap();
		});
		test_env.advance_time(2u64);
		test_env.ext_env.borrow_mut().execute_with(|| {
			let stored = crate::ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored.len(), 3, "There should be 3 blocks scheduled for updating");

			FundingModule::remove_from_update_store(&69u32).unwrap();

			let stored = crate::ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored[2], vec![], "Vector should be empty for that block after deletion");
		});
	}
}

#[cfg(test)]
mod creation_round_success {
	use super::*;

	#[test]
	fn basic_plmc_transfer_works() {
		let test_env = TestEnvironment::new();
		test_env.fund_accounts(default_fundings());
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_ok!(Balances::transfer(
				RuntimeOrigin::signed(EVALUATOR_1),
				EVALUATOR_2,
				1 * PLMC
			));
		});
	}

	#[test]
	fn create_works() {
		let test_env = TestEnvironment::new();
		let _project = CreatedProject::new_default(&test_env);
	}

	#[test]
	fn project_id_autoincrement_works() {
		let test_env = TestEnvironment::new();

		let project_1 = CreatedProject::new_default(&test_env);
		let project_2 = CreatedProject::new_default(&test_env);
		let project_3 = CreatedProject::new_default(&test_env);

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
			let project = default_project(0);
			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(BIDDER_1), project),
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
		test_env.fund_accounts(default_fundings());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into(),);
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
		test_env.fund_accounts(default_fundings());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::ParticipantsSizeError.into(),);
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
		test_env.fund_accounts(default_fundings());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
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
		test_env.fund_accounts(default_fundings());
		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
	}
}

#[cfg(test)]
mod evaluation_round_success {
	use super::*;
	use crate::AuctionPhase::English;

	#[test]
	fn evaluation_start_works() {
		let test_env = TestEnvironment::new();
		let _evaluating_project = EvaluatingProject::new_default(&test_env);
	}

	#[test]
	fn evaluation_end_works() {
		let test_env = TestEnvironment::new();
		let _auctioning_project = AuctioningProject::new_default(&test_env);
	}

	#[test]
	fn automatic_transition_works() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);
		evaluating_project.bond_for_users(default_evaluation_bonds()).unwrap();
		let project_info = evaluating_project.get_project_info();
		test_env
			.advance_time(project_info.phase_transition_points.evaluation.end().unwrap() + 1u64);
		let end_block = evaluating_project
			.get_project_info()
			.phase_transition_points
			.auction_initialize_period
			.end()
			.unwrap();
		test_env.advance_time(end_block - test_env.current_block() + 1);
		assert_eq!(
			evaluating_project.get_project_info().project_status,
			ProjectStatus::AuctionRound(English)
		);
	}
}

#[cfg(test)]
mod evaluation_round_failure {
	use super::*;

	#[test]
	fn not_enough_bonds() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);

		// Partially bond for evaluation
		evaluating_project
			.bond_for_users(default_failing_evaluation_bonds())
			.expect("Bonding should work");

		// Check that enough funds are reserved
		test_env
			.do_reserved_funds_assertions(default_failing_evaluation_bonds(), BondType::Evaluation);

		// Check that free funds were reduced
		let mut free_funds = default_fundings();

		// Remove accounts that didnt bond from free_funds
		free_funds =
			remove_missing_accounts_from_fundings(free_funds, default_failing_evaluation_bonds());
		free_funds = free_funds
			.iter()
			.zip(default_failing_evaluation_bonds().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();
		test_env.do_free_funds_assertions(free_funds.clone());

		let evaluation_end = evaluating_project
			.get_project_info()
			.phase_transition_points
			.evaluation
			.end()
			.expect("Evaluation end point should exist");
		test_env.advance_time(evaluation_end - test_env.current_block() + 2);
		let project_info = evaluating_project.get_project_info();
		assert_eq!(project_info.project_status, ProjectStatus::EvaluationFailed);
		// Check that on_idle has unlocked the failed bonds
		test_env.advance_time(10);
		test_env.do_free_funds_assertions(default_fundings());
	}

	#[test]
	fn insufficient_balance_bonding() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);

		// Try to bond twice as much as the second user of default_fundings has
		let mut user_funding = default_fundings()[1];
		user_funding.1 *= 2;

		let dispatch_error = evaluating_project.bond_for_users(vec![user_funding]).unwrap_err();
		assert_eq!(dispatch_error, Error::<TestRuntime>::InsufficientBalance.into())
	}
}

#[cfg(test)]
mod auction_round_success {
	use super::*;

	#[test]
	fn auction_works() {
		let test_env = TestEnvironment::new();
		let _community_funding_project = CommunityFundingProject::new_default(&test_env);
	}

	#[test]
	fn only_candle_bids_before_random_block_get_included() {
		let test_env = TestEnvironment::new();
		let auctioning_project = AuctioningProject::new_default(&test_env);
		let english_end_block = auctioning_project
			.get_project_info()
			.phase_transition_points
			.english_auction
			.end()
			.expect("Auction start point should exist");
		// The block following the end of the english auction, is used to transition the project into candle auction.
		// We move past that transition, into the start of the candle auction.
		test_env.advance_time(english_end_block - test_env.current_block() + 1);
		assert_eq!(
			auctioning_project.get_project_info().project_status,
			ProjectStatus::AuctionRound(AuctionPhase::Candle)
		);

		let candle_end_block = auctioning_project
			.get_project_info()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle auction end point should exist");

		// do one candle bid for each block until the end of candle auction with a new user
		let mut bidding_account = 1000;
		let bid_info = default_auction_bids()[0].1;
		let necessary_funding =
			(bid_info.0 * bid_info.1) + (bid_info.0 * bid_info.1 * bid_info.2.unwrap() as u128);
		let mut bids_made: UserToBid = vec![];
		let starting_bid_block = test_env.current_block();
		assert_eq!(test_env.current_block(), starting_bid_block);
		let blocks_to_bid = test_env.current_block()..candle_end_block;
		for _block in blocks_to_bid {
			assert_eq!(
				auctioning_project.get_project_info().project_status,
				ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			test_env.fund_accounts(vec![(bidding_account, necessary_funding)]);
			let bids: UserToBid = vec![(bidding_account, bid_info)];
			auctioning_project
				.bid_for_users(bids.clone())
				.expect("Candle Bidding should not fail");
			bids_made.push(bids[0]);
			bidding_account += 1;
			test_env.advance_time(1);
		}
		test_env.advance_time(candle_end_block - test_env.current_block() + 1);

		let random_end = auctioning_project
			.get_project_info()
			.phase_transition_points
			.random_candle_ending
			.expect("Random auction end point should exist");

		let split = (random_end - starting_bid_block + 1) as usize;
		let excluded_bids = bids_made.split_off(split);
		let included_bids = bids_made;
		let _weighted_price = auctioning_project
			.get_project_info()
			.weighted_average_price
			.expect("Weighted price should exist");
		// let all_bids = test_env.ext_env.borrow_mut().execute_with(|| {
		// 	AuctionsInfo::<TestRuntime>::iter_prefix_values(auctioning_project.project_id)
		// 		.map(|bids| bids[0].clone())
		// 		.collect::<Vec<_>>()
		// });
		for (bidder, (_amount, _price, _multiplier)) in included_bids {
			test_env.ext_env.borrow_mut().execute_with(|| {
				let pid = auctioning_project.project_id;
				let stored_bid =
					FundingModule::auctions_info(pid, bidder).expect("Bid should exist");
				assert!(
					matches!(
						stored_bid[0],
						BidInfo {
							project: _pid,
							amount: _,
							price: _,
							ticket_size: _,
							bidder: _,
							..
						}
					),
					"Stored bid does not match the bid that was made"
				);
				assert_eq!(stored_bid[0].status, BidStatus::Accepted, "Bid should be accepted");
			});
		}

		for (bidder, (_amount, _price, _multiplier)) in excluded_bids {
			test_env.ext_env.borrow_mut().execute_with(|| {
				let pid = auctioning_project.project_id;
				let stored_bid =
					FundingModule::auctions_info(pid, bidder).expect("Bid should exist");
				assert!(
					matches!(
						stored_bid[0],
						BidInfo {
							project: _pid,
							amount: _,
							price: _,
							ticket_size: _,
							bidder: _,
							..
						}
					),
					"Stored bid does not match the bid that was made"
				);
				assert_eq!(
					stored_bid[0].status,
					BidStatus::Rejected(RejectionReason::AfterCandleEnd),
					"Bid should not be accepted after candle random end"
				);
			});
		}
	}
}

#[cfg(test)]
mod auction_round_failure {
	use super::*;
	#[test]
	fn cannot_start_auction_before_evaluation_finishes() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);
		let project_id = evaluating_project.project_id;
		let creator = evaluating_project.creator;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::start_auction(RuntimeOrigin::signed(creator), project_id),
				Error::<TestRuntime>::EvaluationPeriodNotEnded
			);
		});
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);
		let _project_id = evaluating_project.project_id;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::bid(RuntimeOrigin::signed(BIDDER_2), 0, 1, 100, None),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn contribute_does_not_work() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);
		let project_id = evaluating_project.project_id;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::contribute(RuntimeOrigin::signed(BIDDER_1), project_id, 100),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn bids_overflow() {
		let test_env = TestEnvironment::new();
		let auctioning_project = AuctioningProject::new_default(&test_env);
		let project_id = auctioning_project.project_id;
		const DAVE: AccountId = 42;
		let bids: UserToBid = vec![
			(DAVE, (10_000, 2 * PLMC, Some(1))),
			(DAVE, (13_000, 3 * PLMC, Some(1))),
			(DAVE, (15_000, 5 * PLMC, Some(1))),
			(DAVE, (1_000, 7 * PLMC, Some(1))),
			(DAVE, (20_000, 8 * PLMC, Some(1))),
		];

		let mut fundings: UserToBalance = bids
			.iter()
			.map(|(user, (amount, price, _))| (*user, *amount * *price))
			.collect::<Vec<_>>();
		// Existential deposit on DAVE
		fundings.push((DAVE, 100 * PLMC));

		let _free_balance = fundings.iter().fold(0, |acc, (_, balance)| acc + balance);

		// Fund enough for all bids
		test_env.fund_accounts(fundings.clone());

		// Fund enough for all PLMC bonds for the bids (multiplier of 1)
		test_env.fund_accounts(fundings.clone());

		auctioning_project.bid_for_users(bids).expect("Bids should pass");

		test_env.ext_env.borrow_mut().execute_with(|| {
			let stored_bids = FundingModule::auctions_info(project_id, DAVE).unwrap();
			assert_eq!(stored_bids.len(), 4);
			assert_eq!(stored_bids[0].ticket_size, 20_000 * 8 * PLMC);
			assert_eq!(stored_bids[1].ticket_size, 1_000 * 7 * PLMC);
			assert_eq!(stored_bids[2].ticket_size, 15_000 * 5 * PLMC);
			assert_eq!(stored_bids[3].ticket_size, 13_000 * 3 * PLMC);
		});
	}
}

#[cfg(test)]
mod community_round_success {
	use super::*;
	pub const HOURS: BlockNumber = 300u64;

	#[test]
	fn community_round_works() {
		let test_env = TestEnvironment::new();
		let _community_funding_project = CommunityFundingProject::new_default(&test_env);
	}

	#[test]
	fn remainder_round_works() {
		let test_env = TestEnvironment::new();
		let _remainder_funding_project = RemainderFundingProject::new_default(&test_env);
	}

	#[test]
	fn contribute_multiple_times_works() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_default(&test_env);
		const BOB: AccountId = 42;
		let buyers_1: UserToBalance = vec![(BOB, 3_000 * PLMC), (BOB, 3_000 * PLMC)];
		let buyers_2: UserToBalance = vec![(BOB, 40_000 * PLMC), (BOB, 13_550 * PLMC)];
		// Fund for buy
		test_env.fund_accounts(buyers_1.clone());
		// Fund for PLMC bond
		test_env.fund_accounts(buyers_1.clone());
		// Fund for buy
		test_env.fund_accounts(buyers_2.clone());
		// Fund for PLMC bond
		test_env.fund_accounts(buyers_2.clone());

		// TODO: Set a reasonable amount of Contribution Tokens that the user wants to buy
		community_funding_project
			.buy_for_users(vec![(BOB, 3)])
			.expect("The Buyer should be able to buy multiple times");
		test_env.advance_time((1 * HOURS) as BlockNumber);
		community_funding_project
			.buy_for_users(vec![(BOB, 4)])
			.expect("The Buyer should be able to buy multiple times");
	}

	#[test]
	fn contribution_is_returned_on_limit_reached() {
		let test_env = TestEnvironment::new();
		let project = CommunityFundingProject::new_default(&test_env);
		let range = 0..<TestRuntime as crate::Config>::MaxContributionsPerUser::get();
		let mut contributions: UserToBalance = range.map(|_| (BUYER_2, 1)).collect();
		// Reach the limit of contributions for a user-project
		project.buy_for_users(contributions.clone()).unwrap();
		// TODO: wait until multiplier is added to the contribute extrinsic, so a contribution can have different PLMC bondings, and so an old contribution can be drop once the limit is reached
		assert!(false);
	}
}

#[cfg(test)]
mod community_round_failure {
	// TODO: Maybe here we can test what happens if we sell all the CTs in the community round
}

#[cfg(test)]
mod purchased_vesting {
	use super::*;

	#[test]
	fn contribution_token_mints() {
		// TODO: currently the vesting is limited to the whole payment at once. We should test it with several payments over a vesting period.
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let token_price = finished_project
			.get_project_info()
			.weighted_average_price
			.expect("CT price should exist at this point");
		let buyers = default_community_buys();
		let project = finished_project.get_project();
		let decimals = project.token_information.decimals;

		test_env.ext_env.borrow_mut().execute_with(|| {
			for (buyer, amount) in buyers {
				let _token_amount = amount / token_price;
				assert_ok!(FundingModule::vested_contribution_token_purchase_mint_for(
					RuntimeOrigin::signed(buyer),
					project_id,
					buyer
				));

				let minted_balance = Assets::balance(project_id, buyer);
				let desired_balance = FundingModule::add_decimals_to_number(amount, decimals);
				assert_eq!(minted_balance, desired_balance);
			}
		});
	}

	#[test]
	fn plmc_unbonded() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let buyers = default_community_buys();
		let price = finished_project
			.get_project_info()
			.weighted_average_price
			.expect("CT price should exist at this point");
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (buyer, token_amount) in buyers {
				let theoretical_bonded_plmc = token_amount * price;
				let actual_bonded_plmc =
					Balances::reserved_balance_named(&BondType::Contributing, &buyer);
				assert_eq!(theoretical_bonded_plmc, actual_bonded_plmc);
				assert_ok!(FundingModule::vested_plmc_purchase_unbond_for(
					RuntimeOrigin::signed(buyer),
					project_id,
					buyer
				));
				let actual_bonded_plmc =
					Balances::reserved_balance_named(&BondType::Contributing, &buyer);
				assert_eq!(actual_bonded_plmc, 0u32.into());
			}
		});
	}
}

#[cfg(test)]
mod bids_vesting {
	use super::*;

	#[test]
	fn contribution_token_mints() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let bidders = default_auction_bids();
		let project = finished_project.get_project();
		let decimals = project.token_information.decimals;
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (bidder, (amount, _price, _multiplier)) in bidders {
				assert_ok!(FundingModule::vested_contribution_token_bid_mint_for(
					RuntimeOrigin::signed(bidder),
					project_id,
					bidder
				));
				let minted_balance = Assets::balance(project_id, bidder);
				let desired_balance = FundingModule::add_decimals_to_number(amount, decimals);

				assert_eq!(minted_balance, desired_balance);
			}
		});
	}

	#[test]
	fn plmc_unbonded() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let bidders = default_auction_bids();
		let project = finished_project.get_project();
		let _decimals = project.token_information.decimals;
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (bidder, (amount, price, multiplier)) in bidders {
				let theoretical_bonded_plmc = (amount * price) / multiplier.unwrap();
				let actual_bonded_plmc =
					Balances::reserved_balance_named(&BondType::Bidding, &bidder);
				assert_eq!(theoretical_bonded_plmc, actual_bonded_plmc);
				assert_ok!(FundingModule::vested_plmc_bid_unbond_for(
					RuntimeOrigin::signed(bidder),
					project_id,
					bidder
				));
				let actual_bonded_plmc =
					Balances::reserved_balance_named(&BondType::Bidding, &bidder);
				assert_eq!(actual_bonded_plmc, 0u32.into());
			}
		});
	}
}
