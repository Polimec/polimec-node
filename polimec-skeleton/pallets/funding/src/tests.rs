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
use crate as pallet_funding;
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
		OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
};
use helper_functions::*;

use crate::traits::BondingRequirementCalculation;
use sp_runtime::DispatchError;
use std::cell::RefCell;
use std::iter::zip;

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
type UserToPLMCBalance = Vec<(AccountId, BalanceOf<TestRuntime>)>;
type UserToUSDBalance = Vec<(AccountId, BalanceOf<TestRuntime>)>;
type UserToStatemintAsset = Vec<(
	AccountId,
	BalanceOf<TestRuntime>,
	<TestRuntime as pallet_assets::Config<StatemintAssetsInstance>>::AssetId,
)>;

#[derive(Clone, Copy)]
pub struct TestBid {
	bidder: AccountId,
	amount: BalanceOf<TestRuntime>,
	price: PriceOf<TestRuntime>,
	multiplier: Option<MultiplierOf<TestRuntime>>,
	asset: AcceptedFundingAsset,
}
impl TestBid {
	fn new(
		bidder: AccountId, amount: BalanceOf<TestRuntime>, price: PriceOf<TestRuntime>,
		multiplier: Option<MultiplierOf<TestRuntime>>, asset: AcceptedFundingAsset,
	) -> Self {
		Self {
			bidder,
			amount,
			price,
			multiplier,
			asset,
		}
	}
}
pub type TestBids = Vec<TestBid>;

#[derive(Clone, Copy)]
pub struct TestContribution {
	contributor: AccountId,
	amount: BalanceOf<TestRuntime>,
	multiplier: Option<MultiplierOf<TestRuntime>>,
	asset: AcceptedFundingAsset,
}
impl TestContribution {
	fn new(
		contributor: AccountId, amount: BalanceOf<TestRuntime>, multiplier: Option<MultiplierOf<TestRuntime>>,
		asset: AcceptedFundingAsset,
	) -> Self {
		Self {
			contributor,
			amount,
			multiplier,
			asset,
		}
	}
}
pub type TestContributions = Vec<TestContribution>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<BidId, ProjectId, Balance: BalanceT, Price, AccountId, BlockNumber, PlmcVesting, CTVesting> {
	pub bid_id: Option<BidId>,
	pub when: Option<BlockNumber>,
	pub status: Option<BidStatus<Balance>>,
	pub project: Option<ProjectId>,
	pub bidder: Option<AccountId>,
	pub ct_amount: Option<Balance>,
	pub ct_usd_price: Option<Price>,
	pub funded: Option<bool>,
	pub plmc_vesting_period: Option<PlmcVesting>,
	pub ct_vesting_period: Option<CTVesting>,
	pub funding_asset: Option<AcceptedFundingAsset>,
	pub funding_asset_amount: Option<Balance>,
}
type BidInfoFilterOf<T> = BidInfoFilter<
	<T as Config>::StorageItemId,
	<T as Config>::ProjectIdentifier,
	BalanceOf<T>,
	PriceOf<T>,
	<T as frame_system::Config>::AccountId,
	BlockNumberOf<T>,
	VestingOf<T>,
	VestingOf<T>,
>;
impl Default for BidInfoFilterOf<TestRuntime> {
	fn default() -> Self {
		BidInfoFilter {
			bid_id: None,
			when: None,
			status: None,
			project: None,
			bidder: None,
			ct_amount: None,
			ct_usd_price: None,
			funded: None,
			plmc_vesting_period: None,
			ct_vesting_period: None,
			funding_asset: None,
			funding_asset_amount: None,
		}
	}
}
impl BidInfoFilterOf<TestRuntime> {
	fn matches_bid(&self, bid: &BidInfoOf<TestRuntime>) -> bool {
		if self.bid_id.is_some() && self.bid_id.unwrap() != bid.id {
			return false;
		}
		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false;
		}
		if self.status.is_some() && self.status.as_ref().unwrap() != &bid.status {
			return false;
		}
		if self.project.is_some() && self.project.unwrap() != bid.project_id {
			return false;
		}
		if self.bidder.is_some() && self.bidder.unwrap() != bid.bidder {
			return false;
		}
		if self.ct_amount.is_some() && self.ct_amount.unwrap() != bid.ct_amount {
			return false;
		}
		if self.ct_usd_price.is_some() && self.ct_usd_price.unwrap() != bid.ct_usd_price {
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
		if self.funding_asset.is_some() && self.funding_asset.as_ref().unwrap() != &bid.funding_asset {
			return false;
		}
		if self.funding_asset_amount.is_some()
			&& self.funding_asset_amount.as_ref().unwrap() != &bid.funding_asset_amount
		{
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

const ASSET_DECIMALS: u8 = 10;
const ASSET_UNIT: u128 = 10u128.pow(ASSET_DECIMALS as u32);

const USDT_STATEMINT_ID: AssetId = 1984u32;
const USDT_UNIT: u128 = 10_000_000_000_u128;

pub const US_DOLLAR: u128 = 1_0_000_000_000;
pub const US_CENT: u128 = 0_1_000_000_000;

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

trait TestInstance {}
trait ProjectInstance {
	fn get_test_environment(&self) -> &TestEnvironment;
	fn get_issuer(&self) -> AccountId;
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
	fn in_ext<R>(&self, execute: impl FnOnce() -> R) -> R {
		self.get_test_environment().ext_env.borrow_mut().execute_with(execute)
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
	fn get_new_nonce(&self) -> u64 {
		let nonce = self.nonce.borrow_mut().clone();
		self.nonce.replace(nonce + 1);
		nonce
	}
	fn create_project(
		&self, issuer: AccountId, project: ProjectMetadataOf<TestRuntime>,
	) -> Result<CreatedProject, DispatchError> {
		// Create project in the externalities environment of this struct instance
		self.ext_env
			.borrow_mut()
			.execute_with(|| FundingModule::create(RuntimeOrigin::signed(issuer), project))?;

		// Retrieve the project_id from the events
		let project_id = self.ext_env.borrow_mut().execute_with(|| {
			frame_system::Pallet::<TestRuntime>::events()
				.iter()
				.filter_map(|event| match event.event {
					RuntimeEvent::FundingModule(Event::Created { project_id }) => Some(project_id),
					_ => None,
				})
				.last()
				.expect("Project created event expected")
				.clone()
		});

		Ok(CreatedProject {
			test_env: self,
			issuer,
			project_id,
		})
	}
	fn in_ext<R>(&self, execute: impl FnOnce() -> R) -> R {
		self.ext_env.borrow_mut().execute_with(execute)
	}

	#[allow(dead_code)]
	fn get_all_free_plmc_balances(&self) -> UserToPLMCBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut balances = UserToPLMCBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::free_balance(&user);
				balances.push((user, funding));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}
	#[allow(dead_code)]
	fn get_all_free_statemint_asset_balances(&self, asset_id: AssetId) -> UserToStatemintAsset {
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
	fn get_free_plmc_balances_for(&self, user_keys: Vec<AccountId>) -> UserToPLMCBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut balances = UserToPLMCBalance::new();
			for user in user_keys {
				let funding = Balances::free_balance(&user);
				balances.push((user, funding));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}
	fn get_free_statemint_asset_balances_for(
		&self, asset_id: AssetId, user_keys: Vec<AccountId>,
	) -> UserToStatemintAsset {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut balances = UserToStatemintAsset::new();
			for user in user_keys {
				let asset_balance = StatemintAssets::balance(asset_id, &user);
				balances.push((user, asset_balance, asset_id.clone()));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}
	fn get_plmc_total_supply(&self) -> BalanceOf<TestRuntime> {
		self.ext_env
			.borrow_mut()
			.execute_with(|| <TestRuntime as pallet_funding::Config>::NativeCurrency::total_issuance())
	}
	fn do_reserved_plmc_assertions(
		&self, correct_funds: UserToPLMCBalance, reserve_type: BondType<ProjectIdOf<TestRuntime>>,
	) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let reserved = Balances::balance_on_hold(&reserve_type, &user);
				assert_eq!(reserved, balance);
			});
		}
	}
	#[allow(dead_code)]
	fn get_reserved_fundings(&self, reserve_type: BondType<ProjectIdOf<TestRuntime>>) -> UserToPLMCBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut fundings = UserToPLMCBalance::new();
			let user_keys: Vec<AccountId> = frame_system::Account::<TestRuntime>::iter_keys().collect();
			for user in user_keys {
				let funding = Balances::balance_on_hold(&reserve_type, &user);
				fundings.push((user, funding));
			}
			fundings
		})
	}
	fn mint_plmc_to(&self, mapping: UserToPLMCBalance) {
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
	fn do_free_plmc_assertions(&self, correct_funds: UserToPLMCBalance) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let free = Balances::free_balance(user);
				assert_eq!(free, balance);
			});
		}
	}

	fn do_total_plmc_assertions(&self, expected_supply: BalanceOf<TestRuntime>) {
		let real_supply = self.get_plmc_total_supply();
		assert_eq!(real_supply, expected_supply);
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
					Bids::<TestRuntime>::get(project_id, user.clone())
						.iter()
						.map(|c| c.funding_asset_amount)
						.sum();
				assert_eq!(
					contribution_total, expected_amount,
					"Wrong statemint asset balance expected for stored auction info on user {}",
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
						.iter()
						.map(|c| c.funding_asset_amount)
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
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for CreatedProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> CreatedProject<'a> {
	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
	) -> Self {
		let now = test_env.current_block();
		test_env.mint_plmc_to(vec![(issuer, get_ed())]);
		let created_project = test_env.create_project(issuer, project_metadata.clone()).unwrap();
		created_project.creation_assertions(project_metadata, now);
		created_project
	}

	fn creation_assertions(
		&self, expected_metadata: ProjectMetadataOf<TestRuntime>, creation_start_block: BlockNumberOf<TestRuntime>,
	) {
		let metadata = self.get_project_metadata();
		let details = self.get_project_details();
		let expected_details = ProjectDetailsOf::<TestRuntime> {
			issuer: self.get_issuer(),
			is_frozen: false,
			weighted_average_price: None,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair {
					start: Some(creation_start_block),
					end: None,
				},
				..Default::default()
			},
			fundraising_target: expected_metadata
				.minimum_price
				.checked_mul_int(expected_metadata.total_allocation_size)
				.unwrap(),
			remaining_contribution_tokens: expected_metadata.total_allocation_size,
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	// Move to next project phase
	fn start_evaluation(self, caller: AccountId) -> Result<EvaluatingProject<'a>, DispatchError> {
		assert_eq!(self.get_project_details().status, ProjectStatus::Application);
		self.in_ext(|| FundingModule::start_evaluation(RuntimeOrigin::signed(caller), self.project_id))?;
		assert_eq!(self.get_project_details().status, ProjectStatus::EvaluationRound);

		Ok(EvaluatingProject {
			test_env: self.test_env,
			issuer: self.issuer,
			project_id: self.project_id,
		})
	}
}

#[derive(Debug, Clone)]
struct EvaluatingProject<'a> {
	test_env: &'a TestEnvironment,
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for EvaluatingProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> EvaluatingProject<'a> {
	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
	) -> Self {
		let created_project = CreatedProject::new_with(test_env, project_metadata.clone(), issuer);
		let creator = created_project.get_issuer();

		let evaluating_project = created_project.start_evaluation(creator).unwrap();

		evaluating_project
	}

	pub fn evaluation_assertions(
		&self, expected_free_plmc_balances: UserToPLMCBalance, expected_reserved_plmc_balances: UserToPLMCBalance,
		total_plmc_supply: BalanceOf<TestRuntime>,
	) {
		let project_details = self.get_project_details();
		let test_env = self.test_env;
		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_reserved_plmc_assertions(
			expected_reserved_plmc_balances,
			BondType::Evaluation(self.get_project_id()),
		);
		test_env.do_total_plmc_assertions(total_plmc_supply);
	}

	fn bond_for_users(&self, bonds: UserToUSDBalance) -> Result<(), DispatchError> {
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
		let project_details = self.get_project_details();

		if project_details.status == ProjectStatus::EvaluationRound {
			let evaluation_end = project_details.phase_transition_points.evaluation.end().unwrap();
			let auction_start = evaluation_end.saturating_add(2);
			let blocks_to_start = auction_start.saturating_sub(self.test_env.current_block());
			self.test_env.advance_time(blocks_to_start);
		};

		assert_eq!(
			self.get_project_details().status,
			ProjectStatus::AuctionInitializePeriod
		);

		self.in_ext(|| FundingModule::start_auction(RuntimeOrigin::signed(caller), self.get_project_id()))?;

		assert_eq!(
			self.get_project_details().status,
			ProjectStatus::AuctionRound(AuctionPhase::English)
		);

		Ok(AuctioningProject {
			test_env: self.test_env,
			issuer: self.issuer,
			project_id: self.project_id,
		})
	}
}

#[derive(Debug)]
struct AuctioningProject<'a> {
	test_env: &'a TestEnvironment,
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for AuctioningProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> AuctioningProject<'a> {
	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId, evaluations: UserToUSDBalance,
	) -> Self {
		let evaluating_project = EvaluatingProject::new_with(test_env, project_metadata, issuer);

		let evaluators = evaluations
			.iter()
			.map(|(acc, _val)| acc.clone())
			.collect::<Vec<AccountIdOf<TestRuntime>>>();
		let prev_supply = test_env.get_plmc_total_supply();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(evaluators);

		let plmc_eval_deposits: UserToPLMCBalance = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: UserToPLMCBalance = evaluations
			.iter()
			.map(|(account, _amount)| (account.clone(), get_ed()))
			.collect::<_>();

		let expected_remaining_plmc: UserToPLMCBalance =
			merge_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		test_env.mint_plmc_to(plmc_eval_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());

		evaluating_project.bond_for_users(evaluations).unwrap();

		let expected_evaluator_balances =
			sum_balance_mappings(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

		let expected_total_supply = prev_supply + expected_evaluator_balances;

		evaluating_project.evaluation_assertions(expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

		evaluating_project.start_auction(issuer).unwrap()
	}

	fn bid_for_users(&self, bids: TestBids) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for bid in bids {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::bid(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.amount,
					bid.price,
					bid.multiplier,
					bid.asset,
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

		let candle_start = english_end + 2;

		self.test_env
			.advance_time(candle_start.saturating_sub(self.test_env.current_block()));
		let candle_end = self
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");

		let community_start = candle_end + 2;

		self.test_env
			.advance_time(community_start.saturating_sub(self.test_env.current_block()));

		assert_eq!(self.get_project_details().status, ProjectStatus::CommunityRound);

		CommunityFundingProject {
			test_env: self.test_env,
			issuer: self.issuer,
			project_id: self.project_id,
		}
	}
}

#[derive(Debug)]
pub struct CommunityFundingProject<'a> {
	test_env: &'a TestEnvironment,
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for CommunityFundingProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> CommunityFundingProject<'a> {
	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId, evaluations: UserToUSDBalance, bids: TestBids,
	) -> Self {
		let auctioning_project = AuctioningProject::new_with(test_env, project_metadata, issuer, evaluations);

		let project_id = auctioning_project.get_project_id();
		let bidders = bids
			.iter()
			.map(|b| b.bidder.clone())
			.collect::<Vec<AccountIdOf<TestRuntime>>>();
		let asset_id = bids[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(bidders.clone());
		let prev_funding_asset_balances = test_env.get_free_statemint_asset_balances_for(asset_id, bidders);
		let plmc_bid_deposits: UserToPLMCBalance = calculate_auction_plmc_spent(bids.clone());
		let plmc_existential_deposits: UserToPLMCBalance = bids.iter().map(|bid| (bid.bidder, get_ed())).collect::<_>();
		let funding_asset_deposits = calculate_auction_funding_asset_spent(bids.clone());

		let bidder_balances = sum_balance_mappings(vec![plmc_bid_deposits.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			merge_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		let bid_expectations = bids
			.iter()
			.map(|bid| BidInfoFilter {
				ct_amount: Some(bid.amount),
				ct_usd_price: Some(bid.price),
				..Default::default()
			})
			.collect::<Vec<_>>();
		let total_ct_sold = bids.iter().map(|bid| bid.amount).sum::<u128>();

		test_env.mint_plmc_to(plmc_bid_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		auctioning_project.bid_for_users(bids).expect("Bidding should work");

		test_env.do_reserved_plmc_assertions(plmc_bid_deposits, BondType::Bid(project_id));
		test_env.do_bid_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		let community_project = auctioning_project.start_community_funding();

		community_project.finalized_bids_assertions(bid_expectations, total_ct_sold);

		community_project
	}

	fn buy_for_retail_users(&self, contributions: TestContributions) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for cont in contributions {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::contribute(
					RuntimeOrigin::signed(cont.contributor),
					project_id,
					cont.amount,
					cont.multiplier,
					cont.asset,
				)
			})?;
		}
		Ok(())
	}

	fn finalized_bids_assertions(
		&self, bid_expectations: Vec<BidInfoFilterOf<TestRuntime>>, expected_ct_sold: BalanceOf<TestRuntime>,
	) {
		let project_metadata = self.get_project_metadata();
		let project_details = self.get_project_details();
		let project_id = self.get_project_id();
		let project_bids = self.in_ext(|| Bids::<TestRuntime>::iter_prefix(project_id).collect::<Vec<_>>());
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
	}

	fn start_remainder_funding(self) -> RemainderFundingProject<'a> {
		assert_eq!(self.get_project_details().status, ProjectStatus::CommunityRound);
		let community_funding_end = self
			.get_project_details()
			.phase_transition_points
			.community
			.end()
			.expect("Community funding end point should exist");
		let remainder_start = community_funding_end + 1;
		self.test_env
			.advance_time(remainder_start.saturating_sub(self.test_env.current_block()));
		assert_eq!(self.get_project_details().status, ProjectStatus::RemainderRound);
		RemainderFundingProject {
			test_env: self.test_env,
			issuer: self.issuer,
			project_id: self.project_id,
		}
	}
}

#[derive(Debug)]
struct RemainderFundingProject<'a> {
	test_env: &'a TestEnvironment,
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for RemainderFundingProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> RemainderFundingProject<'a> {
	fn buy_for_any_user(&self, contributions: TestContributions) -> Result<(), DispatchError> {
		let project_id = self.get_project_id();
		for cont in contributions {
			self.test_env.ext_env.borrow_mut().execute_with(|| {
				FundingModule::contribute(
					RuntimeOrigin::signed(cont.contributor),
					project_id,
					cont.amount,
					cont.multiplier,
					cont.asset,
				)
			})?;
		}
		Ok(())
	}

	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>, issuer: AccountId,
		evaluations: UserToUSDBalance, bids: TestBids, contributions: TestContributions,
	) -> Self {
		let community_funding_project =
			CommunityFundingProject::new_with(test_env, project_metadata, issuer, evaluations, bids);

		let project_id = community_funding_project.get_project_id();
		let ct_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.unwrap();
		let contributors = contributions.iter().map(|cont| cont.contributor).collect::<Vec<_>>();
		let asset_id = contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances =
			test_env.get_free_statemint_asset_balances_for(asset_id, contributors.clone());
		let plmc_contribution_deposits = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		let plmc_existential_deposits: UserToPLMCBalance = contributors
			.iter()
			.map(|acc| (acc.clone(), get_ed()))
			.collect::<Vec<_>>();
		let funding_asset_deposits = calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		let contributor_balances = sum_balance_mappings(vec![
			plmc_contribution_deposits.clone(),
			plmc_existential_deposits.clone(),
		]);

		let expected_free_plmc_balances =
			merge_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		test_env.mint_plmc_to(plmc_contribution_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		community_funding_project
			.buy_for_retail_users(contributions.clone())
			.expect("Contributing should work");

		test_env.do_reserved_plmc_assertions(plmc_contribution_deposits, BondType::Contribution(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		community_funding_project.start_remainder_funding()
	}

	fn end_funding(&self) -> FinishedProject<'a> {
		assert_eq!(self.get_project_details().status, ProjectStatus::RemainderRound);
		let remainder_funding_end = self
			.get_project_details()
			.phase_transition_points
			.remainder
			.end()
			.expect("Should have remainder end");
		let finish_block = remainder_funding_end + 1;
		self.test_env
			.advance_time(finish_block.saturating_sub(self.test_env.current_block()));
		assert_eq!(self.get_project_details().status, ProjectStatus::FundingEnded);

		FinishedProject {
			test_env: self.test_env,
			issuer: self.issuer.clone(),
			project_id: self.project_id.clone(),
		}
	}
}

#[derive(Debug)]
struct FinishedProject<'a> {
	test_env: &'a TestEnvironment,
	issuer: AccountId,
	project_id: ProjectIdOf<TestRuntime>,
}
impl<'a> ProjectInstance for FinishedProject<'a> {
	fn get_test_environment(&self) -> &TestEnvironment {
		self.test_env
	}
	fn get_issuer(&self) -> AccountId {
		self.issuer.clone()
	}
	fn get_project_id(&self) -> ProjectIdOf<TestRuntime> {
		self.project_id.clone()
	}
}
impl<'a> FinishedProject<'a> {
	fn new_with(
		test_env: &'a TestEnvironment, project_metadata: ProjectMetadataOf<TestRuntime>, issuer: AccountId,
		evaluations: UserToUSDBalance, bids: TestBids, community_contributions: TestContributions,
		remainder_contributions: TestContributions,
	) -> Self {
		let remainder_funding_project = RemainderFundingProject::new_with(
			test_env,
			project_metadata.clone(),
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
		);

		let project_id = remainder_funding_project.get_project_id();
		let ct_price = remainder_funding_project
			.get_project_details()
			.weighted_average_price
			.unwrap();
		let contributors = remainder_contributions
			.iter()
			.map(|cont| cont.contributor)
			.collect::<Vec<_>>();
		let asset_id = remainder_contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances =
			test_env.get_free_statemint_asset_balances_for(asset_id, contributors.clone());
		let plmc_contribution_deposits = calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);
		let plmc_existential_deposits: UserToPLMCBalance = contributors
			.iter()
			.map(|acc| (acc.clone(), get_ed()))
			.collect::<Vec<_>>();
		let funding_asset_deposits =
			calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

		let contributor_balances = sum_balance_mappings(vec![
			plmc_contribution_deposits.clone(),
			plmc_existential_deposits.clone(),
		]);

		let expected_free_plmc_balances =
			merge_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		test_env.mint_plmc_to(plmc_contribution_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		remainder_funding_project
			.buy_for_any_user(remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		test_env.do_reserved_plmc_assertions(plmc_contribution_deposits, BondType::Contribution(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		let finished_project = remainder_funding_project.end_funding();

		// Check that remaining CTs are updated
		let project_details = finished_project.get_project_details();
		let auction_bought_tokens: u128 = bids.iter().map(|bid| bid.amount).sum();
		let community_bought_tokens: u128 = community_contributions.iter().map(|cont| cont.amount).sum();
		let remainder_bought_tokens: u128 = remainder_contributions.iter().map(|cont| cont.amount).sum();

		assert_eq!(
			project_details.remaining_contribution_tokens,
			project_metadata.total_allocation_size
				- auction_bought_tokens
				- community_bought_tokens
				- remainder_bought_tokens,
			"Remaining CTs are incorrect"
		);

		finished_project
	}
}

mod defaults {
	use super::*;

	pub fn default_project(nonce: u64) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		ProjectMetadata {
			total_allocation_size: 1_000_000_0_000_000_000,
			minimum_price: PriceOf::<TestRuntime>::from_float(1.0),
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

	pub fn default_plmc_balances() -> UserToPLMCBalance {
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

	pub fn default_evaluations() -> UserToPLMCBalance {
		vec![
			(EVALUATOR_1, 50_000 * PLMC),
			(EVALUATOR_2, 25_000 * PLMC),
			(EVALUATOR_3, 32_000 * PLMC),
		]
	}

	pub fn default_failing_evaluations() -> UserToPLMCBalance {
		vec![(EVALUATOR_1, 10_000 * PLMC), (EVALUATOR_2, 5_000 * PLMC)]
	}

	pub fn default_bids() -> TestBids {
		// This should reflect the bidding currency, which currently is USDT
		vec![
			TestBid::new(
				BIDDER_1,
				3000 * ASSET_UNIT,
				50u128.into(),
				None,
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(
				BIDDER_2,
				5000 * ASSET_UNIT,
				15u128.into(),
				None,
				AcceptedFundingAsset::USDT,
			),
		]
	}

	pub fn default_community_buys() -> TestContributions {
		vec![
			TestContribution::new(BUYER_1, 10 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 20 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_remainder_buys() -> TestContributions {
		vec![
			TestContribution::new(EVALUATOR_2, 6 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
		]
	}
}

pub mod helper_functions {
	use super::*;

	pub fn get_ed() -> BalanceOf<TestRuntime> {
		<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get()
	}

	pub fn calculate_evaluation_plmc_spent(evals: UserToPLMCBalance) -> UserToPLMCBalance {
		let plmc_price = PriceMap::get().get(&PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = UserToPLMCBalance::new();
		for eval in evals {
			let usd_bond = eval.1;
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push((eval.0, plmc_bond));
		}
		output
	}

	pub fn calculate_auction_plmc_spent(bids: TestBids) -> UserToPLMCBalance {
		let plmc_price = PriceMap::get().get(&PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = UserToPLMCBalance::new();
		for bid in bids {
			let usd_ticket_size = bid.price.saturating_mul_int(bid.amount);
			let usd_bond = bid
				.multiplier
				.unwrap_or_default()
				.calculate_bonding_requirement(usd_ticket_size)
				.unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push((bid.bidder, plmc_bond));
		}
		output
	}

	pub fn calculate_auction_funding_asset_spent(bids: TestBids) -> UserToStatemintAsset {
		let mut output = UserToStatemintAsset::new();
		for bid in bids {
			let asset_price = PriceMap::get().get(&(bid.asset.to_statemint_id())).unwrap().clone();
			let usd_ticket_size = bid.price.saturating_mul_int(bid.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push((bid.bidder, funding_asset_spent, bid.asset.to_statemint_id()));
		}
		output
	}

	pub fn calculate_contributed_plmc_spent(
		contributions: TestContributions, token_usd_price: PriceOf<TestRuntime>,
	) -> UserToPLMCBalance {
		let plmc_price = PriceMap::get().get(&PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = UserToPLMCBalance::new();
		for cont in contributions {
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let usd_bond = cont
				.multiplier
				.unwrap_or_default()
				.calculate_bonding_requirement(usd_ticket_size)
				.unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push((cont.contributor, plmc_bond));
		}
		output
	}

	pub fn calculate_contributed_funding_asset_spent(
		contributions: TestContributions, token_usd_price: PriceOf<TestRuntime>,
	) -> UserToStatemintAsset {
		let mut output = UserToStatemintAsset::new();
		for cont in contributions {
			let asset_price = PriceMap::get().get(&(cont.asset.to_statemint_id())).unwrap().clone();
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push((cont.contributor, funding_asset_spent, cont.asset.to_statemint_id()));
		}
		output
	}

	pub fn merge_mappings_by_user<I: std::ops::Add<Output = I>>(
		mut mappings: Vec<Vec<(AccountIdOf<TestRuntime>, I)>>,
	) -> Vec<(AccountIdOf<TestRuntime>, I)> {
		let mut output = mappings.swap_remove(0);
		for map in mappings {
			output = zip(output, map)
				.map(|(a, b)| {
					assert_eq!(a.0, b.0, "Bad order for users");
					(a.0, a.1 + b.1)
				})
				.collect();
		}
		output
	}

	pub fn sum_balance_mappings<I: sp_std::ops::Add<Output = I> + sp_std::iter::Sum>(
		mut mappings: Vec<Vec<(AccountIdOf<TestRuntime>, I)>>,
	) -> I {
		let mut output: I = mappings.swap_remove(0).into_iter().map(|(_, val)| val).sum();
		for map in mappings {
			output = output + map.into_iter().map(|(_, val)| val).sum();
		}
		output
	}

	pub fn sum_statemint_mappings<I: sp_std::ops::Add<Output = I> + sp_std::iter::Sum, S>(
		mut mappings: Vec<Vec<(AccountIdOf<TestRuntime>, I, S)>>,
	) -> I {
		let mut output: I = mappings.swap_remove(0).into_iter().map(|(_, val, _)| val).sum();
		for map in mappings {
			output = output + map.into_iter().map(|(_, val, _)| val).sum();
		}
		output
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
	fn creation_round_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());

		EvaluatingProject::new_with(&test_env, project, issuer);
	}

	#[test]
	fn multiple_creation_rounds() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project1 = default_project(test_env.get_new_nonce());
		let project2 = default_project(test_env.get_new_nonce());
		let project3 = default_project(test_env.get_new_nonce());
		let project4 = default_project(test_env.get_new_nonce());

		EvaluatingProject::new_with(&test_env, project1, issuer);
		EvaluatingProject::new_with(&test_env, project2, issuer);
		EvaluatingProject::new_with(&test_env, project3, issuer);
		EvaluatingProject::new_with(&test_env, project4, issuer);
	}

	#[test]
	fn project_id_autoincrement_works() {
		let test_env = TestEnvironment::new();
		let project_1 = default_project(test_env.get_new_nonce());
		let project_2 = default_project(test_env.get_new_nonce());
		let project_3 = default_project(test_env.get_new_nonce());

		let created_project_1 = CreatedProject::new_with(&test_env, project_1, ISSUER);
		let created_project_2 = CreatedProject::new_with(&test_env, project_2, ISSUER);
		let created_project_3 = CreatedProject::new_with(&test_env, project_3, ISSUER);

		assert_eq!(created_project_1.get_project_id(), 0);
		assert_eq!(created_project_2.get_project_id(), 1);
		assert_eq!(created_project_3.get_project_id(), 2);
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
				FundingModule::create(RuntimeOrigin::signed(ISSUER), project_metadata),
				Error::<TestRuntime>::NotAuthorized
			);
		})
	}

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 0u128.into(),
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
			minimum_price: 1u128.into(),
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
			minimum_price: 1u128.into(),
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
			minimum_price: 0u128.into(),
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

	#[test]
	fn evaluation_round_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		AuctioningProject::new_with(&test_env, project, issuer, evaluations);
	}

	#[test]
	fn multiple_evaluation_projects() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project1 = default_project(test_env.get_new_nonce());
		let project2 = default_project(test_env.get_new_nonce());
		let project3 = default_project(test_env.get_new_nonce());
		let project4 = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		AuctioningProject::new_with(&test_env, project1, issuer, evaluations.clone());
		AuctioningProject::new_with(&test_env, project2, issuer, evaluations.clone());
		AuctioningProject::new_with(&test_env, project3, issuer, evaluations.clone());
		AuctioningProject::new_with(&test_env, project4, issuer, evaluations);
	}
}

#[cfg(test)]
mod evaluation_round_failure {
	use super::*;

	#[test]
	fn not_enough_bonds() {
		let test_env = TestEnvironment::new();
		let now = test_env.current_block();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_failing_evaluations();
		let plmc_eval_deposits: UserToPLMCBalance = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: UserToPLMCBalance = evaluations
			.iter()
			.map(|(account, _amount)| (account.clone(), get_ed()))
			.collect::<_>();
		let expected_evaluator_balances =
			merge_mappings_by_user(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

		test_env.mint_plmc_to(plmc_eval_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());

		let evaluating_project = EvaluatingProject::new_with(&test_env, project, issuer);

		let evaluation_end = evaluating_project
			.get_project_details()
			.phase_transition_points
			.evaluation
			.end
			.expect("Evaluation round end block should be set");
		let project_id = evaluating_project.get_project_id();

		evaluating_project
			.bond_for_users(default_failing_evaluations())
			.expect("Bonding should work");

		test_env.do_free_plmc_assertions(plmc_existential_deposits);
		test_env.do_reserved_plmc_assertions(plmc_eval_deposits, BondType::Evaluation(project_id));

		test_env.advance_time(evaluation_end - now + 1);

		assert_eq!(
			evaluating_project.get_project_details().status,
			ProjectStatus::EvaluationFailed
		);

		// Check that on_idle has unlocked the failed bonds
		test_env.advance_time(10);
		test_env.do_free_plmc_assertions(expected_evaluator_balances);
	}

	#[test]
	fn insufficient_balance() {
		let test_env = TestEnvironment::new();
		let _now = test_env.current_block();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let insufficient_eval_deposits = calculate_evaluation_plmc_spent(evaluations.clone())
			.iter()
			.map(|(account, amount)| (account.clone(), amount / 2))
			.collect::<UserToPLMCBalance>();

		let plmc_existential_deposits: UserToPLMCBalance = evaluations
			.iter()
			.map(|(account, _amount)| (account.clone(), get_ed()))
			.collect::<_>();

		test_env.mint_plmc_to(insufficient_eval_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits);

		let evaluating_project = EvaluatingProject::new_with(&test_env, project, issuer);

		let dispatch_error = evaluating_project.bond_for_users(evaluations).unwrap_err();
		assert_eq!(dispatch_error, Error::<TestRuntime>::InsufficientBalance.into())
	}
}

#[cfg(test)]
mod auction_round_success {
	use super::*;

	#[test]
	fn auction_round_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let _community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
	}

	#[test]
	fn multiple_auction_projects_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project1 = default_project(test_env.get_new_nonce());
		let project2 = default_project(test_env.get_new_nonce());
		let project3 = default_project(test_env.get_new_nonce());
		let project4 = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();

		CommunityFundingProject::new_with(&test_env, project1, issuer, evaluations.clone(), bids.clone());
		CommunityFundingProject::new_with(&test_env, project2, issuer, evaluations.clone(), bids.clone());
		CommunityFundingProject::new_with(&test_env, project3, issuer, evaluations.clone(), bids.clone());
		CommunityFundingProject::new_with(&test_env, project4, issuer, evaluations, bids);
	}

	#[test]
	fn price_calculation() {
		// Calculate the weighted price of the token for the next funding rounds, using winning bids.
		// for example: if there are 3 winning bids,
		// A: 10K tokens @ USD15 per token = 150K USD value
		// B: 20K tokens @ USD20 per token = 400K USD value
		// C: 20K tokens @ USD10 per token = 200K USD value,

		// then the weight for each bid is:
		// A: 150K / (150K + 400K + 200K) = 0.20
		// B: 400K / (150K + 400K + 200K) = 0.533...
		// C: 200K / (150K + 400K + 200K) = 0.266...

		// then multiply each weight by the price of the token to get the weighted price per bid
		// A: 0.20 * 15 = 3
		// B: 0.533... * 20 = 10.666...
		// C: 0.266... * 10 = 2.666...

		// lastly, sum all the weighted prices to get the final weighted price for the next funding round
		// 3 + 10.6 + 2.6 = 16.333...
		let test_env = TestEnvironment::new();
		let project_metadata = default_project(test_env.get_new_nonce());
		let auctioning_project =
			AuctioningProject::new_with(&test_env, project_metadata, ISSUER, default_evaluations());
		let bids = vec![
			TestBid::new(100, 10_000_0_000_000_000, 15.into(), None, AcceptedFundingAsset::USDT),
			TestBid::new(200, 20_000_0_000_000_000, 20.into(), None, AcceptedFundingAsset::USDT),
			TestBid::new(300, 20_000_0_000_000_000, 10.into(), None, AcceptedFundingAsset::USDT),
		];
		let statemint_funding = calculate_auction_funding_asset_spent(bids.clone());
		let plmc_funding = calculate_auction_plmc_spent(bids.clone());
		let ed_funding = plmc_funding
			.clone()
			.into_iter()
			.map(|(account, _amount)| (account, get_ed()))
			.collect::<UserToPLMCBalance>();

		test_env.mint_plmc_to(ed_funding);
		test_env.mint_plmc_to(plmc_funding);
		test_env.mint_statemint_asset_to(statemint_funding);

		auctioning_project.bid_for_users(bids).unwrap();

		let community_funding_project = auctioning_project.start_community_funding();
		let token_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.unwrap();

		let price_in_10_decimals = token_price.checked_mul_int(1_0_000_000_000u128).unwrap();
		let price_in_12_decimals = token_price.checked_mul_int(1_000_000_000_000u128).unwrap();
		assert_eq!(price_in_10_decimals, 16_3_333_333_333u128);
		assert_eq!(price_in_12_decimals, 16_333_333_333_333u128);
	}

	#[test]
	fn only_candle_bids_before_random_block_get_included() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let auctioning_project = AuctioningProject::new_with(&test_env, project, issuer, evaluations);
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
			auctioning_project.get_project_details().status,
			ProjectStatus::AuctionRound(AuctionPhase::Candle)
		);

		let candle_end_block = auctioning_project
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle auction end point should exist");

		let mut bidding_account = 1000;
		// Imitate the first default bid
		let bid_info = default_bids()[0];
		let plmc_necessary_funding = calculate_auction_plmc_spent(vec![bid_info.clone()])[0].1;
		let statemint_asset_necessary_funding = calculate_auction_funding_asset_spent(vec![bid_info.clone()])[0].1;

		let mut bids_made: TestBids = vec![];
		let starting_bid_block = test_env.current_block();
		let blocks_to_bid = test_env.current_block()..candle_end_block;

		// Do one candle bid for each block until the end of candle auction with a new user
		for _block in blocks_to_bid {
			assert_eq!(
				auctioning_project.get_project_details().status,
				ProjectStatus::AuctionRound(AuctionPhase::Candle)
			);
			test_env.mint_plmc_to(vec![(bidding_account, get_ed())]);
			test_env.mint_plmc_to(vec![(bidding_account, plmc_necessary_funding)]);
			test_env.mint_statemint_asset_to(vec![(
				bidding_account,
				statemint_asset_necessary_funding,
				bid_info.asset.to_statemint_id(),
			)]);
			let bids: TestBids = vec![TestBid::new(
				bidding_account,
				bid_info.amount,
				bid_info.price,
				bid_info.multiplier,
				bid_info.asset,
			)];
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

		for bid in included_bids {
			let pid = auctioning_project.get_project_id();
			let stored_bids = auctioning_project.in_ext(|| FundingModule::bids(pid, bid.bidder));
			let desired_bid = BidInfoFilter {
				project: Some(pid),
				bidder: Some(bid.bidder),
				ct_amount: Some(bid.amount),
				ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Accepted),
				..Default::default()
			};

			assert!(
				stored_bids.iter().any(|bid| desired_bid.matches_bid(&bid)),
				"Stored bid does not match the given filter"
			)
		}

		for bid in excluded_bids {
			let pid = auctioning_project.get_project_id();
			let stored_bids = auctioning_project.in_ext(|| FundingModule::bids(pid, bid.bidder));
			let desired_bid = BidInfoFilter {
				project: Some(pid),
				bidder: Some(bid.bidder),
				ct_amount: Some(bid.amount),
				ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Rejected(RejectionReason::AfterCandleEnd)),
				..Default::default()
			};
			assert!(
				stored_bids.iter().any(|bid| desired_bid.matches_bid(&bid)),
				"Stored bid does not match the given filter"
			);
		}
	}
}

#[cfg(test)]
mod auction_round_failure {
	use super::*;

	#[test]
	fn cannot_start_auction_before_evaluation_finishes() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let project_id = evaluating_project.project_id;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::start_auction(RuntimeOrigin::signed(ISSUER), project_id),
				Error::<TestRuntime>::EvaluationPeriodNotEnded
			);
		});
	}

	#[test]
	fn cannot_bid_before_auction_round() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let _project_id = evaluating_project.project_id;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::bid(
					RuntimeOrigin::signed(BIDDER_2),
					0,
					1,
					100u128.into(),
					None,
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn contribute_does_not_work() {
		let test_env = TestEnvironment::new();
		let evaluating_project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let project_id = evaluating_project.project_id;
		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_noop!(
				FundingModule::contribute(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					100,
					None,
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::AuctionNotStarted
			);
		});
	}

	#[test]
	fn bids_overflow() {
		let test_env = TestEnvironment::new();
		let auctioning_project =
			AuctioningProject::new_with(&test_env, default_project(0), ISSUER, default_evaluations());
		let project_id = auctioning_project.project_id;
		const DAVE: AccountId = 42;
		let bids: TestBids = vec![
			TestBid::new(DAVE, 10_000 * USDT_UNIT, 2u128.into(), None, AcceptedFundingAsset::USDT), // 20k
			TestBid::new(DAVE, 12_000 * USDT_UNIT, 8u128.into(), None, AcceptedFundingAsset::USDT), // 96k
			TestBid::new(DAVE, 15_000 * USDT_UNIT, 5u128.into(), None, AcceptedFundingAsset::USDT), // 75k
			TestBid::new(DAVE, 1_000 * USDT_UNIT, 7u128.into(), None, AcceptedFundingAsset::USDT),  // 7k
			TestBid::new(DAVE, 20_000 * USDT_UNIT, 5u128.into(), None, AcceptedFundingAsset::USDT), // 100k
		];

		let mut plmc_fundings: UserToPLMCBalance = calculate_auction_plmc_spent(bids.clone());
		// Existential deposit on DAVE
		plmc_fundings.push((DAVE, get_ed()));

		let statemint_asset_fundings: UserToStatemintAsset = calculate_auction_funding_asset_spent(bids.clone());

		// Fund enough for all PLMC bonds for the bids (multiplier of 1)
		test_env.mint_plmc_to(plmc_fundings);

		// Fund enough for all bids
		test_env.mint_statemint_asset_to(statemint_asset_fundings);

		auctioning_project.bid_for_users(bids).expect("Bids should pass");

		test_env.ext_env.borrow_mut().execute_with(|| {
			let stored_bids = FundingModule::bids(project_id, DAVE);
			assert_eq!(stored_bids.len(), 4);
			assert_eq!(stored_bids[0].ct_usd_price, 5u128.into());
			assert_eq!(stored_bids[1].ct_usd_price, 8u128.into());
			assert_eq!(stored_bids[2].ct_usd_price, 5u128.into());
			assert_eq!(stored_bids[3].ct_usd_price, 2u128.into());
		});
	}

	#[test]
	fn bid_with_asset_not_accepted() {
		let test_env = TestEnvironment::new();
		let auctioning_project =
			AuctioningProject::new_with(&test_env, default_project(0), ISSUER, default_evaluations());
		let mul_2 = MultiplierOf::<TestRuntime>::from(2u32);
		let bids = vec![
			TestBid::new(BIDDER_1, 10_000, 2u128.into(), None, AcceptedFundingAsset::USDC),
			TestBid::new(BIDDER_2, 13_000, 3u128.into(), Some(mul_2), AcceptedFundingAsset::USDC),
		];
		let outcome = auctioning_project.bid_for_users(bids);
		frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
	}
}

#[cfg(test)]
mod community_round_success {
	use super::*;

	pub const HOURS: BlockNumber = 300u64;

	#[test]
	fn community_round_completed() {
		let test_env = TestEnvironment::new();
		let _community_funding_project = RemainderFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		);
	}

	#[test]
	fn multiple_contribution_projects_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project1 = default_project(test_env.get_new_nonce());
		let project2 = default_project(test_env.get_new_nonce());
		let project3 = default_project(test_env.get_new_nonce());
		let project4 = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_buys = default_community_buys();

		RemainderFundingProject::new_with(
			&test_env,
			project1,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		RemainderFundingProject::new_with(
			&test_env,
			project2,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		RemainderFundingProject::new_with(
			&test_env,
			project3,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_buys.clone(),
		);
		RemainderFundingProject::new_with(&test_env, project4, issuer, evaluations, bids, community_buys);
	}

	#[test]
	fn contribute_multiple_times_works() {
		let test_env = TestEnvironment::new();
		let metadata = default_project(0);
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, metadata, issuer, evaluations, bids);

		const BOB: AccountId = 42;
		let token_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.unwrap();
		let contributions: TestContributions = vec![
			TestContribution::new(BOB, 3 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
			TestContribution::new(BOB, 4 * ASSET_UNIT, None, AcceptedFundingAsset::USDT),
		];

		let mut plmc_funding: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), token_price);
		plmc_funding.push((BOB, get_ed()));
		let statemint_funding: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		test_env.mint_plmc_to(plmc_funding);
		test_env.mint_statemint_asset_to(statemint_funding.clone());

		// TODO: Set a reasonable amount of Contribution Tokens that the user wants to buy
		community_funding_project
			.buy_for_retail_users(vec![contributions[0]])
			.expect("The Buyer should be able to buy multiple times");
		test_env.advance_time((1 * HOURS) as BlockNumber);

		community_funding_project
			.buy_for_retail_users(vec![contributions[1]])
			.expect("The Buyer should be able to buy multiple times");

		let project_id = community_funding_project.get_project_id();
		let bob_total_contributions: BalanceOf<TestRuntime> = community_funding_project.in_ext(|| {
			Contributions::<TestRuntime>::get(project_id, BOB)
				.iter()
				.map(|c| c.funding_asset_amount)
				.sum()
		});

		let total_contributed = calculate_contributed_funding_asset_spent(contributions.clone(), token_price)
			.iter()
			.map(|(_account, amount, _asset)| amount)
			.sum::<BalanceOf<TestRuntime>>();

		assert_eq!(bob_total_contributions, total_contributed);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_exact() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		const BOB: AccountId = 808;

		let remaining_ct = community_funding_project
			.get_project_details()
			.remaining_contribution_tokens;
		let ct_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.expect("CT Price should exist");
		let project_id = community_funding_project.get_project_id();

		let contributions: TestContributions = vec![TestContribution::new(
			BOB,
			remaining_ct,
			None,
			AcceptedFundingAsset::USDT,
		)];
		let mut plmc_fundings: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push((BOB, get_ed()));
		let statemint_asset_fundings: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		community_funding_project
			.buy_for_retail_users(contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64);
		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project
				.get_project_details()
				.remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(
			community_funding_project.get_project_details().status,
			ProjectStatus::FundingEnded
		);

		test_env.do_free_plmc_assertions(vec![plmc_fundings[1].clone()]);
		test_env.do_free_statemint_asset_assertions(vec![(BOB, 0u128, AcceptedFundingAsset::USDT.to_statemint_id())]);
		test_env.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], BondType::Contribution(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(
			statemint_asset_fundings,
			community_funding_project.get_project_id(),
		);
	}

	#[test]
	fn community_round_ends_on_all_ct_sold_overbuy() {
		let test_env = TestEnvironment::new();
		let community_funding_project = CommunityFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		const BOB: AccountId = 808;
		const OVERBUY_CT: BalanceOf<TestRuntime> = 40 * ASSET_UNIT;

		let remaining_ct = community_funding_project
			.get_project_details()
			.remaining_contribution_tokens;

		let ct_price = community_funding_project
			.get_project_details()
			.weighted_average_price
			.expect("CT Price should exist");

		let project_id = community_funding_project.get_project_id();

		let contributions: TestContributions = vec![
			TestContribution::new(BOB, remaining_ct, None, AcceptedFundingAsset::USDT),
			TestContribution::new(BOB, OVERBUY_CT, None, AcceptedFundingAsset::USDT),
		];
		let mut plmc_fundings: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push((BOB, get_ed()));
		let mut statemint_asset_fundings: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		community_funding_project
			.buy_for_retail_users(contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64);

		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project
				.get_project_details()
				.remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(
			community_funding_project.get_project_details().status,
			ProjectStatus::FundingEnded
		);

		let reserved_plmc = plmc_fundings.swap_remove(0).1;
		let remaining_plmc: Balance = plmc_fundings.iter().fold(0u128, |acc, (_, amount)| acc + amount);

		let actual_funding_transferred = statemint_asset_fundings.swap_remove(0).1;
		let remaining_statemint_assets: Balance = statemint_asset_fundings
			.iter()
			.fold(0u128, |acc, (_, amount, _)| acc + amount);

		test_env.do_free_plmc_assertions(vec![(BOB, remaining_plmc)]);
		test_env.do_free_statemint_asset_assertions(vec![(
			BOB,
			remaining_statemint_assets,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		test_env.do_reserved_plmc_assertions(vec![(BOB, reserved_plmc)], BondType::Contribution(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(
			vec![(
				BOB,
				actual_funding_transferred,
				AcceptedFundingAsset::USDT.to_statemint_id(),
			)],
			community_funding_project.get_project_id(),
		);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_same_mult_diff_ct() {
		let test_env = TestEnvironment::new();
		let project = CommunityFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		let project_id = project.get_project_id();
		const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

		let project_details = project.get_project_details();
		let token_price = project_details.weighted_average_price.unwrap();

		// Create a contribution vector that will reach the limit of contributions for a user-project
		let multiplier: Option<MultiplierOf<TestRuntime>> = None;
		let token_amount: BalanceOf<TestRuntime> = 1 * ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: TestContributions = range
			.map(|_| TestContribution::new(CONTRIBUTOR, token_amount, multiplier, AcceptedFundingAsset::USDT))
			.collect();

		let plmc_funding = calculate_contributed_plmc_spent(contributions.clone(), token_price);
		let ed_funding: UserToPLMCBalance = vec![(CONTRIBUTOR, get_ed())];
		let statemint_funding = calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		test_env.mint_plmc_to(plmc_funding.clone());
		test_env.mint_plmc_to(ed_funding);
		test_env.mint_statemint_asset_to(statemint_funding.clone());

		// Reach the limit of contributions for a user-project
		project.buy_for_retail_users(contributions.clone()).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let contributor_post_buy_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_buy_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, get_ed());
		assert_eq!(contributor_post_buy_statemint_asset_balance, 0);

		let plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Contribution(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::get(project.project_id, CONTRIBUTOR)
				.iter()
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(
			statemint_asset_contributions_stored,
			sum_statemint_mappings(vec![statemint_funding.clone()])
		);

		let new_multiplier: Option<MultiplierOf<TestRuntime>> = None;
		let new_token_amount: BalanceOf<TestRuntime> = 2 * ASSET_UNIT;
		let new_contribution: TestContributions = vec![TestContribution::new(
			CONTRIBUTOR,
			new_token_amount,
			new_multiplier,
			AcceptedFundingAsset::USDT,
		)];

		let new_plmc_funding = calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding = calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		test_env.mint_plmc_to(new_plmc_funding.clone());
		test_env.mint_statemint_asset_to(new_statemint_funding.clone());

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		let contributor_post_return_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(
			contributor_post_return_plmc_balance,
			contributor_post_buy_plmc_balance + plmc_funding[0].1
		);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding.clone()[0].1
		);

		let new_plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Contribution(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::get(project.project_id, CONTRIBUTOR)
				.iter()
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + sum_balance_mappings(vec![new_plmc_funding.clone()]) - plmc_funding[0].1
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored + sum_statemint_mappings(vec![new_statemint_funding.clone()])
				- statemint_funding[0].1
		);
	}

	#[test]
	fn contribution_is_returned_on_limit_reached_diff_mult_same_ct() {
		let test_env = TestEnvironment::new();
		let project = CommunityFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
		);
		let project_id = project.get_project_id();
		const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

		let project_details = project.get_project_details();
		let token_price = project_details.weighted_average_price.unwrap();

		// Create a contribution vector that will reach the limit of contributions for a user-project
		let multiplier: Option<MultiplierOf<TestRuntime>> = Some(MultiplierOf::<TestRuntime>::from(3u32));
		let token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: TestContributions = range
			.map(|_| TestContribution::new(CONTRIBUTOR, token_amount, multiplier, AcceptedFundingAsset::USDT))
			.collect();

		let plmc_funding = calculate_contributed_plmc_spent(contributions.clone(), token_price);
		let ed_funding: UserToPLMCBalance = vec![(CONTRIBUTOR, get_ed())];
		let statemint_funding = calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		test_env.mint_plmc_to(plmc_funding.clone());
		test_env.mint_plmc_to(ed_funding);
		test_env.mint_statemint_asset_to(statemint_funding.clone());

		// Reach the limit of contributions for a user-project
		project.buy_for_retail_users(contributions.clone()).unwrap();

		// Check that the right amount of PLMC is bonded, and funding currency is transferred
		let contributor_post_buy_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_buy_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, get_ed());
		assert_eq!(contributor_post_buy_statemint_asset_balance, 0);

		let plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Contribution(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::get(project.project_id, CONTRIBUTOR)
				.iter()
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(
			statemint_asset_contributions_stored,
			sum_statemint_mappings(vec![statemint_funding.clone()])
		);

		let new_multiplier: Option<MultiplierOf<TestRuntime>> = None;
		let new_token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let new_contribution: TestContributions = vec![TestContribution::new(
			CONTRIBUTOR,
			new_token_amount,
			new_multiplier,
			AcceptedFundingAsset::USDT,
		)];

		let new_plmc_funding = calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding = calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		test_env.mint_plmc_to(new_plmc_funding.clone());
		test_env.mint_statemint_asset_to(new_statemint_funding.clone());

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		let contributor_post_return_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(
			contributor_post_return_plmc_balance,
			contributor_post_buy_plmc_balance + plmc_funding[0].1
		);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding.clone()[0].1
		);

		let new_plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Contribution(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::get(project.project_id, CONTRIBUTOR)
				.iter()
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + sum_balance_mappings(vec![new_plmc_funding.clone()]) - plmc_funding[0].1
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored + sum_statemint_mappings(vec![new_statemint_funding.clone()])
				- statemint_funding[0].1
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
		let _remainder_funding_project = FinishedProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
			default_remainder_buys(),
		);
	}
}

#[cfg(test)]
mod purchased_vesting {
	use super::*;

	#[test]
	fn contribution_token_mints() {
		// TODO: currently the vesting is limited to the whole payment at once. We should test it with several payments over a vesting period.
		let test_env = TestEnvironment::new();
		let community_contributions = default_community_buys();
		let finished_project = FinishedProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			default_evaluations(),
			default_bids(),
			community_contributions.clone(),
			default_remainder_buys(),
		);
		let project_id = finished_project.project_id;

		for contribution in community_contributions {
			let result = test_env.in_ext(|| {
				FundingModule::vested_contribution_token_purchase_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
				)
			});
			assert_ok!(result);
			let minted_balance = test_env.in_ext(|| {
				<TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contribution.contributor)
			});
			let desired_balance = contribution.amount;
			assert_eq!(minted_balance, desired_balance);
		}
	}

	#[test]
	fn plmc_unbonded() {
		let test_env = TestEnvironment::new();
		let community_contributions = default_community_buys();
		let finished_project = FinishedProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			default_evaluations(),
			default_bids(),
			community_contributions.clone(),
			default_remainder_buys(),
		);
		let project_id = finished_project.project_id;
		let token_price = finished_project.get_project_details().weighted_average_price.unwrap();

		for contribution in community_contributions {
			let actual_bonded_plmc = test_env.in_ext(|| {
				<TestRuntime as Config>::NativeCurrency::balance_on_hold(
					&BondType::Contribution(project_id),
					&contribution.contributor,
				)
			});
			let desired_bonded_plmc = sum_balance_mappings(vec![calculate_contributed_plmc_spent(
				vec![contribution.clone()],
				token_price,
			)]);
			assert_eq!(actual_bonded_plmc, desired_bonded_plmc);

			let result = test_env.in_ext(|| {
				FundingModule::vested_plmc_purchase_unbond_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
				)
			});
			let actual_bonded_plmc = test_env.in_ext(|| {
				<TestRuntime as Config>::NativeCurrency::balance_on_hold(
					&BondType::Contribution(project_id),
					&contribution.contributor,
				)
			});
			assert_ok!(result);
			assert_eq!(actual_bonded_plmc, 0u32.into());
		}
	}
}

#[cfg(test)]
mod bids_vesting {
	use super::*;

	#[test]
	fn contribution_token_mints() {
		let test_env = TestEnvironment::new();
		let bids = default_bids();
		let finished_project = FinishedProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			default_evaluations(),
			bids.clone(),
			default_community_buys(),
			default_remainder_buys(),
		);
		let project_id = finished_project.project_id;

		for bid in bids {
			let actual_ct_balance =
				test_env.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bid.bidder));
			assert_eq!(actual_ct_balance, 0u32.into());

			let result = test_env.in_ext(|| {
				FundingModule::vested_contribution_token_bid_mint_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
				)
			});
			assert_ok!(result);
			let minted_balance =
				test_env.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bid.bidder));
			assert_eq!(minted_balance, bid.amount);
		}
	}

	#[test]
	fn plmc_unbonded() {
		let test_env = TestEnvironment::new();
		let bids = default_bids();
		let finished_project = FinishedProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			default_evaluations(),
			bids.clone(),
			default_community_buys(),
			default_remainder_buys(),
		);
		let project_id = finished_project.project_id;

		for bid in bids {
			let prev_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&bid.bidder));
			let bonded_plmc = test_env.in_ext(|| {
				<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Bid(project_id), &bid.bidder)
			});
			let desired_bond = calculate_auction_plmc_spent(vec![bid.clone()]);
			assert_eq!(bonded_plmc, sum_balance_mappings(vec![desired_bond.clone()]));

			let result = test_env.in_ext(|| {
				FundingModule::vested_plmc_bid_unbond_for(RuntimeOrigin::signed(bid.bidder), project_id, bid.bidder)
			});
			assert_ok!(result);
			let new_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(bid.bidder));
			assert_eq!(new_balance, prev_balance + sum_balance_mappings(vec![desired_bond]));
			let bonded_plmc = test_env.in_ext(|| {
				<TestRuntime as Config>::NativeCurrency::balance_on_hold(&BondType::Bid(project_id), &bid.bidder)
			});
			assert_eq!(bonded_plmc, 0u32.into());
		}
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
