use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold, Mutate as FungibleMutate},
		fungibles::{
			metadata::Inspect as MetadataInspect, roles::Inspect as RolesInspect, Inspect as FungiblesInspect,
			Mutate as FungiblesMutate,
		},
		Get, OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
	Parameter,
};

use sp_arithmetic::Perquintill;

use itertools::Itertools;
use parity_scale_codec::Decode;
use sp_arithmetic::{
	traits::{SaturatedConversion, Saturating, Zero},
	FixedPointNumber, Percent,
};
use sp_core::H256;
use sp_runtime::{
	traits::{Member, One},
	DispatchError,
};
use sp_std::{
	cell::RefCell,
	collections::{btree_map::*, btree_set::*},
	iter::zip,
	marker::PhantomData,
	prelude::*,
};

use crate::{
	traits::{BondingRequirementCalculation, ProvideStatemintPrice},
	AcceptedFundingAsset, AccountIdOf, AssetIdOf, AuctionPhase, BalanceOf, BidInfoOf, BidStatus, Bids, BlockNumberOf,
	BlockNumberPair, Cleaner, CommStatus, Config, Contributions, Error, EvaluationInfoOf, EvaluationRoundInfoOf,
	EvaluatorsOutcome, Event, LockType, MultiplierOf, NextProjectId, PhaseTransitionPoints, PriceOf, ProjectDetailsOf,
	ProjectIdOf, ProjectMetadataOf, ProjectStatus, ProjectsDetails, ProjectsMetadata, ProjectsToUpdate, RewardInfoOf,
	UpdateType, VestingInfoOf, PLMC_STATEMINT_ID,
};

pub use testing_macros::*;
pub type RuntimeOriginOf<T> = <T as frame_system::Config>::RuntimeOrigin;

pub struct BoxToFunction(pub Box<dyn FnOnce()>);
impl Default for BoxToFunction {
	fn default() -> Self {
		BoxToFunction(Box::new(|| ()))
	}
}
pub struct Instantiator<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
> {
	#[cfg(all(feature = "std", not(feature = "testing-node")))]
	ext: Option<RefCell<sp_io::TestExternalities>>,
	#[cfg(not(all(feature = "std", not(feature = "testing-node"))))]
	ext: Option<()>,
	nonce: RefCell<u64>,
	_marker: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
}

// general chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
where
// AccountIdOf<T>: Into<<RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
// <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	pub fn new(
		#[cfg(all(feature = "std", not(feature = "testing-node")))] ext: Option<RefCell<sp_io::TestExternalities>>,
		#[cfg(not(all(feature = "std", not(feature = "testing-node"))))] ext: Option<()>,
	) -> Self {
		Self { ext, nonce: RefCell::new(0u64), _marker: PhantomData }
	}

	pub fn set_ext(
		&mut self,
		#[cfg(all(feature = "std", not(feature = "testing-node")))] ext: Option<RefCell<sp_io::TestExternalities>>,
		#[cfg(not(all(feature = "std", not(feature = "testing-node"))))] ext: Option<()>,
	) {
		self.ext = ext;
	}

	pub fn execute<R>(&mut self, execution: impl FnOnce() -> R) -> R {
		#[cfg(all(feature = "std", not(feature = "testing-node")))]
		if let Some(ext) = &self.ext {
			return ext.borrow_mut().execute_with(execution)
		}

		execution()
	}

	pub fn get_new_nonce(&self) -> u64 {
		let nonce = self.nonce.borrow_mut().clone();
		self.nonce.replace(nonce + 1);
		nonce
	}

	pub fn get_free_plmc_balances_for(&mut self, user_keys: Vec<AccountIdOf<T>>) -> Vec<UserToPLMCBalance<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToPLMCBalance<T>> = Vec::new();
			for account in user_keys {
				let plmc_amount = <T as Config>::NativeCurrency::balance(&account);
				balances.push(UserToPLMCBalance { account, plmc_amount });
			}
			balances.sort_by_key(|a| a.account.clone());
			balances
		})
	}

	pub fn get_reserved_plmc_balances_for(
		&mut self,
		user_keys: Vec<AccountIdOf<T>>,
		lock_type: LockType<ProjectIdOf<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToPLMCBalance<T>> = Vec::new();
			for account in user_keys {
				let plmc_amount = <T as Config>::NativeCurrency::balance_on_hold(&lock_type, &account);
				balances.push(UserToPLMCBalance { account, plmc_amount });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_free_statemint_asset_balances_for(
		&mut self,
		asset_id: AssetIdOf<T>,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<UserToStatemintAsset<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToStatemintAsset<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::FundingCurrency::balance(asset_id.clone(), &account);
				balances.push(UserToStatemintAsset { account, asset_amount, asset_id });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_ct_asset_balances_for(
		&mut self,
		project_id: ProjectIdOf<T>,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<BalanceOf<T>> {
		self.execute(|| {
			let mut balances: Vec<BalanceOf<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::ContributionTokenCurrency::balance(project_id.into(), &account);
				balances.push(asset_amount);
			}
			balances
		})
	}

	pub fn get_all_free_plmc_balances(&mut self) -> Vec<UserToPLMCBalance<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_free_plmc_balances_for(user_keys)
	}

	pub fn get_all_reserved_plmc_balances(
		&mut self,
		reserve_type: LockType<ProjectIdOf<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_reserved_plmc_balances_for(user_keys, reserve_type)
	}

	pub fn get_all_free_statemint_asset_balances(&mut self, asset_id: AssetIdOf<T>) -> Vec<UserToStatemintAsset<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_free_statemint_asset_balances_for(asset_id, user_keys)
	}

	pub fn get_plmc_total_supply(&mut self) -> BalanceOf<T> {
		self.execute(|| <T as Config>::NativeCurrency::total_issuance())
	}

	pub fn do_reserved_plmc_assertions(
		&mut self,
		correct_funds: Vec<UserToPLMCBalance<T>>,
		reserve_type: LockType<ProjectIdOf<T>>,
	) {
		for UserToPLMCBalance { account, .. } in correct_funds {
			self.execute(|| {
				let _reserved = <T as Config>::NativeCurrency::balance_on_hold(&reserve_type, &account);
				// TODO: Enable this
				// assert_eq!(reserved, plmc_amount);
			});
		}
	}

	pub fn mint_plmc_to(&mut self, mapping: Vec<UserToPLMCBalance<T>>) {
		self.execute(|| {
			for UserToPLMCBalance { account, plmc_amount } in mapping {
				<T as Config>::NativeCurrency::mint_into(&account, plmc_amount).expect("Minting should work");
			}
		});
	}

	pub fn mint_statemint_asset_to(&mut self, mapping: Vec<UserToStatemintAsset<T>>) {
		self.execute(|| {
			for UserToStatemintAsset { account, asset_amount, asset_id } in mapping {
				<T as Config>::FundingCurrency::mint_into(asset_id, &account, asset_amount)
					.expect("Minting should work");
			}
		});
	}

	pub fn current_block(&mut self) -> BlockNumberOf<T> {
		self.execute(|| frame_system::Pallet::<T>::block_number())
	}

	pub fn advance_time(&mut self, amount: BlockNumberOf<T>) -> Result<(), DispatchError> {
		self.execute(|| {
			for _block in 0u32..amount.saturated_into() {
				let mut current_block = frame_system::Pallet::<T>::block_number();

				<AllPalletsWithoutSystem as OnFinalize<BlockNumberOf<T>>>::on_finalize(current_block);
				<frame_system::Pallet<T> as OnFinalize<BlockNumberOf<T>>>::on_finalize(current_block);

				<AllPalletsWithoutSystem as OnIdle<BlockNumberOf<T>>>::on_idle(current_block, Weight::MAX);
				<frame_system::Pallet<T> as OnIdle<BlockNumberOf<T>>>::on_idle(current_block, Weight::MAX);

				current_block += One::one();
				frame_system::Pallet::<T>::set_block_number(current_block);

				let pre_events = frame_system::Pallet::<T>::events();

				<frame_system::Pallet<T> as OnInitialize<BlockNumberOf<T>>>::on_initialize(current_block);
				<AllPalletsWithoutSystem as OnInitialize<BlockNumberOf<T>>>::on_initialize(current_block);

				let post_events = frame_system::Pallet::<T>::events();
				if post_events.len() > pre_events.len() {
					Self::err_if_on_initialize_failed(post_events)?;
				}
			}
			Ok(())
		})
	}

	pub fn do_free_plmc_assertions(&mut self, correct_funds: Vec<UserToPLMCBalance<T>>) {
		for UserToPLMCBalance { account, .. } in correct_funds {
			self.execute(|| {
				let _free = <T as Config>::NativeCurrency::balance(&account);
				// TODO: Enable this
				// assert_eq!(free, plmc_amount);
			});
		}
	}

	pub fn do_free_statemint_asset_assertions(&mut self, correct_funds: Vec<UserToStatemintAsset<T>>) {
		for UserToStatemintAsset { account, asset_amount, asset_id } in correct_funds {
			self.execute(|| {
				let real_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				assert_eq!(asset_amount, real_amount, "Wrong statemint asset balance expected for user {:?}", account);
			});
		}
	}

	pub fn do_bid_transferred_statemint_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToStatemintAsset<T>>,
		project_id: ProjectIdOf<T>,
	) {
		for UserToStatemintAsset { account, .. } in correct_funds {
			self.execute(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let _contribution_total: <T as Config>::Balance =
					Bids::<T>::iter_prefix_values((project_id, account.clone()))
						.map(|c| c.funding_asset_amount_locked)
						.fold(Zero::zero(), |a, b| a + b);
				// assert_eq!(
				// 	contribution_total, asset_amount,
				// 	"Wrong statemint asset balance expected for stored auction info on user {:?}",
				// 	account
				// );
			});
		}
	}

	// Check if a Contribution storage item exists for the given funding asset transfer
	pub fn do_contribution_transferred_statemint_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToStatemintAsset<T>>,
		project_id: ProjectIdOf<T>,
	) {
		for UserToStatemintAsset { account, asset_amount, .. } in correct_funds {
			self.execute(|| {
				Contributions::<T>::iter_prefix_values((project_id, account.clone()))
					.find(|c| c.funding_asset_amount == asset_amount)
					.expect("Contribution not found in storage");
			});
		}
	}
}

// assertions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
where
// AccountIdOf<T>: Into<<RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
// <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	pub fn test_ct_created_for(&mut self, project_id: ProjectIdOf<T>) {
		self.execute(|| {
			let metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
			let details = ProjectsDetails::<T>::get(project_id).unwrap();
			assert_eq!(
				<T as Config>::ContributionTokenCurrency::name(project_id),
				metadata.token_information.name.to_vec()
			);
			assert_eq!(<T as Config>::ContributionTokenCurrency::admin(project_id).unwrap(), details.issuer);
			assert_eq!(
				<T as Config>::ContributionTokenCurrency::total_issuance(project_id),
				0u32.into(),
				"No CTs should have been minted at this point"
			);
		});
	}

	pub fn test_ct_not_created_for(&mut self, project_id: ProjectIdOf<T>) {
		self.execute(|| {
			assert!(
				!<T as Config>::ContributionTokenCurrency::asset_exists(project_id),
				"Asset shouldn't exist, since funding failed"
			);
		});
	}

	pub fn creation_assertions(
		&mut self,
		project_id: ProjectIdOf<T>,
		expected_metadata: ProjectMetadataOf<T>,
		creation_start_block: BlockNumberOf<T>,
	) {
		let metadata = self.get_project_metadata(project_id);
		let details = self.get_project_details(project_id);
		let expected_details = ProjectDetailsOf::<T> {
			issuer: self.get_issuer(project_id),
			is_frozen: false,
			weighted_average_price: None,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair { start: Some(creation_start_block), end: None },
				..Default::default()
			},
			fundraising_target: expected_metadata
				.minimum_price
				.checked_mul_int(expected_metadata.total_allocation_size.0 + expected_metadata.total_allocation_size.1)
				.unwrap(),
			remaining_contribution_tokens: expected_metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<T>::zero(),
			cleanup: Cleaner::NotReady,
			evaluation_round_info: EvaluationRoundInfoOf::<T> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
			parachain_id: None,
			migration_readiness_check: None,
			comm_status: CommStatus {
				project_to_polimec: crate::ChannelStatus::Closed,
				polimec_to_project: crate::ChannelStatus::Closed,
			},
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	pub fn evaluation_assertions(
		&mut self,
		project_id: ProjectIdOf<T>,
		expected_free_plmc_balances: Vec<UserToPLMCBalance<T>>,
		expected_reserved_plmc_balances: Vec<UserToPLMCBalance<T>>,
		total_plmc_supply: BalanceOf<T>,
	) {
		let project_details = self.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_reserved_plmc_assertions(expected_reserved_plmc_balances, LockType::Evaluation(project_id));
		assert_eq!(self.get_plmc_total_supply(), total_plmc_supply)
	}

	#[allow(unused)]
	pub fn finalized_bids_assertions(
		&mut self,
		project_id: ProjectIdOf<T>,
		bid_expectations: Vec<BidInfoFilter<T>>,
		expected_ct_sold: BalanceOf<T>,
	) {
		let project_metadata = self.get_project_metadata(project_id);
		let project_details = self.get_project_details(project_id);
		let project_bids = self.execute(|| Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert!(matches!(project_details.weighted_average_price, Some(_)), "Weighted average price should exist");

		for filter in bid_expectations {
			let _found_bid = project_bids.iter().find(|bid| filter.matches_bid(&bid)).unwrap();
		}

		// Remaining CTs are updated
		assert_eq!(
			project_details.remaining_contribution_tokens.0,
			project_metadata.total_allocation_size.0 - expected_ct_sold,
			"Remaining CTs are incorrect"
		);
	}
}

// calculations
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
where
// AccountIdOf<T>: Into<<RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
// <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	pub fn get_ed() -> BalanceOf<T> {
		T::ExistentialDeposit::get()
	}

	pub fn calculate_evaluation_plmc_spent(evaluations: Vec<UserToUSDBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = Vec::new();
		for eval in evaluations {
			let usd_bond = eval.usd_amount;
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(eval.account, plmc_bond));
		}
		output
	}

	pub fn calculate_auction_plmc_spent(bids: Vec<BidParams<T>>) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = Vec::new();
		for bid in bids {
			let usd_ticket_size = bid.price.saturating_mul_int(bid.amount);
			let multiplier = bid.multiplier;
			let usd_bond = multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(bid.bidder, plmc_bond));
		}
		output
	}

	// This differs from `calculate_auction_plmc_spent` in that it recalculates bids over the average price as using that price.
	pub fn calculate_auction_plmc_spent_after_price_calculation(
		bids: Vec<BidParams<T>>,
		price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = Vec::new();
		for bid in bids {
			let final_price = if bid.price < price { bid.price } else { price };

			let usd_ticket_size = final_price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(bid.bidder, plmc_bond));
		}
		output
	}

	pub fn calculate_auction_funding_asset_spent(bids: Vec<BidParams<T>>) -> Vec<UserToStatemintAsset<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let asset_price = T::PriceProvider::get_price(bid.asset.to_statemint_id()).unwrap().clone();
			let usd_ticket_size = bid.price.saturating_mul_int(bid.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToStatemintAsset::new(bid.bidder, funding_asset_spent, bid.asset.to_statemint_id()));
		}
		output
	}

	pub fn calculate_contributed_plmc_spent(
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).unwrap().clone();
		let mut output = Vec::new();
		for cont in contributions {
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let usd_bond = cont.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(cont.contributor, plmc_bond));
		}
		output
	}

	pub fn calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
		evaluations: Vec<UserToUSDBalance<T>>,
		contributions: Vec<ContributionParams<T>>,
		price: PriceOf<T>,
		slashed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let evaluation_locked_plmc_amounts = Self::calculate_evaluation_plmc_spent(evaluations);
		// how much new plmc would be locked without considering evaluation bonds
		let theoretical_contribution_locked_plmc_amounts = Self::calculate_contributed_plmc_spent(contributions, price);

		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		let slashable_min_deposits = evaluation_locked_plmc_amounts
			.clone()
			.iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance {
				account: account.clone(),
				plmc_amount: slash_percentage * plmc_amount.clone(),
			})
			.collect::<Vec<_>>();
		let available_evaluation_locked_plmc_for_lock_transfer = Self::merge_subtract_mappings_by_user(
			evaluation_locked_plmc_amounts.clone(),
			vec![slashable_min_deposits.clone()],
		);

		// how much new plmc was actually locked, considering already evaluation bonds used first.
		let actual_contribution_locked_plmc_amounts = Self::generic_map_merge(
			vec![
				theoretical_contribution_locked_plmc_amounts.clone(),
				available_evaluation_locked_plmc_for_lock_transfer,
			],
			|user_to_plmc| user_to_plmc.account.clone(),
			|UserToPLMCBalance { account: acc_1, plmc_amount: contribution_amount },
			 UserToPLMCBalance { account: _acc_2, plmc_amount: evaluation_amount }| {
				if contribution_amount > evaluation_amount {
					UserToPLMCBalance::new(acc_1.clone(), *contribution_amount - *evaluation_amount)
				} else {
					UserToPLMCBalance::new(acc_1.clone(), Zero::zero())
				}
			},
		);
		let mut result = Self::merge_add_mappings_by_user(vec![
			evaluation_locked_plmc_amounts,
			actual_contribution_locked_plmc_amounts,
		]);

		if slashed {
			result = Self::merge_subtract_mappings_by_user(result, vec![slashable_min_deposits]);
		}

		result
	}

	pub fn calculate_contributed_funding_asset_spent(
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToStatemintAsset<T>> {
		let mut output = Vec::new();
		for cont in contributions {
			let asset_price = T::PriceProvider::get_price(cont.asset.to_statemint_id()).unwrap().clone();
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToStatemintAsset::new(cont.contributor, funding_asset_spent, cont.asset.to_statemint_id()));
		}
		output
	}

	/// add all the user -> I maps together, and add the I's of the ones with the same user.
	// Mappings should be sorted based on their account id, ascending.
	pub fn merge_add_mappings_by_user(mut mappings: Vec<Vec<UserToPLMCBalance<T>>>) -> Vec<UserToPLMCBalance<T>> {
		let mut output = mappings.swap_remove(0);
		output.sort_by_key(|k| k.account.clone());
		for mut map in mappings {
			map.sort_by_key(|k| k.account.clone());
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
					(
						Some(UserToPLMCBalance { account: acc_i, plmc_amount: val_i }),
						Some(UserToPLMCBalance { account: acc_j, plmc_amount: val_j }),
					) =>
						if acc_i == acc_j {
							output.push(UserToPLMCBalance::new(acc_i.clone(), val_i.clone() + val_j.clone()));
							i += 1;
							j += 1;
						} else if acc_i < acc_j {
							output.push(old_output[i].clone());
							i += 1;
						} else {
							output.push(map[j].clone());
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

	pub fn generic_map_subtract<M: Clone, K: Ord + Clone>(
		original_mapping: Vec<M>,
		mappings: Vec<Vec<M>>,
		key_extractor: impl Fn(&M) -> K,
		subtractor: impl Fn(&M, &M) -> M,
	) -> Vec<M> {
		let mut output = original_mapping;
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
							output.push(subtractor(m_i, m_j));
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
	pub fn merge_subtract_mappings_by_user(
		base_mapping: Vec<UserToPLMCBalance<T>>,
		subtract_mappings: Vec<Vec<UserToPLMCBalance<T>>>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = base_mapping;
		output.sort_by_key(|k| k.account.clone());
		for mut map in subtract_mappings {
			map.sort_by_key(|k| k.account.clone());
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
					(
						Some(UserToPLMCBalance { account: acc_i, plmc_amount: val_i }),
						Some(UserToPLMCBalance { account: acc_j, plmc_amount: val_j }),
					) => {
						if acc_i == acc_j {
							output.push(UserToPLMCBalance::new(
								acc_i.clone(),
								val_i.clone().saturating_sub(val_j.clone()),
							));
							i += 1;
							j += 1;
						} else if acc_i < acc_j {
							output.push(old_output[i].clone());
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

	pub fn sum_balance_mappings(mut mappings: Vec<Vec<UserToPLMCBalance<T>>>) -> BalanceOf<T> {
		let mut output = mappings
			.swap_remove(0)
			.into_iter()
			.map(|user_to_plmc| user_to_plmc.plmc_amount)
			.fold(Zero::zero(), |a, b| a + b);
		for map in mappings {
			output =
				output + map.into_iter().map(|user_to_plmc| user_to_plmc.plmc_amount).fold(Zero::zero(), |a, b| a + b);
		}
		output
	}

	pub fn sum_statemint_mappings(mut mappings: Vec<Vec<UserToStatemintAsset<T>>>) -> BalanceOf<T> {
		let mut output = mappings
			.swap_remove(0)
			.into_iter()
			.map(|user_to_asset| user_to_asset.asset_amount)
			.fold(Zero::zero(), |a, b| a + b);
		for map in mappings {
			output = output +
				map.into_iter().map(|user_to_asset| user_to_asset.asset_amount).fold(Zero::zero(), |a, b| a + b);
		}
		output
	}

	pub fn calculate_price_from_test_bids(bids: Vec<BidParams<T>>) -> PriceOf<T> {
		// temp variable to store the total value of the bids (i.e price * amount)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();

		for bid in bids.iter() {
			let ticket_size = bid.price.checked_mul_int(bid.amount).unwrap();
			bid_usd_value_sum.saturating_accrue(ticket_size);
		}

		bids.into_iter()
			.map(|bid| {
				let bid_weight = <PriceOf<T> as FixedPointNumber>::saturating_from_rational(
					bid.price.saturating_mul_int(bid.amount),
					bid_usd_value_sum,
				);
				bid.price * bid_weight
			})
			.reduce(|a, b| a.saturating_add(b))
			.unwrap()
	}

	pub fn panic_if_on_initialize_failed(events: Vec<frame_system::EventRecord<RuntimeEvent, H256>>) {
		let last_event_record = events.into_iter().last().expect("No events found for this action.");
		let last_event = last_event_record.event;
		let maybe_funding_event = last_event.try_into();
		if let Ok(funding_event) = maybe_funding_event {
			if let Event::TransitionError { project_id, error } = funding_event {
				panic!("Project {:?} transition failed in on_initialize: {:?}", project_id, error);
			}
		}
	}

	pub fn err_if_on_initialize_failed(
		events: Vec<frame_system::EventRecord<<T as frame_system::Config>::RuntimeEvent, T::Hash>>,
	) -> Result<(), Error<T>> {
		let last_event_record = events.into_iter().last().expect("No events found for this action.");
		let last_event = last_event_record.event;
		let maybe_funding_event = <T as Config>::RuntimeEvent::from(last_event).try_into();
		if let Ok(funding_event) = maybe_funding_event {
			if let Event::TransitionError { project_id: _, error } = funding_event {
				if let DispatchError::Module(module_error) = error {
					let pallet_error: Error<T> = Decode::decode(&mut &module_error.error[..]).unwrap();
					return Err(pallet_error)
				}
			}
		}
		Ok(())
	}

	pub fn generate_bids_from_total_usd(
		usd_amount: BalanceOf<T>,
		min_price: PriceOf<T>,
		weights: Vec<u8>,
		bidders: Vec<AccountIdOf<T>>,
	) -> Vec<BidParams<T>> {
		assert_eq!(weights.len(), bidders.len(), "Should have enough weights for all the bidders");

		zip(weights, bidders)
			.map(|(weight, bidder)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = min_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				BidParams::new(bidder, token_amount, min_price, 1u8, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn generate_contributions_from_total_usd(
		usd_amount: BalanceOf<T>,
		final_price: PriceOf<T>,
		weights: Vec<u8>,
		contributors: Vec<AccountIdOf<T>>,
	) -> Vec<ContributionParams<T>> {
		zip(weights, contributors)
			.map(|(weight, bidder)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = final_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				ContributionParams::new(bidder, token_amount, 1u8, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn slash_evaluator_balances(mut balances: Vec<UserToPLMCBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		for UserToPLMCBalance { account: _acc, plmc_amount: balance } in balances.iter_mut() {
			*balance -= slash_percentage * *balance;
		}
		balances
	}

	pub fn calculate_total_reward_for_evaluation(
		evaluation: EvaluationInfoOf<T>,
		reward_info: RewardInfoOf<T>,
	) -> BalanceOf<T> {
		let early_reward_weight =
			Perquintill::from_rational(evaluation.early_usd_amount, reward_info.early_evaluator_total_bonded_usd);
		let normal_reward_weight = Perquintill::from_rational(
			evaluation.late_usd_amount.saturating_add(evaluation.early_usd_amount),
			reward_info.normal_evaluator_total_bonded_usd,
		);
		let early_evaluators_rewards = early_reward_weight * reward_info.early_evaluator_reward_pot;
		let normal_evaluators_rewards = normal_reward_weight * reward_info.normal_evaluator_reward_pot;
		let total_reward_amount = early_evaluators_rewards.saturating_add(normal_evaluators_rewards);
		total_reward_amount.into()
	}
}

// project chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberOf<T>> + OnIdle<BlockNumberOf<T>> + OnInitialize<BlockNumberOf<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
where
// AccountIdOf<T>: Into<<RuntimeOriginOf<T> as OriginTrait>::AccountId> + sp_std::fmt::Debug,
// <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	pub fn get_issuer(&mut self, project_id: ProjectIdOf<T>) -> AccountIdOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).unwrap().issuer)
	}

	pub fn get_project_metadata(&mut self, project_id: ProjectIdOf<T>) -> ProjectMetadataOf<T> {
		self.execute(|| ProjectsMetadata::<T>::get(project_id).expect("Project metadata exists"))
	}

	pub fn get_project_details(&mut self, project_id: ProjectIdOf<T>) -> ProjectDetailsOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).expect("Project details exists"))
	}

	pub fn get_update_pair(&mut self, project_id: ProjectIdOf<T>) -> (BlockNumberOf<T>, UpdateType) {
		self.execute(|| {
			ProjectsToUpdate::<T>::iter()
				.find_map(|(block, update_vec)| {
					update_vec
						.iter()
						.find(|(pid, _update)| *pid == project_id)
						.map(|(_pid, update)| (block, update.clone()))
				})
				.unwrap()
		})
	}

	pub fn create_new_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectIdOf<T> {
		let now = self.current_block();
		self.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), Self::get_ed())]);
		self.execute(|| {
			crate::Pallet::<T>::do_create(&issuer, project_metadata.clone()).unwrap();
			let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
			log::trace!("Last project metadata: {:?}", last_project_metadata);
		});

		self.advance_time(10u32.into()).unwrap();
		// let created_project_id = self.execute(|| {
		// 	let events = frame_system::Pallet::<T>::events();
		// 	log::trace!("Events: {:?}", events);
		// 	let project_id = events
		// 		.iter()
		// 		.filter_map(|event| match RuntimeEvent::from(event.event.clone()).try_into() {
		// 			Ok(Event::ProjectCreated { project_id, issuer: _ }) => Some(project_id),
		// 			_ => None,
		// 		})
		// 		.last()
		// 		.expect("Project created event expected")
		// 		.clone();
		//
		// 	project_id
		// });
		let created_project_id = self.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
		self.creation_assertions(created_project_id, project_metadata, now);
		created_project_id
	}

	pub fn start_evaluation(
		&mut self,
		project_id: ProjectIdOf<T>,
		caller: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::Application);
		self.execute(|| crate::Pallet::<T>::do_evaluation_start(caller, project_id))?;
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::EvaluationRound);

		Ok(())
	}

	pub fn create_evaluating_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectIdOf<T> {
		let project_id = self.create_new_project(project_metadata.clone(), issuer.clone());
		self.start_evaluation(project_id, issuer).unwrap();
		project_id
	}

	pub fn bond_for_users(
		&mut self,
		project_id: ProjectIdOf<T>,
		bonds: Vec<UserToUSDBalance<T>>,
	) -> Result<(), DispatchError> {
		for UserToUSDBalance { account, usd_amount } in bonds {
			self.execute(|| crate::Pallet::<T>::do_evaluate(&account, project_id, usd_amount))?;
		}
		Ok(())
	}

	pub fn start_auction(&mut self, project_id: ProjectIdOf<T>, caller: AccountIdOf<T>) -> Result<(), DispatchError> {
		let project_details = self.get_project_details(project_id);

		if project_details.status == ProjectStatus::EvaluationRound {
			let evaluation_end = project_details.phase_transition_points.evaluation.end().unwrap();
			let auction_start = evaluation_end.saturating_add(2u32.into());
			let blocks_to_start = auction_start.saturating_sub(self.current_block());
			self.advance_time(blocks_to_start).unwrap();
		};

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		self.execute(|| crate::Pallet::<T>::do_english_auction(caller, project_id))?;

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));

		Ok(())
	}

	pub fn create_auctioning_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
	) -> ProjectIdOf<T> {
		let project_id = self.create_evaluating_project(project_metadata, issuer.clone());

		let evaluators = evaluations.accounts();
		let prev_supply = self.get_plmc_total_supply();
		let prev_plmc_balances = self.get_free_plmc_balances_for(evaluators.clone());

		let plmc_eval_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = evaluators.existential_deposits();

		let expected_remaining_plmc: Vec<UserToPLMCBalance<T>> =
			Self::merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		self.mint_plmc_to(plmc_eval_deposits.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());

		self.bond_for_users(project_id, evaluations).unwrap();

		let expected_evaluator_balances =
			Self::sum_balance_mappings(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

		let expected_total_supply = prev_supply + expected_evaluator_balances;

		self.evaluation_assertions(project_id, expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

		self.start_auction(project_id, issuer).unwrap();
		project_id
	}

	pub fn bid_for_users(&mut self, project_id: ProjectIdOf<T>, bids: Vec<BidParams<T>>) -> Result<(), DispatchError> {
		for bid in bids {
			self.execute(|| {
				crate::Pallet::<T>::do_bid(&bid.bidder, project_id, bid.amount, bid.price, bid.multiplier, bid.asset)
			})?;
		}
		Ok(())
	}

	pub fn start_community_funding(&mut self, project_id: ProjectIdOf<T>) -> Result<(), DispatchError> {
		let english_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.english_auction
			.end()
			.expect("English end point should exist");

		let candle_start = english_end + 2u32.into();
		let current_block = self.current_block();
		self.advance_time(candle_start.saturating_sub(current_block)).unwrap();
		let candle_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");

		let community_start = candle_end + 2u32.into();

		let current_block = self.current_block();
		self.advance_time(community_start.saturating_sub(current_block)).unwrap();

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::CommunityRound);

		Ok(())
	}

	pub fn create_community_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectIdOf<T> {
		let project_id = self.create_auctioning_project(project_metadata, issuer, evaluations.clone());

		if bids.is_empty() {
			panic!("Cannot start community funding without bids")
		}

		let bidders = bids.accounts();
		let asset_id = bids[0].asset.to_statemint_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(bidders.clone());
		let _prev_funding_asset_balances = self.get_free_statemint_asset_balances_for(asset_id, bidders.clone());
		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> =
			Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_auction_plmc_spent(bids.clone());
		let participation_usable_evaluation_deposits = plmc_evaluation_deposits
			.clone()
			.into_iter()
			.map(|mut x| {
				x.plmc_amount = x.plmc_amount.saturating_sub(<T as Config>::EvaluatorSlash::get() * x.plmc_amount);
				x
			})
			.collect::<Vec<UserToPLMCBalance<T>>>();
		let necessary_plmc_mint = Self::merge_subtract_mappings_by_user(
			plmc_bid_deposits.clone(),
			vec![participation_usable_evaluation_deposits],
		);
		let total_plmc_participation_locked = plmc_bid_deposits;
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bidders.existential_deposits();
		let funding_asset_deposits = Self::calculate_auction_funding_asset_spent(bids.clone());

		let bidder_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			Self::merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		let stored_bids = self.execute(|| Bids::<T>::iter_prefix_values((project_id,)).collect_vec());
		let new_bids = stored_bids
			.into_iter()
			.filter(|bid| bid.final_ct_amount != 0u32.into())
			.map(|bid| BidParams::<T>::from(bid.bidder, bid.final_ct_amount, bid.final_ct_usd_price))
			.collect_vec();

		let _bid_expectations = new_bids
			.iter()
			.map(|bid| BidInfoFilter::<T> {
				final_ct_amount: Some(bid.amount),
				final_ct_usd_price: Some(bid.price),
				..Default::default()
			})
			.collect_vec();

		let _total_ct_sold = new_bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_statemint_asset_to(funding_asset_deposits.clone());

		self.bid_for_users(project_id, bids).expect("Bidding should work");

		self.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		self.do_bid_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		// self.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.start_community_funding(project_id).unwrap();

		// self.finalized_bids_assertions(project_id, bid_expectations, total_ct_sold);

		project_id
	}

	pub fn contribute_for_users(
		&mut self,
		project_id: ProjectIdOf<T>,
		contributions: Vec<ContributionParams<T>>,
	) -> DispatchResultWithPostInfo {
		for cont in contributions {
			self.execute(|| {
				crate::Pallet::<T>::do_contribute(
					&cont.contributor,
					project_id,
					cont.amount,
					cont.multiplier,
					cont.asset,
				)
			})?;
		}
		Ok(().into())
	}

	pub fn start_remainder_or_end_funding(&mut self, project_id: ProjectIdOf<T>) -> Result<(), DispatchError> {
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::CommunityRound);
		let community_funding_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.community
			.end()
			.expect("Community funding end point should exist");
		let remainder_start = community_funding_end + 1u32.into();
		let current_block = self.current_block();
		self.advance_time(remainder_start.saturating_sub(current_block)).unwrap();
		match self.get_project_details(project_id).status {
			ProjectStatus::RemainderRound | ProjectStatus::FundingSuccessful => Ok(()),
			_ => panic!("Bad state"),
		}
	}

	pub fn finish_funding(&mut self, project_id: ProjectIdOf<T>) -> Result<(), DispatchError> {
		let (update_block, _) = self.get_update_pair(project_id);
		let current_block = self.current_block();
		self.advance_time(update_block.saturating_sub(current_block)).unwrap();
		if self.get_project_details(project_id).status == ProjectStatus::RemainderRound {
			let (end_block, _) = self.get_update_pair(project_id);
			let current_block = self.current_block();
			self.advance_time(end_block.saturating_sub(current_block)).unwrap();
		}
		let project_details = self.get_project_details(project_id);
		assert!(
			matches!(
				project_details.status,
				ProjectStatus::FundingSuccessful |
					ProjectStatus::FundingFailed |
					ProjectStatus::AwaitingProjectDecision
			),
			"Project should be in Finished status"
		);
		Ok(())
	}

	pub fn create_remainder_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		contributions: Vec<ContributionParams<T>>,
	) -> ProjectIdOf<T> {
		let project_id =
			self.create_community_contributing_project(project_metadata, issuer, evaluations.clone(), bids.clone());

		if contributions.is_empty() {
			self.start_remainder_or_end_funding(project_id).unwrap();
			return project_id
		}
		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = contributions.accounts();
		let asset_id = contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = self.get_free_statemint_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_after_price_calculation(bids.clone(), ct_price);
		let plmc_contribution_deposits = Self::calculate_contributed_plmc_spent(contributions.clone(), ct_price);

		let necessary_plmc_mint =
			Self::merge_subtract_mappings_by_user(plmc_contribution_deposits.clone(), vec![plmc_evaluation_deposits]);
		let total_plmc_participation_locked =
			Self::merge_add_mappings_by_user(vec![plmc_bid_deposits, plmc_contribution_deposits.clone()]);
		let plmc_existential_deposits = contributors.existential_deposits();

		let funding_asset_deposits = Self::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);
		let contributor_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			Self::merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_statemint_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, contributions.clone()).expect("Contributing should work");

		self.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		// self.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.start_remainder_or_end_funding(project_id).unwrap();
		project_id
	}

	pub fn create_finished_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectIdOf<T> {
		let project_id = self.create_remainder_contributing_project(
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
		);

		match self.get_project_details(project_id).status {
			ProjectStatus::FundingSuccessful => return project_id,
			ProjectStatus::RemainderRound if remainder_contributions.is_empty() => {
				self.finish_funding(project_id).unwrap();
				return project_id
			},
			_ => {},
		};

		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = remainder_contributions.accounts();
		let asset_id = remainder_contributions[0].asset.to_statemint_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let _prev_funding_asset_balances = self.get_free_statemint_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_after_price_calculation(bids.clone(), ct_price);
		let plmc_community_contribution_deposits =
			Self::calculate_contributed_plmc_spent(community_contributions.clone(), ct_price);
		let plmc_remainder_contribution_deposits =
			Self::calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);

		let necessary_plmc_mint = Self::merge_subtract_mappings_by_user(
			plmc_remainder_contribution_deposits.clone(),
			vec![plmc_evaluation_deposits],
		);
		let total_plmc_participation_locked = Self::merge_add_mappings_by_user(vec![
			plmc_bid_deposits,
			plmc_community_contribution_deposits,
			plmc_remainder_contribution_deposits.clone(),
		]);
		let plmc_existential_deposits = contributors.existential_deposits();
		let funding_asset_deposits =
			Self::calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

		let contributor_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances =
			Self::merge_add_mappings_by_user(vec![prev_plmc_balances.clone(), plmc_existential_deposits.clone()]);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_statemint_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		self.do_reserved_plmc_assertions(total_plmc_participation_locked, LockType::Participation(project_id));
		// self.do_contribution_transferred_statemint_asset_assertions(funding_asset_deposits, project_id);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		// self.do_free_statemint_asset_assertions(prev_funding_asset_balances);
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.finish_funding(project_id).unwrap();

		if self.get_project_details(project_id).status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let _project_details = self.get_project_details(project_id);
			let _auction_bought_tokens = bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);
			let __community_bought_tokens =
				community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
			let _remainder_bought_tokens =
				remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

			// assert_eq!(
			// 	project_details.remaining_contribution_tokens.0 + project_details.remaining_contribution_tokens.1,
			// 	project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1 -
			// 		auction_bought_tokens -
			// 		community_bought_tokens -
			// 		remainder_bought_tokens,
			// 	"Remaining CTs are incorrect"
			// );
		}

		project_id
	}

	pub fn create_project_at(
		&mut self,
		status: ProjectStatus,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectIdOf<T> {
		match status {
			ProjectStatus::FundingSuccessful => self.create_finished_project(
				project_metadata,
				issuer,
				evaluations,
				bids,
				community_contributions,
				remainder_contributions,
			),
			ProjectStatus::RemainderRound => self.create_remainder_contributing_project(
				project_metadata,
				issuer,
				evaluations,
				bids,
				community_contributions,
			),
			ProjectStatus::CommunityRound =>
				self.create_community_contributing_project(project_metadata, issuer, evaluations, bids),
			ProjectStatus::AuctionRound(AuctionPhase::English) =>
				self.create_auctioning_project(project_metadata, issuer, evaluations),
			ProjectStatus::EvaluationRound => self.create_evaluating_project(project_metadata, issuer),
			ProjectStatus::Application => self.create_new_project(project_metadata, issuer),
			_ => panic!("unsupported project creation in that status"),
		}
	}
}

pub trait Accounts {
	type Account;

	fn accounts(&self) -> Vec<Self::Account>;
}
pub trait ExistentialDeposits<T: Config> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>>;
}

impl<T: Config + pallet_balances::Config> ExistentialDeposits<T> for Vec<AccountIdOf<T>>
where
// <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>> {
		self.iter()
			.map(|x| {
				UserToPLMCBalance::new(x.clone(), <T as pallet_balances::Config>::ExistentialDeposit::get().into())
			})
			.collect::<Vec<_>>()
	}
}
#[derive(Clone, PartialEq, Debug)]
pub struct UserToPLMCBalance<T: Config> {
	pub account: AccountIdOf<T>,
	pub plmc_amount: BalanceOf<T>,
}
impl<T: Config> UserToPLMCBalance<T> {
	pub fn new(account: AccountIdOf<T>, plmc_amount: BalanceOf<T>) -> Self {
		Self { account, plmc_amount }
	}
}
impl<T: Config> Accounts for Vec<UserToPLMCBalance<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToPLMCBalance { account, plmc_amount: _ } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "std",
	serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))
)]
pub struct UserToUSDBalance<T: Config> {
	pub account: AccountIdOf<T>,
	pub usd_amount: BalanceOf<T>,
}
impl<T: Config> UserToUSDBalance<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: BalanceOf<T>) -> Self {
		Self { account, usd_amount }
	}
}
impl<T: Config> Accounts for Vec<UserToUSDBalance<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToUSDBalance { account, usd_amount: _ } in self {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserToStatemintAsset<T: Config> {
	pub account: AccountIdOf<T>,
	pub asset_amount: BalanceOf<T>,
	pub asset_id: AssetIdOf<T>,
}
impl<T: Config> UserToStatemintAsset<T> {
	pub fn new(account: AccountIdOf<T>, asset_amount: BalanceOf<T>, asset_id: AssetIdOf<T>) -> Self {
		Self { account, asset_amount, asset_id }
	}
}
impl<T: Config> Accounts for Vec<UserToStatemintAsset<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToStatemintAsset { account, .. } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "std",
	serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))
)]
pub struct BidParams<T: Config> {
	pub bidder: AccountIdOf<T>,
	pub amount: BalanceOf<T>,
	pub price: PriceOf<T>,
	pub multiplier: MultiplierOf<T>,
	pub asset: AcceptedFundingAsset,
}
impl<T: Config> BidParams<T> {
	pub fn new(
		bidder: AccountIdOf<T>,
		amount: BalanceOf<T>,
		price: PriceOf<T>,
		multiplier: u8,
		asset: AcceptedFundingAsset,
	) -> Self {
		Self { bidder, amount, price, multiplier: multiplier.try_into().map_err(|_| ()).unwrap(), asset }
	}

	pub fn from(bidder: AccountIdOf<T>, amount: BalanceOf<T>, price: PriceOf<T>) -> Self {
		Self {
			bidder,
			amount,
			price,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> Accounts for Vec<BidParams<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for BidParams { bidder, .. } in self {
			btree.insert(bidder.clone());
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "std",
	serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))
)]
pub struct ContributionParams<T: Config> {
	pub contributor: AccountIdOf<T>,
	pub amount: BalanceOf<T>,
	pub multiplier: MultiplierOf<T>,
	pub asset: AcceptedFundingAsset,
}
impl<T: Config> ContributionParams<T> {
	pub fn new(contributor: AccountIdOf<T>, amount: BalanceOf<T>, multiplier: u8, asset: AcceptedFundingAsset) -> Self {
		Self { contributor, amount, multiplier: multiplier.try_into().map_err(|_| ()).unwrap(), asset }
	}

	pub fn from(contributor: AccountIdOf<T>, amount: BalanceOf<T>) -> Self {
		Self {
			contributor,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> Accounts for Vec<ContributionParams<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for ContributionParams { contributor, .. } in self.iter() {
			btree.insert(contributor.clone());
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<T: Config> {
	pub id: Option<u32>,
	pub project_id: Option<ProjectIdOf<T>>,
	pub bidder: Option<AccountIdOf<T>>,
	pub status: Option<BidStatus<BalanceOf<T>>>,
	pub original_ct_amount: Option<BalanceOf<T>>,
	pub original_ct_usd_price: Option<PriceOf<T>>,
	pub final_ct_amount: Option<BalanceOf<T>>,
	pub final_ct_usd_price: Option<PriceOf<T>>,
	pub funding_asset: Option<AcceptedFundingAsset>,
	pub funding_asset_amount_locked: Option<BalanceOf<T>>,
	pub multiplier: Option<MultiplierOf<T>>,
	pub plmc_bond: Option<BalanceOf<T>>,
	pub plmc_vesting_info: Option<Option<VestingInfoOf<T>>>,
	pub when: Option<BlockNumberOf<T>>,
	pub funds_released: Option<bool>,
	pub ct_minted: Option<bool>,
}
impl<T: Config> BidInfoFilter<T> {
	pub(crate) fn matches_bid(&self, bid: &BidInfoOf<T>) -> bool {
		if self.id.is_some() && self.id.unwrap() != bid.id {
			return false
		}
		if self.project_id.is_some() && self.project_id.unwrap() != bid.project_id {
			return false
		}
		if self.bidder.is_some() && self.bidder.clone().unwrap() != bid.bidder.clone() {
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
		if self.plmc_vesting_info.is_some() && self.plmc_vesting_info.unwrap() != bid.plmc_vesting_info {
			return false
		}
		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false
		}
		if self.funds_released.is_some() && self.funds_released.unwrap() != bid.funds_released {
			return false
		}
		if self.ct_minted.is_some() && self.ct_minted.unwrap() != bid.ct_minted {
			return false
		}

		return true
	}
}
impl<T: Config> Default for BidInfoFilter<T> {
	fn default() -> Self {
		BidInfoFilter::<T> {
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
			plmc_vesting_info: None,
			when: None,
			funds_released: None,
			ct_minted: None,
		}
	}
}

pub mod testing_macros {

	#[macro_export]
	macro_rules! assert_close_enough {
		($real:expr, $desired:expr, $max_approximation:expr) => {
			let real_parts = Perquintill::from_rational($real, $desired);
			let one = Perquintill::from_percent(100u64);
			let real_approximation = one - real_parts;
			assert!(real_approximation <= $max_approximation);
		};
	}

	#[macro_export]
	macro_rules! call_and_is_ok {
		($inst: expr, $( $call: expr ),* ) => {
			$inst.execute(|| {
				$(
					let result = $call;
					assert!(result.is_ok(), "Call failed: {:?}", result);
				)*
			})
		};
	}

	// #[macro_export]
	// macro_rules! find_event {
	// 	($env: expr, $pattern:pat) => {
	// 		$env.execute(|| {
	// 			let events = System::events();
	//
	// 			events.iter().find_map(|event_record| {
	// 				if let frame_system::EventRecord {
	// 					event: RuntimeEvent::FundingModule(desired_event @ $pattern),
	// 					..
	// 				} = event_record
	// 				{
	// 					Some(desired_event.clone())
	// 				} else {
	// 					None
	// 				}
	// 			})
	// 		})
	// 	};
	// }

	#[macro_export]
	macro_rules! extract_from_event {
		($env: expr, $pattern:pat, $field:ident) => {
			$env.execute(|| {
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

	#[macro_export]
	macro_rules! define_names {
		($($name:ident: $id:expr, $label:expr);* $(;)?) => {
			$(
				pub const $name: AccountId = $id;
			)*

			pub fn names() -> std::collections::HashMap<AccountId, &'static str> {
				let mut names = std::collections::HashMap::new();
				$(
					names.insert($name, $label);
				)*
				names
			}
		};
	}
}
