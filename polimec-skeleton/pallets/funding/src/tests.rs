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
	CurrencyMetadata, Error, ParticipantsSize, ProjectMetadata, TicketSize,
};
use defaults::*;
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{InspectHold as FungibleInspectHold, Mutate as FungibleMutate},
		fungibles::Mutate as FungiblesMutate,
		tokens::Balance as BalanceT,
		ConstU32, OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
};
use helper_functions::*;

use sp_runtime::DispatchError;
use std::cell::RefCell;

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
type UserToBalance = Vec<(AccountId, BalanceOf<TestRuntime>)>;
// User -> token_amount, token_id
type UserToStatemintAsset = Vec<(
	AccountId,
	BalanceOf<TestRuntime>,
	<TestRuntime as pallet_assets::Config<StatemintAssetsInstance>>::AssetId,
)>;
// User -> token_amount, price_per_token, multiplier, asset
type UserToBid = Vec<(
	AccountId,
	(
		BalanceOf<TestRuntime>,
		BalanceOf<TestRuntime>,
		Option<MultiplierOf<TestRuntime>>,
		AcceptedFundingAsset
	),
)>;

// User -> ct_amount, multiplier, funding_asset
type UserToContribution = Vec<(AccountId, (BalanceOf<TestRuntime>, Option<MultiplierOf<TestRuntime>>, AcceptedFundingAsset))>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<BidId, ProjectId, Balance: BalanceT, AccountId, BlockNumber, PlmcVesting, CTVesting> {
	pub bid_id: Option<BidId>,
	pub project: Option<ProjectId>,
	pub amount: Option<Balance>,
	pub price: Option<Balance>,
	pub ticket_size: Option<Balance>,
	pub when: Option<BlockNumber>,
	pub bidder: Option<AccountId>,
	pub funded: Option<bool>,
	pub plmc_vesting_period: Option<PlmcVesting>,
	pub ct_vesting_period: Option<CTVesting>,
	pub status: Option<BidStatus<Balance>>,
}
type BidInfoFilterOf<T> = BidInfoFilter<
	<T as Config>::StorageItemId,
	<T as Config>::ProjectIdentifier,
	BalanceOf<T>,
	<T as frame_system::Config>::AccountId,
	BlockNumberOf<T>,
	VestingOf<T>,
	VestingOf<T>,
>;
impl Default for BidInfoFilterOf<TestRuntime> {
	fn default() -> Self {
		BidInfoFilter {
			bid_id: None,
			project: None,
			amount: None,
			price: None,
			ticket_size: None,
			when: None,
			bidder: None,
			funded: None,
			plmc_vesting_period: None,
			ct_vesting_period: None,
			status: None,
		}
	}
}
impl BidInfoFilterOf<TestRuntime> {
	fn matches_bid(&self, bid: &BidInfoOf<TestRuntime>) -> bool {
		if self.bid_id.is_some() && self.bid_id.unwrap() != bid.bid_id {
			return false;
		}

		if self.project.is_some() && self.project.unwrap() != bid.project {
			return false;
		}

		if self.amount.is_some() && self.amount.unwrap() != bid.amount {
			return false;
		}

		if self.price.is_some() && self.price.unwrap() != bid.price {
			return false;
		}

		if self.ticket_size.is_some() && self.ticket_size.unwrap() != bid.ticket_size {
			return false;
		}

		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false;
		}

		if self.bidder.is_some() && self.bidder.unwrap() != bid.bidder {
			return false;
		}

		if self.funded.is_some() && self.funded.unwrap() != bid.funded {
			return false;
		}

		if self.plmc_vesting_period.is_some() && self.plmc_vesting_period.as_ref().unwrap() != &bid.plmc_vesting_period
		{
			return false;
		}

		if self.ct_vesting_period.is_some() && self.ct_vesting_period.as_ref().unwrap() != &bid.ct_vesting_period {
			return false;
		}

		if self.status.is_some() && self.status.as_ref().unwrap() != &bid.status {
			return false;
		}

		return true;
	}
}



const ISSUER: AccountId = 1;
const EVALUATOR_1: AccountId = 2;
const EVALUATOR_2: AccountId = 3;
const EVALUATOR_3: AccountId = 4;
const BIDDER_1: AccountId = 5;
const BIDDER_2: AccountId = 6;
const BUYER_1: AccountId = 7;
const BUYER_2: AccountId = 8;

const ASSET_DECIMALS: u8 = 12;

const USDT_STATEMINT_ID: AssetId = 1984u32;
const USDT_UNIT: u128 = 10_000_000_000_u128;

pub const US_DOLLAR: u128 = 1_0_000_000_000;
pub const US_CENT: u128= 0_1_000_000_000;

const METADATA: &str = r#"
{
    "whitepaper":"ipfs_url",
    "team_description":"ipfs_url",
    "tokenomics":"ipfs_url",
    "roadmap":"ipfs_url",
    "usage_of_founds":"ipfs_url"
}"#;
const ISSUING_FEE: u128 = 0;

// REMARK: Uncomment if we want to test the events.
// fn last_event() -> RuntimeEvent {
// 	frame_system::Pallet::<TestRuntime>::events()
// 		.pop()
// 		.expect("Event expected")
// 		.event
// }

/// Remove accounts from fundings_1 that are not in fundings_2
fn remove_missing_accounts_from_fundings(fundings_1: UserToBalance, fundings_2: UserToBalance) -> UserToBalance {
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
	fn get_project_metadata(&self) -> ProjectMetadataOf<TestRuntime> {
		self.get_test_environment().ext_env.borrow_mut().execute_with(|| {
			FundingModule::projects_metadata(self.get_project_id()).expect("Project info should exist")
		})
	}
	fn get_project_details(&self) -> ProjectDetailsOf<TestRuntime> {
		self.get_test_environment()
			.ext_env
			.borrow_mut()
			.execute_with(|| FundingModule::project_details(self.get_project_id()).expect("Project info should exist"))
	}
	fn do_project_assertions(&self, project_assertions: impl Fn(ProjectIdOf<TestRuntime>, &TestEnvironment) -> ()) {
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
		Self {
			ext_env: RefCell::new(new_test_ext()),
			nonce: RefCell::new(0u64),
		}
	}
	fn create_project(
		&self, creator: mock::AccountId, project: ProjectMetadataOf<TestRuntime>,
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
					RuntimeEvent::FundingModule(crate::Event::Created { project_id }) => Some(project_id),
					_ => None,
				})
				.last()
				.expect("Project created event expected")
				.clone()
		});

		Ok(CreatedProject {
			test_env: self,
			creator,
			project_id,
		})
	}
	/// Returns the *free* fundings of the Users.
	fn get_free_plmc_balances(&self) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut balances = UserToBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::free_balance(&user);
				balances.push((user, funding));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}

	fn get_free_statemint_asset_balances(&self, asset_id: AssetId) -> UserToStatemintAsset {
		self.ext_env.borrow_mut().execute_with(|| {
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			let mut balances: UserToStatemintAsset = UserToStatemintAsset::new();
			for user in user_keys {
				let asset_balance = StatemintAssets::balance(asset_id, &user);
				balances.push((user, asset_balance, asset_id.clone()));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}
	/// Returns the *reserved* fundings of the Users.
	#[allow(dead_code)]
	fn get_reserved_fundings(&self, reserve_type: BondType) -> UserToBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::balance_on_hold(&reserve_type, &user);
				fundings.push((user, funding));
			}
			fundings
		})
	}
	fn mint_plmc_to(&self, mapping: UserToBalance) {
		self.ext_env.borrow_mut().execute_with(|| {
			for (account, amount) in mapping {
				Balances::mint_into(&account, amount).expect("Minting should work");
			}
		});
	}
	fn mint_statemint_asset_to(&self, mapping: UserToStatemintAsset) {
		self.ext_env.borrow_mut().execute_with(|| {
			for (account, amount, id) in mapping {
				StatemintAssets::mint_into(id, &account, amount).expect("Minting should work");
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
				<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::MAX);
				System::set_block_number(System::block_number() + 1);
				<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
			}
		});
	}
	fn do_free_plmc_assertions(&self, correct_funds: UserToBalance) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let free = Balances::free_balance(user);
				assert_eq!(free, balance);
			});
		}
	}
	fn do_reserved_plmc_assertions(&self, correct_funds: UserToBalance, reserve_type: BondType) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let reserved = Balances::balance_on_hold(&reserve_type, &user);
				assert_eq!(reserved, balance);
			});
		}
	}
	fn do_free_statemint_asset_assertions(&self, correct_funds: UserToStatemintAsset) {
		for (user, expected_amount, token_id) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let real_amount = <TestRuntime as Config>::FundingCurrency::balance(token_id, &user);
				assert_eq!(
					expected_amount, real_amount,
					"Wrong statemint asset balance expected for user {}",
					user
				);
			});
		}
	}
	fn do_bid_transferred_statemint_asset_assertions(
		&self, correct_funds: UserToStatemintAsset, project_id: ProjectIdOf<TestRuntime>,
	) {
		for (user, expected_amount, _token_id) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let contribution_total: <TestRuntime as Config>::Balance =
					AuctionsInfo::<TestRuntime>::get(project_id, user.clone())
						.unwrap()
						.iter()
						.map(|c| c.amount * c.price)
						.sum();
				assert_eq!(
					contribution_total, expected_amount,
					"Wrong statemint asset balance expected for user {}",
					user
				);
			});
		}
	}
	fn do_contribution_transferred_statemint_asset_assertions(
		&self, correct_funds: UserToStatemintAsset, project_id: ProjectIdOf<TestRuntime>,
	) {
		for (user, expected_amount, _token_id) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let contribution_total: <TestRuntime as Config>::Balance =
					Contributions::<TestRuntime>::get(project_id, user.clone())
						.unwrap()
						.iter()
						.map(|c| c.contribution_amount)
						.sum();
				assert_eq!(
					contribution_total, expected_amount,
					"Wrong statemint asset balance expected for user {}",
					user
				);
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
		test_env.mint_plmc_to(default_plmc_balances());
		test_env.mint_statemint_asset_to(default_statemint_assets());
		let creator = default_plmc_balances()[0].0;
		let project = test_env
			.create_project(creator, default_project(test_env.nonce.borrow().clone()))
			.unwrap();
		project.do_project_assertions(default_creation_assertions);
		*test_env.nonce.borrow_mut() += 1;
		project
	}

	// Move to next project phase
	fn start_evaluation(self, caller: mock::AccountId) -> Result<EvaluatingProject<'a>, DispatchError> {
		self.test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| FundingModule::start_evaluation(RuntimeOrigin::signed(caller), self.project_id))?;

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
			self.test_env
				.ext_env
				.borrow_mut()
				.execute_with(|| FundingModule::bond_evaluation(RuntimeOrigin::signed(account), project_id, amount))?;
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
		test_env.do_reserved_plmc_assertions(default_evaluation_bonds(), BondType::Evaluation);

		// Check that free funds were reduced
		let mut free_funds = default_plmc_balances();
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
		test_env.do_free_plmc_assertions(free_funds.clone());
		let evaluation_end = evaluating_project
			.get_project_details()
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
		for (account, (token_amount, price_per_token, multiplier, asset)) in bids {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::bid(
					RuntimeOrigin::signed(account),
					project_id,
					token_amount,
					price_per_token,
					multiplier,
					asset
				)
			})?;
		}
		Ok(())
	}

	fn start_community_funding(self) -> CommunityFundingProject<'a> {
		let english_end = self
			.get_project_details()
			.phase_transition_points
			.english_auction
			.end()
			.expect("English end point should exist");
		self.test_env
			.advance_time(english_end - self.test_env.current_block() + 1);
		let candle_end = self
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");
		self.test_env
			.advance_time(candle_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_details().project_status, ProjectStatus::CommunityRound);
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

		let plmc_balances = test_env.get_free_plmc_balances();
		let statemint_asset_balances = test_env.get_free_statemint_asset_balances(USDT_STATEMINT_ID);
		let actual_previous_balances = (plmc_balances, statemint_asset_balances);

		// Do Auction bidding
		auctioning_project
			.bid_for_users(default_auction_bids())
			.expect("Bidding should work");

		// Check our auction was properly interpreted
		test_env.advance_time(1);

		let expected_updated_balances = (
			default_post_auction_plmc_balances(),
			default_post_auction_statemint_asset_balances(),
		);

		auctioning_project.do_project_assertions(|pid, env| {
			bid_assertions(
				pid,
				env,
				actual_previous_balances.clone(),
				expected_updated_balances.clone(),
			)
		});

		let community_funding_project = auctioning_project.start_community_funding();

		let total_ct_sold = default_auction_bids()
			.iter()
			.map(|(_, (ct_amount, _, _, _))| ct_amount)
			.sum::<u128>();
		let bid_expectations = default_auction_bids()
			.iter()
			.map(|(_account, (amount, price, _multiplier, _))| BidInfoFilter {
				amount: Some(*amount),
				price: Some(*price),
				..Default::default()
			})
			.collect::<Vec<_>>();

		community_funding_project.do_project_assertions(|pid, env| {
			community_funding_start_assertions(pid, env, bid_expectations.clone(), total_ct_sold)
		});

		community_funding_project
	}

	fn buy_for_retail_users(&self, buys: UserToContribution) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, (ct_amount, multiplier, asset)) in buys {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::contribute(RuntimeOrigin::signed(account), project_id, ct_amount, multiplier, asset)
			})?;
		}
		Ok(())
	}

	fn start_remainder_funding(self) -> RemainderFundingProject<'a> {
		let community_funding_end = self
			.get_project_details()
			.phase_transition_points
			.community
			.end()
			.expect("Community funding end point should exist");
		self.test_env
			.advance_time(community_funding_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_details().project_status, ProjectStatus::RemainderRound);
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
	fn buy_for_any_user(&self, buys: UserToContribution) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for (account, (ct_amount, multiplier, asset)) in buys {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::contribute(RuntimeOrigin::signed(account), project_id, ct_amount, multiplier, asset)
			})?;
		}
		Ok(())
	}

	fn new_default(test_env: &'a TestEnvironment) -> Self {
		let community_funding_project = CommunityFundingProject::new_default(test_env);
		let project_details = community_funding_project.get_project_details();
		let plmc_balances = test_env.get_free_plmc_balances();
		let statemint_asset_balances = test_env.get_free_statemint_asset_balances(USDT_STATEMINT_ID);
		let actual_previous_balances = (plmc_balances, statemint_asset_balances);
		let cts_bought = default_community_buys()
			.iter()
			.map(|(_, (ct_amount, _, _))| ct_amount)
			.sum::<u128>();
		let expected_remaining_cts = project_details.remaining_contribution_tokens - cts_bought;
		// Do community buying
		community_funding_project
			.buy_for_retail_users(default_community_buys())
			.expect("Community buying should work");

		// Check our buys were properly interpreted
		test_env.advance_time(1);

		let expected_updated_balances = (
			default_post_contribution_plmc_balances(),
			default_post_contribution_statemint_asset_balances(),
		);

		community_funding_project.do_project_assertions(|project_id, test_env| {
			buy_assertions(
				project_id,
				test_env,
				actual_previous_balances.clone(),
				expected_updated_balances.clone(),
				expected_remaining_cts,
			)
		});

		// Start remainder funding by moving block to after the end of community round
		let remainder_funding_project = community_funding_project.start_remainder_funding();

		// Check the community funding round started correctly
		remainder_funding_project.do_project_assertions(default_remainder_funding_start_assertions);

		remainder_funding_project
	}

	fn finish_project(self) -> FinishedProject<'a> {
		let remainder_funding_end = self
			.get_project_details()
			.phase_transition_points
			.remainder
			.end()
			.expect("Remainder funding end point should exist");
		self.test_env
			.advance_time(remainder_funding_end - self.test_env.current_block() + 1);
		assert_eq!(self.get_project_details().project_status, ProjectStatus::FundingEnded);
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
		remainder_funding_project
			.buy_for_any_user(default_remainder_buys())
			.expect("Buying should work");

		// End project funding by moving block to after the end of remainder round
		let finished_project = remainder_funding_project.finish_project();

		// Check the community funding round started correctly
		finished_project.do_project_assertions(default_project_end_assertions);

		finished_project
	}
}

mod defaults {
	use super::*;
	use crate::traits::BondingRequirementCalculation;

	pub fn default_project(nonce: u64) -> ProjectMetadata<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		ProjectMetadata {
			total_allocation_size: 1_000_000,
			minimum_price: 1 * PLMC,
			ticket_size: TicketSize {
				minimum: Some(1),
				maximum: None,
			},
			participants_size: ParticipantsSize {
				minimum: Some(2),
				maximum: None,
			},
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: AcceptedFundingAsset::USDT,
			offchain_information_hash: Some(metadata_hash),
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
		}
	}

	pub fn default_plmc_balances() -> UserToBalance {
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
	}

	pub fn default_statemint_assets() -> UserToStatemintAsset {
		vec![
			(ISSUER, 20_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(EVALUATOR_1, 35_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(EVALUATOR_2, 60_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(EVALUATOR_3, 100_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(BIDDER_1, 500_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(BIDDER_2, 300_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(BUYER_1, 30_000 * USDT_UNIT, USDT_STATEMINT_ID),
			(BUYER_2, 30_000 * USDT_UNIT, USDT_STATEMINT_ID),
		]
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
		// This should reflect the bidding currency, which currently is USDT
		vec![
			(BIDDER_1, (300, 500 * USDT_UNIT, None, AcceptedFundingAsset::USDT)),
			(BIDDER_2, (500, 150 * USDT_UNIT, None, AcceptedFundingAsset::USDT)),
		]
	}

	pub fn default_post_auction_plmc_balances() -> UserToBalance {
		let mut balances = default_plmc_balances();
		// ISSUER
		balances[0].1 -= ISSUING_FEE;
		// EVALUATORS
		balances[1].1 -= default_evaluation_bonds()[0].1;
		balances[2].1 -= default_evaluation_bonds()[1].1;
		balances[3].1 -= default_evaluation_bonds()[2].1;
		// BIDDERS
		balances[4].1 -= default_auction_bids()[0]
			.1
			 .2
			.unwrap_or_default()
			.calculate_bonding_requirement(default_auction_bids()[0].1 .0 * default_auction_bids()[0].1 .1)
			.unwrap();
		balances[5].1 -= default_auction_bids()[1]
			.1
			 .2
			.unwrap_or_default()
			.calculate_bonding_requirement(default_auction_bids()[1].1 .0 * default_auction_bids()[1].1 .1)
			.unwrap();
		// BUYERS
		// unchanged

		balances
	}

	pub fn default_post_auction_statemint_asset_balances() -> UserToStatemintAsset {
		let mut balances = default_statemint_assets();
		// ISSUER
		// unchanged
		// EVALUATORS
		// unchanged
		// BIDDERS
		balances[4].1 -= default_auction_bids()[0].1 .0 * default_auction_bids()[0].1 .1;
		balances[5].1 -= default_auction_bids()[1].1 .0 * default_auction_bids()[1].1 .1;
		// BUYERS
		// unchanged

		balances
	}

	pub fn default_token_average_price() -> BalanceOf<TestRuntime> {
		383_3_333_329_500
	}

	pub fn default_post_contribution_plmc_balances() -> UserToBalance {
		let mut balances = default_post_auction_plmc_balances();
		// ISSUER
		// unchanged
		// EVALUATORS
		// unchanged
		// BIDDERS
		// unchanged
		// BUYERS
		balances[6].1 -= default_community_buys()[0]
			.1
			 .1
			.unwrap_or_default()
			.calculate_bonding_requirement(default_community_buys()[0].1 .0 * default_token_average_price())
			.unwrap();
		balances[7].1 -= default_community_buys()[1]
			.1
			 .1
			.unwrap_or_default()
			.calculate_bonding_requirement(default_community_buys()[1].1 .0 * default_token_average_price())
			.unwrap();

		balances
	}

	pub fn default_post_contribution_statemint_asset_balances() -> UserToStatemintAsset {
		let mut balances = default_post_auction_statemint_asset_balances();
		// ISSUER
		// unchanged
		// EVALUATORS
		// unchanged
		// BIDDERS
		// unchanged
		// BUYERS
		balances[6].1 -= default_community_buys()[0].1 .0 * default_token_average_price();
		balances[7].1 -= default_community_buys()[1].1 .0 * default_token_average_price();

		balances
	}

	pub fn default_community_buys() -> UserToContribution {
		vec![(BUYER_1, (10, None, AcceptedFundingAsset::USDT)), (BUYER_2, (20, None, AcceptedFundingAsset::USDT))]
	}

	pub fn default_remainder_buys() -> UserToContribution {
		vec![(EVALUATOR_2, (6, None, AcceptedFundingAsset::USDT)), (BIDDER_1, (4, None, AcceptedFundingAsset::USDT))]
	}

	// pub fn default_community_funding_plmc_bondings() -> UserToBalance {
	// 	// for now multiplier is always 1, and since plmc and bidding currency are the same,
	// 	// we can just use the same values
	// 	vec![(BUYER_1, (100 * PLMC)), (BUYER_2, (6000 * PLMC))]
	// }

	pub fn default_creation_assertions(project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(project_details.project_status, ProjectStatus::Application);
		});
	}

	pub fn default_evaluation_start_assertions(project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(project_details.project_status, ProjectStatus::EvaluationRound);
		});
	}

	pub fn default_evaluation_end_assertions(project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(project_details.project_status, ProjectStatus::AuctionInitializePeriod);
		});
	}

	pub fn default_auction_start_assertions(project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(
				project_details.project_status,
				ProjectStatus::AuctionRound(AuctionPhase::English)
			);
		});
	}

	pub fn community_funding_start_assertions(
		project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment,
		bid_expectations: Vec<BidInfoFilterOf<TestRuntime>>, expected_ct_sold: BalanceOf<TestRuntime>,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_metadata = FundingModule::projects_metadata(project_id).expect("Project should exist");
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");

			// Round transition happened
			assert_eq!(project_details.project_status, ProjectStatus::CommunityRound);

			// Bids are finalized
			let project_bids = AuctionsInfo::<TestRuntime>::iter_prefix(project_id).collect::<Vec<_>>();
			let project_details = FundingModule::project_details(project_id).unwrap();
			let flattened_bids = project_bids.into_iter().map(|bid| bid.1).flatten().collect::<Vec<_>>();
			assert!(
				matches!(project_details.weighted_average_price, Some(_)),
				"Weighted average price should exist"
			);
			for filter in bid_expectations {
				assert!(flattened_bids.iter().any(|bid| filter.matches_bid(&bid)))
			}

			// Remaining CTs are updated
			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size - expected_ct_sold,
				"Remaining CTs are incorrect"
			);
		});
	}

	pub fn default_remainder_funding_start_assertions(
		project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment,
	) {
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(project_details.project_status, ProjectStatus::RemainderRound);
		});
	}

	pub fn default_project_end_assertions(project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment) {
		// Check that project status is correct
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project info should exist");
			assert_eq!(project_details.project_status, ProjectStatus::FundingEnded);
		});

		// Check that remaining CTs are updated
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_metadata = FundingModule::project_details(project_id).expect("Project should exist");
			let auction_bought_tokens: u128 = default_auction_bids()
				.iter()
				.map(|(_account, (amount, _price, _multiplier, _asset))| amount)
				.sum();
			let community_bought_tokens: u128 = default_community_buys()
				.iter()
				.map(|(_account, (amount, _multiplier, _asset))| amount)
				.sum();
			let remainder_bought_tokens: u128 = default_remainder_buys()
				.iter()
				.map(|(_account, (amount, _multiplier, _asset))| amount)
				.sum();
			assert_eq!(
				project_metadata.remaining_contribution_tokens,
				default_project(0).total_allocation_size
					- auction_bought_tokens
					- community_bought_tokens
					- remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
		});
	}
}

pub mod helper_functions {
	use super::*;

	pub fn get_ed() -> BalanceOf<TestRuntime> {
		<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get()
	}

	pub fn bid_assertions(
		project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment,
		actual_previous_balances: (UserToBalance, UserToStatemintAsset),
		expected_updated_balances: (UserToBalance, UserToStatemintAsset),
	) {
		// Calculate how much PLMC should be bonded
		let expected_plmc_bondings = expected_updated_balances
			.0
			.clone()
			.into_iter()
			.zip(actual_previous_balances.0.clone().into_iter())
			.map(|((post_user, post_plmc), (pre_user, pre_plmc))| {
				assert_eq!(
					post_user, pre_user,
					"Balances iterators should contain the users in the same order"
				);
				(post_user, pre_plmc - post_plmc)
			})
			.collect::<Vec<_>>();

		// Calculate how much each user contributed based on the bids
		let expected_statemint_asset_transfers: UserToStatemintAsset = expected_updated_balances
			.1
			.clone()
			.into_iter()
			.zip(actual_previous_balances.1.clone().into_iter())
			.filter_map(
				|((post_user, post_buys, post_asset_id), (pre_user, pre_buys, pre_asset_id))| {
					assert_eq!(
						post_user, pre_user,
						"Balances iterators should contain the users in the same order"
					);
					assert_eq!(post_asset_id, pre_asset_id, "Same asset ID expected");
					if pre_buys - post_buys > 0 {
						Some((post_user, pre_buys - post_buys, post_asset_id))
					} else {
						None
					}
				},
			)
			.collect();

		// Check that enough PLMC is bonded
		test_env.do_reserved_plmc_assertions(expected_plmc_bondings, BondType::Bidding);

		// Check that the bidding currency is reserved
		test_env.do_bid_transferred_statemint_asset_assertions(expected_statemint_asset_transfers, project_id);

		// Check that PLMC funds were reduced
		test_env.do_free_plmc_assertions(expected_updated_balances.0);

		// Check that statemint asset funds were reduced
		test_env.do_free_statemint_asset_assertions(expected_updated_balances.1);
	}

	pub fn buy_assertions(
		project_id: ProjectIdOf<TestRuntime>, test_env: &TestEnvironment,
		actual_previous_balances: (UserToBalance, UserToStatemintAsset),
		expected_updated_balances: (UserToBalance, UserToStatemintAsset),
		expected_remaining_cts: BalanceOf<TestRuntime>,
	) {
		// Calculate how much PLMC should be bonded
		let expected_plmc_bondings = expected_updated_balances
			.0
			.clone()
			.into_iter()
			.zip(actual_previous_balances.0.clone().into_iter())
			.map(|((post_user, post_plmc), (pre_user, pre_plmc))| {
				assert_eq!(
					post_user, pre_user,
					"Balances iterators should contain the users in the same order"
				);
				(post_user, pre_plmc - post_plmc)
			})
			.collect::<Vec<_>>();

		let expected_buys: UserToStatemintAsset = expected_updated_balances
			.1
			.clone()
			.into_iter()
			.zip(actual_previous_balances.1.clone().into_iter())
			.filter_map(
				|((post_user, post_buys, post_asset_id), (pre_user, pre_buys, pre_asset_id))| {
					assert_eq!(
						post_user, pre_user,
						"Balances iterators should contain the users in the same order"
					);
					assert_eq!(post_asset_id, pre_asset_id, "Same asset ID expected");
					if pre_buys - post_buys > 0 {
						Some((post_user, pre_buys - post_buys, post_asset_id))
					} else {
						None
					}
				},
			)
			.collect();

		// Check that enough PLMC is bonded
		test_env.do_reserved_plmc_assertions(expected_plmc_bondings, BondType::Contributing);
		// Check that the bidding currency is reserved
		test_env.do_contribution_transferred_statemint_asset_assertions(expected_buys.clone(), project_id);
		// Check that PLMC funds were reduced
		test_env.do_free_plmc_assertions(expected_updated_balances.0);
		// Check that statemint asset funds were reduced
		test_env.do_free_statemint_asset_assertions(expected_updated_balances.1);

		// Check that remaining CTs are updated
		test_env.ext_env.borrow_mut().execute_with(|| {
			let project_details = FundingModule::project_details(project_id).expect("Project should exist");
			assert_eq!(
				project_details.remaining_contribution_tokens, expected_remaining_cts,
				"Remaining CTs are incorrect"
			);
		});
	}
}

#[cfg(test)]
mod creation_round_success {
	use super::*;

	#[test]
	fn basic_plmc_transfer_works() {
		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());
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
			let project_metadata = default_project(0);
			assert_noop!(
				FundingModule::create(RuntimeOrigin::signed(BIDDER_1), project_metadata),
				Error::<TestRuntime>::NotAuthorized
			);
		})
	}

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 0,
			ticket_size: TicketSize {
				minimum: Some(1),
				maximum: None,
			},
			participants_size: ParticipantsSize {
				minimum: Some(2),
				maximum: None,
			},
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into(),);
	}

	#[test]
	fn participants_size_error() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 1,
			ticket_size: TicketSize {
				minimum: Some(1),
				maximum: None,
			},
			participants_size: ParticipantsSize {
				minimum: None,
				maximum: None,
			},
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::ParticipantsSizeError.into(),);
	}

	#[test]
	fn ticket_size_error() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 1,
			ticket_size: TicketSize {
				minimum: None,
				maximum: None,
			},
			participants_size: ParticipantsSize {
				minimum: Some(1),
				maximum: None,
			},
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
	}

	#[test]
	#[ignore = "ATM only the first error will be thrown"]
	fn multiple_field_error() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 0,
			ticket_size: TicketSize {
				minimum: None,
				maximum: None,
			},
			participants_size: ParticipantsSize {
				minimum: None,
				maximum: None,
			},
			..Default::default()
		};
		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());
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
		let project_details = evaluating_project.get_project_details();
		test_env.advance_time(project_details.phase_transition_points.evaluation.end().unwrap() + 1u64);
		let end_block = evaluating_project
			.get_project_details()
			.phase_transition_points
			.auction_initialize_period
			.end()
			.unwrap();
		test_env.advance_time(end_block - test_env.current_block() + 1);
		assert_eq!(
			evaluating_project.get_project_details().project_status,
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
		test_env.do_reserved_plmc_assertions(default_failing_evaluation_bonds(), BondType::Evaluation);

		// Check that free funds were reduced
		let mut free_funds = default_plmc_balances();

		// Remove accounts that didnt bond from free_funds
		free_funds = remove_missing_accounts_from_fundings(free_funds, default_failing_evaluation_bonds());
		free_funds = free_funds
			.iter()
			.zip(default_failing_evaluation_bonds().iter())
			.map(|(original, bonded)| {
				assert_eq!(original.0, bonded.0, "User should be the same");
				(original.0, original.1 - bonded.1)
			})
			.collect::<UserToBalance>();
		test_env.do_free_plmc_assertions(free_funds.clone());

		let evaluation_end = evaluating_project
			.get_project_details()
			.phase_transition_points
			.evaluation
			.end()
			.expect("Evaluation end point should exist");
		test_env.advance_time(evaluation_end - test_env.current_block() + 2);
		let project_details = evaluating_project.get_project_details();
		assert_eq!(project_details.project_status, ProjectStatus::EvaluationFailed);
		// Check that on_idle has unlocked the failed bonds
		test_env.advance_time(10);
		test_env.do_free_plmc_assertions(default_plmc_balances());
	}

	#[test]
	fn insufficient_balance_bonding() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_default(&test_env);

		// Try to bond twice as much as the second user of default_fundings has
		let mut user_funding = default_plmc_balances()[1];
		user_funding.1 *= 2;

		let dispatch_error = evaluating_project.bond_for_users(vec![user_funding]).unwrap_err();
		assert_eq!(dispatch_error, Error::<TestRuntime>::InsufficientBalance.into())
	}
}

#[cfg(test)]
mod auction_round_success {
	use super::*;
	use crate::traits::BondingRequirementCalculation;

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
			.get_project_details()
			.phase_transition_points
			.english_auction
			.end()
			.expect("Auction start point should exist");
		// The block following the end of the english auction, is used to transition the project into candle auction.
		// We move past that transition, into the start of the candle auction.
		test_env.advance_time(english_end_block - test_env.current_block() + 1);
		assert_eq!(
			auctioning_project.get_project_details().project_status,
			ProjectStatus::AuctionRound(AuctionPhase::Candle)
		);

		let candle_end_block = auctioning_project
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle auction end point should exist");

		// Do one candle bid for each block until the end of candle auction with a new user
		let mut bidding_account = 1000;
		// Imitate the first default bid
		let bid_info = default_auction_bids()[0].1;
		// Calculate necessary PLMC from the multiplier
		let plmc_necessary_funding = 100 * PLMC
			+ bid_info
				.2
				.unwrap_or_default()
				.calculate_bonding_requirement(bid_info.0 * bid_info.1)
				.unwrap();
		let statemint_asset_necessary_funding = bid_info.0 * bid_info.1;

		let mut bids_made: UserToBid = vec![];
		let starting_bid_block = test_env.current_block();
		let blocks_to_bid = test_env.current_block()..candle_end_block;

		for _block in blocks_to_bid {
			// Check we are still in candle auction
			assert_eq!(
				auctioning_project.get_project_details().project_status,
				ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			// Fund account with necessary assets
			test_env.mint_plmc_to(vec![(bidding_account, plmc_necessary_funding)]);
			test_env.mint_statemint_asset_to(vec![(
				bidding_account,
				statemint_asset_necessary_funding,
				USDT_STATEMINT_ID,
			)]);
			// Make the bid
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
			.get_project_details()
			.phase_transition_points
			.random_candle_ending
			.expect("Random auction end point should exist");

		let split = (random_end - starting_bid_block + 1) as usize;
		let excluded_bids = bids_made.split_off(split);
		let included_bids = bids_made;
		let _weighted_price = auctioning_project
			.get_project_details()
			.weighted_average_price
			.expect("Weighted price should exist");
		// let all_bids = test_env.ext_env.borrow_mut().execute_with(|| {
		// 	AuctionsInfo::<TestRuntime>::iter_prefix_values(auctioning_project.project_id)
		// 		.map(|bids| bids[0].clone())
		// 		.collect::<Vec<_>>()
		// });
		for (bidder, (_amount, _price, _multiplier, _asset)) in included_bids {
			test_env.ext_env.borrow_mut().execute_with(|| {
				let pid = auctioning_project.project_id;
				let stored_bid = FundingModule::auctions_info(pid, bidder).expect("Bid should exist");
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

		for (bidder, (_amount, _price, _multiplier, _asset)) in excluded_bids {
			test_env.ext_env.borrow_mut().execute_with(|| {
				let pid = auctioning_project.project_id;
				let stored_bid = FundingModule::auctions_info(pid, bidder).expect("Bid should exist");
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
				FundingModule::bid(RuntimeOrigin::signed(BIDDER_2), 0, 1, 100, None, AcceptedFundingAsset::USDT),
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
				FundingModule::contribute(RuntimeOrigin::signed(BIDDER_1), project_id, 100, None, AcceptedFundingAsset::USDT),
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
			(DAVE, (10_000, 2 * PLMC, None, AcceptedFundingAsset::USDT)),
			(DAVE, (13_000, 3 * PLMC, None, AcceptedFundingAsset::USDT)),
			(DAVE, (15_000, 5 * PLMC, None, AcceptedFundingAsset::USDT)),
			(DAVE, (1_000, 7 * PLMC, None, AcceptedFundingAsset::USDT)),
			(DAVE, (20_000, 8 * PLMC, None, AcceptedFundingAsset::USDT)),
		];

		let mut plmc_fundings: UserToBalance = bids
			.iter()
			.map(|(user, (amount, price, _, _))| (*user, *amount * *price))
			.collect::<Vec<_>>();
		// Existential deposit on DAVE
		plmc_fundings.push((DAVE, 100 * PLMC));

		let statemint_asset_fundings: UserToStatemintAsset = plmc_fundings
			.clone()
			.into_iter()
			.map(|(user, amount)| (user, amount, USDT_STATEMINT_ID))
			.collect();

		// Fund enough for all PLMC bonds for the bids (multiplier of 1)
		test_env.mint_plmc_to(plmc_fundings.clone());

		// Fund enough for all bids
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

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

	#[test]
	fn bid_with_asset_not_accepted() {
		let test_env = TestEnvironment::new();
		let auctioning_project = AuctioningProject::new_default(&test_env);
		let mul_2 = MultiplierOf::<TestRuntime>::from(2u32);
		let bids = vec![
			(BIDDER_1, (10_000, 2 * PLMC, None, AcceptedFundingAsset::USDC)),
			(BIDDER_2, (13_000, 3 * PLMC, Some(mul_2), AcceptedFundingAsset::USDC)),
		];
		let outcome = auctioning_project.bid_for_users(bids);
		frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
	}
}

#[cfg(test)]
mod community_round_success {
	use super::*;
	use crate::traits::BondingRequirementCalculation;
	pub const HOURS: BlockNumber = 300u64;

	#[test]
	fn community_round_works() {
		let test_env = TestEnvironment::new();
		let _community_funding_project = CommunityFundingProject::new_default(&test_env);
	}

	#[test]
	fn contribute_multiple_times_works() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_default(&test_env);
		const BOB: AccountId = 42;
		let token_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.unwrap();
		let plmc_funding: UserToBalance = vec![(BOB, 3 * token_price), (BOB, 4 * token_price), (BOB, get_ed() * 10)];
		let statemint_funding: UserToStatemintAsset = plmc_funding
			.clone()
			.into_iter()
			.map(|(user, amount)| (user, amount, USDT_STATEMINT_ID))
			.collect();

		// Fund for PLMC bond
		test_env.mint_plmc_to(plmc_funding);
		// Fund for buy
		test_env.mint_statemint_asset_to(statemint_funding);

		// TODO: Set a reasonable amount of Contribution Tokens that the user wants to buy
		community_funding_project
			.buy_for_retail_users(vec![(BOB, (3, None, AcceptedFundingAsset::USDT))])
			.expect("The Buyer should be able to buy multiple times");
		test_env.advance_time((1 * HOURS) as BlockNumber);

		community_funding_project
			.buy_for_retail_users(vec![(BOB, (4, None, AcceptedFundingAsset::USDT))])
			.expect("The Buyer should be able to buy multiple times");

		test_env.ext_env.borrow_mut().execute_with(|| {
			let bob_contributions: BalanceOf<TestRuntime> =
				Contributions::<TestRuntime>::get(community_funding_project.project_id, BOB)
					.unwrap()
					.iter()
					.map(|c| c.contribution_amount)
					.sum();
			assert_eq!(bob_contributions, 7 * token_price);
		});
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_exact() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_default(&test_env);
		const BOB: AccountId = 808;

		let remaining_ct = community_funding_project
			.get_project_details()
			.remaining_contribution_tokens;
		let ct_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.expect("CT Price should exist");

		let plmc_fundings: UserToBalance = vec![(BOB, remaining_ct * ct_price), (BOB, get_ed())];
		let statemint_asset_fundings: UserToStatemintAsset = plmc_fundings
			.clone()
			.into_iter()
			.map(|(user, amount)| (user, amount, USDT_STATEMINT_ID))
			.collect();

		// Fund for buy and PLMC bond
		test_env.mint_plmc_to(plmc_fundings.clone());
		// Fund for PLMC bond
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		community_funding_project
			.buy_for_retail_users(vec![(BOB, (remaining_ct, None, AcceptedFundingAsset::USDT))])
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64);
		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project
				.get_project_details()
				.remaining_contribution_tokens,
			0
		);

		// Check project is in FundingEnded state
		assert_eq!(
			community_funding_project.get_project_details().project_status,
			ProjectStatus::FundingEnded
		);

		test_env.do_free_plmc_assertions(vec![plmc_fundings[1]]);
		test_env.do_free_statemint_asset_assertions(vec![statemint_asset_fundings[1]]);
		test_env.do_reserved_plmc_assertions(vec![plmc_fundings[0]], BondType::Contributing);
		test_env.do_contribution_transferred_statemint_asset_assertions(
			vec![statemint_asset_fundings[0]],
			community_funding_project.get_project_id(),
		);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_overbuy() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_default(&test_env);
		const BOB: AccountId = 808;

		let remaining_ct = community_funding_project
			.get_project_details()
			.remaining_contribution_tokens
			+ 40; // Overbuy
		let ct_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.expect("CT Price should exist");

		// Necessary funds to buy remaining CTs, plus some extra for keeping it account alive
		let plmc_fundings: UserToBalance = vec![(BOB, remaining_ct * ct_price), (BOB, get_ed())];
		let statemint_asset_fundings: UserToStatemintAsset = plmc_fundings
			.clone()
			.into_iter()
			.map(|(user, amount)| (user, amount, USDT_STATEMINT_ID))
			.collect();

		// Fund for buy and PLMC bond
		test_env.mint_plmc_to(plmc_fundings.clone());
		// Fund for PLMC bond
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		community_funding_project
			.buy_for_retail_users(vec![(BOB, (remaining_ct, None, AcceptedFundingAsset::USDT))])
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64);

		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project
				.get_project_details()
				.remaining_contribution_tokens,
			0
		);

		// Check project is in FundingEnded state
		assert_eq!(
			community_funding_project.get_project_details().project_status,
			ProjectStatus::FundingEnded
		);

		test_env.do_free_plmc_assertions(vec![(BOB, (40 * ct_price) + get_ed())]);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_same_mult_diff_ct() {
		let test_env = TestEnvironment::new();
		let project = CommunityFundingProject::new_default(&test_env);
		let buyer_2_initial_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_initial_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));
		let project_details = project.get_project_details();

		// Create a contribution vector that will reach the limit of contributions for a user-project
		let multiplier: Option<MultiplierOf<TestRuntime>> = None;
		let token_amount: BalanceOf<TestRuntime> = 1;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: UserToContribution = range.map(|_| (BUYER_2, (token_amount, multiplier, AcceptedFundingAsset::USDT))).collect();

		// Calculate currencies being transferred and bonded
		let contribution_ticket_size = token_amount * project_details.weighted_average_price.unwrap();
		let plmc_bond = multiplier
			.unwrap_or_default()
			.calculate_bonding_requirement(contribution_ticket_size)
			.unwrap();

		// Reach the limit of contributions for a user-project
		project.buy_for_retail_users(contributions.clone()).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let buyer_2_post_buy_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_post_buy_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));

		assert_eq!(
			buyer_2_post_buy_plmc_balance,
			buyer_2_initial_plmc_balance - plmc_bond * contributions.len() as u128
		);
		assert_eq!(
			buyer_2_post_buy_statemint_asset_balance,
			buyer_2_initial_statemint_asset_balance - contribution_ticket_size * contributions.len() as u128
		);

		let plmc_bond_stored = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| ContributingBonds::<TestRuntime>::get(project.project_id, BUYER_2.clone()).unwrap());
		let statemint_asset_contributions_stored = test_env.ext_env.borrow_mut().execute_with(|| {
			Contributions::<TestRuntime>::get(project.project_id, BUYER_2)
				.unwrap()
				.iter()
				.map(|c| c.contribution_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored.amount, plmc_bond * contributions.len() as u128);
		assert_eq!(
			statemint_asset_contributions_stored,
			contribution_ticket_size * contributions.len() as u128
		);

		// Make a new contribution with a PLMC bond bigger than the lowest bond already in store for that account
		let new_multiplier: Option<MultiplierOf<TestRuntime>> = None;
		let new_token_amount: BalanceOf<TestRuntime> = 2;
		let new_contribution: UserToContribution = vec![(BUYER_2, (new_token_amount, new_multiplier, AcceptedFundingAsset::USDT))];
		let new_ticket_size = new_token_amount * project_details.weighted_average_price.unwrap();
		let new_plmc_bond = new_multiplier
			.unwrap_or_default()
			.calculate_bonding_requirement(new_ticket_size)
			.unwrap();

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		// Check that the previous contribution returned the reserved PLMC and the transferred funding currency
		let buyer_2_post_return_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_post_return_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));

		assert_eq!(
			buyer_2_post_return_plmc_balance,
			buyer_2_post_buy_plmc_balance + plmc_bond - new_plmc_bond
		);
		assert_eq!(
			buyer_2_post_return_statemint_asset_balance,
			buyer_2_post_buy_statemint_asset_balance + contribution_ticket_size - new_ticket_size
		);

		let new_plmc_bond_stored = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| crate::ContributingBonds::<TestRuntime>::get(project.project_id, BUYER_2).unwrap());
		let new_statemint_asset_contributions_stored = test_env.ext_env.borrow_mut().execute_with(|| {
			Contributions::<TestRuntime>::get(project.project_id, BUYER_2)
				.unwrap()
				.iter()
				.map(|c| c.contribution_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored.amount,
			plmc_bond_stored.amount - plmc_bond + new_plmc_bond
		);
		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored - contribution_ticket_size + new_ticket_size
		);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_diff_mult_same_ct() {
		let test_env = TestEnvironment::new();
		let project = CommunityFundingProject::new_default(&test_env);
		let buyer_2_initial_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_initial_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));
		let project_details = project.get_project_details();

		// Create a contribution that will reach the limit of contributions for a user-project
		let multiplier: Option<MultiplierOf<TestRuntime>> = Some(Multiplier(2));
		let token_amount: BalanceOf<TestRuntime> = 1;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: UserToContribution = range.map(|_| (BUYER_2, (token_amount, multiplier, AcceptedFundingAsset::USDT))).collect();

		// Calculate currencies being transferred and bonded
		let contribution_ticket_size = token_amount * project_details.weighted_average_price.unwrap();
		let plmc_bond = multiplier
			.unwrap_or_default()
			.calculate_bonding_requirement(contribution_ticket_size)
			.unwrap();

		// Reach the limit of contributions for a user-project
		project.buy_for_retail_users(contributions.clone()).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let buyer_2_post_buy_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_post_buy_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));

		assert_eq!(
			buyer_2_post_buy_plmc_balance,
			buyer_2_initial_plmc_balance - plmc_bond * contributions.len() as u128
		);
		assert_eq!(
			buyer_2_post_buy_statemint_asset_balance,
			buyer_2_initial_statemint_asset_balance - contribution_ticket_size * contributions.len() as u128
		);

		let plmc_bond_stored = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| ContributingBonds::<TestRuntime>::get(project.project_id, BUYER_2.clone()).unwrap());
		let statemint_asset_contributions_stored = test_env.ext_env.borrow_mut().execute_with(|| {
			Contributions::<TestRuntime>::get(project.project_id, BUYER_2)
				.unwrap()
				.iter()
				.map(|c| c.contribution_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored.amount, plmc_bond * contributions.len() as u128);
		assert_eq!(
			statemint_asset_contributions_stored,
			contribution_ticket_size * contributions.len() as u128
		);

		// Make a new contribution with a PLMC bond bigger than the lowest bond already in store for that account
		let new_multiplier: Option<MultiplierOf<TestRuntime>> = Some(Multiplier(1));
		let new_token_amount: BalanceOf<TestRuntime> = 1;
		let new_contribution: UserToContribution = vec![(BUYER_2, (new_token_amount, new_multiplier, AcceptedFundingAsset::USDT))];
		let new_ticket_size = new_token_amount * project_details.weighted_average_price.unwrap();
		let new_plmc_bond = new_multiplier
			.unwrap_or_default()
			.calculate_bonding_requirement(new_ticket_size)
			.unwrap();

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		// Check that the previous contribution returned the reserved PLMC and the transferred funding currency
		let buyer_2_post_return_plmc_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::NativeCurrency::free_balance(&BUYER_2));
		let buyer_2_post_return_statemint_asset_balance = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &BUYER_2));

		assert_eq!(
			buyer_2_post_return_plmc_balance,
			buyer_2_post_buy_plmc_balance + plmc_bond - new_plmc_bond
		);
		assert_eq!(
			buyer_2_post_return_statemint_asset_balance,
			buyer_2_post_buy_statemint_asset_balance + contribution_ticket_size - new_ticket_size
		);

		let new_plmc_bond_stored = test_env
			.ext_env
			.borrow_mut()
			.execute_with(|| crate::ContributingBonds::<TestRuntime>::get(project.project_id, BUYER_2).unwrap());
		let new_statemint_asset_contributions_stored = test_env.ext_env.borrow_mut().execute_with(|| {
			Contributions::<TestRuntime>::get(project.project_id, BUYER_2)
				.unwrap()
				.iter()
				.map(|c| c.contribution_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored.amount,
			plmc_bond_stored.amount - plmc_bond + new_plmc_bond
		);
		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored - contribution_ticket_size + new_ticket_size
		);
	}
}

#[cfg(test)]
mod community_round_failure {
	// TODO: Maybe here we can test what happens if we sell all the CTs in the community round
}

#[cfg(test)]
mod remainder_round_success {
	use super::*;

	#[test]
	fn remainder_round_works() {
		let test_env = TestEnvironment::new();
		let _remainder_funding_project = RemainderFundingProject::new_default(&test_env);
	}
}

#[cfg(test)]
mod purchased_vesting {
	use super::*;
	use crate::traits::BondingRequirementCalculation;

	#[test]
	fn contribution_token_mints() {
		// TODO: currently the vesting is limited to the whole payment at once. We should test it with several payments over a vesting period.
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let _token_price = finished_project
			.get_project_details()
			.weighted_average_price
			.expect("CT price should exist at this point");
		let project_metadata = finished_project.get_project_metadata();
		let decimals = project_metadata.token_information.decimals;

		test_env.ext_env.borrow_mut().execute_with(|| {
			for (buyer, (amount, _multiplier, _asset)) in default_community_buys() {
				assert_ok!(FundingModule::vested_contribution_token_purchase_mint_for(
					RuntimeOrigin::signed(buyer),
					project_id,
					buyer
				));

				let minted_balance = LocalAssets::balance(project_id, buyer);
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
		let price = finished_project
			.get_project_details()
			.weighted_average_price
			.expect("CT price should exist at this point");
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (buyer, (token_amount, multiplier, _asset)) in default_community_buys() {
				let theoretical_bonded_plmc = multiplier
					.unwrap_or_default()
					.calculate_bonding_requirement(token_amount * price)
					.unwrap();
				let actual_bonded_plmc = Balances::balance_on_hold(&BondType::Contributing, &buyer);
				assert_eq!(theoretical_bonded_plmc, actual_bonded_plmc);
				assert_ok!(FundingModule::vested_plmc_purchase_unbond_for(
					RuntimeOrigin::signed(buyer),
					project_id,
					buyer
				));
				let actual_bonded_plmc = Balances::balance_on_hold(&BondType::Contributing, &buyer);
				assert_eq!(actual_bonded_plmc, 0u32.into());
			}
		});
	}
}

#[cfg(test)]
mod bids_vesting {
	use super::*;
	use crate::traits::BondingRequirementCalculation;

	#[test]
	fn contribution_token_mints() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::new_default(&test_env);
		let project_id = finished_project.project_id;
		let bidders = default_auction_bids();
		let project_metadata = finished_project.get_project_metadata();
		let decimals = project_metadata.token_information.decimals;
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (bidder, (amount, _price, _multiplier, _asset)) in bidders {
				assert_ok!(FundingModule::vested_contribution_token_bid_mint_for(
					RuntimeOrigin::signed(bidder),
					project_id,
					bidder
				));
				let minted_balance = LocalAssets::balance(project_id, bidder);
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
		let project_metadata = finished_project.get_project_metadata();
		let _decimals = project_metadata.token_information.decimals;
		test_env.ext_env.borrow_mut().execute_with(|| {
			for (bidder, (amount, price, multiplier, _asset)) in bidders {
				let theoretical_bonded_plmc = multiplier
					.unwrap_or_default()
					.calculate_bonding_requirement(amount * price)
					.unwrap();
				let actual_bonded_plmc = Balances::balance_on_hold(&BondType::Bidding, &bidder);
				assert_eq!(theoretical_bonded_plmc, actual_bonded_plmc);
				assert_ok!(FundingModule::vested_plmc_bid_unbond_for(
					RuntimeOrigin::signed(bidder),
					project_id,
					bidder
				));
				let actual_bonded_plmc = Balances::balance_on_hold(&BondType::Bidding, &bidder);
				assert_eq!(actual_bonded_plmc, 0u32.into());
			}
		});
	}
}

#[cfg(test)]
mod misc_features {
	use super::*;
	use crate::UpdateType::{CommunityFundingStart, RemainderFundingStart};

	#[test]
	fn remove_from_update_store_works() {
		let test_env = TestEnvironment::new();
		let now = test_env.current_block();
		test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::add_to_update_store(now + 10u64, (&42u32, CommunityFundingStart));
			FundingModule::add_to_update_store(now + 20u64, (&69u32, RemainderFundingStart));
			FundingModule::add_to_update_store(now + 5u64, (&404u32, RemainderFundingStart));
		});
		test_env.advance_time(2u64);
		test_env.ext_env.borrow_mut().execute_with(|| {
			let stored = crate::ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored.len(), 3, "There should be 3 blocks scheduled for updating");

			FundingModule::remove_from_update_store(&69u32).unwrap();

			let stored = crate::ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(
				stored[2],
				vec![],
				"Vector should be empty for that block after deletion"
			);
		});
	}

	#[test]
	fn sandbox() {
		// let plmc_price_in_usd = 8_5_000_000_000u128;
		// let token_amount= FixedU128::from_float(12.5);
		// let ticket_size: u128 = token_amount.checked_mul_int(plmc_price_in_usd).unwrap();
		//
		// let ticket_size = 250_0_000_000_000u128;
		// let rate = FixedU128::from_float(8.5f64);
		// let inv_rate = rate.reciprocal().unwrap();
		// let amount = inv_rate.checked_mul_int(ticket_size).unwrap();
		// let a = FixedU128::from
		// let x = "x";
		// 29_4_117_647_058
	}
}
