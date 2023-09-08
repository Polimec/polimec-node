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

// If you feel like getting in touch with us, you ca ,n do so at info@polimec.org

//! Tests for Funding pallet.
use std::{
	cell::RefCell,
	cmp::min,
	collections::{BTreeMap, HashMap},
	iter::zip,
	ops::Div,
};

use assert_matches2::assert_matches;
use frame_support::{
	assert_err, assert_noop, assert_ok,
	pallet_prelude::{Decode, DispatchResultWithPostInfo},
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold, Mutate as FungibleMutate},
		fungibles::{
			metadata::Inspect as MetadataInspect, roles::Inspect as RolesInspect, Inspect as FungiblesInspect,
			Mutate as FungiblesMutate,
		},
		tokens::Balance as BalanceT,
		Get, OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
};
use itertools::Itertools;
use parachains_common::DAYS;
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_core::H256;
use sp_runtime::{DispatchError, Either, TokenError};
use sp_std::marker::PhantomData;

use defaults::*;
use helper_functions::*;
use polimec_traits::ReleaseSchedule;

use crate as pallet_funding;
use crate::{
	mock::{FundingModule, *},
	tests::testing_macros::{assert_close_enough, call_and_is_ok, extract_from_event},
	traits::{BondingRequirementCalculation, ProvideStatemintPrice, VestingDurationCalculation},
	CurrencyMetadata, Error, ParticipantsSize, ProjectMetadata, TicketSize,
	UpdateType::{CommunityFundingStart, RemainderFundingStart},
};

use super::*;

type ProjectIdOf<T> = <T as Config>::ProjectIdentifier;
type UserToPLMCBalance = Vec<(AccountId, BalanceOf<TestRuntime>)>;
type UserToUSDBalance = Vec<(AccountId, BalanceOf<TestRuntime>)>;
type UserToStatemintAsset =
	Vec<(AccountId, BalanceOf<TestRuntime>, <TestRuntime as pallet_assets::Config<StatemintAssetsInstance>>::AssetId)>;
type UserToCTBalance = Vec<(AccountId, BalanceOf<TestRuntime>, ProjectIdOf<TestRuntime>)>;

#[derive(Clone, Copy)]
pub struct TestBid {
	bidder: AccountId,
	amount: BalanceOf<TestRuntime>,
	price: PriceOf<TestRuntime>,
	multiplier: MultiplierOf<TestRuntime>,
	asset: AcceptedFundingAsset,
}
impl TestBid {
	fn new(
		bidder: AccountId,
		amount: BalanceOf<TestRuntime>,
		price: PriceOf<TestRuntime>,
		multiplier: u8,
		asset: AcceptedFundingAsset,
	) -> Self {
		Self { bidder, amount, price, multiplier: multiplier.try_into().unwrap(), asset }
	}

	fn from(bidder: AccountId, amount: BalanceOf<TestRuntime>, price: PriceOf<TestRuntime>) -> Self {
		Self { bidder, amount, price, multiplier: 1_u8.try_into().unwrap(), asset: AcceptedFundingAsset::USDT }
	}
}
pub type TestBids = Vec<TestBid>;

#[derive(Clone, Copy)]
pub struct TestContribution {
	contributor: AccountId,
	amount: BalanceOf<TestRuntime>,
	multiplier: MultiplierOf<TestRuntime>,
	asset: AcceptedFundingAsset,
}
impl TestContribution {
	fn new(
		contributor: AccountId,
		amount: BalanceOf<TestRuntime>,
		multiplier: u8,
		asset: AcceptedFundingAsset,
	) -> Self {
		Self { contributor, amount, multiplier: multiplier.try_into().unwrap(), asset }
	}

	fn from(contributor: AccountId, amount: BalanceOf<TestRuntime>) -> Self {
		Self { contributor, amount, multiplier: 1_u8.try_into().unwrap(), asset: AcceptedFundingAsset::USDT }
	}
}
pub type TestContributions = Vec<TestContribution>;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<Id, ProjectId, Balance: BalanceT, Price, AccountId, Multiplier, BlockNumber, PlmcVesting> {
	pub id: Option<Id>,
	pub project_id: Option<ProjectId>,
	pub bidder: Option<AccountId>,
	pub status: Option<BidStatus<Balance>>,
	pub original_ct_amount: Option<Balance>,
	pub original_ct_usd_price: Option<Price>,
	pub final_ct_amount: Option<Balance>,
	pub final_ct_usd_price: Option<Price>,
	pub funding_asset: Option<AcceptedFundingAsset>,
	pub funding_asset_amount_locked: Option<Balance>,
	pub multiplier: Option<Multiplier>,
	pub plmc_bond: Option<Balance>,
	pub funded: Option<bool>,
	pub plmc_vesting_info: Option<PlmcVesting>,
	pub when: Option<BlockNumber>,
	pub funds_released: Option<bool>,
}
type BidInfoFilterOf<T> = BidInfoFilter<
	u32,
	<T as Config>::ProjectIdentifier,
	BalanceOf<T>,
	PriceOf<T>,
	<T as frame_system::Config>::AccountId,
	MultiplierOf<T>,
	BlockNumberOf<T>,
	Option<VestingInfoOf<T>>,
>;
impl Default for BidInfoFilterOf<TestRuntime> {
	fn default() -> Self {
		BidInfoFilter {
			id: None,
			project_id: None,
			bidder: None,
			status: None,
			original_ct_amount: None,
			original_ct_usd_price: None,
			final_ct_amount: None,
			final_ct_usd_price: None,
			funding_asset: None,
			funding_asset_amount_locked: None,
			multiplier: None,
			plmc_bond: None,
			funded: None,
			plmc_vesting_info: None,
			when: None,
			funds_released: None,
		}
	}
}
impl BidInfoFilterOf<TestRuntime> {
	fn matches_bid(&self, bid: &BidInfoOf<TestRuntime>) -> bool {
		if self.id.is_some() && self.id.unwrap() != bid.id {
			return false
		}
		if self.project_id.is_some() && self.project_id.unwrap() != bid.project_id {
			return false
		}
		if self.bidder.is_some() && self.bidder.unwrap() != bid.bidder {
			return false
		}
		if self.status.is_some() && self.status.as_ref().unwrap() != &bid.status {
			return false
		}
		if self.original_ct_amount.is_some() && self.original_ct_amount.unwrap() != bid.original_ct_amount {
			return false
		}
		if self.original_ct_usd_price.is_some() && self.original_ct_usd_price.unwrap() != bid.original_ct_usd_price {
			return false
		}
		if self.final_ct_amount.is_some() && self.final_ct_amount.unwrap() != bid.final_ct_amount {
			return false
		}
		if self.final_ct_usd_price.is_some() && self.final_ct_usd_price.unwrap() != bid.final_ct_usd_price {
			return false
		}
		if self.funding_asset.is_some() && self.funding_asset.unwrap() != bid.funding_asset {
			return false
		}
		if self.funding_asset_amount_locked.is_some() &&
			self.funding_asset_amount_locked.unwrap() != bid.funding_asset_amount_locked
		{
			return false
		}
		if self.multiplier.is_some() && self.multiplier.unwrap() != bid.multiplier {
			return false
		}
		if self.plmc_bond.is_some() && self.plmc_bond.unwrap() != bid.plmc_bond {
			return false
		}
		if self.funded.is_some() && self.funded.unwrap() != bid.funded {
			return false
		}
		if self.plmc_vesting_info.is_some() && self.plmc_vesting_info.unwrap() != bid.plmc_vesting_info {
			return false
		}
		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false
		}
		if self.funds_released.is_some() && self.funds_released.unwrap() != bid.funds_released {
			return false
		}

		return true
	}
}

const ISSUER: AccountId = 10;
const EVALUATOR_1: AccountId = 20;
const EVALUATOR_2: AccountId = 21;
const EVALUATOR_3: AccountId = 22;
const BIDDER_1: AccountId = 30;
const BIDDER_2: AccountId = 31;
const BIDDER_3: AccountId = 32;
const BIDDER_4: AccountId = 33;
const BIDDER_5: AccountId = 34;
const BIDDER_6: AccountId = 35;
const BIDDER_7: AccountId = 36;
const BIDDER_8: AccountId = 37;
const BUYER_1: AccountId = 40;
const BUYER_2: AccountId = 41;
const BUYER_3: AccountId = 42;
const BUYER_4: AccountId = 43;
const BUYER_5: AccountId = 44;
const BUYER_6: AccountId = 45;
const BUYER_7: AccountId = 46;

const ASSET_DECIMALS: u8 = 10;
const ASSET_UNIT: u128 = 10_u128.pow(ASSET_DECIMALS as u32);

const USDT_STATEMINT_ID: AssetId = 1984u32;
const USDT_UNIT: u128 = 10_000_000_000_u128;

pub const US_DOLLAR: u128 = 1_0_000_000_000;

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
	fn get_update_pair(&self) -> (BlockNumber, UpdateType) {
		self.in_ext(|| {
			ProjectsToUpdate::<TestRuntime>::iter()
				.find_map(|(block, update_vec)| {
					update_vec
						.iter()
						.find(|(project_id, _update)| *project_id == self.get_project_id())
						.map(|(_project_id, update)| (block, update.clone()))
				})
				.unwrap()
		})
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

	fn get_new_nonce(&self) -> u64 {
		let nonce = self.nonce.borrow_mut().clone();
		self.nonce.replace(nonce + 1);
		nonce
	}

	fn create_project(
		&self,
		issuer: AccountId,
		project: ProjectMetadataOf<TestRuntime>,
	) -> Result<CreatedProject, DispatchError> {
		// Create project in the externalities environment of this struct instance
		self.ext_env.borrow_mut().execute_with(|| FundingModule::create(RuntimeOrigin::signed(issuer), project))?;

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

		Ok(CreatedProject { test_env: self, issuer, project_id })
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
	fn get_all_reserved_plmc_balances(&self, reserve_type: LockType<ProjectIdOf<TestRuntime>>) -> UserToPLMCBalance {
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

	fn get_reserved_plmc_balances_for(
		&self,
		user_keys: Vec<AccountId>,
		lock_type: LockType<ProjectIdOf<TestRuntime>>,
	) -> UserToPLMCBalance {
		self.ext_env.borrow_mut().execute_with(|| {
			let mut balances = UserToPLMCBalance::new();
			for user in user_keys {
				let funding = Balances::balance_on_hold(&lock_type, &user);
				balances.push((user, funding));
			}
			balances.sort_by(|a, b| a.0.cmp(&b.0));
			balances
		})
	}

	fn get_free_statemint_asset_balances_for(
		&self,
		asset_id: AssetId,
		user_keys: Vec<AccountId>,
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
		&self,
		correct_funds: UserToPLMCBalance,
		reserve_type: LockType<ProjectIdOf<TestRuntime>>,
	) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let reserved = Balances::balance_on_hold(&reserve_type, &user);
				// assert_eq!(reserved, balance);
			});
		}
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

	fn advance_time(&self, amount: BlockNumber) -> Result<(), DispatchError> {
		self.ext_env.borrow_mut().execute_with(|| {
			for _block in 0..amount {
				<AllPalletsWithoutSystem as OnFinalize<u64>>::on_finalize(System::block_number());
				<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(System::block_number(), Weight::MAX);
				System::set_block_number(System::block_number() + 1);
				let pre_events = System::events();
				<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
				let post_events = System::events();
				if post_events.len() > pre_events.len() {
					err_if_on_initialize_failed(System::events())?;
				}
			}
			Ok(())
		})
	}

	fn do_free_plmc_assertions(&self, correct_funds: UserToPLMCBalance) {
		for (user, balance) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				let free = Balances::free_balance(user);
				// assert_eq!(free, balance);
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
				// assert_eq!(expected_amount, real_amount, "Wrong statemint asset balance expected for user {}", user);
			});
		}
	}

	fn do_bid_transferred_statemint_asset_assertions(
		&self,
		correct_funds: UserToStatemintAsset,
		project_id: ProjectIdOf<TestRuntime>,
	) {
		for (user, expected_amount, _token_id) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let contribution_total: <TestRuntime as Config>::Balance =
					Bids::<TestRuntime>::iter_prefix_values((project_id, user.clone()))
						.map(|c| c.funding_asset_amount_locked)
						.sum();
				// assert_eq!(
				// 	contribution_total, expected_amount,
				// 	"Wrong statemint asset balance expected for stored auction info on user {}",
				// 	user
				// );
			});
		}
	}

	// Check if a Contribution storage item exists for the given funding asset transfer
	fn do_contribution_transferred_statemint_asset_assertions(
		&self,
		correct_funds: UserToStatemintAsset,
		project_id: ProjectIdOf<TestRuntime>,
	) {
		for (user, expected_amount, _token_id) in correct_funds {
			self.ext_env.borrow_mut().execute_with(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, user.clone()))
					.find(|c| c.funding_asset_amount == expected_amount)
					.expect("Contribution not found in storage");
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
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
	) -> Self {
		let now = test_env.current_block();
		test_env.mint_plmc_to(vec![(issuer, get_ed())]);
		let created_project = test_env.create_project(issuer, project_metadata.clone()).unwrap();
		created_project.creation_assertions(project_metadata, now);
		created_project
	}

	fn creation_assertions(
		&self,
		expected_metadata: ProjectMetadataOf<TestRuntime>,
		creation_start_block: BlockNumberOf<TestRuntime>,
	) {
		let metadata = self.get_project_metadata();
		let details = self.get_project_details();
		let expected_details = ProjectDetailsOf::<TestRuntime> {
			issuer: self.get_issuer(),
			is_frozen: false,
			weighted_average_price: None,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair { start: Some(creation_start_block), end: None },
				..Default::default()
			},
			fundraising_target: expected_metadata
				.minimum_price
				.checked_mul_int(expected_metadata.total_allocation_size)
				.unwrap(),
			remaining_contribution_tokens: expected_metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<TestRuntime>::zero(),
			cleanup: Cleaner::NotReady,
			evaluation_round_info: EvaluationRoundInfoOf::<TestRuntime> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	// Move to next project phase
	fn start_evaluation(self, caller: AccountId) -> Result<EvaluatingProject<'a>, DispatchError> {
		assert_eq!(self.get_project_details().status, ProjectStatus::Application);
		self.in_ext(|| FundingModule::start_evaluation(RuntimeOrigin::signed(caller), self.project_id))?;
		assert_eq!(self.get_project_details().status, ProjectStatus::EvaluationRound);

		Ok(EvaluatingProject { test_env: self.test_env, issuer: self.issuer, project_id: self.project_id })
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
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
	) -> Self {
		let created_project = CreatedProject::new_with(test_env, project_metadata.clone(), issuer);
		let creator = created_project.get_issuer();

		let evaluating_project = created_project.start_evaluation(creator).unwrap();

		evaluating_project
	}

	pub fn evaluation_assertions(
		&self,
		expected_free_plmc_balances: UserToPLMCBalance,
		expected_reserved_plmc_balances: UserToPLMCBalance,
		total_plmc_supply: BalanceOf<TestRuntime>,
	) {
		let project_details = self.get_project_details();
		let test_env = self.test_env;
		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env
			.do_reserved_plmc_assertions(expected_reserved_plmc_balances, LockType::Evaluation(self.get_project_id()));
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
			self.test_env.advance_time(blocks_to_start).unwrap();
		};

		assert_eq!(self.get_project_details().status, ProjectStatus::AuctionInitializePeriod);

		self.in_ext(|| FundingModule::start_auction(RuntimeOrigin::signed(caller), self.get_project_id()))?;

		assert_eq!(self.get_project_details().status, ProjectStatus::AuctionRound(AuctionPhase::English));

		Ok(AuctioningProject { test_env: self.test_env, issuer: self.issuer, project_id: self.project_id })
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
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
		evaluations: UserToUSDBalance,
	) -> Self {
		let evaluating_project = EvaluatingProject::new_with(test_env, project_metadata, issuer);

		let evaluators = evaluations.iter().map(|(acc, _val)| acc.clone()).collect::<Vec<AccountIdOf<TestRuntime>>>();
		let prev_supply = test_env.get_plmc_total_supply();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(evaluators);

		let plmc_eval_deposits: UserToPLMCBalance = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: UserToPLMCBalance =
			evaluations.iter().map(|(account, _amount)| (account.clone(), get_ed())).collect::<_>();

		let expected_remaining_plmc: UserToPLMCBalance =
			merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

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

		self.test_env.advance_time(candle_start.saturating_sub(self.test_env.current_block())).unwrap();
		let candle_end = self
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");

		let community_start = candle_end + 2;

		self.test_env.advance_time(community_start.saturating_sub(self.test_env.current_block())).unwrap();

		assert_eq!(self.get_project_details().status, ProjectStatus::CommunityRound);

		CommunityFundingProject { test_env: self.test_env, issuer: self.issuer, project_id: self.project_id }
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
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: <TestRuntime as frame_system::Config>::AccountId,
		evaluations: UserToUSDBalance,
		bids: TestBids,
	) -> Self {
		let auctioning_project = AuctioningProject::new_with(test_env, project_metadata, issuer, evaluations.clone());

		let project_id = auctioning_project.get_project_id();

		if bids.is_empty() {
			panic!("Cannot start community funding without bids")
		}

		let bidders = bids.iter().map(|b| b.bidder.clone()).collect::<Vec<AccountIdOf<TestRuntime>>>();
		let asset_id = bids[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(bidders.clone());
		let prev_funding_asset_balances = test_env.get_free_statemint_asset_balances_for(asset_id, bidders);
		let plmc_evaluation_deposits: UserToPLMCBalance = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits: UserToPLMCBalance = calculate_auction_plmc_spent(bids.clone());
		let participation_usable_evaluation_deposits = plmc_evaluation_deposits
			.clone()
			.into_iter()
			.map(|(acc, amount)| (acc, amount.saturating_sub(<TestRuntime as Config>::EvaluatorSlash::get() * amount)))
			.collect::<UserToPLMCBalance>();
		let necessary_plmc_mint =
			merge_subtract_mappings_by_user(plmc_bid_deposits.clone(), vec![participation_usable_evaluation_deposits]);
		let total_plmc_participation_locked = plmc_bid_deposits;
		let plmc_existential_deposits: UserToPLMCBalance = bids.iter().map(|bid| (bid.bidder, get_ed())).collect::<_>();
		let funding_asset_deposits = calculate_auction_funding_asset_spent(bids.clone());

		let bidder_balances =
			sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		let bid_expectations = bids
			.iter()
			.map(|bid| BidInfoFilter {
				original_ct_amount: Some(bid.amount),
				original_ct_usd_price: Some(bid.price),
				..Default::default()
			})
			.collect::<Vec<_>>();
		let total_ct_sold = bids.iter().map(|bid| bid.amount).sum::<u128>();

		test_env.mint_plmc_to(necessary_plmc_mint.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		auctioning_project.bid_for_users(bids).expect("Bidding should work");

		test_env.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		test_env.do_bid_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		let community_project = auctioning_project.start_community_funding();

		community_project.finalized_bids_assertions(bid_expectations, total_ct_sold);

		community_project
	}

	fn buy_for_retail_users(&self, contributions: TestContributions) -> DispatchResultWithPostInfo {
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
		Ok(().into())
	}

	fn finalized_bids_assertions(
		&self,
		bid_expectations: Vec<BidInfoFilterOf<TestRuntime>>,
		expected_ct_sold: BalanceOf<TestRuntime>,
	) {
		// let project_metadata = self.get_project_metadata();
		// let project_details = self.get_project_details();
		// let project_id = self.get_project_id();
		// let project_bids = self.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		// dbg!(project_bids.clone());
		// dbg!(bid_expectations.clone());
		// assert!(matches!(project_details.weighted_average_price, Some(_)), "Weighted average price should exist");

		// for filter in bid_expectations {
		// 	let _found_bid = project_bids.iter().find(|bid| filter.matches_bid(&bid)).unwrap();
		// }

		// // Remaining CTs are updated
		// assert_eq!(
		// 	project_details.remaining_contribution_tokens,
		// 	project_metadata.total_allocation_size - expected_ct_sold,
		// 	"Remaining CTs are incorrect"
		// );
	}

	fn start_remainder_or_end_funding(self) -> Either<RemainderFundingProject<'a>, FinishedProject<'a>> {
		assert_eq!(self.get_project_details().status, ProjectStatus::CommunityRound);
		let community_funding_end = self
			.get_project_details()
			.phase_transition_points
			.community
			.end()
			.expect("Community funding end point should exist");
		let remainder_start = community_funding_end + 1;
		self.test_env.advance_time(remainder_start.saturating_sub(self.test_env.current_block())).unwrap();
		match self.get_project_details().status {
			ProjectStatus::RemainderRound => Either::Left(RemainderFundingProject {
				test_env: self.test_env,
				issuer: self.issuer,
				project_id: self.project_id,
			}),
			ProjectStatus::FundingSuccessful => Either::Right(FinishedProject {
				test_env: self.test_env,
				issuer: self.issuer,
				project_id: self.project_id,
			}),
			_ => panic!("Unknown state"),
		}
	}

	fn finish_funding(self) -> FinishedProject<'a> {
		let test_env = self.get_test_environment();
		let (update_block, _) = self.get_update_pair();
		test_env.advance_time(update_block.saturating_sub(test_env.current_block())).unwrap();
		if self.get_project_details().status == ProjectStatus::RemainderRound {
			let (end_block, _) = self.get_update_pair();
			self.test_env.advance_time(end_block.saturating_sub(self.test_env.current_block())).unwrap();
		}
		let project_details = self.get_project_details();
		assert!(
			matches!(project_details.status, ProjectStatus::FundingSuccessful) ||
				matches!(project_details.status, ProjectStatus::FundingFailed),
			"Project should be in Finished status"
		);
		FinishedProject { test_env: self.test_env, issuer: self.issuer, project_id: self.project_id }
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
	fn buy_for_any_user(&self, contributions: TestContributions) -> DispatchResultWithPostInfo {
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
		Ok(().into())
	}

	fn new_with(
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: AccountId,
		evaluations: UserToUSDBalance,
		bids: TestBids,
		contributions: TestContributions,
	) -> Either<Self, FinishedProject> {
		let community_funding_project =
			CommunityFundingProject::new_with(test_env, project_metadata, issuer, evaluations.clone(), bids.clone());

		if contributions.is_empty() {
			return community_funding_project.start_remainder_or_end_funding()
		}
		let project_id = community_funding_project.get_project_id();
		let ct_price = community_funding_project.get_project_details().weighted_average_price.unwrap();
		let contributors = contributions.iter().map(|cont| cont.contributor).collect::<Vec<_>>();
		let asset_id = contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances =
			test_env.get_free_statemint_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits = calculate_auction_plmc_spent_after_price_calculation(bids.clone(), ct_price);
		let plmc_contribution_deposits = calculate_contributed_plmc_spent(contributions.clone(), ct_price);

		let necessary_plmc_mint =
			merge_subtract_mappings_by_user(plmc_contribution_deposits.clone(), vec![plmc_evaluation_deposits]);
		let total_plmc_participation_locked =
			merge_add_mappings_by_user(vec![plmc_bid_deposits, plmc_contribution_deposits.clone()]);
		let plmc_existential_deposits: UserToPLMCBalance =
			contributors.iter().map(|acc| (acc.clone(), get_ed())).collect::<Vec<_>>();

		let funding_asset_deposits = calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);
		let contributor_balances =
			sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		test_env.mint_plmc_to(necessary_plmc_mint.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		community_funding_project.buy_for_retail_users(contributions.clone()).expect("Contributing should work");

		test_env.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		community_funding_project.start_remainder_or_end_funding()
	}

	fn end_funding(&self) -> FinishedProject<'a> {
		assert_eq!(self.get_project_details().status, ProjectStatus::RemainderRound);
		let remainder_funding_end =
			self.get_project_details().phase_transition_points.remainder.end().expect("Should have remainder end");
		let finish_block = remainder_funding_end + 1;
		self.test_env.advance_time(finish_block.saturating_sub(self.test_env.current_block())).unwrap();
		assert!(matches!(
			self.get_project_details().status,
			ProjectStatus::FundingSuccessful | ProjectStatus::FundingFailed | ProjectStatus::AwaitingProjectDecision
		));

		FinishedProject { test_env: self.test_env, issuer: self.issuer.clone(), project_id: self.project_id.clone() }
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
		test_env: &'a TestEnvironment,
		project_metadata: ProjectMetadataOf<TestRuntime>,
		issuer: AccountId,
		evaluations: UserToUSDBalance,
		bids: TestBids,
		community_contributions: TestContributions,
		remainder_contributions: TestContributions,
	) -> Self {
		let project = RemainderFundingProject::new_with(
			test_env,
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
		);

		let remainder_funding_project = match project {
			Either::Right(finished_project) => return finished_project,
			Either::Left(remainder_project) if remainder_contributions.is_empty() =>
				return remainder_project.end_funding(),
			Either::Left(remainder_project) => remainder_project,
		};

		let project_id = remainder_funding_project.get_project_id();
		let ct_price = remainder_funding_project.get_project_details().weighted_average_price.unwrap();
		let contributors = remainder_contributions.iter().map(|cont| cont.contributor).collect::<Vec<_>>();
		let asset_id = remainder_contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = test_env.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances =
			test_env.get_free_statemint_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits = calculate_auction_plmc_spent_after_price_calculation(bids.clone(), ct_price);
		let plmc_community_contribution_deposits =
			calculate_contributed_plmc_spent(community_contributions.clone(), ct_price);
		let plmc_remainder_contribution_deposits =
			calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);

		let necessary_plmc_mint = merge_subtract_mappings_by_user(
			plmc_remainder_contribution_deposits.clone(),
			vec![plmc_evaluation_deposits],
		);
		let total_plmc_participation_locked = merge_add_mappings_by_user(vec![
			plmc_bid_deposits,
			plmc_community_contribution_deposits,
			plmc_remainder_contribution_deposits.clone(),
		]);
		let plmc_existential_deposits: UserToPLMCBalance =
			contributors.iter().map(|acc| (acc.clone(), get_ed())).collect::<Vec<_>>();
		let funding_asset_deposits =
			calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

		let contributor_balances =
			sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = test_env.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		test_env.mint_plmc_to(necessary_plmc_mint.clone());
		test_env.mint_plmc_to(plmc_existential_deposits.clone());
		test_env.mint_statemint_asset_to(funding_asset_deposits.clone());

		remainder_funding_project
			.buy_for_any_user(remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		test_env.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		test_env.do_free_plmc_assertions(expected_free_plmc_balances);
		test_env.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		test_env.do_total_plmc_assertions(post_supply);

		let finished_project = remainder_funding_project.end_funding();

		if finished_project.get_project_details().status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let project_details = finished_project.get_project_details();
			let auction_bought_tokens: u128 = bids.iter().map(|bid| bid.amount).sum();
			let community_bought_tokens: u128 = community_contributions.iter().map(|cont| cont.amount).sum();
			let remainder_bought_tokens: u128 = remainder_contributions.iter().map(|cont| cont.amount).sum();

			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size -
					auction_bought_tokens -
					community_bought_tokens -
					remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
		}

		finished_project
	}

	fn from_funding_reached(test_env: &'a TestEnvironment, percent: u64) -> Self {
		let project_metadata = default_project(test_env.get_new_nonce());
		let min_price = project_metadata.minimum_price;
		let usd_to_reach = Perquintill::from_percent(percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(Percent::from_percent(50u8) * usd_to_reach, min_price);
		let contributions =
			generate_contributions_from_total_usd(Percent::from_percent(50u8) * usd_to_reach, min_price);
		FinishedProject::new_with(test_env, project_metadata, ISSUER, evaluations, bids, contributions, vec![])
	}
}

mod defaults {
	use super::*;

	pub fn default_project(nonce: u64) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		ProjectMetadata {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000_0_000_000_000,
			total_allocation_size: 1_000_000_0_000_000_00,
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: AcceptedFundingAsset::USDT,
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
		}
	}

	pub fn excel_project(nonce: u64) -> ProjectMetadataOf<TestRuntime> {
		let bounded_name = BoundedVec::try_from("Polimec".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("PLMC".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
		ProjectMetadata {
			token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: 10 },
			mainnet_token_max_supply: 1_000_000_0_000_000_000, // Made up, not in the Sheet.
			// Total Allocation of Contribution Tokens Available for the Funding Round
			total_allocation_size: 50_000_0_000_000_000,
			// Minimum Price per Contribution Token (in USDT)
			minimum_price: PriceOf::<TestRuntime>::from(10),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 1,
			participation_currencies: AcceptedFundingAsset::USDT,
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
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

	pub fn default_evaluations() -> UserToUSDBalance {
		vec![(EVALUATOR_1, 50_000 * PLMC), (EVALUATOR_2, 25_000 * PLMC), (EVALUATOR_3, 32_000 * PLMC)]
	}

	pub fn default_failing_evaluations() -> UserToPLMCBalance {
		vec![(EVALUATOR_1, 10_000 * PLMC), (EVALUATOR_2, 5_000 * PLMC)]
	}

	pub fn default_bids() -> TestBids {
		// This should reflect the bidding currency, which currently is USDT
		vec![
			TestBid::new(BIDDER_1, 50000 * ASSET_UNIT, 18_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40000 * ASSET_UNIT, 15_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_community_buys() -> TestContributions {
		vec![
			TestContribution::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}

	pub fn default_remainder_buys() -> TestContributions {
		vec![
			TestContribution::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		]
	}
}

pub mod helper_functions {
	use super::*;

	pub fn get_ed() -> BalanceOf<TestRuntime> {
		<TestRuntime as pallet_balances::Config>::ExistentialDeposit::get()
	}

	pub fn calculate_evaluation_plmc_spent(evals: UserToUSDBalance) -> UserToPLMCBalance {
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
			let multiplier = bid.multiplier;
			let usd_bond = multiplier.calculate_bonding_requirement::<TestRuntime>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push((bid.bidder, plmc_bond));
		}
		output
	}

	// This differs from `calculate_auction_plmc_spent` in that it recalculates bids over the average price as using that price.
	pub fn calculate_auction_plmc_spent_after_price_calculation(
		bids: TestBids,
		price: PriceOf<TestRuntime>,
	) -> UserToPLMCBalance {
		let plmc_price = PriceMap::get().get(&PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = UserToPLMCBalance::new();
		for bid in bids {
			let final_price = if bid.price < price { bid.price } else { price };

			let usd_ticket_size = final_price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<TestRuntime>(usd_ticket_size).unwrap();
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
		contributions: TestContributions,
		token_usd_price: PriceOf<TestRuntime>,
	) -> UserToPLMCBalance {
		let plmc_price = PriceMap::get().get(&PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = UserToPLMCBalance::new();
		for cont in contributions {
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let usd_bond = cont.multiplier.calculate_bonding_requirement::<TestRuntime>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push((cont.contributor, plmc_bond));
		}
		output
	}

	pub fn calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
		evaluations: UserToUSDBalance,
		contributions: TestContributions,
		price: PriceOf<TestRuntime>,
		slashed: bool,
	) -> UserToPLMCBalance {
		let evaluation_locked_plmc_amounts = calculate_evaluation_plmc_spent(evaluations);
		// how much new plmc would be locked without considering evaluation bonds
		let theoretical_contribution_locked_plmc_amounts = calculate_contributed_plmc_spent(contributions, price);

		let slash_percentage = <TestRuntime as Config>::EvaluatorSlash::get();
		let slashable_min_deposits = evaluation_locked_plmc_amounts
			.clone()
			.iter()
			.map(|(acc, val)| (acc.clone(), slash_percentage * val.clone()))
			.collect::<Vec<_>>();
		let available_evaluation_locked_plmc_for_lock_transfer = merge_subtract_mappings_by_user(
			evaluation_locked_plmc_amounts.clone(),
			vec![slashable_min_deposits.clone()],
		);

		// how much new plmc was actually locked, considering already evaluation bonds used first.
		let actual_contribution_locked_plmc_amounts = generic_map_merge(
			vec![
				theoretical_contribution_locked_plmc_amounts.clone(),
				available_evaluation_locked_plmc_for_lock_transfer,
			],
			|(acc, _)| acc.clone(),
			|(acc_1, contribution_amount), (_acc_2, evaluation_amount)| {
				if contribution_amount > evaluation_amount {
					(acc_1.clone(), contribution_amount - evaluation_amount)
				} else {
					(acc_1.clone(), Zero::zero())
				}
			},
		);
		let mut result =
			merge_add_mappings_by_user(vec![evaluation_locked_plmc_amounts, actual_contribution_locked_plmc_amounts]);

		if slashed {
			result = merge_subtract_mappings_by_user(result, vec![slashable_min_deposits]);
		}

		result
	}

	pub fn calculate_contributed_funding_asset_spent(
		contributions: TestContributions,
		token_usd_price: PriceOf<TestRuntime>,
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

	/// add all the user -> I maps together, and add the I's of the ones with the same user.
	// Mappings should be sorted based on their account id, ascending.
	pub fn merge_add_mappings_by_user<I: std::ops::Add<Output = I> + Ord + Copy>(
		mut mappings: Vec<Vec<(AccountIdOf<TestRuntime>, I)>>,
	) -> Vec<(AccountIdOf<TestRuntime>, I)> {
		let mut output = mappings.swap_remove(0);
		output.sort_by_key(|k| k.0);
		for mut map in mappings {
			map.sort_by_key(|k| k.0);
			let old_output = output.clone();
			output = Vec::new();
			let mut i = 0;
			let mut j = 0;
			loop {
				let old_tup = old_output.get(i);
				let new_tup = map.get(j);

				match (old_tup, new_tup) {
					(None, None) => break,
					(Some(_), None) => {
						output.extend_from_slice(&old_output[i..]);
						break
					},
					(None, Some(_)) => {
						output.extend_from_slice(&map[j..]);
						break
					},
					(Some((acc_i, val_i)), Some((acc_j, val_j))) =>
						if acc_i == acc_j {
							output.push((acc_i.clone(), val_i.clone() + val_j.clone()));
							i += 1;
							j += 1;
						} else if acc_i < acc_j {
							output.push(old_output[i]);
							i += 1;
						} else {
							output.push(map[j]);
							j += 1;
						},
				}
			}
		}
		output
	}

	pub fn generic_map_merge_reduce<M: Clone, K: Ord + Clone, S: Clone>(
		mappings: Vec<Vec<M>>,
		key_extractor: impl Fn(&M) -> K,
		initial_state: S,
		merge_reduce: impl Fn(&M, S) -> S,
	) -> Vec<(K, S)> {
		let mut output = BTreeMap::new();
		for mut map in mappings {
			for item in map.drain(..) {
				let key = key_extractor(&item);
				let new_state = merge_reduce(&item, output.get(&key).cloned().unwrap_or(initial_state.clone()));
				output.insert(key, new_state);
			}
		}
		output.into_iter().collect()
	}

	pub fn generic_map_merge<M: Clone, K: Ord + Clone>(
		mut mappings: Vec<Vec<M>>,
		key_extractor: impl Fn(&M) -> K,
		merger: impl Fn(&M, &M) -> M,
	) -> Vec<M> {
		let mut output = mappings.swap_remove(0);
		output.sort_by_key(|k| key_extractor(k));
		for mut new_map in mappings {
			new_map.sort_by_key(|k| key_extractor(k));
			let old_output = output.clone();
			output = Vec::new();
			let mut i = 0;
			let mut j = 0;
			loop {
				let output_item = old_output.get(i);
				let new_item = new_map.get(j);

				match (output_item, new_item) {
					(None, None) => break,
					(Some(_), None) => {
						output.extend_from_slice(&old_output[i..]);
						break
					},
					(None, Some(_)) => {
						output.extend_from_slice(&new_map[j..]);
						break
					},
					(Some(m_i), Some(m_j)) => {
						let k_i = key_extractor(m_i);
						let k_j = key_extractor(m_j);
						if k_i == k_j {
							output.push(merger(m_i, m_j));
							i += 1;
							j += 1;
						} else if k_i < k_j {
							output.push(old_output[i].clone());
							i += 1;
						} else {
							output.push(new_map[j].clone());
							j += 1;
						}
					},
				}
			}
		}
		output
	}

	// Accounts in base_mapping will be deducted balances from the matching accounts in substract_mappings.
	// Mappings in substract_mappings without a match in base_mapping have no effect, nor will they get returned
	pub fn merge_subtract_mappings_by_user<I: Saturating + Ord + Copy>(
		base_mapping: Vec<(AccountIdOf<TestRuntime>, I)>,
		subtract_mappings: Vec<Vec<(AccountIdOf<TestRuntime>, I)>>,
	) -> Vec<(AccountIdOf<TestRuntime>, I)> {
		let mut output = base_mapping;
		output.sort_by_key(|k| k.0);
		for mut map in subtract_mappings {
			map.sort_by_key(|k| k.0);
			let old_output = output.clone();
			output = Vec::new();
			let mut i = 0;
			let mut j = 0;
			loop {
				let old_tup = old_output.get(i);
				let new_tup = map.get(j);

				match (old_tup, new_tup) {
					(None, None) => break,
					(Some(_), None) => {
						output.extend_from_slice(&old_output[i..]);
						break
					},
					(None, Some(_)) => {
						// uncomment this if we want to keep unmatched mappings on the substractor
						// output.extend_from_slice(&map[j..]);
						break
					},
					(Some((acc_i, val_i)), Some((acc_j, val_j))) => {
						if acc_i == acc_j {
							output.push((acc_i.clone(), val_i.clone().saturating_sub(val_j.clone())));
							i += 1;
							j += 1;
						} else if acc_i < acc_j {
							output.push(old_output[i]);
							i += 1;
						} else {
							// uncomment to keep unmatched maps
							// output.push(map[j]);
							j += 1;
						}
					},
				}
			}
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

	pub fn calculate_price_from_test_bids(bids: TestBids) -> PriceOf<TestRuntime> {
		// temp variable to store the total value of the bids (i.e price * amount)
		let mut bid_usd_value_sum = BalanceOf::<TestRuntime>::zero();

		for bid in bids.iter() {
			let ticket_size = bid.price.checked_mul_int(bid.amount).unwrap();
			bid_usd_value_sum.saturating_accrue(ticket_size);
		}

		bids.into_iter()
			.map(|bid| {
				let bid_weight = <PriceOf<TestRuntime> as FixedPointNumber>::saturating_from_rational(
					bid.price.saturating_mul_int(bid.amount),
					bid_usd_value_sum,
				);
				bid.price * bid_weight
			})
			.reduce(|a, b| a.saturating_add(b))
			.unwrap()
	}

	pub fn panic_if_on_initialize_failed(events: Vec<frame_system::EventRecord<RuntimeEvent, H256>>) {
		let last_event = events.into_iter().last().expect("No events found for this action.");
		match last_event {
			frame_system::EventRecord {
				event: RuntimeEvent::FundingModule(Event::TransitionError { project_id, error }),
				..
			} => {
				panic!("Project {} transition failed in on_initialize: {:?}", project_id, error);
			},
			_ => {},
		}
	}

	pub fn err_if_on_initialize_failed(
		events: Vec<frame_system::EventRecord<RuntimeEvent, H256>>,
	) -> Result<(), Error<TestRuntime>> {
		let last_event = events.into_iter().last().expect("No events found for this action.");
		match last_event {
			frame_system::EventRecord {
				event: RuntimeEvent::FundingModule(Event::TransitionError { project_id: _, error }),
				..
			} => match error {
				DispatchError::Module(module_error) => {
					let pallet_error: Error<TestRuntime> = Decode::decode(&mut &module_error.error[..]).unwrap();
					Err(pallet_error)
				},
				_ => panic!("wrong conversion"),
			},
			_ => Ok(()),
		}
	}

	pub fn generate_bids_from_total_usd(
		usd_amount: BalanceOf<TestRuntime>,
		min_price: PriceOf<TestRuntime>,
	) -> TestBids {
		const WEIGHTS: [u8; 5] = [30u8, 20u8, 15u8, 10u8, 25u8];
		const BIDDERS: [AccountIdOf<TestRuntime>; 5] = [BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5];

		zip(WEIGHTS, BIDDERS)
			.map(|(weight, bidder)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = min_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				TestBid::new(bidder, token_amount, min_price, 1u8, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn generate_contributions_from_total_usd(
		usd_amount: BalanceOf<TestRuntime>,
		final_price: PriceOf<TestRuntime>,
	) -> TestContributions {
		const WEIGHTS: [u8; 5] = [30u8, 20u8, 15u8, 10u8, 25u8];
		const BIDDERS: [AccountIdOf<TestRuntime>; 5] = [BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5];

		zip(WEIGHTS, BIDDERS)
			.map(|(weight, bidder)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = final_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				TestContribution::new(bidder, token_amount, 1u8, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn test_ct_created_for(test_env: &TestEnvironment, project_id: ProjectIdOf<TestRuntime>) {
		test_env.in_ext(|| {
			let metadata = ProjectsMetadata::<TestRuntime>::get(project_id).unwrap();
			let details = ProjectsDetails::<TestRuntime>::get(project_id).unwrap();
			assert_eq!(
				<TestRuntime as Config>::ContributionTokenCurrency::name(project_id),
				metadata.token_information.name.to_vec()
			);
			assert_eq!(<TestRuntime as Config>::ContributionTokenCurrency::admin(project_id).unwrap(), details.issuer);
			assert_eq!(
				<TestRuntime as Config>::ContributionTokenCurrency::total_issuance(project_id),
				0u32.into(),
				"No CTs should have been minted at this point"
			);
		});
	}

	pub fn test_ct_not_created_for(test_env: &TestEnvironment, project_id: ProjectIdOf<TestRuntime>) {
		test_env.in_ext(|| {
			assert!(
				!<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id),
				"Asset shouldn't exist, since funding failed"
			);
		});
	}

	pub fn slash_evaluator_balances(mut balances: UserToPLMCBalance) -> UserToPLMCBalance {
		let slash_percentage = <TestRuntime as Config>::EvaluatorSlash::get();
		for (_acc, balance) in balances.iter_mut() {
			*balance -= slash_percentage * *balance;
		}
		balances
	}
}

mod creation_round_success {
	use super::*;

	#[test]
	fn basic_plmc_transfer_works() {
		let test_env = TestEnvironment::new();

		test_env.mint_plmc_to(default_plmc_balances());

		test_env.ext_env.borrow_mut().execute_with(|| {
			assert_ok!(Balances::transfer(RuntimeOrigin::signed(EVALUATOR_1), EVALUATOR_2, 1 * PLMC));
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

mod creation_round_failure {
	use super::*;

	#[test]
	fn price_too_low() {
		let wrong_project: ProjectMetadataOf<TestRuntime> = ProjectMetadata {
			minimum_price: 0_u128.into(),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
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
			minimum_price: 1_u128.into(),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: None, maximum: None },
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
			minimum_price: 1_u128.into(),
			ticket_size: TicketSize { minimum: None, maximum: None },
			participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
			offchain_information_hash: Some(hashed(METADATA)),
			..Default::default()
		};

		let test_env = TestEnvironment::new();
		test_env.mint_plmc_to(default_plmc_balances());

		let project_err = test_env.create_project(ISSUER, wrong_project).unwrap_err();
		assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
	}
}

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

	#[test]
	fn rewards_are_paid_full_funding() {
		let test_env = TestEnvironment::new();

		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, 420));
		let project_metadata = ProjectMetadataOf::<TestRuntime> {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000_0_000_000_000,
			total_allocation_size: 100_000_0_000_000_000,
			minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
			ticket_size: TicketSize { minimum: Some(1), maximum: None },
			participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
			funding_thresholds: Default::default(),
			conversion_rate: 0,
			participation_currencies: AcceptedFundingAsset::USDT,
			funding_destination_account: ISSUER,
			offchain_information_hash: Some(metadata_hash),
		};

		// all values taken from the knowledge hub
		let evaluations: UserToUSDBalance = vec![
			(EVALUATOR_1, 75_000 * US_DOLLAR),
			(EVALUATOR_2, 65_000 * US_DOLLAR),
			(EVALUATOR_3, 60_000 * US_DOLLAR),
		];

		let bids: TestBids = vec![
			TestBid::new(BIDDER_1, 10_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 20_000 * ASSET_UNIT, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_4, 20_000 * ASSET_UNIT, 16.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let contributions: TestContributions = vec![
			TestContribution::new(BUYER_1, 4_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_4, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_5, 30_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_6, 5_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_7, 2_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, project_metadata, ISSUER, evaluations, bids);
		let details = community_funding_project.get_project_details();
		let ct_price = details.weighted_average_price.unwrap();
		let mut plmc_deposits = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_deposits = plmc_deposits.into_iter().map(|(account, balance)| (account, balance + get_ed())).collect();
		let funding_deposits = calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		test_env.mint_plmc_to(plmc_deposits);
		test_env.mint_statemint_asset_to(funding_deposits);

		community_funding_project.buy_for_retail_users(contributions).unwrap();
		let finished_project = community_funding_project.finish_funding();
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10).unwrap();
		let project_id = finished_project.project_id;
		let actual_reward_balances = test_env.in_ext(|| {
			vec![
				(EVALUATOR_1, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_1)),
				(EVALUATOR_2, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_2)),
				(EVALUATOR_3, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_3)),
			]
		});
		let expected_ct_rewards = vec![
			(EVALUATOR_1, 1_196_1_509_434_007),
			(EVALUATOR_2, 824_0_150_943_427),
			(EVALUATOR_3, 637_9_471_698_137),
		];

		for (real, desired) in zip(actual_reward_balances.iter(), expected_ct_rewards.iter()) {
			assert_eq!(real.0, desired.0, "bad accounts order");
			// 0.01 parts of a Perbill
			assert_close_enough!(real.1, desired.1, Perquintill::from_parts(10_000_000u64));
		}
	}

	#[test]
	fn plmc_unbonded_after_funding_success() {
		let test_env = TestEnvironment::new();
		let evaluations = default_evaluations();
		let evaluators = evaluations.iter().map(|ev| ev.0.clone()).collect::<Vec<_>>();

		let remainder_funding_project = RemainderFundingProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			evaluations.clone(),
			default_bids(),
			default_community_buys(),
		)
		.unwrap_left();
		let project_id = remainder_funding_project.get_project_id();
		let prev_reserved_plmc =
			test_env.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Evaluation(project_id));

		let prev_free_plmc = test_env.get_free_plmc_balances_for(evaluators.clone());

		remainder_funding_project.end_funding();
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10).unwrap();
		let post_unbond_amounts: UserToPLMCBalance =
			prev_reserved_plmc.iter().map(|(evaluator, _amount)| (*evaluator, Zero::zero())).collect();

		test_env.do_reserved_plmc_assertions(post_unbond_amounts.clone(), LockType::Evaluation(project_id));
		test_env.do_reserved_plmc_assertions(post_unbond_amounts, LockType::Participation(project_id));

		let post_free_plmc = test_env.get_free_plmc_balances_for(evaluators.clone());

		let increased_amounts = merge_subtract_mappings_by_user(post_free_plmc, vec![prev_free_plmc]);

		assert_eq!(increased_amounts, calculate_evaluation_plmc_spent(evaluations))
	}

	#[test]
	fn plmc_unbonded_after_funding_failure() {
		let test_env = TestEnvironment::new();
		let evaluations = default_evaluations();
		let evaluators = evaluations.iter().map(|ev| ev.0.clone()).collect::<Vec<_>>();

		let remainder_funding_project = RemainderFundingProject::new_with(
			&test_env,
			default_project(test_env.get_new_nonce()),
			ISSUER,
			evaluations.clone(),
			vec![TestBid::new(BUYER_1, 1000 * ASSET_UNIT, 10u128.into(), 1u8, AcceptedFundingAsset::USDT)],
			vec![TestContribution::new(BUYER_1, 1000 * US_DOLLAR, 1u8, AcceptedFundingAsset::USDT)],
		)
		.unwrap_left();

		let project_id = remainder_funding_project.get_project_id();
		let prev_reserved_plmc =
			test_env.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Evaluation(project_id));
		let prev_free_plmc = test_env.get_free_plmc_balances_for(evaluators.clone());

		let finished_project = remainder_funding_project.end_funding();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingFailed);
		test_env.advance_time(10).unwrap();

		let post_unbond_amounts: UserToPLMCBalance =
			prev_reserved_plmc.iter().map(|(evaluator, _amount)| (*evaluator, Zero::zero())).collect();

		test_env.do_reserved_plmc_assertions(post_unbond_amounts.clone(), LockType::Evaluation(project_id));
		test_env.do_reserved_plmc_assertions(post_unbond_amounts, LockType::Participation(project_id));

		let post_free_plmc = test_env.get_free_plmc_balances_for(evaluators.clone());

		let increased_amounts = merge_subtract_mappings_by_user(post_free_plmc, vec![prev_free_plmc]);

		assert_eq!(increased_amounts, slash_evaluator_balances(calculate_evaluation_plmc_spent(evaluations)))
	}
}

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
		let plmc_existential_deposits: UserToPLMCBalance =
			evaluations.iter().map(|(account, _amount)| (account.clone(), get_ed())).collect::<_>();
		let expected_evaluator_balances =
			merge_add_mappings_by_user(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

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

		evaluating_project.bond_for_users(default_failing_evaluations()).expect("Bonding should work");

		test_env.do_free_plmc_assertions(plmc_existential_deposits);
		test_env.do_reserved_plmc_assertions(plmc_eval_deposits, LockType::Evaluation(project_id));

		test_env.advance_time(evaluation_end - now + 1).unwrap();

		assert_eq!(evaluating_project.get_project_details().status, ProjectStatus::EvaluationFailed);

		// Check that on_idle has unlocked the failed bonds
		test_env.advance_time(10).unwrap();
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

		let plmc_existential_deposits: UserToPLMCBalance =
			evaluations.iter().map(|(account, _amount)| (account.clone(), get_ed())).collect::<_>();

		test_env.mint_plmc_to(insufficient_eval_deposits.clone());
		test_env.mint_plmc_to(plmc_existential_deposits);

		let evaluating_project = EvaluatingProject::new_with(&test_env, project, issuer);

		let dispatch_error = evaluating_project.bond_for_users(evaluations);
		assert_err!(dispatch_error, TokenError::FundsUnavailable)
	}
}

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
	fn evaluation_bond_counts_towards_bid() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let evaluator_bidder = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let evaluator_bid =
			TestBid::new(evaluator_bidder, 600 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_bidder, evaluation_amount));

		let bidding_project = AuctioningProject::new_with(&test_env, project, issuer, evaluations);

		let already_bonded_plmc = calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount)])[0].1;
		let usable_evaluation_plmc =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_bid = calculate_auction_plmc_spent(vec![evaluator_bid])[0].1;
		let necessary_usdt_for_bid = calculate_auction_funding_asset_spent(vec![evaluator_bid]);

		test_env.mint_plmc_to(vec![(evaluator_bidder, necessary_plmc_for_bid - usable_evaluation_plmc)]);
		test_env.mint_statemint_asset_to(necessary_usdt_for_bid);

		bidding_project.bid_for_users(vec![evaluator_bid]).unwrap();
	}

	#[test]
	fn evaluation_bond_counts_towards_bid_vec_full() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let evaluator_bidder = 69;
		let evaluator_bid =
			TestBid::new(evaluator_bidder, 600 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT);

		let mut bids = Vec::new();
		for _ in 0..<TestRuntime as Config>::MaxBidsPerUser::get() {
			bids.push(TestBid::new(evaluator_bidder, 10 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT));
		}

		let fill_necessary_plmc_for_bids = calculate_auction_plmc_spent(bids.clone());
		let fill_necessary_usdt_for_bids = calculate_auction_funding_asset_spent(bids.clone());

		let bid_necessary_plmc = calculate_auction_plmc_spent(vec![evaluator_bid]);
		let bid_necessary_usdt = calculate_auction_funding_asset_spent(vec![evaluator_bid]);

		let evaluation_bond = sum_balance_mappings(vec![fill_necessary_plmc_for_bids, bid_necessary_plmc.clone()]);
		let plmc_available_for_participation =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push((evaluator_bidder, evaluation_usd_amount));

		let bidding_project = AuctioningProject::new_with(&test_env, project, issuer, evaluations);
		let project_id = bidding_project.get_project_id();

		test_env.mint_plmc_to(vec![(evaluator_bidder, evaluation_bond - plmc_available_for_participation)]);
		test_env.mint_statemint_asset_to(fill_necessary_usdt_for_bids);
		test_env.mint_statemint_asset_to(bid_necessary_usdt);

		bidding_project.bid_for_users(bids).unwrap();
		bidding_project.bid_for_users(vec![evaluator_bid]).unwrap();

		let evaluation_bonded = test_env.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_bidder,
			)
		});
		assert_eq!(evaluation_bonded, <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond);
	}

	#[test]
	fn price_calculation_1() {
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
			TestBid::new(100, 10_000_0_000_000_000, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(200, 20_000_0_000_000_000, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(300, 20_000_0_000_000_000, 10.into(), 1u8, AcceptedFundingAsset::USDT),
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
		let token_price = community_funding_project.get_project_details().weighted_average_price.unwrap();

		let price_in_10_decimals = token_price.checked_mul_int(1_0_000_000_000_u128).unwrap();
		let price_in_12_decimals = token_price.checked_mul_int(1_000_000_000_000_u128).unwrap();
		assert_eq!(price_in_10_decimals, 16_3_333_333_333_u128);
		assert_eq!(price_in_12_decimals, 16_333_333_333_333_u128);
	}

	#[test]
	fn price_calculation_2() {
		// From the knowledge hub
		let test_env = TestEnvironment::new();
		let project_metadata = default_project(test_env.get_new_nonce());
		let auctioning_project =
			AuctioningProject::new_with(&test_env, project_metadata, ISSUER, default_evaluations());
		let bids = vec![
			TestBid::new(BIDDER_1, 10_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 20_000 * ASSET_UNIT, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_3, 20_000 * ASSET_UNIT, 16.into(), 1u8, AcceptedFundingAsset::USDT),
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
		let token_price = community_funding_project.get_project_details().weighted_average_price.unwrap();

		let price_in_10_decimals = token_price.checked_mul_int(1_0_000_000_000_u128).unwrap();
		assert_eq!(price_in_10_decimals, 17_6_666_666_666);
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
		test_env.advance_time(english_end_block - test_env.current_block() + 1).unwrap();
		assert_eq!(auctioning_project.get_project_details().status, ProjectStatus::AuctionRound(AuctionPhase::Candle));

		let candle_end_block = auctioning_project
			.get_project_details()
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle auction end point should exist");

		let mut bidding_account = 1000;
		let bid_info =
			TestBid::new(0, 50u128, PriceOf::<TestRuntime>::from_float(15f64), 1u8, AcceptedFundingAsset::USDT);
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
			let bids: TestBids = vec![TestBid {
				bidder: bidding_account,
				amount: bid_info.amount,
				price: bid_info.price,
				multiplier: bid_info.multiplier,
				asset: bid_info.asset,
			}];
			auctioning_project.bid_for_users(bids.clone()).expect("Candle Bidding should not fail");

			bids_made.push(bids[0]);
			bidding_account += 1;

			test_env.advance_time(1).unwrap();
		}
		test_env.advance_time(candle_end_block - test_env.current_block() + 1).unwrap();

		let random_end = auctioning_project
			.get_project_details()
			.phase_transition_points
			.random_candle_ending
			.expect("Random auction end point should exist");

		let split = (random_end - starting_bid_block + 1) as usize;
		let excluded_bids = bids_made.split_off(split);
		let included_bids = bids_made;
		let _weighted_price =
			auctioning_project.get_project_details().weighted_average_price.expect("Weighted price should exist");

		for bid in included_bids {
			let pid = auctioning_project.get_project_id();
			let mut stored_bids =
				auctioning_project.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((pid, bid.bidder.clone())));
			let desired_bid = BidInfoFilter {
				project_id: Some(pid),
				bidder: Some(bid.bidder),
				original_ct_amount: Some(bid.amount),
				original_ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Accepted),
				..Default::default()
			};

			assert!(
				test_env.in_ext(|| stored_bids.any(|bid| desired_bid.matches_bid(&bid))),
				"Stored bid does not match the given filter"
			)
		}

		for bid in excluded_bids {
			let pid = auctioning_project.get_project_id();
			let mut stored_bids =
				auctioning_project.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((pid, bid.bidder.clone())));
			let desired_bid = BidInfoFilter {
				project_id: Some(pid),
				bidder: Some(bid.bidder),
				original_ct_amount: Some(bid.amount),
				original_ct_usd_price: Some(bid.price),
				status: Some(BidStatus::Rejected(RejectionReason::AfterCandleEnd)),
				..Default::default()
			};
			assert!(
				test_env.in_ext(|| stored_bids.any(|bid| desired_bid.matches_bid(&bid))),
				"Stored bid does not match the given filter"
			);
		}
	}

	#[test]
	fn pallet_can_start_auction_automatically() {
		let test_env = TestEnvironment::new();
		let project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc: UserToPLMCBalance =
			evaluations.clone().into_iter().map(|(account, _amount)| (account, get_ed())).collect();
		test_env.mint_plmc_to(required_plmc);
		test_env.mint_plmc_to(ed_plmc);
		project.bond_for_users(evaluations).unwrap();
		test_env.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(project.get_project_details().status, ProjectStatus::AuctionInitializePeriod);
		test_env.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
		assert_eq!(project.get_project_details().status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn issuer_can_start_auction_manually() {
		let test_env = TestEnvironment::new();
		let project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc: UserToPLMCBalance =
			evaluations.clone().into_iter().map(|(account, _amount)| (account, get_ed())).collect();
		test_env.mint_plmc_to(required_plmc);
		test_env.mint_plmc_to(ed_plmc);
		project.bond_for_users(evaluations).unwrap();
		test_env.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(project.get_project_details().status, ProjectStatus::AuctionInitializePeriod);
		test_env.advance_time(1).unwrap();

		test_env
			.in_ext(|| FundingModule::start_auction(RuntimeOrigin::signed(ISSUER), project.get_project_id()))
			.unwrap();
		assert_eq!(project.get_project_details().status, ProjectStatus::AuctionRound(AuctionPhase::English));
	}

	#[test]
	fn stranger_cannot_start_auction_manually() {
		let test_env = TestEnvironment::new();
		let project = EvaluatingProject::new_with(&test_env, default_project(0), ISSUER);
		let evaluations = default_evaluations();
		let required_plmc = calculate_evaluation_plmc_spent(evaluations.clone());
		let ed_plmc: UserToPLMCBalance =
			evaluations.clone().into_iter().map(|(account, _amount)| (account, get_ed())).collect();
		test_env.mint_plmc_to(required_plmc);
		test_env.mint_plmc_to(ed_plmc);
		project.bond_for_users(evaluations).unwrap();
		test_env.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
		assert_eq!(project.get_project_details().status, ProjectStatus::AuctionInitializePeriod);
		test_env.advance_time(1).unwrap();

		for account in 6000..6010 {
			test_env.in_ext(|| {
				let response = FundingModule::start_auction(RuntimeOrigin::signed(account), project.get_project_id());
				assert_noop!(response, Error::<TestRuntime>::NotAllowed);
			});
		}
	}

	#[test]
	fn bidder_was_evaluator() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let mut bids = default_bids();
		let evaluator = evaluations[0].0;
		bids.push(TestBid::new(evaluator, 150 * ASSET_UNIT, 21_u128.into(), 1u8, AcceptedFundingAsset::USDT));
		let _community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
	}

	#[test]
	fn bids_at_higher_price_than_weighted_average_use_average() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids: TestBids = vec![
			TestBid::new(BIDDER_1, 10_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 20_000 * ASSET_UNIT, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_4, 20_000 * ASSET_UNIT, 16.into(), 1u8, AcceptedFundingAsset::USDT),
		];

		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let project_id = community_funding_project.project_id;
		let bidder_2_bid =
			test_env.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id, BIDDER_2)).next().unwrap());
		assert_eq!(bidder_2_bid.final_ct_usd_price.checked_mul_int(US_DOLLAR).unwrap(), 17_6_666_666_666);
	}

	#[test]
	fn ct_minted_for_bids_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids =
			test_env.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_bids],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				test_env.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_bids_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_bids =
			test_env.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for bid in stored_bids.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for bid in stored_bids.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::bid_ct_mint_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
				.unwrap()
			});
		}

		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_bids],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				test_env.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_bids =
			test_env.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for bid in stored_bids.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::bid_ct_mint_for(
					RuntimeOrigin::signed(bid.bidder),
					project_id,
					bid.bidder,
					bid.id,
				)
				.unwrap();

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
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10u64.into()).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids =
			test_env.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_bids.len(), bids.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_bids.clone()],
			|bid| bid.bidder,
			BalanceOf::<TestRuntime>::zero(),
			|bid, acc| acc + bid.final_ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), bids.len());

		for (bidder, amount) in user_ct_amounts {
			let minted =
				test_env.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, bidder));
			assert_eq!(minted, amount);
		}

		for bid in stored_bids.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			})
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		let mut bids = default_bids();
		let median_price = bids[bids.len().div(2)].price;
		let new_bids = vec![
			TestBid::new(BIDDER_4, 30_000 * US_DOLLAR, median_price, 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_5, 167_000 * US_DOLLAR, median_price, 1u8, AcceptedFundingAsset::USDT),
		];
		bids.extend(new_bids.clone());

		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let final_price = details.weighted_average_price.unwrap();
		let plmc_locked_for_bids = calculate_auction_plmc_spent_after_price_calculation(new_bids, final_price);

		for (user, amount) in plmc_locked_for_bids {
			let schedule = test_env.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&user,
					LockType::Participation(finished_project.project_id),
				)
			});

			assert_eq!(schedule.unwrap(), amount);
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);

		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		for bid in stored_bids {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::start_bid_vesting_schedule_for(
					RuntimeOrigin::signed(bid.bidder),
					finished_project.project_id,
					bid.bidder,
					bid.id,
				)
			);

			let schedule = test_env.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&bid.bidder,
					LockType::Participation(finished_project.project_id),
				)
			});

			let bid = test_env
				.in_ext(|| Bids::<TestRuntime>::get((finished_project.project_id, bid.bidder, bid.id)).unwrap());
			assert_eq!(schedule.unwrap(), bid.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());

		test_env.advance_time((10 * DAYS).into()).unwrap();

		for bid in stored_bids {
			let vesting_info = bid.plmc_vesting_info.unwrap();
			let locked_amount = vesting_info.total_amount;

			let prev_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));

			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(
						bid.bidder.clone(),
						finished_project.project_id,
						bid.bidder.clone(),
					)
				})
				.unwrap();

			let post_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));
			assert_eq!(locked_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = vec![
			TestBid::new(BIDDER_1, 40_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_2,
				152_000 * ASSET_UNIT,
				11.into(),
				10u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(BIDDER_3, 20_000 * ASSET_UNIT, 17.into(), 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_4,
				88_000 * ASSET_UNIT,
				18.into(),
				25u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(15u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();
		let stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());

		for bid in stored_bids {
			let vesting_info = bid.plmc_vesting_info.unwrap();

			let now = test_env.current_block();

			let blocks_vested = min(vesting_info.duration, now - vest_start_block);
			let vested_amount = vesting_info.amount_per_block * blocks_vested as u128;

			let prev_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));

			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(
						bid.bidder.clone(),
						finished_project.project_id,
						bid.bidder.clone(),
					)
				})
				.unwrap();

			let post_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&bid.bidder));
			assert_eq!(vested_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn unsuccessful_bids_dont_get_vest_schedule() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let mut bids = default_bids();

		let available_tokens =
			project.total_allocation_size.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));

		let median_price = bids[bids.len().div(2)].price;
		let accepted_bid =
			vec![TestBid::new(BIDDER_4, available_tokens, median_price, 1u8, AcceptedFundingAsset::USDT)];
		let rejected_bid =
			vec![TestBid::new(BIDDER_5, 50_000 * ASSET_UNIT, median_price, 1u8, AcceptedFundingAsset::USDT)];
		bids.extend(accepted_bid.clone());
		bids.extend(rejected_bid.clone());

		let community_contributions = default_community_buys();

		let auctioning_project = AuctioningProject::new_with(&test_env, project, issuer, evaluations);
		let mut bidders_plmc = calculate_auction_plmc_spent(bids.clone());
		bidders_plmc.iter_mut().for_each(|(_acc, amount)| *amount += get_ed());
		test_env.mint_plmc_to(bidders_plmc.clone());

		let bidders_funding_assets = calculate_auction_funding_asset_spent(bids.clone());
		test_env.mint_statemint_asset_to(bidders_funding_assets.clone());

		auctioning_project.bid_for_users(bids).unwrap();

		let community_funding_project = auctioning_project.start_community_funding();
		let final_price = community_funding_project.get_project_details().weighted_average_price.unwrap();
		let mut contributors_plmc = calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		contributors_plmc.iter_mut().for_each(|(_acc, amount)| *amount += get_ed());
		test_env.mint_plmc_to(contributors_plmc.clone());

		let contributors_funding_assets =
			calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
		test_env.mint_statemint_asset_to(contributors_funding_assets.clone());

		community_funding_project.buy_for_retail_users(community_contributions).unwrap();
		let finished_project = community_funding_project.finish_funding();

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let plmc_locked_for_accepted_bid =
			calculate_auction_plmc_spent_after_price_calculation(accepted_bid, final_price);
		let plmc_locked_for_rejected_bid =
			calculate_auction_plmc_spent_after_price_calculation(rejected_bid, final_price);

		let (accepted_user, accepted_plmc_amount) = plmc_locked_for_accepted_bid[0];
		let schedule = test_env.in_ext(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&accepted_user,
				LockType::Participation(finished_project.project_id),
			)
		});
		assert_eq!(schedule.unwrap(), accepted_plmc_amount);

		let (rejected_user, _rejected_plmc_amount) = plmc_locked_for_rejected_bid[0];
		let schedule_exists = test_env
			.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&rejected_user,
					LockType::Participation(finished_project.project_id),
				)
			})
			.is_some();
		assert!(!schedule_exists);
	}

	#[test]
	pub fn bid_funding_assets_are_paid_automatically_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta, total_expected_bid_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn bid_funding_assets_are_paid_manually_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_winning_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::payout_bid_funds_for(
						RuntimeOrigin::signed(issuer),
						project_id,
						bid.bidder,
						bid.id,
					)
				})
				.unwrap();
		}

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta, total_expected_bid_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn bid_funding_assets_are_released_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let mut bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		bids.remove(0);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).sorted().collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
	}

	#[test]
	pub fn bid_funding_assets_are_released_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let mut bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		bids.remove(0);
		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_winning_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::release_bid_funds_for(
						RuntimeOrigin::signed(bid.bidder.clone()),
						project_id,
						bid.bidder,
						bid.id,
					)
				})
				.unwrap();
		}

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).sorted().collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
	}

	#[test]
	pub fn bid_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		let mut bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let prev_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());

		let mut delta_bidders_plmc_balances =
			merge_subtract_mappings_by_user(post_bidders_plmc_balances, vec![prev_bidders_plmc_balances]);
		delta_bidders_plmc_balances.sort();

		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = calculate_auction_plmc_spent_after_price_calculation(bids, final_price);
		plmc_locked_for_bids.sort();

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
	}

	#[test]
	pub fn bid_plmc_bonded_is_returned_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		let mut bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_winning_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let prev_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder.clone()),
					project_id,
					bid.bidder,
					bid.id,
				),
				Pallet::<TestRuntime>::bid_unbond_for(
					RuntimeOrigin::signed(bid.bidder.clone()),
					project_id,
					bid.bidder,
					bid.id,
				)
			);
		}

		let post_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());

		let mut delta_bidders_plmc_balances =
			merge_subtract_mappings_by_user(post_bidders_plmc_balances, vec![prev_bidders_plmc_balances]);
		delta_bidders_plmc_balances.sort();

		let details = finished_project.get_project_details();
		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = calculate_auction_plmc_spent_after_price_calculation(bids, final_price);
		plmc_locked_for_bids.sort();

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
	}
}

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
					100_u128.into(),
					1u8.try_into().unwrap(),
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
					1u8.try_into().unwrap(),
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
			TestBid::new(DAVE, 10_000 * USDT_UNIT, 2_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 20k
			TestBid::new(DAVE, 12_000 * USDT_UNIT, 8_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 96k
			TestBid::new(DAVE, 15_000 * USDT_UNIT, 5_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 75k
			// Bid with lowest PLMC bonded gets dropped
			TestBid::new(DAVE, 1_000 * USDT_UNIT, 7_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 7k
			TestBid::new(DAVE, 20_000 * USDT_UNIT, 5_u128.into(), 1u8, AcceptedFundingAsset::USDT), // 100k
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
			let mut stored_bids = Bids::<TestRuntime>::iter_prefix_values((project_id, DAVE)).collect::<Vec<_>>();
			assert_eq!(stored_bids.len(), 4);
			stored_bids.sort();
			assert_eq!(stored_bids[0].original_ct_usd_price, 2_u128.into());
			assert_eq!(stored_bids[1].original_ct_usd_price, 5_u128.into());
			assert_eq!(stored_bids[2].original_ct_usd_price, 5_u128.into());
			assert_eq!(stored_bids[3].original_ct_usd_price, 8_u128.into());
		});
	}

	#[test]
	fn bid_with_asset_not_accepted() {
		let test_env = TestEnvironment::new();
		let auctioning_project =
			AuctioningProject::new_with(&test_env, default_project(0), ISSUER, default_evaluations());
		let bids = vec![
			TestBid::new(BIDDER_1, 10_000, 2_u128.into(), 1u8, AcceptedFundingAsset::USDC),
			TestBid::new(BIDDER_2, 13_000, 3_u128.into(), 2u8, AcceptedFundingAsset::USDC),
		];
		let outcome = auctioning_project.bid_for_users(bids);
		frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
	}

	#[test]
	fn no_bids_made() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bidding_project = AuctioningProject::new_with(&test_env, project, issuer, evaluations);

		let details = bidding_project.get_project_details();
		let english_end = details.phase_transition_points.english_auction.end().unwrap();
		let now = test_env.current_block();
		test_env.advance_time(english_end - now + 2).unwrap();

		let details = bidding_project.get_project_details();
		let candle_end = details.phase_transition_points.candle_auction.end().unwrap();
		let now = test_env.current_block();
		test_env.advance_time(candle_end - now + 2).unwrap();

		let details = bidding_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingFailed);
	}

	#[test]
	fn after_ct_soldout_bid_gets_refunded() {
		let test_env = TestEnvironment::new();
		let auctioning_project =
			AuctioningProject::new_with(&test_env, default_project(0), ISSUER, default_evaluations());
		let metadata = auctioning_project.get_project_metadata();
		let max_cts_for_bids = metadata.total_allocation_size.clone();
		let project_id = auctioning_project.get_project_id();

		let glutton_bid = TestBid::new(BIDDER_1, max_cts_for_bids, 10_u128.into(), 1u8, AcceptedFundingAsset::USDT);
		let rejected_bid = TestBid::new(BIDDER_2, 10_000 * ASSET_UNIT, 5_u128.into(), 1u8, AcceptedFundingAsset::USDT);

		let mut plmc_fundings: UserToPLMCBalance =
			calculate_auction_plmc_spent(vec![glutton_bid.clone(), rejected_bid.clone()]);
		plmc_fundings.push((BIDDER_1, get_ed()));
		plmc_fundings.push((BIDDER_2, get_ed()));

		let usdt_fundings = calculate_auction_funding_asset_spent(vec![glutton_bid.clone(), rejected_bid.clone()]);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(usdt_fundings.clone());

		auctioning_project.bid_for_users(vec![glutton_bid, rejected_bid]).expect("Bids should pass");

		test_env.do_free_plmc_assertions(vec![(BIDDER_1, get_ed()), (BIDDER_2, get_ed())]);
		test_env.do_reserved_plmc_assertions(
			vec![(BIDDER_1, plmc_fundings[0].1), (BIDDER_2, plmc_fundings[1].1)],
			LockType::Participation(project_id),
		);
		test_env.do_bid_transferred_statemint_asset_assertions(
			vec![
				(BIDDER_1, usdt_fundings[0].1, AcceptedFundingAsset::USDT.to_statemint_id()),
				(BIDDER_2, usdt_fundings[1].1, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);

		let _community_funding_project = auctioning_project.start_community_funding();

		test_env.do_free_plmc_assertions(vec![(BIDDER_1, get_ed()), (BIDDER_2, plmc_fundings[1].1 + get_ed())]);

		test_env.do_reserved_plmc_assertions(
			vec![(BIDDER_1, plmc_fundings[0].1), (BIDDER_2, 0)],
			LockType::Participation(project_id),
		);

		test_env.do_bid_transferred_statemint_asset_assertions(
			vec![
				(BIDDER_1, usdt_fundings[0].1, AcceptedFundingAsset::USDT.to_statemint_id()),
				(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);
	}

	#[test]
	fn after_random_end_bid_gets_refunded() {
		let test_env = TestEnvironment::new();
		let auctioning_project =
			AuctioningProject::new_with(&test_env, default_project(0), ISSUER, default_evaluations());
		let project_id = auctioning_project.get_project_id();

		let (bid_in, bid_out) = (default_bids()[0], default_bids()[1]);

		let mut plmc_fundings: UserToPLMCBalance = calculate_auction_plmc_spent(vec![bid_in.clone(), bid_out.clone()]);
		plmc_fundings.push((BIDDER_1, get_ed()));
		plmc_fundings.push((BIDDER_2, get_ed()));

		let usdt_fundings = calculate_auction_funding_asset_spent(vec![bid_in.clone(), bid_out.clone()]);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(usdt_fundings.clone());

		auctioning_project.bid_for_users(vec![bid_in]).expect("Bids should pass");
		test_env
			.advance_time(
				<TestRuntime as Config>::EnglishAuctionDuration::get() +
					<TestRuntime as Config>::CandleAuctionDuration::get() -
					1,
			)
			.unwrap();

		auctioning_project.bid_for_users(vec![bid_out]).expect("Bids should pass");

		test_env.do_free_plmc_assertions(vec![(BIDDER_1, get_ed()), (BIDDER_2, get_ed())]);
		test_env.do_reserved_plmc_assertions(
			vec![(BIDDER_1, plmc_fundings[0].1), (BIDDER_2, plmc_fundings[1].1)],
			LockType::Participation(project_id),
		);
		test_env.do_bid_transferred_statemint_asset_assertions(
			vec![
				(BIDDER_1, usdt_fundings[0].1, AcceptedFundingAsset::USDT.to_statemint_id()),
				(BIDDER_2, usdt_fundings[1].1, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);
		let _community_funding_project = auctioning_project.start_community_funding();
		test_env.do_free_plmc_assertions(vec![(BIDDER_1, get_ed()), (BIDDER_2, plmc_fundings[1].1 + get_ed())]);

		test_env.do_reserved_plmc_assertions(
			vec![(BIDDER_1, plmc_fundings[0].1), (BIDDER_2, 0)],
			LockType::Participation(project_id),
		);

		test_env.do_bid_transferred_statemint_asset_assertions(
			vec![
				(BIDDER_1, usdt_fundings[0].1, AcceptedFundingAsset::USDT.to_statemint_id()),
				(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_statemint_id()),
			],
			project_id,
		);
	}
}

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
		let token_price = community_funding_project.get_project_details().weighted_average_price.unwrap();
		let contributions: TestContributions = vec![
			TestContribution::new(BOB, 3 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BOB, 4 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let mut plmc_funding: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), token_price);
		plmc_funding.push((BOB, get_ed()));
		let statemint_funding: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

		test_env.mint_plmc_to(plmc_funding);
		test_env.mint_statemint_asset_to(statemint_funding.clone());

		community_funding_project
			.buy_for_retail_users(vec![contributions[0]])
			.expect("The Buyer should be able to buy multiple times");
		test_env.advance_time((1 * HOURS) as BlockNumber).unwrap();

		community_funding_project
			.buy_for_retail_users(vec![contributions[1]])
			.expect("The Buyer should be able to buy multiple times");

		let project_id = community_funding_project.get_project_id();
		let bob_total_contributions: BalanceOf<TestRuntime> = community_funding_project.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project_id, BOB)).map(|c| c.funding_asset_amount).sum()
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

		let remaining_ct = community_funding_project.get_project_details().remaining_contribution_tokens;
		let ct_price =
			community_funding_project.get_project_details().weighted_average_price.expect("CT Price should exist");
		let project_id = community_funding_project.get_project_id();

		let contributions: TestContributions =
			vec![TestContribution::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
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
		test_env.advance_time(2u64).unwrap();
		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project.get_project_details().remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(community_funding_project.get_project_details().status, ProjectStatus::FundingSuccessful);

		test_env.do_free_plmc_assertions(vec![plmc_fundings[1].clone()]);
		test_env.do_free_statemint_asset_assertions(vec![(BOB, 0_u128, AcceptedFundingAsset::USDT.to_statemint_id())]);
		test_env.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], LockType::Participation(project_id));
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

		let remaining_ct = community_funding_project.get_project_details().remaining_contribution_tokens;

		let ct_price =
			community_funding_project.get_project_details().weighted_average_price.expect("CT Price should exist");

		let project_id = community_funding_project.get_project_id();

		let contributions: TestContributions = vec![
			TestContribution::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BOB, OVERBUY_CT, 1u8, AcceptedFundingAsset::USDT),
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
		test_env.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			community_funding_project.get_project_details().remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(community_funding_project.get_project_details().status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).1;
		let remaining_plmc: BalanceOf<TestRuntime> = plmc_fundings.iter().fold(0_u128, |acc, (_, amount)| acc + amount);

		let actual_funding_transferred = statemint_asset_fundings.swap_remove(0).1;
		let remaining_statemint_assets: BalanceOf<TestRuntime> =
			statemint_asset_fundings.iter().fold(0_u128, |acc, (_, amount, _)| acc + amount);

		test_env.do_free_plmc_assertions(vec![(BOB, remaining_plmc)]);
		test_env.do_free_statemint_asset_assertions(vec![(
			BOB,
			remaining_statemint_assets,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		test_env.do_reserved_plmc_assertions(vec![(BOB, reserved_plmc)], LockType::Participation(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(
			vec![(BOB, actual_funding_transferred, AcceptedFundingAsset::USDT.to_statemint_id())],
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
		let token_amount: BalanceOf<TestRuntime> = 1 * ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: TestContributions =
			range.map(|_| TestContribution::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT)).collect();

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
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&CONTRIBUTOR));
		let contributor_post_buy_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(contributor_post_buy_plmc_balance, get_ed());
		assert_eq!(contributor_post_buy_statemint_asset_balance, 0);

		let plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project.project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(statemint_asset_contributions_stored, sum_statemint_mappings(vec![statemint_funding.clone()]));

		let new_token_amount: BalanceOf<TestRuntime> = 2 * ASSET_UNIT;
		let new_contribution: TestContributions =
			vec![TestContribution::new(CONTRIBUTOR, new_token_amount, 1u8, AcceptedFundingAsset::USDT)];

		let new_plmc_funding = calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding = calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		test_env.mint_plmc_to(new_plmc_funding.clone());
		test_env.mint_statemint_asset_to(new_statemint_funding.clone());

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		let contributor_post_return_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(contributor_post_return_plmc_balance, contributor_post_buy_plmc_balance + plmc_funding[0].1);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding.clone()[0].1
		);

		let new_plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project.project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + sum_balance_mappings(vec![new_plmc_funding.clone()]) - plmc_funding[0].1
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored + sum_statemint_mappings(vec![new_statemint_funding.clone()]) -
				statemint_funding[0].1
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
		let token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
		let contributions: TestContributions =
			range.map(|_| TestContribution::new(CONTRIBUTOR, token_amount, 3u8, AcceptedFundingAsset::USDT)).collect();

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
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project.project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(plmc_bond_stored, sum_balance_mappings(vec![plmc_funding.clone()]));
		assert_eq!(statemint_asset_contributions_stored, sum_statemint_mappings(vec![statemint_funding.clone()]));

		let new_token_amount: BalanceOf<TestRuntime> = 10 * ASSET_UNIT;
		let new_contribution: TestContributions =
			vec![TestContribution::new(CONTRIBUTOR, new_token_amount, 1u8, AcceptedFundingAsset::USDT)];

		let new_plmc_funding = calculate_contributed_plmc_spent(new_contribution.clone(), token_price);
		let new_statemint_funding = calculate_contributed_funding_asset_spent(new_contribution.clone(), token_price);

		test_env.mint_plmc_to(new_plmc_funding.clone());
		test_env.mint_statemint_asset_to(new_statemint_funding.clone());

		project.buy_for_retail_users(new_contribution.clone()).unwrap();

		let contributor_post_return_plmc_balance =
			project.in_ext(|| <TestRuntime as Config>::NativeCurrency::free_balance(&CONTRIBUTOR));
		let contributor_post_return_statemint_asset_balance =
			project.in_ext(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_STATEMINT_ID, &CONTRIBUTOR));

		assert_eq!(contributor_post_return_plmc_balance, contributor_post_buy_plmc_balance + plmc_funding[0].1);
		assert_eq!(
			contributor_post_return_statemint_asset_balance,
			contributor_post_buy_statemint_asset_balance + statemint_funding.clone()[0].1
		);

		let new_plmc_bond_stored = project.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(&LockType::Participation(project_id), &CONTRIBUTOR)
		});
		let new_statemint_asset_contributions_stored = project.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((project.project_id, CONTRIBUTOR))
				.map(|c| c.funding_asset_amount)
				.sum::<BalanceOf<TestRuntime>>()
		});

		assert_eq!(
			new_plmc_bond_stored,
			plmc_bond_stored + sum_balance_mappings(vec![new_plmc_funding.clone()]) - plmc_funding[0].1
		);

		assert_eq!(
			new_statemint_asset_contributions_stored,
			statemint_asset_contributions_stored + sum_statemint_mappings(vec![new_statemint_funding.clone()]) -
				statemint_funding[0].1
		);
	}

	#[test]
	fn retail_contributor_was_evaluator() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			TestContribution::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let contributing_project = CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let ct_price = contributing_project.get_project_details().weighted_average_price.unwrap();
		let already_bonded_plmc =
			calculate_evaluation_plmc_spent(vec![(evaluator_contributor, evaluation_amount)])[0].1;
		let plmc_available_for_participating =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_contribution = calculate_contributed_plmc_spent(vec![contribution], ct_price)[0].1;
		let necessary_usdt_for_contribution = calculate_contributed_funding_asset_spent(vec![contribution], ct_price);

		test_env.mint_plmc_to(vec![(
			evaluator_contributor,
			necessary_plmc_for_contribution - plmc_available_for_participating,
		)]);
		test_env.mint_statemint_asset_to(necessary_usdt_for_contribution);

		contributing_project.buy_for_retail_users(vec![contribution]).unwrap();
	}

	#[test]
	fn retail_contributor_was_evaluator_vec_full() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let bids = default_bids();
		let evaluator_contributor = 69;
		let overflow_contribution =
			TestContribution::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let mut fill_contributions = Vec::new();
		for _i in 0..<TestRuntime as Config>::MaxContributionsPerUser::get() {
			fill_contributions.push(TestContribution::new(
				evaluator_contributor,
				10 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			));
		}

		let expected_price = calculate_price_from_test_bids(bids.clone());
		let fill_necessary_plmc = calculate_contributed_plmc_spent(fill_contributions.clone(), expected_price);
		let fill_necessary_usdt = calculate_contributed_funding_asset_spent(fill_contributions.clone(), expected_price);

		let overflow_necessary_plmc = calculate_contributed_plmc_spent(vec![overflow_contribution], expected_price);
		let overflow_necessary_usdt =
			calculate_contributed_funding_asset_spent(vec![overflow_contribution], expected_price);

		let evaluation_bond = sum_balance_mappings(vec![fill_necessary_plmc, overflow_necessary_plmc.clone()]);
		let plmc_available_for_participating =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push((evaluator_contributor, evaluation_usd_amount));

		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let project_id = community_funding_project.get_project_id();

		test_env.mint_plmc_to(vec![(evaluator_contributor, evaluation_bond - plmc_available_for_participating)]);
		test_env.mint_statemint_asset_to(fill_necessary_usdt);
		test_env.mint_statemint_asset_to(overflow_necessary_usdt);

		community_funding_project.buy_for_retail_users(fill_contributions).unwrap();
		community_funding_project.buy_for_retail_users(vec![overflow_contribution]).unwrap();

		let evaluation_bonded = test_env.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_contributor,
			)
		});
		assert_eq!(evaluation_bonded, <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond);
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_fail() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			TestContribution::new(evaluator_contributor, 22 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let contributing_project = CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let ct_price = contributing_project.get_project_details().weighted_average_price.unwrap();
		let necessary_plmc_for_contribution = calculate_contributed_plmc_spent(vec![contribution], ct_price)[0].1;
		let plmc_evaluation_amount =
			calculate_evaluation_plmc_spent(vec![(evaluator_contributor, evaluation_amount)])[0].1;
		let plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;
		assert!(
			necessary_plmc_for_contribution > plmc_available_for_participating &&
				necessary_plmc_for_contribution < plmc_evaluation_amount
		);
		// 1199_9_999_999_999
		// 49_9_999_999_999
		let necessary_usdt_for_contribution = calculate_contributed_funding_asset_spent(vec![contribution], ct_price);

		test_env.mint_statemint_asset_to(necessary_usdt_for_contribution);

		assert_matches!(contributing_project.buy_for_retail_users(vec![contribution]), Err(_));
	}

	#[test]
	fn evaluator_cannot_use_slash_reserve_for_contributing_call_success() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let contribution =
			TestContribution::new(evaluator_contributor, 22 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let contributing_project = CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let project_id = contributing_project.get_project_id();

		let ct_price = contributing_project.get_project_details().weighted_average_price.unwrap();
		let necessary_plmc_for_contribution = calculate_contributed_plmc_spent(vec![contribution], ct_price)[0].1;
		let plmc_evaluation_amount =
			calculate_evaluation_plmc_spent(vec![(evaluator_contributor, evaluation_amount)])[0].1;
		let plmc_available_for_participating =
			plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;
		assert!(
			necessary_plmc_for_contribution > plmc_available_for_participating &&
				necessary_plmc_for_contribution < plmc_evaluation_amount
		);
		let necessary_usdt_for_contribution = calculate_contributed_funding_asset_spent(vec![contribution], ct_price);

		test_env.mint_plmc_to(vec![(
			evaluator_contributor,
			necessary_plmc_for_contribution - plmc_available_for_participating,
		)]);
		test_env.mint_statemint_asset_to(necessary_usdt_for_contribution);

		contributing_project.buy_for_retail_users(vec![contribution]).unwrap();
		let evaluation_locked =
			test_env.get_reserved_plmc_balances_for(vec![evaluator_contributor], LockType::Evaluation(project_id))[0].1;
		let participation_locked = test_env
			.get_reserved_plmc_balances_for(vec![evaluator_contributor], LockType::Participation(project_id))[0]
			.1;

		assert_eq!(evaluation_locked, <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount);
		assert_eq!(participation_locked, necessary_plmc_for_contribution);
	}

	#[test]
	fn ct_minted_for_community_buys_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_community_buys =
			test_env.in_ext(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_community_buys.len(), community_contributions.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_community_buys],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_community_buys_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_contributions =
			test_env.in_ext(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap()
			});
		}

		assert_eq!(stored_contributions.len(), community_contributions.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_contributions],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		let stored_contributions =
			test_env.in_ext(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap();

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
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_contributions =
			test_env.in_ext(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert_eq!(stored_contributions.len(), community_contributions.len());
		let user_ct_amounts = generic_map_merge_reduce(
			vec![stored_contributions.clone()],
			|contribution| contribution.contributor,
			BalanceOf::<TestRuntime>::zero(),
			|contribution, acc| acc + contribution.ct_amount,
		);
		assert_eq!(user_ct_amounts.len(), community_contributions.len());

		for (contributor, amount) in user_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::NotAllowed
				);
			})
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);

		let price = finished_project.get_project_details().weighted_average_price.unwrap();
		let contribution_locked_plmc = calculate_contributed_plmc_spent(community_contributions, price);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		for (user, amount) in contribution_locked_plmc {
			let schedule = test_env.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&user,
					LockType::Participation(finished_project.project_id),
				)
			});

			assert_eq!(schedule.unwrap(), amount);
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);

		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		for contribution in contributions {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::start_contribution_vesting_schedule_for(
					RuntimeOrigin::signed(contribution.contributor),
					finished_project.project_id,
					contribution.contributor,
					contribution.id,
				)
			);

			let schedule = test_env.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&contribution.contributor,
					LockType::Participation(finished_project.project_id),
				)
			});

			let contribution = test_env.in_ext(|| {
				Contributions::<TestRuntime>::get((
					finished_project.project_id,
					contribution.contributor,
					contribution.id,
				))
				.unwrap()
			});
			assert_eq!(schedule.unwrap(), contribution.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		test_env.advance_time((10 * DAYS).into()).unwrap();

		for contribution in stored_contributions {
			let vesting_info = contribution.plmc_vesting_info.unwrap();
			let locked_amount = vesting_info.total_amount;

			let prev_free_balance =
				test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));

			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(
						contribution.contributor.clone(),
						finished_project.project_id,
						contribution.contributor.clone(),
					)
				})
				.unwrap();

			let post_free_balance =
				test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));
			assert_eq!(locked_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = vec![
			TestBid::new(BIDDER_1, 40_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_2,
				152_000 * ASSET_UNIT,
				11.into(),
				10u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(BIDDER_3, 20_000 * ASSET_UNIT, 17.into(), 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_4,
				88_000 * ASSET_UNIT,
				18.into(),
				25u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(15u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();
		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		for contribution in stored_contributions {
			let vesting_info = contribution.plmc_vesting_info.unwrap();

			let now = test_env.current_block();
			let blocks_vested = min(vesting_info.duration, now - vest_start_block);
			let vested_amount = vesting_info.amount_per_block * blocks_vested as u128;

			let prev_free_balance =
				test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));

			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(
						contribution.contributor.clone(),
						finished_project.project_id,
						contribution.contributor.clone(),
					)
				})
				.unwrap();

			let post_free_balance =
				test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contribution.contributor));
			assert_eq!(vested_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_contributors_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].2,
			final_contribution_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);

		let prev_total_contributor_balance =
			prev_contributors_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_contributors_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].2,
			final_contribution_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);

		let post_total_contributor_balance =
			post_contributors_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_contributor_balance, 0u128);
		assert_eq!(post_total_contributor_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_winning_bids = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_contributors_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].2,
			final_contribution_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);

		let prev_total_contributor_balance =
			prev_contributors_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
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
			test_env
				.in_ext(|| {
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
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_contributors_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_contribution_payouts[0].2,
			final_contribution_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);

		let post_total_contributor_balance =
			post_contributors_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(prev_total_contributor_balance, 0u128);
		assert_eq!(post_total_contributor_balance, 0u128);
		assert_eq!(post_project_pot_funding_balance, 0u128);
	}
}

mod community_round_failure {
	use super::*;

	#[test]
	pub fn bid_and_community_contribution_funding_assets_are_released_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});
		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let expected_community_contribution_payouts = calculate_contributed_funding_asset_spent(
			community_contributions.clone(),
			finished_project.get_project_details().weighted_average_price.unwrap(),
		)
		.iter()
		.map(|(acc, balance, _asset)| (acc.clone(), *balance))
		.collect::<Vec<_>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_contributors_funding_balances = test_env
			.get_free_statemint_asset_balances_for(
				final_bid_payouts[0].2,
				community_contributions
					.iter()
					.map(|test_contribution| test_contribution.contributor.clone())
					.collect::<Vec<_>>(),
			)
			.iter()
			.map(|(acc, balance, _asset)| (acc.clone(), *balance))
			.collect::<Vec<_>>();
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).sorted().collect::<Vec<_>>(),
		);
		let post_contributors_funding_balances = test_env
			.get_free_statemint_asset_balances_for(
				final_bid_payouts[0].2,
				community_contributions
					.iter()
					.map(|test_contribution| test_contribution.contributor.clone())
					.collect::<Vec<_>>(),
			)
			.iter()
			.map(|(acc, balance, _asset)| (acc.clone(), *balance))
			.collect::<Vec<_>>();
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let contributors_funding_delta = merge_subtract_mappings_by_user(
			post_contributors_funding_balances,
			vec![prev_contributors_funding_balances],
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
		assert_eq!(contributors_funding_delta, expected_community_contribution_payouts)
	}

	#[test]
	pub fn bid_and_community_contribution_funding_assets_are_released_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_winning_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let expected_community_contribution_payouts = calculate_contributed_funding_asset_spent(
			community_contributions.clone(),
			finished_project.get_project_details().weighted_average_price.unwrap(),
		)
		.iter()
		.map(|(acc, balance, _asset)| (acc.clone(), *balance))
		.collect::<Vec<_>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let prev_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).collect::<Vec<_>>(),
		);
		let prev_contributors_funding_balances = test_env
			.get_free_statemint_asset_balances_for(
				final_bid_payouts[0].2,
				community_contributions
					.iter()
					.map(|test_contribution| test_contribution.contributor.clone())
					.collect::<Vec<_>>(),
			)
			.iter()
			.map(|(acc, balance, _asset)| (acc.clone(), *balance))
			.collect::<Vec<_>>();
		let prev_total_bidder_balance =
			prev_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::release_bid_funds_for(
						RuntimeOrigin::signed(bid.bidder.clone()),
						project_id,
						bid.bidder,
						bid.id,
					)
				})
				.unwrap();
		}

		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		for contribution in stored_contributions {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(contribution.contributor.clone()),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_bidders_funding_balances = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			final_bid_payouts.iter().map(|(acc, _, _)| acc.clone()).sorted().collect::<Vec<_>>(),
		);
		let post_total_bidder_balance =
			post_bidders_funding_balances.iter().map(|(_, balance, _)| balance).sum::<BalanceOf<TestRuntime>>();
		let post_contributors_funding_balances = test_env
			.get_free_statemint_asset_balances_for(
				final_bid_payouts[0].2,
				community_contributions
					.iter()
					.map(|test_contribution| test_contribution.contributor.clone())
					.collect::<Vec<_>>(),
			)
			.iter()
			.map(|(acc, balance, _asset)| (acc.clone(), *balance))
			.collect::<Vec<_>>();
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let contributors_funding_delta = merge_subtract_mappings_by_user(
			post_contributors_funding_balances,
			vec![prev_contributors_funding_balances],
		);
		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(prev_total_bidder_balance, 0u128);
		assert_eq!(post_total_bidder_balance, total_expected_bid_payout);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(post_bidders_funding_balances, final_bid_payouts);
		assert_eq!(contributors_funding_delta, expected_community_contribution_payouts)
	}

	#[test]
	pub fn bid_and_community_contribution_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];

		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let prev_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		let prev_contributors_plmc_balances = test_env.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor.clone()).collect::<Vec<_>>(),
		);

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);
		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

		let post_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		let post_contributors_plmc_balances = test_env.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor.clone()).collect::<Vec<_>>(),
		);

		let mut delta_bidders_plmc_balances =
			merge_subtract_mappings_by_user(post_bidders_plmc_balances, vec![prev_bidders_plmc_balances]);
		delta_bidders_plmc_balances.sort();

		let mut delta_contributors_plmc_balances =
			merge_subtract_mappings_by_user(post_contributors_plmc_balances, vec![prev_contributors_plmc_balances]);
		delta_contributors_plmc_balances.sort();

		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = calculate_auction_plmc_spent_after_price_calculation(bids, final_price);
		plmc_locked_for_bids.sort();
		let mut plmc_locked_for_contributions = calculate_contributed_plmc_spent(community_contributions, final_price);
		plmc_locked_for_contributions.sort();

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
		assert_eq!(delta_contributors_plmc_balances, plmc_locked_for_contributions);
	}

	#[test]
	pub fn bid_and_community_contribution_plmc_bonded_is_returned_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();

		let mut bids = generate_bids_from_total_usd(project.total_allocation_size, project.minimum_price);
		bids.remove(0);

		let community_contributions = vec![];
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();
		let final_winning_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let prev_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		let prev_contributors_plmc_balances = test_env.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor.clone()).collect::<Vec<_>>(),
		);
		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		for bid in final_winning_bids {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::release_bid_funds_for(
					RuntimeOrigin::signed(bid.bidder.clone()),
					project_id,
					bid.bidder,
					bid.id,
				),
				Pallet::<TestRuntime>::bid_unbond_for(
					RuntimeOrigin::signed(bid.bidder.clone()),
					project_id,
					bid.bidder,
					bid.id,
				)
			);
		}

		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		for contribution in stored_contributions {
			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(contribution.contributor.clone()),
					project_id,
					contribution.contributor,
					contribution.id,
				),
				Pallet::<TestRuntime>::contribution_unbond_for(
					RuntimeOrigin::signed(contribution.contributor.clone()),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_bidders_plmc_balances =
			test_env.get_free_plmc_balances_for(bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>());
		let post_contributors_plmc_balances = test_env.get_free_plmc_balances_for(
			community_contributions.iter().map(|contribution| contribution.contributor.clone()).collect::<Vec<_>>(),
		);

		let mut delta_bidders_plmc_balances =
			merge_subtract_mappings_by_user(post_bidders_plmc_balances, vec![prev_bidders_plmc_balances]);
		delta_bidders_plmc_balances.sort();
		let mut delta_contributors_plmc_balances =
			merge_subtract_mappings_by_user(post_contributors_plmc_balances, vec![prev_contributors_plmc_balances]);
		delta_contributors_plmc_balances.sort();

		let details = finished_project.get_project_details();
		let final_price = details.weighted_average_price.unwrap();
		let mut plmc_locked_for_bids = calculate_auction_plmc_spent_after_price_calculation(bids, final_price);
		plmc_locked_for_bids.sort();
		let mut plmc_locked_for_contributions = calculate_contributed_plmc_spent(community_contributions, final_price);
		plmc_locked_for_contributions.sort();

		assert_eq!(delta_bidders_plmc_balances, plmc_locked_for_bids);
		assert_eq!(delta_contributors_plmc_balances, plmc_locked_for_contributions);
	}
}

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

	#[test]
	fn remainder_contributor_was_evaluator() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let community_contributions = default_community_buys();
		let evaluator_contributor = 69;
		let evaluation_amount = 420 * US_DOLLAR;
		let remainder_contribution =
			TestContribution::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
		evaluations.push((evaluator_contributor, evaluation_amount));
		let bids = default_bids();

		let remainder_funding_project =
			RemainderFundingProject::new_with(&test_env, project, issuer, evaluations, bids, community_contributions)
				.unwrap_left();
		let ct_price = remainder_funding_project.get_project_details().weighted_average_price.unwrap();
		let already_bonded_plmc =
			calculate_evaluation_plmc_spent(vec![(evaluator_contributor, evaluation_amount)])[0].1;
		let plmc_available_for_contribution =
			already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
		let necessary_plmc_for_buy = calculate_contributed_plmc_spent(vec![remainder_contribution], ct_price)[0].1;
		let necessary_usdt_for_buy = calculate_contributed_funding_asset_spent(vec![remainder_contribution], ct_price);

		test_env.mint_plmc_to(vec![(evaluator_contributor, necessary_plmc_for_buy - plmc_available_for_contribution)]);
		test_env.mint_statemint_asset_to(necessary_usdt_for_buy);

		remainder_funding_project.buy_for_any_user(vec![remainder_contribution]).unwrap();
	}

	#[test]
	fn remainder_contributor_was_evaluator_vec_full() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let mut evaluations = default_evaluations();
		let bids = default_bids();
		let evaluator_contributor = 69;
		let overflow_contribution =
			TestContribution::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

		let mut fill_contributions = Vec::new();
		for _i in 0..<TestRuntime as Config>::MaxContributionsPerUser::get() {
			fill_contributions.push(TestContribution::new(
				evaluator_contributor,
				10 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			));
		}

		let expected_price = calculate_price_from_test_bids(bids.clone());
		let fill_necessary_plmc = calculate_contributed_plmc_spent(fill_contributions.clone(), expected_price);
		let fill_necessary_usdt_for_bids =
			calculate_contributed_funding_asset_spent(fill_contributions.clone(), expected_price);

		let overflow_necessary_plmc = calculate_contributed_plmc_spent(vec![overflow_contribution], expected_price);
		let overflow_necessary_usdt =
			calculate_contributed_funding_asset_spent(vec![overflow_contribution], expected_price);

		let evaluation_bond = sum_balance_mappings(vec![fill_necessary_plmc, overflow_necessary_plmc.clone()]);
		let plmc_available_for_participating =
			evaluation_bond - <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;

		let evaluation_usd_amount = <TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID)
			.unwrap()
			.saturating_mul_int(evaluation_bond);
		evaluations.push((evaluator_contributor, evaluation_usd_amount));

		let remainder_funding_project =
			RemainderFundingProject::new_with(&test_env, project, issuer, evaluations, bids, default_community_buys())
				.unwrap_left();
		let project_id = remainder_funding_project.get_project_id();

		test_env.mint_plmc_to(vec![(evaluator_contributor, evaluation_bond - plmc_available_for_participating)]);
		test_env.mint_statemint_asset_to(fill_necessary_usdt_for_bids);
		test_env.mint_statemint_asset_to(overflow_necessary_usdt);

		remainder_funding_project.buy_for_any_user(fill_contributions).unwrap();
		remainder_funding_project.buy_for_any_user(vec![overflow_contribution]).unwrap();

		let evaluation_bonded = test_env.in_ext(|| {
			<TestRuntime as Config>::NativeCurrency::balance_on_hold(
				&LockType::Evaluation(project_id),
				&evaluator_contributor,
			)
		});
		assert_eq!(evaluation_bonded, <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond);
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_exact() {
		let test_env = TestEnvironment::new();
		let remainder_funding_project = RemainderFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		)
		.unwrap_left();
		const BOB: AccountId = 808;

		let remaining_ct = remainder_funding_project.get_project_details().remaining_contribution_tokens;
		let ct_price =
			remainder_funding_project.get_project_details().weighted_average_price.expect("CT Price should exist");
		let project_id = remainder_funding_project.get_project_id();

		let contributions: TestContributions =
			vec![TestContribution::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
		let mut plmc_fundings: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push((BOB, get_ed()));
		let statemint_asset_fundings: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		remainder_funding_project
			.buy_for_any_user(contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			remainder_funding_project.get_project_details().remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(remainder_funding_project.get_project_details().status, ProjectStatus::FundingSuccessful);

		test_env.do_free_plmc_assertions(vec![plmc_fundings[1].clone()]);
		test_env.do_free_statemint_asset_assertions(vec![(BOB, 0_u128, AcceptedFundingAsset::USDT.to_statemint_id())]);
		test_env.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], LockType::Participation(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(
			statemint_asset_fundings,
			remainder_funding_project.get_project_id(),
		);
	}

	#[test]
	fn remainder_round_ends_on_all_ct_sold_overbuy() {
		let test_env = TestEnvironment::new();
		let remainder_funding_project = RemainderFundingProject::new_with(
			&test_env,
			default_project(0),
			ISSUER,
			default_evaluations(),
			default_bids(),
			default_community_buys(),
		)
		.unwrap_left();
		const BOB: AccountId = 808;
		const OVERBUY_CT: BalanceOf<TestRuntime> = 40 * ASSET_UNIT;

		let remaining_ct = remainder_funding_project.get_project_details().remaining_contribution_tokens;

		let ct_price =
			remainder_funding_project.get_project_details().weighted_average_price.expect("CT Price should exist");

		let project_id = remainder_funding_project.get_project_id();

		let contributions: TestContributions = vec![
			TestContribution::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BOB, OVERBUY_CT, 1u8, AcceptedFundingAsset::USDT),
		];
		let mut plmc_fundings: UserToPLMCBalance = calculate_contributed_plmc_spent(contributions.clone(), ct_price);
		plmc_fundings.push((BOB, get_ed()));
		let mut statemint_asset_fundings: UserToStatemintAsset =
			calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

		test_env.mint_plmc_to(plmc_fundings.clone());
		test_env.mint_statemint_asset_to(statemint_asset_fundings.clone());

		// Buy remaining CTs
		remainder_funding_project
			.buy_for_any_user(contributions)
			.expect("The Buyer should be able to buy the exact amount of remaining CTs");
		test_env.advance_time(2u64).unwrap();

		// Check remaining CTs is 0
		assert_eq!(
			remainder_funding_project.get_project_details().remaining_contribution_tokens,
			0,
			"There are still remaining CTs"
		);

		// Check project is in FundingEnded state
		assert_eq!(remainder_funding_project.get_project_details().status, ProjectStatus::FundingSuccessful);

		let reserved_plmc = plmc_fundings.swap_remove(0).1;
		let remaining_plmc: BalanceOf<TestRuntime> = plmc_fundings.iter().fold(0_u128, |acc, (_, amount)| acc + amount);

		let actual_funding_transferred = statemint_asset_fundings.swap_remove(0).1;
		let remaining_statemint_assets: BalanceOf<TestRuntime> =
			statemint_asset_fundings.iter().fold(0_u128, |acc, (_, amount, _)| acc + amount);

		test_env.do_free_plmc_assertions(vec![(BOB, remaining_plmc)]);
		test_env.do_free_statemint_asset_assertions(vec![(
			BOB,
			remaining_statemint_assets,
			AcceptedFundingAsset::USDT.to_statemint_id(),
		)]);
		test_env.do_reserved_plmc_assertions(vec![(BOB, reserved_plmc)], LockType::Participation(project_id));
		test_env.do_contribution_transferred_statemint_asset_assertions(
			vec![(BOB, actual_funding_transferred, AcceptedFundingAsset::USDT.to_statemint_id())],
			remainder_funding_project.get_project_id(),
		);
	}

	#[test]
	fn ct_minted_for_remainder_buys_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations =
			vec![(EVALUATOR_1, 50_000 * PLMC), (EVALUATOR_2, 25_000 * PLMC), (EVALUATOR_3, 32_000 * PLMC)];
		let bids = vec![
			TestBid::new(BIDDER_1, 50000 * ASSET_UNIT, 18_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40000 * ASSET_UNIT, 15_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = vec![
			TestContribution::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions.clone(),
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&test_env,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 50000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	fn ct_minted_for_community_buys_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations =
			vec![(EVALUATOR_1, 50_000 * PLMC), (EVALUATOR_2, 25_000 * PLMC), (EVALUATOR_3, 32_000 * PLMC)];
		let bids = vec![
			TestBid::new(BIDDER_1, 50000 * ASSET_UNIT, 18_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40000 * ASSET_UNIT, 15_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = vec![
			TestContribution::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions.clone(),
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = test_env.in_ext(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution.clone(), buyer_contribution.clone(), bidder_contribution.clone()]
		});
		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap()
			});
		}

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&test_env,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 50000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_twice_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations =
			vec![(EVALUATOR_1, 50_000 * PLMC), (EVALUATOR_2, 25_000 * PLMC), (EVALUATOR_3, 32_000 * PLMC)];
		let bids = vec![
			TestBid::new(BIDDER_1, 50000 * ASSET_UNIT, 18_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40000 * ASSET_UNIT, 15_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = vec![
			TestContribution::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions.clone(),
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = test_env.in_ext(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution.clone(), buyer_contribution.clone(), bidder_contribution.clone()]
		});
		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				assert_noop!(
					Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					),
					Error::<TestRuntime>::CannotClaimYet
				);
			})
		}
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
				Pallet::<TestRuntime>::contribution_ct_mint_for(
					RuntimeOrigin::signed(contribution.contributor),
					project_id,
					contribution.contributor,
					contribution.id,
				)
				.unwrap();

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

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&test_env,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 50000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}
	}

	#[test]
	pub fn cannot_mint_ct_manually_after_automatic_mint() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations =
			vec![(EVALUATOR_1, 50_000 * PLMC), (EVALUATOR_2, 25_000 * PLMC), (EVALUATOR_3, 32_000 * PLMC)];
		let bids = vec![
			TestBid::new(BIDDER_1, 50000 * ASSET_UNIT, 18_u128.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40000 * ASSET_UNIT, 15_u128.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let community_contributions = vec![
			TestContribution::new(BUYER_1, 100 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 200 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_2, 300 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 4000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions.clone(),
		);
		let project_id = finished_project.get_project_id();
		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);

		let stored_contributions = test_env.in_ext(|| {
			let evaluator_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, EVALUATOR_2)).next().unwrap();
			let buyer_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BUYER_2)).next().unwrap();
			let bidder_contribution =
				Contributions::<TestRuntime>::iter_prefix_values((project_id, BIDDER_1)).next().unwrap();
			vec![evaluator_contribution.clone(), buyer_contribution.clone(), bidder_contribution.clone()]
		});
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let evaluator_2_reward = extract_from_event!(
			&test_env,
			Event::<TestRuntime>::EvaluationRewarded { evaluator: EVALUATOR_2, amount, .. },
			amount
		)
		.unwrap();

		let total_remainder_participant_ct_amounts = vec![
			(EVALUATOR_2, 300 * ASSET_UNIT + evaluator_2_reward),
			(BUYER_2, 600 * ASSET_UNIT + 200 * ASSET_UNIT),
			(BIDDER_1, 50000 * ASSET_UNIT + 4000 * ASSET_UNIT),
		];
		for (contributor, amount) in total_remainder_participant_ct_amounts {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, contributor));
			assert_eq!(minted, amount);
		}

		for contribution in stored_contributions.clone() {
			test_env.in_ext(|| {
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
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_automatically() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		let price = finished_project.get_project_details().weighted_average_price.unwrap();
		let auction_locked_plmc = calculate_auction_plmc_spent_after_price_calculation(bids, price);
		let community_locked_plmc = calculate_contributed_plmc_spent(community_contributions, price);
		let remainder_locked_plmc = calculate_contributed_plmc_spent(remainder_contributions, price);
		let all_plmc_locks =
			merge_add_mappings_by_user(vec![auction_locked_plmc, community_locked_plmc, remainder_locked_plmc]);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		for (user, amount) in all_plmc_locks {
			let schedule = test_env.in_ext(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&user,
					LockType::Participation(finished_project.project_id),
				)
			});

			assert_eq!(schedule.unwrap(), amount);
		}
	}

	#[test]
	pub fn plmc_vesting_schedule_starts_manually() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		let details = finished_project.get_project_details();
		assert_eq!(details.status, ProjectStatus::FundingSuccessful);
		assert_eq!(details.cleanup, Cleaner::NotReady);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

		let contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		for contribution in contributions {
			let prev_scheduled = test_env
				.in_ext(|| {
					<TestRuntime as Config>::Vesting::total_scheduled_amount(
						&contribution.contributor,
						LockType::Participation(finished_project.project_id),
					)
				})
				.unwrap_or(Zero::zero());

			call_and_is_ok!(
				test_env,
				Pallet::<TestRuntime>::start_contribution_vesting_schedule_for(
					RuntimeOrigin::signed(contribution.contributor),
					finished_project.project_id,
					contribution.contributor,
					contribution.id,
				)
			);

			let post_scheduled = test_env
				.in_ext(|| {
					<TestRuntime as Config>::Vesting::total_scheduled_amount(
						&contribution.contributor,
						LockType::Participation(finished_project.project_id),
					)
				})
				.unwrap();

			let new_scheduled = post_scheduled - prev_scheduled;

			let contribution = test_env.in_ext(|| {
				Contributions::<TestRuntime>::get((
					finished_project.project_id,
					contribution.contributor,
					contribution.id,
				))
				.unwrap()
			});
			assert_eq!(new_scheduled, contribution.plmc_vesting_info.unwrap().total_amount);
		}
	}

	#[test]
	pub fn plmc_vesting_full_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		let bid_plmc_balances =
			stored_bids.into_iter().map(|b| (b.bidder, b.plmc_vesting_info.unwrap().total_amount)).collect::<Vec<_>>();
		let contributed_plmc_balances = stored_contributions
			.into_iter()
			.map(|c| (c.contributor, c.plmc_vesting_info.unwrap().total_amount))
			.collect::<Vec<_>>();

		let merged_plmc_balances = generic_map_merge_reduce(
			vec![contributed_plmc_balances.clone(), bid_plmc_balances.clone()],
			|(account, _amount)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_account, amount), total| total + amount,
		);
		test_env.advance_time((1 * DAYS + 1u32).into()).unwrap();

		for (contributor, plmc_amount) in merged_plmc_balances {
			let prev_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(
						contributor.clone(),
						finished_project.project_id,
						contributor.clone(),
					)
				})
				.unwrap();

			let post_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			assert_eq!(plmc_amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn plmc_vesting_partial_amount() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(15u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
		let vest_start_block = details.funding_end_block.unwrap();

		let stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());
		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		let now = test_env.current_block();

		let bid_plmc_balances = stored_bids
			.into_iter()
			.map(|b| {
				(b.bidder, {
					let blocks_vested = min(b.plmc_vesting_info.unwrap().duration, now - vest_start_block);
					b.plmc_vesting_info.unwrap().amount_per_block * blocks_vested as u128
				})
			})
			.collect::<Vec<_>>();
		let contributed_plmc_balances = stored_contributions
			.into_iter()
			.map(|c| {
				(c.contributor, {
					let blocks_vested = min(c.plmc_vesting_info.unwrap().duration, now - vest_start_block);
					c.plmc_vesting_info.unwrap().amount_per_block * blocks_vested as u128
				})
			})
			.collect::<Vec<_>>();

		let merged_plmc_balances = generic_map_merge_reduce(
			vec![contributed_plmc_balances.clone(), bid_plmc_balances.clone()],
			|(account, _amount)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_account, amount), total| total + amount,
		);

		for (contributor, amount) in merged_plmc_balances {
			let prev_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));

			test_env
				.in_ext(|| {
					Pallet::<TestRuntime>::do_vest_plmc_for(contributor, finished_project.project_id, contributor)
				})
				.unwrap();

			let post_free_balance = test_env.in_ext(|| <TestRuntime as Config>::NativeCurrency::balance(&contributor));
			assert_eq!(amount, post_free_balance - prev_free_balance);
		}
	}

	#[test]
	pub fn remainder_contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = vec![];
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn community_contribution_remainder_contribution_and_bid_funding_assets_are_paid_automatically_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn remainder_contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = vec![];
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_winning_bids = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
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
			test_env
				.in_ext(|| {
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
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}

	#[test]
	pub fn remainder_contribution_community_contribution_and_bid_funding_assets_are_paid_manually_to_issuer() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = default_bids();
		let community_contributions = default_community_buys();
		let remainder_contributions = default_remainder_buys();

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		let project_id = finished_project.get_project_id();

		let final_winning_bids = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let final_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.collect::<UserToStatemintAsset>()
		});
		let final_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		let final_contribution_payouts = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|contribution| {
					(
						contribution.contributor,
						contribution.funding_asset_amount,
						contribution.funding_asset.to_statemint_id(),
					)
				})
				.collect::<UserToStatemintAsset>()
		});

		let total_expected_bid_payout =
			final_bid_payouts.iter().map(|bid| bid.1.clone()).sum::<BalanceOf<TestRuntime>>();
		let total_expected_contribution_payout = final_contribution_payouts
			.iter()
			.map(|contribution| contribution.1.clone())
			.sum::<BalanceOf<TestRuntime>>();

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let prev_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		for bid in final_winning_bids {
			test_env
				.in_ext(|| {
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
			test_env
				.in_ext(|| {
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
			test_env.get_free_statemint_asset_balances_for(final_bid_payouts[0].2, vec![issuer.clone()])[0].1;

		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			final_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;
		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
		let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

		assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
		assert_eq!(issuer_funding_delta, project_pot_funding_delta);

		assert_eq!(post_project_pot_funding_balance, 0u128);
	}
}

mod remainder_round_failure {
	use super::*;

	#[test]
	pub fn bids_and_community_and_remainder_contribution_funding_assets_are_released_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 130_400 * ASSET_UNIT, 3u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = finished_project.get_project_details().weighted_average_price.unwrap();
		let project_id = finished_project.get_project_id();
		let expected_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});
		let expected_community_contribution_payouts =
			calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
		let expected_remainder_contribution_payouts =
			calculate_contributed_funding_asset_spent(remainder_contributions.clone(), final_price);
		let all_expected_payouts = generic_map_merge_reduce(
			vec![
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			|(account, _amount, _asset)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_acc_1, amount, _asset), s| amount + s,
		);

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let bidders = bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>();
		let community_contributors = community_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let remainder_contributors = remainder_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let all_participants = generic_map_merge(
			vec![bidders, community_contributors, remainder_contributors],
			|account| account.clone(),
			|acc_1, _acc_2| acc_1.clone(),
		);
		let prev_participants_funding_balances =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, all_participants.clone());

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_participants_funding_balances =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, all_participants);
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			expected_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let all_participants_funding_delta = generic_map_merge_reduce(
			vec![prev_participants_funding_balances, post_participants_funding_balances],
			|(acc, _, _)| acc.clone(),
			Zero::zero(),
			|(_acc_1, balance_1, _), s| balance_1 + s,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(all_expected_payouts, all_participants_funding_delta);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_funding_assets_are_released_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 130_400 * ASSET_UNIT, 3u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = finished_project.get_project_details().weighted_average_price.unwrap();
		let project_id = finished_project.get_project_id();
		let expected_bid_payouts = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.map(|bid| (bid.bidder, bid.funding_asset_amount_locked, bid.funding_asset.to_statemint_id()))
				.sorted()
				.collect::<UserToStatemintAsset>()
		});
		let expected_community_contribution_payouts =
			calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
		let expected_remainder_contribution_payouts =
			calculate_contributed_funding_asset_spent(remainder_contributions.clone(), final_price);
		let all_expected_payouts = generic_map_merge_reduce(
			vec![
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			|(account, _amount, _asset)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_acc_1, amount, _asset), s| amount + s,
		);

		let prev_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let bidders = bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>();
		let community_contributors = community_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let remainder_contributors = remainder_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let all_participants = generic_map_merge(
			vec![bidders, community_contributors, remainder_contributors],
			|account| account.clone(),
			|acc_1, _acc_2| acc_1.clone(),
		);
		let prev_participants_funding_balances =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, all_participants.clone());

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		let stored_bids = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		for bid in stored_bids {
			call_and_is_ok!(
				test_env,
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
				test_env,
				Pallet::<TestRuntime>::release_contribution_funds_for(
					RuntimeOrigin::signed(issuer),
					project_id,
					contribution.contributor,
					contribution.id,
				)
			)
		}

		let post_issuer_funding_balance =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, vec![issuer.clone()])[0].1;
		let post_participants_funding_balances =
			test_env.get_free_statemint_asset_balances_for(expected_bid_payouts[0].2, all_participants);
		let post_project_pot_funding_balance = test_env.get_free_statemint_asset_balances_for(
			expected_bid_payouts[0].2,
			vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
		)[0]
		.1;

		let all_participants_funding_delta = generic_map_merge_reduce(
			vec![prev_participants_funding_balances, post_participants_funding_balances],
			|(acc, _, _)| acc.clone(),
			Zero::zero(),
			|(_acc_1, balance_1, _), s| balance_1 + s,
		);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(post_project_pot_funding_balance, 0u128);
		assert_eq!(all_expected_payouts, all_participants_funding_delta);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_plmc_bonded_is_returned_automatically_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = vec![
			(EVALUATOR_1, 50_000 * US_DOLLAR),
			(EVALUATOR_2, 25_000 * US_DOLLAR),
			(EVALUATOR_3, 32_000 * US_DOLLAR),
		];
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 130_400 * ASSET_UNIT, 3u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = finished_project.get_project_details().weighted_average_price.unwrap();
		let project_id = finished_project.get_project_id();
		let expected_evaluator_contributor_return =
			calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				vec![(EVALUATOR_1, 50_000 * US_DOLLAR)],
				vec![TestContribution::new(
					EVALUATOR_1,
					250 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				)],
				final_price,
				true,
			);
		let expected_bid_payouts = calculate_auction_plmc_spent_after_price_calculation(bids.clone(), final_price);
		let expected_community_contribution_payouts =
			calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		let expected_remainder_contribution_payouts = calculate_contributed_plmc_spent(
			vec![
				TestContribution::new(
					BIDDER_1,
					130_400 * ASSET_UNIT,
					3u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			],
			final_price,
		);
		let all_expected_payouts = generic_map_merge_reduce(
			vec![
				expected_evaluator_contributor_return,
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			|(account, _amount)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_acc_1, amount), s| amount + s,
		);

		let prev_issuer_funding_balance = test_env.get_free_plmc_balances_for(vec![issuer.clone()])[0].1;
		let bidders = bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>();
		let community_contributors = community_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let remainder_contributors = remainder_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let all_participants = generic_map_merge(
			vec![bidders, community_contributors, remainder_contributors],
			|account| account.clone(),
			|acc_1, _acc_2| acc_1.clone(),
		);
		let prev_participants_plmc_balances = test_env.get_free_plmc_balances_for(all_participants.clone());

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		test_env.advance_time(10).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let post_issuer_funding_balance = test_env.get_free_plmc_balances_for(vec![issuer.clone()])[0].1;
		let post_participants_plmc_balances = test_env.get_free_plmc_balances_for(all_participants);

		let all_participants_plmc_deltas =
			merge_subtract_mappings_by_user(post_participants_plmc_balances, vec![prev_participants_plmc_balances]);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_expected_payouts, all_participants_plmc_deltas);
	}

	#[test]
	pub fn bids_and_community_and_remainder_contribution_plmc_bonded_is_returned_manually_on_funding_fail() {
		let test_env = TestEnvironment::new();
		let project = default_project(test_env.get_new_nonce());
		let issuer = ISSUER;
		let evaluations = vec![
			(EVALUATOR_1, 50_000 * US_DOLLAR),
			(EVALUATOR_2, 25_000 * US_DOLLAR),
			(EVALUATOR_3, 32_000 * US_DOLLAR),
		];
		let bids = generate_bids_from_total_usd(project.total_allocation_size / 2, project.minimum_price);

		let community_contributions = vec![
			TestContribution::new(BUYER_1, 1_000 * ASSET_UNIT, 2u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_2, 500 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_3, 73 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];
		let remainder_contributions = vec![
			TestContribution::new(EVALUATOR_1, 250 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BIDDER_1, 130_400 * ASSET_UNIT, 3u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
		];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);
		let final_price = finished_project.get_project_details().weighted_average_price.unwrap();
		let project_id = finished_project.get_project_id();
		let expected_evaluator_contributor_return =
			calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
				vec![(EVALUATOR_1, 50_000 * US_DOLLAR)],
				vec![TestContribution::new(
					EVALUATOR_1,
					250 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				)],
				final_price,
				true,
			);
		let expected_bid_payouts = calculate_auction_plmc_spent_after_price_calculation(bids.clone(), final_price);
		let expected_community_contribution_payouts =
			calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
		let expected_remainder_contribution_payouts = calculate_contributed_plmc_spent(
			vec![
				TestContribution::new(
					BIDDER_1,
					130_400 * ASSET_UNIT,
					3u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				),
				TestContribution::new(BUYER_1, 42 * ASSET_UNIT, 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			],
			final_price,
		);
		let all_expected_payouts = generic_map_merge_reduce(
			vec![
				expected_evaluator_contributor_return,
				expected_bid_payouts.clone(),
				expected_community_contribution_payouts,
				expected_remainder_contribution_payouts,
			],
			|(account, _amount)| account.clone(),
			BalanceOf::<TestRuntime>::zero(),
			|(_acc_1, amount), s| amount + s,
		);

		let prev_issuer_funding_balance = test_env.get_free_plmc_balances_for(vec![issuer.clone()])[0].1;
		let bidders = bids.iter().map(|bid| bid.bidder.clone()).collect::<Vec<_>>();
		let community_contributors = community_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let remainder_contributors = remainder_contributions
			.iter()
			.map(|test_contribution| test_contribution.contributor.clone())
			.collect::<Vec<_>>();
		let all_participants = generic_map_merge(
			vec![bidders, community_contributors, remainder_contributors],
			|account| account.clone(),
			|acc_1, _acc_2| acc_1.clone(),
		);
		let prev_participants_plmc_balances = test_env.get_free_plmc_balances_for(all_participants.clone());

		call_and_is_ok!(
			test_env,
			Pallet::<TestRuntime>::decide_project_outcome(
				RuntimeOrigin::signed(issuer),
				project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
		assert_eq!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_evaluations = test_env.in_ext(|| {
			Evaluations::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});
		let stored_bids = test_env.in_ext(|| {
			Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect::<Vec<_>>()
		});
		let stored_contributions = test_env.in_ext(|| {
			Contributions::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>()
		});

		for evaluation in stored_evaluations {
			call_and_is_ok!(
				test_env,
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
				test_env,
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
				test_env,
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

		let post_issuer_funding_balance = test_env.get_free_plmc_balances_for(vec![issuer.clone()])[0].1;
		let post_participants_plmc_balances = test_env.get_free_plmc_balances_for(all_participants);

		let all_participants_plmc_deltas =
			merge_subtract_mappings_by_user(post_participants_plmc_balances, vec![prev_participants_plmc_balances]);

		let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

		assert_eq!(issuer_funding_delta, 0);
		assert_eq!(all_expected_payouts, all_participants_plmc_deltas);
	}
}

mod funding_end {
	use super::*;

	#[test]
	fn automatic_fail_less_eq_33_percent() {
		for funding_percent in (1..=33).step_by(5) {
			let test_env = TestEnvironment::new();
			let project_metadata = default_project(test_env.get_new_nonce());
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids =
				generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
			let contributions = generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
			);
			let finished_project = FinishedProject::new_with(
				&test_env,
				project_metadata,
				ISSUER,
				evaluations,
				bids,
				contributions,
				vec![],
			);
			assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingFailed);
		}
	}

	#[test]
	fn automatic_success_bigger_eq_90_percent() {
		for funding_percent in (90..=100).step_by(2) {
			let test_env = TestEnvironment::new();
			let project_metadata = default_project(test_env.get_new_nonce());
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids =
				generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
			let contributions = generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
			);
			let finished_project = FinishedProject::new_with(
				&test_env,
				project_metadata,
				ISSUER,
				evaluations,
				bids,
				contributions,
				vec![],
			);
			assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingSuccessful);
		}
	}

	#[test]
	fn manual_outcome_above33_to_below90() {
		for funding_percent in (34..90).step_by(5) {
			let test_env = TestEnvironment::new();
			let project_metadata = default_project(test_env.get_new_nonce());
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids =
				generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
			let contributions = generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
			);
			let finished_project = FinishedProject::new_with(
				&test_env,
				project_metadata,
				ISSUER,
				evaluations,
				bids,
				contributions,
				vec![],
			);
			assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);
		}
	}

	#[test]
	fn manual_acceptance() {
		let test_env = TestEnvironment::new();
		let project_metadata = default_project(test_env.get_new_nonce());
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let contributions =
			generate_contributions_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let finished_project =
			FinishedProject::new_with(&test_env, project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);

		let project_id = finished_project.project_id;
		test_env
			.in_ext(|| {
				FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::AcceptFunding)
			})
			.unwrap();

		test_env.advance_time(1u64).unwrap();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingSuccessful);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		assert_matches!(finished_project.get_project_details().cleanup, Cleaner::Success(CleanerState::Initialized(_)));
		test_ct_created_for(&test_env, project_id);

		test_env.advance_time(10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn manual_rejection() {
		let test_env = TestEnvironment::new();
		let project_metadata = default_project(test_env.get_new_nonce());
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let contributions =
			generate_contributions_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let finished_project =
			FinishedProject::new_with(&test_env, project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);

		let project_id = finished_project.project_id;
		test_env
			.in_ext(|| {
				FundingModule::do_decide_project_outcome(ISSUER, project_id, FundingOutcomeDecision::RejectFunding)
			})
			.unwrap();

		test_env.advance_time(1u64).unwrap();

		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingFailed);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		test_ct_not_created_for(&test_env, project_id);

		test_env.advance_time(10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn automatic_acceptance_on_manual_decision_after_time_delta() {
		let test_env = TestEnvironment::new();
		let project_metadata = default_project(test_env.get_new_nonce());
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(55) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = generate_bids_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let contributions =
			generate_contributions_from_total_usd(Percent::from_percent(50u8) * twenty_percent_funding_usd, min_price);
		let finished_project =
			FinishedProject::new_with(&test_env, project_metadata, ISSUER, evaluations, bids, contributions, vec![]);
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);

		let project_id = finished_project.project_id;
		test_env.advance_time(1u64 + <TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingSuccessful);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
		test_ct_created_for(&test_env, project_id);

		test_env.advance_time(10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);
	}

	#[test]
	fn evaluators_get_slashed_funding_accepted() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::from_funding_reached(&test_env, 43u64);
		let project_id = finished_project.get_project_id();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);

		let old_evaluation_locked_plmc: UserToPLMCBalance = test_env
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|(_acc, amount)| amount > &Zero::zero())
			.collect::<Vec<_>>();

		let evaluators = old_evaluation_locked_plmc.iter().map(|(acc, _)| acc.clone()).collect::<Vec<_>>();

		let old_participation_locked_plmc =
			test_env.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc: UserToPLMCBalance = test_env.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			test_env,
			FundingModule::do_decide_project_outcome(
				ISSUER,
				finished_project.project_id,
				FundingOutcomeDecision::AcceptFunding
			)
		);
		test_env.advance_time(1u64).unwrap();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingSuccessful);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Success(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = merge_add_mappings_by_user(vec![
			slashed_evaluation_locked_plmc,
			old_participation_locked_plmc,
			old_free_plmc,
		]);

		let actual_evaluator_free_balances = test_env.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_funding_rejected() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::from_funding_reached(&test_env, 56u64);
		let project_id = finished_project.get_project_id();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::AwaitingProjectDecision);

		let old_evaluation_locked_plmc: UserToPLMCBalance = test_env
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|(_acc, amount)| amount > &Zero::zero())
			.collect::<Vec<_>>();

		let evaluators = old_evaluation_locked_plmc.iter().map(|(acc, _)| acc.clone()).collect::<Vec<_>>();

		let old_participation_locked_plmc =
			test_env.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc: UserToPLMCBalance = test_env.get_free_plmc_balances_for(evaluators.clone());

		call_and_is_ok!(
			test_env,
			FundingModule::do_decide_project_outcome(
				ISSUER,
				finished_project.project_id,
				FundingOutcomeDecision::RejectFunding
			)
		);
		test_env.advance_time(1u64).unwrap();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingFailed);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = merge_add_mappings_by_user(vec![
			slashed_evaluation_locked_plmc,
			old_participation_locked_plmc,
			old_free_plmc,
		]);

		let actual_evaluator_free_balances = test_env.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn evaluators_get_slashed_funding_failed() {
		let test_env = TestEnvironment::new();
		let finished_project = FinishedProject::from_funding_reached(&test_env, 24u64);
		let project_id = finished_project.get_project_id();
		assert_eq!(finished_project.get_project_details().status, ProjectStatus::FundingFailed);

		let old_evaluation_locked_plmc: UserToPLMCBalance = test_env
			.get_all_reserved_plmc_balances(LockType::Evaluation(project_id))
			.into_iter()
			.filter(|(_acc, amount)| amount > &Zero::zero())
			.collect::<Vec<_>>();

		let evaluators = old_evaluation_locked_plmc.iter().map(|(acc, _)| acc.clone()).collect::<Vec<_>>();

		let old_participation_locked_plmc =
			test_env.get_reserved_plmc_balances_for(evaluators.clone(), LockType::Participation(project_id));
		let old_free_plmc: UserToPLMCBalance = test_env.get_free_plmc_balances_for(evaluators.clone());

		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
		assert_matches!(
			finished_project.get_project_details().cleanup,
			Cleaner::Failure(CleanerState::Finished(PhantomData))
		);

		let slashed_evaluation_locked_plmc = slash_evaluator_balances(old_evaluation_locked_plmc);
		let expected_evaluator_free_balances = merge_add_mappings_by_user(vec![
			slashed_evaluation_locked_plmc,
			old_participation_locked_plmc,
			old_free_plmc,
		]);

		let actual_evaluator_free_balances = test_env.get_free_plmc_balances_for(evaluators.clone());

		assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
	}

	#[test]
	fn multiplier_gets_correct_vesting_duration() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = default_project(test_env.get_new_nonce());
		let evaluations = default_evaluations();
		let bids = vec![
			TestBid::new(BIDDER_1, 40_000 * ASSET_UNIT, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_2, 40_000 * ASSET_UNIT, 15.into(), 1u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_3,
				152_000 * ASSET_UNIT,
				11.into(),
				2u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(BIDDER_4, 20_000 * ASSET_UNIT, 17.into(), 3u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_5, 9_000 * ASSET_UNIT, 18.into(), 19u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_6, 1_000 * ASSET_UNIT, 18.into(), 20u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(BIDDER_7, 8_000 * ASSET_UNIT, 18.into(), 24u8.try_into().unwrap(), AcceptedFundingAsset::USDT),
			TestBid::new(
				BIDDER_8,
				68_000 * ASSET_UNIT,
				18.into(),
				25u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
		];
		let community_contributions = default_community_buys();
		let remainder_contributions = vec![];

		let finished_project = FinishedProject::new_with(
			&test_env,
			project,
			issuer,
			evaluations,
			bids,
			community_contributions,
			remainder_contributions,
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10u64).unwrap();
		let details = finished_project.get_project_details();
		assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

		let mut stored_bids = test_env
			.in_ext(|| Bids::<TestRuntime>::iter_prefix_values((finished_project.project_id,)).collect::<Vec<_>>());

		stored_bids.sort_by_key(|bid| bid.bidder);
		let one_week_in_blocks = DAYS * 7;
		assert_eq!(stored_bids[0].plmc_vesting_info.unwrap().duration, 1u64);
		assert_eq!(stored_bids[1].plmc_vesting_info.unwrap().duration, 1u64);
		assert_eq!(
			stored_bids[2].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(2167, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[3].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(4334, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[4].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(39006, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[5].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(41173, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[6].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(49841, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
		assert_eq!(
			stored_bids[7].plmc_vesting_info.unwrap().duration,
			FixedU128::from_rational(52008, 1000).saturating_mul_int(one_week_in_blocks as u64)
		);
	}
}

mod test_helper_functions {
	use super::*;

	#[test]
	fn calculate_evaluation_plmc_spent() {
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1u64;
		const USD_AMOUNT_1: u128 = 150_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 17_857_1_428_571_428_u128;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2u64;
		const USD_AMOUNT_2: u128 = 50_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 5_952_3_809_523_809_u128;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3u64;
		const USD_AMOUNT_3: u128 = 75_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 8_928_5_714_285_714_u128;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4u64;
		const USD_AMOUNT_4: u128 = 100_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 11_9_047_619_047_u128;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5u64;
		const USD_AMOUNT_5: u128 = 123_7_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 14_7_261_904_761_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID.into()).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let evaluations = vec![
			(EVALUATOR_1, USD_AMOUNT_1),
			(EVALUATOR_2, USD_AMOUNT_2),
			(EVALUATOR_3, USD_AMOUNT_3),
			(EVALUATOR_4, USD_AMOUNT_4),
			(EVALUATOR_5, USD_AMOUNT_5),
		];

		let expected_plmc_spent = vec![
			(EVALUATOR_1, EXPECTED_PLMC_AMOUNT_1),
			(EVALUATOR_2, EXPECTED_PLMC_AMOUNT_2),
			(EVALUATOR_3, EXPECTED_PLMC_AMOUNT_3),
			(EVALUATOR_4, EXPECTED_PLMC_AMOUNT_4),
			(EVALUATOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = super::helper_functions::calculate_evaluation_plmc_spent(evaluations);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_auction_plmc_spent() {
		const BIDDER_1: AccountIdOf<TestRuntime> = 1u64;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const PRICE_PER_TOKEN_1: f64 = 0.3f64;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 36_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 4_2_857_142_857_u128;

		const BIDDER_2: AccountIdOf<TestRuntime> = 2u64;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const PRICE_PER_TOKEN_2: f64 = 13f64;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 65_299_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 3_886_8_452_380_952_u128;

		const BIDDER_3: AccountIdOf<TestRuntime> = 3u64;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const PRICE_PER_TOKEN_3: f64 = 20f64;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 400_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_801_1_204_481_792_u128;

		const BIDDER_4: AccountIdOf<TestRuntime> = 4u64;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const PRICE_PER_TOKEN_4: f64 = 5.52f64;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_USD_4: u128 = 5_520_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 26_285_7_142_857_142_u128;

		const BIDDER_5: AccountIdOf<TestRuntime> = 5u64;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const PRICE_PER_TOKEN_5: f64 = 11.34f64;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_USD_5: u128 = 1_3_982_220_000_u128;
		// TODO: Is this due to rounding errors?
		// Should be in reality 0.0166455, but we get 0.0166454999. i.e error of 0.0000000001 PLMC
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_166_454_999_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID.into()).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let bids = vec![
			TestBid::new(
				BIDDER_1,
				TOKEN_AMOUNT_1,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_1),
				MULTIPLIER_1,
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(
				BIDDER_2,
				TOKEN_AMOUNT_2,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_2),
				MULTIPLIER_2,
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(
				BIDDER_3,
				TOKEN_AMOUNT_3,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_3),
				MULTIPLIER_3,
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(
				BIDDER_4,
				TOKEN_AMOUNT_4,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_4),
				MULTIPLIER_4,
				AcceptedFundingAsset::USDT,
			),
			TestBid::new(
				BIDDER_5,
				TOKEN_AMOUNT_5,
				PriceOf::<TestRuntime>::from_float(PRICE_PER_TOKEN_5),
				MULTIPLIER_5,
				AcceptedFundingAsset::USDT,
			),
		];

		let expected_plmc_spent = vec![
			(BIDDER_1, EXPECTED_PLMC_AMOUNT_1),
			(BIDDER_2, EXPECTED_PLMC_AMOUNT_2),
			(BIDDER_3, EXPECTED_PLMC_AMOUNT_3),
			(BIDDER_4, EXPECTED_PLMC_AMOUNT_4),
			(BIDDER_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = super::helper_functions::calculate_auction_plmc_spent(bids);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_contributed_plmc_spent() {
		const PLMC_PRICE: f64 = 8.4f64;
		const CT_PRICE: f64 = 16.32f64;

		const CONTRIBUTOR_1: AccountIdOf<TestRuntime> = 1u64;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 1_958_4_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 233_1_428_571_428_u128;

		const CONTRIBUTOR_2: AccountIdOf<TestRuntime> = 2u64;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 81_975_3_600_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 4_879_4_857_142_857_u128;

		const CONTRIBUTOR_3: AccountIdOf<TestRuntime> = 3u64;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 326_400_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_285_7_142_857_142_u128;

		const CONTRIBUTOR_4: AccountIdOf<TestRuntime> = 4u64;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_4: u128 = 16_320_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 77_714_2_857_142_857_u128;

		const CONTRIBUTOR_5: AccountIdOf<TestRuntime> = 5u64;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_5: u128 = 2_0_122_562_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_239_554_285_u128;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_STATEMINT_ID.into()).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let contributions = vec![
			TestContribution::new(CONTRIBUTOR_1, TOKEN_AMOUNT_1, MULTIPLIER_1, AcceptedFundingAsset::USDT),
			TestContribution::new(CONTRIBUTOR_2, TOKEN_AMOUNT_2, MULTIPLIER_2, AcceptedFundingAsset::USDT),
			TestContribution::new(CONTRIBUTOR_3, TOKEN_AMOUNT_3, MULTIPLIER_3, AcceptedFundingAsset::USDT),
			TestContribution::new(CONTRIBUTOR_4, TOKEN_AMOUNT_4, MULTIPLIER_4, AcceptedFundingAsset::USDT),
			TestContribution::new(CONTRIBUTOR_5, TOKEN_AMOUNT_5, MULTIPLIER_5, AcceptedFundingAsset::USDT),
		];

		let expected_plmc_spent = vec![
			(CONTRIBUTOR_1, EXPECTED_PLMC_AMOUNT_1),
			(CONTRIBUTOR_2, EXPECTED_PLMC_AMOUNT_2),
			(CONTRIBUTOR_3, EXPECTED_PLMC_AMOUNT_3),
			(CONTRIBUTOR_4, EXPECTED_PLMC_AMOUNT_4),
			(CONTRIBUTOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = super::helper_functions::calculate_contributed_plmc_spent(
			contributions,
			PriceOf::<TestRuntime>::from_float(CT_PRICE),
		);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn test_calculate_price_from_test_bids() {
		let bids = vec![
			TestBid::new(100, 10_000_0_000_000_000, 15.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(200, 20_000_0_000_000_000, 20.into(), 1u8, AcceptedFundingAsset::USDT),
			TestBid::new(300, 20_000_0_000_000_000, 10.into(), 1u8, AcceptedFundingAsset::USDT),
		];
		let price = calculate_price_from_test_bids(bids);
		let price_in_10_decimals = price.checked_mul_int(1_0_000_000_000_u128).unwrap();

		assert_eq!(price_in_10_decimals, 16_3_333_333_333_u128.into());
	}
}

mod misc_features {
	use super::*;

	#[test]
	fn remove_from_update_store_works() {
		let test_env = TestEnvironment::new();
		let now = test_env.current_block();
		test_env.ext_env.borrow_mut().execute_with(|| {
			FundingModule::add_to_update_store(now + 10u64, (&42u32, CommunityFundingStart));
			FundingModule::add_to_update_store(now + 20u64, (&69u32, RemainderFundingStart));
			FundingModule::add_to_update_store(now + 5u64, (&404u32, RemainderFundingStart));
		});
		test_env.advance_time(2u64).unwrap();
		test_env.ext_env.borrow_mut().execute_with(|| {
			let stored = ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored.len(), 3, "There should be 3 blocks scheduled for updating");

			FundingModule::remove_from_update_store(&69u32).unwrap();

			let stored = ProjectsToUpdate::<TestRuntime>::iter_values().collect::<Vec<_>>();
			assert_eq!(stored[2], vec![], "Vector should be empty for that block after deletion");
		});
	}

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

	#[test]
	fn sandbox() {
		assert!(true);
	}
}

mod testing_macros {
	macro_rules! assert_close_enough {
		($real:expr, $desired:expr, $max_approximation:expr) => {
			let real_parts = Perquintill::from_rational($real, $desired);
			let one = Perquintill::from_percent(100u64);
			let real_approximation = one - real_parts;
			assert!(real_approximation <= $max_approximation);
		};
	}
	pub(crate) use assert_close_enough;

	macro_rules! call_and_is_ok {
		($env: expr, $( $call: expr ),* ) => {
			$env.ext_env.borrow_mut().execute_with(|| {
				$(
					let result = $call;
					assert!(result.is_ok(), "Call failed: {:?}", result);
				)*
			})
		};
	}
	pub(crate) use call_and_is_ok;

	#[allow(unused_macros)]
	macro_rules! find_event {
		($env: expr, $pattern:pat) => {
			$env.ext_env.borrow_mut().execute_with(|| {
				let events = System::events();

				events.iter().find_map(|event_record| {
					if let frame_system::EventRecord {gi
						event: RuntimeEvent::FundingModule(desired_event @ $pattern),
						..
					} = event_record
					{
						Some(desired_event.clone())
					} else {
						None
					}
				})
			})
		};
	}
	#[allow(unused_imports)]
	pub(crate) use find_event;

	macro_rules! extract_from_event {
		($env: expr, $pattern:pat, $field:ident) => {
			$env.ext_env.borrow_mut().execute_with(|| {
				let events = System::events();

				events.iter().find_map(|event_record| {
					if let frame_system::EventRecord { event: RuntimeEvent::FundingModule($pattern), .. } = event_record
					{
						Some($field.clone())
					} else {
						None
					}
				})
			})
		};
	}
	pub(crate) use extract_from_event;

	macro_rules! define_names {
		($($name:ident: $id:expr, $label:expr);* $(;)?) => {
			$(
				pub const $name: AccountId = $id;
			)*

			pub fn names() -> HashMap<AccountId, &'static str> {
				let mut names = HashMap::new();
				$(
					names.insert($name, $label);
				)*
				names
			}
		};
	}
	pub(crate) use define_names;
}

mod e2e_testing {

	use itertools::Itertools;
	use sp_arithmetic::traits::CheckedSub;
	use sp_runtime::traits::AccountIdConversion;

	use super::{testing_macros::define_names, *};

	define_names! {
		// In order to auto-incriment the ids, we have to use unsafe Rust.
		LINA: 101, "Lina";
		MIA: 102, "Mia";
		ALEXEY: 103, "Alexey";
		PAUL: 104, "Paul";
		MARIA: 105, "Maria";
		GEORGE: 106, "George";
		CLARA: 107, "Clara";
		RAMONA: 108, "Ramona";
		PASCAL: 109, "Pascal";
		EMMA: 110, "Emma";
		BIBI: 111, "Bibi";
		AHMED: 112, "Ahmed";
		HERBERT: 113, "Herbert";
		LENI: 114, "Leni";
		XI: 115, "Xi";
		TOM: 116, "Tom";
		ADAMS: 117, "Adams";
		POLK: 118, "Polk";
		MARKUS: 119, "Markus";
		ELLA: 120, "Ella";
		SKR: 121, "Skr";
		ARTHUR: 122, "Arthur";
		MILA: 123, "Mila";
		LINCOLN: 124, "Lincoln";
		MONROE: 125, "Monroe";
		ARBRESHA: 126, "Arbresha";
		ELDIN: 127, "Eldin";
		HARDING: 128, "Harding";
		SOFIA: 129, "Sofia";
		DOMINIK: 130, "Dominik";
		NOLAND: 131, "Noland";
		HANNAH: 132, "Hannah";
		HOOVER: 133, "Hoover";
		GIGI: 134, "Gigi";
		JEFFERSON: 135, "Jefferson";
		LINDI: 136, "Lindi";
		KEVIN: 137, "Kevin";
		ANIS: 138, "Anis";
		RETO: 139, "Reto";
		HAALAND: 140, "Haaland";
		XENIA: 141, "Xenia";
		EVA: 142, "Eva";
		SKARA: 143, "Skara";
		ROOSEVELT: 144, "Roosevelt";
		DRACULA: 145, "Dracula";
		DURIM: 146, "Durim";
		HARRISON: 147, "Harrison";
		DRIN: 148, "Drin";
		PARI: 149, "Pari";
		TUTI: 150, "Tuti";
		BENITO: 151, "Benito";
		VANESSA: 152, "Vanessa";
		ENES: 153, "Enes";
		RUDOLF: 154, "Rudolf";
		CERTO: 155, "Certo";
		TIESTO: 156, "Tiesto";
		DAVID: 157, "David";
		ATAKAN: 158, "Atakan";
		YANN: 159, "Yann";
		ENIS: 160, "Enis";
		ALFREDO: 161, "Alfredo";
		QENDRIM: 162, "Qendrim";
		LEONARDO: 163, "Leonardo";
		KEN: 164, "Ken";
		LUCA: 165, "Luca";
		FLAVIO: 167, "Flavio";
		FREDI: 168, "Fredi";
		ALI: 169, "Ali";
		DILARA: 170, "Dilara";
		DAMIAN: 171, "Damian";
		KAYA: 172, "Kaya";
		IAZI: 173, "Iazi";
		CHRIGI: 174, "Chrigi";
		VALENTINA: 175, "Valentina";
		ALMA: 176, "Alma";
		ALENA: 177, "Alena";
		PATRICK: 178, "Patrick";
		ONTARIO: 179, "Ontario";
		RAKIA: 180, "Rakia";
		HUBERT: 181, "Hubert";
		UTUS: 182, "Utus";
		TOME: 183, "Tome";
		ZUBER: 184, "Zuber";
		ADAM: 185, "Adam";
		STANI: 186, "Stani";
		BETI: 187, "Beti";
		HALIT: 188, "Halit";
		DRAGAN: 189, "Dragan";
		LEA: 190, "Lea";
		LUIS: 191, "Luis";
		TATI: 192, "Tati";
		WEST: 193, "West";
		MIRIJAM: 194, "Mirijam";
		LIONEL: 195, "Lionel";
		GIOVANNI: 196, "Giovanni";
		JOEL: 197, "Joel";
		POLKA: 198, "Polk";
		MALIK: 199, "Malik";
		ALEXANDER: 201, "Alexander";
		SOLOMUN: 203, "Solomun";
		JOHNNY: 204, "Johnny";
		GRINGO: 205, "Gringo";
		JONAS: 206, "Jonas";
		BUNDI: 207, "Bundi";
		FELIX: 208, "Felix";
	}

	fn excel_evaluators() -> UserToUSDBalance {
		vec![
			(LINA, 93754 * US_DOLLAR),
			(MIA, 162 * US_DOLLAR),
			(ALEXEY, 7454 * US_DOLLAR),
			(PAUL, 8192 * US_DOLLAR),
			(MARIA, 11131 * US_DOLLAR),
			(GEORGE, 4765 * US_DOLLAR),
			(CLARA, 4363 * US_DOLLAR),
			(RAMONA, 4120 * US_DOLLAR),
			(PASCAL, 1626 * US_DOLLAR),
			(EMMA, 3996 * US_DOLLAR),
			(BIBI, 3441 * US_DOLLAR),
			(AHMED, 8048 * US_DOLLAR),
			(HERBERT, 2538 * US_DOLLAR),
			(LENI, 5803 * US_DOLLAR),
			(XI, 1669 * US_DOLLAR),
			(TOM, 6526 * US_DOLLAR),
		]
	}

	fn excel_bidders() -> TestBids {
		vec![
			TestBid::from(ADAMS, 700 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(POLK, 4000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(MARKUS, 3000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(ELLA, 700 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(SKR, 3400 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(ARTHUR, 1000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(MILA, 8400 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(LINCOLN, 800 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(MONROE, 1300 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(ARBRESHA, 5000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(ELDIN, 600 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(HARDING, 800 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(SOFIA, 3000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(DOMINIK, 8000 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(NOLAND, 900 * ASSET_UNIT, 10_u128.into()),
			TestBid::from(LINA, 9400 * ASSET_UNIT, 11_u128.into()),
			TestBid::from(HANNAH, 400 * ASSET_UNIT, 11_u128.into()),
			TestBid::from(HOOVER, 2000 * ASSET_UNIT, 11_u128.into()),
			TestBid::from(GIGI, 600 * ASSET_UNIT, 11_u128.into()),
			TestBid::from(JEFFERSON, 3000 * ASSET_UNIT, 12_u128.into()),
		]
	}

	fn excel_contributors() -> TestContributions {
		vec![
			TestContribution::from(DRIN, 692 * US_DOLLAR),
			TestContribution::from(PARI, 236 * US_DOLLAR),
			TestContribution::from(TUTI, 24 * US_DOLLAR),
			TestContribution::from(BENITO, 688 * US_DOLLAR),
			TestContribution::from(VANESSA, 33 * US_DOLLAR),
			TestContribution::from(ENES, 1148 * US_DOLLAR),
			TestContribution::from(RUDOLF, 35 * US_DOLLAR),
			TestContribution::from(CERTO, 840 * US_DOLLAR),
			TestContribution::from(TIESTO, 132 * US_DOLLAR),
			TestContribution::from(DAVID, 21 * US_DOLLAR),
			TestContribution::from(ATAKAN, 59 * US_DOLLAR),
			TestContribution::from(YANN, 89 * US_DOLLAR),
			TestContribution::from(ENIS, 332 * US_DOLLAR),
			TestContribution::from(ALFREDO, 8110 * US_DOLLAR),
			TestContribution::from(QENDRIM, 394 * US_DOLLAR),
			TestContribution::from(LEONARDO, 840 * US_DOLLAR),
			TestContribution::from(KEN, 352 * US_DOLLAR),
			TestContribution::from(LUCA, 640 * US_DOLLAR),
			// TODO: XI is a partipant in the Community Round AND an Evaluator. At the moment, this returns `InsufficientBalance` because it seems we don't mint to him enough USDT.
			// To be addressed and tested in a separate PR.
			// TestContribution::from(XI, 588 * US_DOLLAR),
			TestContribution::from(FLAVIO, 792 * US_DOLLAR),
			TestContribution::from(FREDI, 993 * US_DOLLAR),
			TestContribution::from(ALI, 794 * US_DOLLAR),
			TestContribution::from(DILARA, 256 * US_DOLLAR),
			TestContribution::from(DAMIAN, 431 * US_DOLLAR),
			TestContribution::from(KAYA, 935 * US_DOLLAR),
			TestContribution::from(IAZI, 174 * US_DOLLAR),
			TestContribution::from(CHRIGI, 877 * US_DOLLAR),
			TestContribution::from(VALENTINA, 961 * US_DOLLAR),
			TestContribution::from(ALMA, 394 * US_DOLLAR),
			TestContribution::from(ALENA, 442 * US_DOLLAR),
			TestContribution::from(PATRICK, 486 * US_DOLLAR),
			TestContribution::from(ONTARIO, 17 * US_DOLLAR),
			TestContribution::from(RAKIA, 9424 * US_DOLLAR),
			TestContribution::from(HUBERT, 14 * US_DOLLAR),
			TestContribution::from(UTUS, 4906 * US_DOLLAR),
			TestContribution::from(TOME, 68 * US_DOLLAR),
			TestContribution::from(ZUBER, 9037 * US_DOLLAR),
			TestContribution::from(ADAM, 442 * US_DOLLAR),
			TestContribution::from(STANI, 40 * US_DOLLAR),
			TestContribution::from(BETI, 68 * US_DOLLAR),
			TestContribution::from(HALIT, 68 * US_DOLLAR),
			TestContribution::from(DRAGAN, 98 * US_DOLLAR),
			TestContribution::from(LEA, 17 * US_DOLLAR),
			TestContribution::from(LUIS, 422 * US_DOLLAR),
			TestContribution::from(TATI, 228 * US_DOLLAR),
			TestContribution::from(WEST, 695 * US_DOLLAR),
			TestContribution::from(MIRIJAM, 498 * US_DOLLAR),
			TestContribution::from(LIONEL, 864 * US_DOLLAR),
			TestContribution::from(GIOVANNI, 306 * US_DOLLAR),
		]
	}

	fn excel_remainders() -> TestContributions {
		vec![
			TestContribution::from(JOEL, 692 * US_DOLLAR),
			TestContribution::from(POLK, 236 * US_DOLLAR),
			TestContribution::from(MALIK, 24 * US_DOLLAR),
			TestContribution::from(LEA, 688 * US_DOLLAR),
			TestContribution::from(ALEXANDER, 33 * US_DOLLAR),
			TestContribution::from(BUNDI, 1148 * US_DOLLAR),
			TestContribution::from(RAMONA, 35 * US_DOLLAR),
			TestContribution::from(SOLOMUN, 840 * US_DOLLAR),
			TestContribution::from(JOHNNY, 132 * US_DOLLAR),
			TestContribution::from(GRINGO, 21 * US_DOLLAR),
			TestContribution::from(JONAS, 59 * US_DOLLAR),
			TestContribution::from(FELIX, 89 * US_DOLLAR),
		]
	}

	fn excel_ct_amounts() -> UserToCTBalance {
		vec![
			(ADAMS, 6920000000000, 0),
			(POLK, 4720000000000, 0),
			(MARKUS, 240000000000, 0),
			(ELLA, 6880000000000, 0),
			(SKR, 330000000000, 0),
			(ARTHUR, 11480000000000, 0),
			(MILA, 350000000000, 0),
			(LINCOLN, 8400000000000, 0),
			(MONROE, 1320000000000, 0),
			(ARBRESHA, 210000000000, 0),
			(ELDIN, 590000000000, 0),
			(HARDING, 890000000000, 0),
			(SOFIA, 3320000000000, 0),
			(DOMINIK, 81100000000000, 0),
			(NOLAND, 170000000000, 0),
			(LINA, 111403137276535, 0),
			(HANNAH, 140000000000, 0),
			(HOOVER, 49060000000000, 0),
			(GIGI, 680000000000, 0),
			(JEFFERSON, 90370000000000, 0),
			(LINDI, 4420000000000, 0),
			(KEVIN, 400000000000, 0),
			(ANIS, 680000000000, 0),
			(RETO, 680000000000, 0),
			(HAALAND, 980000000000, 0),
			(XENIA, 170000000000, 0),
			(EVA, 4220000000000, 0),
			(SKARA, 6150000000000, 0),
			(ROOSEVELT, 650000000000, 0),
			(DRACULA, 58630000000000, 0),
			(DURIM, 560000000000, 0),
			(HARRISON, 360000000000, 0),
			(DRIN, 6920000000000, 0),
			(PARI, 2360000000000, 0),
			(TUTI, 240000000000, 0),
			(BENITO, 6880000000000, 0),
			(VANESSA, 330000000000, 0),
			(ENES, 11480000000000, 0),
			(RUDOLF, 350000000000, 0),
			(CERTO, 8400000000000, 0),
			(TIESTO, 1320000000000, 0),
			(DAVID, 210000000000, 0),
			(ATAKAN, 590000000000, 0),
			(YANN, 890000000000, 0),
			(ENIS, 3320000000000, 0),
			(ALFREDO, 81100000000000, 0),
			(QENDRIM, 3940000000000, 0),
			(LEONARDO, 8400000000000, 0),
			(KEN, 3520000000000, 0),
			(LUCA, 6400000000000, 0),
			(XI, 215322787417, 0), //EV
			(FLAVIO, 7920000000000, 0),
			(FREDI, 9930000000000, 0),
			(ALI, 7940000000000, 0),
			(DILARA, 2560000000000, 0),
			(DAMIAN, 4310000000000, 0),
			(KAYA, 9350000000000, 0),
			(IAZI, 1740000000000, 0),
			(CHRIGI, 8770000000000, 0),
			(VALENTINA, 9610000000000, 0),
			(ALMA, 3940000000000, 0),
			(ALENA, 4420000000000, 0),
			(PATRICK, 4860000000000, 0),
			(ONTARIO, 170000000000, 0),
			(RAKIA, 94240000000000, 0),
			(HUBERT, 140000000000, 0),
			(UTUS, 49060000000000, 0),
			(TOME, 680000000000, 0),
			(ZUBER, 90370000000000, 0),
			(ADAM, 4420000000000, 0),
			(STANI, 400000000000, 0),
			(BETI, 680000000000, 0),
			(HALIT, 680000000000, 0),
			(DRAGAN, 980000000000, 0),
			(LEA, 7050000000000, 0),
			(LUIS, 4220000000000, 0),
			(TATI, 2280000000000, 0),
			(WEST, 6950000000000, 0),
			(MIRIJAM, 4980000000000, 0),
			(LIONEL, 8640000000000, 0),
			(GIOVANNI, 3060000000000, 0),
			(JOEL, 6920000000000, 0),
			(MALIK, 240000000000, 0),
			(ALEXANDER, 330000000000, 0),
			(BUNDI, 11480000000000, 0),
			(RAMONA, 881533783198, 0),
			(SOLOMUN, 8400000000000, 0),
			(JOHNNY, 1320000000000, 0),
			(GRINGO, 210000000000, 0),
			(JONAS, 590000000000, 0),
			(FELIX, 890000000000, 0),
			(MIA, 29656635864, 0),
			(ALEXEY, 1290519320002, 0),
			(PAUL, 1056874939797, 0),
			(MARIA, 1436044306015, 0),
			(GEORGE, 614747203141, 0),
			(CLARA, 562883955363, 0),
			(PASCAL, 209775226088, 0),
			(EMMA, 515536164481, 0),
			(BIBI, 443933919414, 0),
			(AHMED, 1038297059995, 0),
			(HERBERT, 327435131495, 0),
			(LENI, 748662753374, 0),
			(TOM, 841939191542, 0),
		]
	}

	#[test]
	fn evaluation_round_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = excel_project(test_env.get_new_nonce());
		let evaluations = excel_evaluators();

		AuctioningProject::new_with(&test_env, project, issuer, evaluations);
	}

	#[test]
	fn auction_round_simple_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = excel_project(test_env.get_new_nonce());
		let evaluations = excel_evaluators();
		let bids = excel_bidders();
		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let metadata = community_funding_project.get_project_metadata();
		let total_allocation_size = metadata.total_allocation_size.clone();
		dbg!(total_allocation_size);
		test_env.in_ext(|| {
			let bids = Bids::<TestRuntime>::iter_prefix_values((0,)).sorted_by_key(|bid| bid.id).collect_vec();

			for bid in bids.clone() {
				if bid.bidder == 135 {
					// dbg!(bid);
				}
			}
			let total_participation = bids.into_iter().fold(0, |acc, bid| acc + bid.final_ct_amount);
			dbg!(total_participation);
		})
	}

	#[test]
	fn auction_round_completed() {
		let test_env = TestEnvironment::new();
		let issuer = ISSUER;
		let project = excel_project(test_env.get_new_nonce());
		let evaluations = excel_evaluators();
		let bids = excel_bidders();
		let community_funding_project =
			CommunityFundingProject::new_with(&test_env, project, issuer, evaluations, bids);
		let wavgp_from_excel = 10.1;
		// Convert the float to a FixedU128
		let wavgp_to_substrate = FixedU128::from_float(wavgp_from_excel);
		let wavgp_from_chain = community_funding_project.get_project_details().weighted_average_price.unwrap();
		let res = wavgp_from_chain.checked_sub(&wavgp_to_substrate).unwrap();
		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.00001.
		assert!(res < FixedU128::from_float(0.00001));
		let names = names();

		test_env.in_ext(|| {
			let bids = Bids::<TestRuntime>::iter_prefix_values((0,)).sorted_by_key(|bid| bid.bidder).collect_vec();

			for bid in bids.clone() {
				println!("{}: {}", names[&bid.bidder], bid.funding_asset_amount_locked);
			}
			let total_participation = bids.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount_locked);
			dbg!(total_participation);
		})
	}

	#[test]
	fn community_round_completed() {
		let test_env = TestEnvironment::new();
		let _ = RemainderFundingProject::new_with(
			&test_env,
			excel_project(0),
			ISSUER,
			excel_evaluators(),
			excel_bidders(),
			excel_contributors(),
		);

		test_env.in_ext(|| {
			let contributions = Contributions::<TestRuntime>::iter_prefix_values((0,))
				.sorted_by_key(|bid| bid.contributor)
				.collect_vec();
			let total_contribution = contributions.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
			let total_contribution_as_fixed = FixedU128::from_rational(total_contribution, PLMC);
			dbg!(total_contribution_as_fixed);
			let total_from_excel = 825070.0361;
			let total_to_substrate = FixedU128::from_float(total_from_excel);
			dbg!(total_to_substrate);
			let res = total_contribution_as_fixed.checked_sub(&total_to_substrate).unwrap();
			// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.00001.
			assert!(res < FixedU128::from_float(0.00001));
		})
	}

	#[test]
	fn remainder_round_completed() {
		let test_env = TestEnvironment::new();
		let _ = FinishedProject::new_with(
			&test_env,
			excel_project(0),
			ISSUER,
			excel_evaluators(),
			excel_bidders(),
			excel_contributors(),
			excel_remainders(),
		);

		test_env.in_ext(|| {
			let contributions = Contributions::<TestRuntime>::iter_prefix_values((0,))
				.sorted_by_key(|bid| bid.contributor)
				.collect_vec();
			let total_contributions = contributions.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
			let total_contributions_as_fixed = FixedU128::from_rational(total_contributions, PLMC);
			dbg!(total_contributions_as_fixed);
			let total_from_excel = 891811.0086;
			let total_to_substrate = FixedU128::from_float(total_from_excel);
			dbg!(total_to_substrate);
			let res = total_contributions_as_fixed.checked_sub(&total_to_substrate).unwrap();
			// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0001.
			assert!(res < FixedU128::from_float(0.0001));
		})
	}

	#[test]
	fn funds_raised() {
		let test_env = TestEnvironment::new();
		let _ = FinishedProject::new_with(
			&test_env,
			excel_project(0),
			ISSUER,
			excel_evaluators(),
			excel_bidders(),
			excel_contributors(),
			excel_remainders(),
		);

		test_env.in_ext(|| {
			let pallet_id = <mock::TestRuntime as pallet::Config>::PalletId::get();
			let project_specific_account: u64 = pallet_id.into_sub_account_truncating(0);
			let funding = StatemintAssets::balance(1984, project_specific_account);
			let fund_raised_from_excel = 1565629.731;
			let fund_raised_to_substrate = FixedU128::from_float(fund_raised_from_excel);
			let fund_raised_as_fixed = FixedU128::from_rational(funding, ASSET_UNIT);
			let res = fund_raised_to_substrate.checked_sub(&fund_raised_as_fixed).unwrap();
			// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0003.
			assert!(res < FixedU128::from_float(0.0003));
		})
	}

	#[test]
	fn ct_minted() {
		let test_env = TestEnvironment::new();
		let _ = FinishedProject::new_with(
			&test_env,
			excel_project(0),
			ISSUER,
			excel_evaluators(),
			excel_bidders(),
			excel_contributors(),
			excel_remainders(),
		);
		test_env.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		test_env.advance_time(10).unwrap();

		for (contributor, expected_amount, project_id) in excel_ct_amounts() {
			let minted = test_env
				.in_ext(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
			assert_eq!(minted, expected_amount);
		}
	}
}
