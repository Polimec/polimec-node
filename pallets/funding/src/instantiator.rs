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

use crate::{
	traits::{BondingRequirementCalculation, ProvideAssetPrice},
	*,
};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold, Mutate as FungibleMutate},
		fungibles::{
			metadata::{Inspect as MetadataInspect, MetadataDeposit},
			roles::Inspect as RolesInspect,
			Inspect as FungiblesInspect, Mutate as FungiblesMutate,
		},
		AccountTouch, Get, OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
	Parameter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use parity_scale_codec::Decode;
use sp_arithmetic::{
	traits::{SaturatedConversion, Saturating, Zero},
	FixedPointNumber, Percent, Perquintill,
};
use sp_runtime::{
	traits::{Member, One},
	DispatchError,
};
use sp_std::{
	cell::RefCell,
	collections::{btree_map::*, btree_set::*},
	iter::zip,
	marker::PhantomData,
	ops::Not,
};

pub type RuntimeOriginOf<T> = <T as frame_system::Config>::RuntimeOrigin;
pub struct BoxToFunction(pub Box<dyn FnOnce()>);
impl Default for BoxToFunction {
	fn default() -> Self {
		BoxToFunction(Box::new(|| ()))
	}
}

#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
	feature = "std",
	serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))
)]
#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct TestProjectParams<T: Config> {
	pub expected_state: ProjectStatus,
	pub metadata: ProjectMetadataOf<T>,
	pub issuer: AccountIdOf<T>,
	pub evaluations: Vec<UserToUSDBalance<T>>,
	pub bids: Vec<BidParams<T>>,
	pub community_contributions: Vec<ContributionParams<T>>,
	pub remainder_contributions: Vec<ContributionParams<T>>,
}

#[cfg(feature = "std")]
type OptionalExternalities = Option<RefCell<sp_io::TestExternalities>>;

#[cfg(not(feature = "std"))]
type OptionalExternalities = Option<()>;

pub struct Instantiator<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
> {
	ext: OptionalExternalities,
	nonce: RefCell<u64>,
	_marker: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
}

// general chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn new(ext: OptionalExternalities) -> Self {
		Self { ext, nonce: RefCell::new(0u64), _marker: PhantomData }
	}

	pub fn set_ext(&mut self, ext: OptionalExternalities) {
		self.ext = ext;
	}

	pub fn execute<R>(&mut self, execution: impl FnOnce() -> R) -> R {
		#[cfg(feature = "std")]
		if let Some(ext) = &self.ext {
			return ext.borrow_mut().execute_with(execution);
		}
		execution()
	}

	pub fn get_new_nonce(&self) -> u64 {
		let nonce = *self.nonce.borrow_mut();
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
		lock_type: <T as Config>::RuntimeHoldReason,
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

	pub fn get_free_foreign_asset_balances_for(
		&mut self,
		asset_id: AssetIdOf<T>,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<UserToForeignAssets<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToForeignAssets<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				balances.push(UserToForeignAssets { account, asset_amount, asset_id });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_ct_asset_balances_for(
		&mut self,
		project_id: ProjectId,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<BalanceOf<T>> {
		self.execute(|| {
			let mut balances: Vec<BalanceOf<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::ContributionTokenCurrency::balance(project_id, &account);
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
		reserve_type: <T as Config>::RuntimeHoldReason,
	) -> Vec<UserToPLMCBalance<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_reserved_plmc_balances_for(user_keys, reserve_type)
	}

	pub fn get_all_free_foreign_asset_balances(&mut self, asset_id: AssetIdOf<T>) -> Vec<UserToForeignAssets<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_free_foreign_asset_balances_for(asset_id, user_keys)
	}

	pub fn get_plmc_total_supply(&mut self) -> BalanceOf<T> {
		self.execute(<T as Config>::NativeCurrency::total_issuance)
	}

	pub fn do_reserved_plmc_assertions(
		&mut self,
		correct_funds: Vec<UserToPLMCBalance<T>>,
		reserve_type: <T as Config>::RuntimeHoldReason,
	) {
		for UserToPLMCBalance { account, plmc_amount } in correct_funds {
			self.execute(|| {
				let reserved = <T as Config>::NativeCurrency::balance_on_hold(&reserve_type, &account);
				assert_eq!(reserved, plmc_amount);
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

	pub fn mint_foreign_asset_to(&mut self, mapping: Vec<UserToForeignAssets<T>>) {
		self.execute(|| {
			for UserToForeignAssets { account, asset_amount, asset_id } in mapping {
				<T as Config>::FundingCurrency::mint_into(asset_id, &account, asset_amount)
					.expect("Minting should work");
			}
		});
	}

	pub fn current_block(&mut self) -> BlockNumberFor<T> {
		self.execute(|| frame_system::Pallet::<T>::block_number())
	}

	pub fn advance_time(&mut self, amount: BlockNumberFor<T>) -> Result<(), DispatchError> {
		self.execute(|| {
			for _block in 0u32..amount.saturated_into() {
				let mut current_block = frame_system::Pallet::<T>::block_number();

				<AllPalletsWithoutSystem as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);
				<frame_system::Pallet<T> as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);

				<AllPalletsWithoutSystem as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);
				<frame_system::Pallet<T> as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);

				current_block += One::one();
				frame_system::Pallet::<T>::set_block_number(current_block);

				<frame_system::Pallet<T> as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
				<AllPalletsWithoutSystem as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
			}
			Ok(())
		})
	}

	pub fn do_free_plmc_assertions(&mut self, correct_funds: Vec<UserToPLMCBalance<T>>) {
		for UserToPLMCBalance { account, plmc_amount } in correct_funds {
			self.execute(|| {
				let free = <T as Config>::NativeCurrency::balance(&account);
				assert_eq!(free, plmc_amount, "account has unexpected free plmc balance");
			});
		}
	}

	pub fn do_free_foreign_asset_assertions(&mut self, correct_funds: Vec<UserToForeignAssets<T>>) {
		for UserToForeignAssets { account, asset_amount, asset_id } in correct_funds {
			self.execute(|| {
				let real_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				assert_eq!(asset_amount, real_amount, "Wrong foreign asset balance expected for user {:?}", account);
			});
		}
	}

	pub fn do_bid_transferred_foreign_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToForeignAssets<T>>,
		project_id: ProjectId,
	) {
		for UserToForeignAssets { account, asset_amount, .. } in correct_funds {
			self.execute(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let contribution_total: <T as Config>::Balance =
					Bids::<T>::iter_prefix_values((project_id, account.clone()))
						.map(|c| c.funding_asset_amount_locked)
						.fold(Zero::zero(), |a, b| a + b);
				assert_eq!(
					contribution_total, asset_amount,
					"Wrong foreign asset balance expected for stored auction info on user {:?}",
					account
				);
			});
		}
	}

	// Check if a Contribution storage item exists for the given funding asset transfer
	pub fn do_contribution_transferred_foreign_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToForeignAssets<T>>,
		project_id: ProjectId,
	) {
		for UserToForeignAssets { account, asset_amount, .. } in correct_funds {
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
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn test_ct_created_for(&mut self, project_id: ProjectId) {
		self.execute(|| {
			let metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
			assert_eq!(
				<T as Config>::ContributionTokenCurrency::name(project_id),
				metadata.token_information.name.to_vec()
			);
			let escrow_account = Pallet::<T>::fund_account_id(project_id);

			assert_eq!(<T as Config>::ContributionTokenCurrency::admin(project_id).unwrap(), escrow_account);
			assert_eq!(
				<T as Config>::ContributionTokenCurrency::total_issuance(project_id),
				0u32.into(),
				"No CTs should have been minted at this point"
			);
		});
	}

	pub fn test_ct_not_created_for(&mut self, project_id: ProjectId) {
		self.execute(|| {
			assert!(
				!<T as Config>::ContributionTokenCurrency::asset_exists(project_id),
				"Asset shouldn't exist, since funding failed"
			);
		});
	}

	pub fn creation_assertions(
		&mut self,
		project_id: ProjectId,
		expected_metadata: ProjectMetadataOf<T>,
		creation_start_block: BlockNumberFor<T>,
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
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: crate::ChannelStatus::Closed,
				polimec_to_project: crate::ChannelStatus::Closed,
			},
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	pub fn evaluation_assertions(
		&mut self,
		project_id: ProjectId,
		expected_free_plmc_balances: Vec<UserToPLMCBalance<T>>,
		expected_reserved_plmc_balances: Vec<UserToPLMCBalance<T>>,
		total_plmc_supply: BalanceOf<T>,
	) {
		// just in case we forgot to merge accounts:
		let expected_free_plmc_balances =
			Self::generic_map_operation(vec![expected_free_plmc_balances], MergeOperation::Add);
		let expected_reserved_plmc_balances =
			Self::generic_map_operation(vec![expected_reserved_plmc_balances], MergeOperation::Add);

		let project_details = self.get_project_details(project_id);
		let accounts = expected_reserved_plmc_balances.accounts();
		let expected_ct_account_deposits = accounts
			.into_iter()
			.map(|account| UserToPLMCBalance {
				account,
				plmc_amount: <T as Config>::ContributionTokenCurrency::deposit_required(One::one()),
			})
			.collect::<Vec<UserToPLMCBalance<T>>>();

		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		assert_eq!(self.get_plmc_total_supply(), total_plmc_supply);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_reserved_plmc_assertions(expected_reserved_plmc_balances, HoldReason::Evaluation(project_id).into());
		self.do_reserved_plmc_assertions(expected_ct_account_deposits, HoldReason::FutureDeposit(project_id).into());
	}

	pub fn finalized_bids_assertions(
		&mut self,
		project_id: ProjectId,
		bid_expectations: Vec<BidInfoFilter<T>>,
		expected_ct_sold: BalanceOf<T>,
	) {
		let project_metadata = self.get_project_metadata(project_id);
		let project_details = self.get_project_details(project_id);
		let project_bids = self.execute(|| Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert!(project_details.weighted_average_price.is_some(), "Weighted average price should exist");

		for filter in bid_expectations {
			let _found_bid = project_bids.iter().find(|bid| filter.matches_bid(bid)).unwrap();
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
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn get_ed() -> BalanceOf<T> {
		T::ExistentialDeposit::get()
	}

	pub fn get_ct_account_deposit() -> BalanceOf<T> {
		<T as crate::Config>::ContributionTokenCurrency::deposit_required(One::one())
	}

	pub fn calculate_evaluation_plmc_spent(evaluations: Vec<UserToUSDBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
		let mut output = Vec::new();
		for eval in evaluations {
			let usd_bond = eval.usd_amount;
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(eval.account, plmc_bond));
		}
		output
	}

	pub fn get_actual_price_charged_for_bucketed_bids(
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<(BidParams<T>, PriceOf<T>)> {
		let mut output = Vec::new();
		let mut bucket = if let Some(bucket) = maybe_bucket {
			bucket
		} else {
			Pallet::<T>::create_bucket_from_metadata(&project_metadata).unwrap()
		};
		for bid in bids {
			let mut amount_to_bid = bid.amount;
			while !amount_to_bid.is_zero() {
				let bid_amount = if amount_to_bid <= bucket.amount_left { amount_to_bid } else { bucket.amount_left };
				output.push((
					BidParams {
						bidder: bid.bidder.clone(),
						amount: bid_amount,
						multiplier: bid.multiplier,
						asset: bid.asset,
					},
					bucket.current_price,
				));
				bucket.update(bid_amount);
				amount_to_bid.saturating_reduce(bid_amount);
			}
		}
		output
	}

	pub fn calculate_auction_plmc_charged_with_given_price(
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
		let mut output = Vec::new();
		for bid in bids {
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::new(bid.bidder.clone(), plmc_bond));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();

		for (bid, price) in Self::get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			output.push(UserToPLMCBalance::<T>::new(bid.bidder.clone(), plmc_bond));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	// WARNING: Only put bids that you are sure will be done before the random end of the candle auction
	pub fn calculate_auction_plmc_returned_from_all_bids_made(
		// bids in the order they were made
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let charged_bids = Self::get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.clone().into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price_)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
		let mut remaining_cts = project_metadata.total_allocation_size.0;

		for (price_charged, bids) in grouped_by_price_bids {
			for bid in bids {
				let charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let charged_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(charged_usd_ticket_size).unwrap();
				let charged_plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(charged_usd_bond);

				if remaining_cts <= Zero::zero() {
					output.push(UserToPLMCBalance::new(bid.bidder, charged_plmc_bond));
					continue
				}

				let bought_cts = if remaining_cts < bid.amount { remaining_cts } else { bid.amount };
				remaining_cts = remaining_cts.saturating_sub(bought_cts);

				let final_price =
					if weighted_average_price > price_charged { price_charged } else { weighted_average_price };

				let actual_usd_ticket_size = final_price.saturating_mul_int(bought_cts);
				let actual_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(actual_usd_ticket_size).unwrap();
				let actual_plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(actual_usd_bond);

				let returned_plmc_bond = charged_plmc_bond - actual_plmc_bond;

				output.push(UserToPLMCBalance::<T>::new(bid.bidder, returned_plmc_bond));
			}
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_plmc_spent_post_wap(
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_charged = Self::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			bids,
			project_metadata.clone(),
			None,
		);
		let plmc_returned = Self::calculate_auction_plmc_returned_from_all_bids_made(
			bids,
			project_metadata.clone(),
			weighted_average_price,
		);

		plmc_charged.subtract_accounts(plmc_returned)
	}

	pub fn calculate_auction_funding_asset_charged_with_given_price(
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let asset_price = T::PriceProvider::get_price(bid.asset.to_assethub_id()).unwrap();
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::new(bid.bidder.clone(), funding_asset_spent, bid.asset.to_assethub_id()));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();

		for (bid, price) in Self::get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let asset_price = T::PriceProvider::get_price(bid.asset.to_assethub_id()).unwrap();
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::<T>::new(
				bid.bidder.clone(),
				funding_asset_spent,
				bid.asset.to_assethub_id(),
			));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	// WARNING: Only put bids that you are sure will be done before the random end of the candle auction
	pub fn calculate_auction_funding_asset_returned_from_all_bids_made(
		// bids in the order they were made
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();
		let charged_bids = Self::get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.clone().into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let mut remaining_cts = project_metadata.total_allocation_size.0;

		for (price_charged, bids) in grouped_by_price_bids {
			for bid in bids {
				let funding_asset_price = T::PriceProvider::get_price(bid.asset.to_assethub_id()).unwrap();

				let charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let charged_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(charged_usd_ticket_size).unwrap();
				let charged_funding_asset =
					funding_asset_price.reciprocal().unwrap().saturating_mul_int(charged_usd_bond);

				if remaining_cts <= Zero::zero() {
					output.push(UserToForeignAssets::new(
						bid.bidder,
						charged_funding_asset,
						bid.asset.to_assethub_id(),
					));
					continue
				}

				let bought_cts = if remaining_cts < bid.amount { remaining_cts } else { bid.amount };
				remaining_cts = remaining_cts.saturating_sub(bought_cts);

				let final_price =
					if weighted_average_price > price_charged { price_charged } else { weighted_average_price };

				let actual_usd_ticket_size = final_price.saturating_mul_int(bought_cts);
				let actual_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(actual_usd_ticket_size).unwrap();
				let actual_funding_asset_spent =
					funding_asset_price.reciprocal().unwrap().saturating_mul_int(actual_usd_bond);

				let returned_foreign_asset = charged_funding_asset - actual_funding_asset_spent;

				output.push(UserToForeignAssets::<T>::new(
					bid.bidder,
					returned_foreign_asset,
					bid.asset.to_assethub_id(),
				));
			}
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_funding_asset_spent_post_wap(
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
		let funding_asset_charged = Self::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			bids,
			project_metadata.clone(),
			None,
		);
		let funding_asset_returned = Self::calculate_auction_funding_asset_returned_from_all_bids_made(
			bids,
			project_metadata.clone(),
			weighted_average_price,
		);

		funding_asset_charged.subtract_accounts(funding_asset_returned)
	}

	/// Filters the bids that would be rejected after the auction ends.
	pub fn filter_bids_after_auction(bids: Vec<BidParams<T>>, total_cts: BalanceOf<T>) -> Vec<BidParams<T>> {
		let mut filtered_bids: Vec<BidParams<T>> = Vec::new();
		let sorted_bids = bids;
		let mut total_cts_left = total_cts;
		for bid in sorted_bids {
			if total_cts_left >= bid.amount {
				total_cts_left.saturating_reduce(bid.amount);
				filtered_bids.push(bid);
			} else if !total_cts_left.is_zero() {
				filtered_bids.push(BidParams {
					bidder: bid.bidder.clone(),
					amount: total_cts_left,
					multiplier: bid.multiplier,
					asset: bid.asset,
				});
				total_cts_left = Zero::zero();
			}
		}
		filtered_bids
	}

	pub fn calculate_contributed_plmc_spent(
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
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
			.iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance {
				account: account.clone(),
				plmc_amount: slash_percentage * *plmc_amount,
			})
			.collect::<Vec<_>>();
		let available_evaluation_locked_plmc_for_lock_transfer = Self::generic_map_operation(
			vec![evaluation_locked_plmc_amounts.clone(), slashable_min_deposits.clone()],
			MergeOperation::Subtract,
		);

		// how much new plmc was actually locked, considering already evaluation bonds used
		// first.
		let actual_contribution_locked_plmc_amounts = Self::generic_map_operation(
			vec![theoretical_contribution_locked_plmc_amounts, available_evaluation_locked_plmc_for_lock_transfer],
			MergeOperation::Subtract,
		);
		let mut result = Self::generic_map_operation(
			vec![evaluation_locked_plmc_amounts, actual_contribution_locked_plmc_amounts],
			MergeOperation::Add,
		);

		if slashed {
			result = Self::generic_map_operation(vec![result, slashable_min_deposits], MergeOperation::Subtract);
		}

		result
	}

	pub fn calculate_contributed_funding_asset_spent(
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();
		for cont in contributions {
			let asset_price = T::PriceProvider::get_price(cont.asset.to_assethub_id()).unwrap();
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let funding_asset_spent = asset_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::new(cont.contributor, funding_asset_spent, cont.asset.to_assethub_id()));
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

	/// Merge the given mappings into one mapping, where the values are merged using the given
	/// merge operation.
	///
	/// In case of the `Add` operation, all values are Unioned, and duplicate accounts are
	/// added together.
	/// In case of the `Subtract` operation, all values of the first mapping are subtracted by
	/// the values of the other mappings. Accounts in the other mappings that are not present
	/// in the first mapping are ignored.
	///
	/// # Pseudocode Example
	/// List1: [(A, 10), (B, 5), (C, 5)]
	/// List2: [(A, 5), (B, 5), (D, 5)]
	///
	/// Add: [(A, 15), (B, 10), (C, 5), (D, 5)]
	/// Subtract: [(A, 5), (B, 0), (C, 5)]
	pub fn generic_map_operation<
		N: AccountMerge + Extend<<N as AccountMerge>::Inner> + IntoIterator<Item = <N as AccountMerge>::Inner>,
	>(
		mut mappings: Vec<N>,
		ops: MergeOperation,
	) -> N {
		let mut output = mappings.swap_remove(0);
		output = output.merge_accounts(MergeOperation::Add);
		for map in mappings {
			match ops {
				MergeOperation::Add => output.extend(map),
				MergeOperation::Subtract => output = output.subtract_accounts(map),
			}
		}
		output.merge_accounts(ops)
	}

	pub fn sum_balance_mappings(mut mappings: Vec<Vec<UserToPLMCBalance<T>>>) -> BalanceOf<T> {
		let mut output = mappings
			.swap_remove(0)
			.into_iter()
			.map(|user_to_plmc| user_to_plmc.plmc_amount)
			.fold(Zero::zero(), |a, b| a + b);
		for map in mappings {
			output += map.into_iter().map(|user_to_plmc| user_to_plmc.plmc_amount).fold(Zero::zero(), |a, b| a + b);
		}
		output
	}

	pub fn sum_foreign_mappings(mut mappings: Vec<Vec<UserToForeignAssets<T>>>) -> BalanceOf<T> {
		let mut output = mappings
			.swap_remove(0)
			.into_iter()
			.map(|user_to_asset| user_to_asset.asset_amount)
			.fold(Zero::zero(), |a, b| a + b);
		for map in mappings {
			output += map.into_iter().map(|user_to_asset| user_to_asset.asset_amount).fold(Zero::zero(), |a, b| a + b);
		}
		output
	}

	pub fn generate_bids_from_total_usd(
		usd_amount: BalanceOf<T>,
		min_price: PriceOf<T>,
		weights: Vec<u8>,
		bidders: Vec<AccountIdOf<T>>,
		multipliers: Vec<u8>,
	) -> Vec<BidParams<T>> {
		assert_eq!(weights.len(), bidders.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, bidders), multipliers)
			.map(|((weight, bidder), multiplier)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = min_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				BidParams::new(bidder, token_amount, multiplier, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn generate_contributions_from_total_usd(
		usd_amount: BalanceOf<T>,
		final_price: PriceOf<T>,
		weights: Vec<u8>,
		contributors: Vec<AccountIdOf<T>>,
		multipliers: Vec<u8>,
	) -> Vec<ContributionParams<T>> {
		zip(zip(weights, contributors), multipliers)
			.map(|((weight, bidder), multiplier)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = final_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				ContributionParams::new(bidder, token_amount, multiplier, AcceptedFundingAsset::USDT)
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

		early_evaluators_rewards.saturating_add(normal_evaluators_rewards)
	}
}

// project chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn get_issuer(&mut self, project_id: ProjectId) -> AccountIdOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).unwrap().issuer)
	}

	pub fn get_project_metadata(&mut self, project_id: ProjectId) -> ProjectMetadataOf<T> {
		self.execute(|| ProjectsMetadata::<T>::get(project_id).expect("Project metadata exists"))
	}

	pub fn get_project_details(&mut self, project_id: ProjectId) -> ProjectDetailsOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).expect("Project details exists"))
	}

	pub fn get_update_block(
		&mut self,
		project_id: ProjectId,
		update_type: &UpdateType,
	) -> Option<BlockNumberFor<T>> {
		self.execute(|| {
			ProjectsToUpdate::<T>::iter().find_map(|(block, update_vec)| {
				update_vec
					.iter()
					.find(|(pid, update)| *pid == project_id && update == update_type)
					.map(|(_pid, _update)| block)
			})
		})
	}

	pub fn create_new_project(&mut self, project_metadata: ProjectMetadataOf<T>, issuer: AccountIdOf<T>) -> ProjectId {
		let now = self.current_block();
		let metadata_deposit = T::ContributionTokenCurrency::calc_metadata_deposit(
			project_metadata.token_information.name.as_slice(),
			project_metadata.token_information.symbol.as_slice(),
		);
		// one ED for the issuer, one ED for the escrow account
		self.mint_plmc_to(vec![UserToPLMCBalance::new(
			issuer.clone(),
			Self::get_ed() * 2u64.into() + metadata_deposit,
		)]);

		self.execute(|| {
			crate::Pallet::<T>::do_create(&issuer, project_metadata.clone()).unwrap();
			let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
			log::trace!("Last project metadata: {:?}", last_project_metadata);
		});

		let created_project_id = self.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
		self.creation_assertions(created_project_id, project_metadata, now);
		created_project_id
	}

	pub fn start_evaluation(&mut self, project_id: ProjectId, caller: AccountIdOf<T>) -> Result<(), DispatchError> {
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::Application);
		self.execute(|| crate::Pallet::<T>::do_start_evaluation(caller, project_id).unwrap());
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::EvaluationRound);

		Ok(())
	}

	pub fn create_evaluating_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectId {
		let project_id = self.create_new_project(project_metadata, issuer.clone());
		self.start_evaluation(project_id, issuer).unwrap();
		project_id
	}

	pub fn bond_for_users(
		&mut self,
		project_id: ProjectId,
		bonds: Vec<UserToUSDBalance<T>>,
	) -> DispatchResultWithPostInfo {
		for UserToUSDBalance { account, usd_amount } in bonds {
			self.execute(|| crate::Pallet::<T>::do_evaluate(&account, project_id, usd_amount))?;
		}
		Ok(().into())
	}

	pub fn start_auction(&mut self, project_id: ProjectId, caller: AccountIdOf<T>) -> Result<(), DispatchError> {
		let project_details = self.get_project_details(project_id);

		if project_details.status == ProjectStatus::EvaluationRound {
			let evaluation_end = project_details.phase_transition_points.evaluation.end().unwrap();
			let auction_start = evaluation_end.saturating_add(2u32.into());
			let blocks_to_start = auction_start.saturating_sub(self.current_block());
			self.advance_time(blocks_to_start).unwrap();
		};

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		self.execute(|| crate::Pallet::<T>::do_english_auction(caller, project_id).unwrap());

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));

		Ok(())
	}

	pub fn create_auctioning_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
	) -> ProjectId {
		let project_id = self.create_evaluating_project(project_metadata, issuer.clone());

		let evaluators = evaluations.accounts();
		let prev_supply = self.get_plmc_total_supply();
		let prev_plmc_balances = self.get_free_plmc_balances_for(evaluators.clone());

		let plmc_eval_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = evaluators.existential_deposits();
		let plmc_ct_account_deposits: Vec<UserToPLMCBalance<T>> = evaluators.ct_account_deposits();

		let expected_remaining_plmc: Vec<UserToPLMCBalance<T>> = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		self.mint_plmc_to(plmc_eval_deposits.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_plmc_to(plmc_ct_account_deposits.clone());

		self.bond_for_users(project_id, evaluations).unwrap();

		let expected_evaluator_balances = Self::sum_balance_mappings(vec![
			plmc_eval_deposits.clone(),
			plmc_existential_deposits.clone(),
			plmc_ct_account_deposits,
		]);

		let expected_total_supply = prev_supply + expected_evaluator_balances;

		self.evaluation_assertions(project_id, expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

		self.start_auction(project_id, issuer).unwrap();
		project_id
	}

	pub fn bid_for_users(&mut self, project_id: ProjectId, bids: Vec<BidParams<T>>) -> DispatchResultWithPostInfo {
		for bid in bids {
			self.execute(|| {
				crate::Pallet::<T>::do_bid(&bid.bidder, project_id, bid.amount, bid.multiplier, bid.asset)
			})?;
		}
		Ok(().into())
	}

	pub fn start_community_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		let english_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.english_auction
			.end()
			.expect("English end point should exist");

		self.execute(|| frame_system::Pallet::<T>::set_block_number(english_end));
		// run on_initialize
		self.advance_time(1u32.into()).unwrap();

		let candle_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.candle_auction
			.end()
			.expect("Candle end point should exist");

		self.execute(|| frame_system::Pallet::<T>::set_block_number(candle_end));
		// run on_initialize
		self.advance_time(1u32.into()).unwrap();

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::CommunityRound);

		Ok(())
	}

	pub fn create_community_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectId {
		if bids.is_empty() {
			panic!("Cannot start community funding without bids")
		}

		let project_id = self.create_auctioning_project(project_metadata.clone(), issuer, evaluations.clone());
		let bidders = bids.accounts();
		let bidders_non_evaluators =
			bidders.clone().into_iter().filter(|account| evaluations.accounts().contains(account).not()).collect_vec();
		let asset_id = bids[0].asset.to_assethub_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(bidders.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, bidders.clone());
		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> =
			Self::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
		let participation_usable_evaluation_deposits = plmc_evaluation_deposits
			.into_iter()
			.map(|mut x| {
				x.plmc_amount = x.plmc_amount.saturating_sub(<T as Config>::EvaluatorSlash::get() * x.plmc_amount);
				x
			})
			.collect::<Vec<UserToPLMCBalance<T>>>();
		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_bid_deposits.clone(), participation_usable_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked = plmc_bid_deposits;
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bidders.existential_deposits();
		let plmc_ct_account_deposits: Vec<UserToPLMCBalance<T>> = bidders_non_evaluators.ct_account_deposits();
		let funding_asset_deposits = Self::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);

		let bidder_balances = Self::sum_balance_mappings(vec![
			necessary_plmc_mint.clone(),
			plmc_existential_deposits.clone(),
			plmc_ct_account_deposits.clone(),
		]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_plmc_to(plmc_ct_account_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.bid_for_users(project_id, bids.clone()).unwrap();

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		self.do_bid_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.start_community_funding(project_id).unwrap();

		project_id
	}

	pub fn contribute_for_users(
		&mut self,
		project_id: ProjectId,
		contributions: Vec<ContributionParams<T>>,
	) -> DispatchResultWithPostInfo {
		match self.get_project_details(project_id).status {
			ProjectStatus::CommunityRound =>
				for cont in contributions {
					self.execute(|| {
						crate::Pallet::<T>::do_community_contribute(
							&cont.contributor,
							project_id,
							cont.amount,
							cont.multiplier,
							cont.asset,
						)
					})?;
				},
			ProjectStatus::RemainderRound =>
				for cont in contributions {
					self.execute(|| {
						crate::Pallet::<T>::do_remaining_contribute(
							&cont.contributor,
							project_id,
							cont.amount,
							cont.multiplier,
							cont.asset,
						)
					})?;
				},
			_ => panic!("Project should be in Community or Remainder status"),
		}

		Ok(().into())
	}

	pub fn start_remainder_or_end_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		let details = self.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::CommunityRound);
		let remaining_tokens =
			details.remaining_contribution_tokens.0.saturating_add(details.remaining_contribution_tokens.1);
		let update_type =
			if remaining_tokens > Zero::zero() { UpdateType::RemainderFundingStart } else { UpdateType::FundingEnd };
		if let Some(transition_block) = self.get_update_block(project_id, &update_type) {
			self.execute(|| frame_system::Pallet::<T>::set_block_number(transition_block - One::one()));
			self.advance_time(1u32.into()).unwrap();
			match self.get_project_details(project_id).status {
				ProjectStatus::RemainderRound | ProjectStatus::FundingSuccessful => Ok(()),
				_ => panic!("Bad state"),
			}
		} else {
			panic!("Bad state")
		}
	}

	pub fn finish_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		if let Some(update_block) = self.get_update_block(project_id, &UpdateType::RemainderFundingStart) {
			self.execute(|| frame_system::Pallet::<T>::set_block_number(update_block - One::one()));
			self.advance_time(1u32.into()).unwrap();
		}
		let update_block = self.get_update_block(project_id, &UpdateType::FundingEnd).expect("Funding end block should exist");
		self.execute(|| frame_system::Pallet::<T>::set_block_number(update_block - One::one()));
		self.advance_time(1u32.into()).unwrap();
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
	) -> ProjectId {
		let project_id = self.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
		);

		if contributions.is_empty() {
			self.start_remainder_or_end_funding(project_id).unwrap();
			return project_id;
		}

		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();

		let contributors = contributions.accounts();
		let contributors_non_evaluators = contributors
			.clone()
			.into_iter()
			.filter(|account| evaluations.accounts().contains(account).not())
			.collect_vec();

		let asset_id = contributions[0].asset.to_assethub_id();

		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_post_wap(&bids, project_metadata.clone(), ct_price);
		let plmc_contribution_deposits = Self::calculate_contributed_plmc_spent(contributions.clone(), ct_price);

		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_contribution_deposits.clone(), plmc_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked =
			Self::generic_map_operation(vec![plmc_bid_deposits, plmc_contribution_deposits], MergeOperation::Add);
		let plmc_existential_deposits = contributors.existential_deposits();
		let plmc_ct_account_deposits = contributors_non_evaluators.ct_account_deposits();

		let funding_asset_deposits = Self::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);
		let contributor_balances = Self::sum_balance_mappings(vec![
			necessary_plmc_mint.clone(),
			plmc_existential_deposits.clone(),
			plmc_ct_account_deposits.clone(),
		]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_plmc_to(plmc_ct_account_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, contributions).expect("Contributing should work");

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);

		self.do_contribution_transferred_foreign_asset_assertions(funding_asset_deposits, project_id);

		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
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
	) -> ProjectId {
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
				return project_id;
			},
			_ => {},
		};

		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = remainder_contributions.accounts();
		let new_contributors = contributors
			.clone()
			.into_iter()
			.filter(|account| {
				evaluations.accounts().contains(account).not() &&
					bids.accounts().contains(account).not() &&
					community_contributions.accounts().contains(account).not()
			})
			.collect_vec();
		let asset_id = remainder_contributions[0].asset.to_assethub_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_post_wap(&bids, project_metadata.clone(), ct_price);
		let plmc_community_contribution_deposits =
			Self::calculate_contributed_plmc_spent(community_contributions.clone(), ct_price);
		let plmc_remainder_contribution_deposits =
			Self::calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);

		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_remainder_contribution_deposits.clone(), plmc_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked = Self::generic_map_operation(
			vec![plmc_bid_deposits, plmc_community_contribution_deposits, plmc_remainder_contribution_deposits],
			MergeOperation::Add,
		);
		let plmc_existential_deposits = contributors.existential_deposits();
		let plmc_ct_account_deposits = new_contributors.ct_account_deposits();
		let funding_asset_deposits =
			Self::calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

		let contributor_balances = Self::sum_balance_mappings(vec![
			necessary_plmc_mint.clone(),
			plmc_existential_deposits.clone(),
			plmc_ct_account_deposits.clone(),
		]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_plmc_to(plmc_ct_account_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		self.do_contribution_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.finish_funding(project_id).unwrap();

		if self.get_project_details(project_id).status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let project_details = self.get_project_details(project_id);
			let auction_bought_tokens = bids
				.iter()
				.map(|bid| bid.amount)
				.fold(Zero::zero(), |acc, item| item + acc)
				.min(project_metadata.total_allocation_size.0);
			let community_bought_tokens =
				community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
			let remainder_bought_tokens =
				remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

			assert_eq!(
				project_details.remaining_contribution_tokens.0 + project_details.remaining_contribution_tokens.1,
				project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1 -
					auction_bought_tokens -
					community_bought_tokens -
					remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
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
	) -> ProjectId {
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

#[cfg(feature = "std")]
pub mod async_features {
	use super::*;
	use assert_matches2::assert_matches;
	use futures::FutureExt;
	use std::{
		collections::HashMap,
		sync::{
			atomic::{AtomicBool, AtomicU32, Ordering},
			Arc,
		},
		time::Duration,
	};
	use tokio::{
		sync::{Mutex, Notify},
		time::sleep,
	};

	pub struct BlockOrchestrator<T: Config, AllPalletsWithoutSystem, RuntimeEvent> {
		pub current_block: Arc<AtomicU32>,
		// used for resuming execution of a project that is waiting for a certain block to be reached
		pub awaiting_projects: Mutex<HashMap<BlockNumberFor<T>, Vec<Arc<Notify>>>>,
		pub should_continue: Arc<AtomicBool>,
		pub instantiator_phantom: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
	}
	pub async fn block_controller<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	) {
		loop {
			if !block_orchestrator.continue_running() {
				break;
			}

			let maybe_target_reached = block_orchestrator.advance_to_next_target(instantiator.clone()).await;

			if let Some(target_reached) = maybe_target_reached {
				block_orchestrator.execute_callbacks(target_reached).await;
			}
			// leaves some time for the projects to submit their targets to the orchestrator
			sleep(Duration::from_millis(100)).await;
		}
	}

	impl<
			T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
			AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
			RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
		> Default for BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>
	{
		fn default() -> Self {
			Self::new()
		}
	}

	impl<
			T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
			AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
			RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
		> BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>
	{
		pub fn new() -> Self {
			BlockOrchestrator::<T, AllPalletsWithoutSystem, RuntimeEvent> {
				current_block: Arc::new(AtomicU32::new(0)),
				awaiting_projects: Mutex::new(HashMap::new()),
				should_continue: Arc::new(AtomicBool::new(true)),
				instantiator_phantom: PhantomData,
			}
		}

		pub async fn add_awaiting_project(&self, block_number: BlockNumberFor<T>, notify: Arc<Notify>) {
			let mut awaiting_projects = self.awaiting_projects.lock().await;
			awaiting_projects.entry(block_number).or_default().push(notify);
			drop(awaiting_projects);
		}

		pub async fn advance_to_next_target(
			&self,
			instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		) -> Option<BlockNumberFor<T>> {
			let mut inst = instantiator.lock().await;
			let now: u32 =
				inst.current_block().try_into().unwrap_or_else(|_| panic!("Block number should fit into u32"));
			self.current_block.store(now, Ordering::SeqCst);

			let awaiting_projects = self.awaiting_projects.lock().await;

			if let Some(&next_block) = awaiting_projects.keys().min() {
				drop(awaiting_projects);

				while self.get_current_block() < next_block {
					inst.advance_time(One::one()).unwrap();
					let current_block: u32 = self
						.get_current_block()
						.try_into()
						.unwrap_or_else(|_| panic!("Block number should fit into u32"));
					self.current_block.store(current_block + 1u32, Ordering::SeqCst);
				}
				Some(next_block)
			} else {
				None
			}
		}

		pub async fn execute_callbacks(&self, block_number: BlockNumberFor<T>) {
			let mut awaiting_projects = self.awaiting_projects.lock().await;
			if let Some(notifies) = awaiting_projects.remove(&block_number) {
				for notify in notifies {
					notify.notify_one();
				}
			}
		}

		pub async fn is_empty(&self) -> bool {
			self.awaiting_projects.lock().await.is_empty()
		}

		// Method to check if the loop should continue
		pub fn continue_running(&self) -> bool {
			self.should_continue.load(Ordering::SeqCst)
		}

		// Method to get the current block number
		pub fn get_current_block(&self) -> BlockNumberFor<T> {
			self.current_block.load(Ordering::SeqCst).into()
		}
	}

	// async instantiations for parallel testing
	pub async fn async_create_new_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectId {
		let mut inst = instantiator.lock().await;

		let now = inst.current_block();
		let metadata_deposit = T::ContributionTokenCurrency::calc_metadata_deposit(
			project_metadata.token_information.name.as_slice(),
			project_metadata.token_information.symbol.as_slice(),
		);
		// One ED for the issuer, one for the escrow account
		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			issuer.clone(),
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::get_ed() * 2u64.into() + metadata_deposit,
		)]);
		inst.execute(|| {
			crate::Pallet::<T>::do_create(&issuer, project_metadata.clone()).unwrap();
			let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
			log::trace!("Last project metadata: {:?}", last_project_metadata);
		});

		let created_project_id = inst.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
		inst.creation_assertions(created_project_id, project_metadata, now);
		created_project_id
	}

	pub async fn async_create_evaluating_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectId {
		let project_id = async_create_new_project(instantiator.clone(), project_metadata, issuer.clone()).await;

		let mut inst = instantiator.lock().await;

		inst.start_evaluation(project_id, issuer).unwrap();
		project_id
	}

	pub async fn async_start_auction<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_id: ProjectId,
		caller: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		let mut inst = instantiator.lock().await;

		let project_details = inst.get_project_details(project_id);

		if project_details.status == ProjectStatus::EvaluationRound {
			let update_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(update_block + 1u32.into(), notify.clone()).await;

			// Wait for the notification that our desired block was reached to continue
			drop(inst);

			notify.notified().await;

			inst = instantiator.lock().await;
		};

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		inst.execute(|| crate::Pallet::<T>::do_english_auction(caller, project_id).unwrap());

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound(AuctionPhase::English));

		Ok(())
	}

	pub async fn async_create_auctioning_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectId {
		let project_id =
			async_create_evaluating_project(instantiator.clone(), project_metadata.clone(), issuer.clone()).await;

		let mut inst = instantiator.lock().await;

		let evaluators = evaluations.accounts();
		let prev_supply = inst.get_plmc_total_supply();
		let prev_plmc_balances = inst.get_free_plmc_balances_for(evaluators.clone());

		let plmc_eval_deposits: Vec<UserToPLMCBalance<T>> =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_evaluation_plmc_spent(
				evaluations.clone(),
			);
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = evaluators.existential_deposits();
		let plmc_ct_account_deposits: Vec<UserToPLMCBalance<T>> = evaluators.ct_account_deposits();

		let expected_remaining_plmc: Vec<UserToPLMCBalance<T>> =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![prev_plmc_balances, plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

		inst.mint_plmc_to(plmc_eval_deposits.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());

		inst.bond_for_users(project_id, evaluations).unwrap();

		let expected_evaluator_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::sum_balance_mappings(vec![
				plmc_eval_deposits.clone(),
				plmc_existential_deposits.clone(),
				plmc_ct_account_deposits,
			]);

		let expected_total_supply = prev_supply + expected_evaluator_balances;

		inst.evaluation_assertions(project_id, expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

		drop(inst);

		async_start_auction(instantiator.clone(), block_orchestrator, project_id, issuer).await.unwrap();

		inst = instantiator.lock().await;
		let plmc_for_bids =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_plmc_charged_with_given_price(
				&bids,
				project_metadata.minimum_price,
			);
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bids.accounts().existential_deposits();
		let plmc_ct_account_deposits: Vec<UserToPLMCBalance<T>> = bids.accounts().ct_account_deposits();
		let usdt_for_bids =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_funding_asset_charged_with_given_price(
				&bids,
				project_metadata.minimum_price,
			);

		inst.mint_plmc_to(plmc_for_bids.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_bids.clone());

		inst.bid_for_users(project_id, bids).unwrap();

		project_id
	}

	pub async fn async_start_community_funding<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_id: ProjectId,
	) -> Result<(), DispatchError> {
		let mut inst = instantiator.lock().await;

		let update_block = inst.get_update_block(project_id, &UpdateType::CandleAuctionStart).unwrap();
		let candle_start = update_block + 1u32.into();

		let notify = Arc::new(Notify::new());

		block_orchestrator.add_awaiting_project(candle_start, notify.clone()).await;

		// Wait for the notification that our desired block was reached to continue

		drop(inst);

		notify.notified().await;

		inst = instantiator.lock().await;
		let update_block = inst.get_update_block(project_id, &UpdateType::CommunityFundingStart).unwrap();
		let community_start = update_block + 1u32.into();

		let notify = Arc::new(Notify::new());

		block_orchestrator.add_awaiting_project(community_start, notify.clone()).await;

		drop(inst);

		notify.notified().await;

		inst = instantiator.lock().await;

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

		Ok(())
	}

	pub async fn async_create_community_contributing_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
	) -> (ProjectId, Vec<BidParams<T>>) {
		if bids.is_empty() {
			panic!("Cannot start community funding without bids")
		}

		let project_id = async_create_auctioning_project(
			instantiator.clone(),
			block_orchestrator.clone(),
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			vec![],
		)
		.await;

		let mut inst = instantiator.lock().await;

		let bidders = bids.accounts();
		let bidders_non_evaluators =
			bidders.clone().into_iter().filter(|account| evaluations.accounts().contains(account).not()).collect_vec();
		let asset_id = bids[0].asset.to_assethub_id();
		let prev_plmc_balances = inst.get_free_plmc_balances_for(bidders.clone());
		let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, bidders.clone());
		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None
			);
		let participation_usable_evaluation_deposits = plmc_evaluation_deposits
			.into_iter()
			.map(|mut x| {
				x.plmc_amount = x.plmc_amount.saturating_sub(<T as Config>::EvaluatorSlash::get() * x.plmc_amount);
				x
			})
			.collect::<Vec<UserToPLMCBalance<T>>>();
		let necessary_plmc_mint = Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
			vec![plmc_bid_deposits.clone(), participation_usable_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked = plmc_bid_deposits;
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bidders.existential_deposits();
		let plmc_ct_account_deposits: Vec<UserToPLMCBalance<T>> = bidders_non_evaluators.ct_account_deposits();
		let funding_asset_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None
			);

		let bidder_balances = Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::sum_balance_mappings(vec![
			necessary_plmc_mint.clone(),
			plmc_existential_deposits.clone(),
			plmc_ct_account_deposits.clone(),
		]);

		let expected_free_plmc_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![prev_plmc_balances, plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

		let prev_supply = inst.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		inst.mint_plmc_to(necessary_plmc_mint.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(funding_asset_deposits.clone());

		inst.bid_for_users(project_id, bids.clone()).unwrap();

		inst.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		inst.do_bid_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(inst.get_plmc_total_supply(), post_supply);

		drop(inst);
		async_start_community_funding(instantiator.clone(), block_orchestrator, project_id).await.unwrap();
		let mut inst = instantiator.lock().await;

		let _weighted_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let accepted_bids = Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::filter_bids_after_auction(
			bids,
			project_metadata.total_allocation_size.0,
		);
		let bid_expectations = accepted_bids
			.iter()
			.map(|bid| BidInfoFilter::<T> {
				bidder: Some(bid.bidder.clone()),
				final_ct_amount: Some(bid.amount),
				..Default::default()
			})
			.collect_vec();

		let total_ct_sold = accepted_bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);

		inst.finalized_bids_assertions(project_id, bid_expectations, total_ct_sold);

		(project_id, accepted_bids)
	}

	pub async fn async_start_remainder_or_end_funding<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_id: ProjectId,
	) -> Result<(), DispatchError> {
		let mut inst = instantiator.lock().await;

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

		let update_block = inst.get_update_block(project_id, &UpdateType::RemainderFundingStart).unwrap();
		let remainder_start = update_block + 1u32.into();

		let notify = Arc::new(Notify::new());

		block_orchestrator.add_awaiting_project(remainder_start, notify.clone()).await;

		// Wait for the notification that our desired block was reached to continue

		drop(inst);

		notify.notified().await;

		let mut inst = instantiator.lock().await;

		assert_matches!(
			inst.get_project_details(project_id).status,
			(ProjectStatus::RemainderRound | ProjectStatus::FundingSuccessful)
		);
		Ok(())
	}

	pub async fn async_create_remainder_contributing_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		contributions: Vec<ContributionParams<T>>,
	) -> (ProjectId, Vec<BidParams<T>>) {
		let (project_id, accepted_bids) = async_create_community_contributing_project(
			instantiator.clone(),
			block_orchestrator.clone(),
			project_metadata,
			issuer,
			evaluations.clone(),
			bids,
		)
		.await;

		if contributions.is_empty() {
			async_start_remainder_or_end_funding(instantiator.clone(), block_orchestrator.clone(), project_id)
				.await
				.unwrap();
			return (project_id, accepted_bids);
		}

		let mut inst = instantiator.lock().await;

		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = contributions.accounts();
		let contributors_non_evaluators = contributors
			.clone()
			.into_iter()
			.filter(|account| evaluations.accounts().contains(account).not())
			.collect_vec();
		let asset_id = contributions[0].asset.to_assethub_id();
		let prev_plmc_balances = inst.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_plmc_charged_with_given_price(
				&accepted_bids,
				ct_price,
			);

		let plmc_contribution_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_contributed_plmc_spent(
				contributions.clone(),
				ct_price,
			);

		let necessary_plmc_mint = Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
			vec![plmc_contribution_deposits.clone(), plmc_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![plmc_bid_deposits, plmc_contribution_deposits],
				MergeOperation::Add,
			);
		let plmc_existential_deposits = contributors.existential_deposits();
		let plmc_ct_account_deposits = contributors_non_evaluators.ct_account_deposits();

		let funding_asset_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_contributed_funding_asset_spent(
				contributions.clone(),
				ct_price,
			);
		let contributor_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::sum_balance_mappings(vec![
				necessary_plmc_mint.clone(),
				plmc_existential_deposits.clone(),
				plmc_ct_account_deposits.clone(),
			]);

		let expected_free_plmc_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![prev_plmc_balances, plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

		let prev_supply = inst.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		inst.mint_plmc_to(necessary_plmc_mint.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(funding_asset_deposits.clone());

		inst.contribute_for_users(project_id, contributions).expect("Contributing should work");

		inst.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		inst.do_contribution_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(inst.get_plmc_total_supply(), post_supply);
		drop(inst);
		async_start_remainder_or_end_funding(instantiator.clone(), block_orchestrator.clone(), project_id)
			.await
			.unwrap();
		(project_id, accepted_bids)
	}

	pub async fn async_finish_funding<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_id: ProjectId,
	) -> Result<(), DispatchError> {
		let mut inst = instantiator.lock().await;
		let update_block = inst.get_update_block(project_id, &UpdateType::FundingEnd).unwrap();

		let notify = Arc::new(Notify::new());
		block_orchestrator.add_awaiting_project(update_block + 1u32.into(), notify.clone()).await;
		Ok(())
	}

	pub async fn async_create_finished_project<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		let (project_id, accepted_bids) = async_create_remainder_contributing_project(
			instantiator.clone(),
			block_orchestrator.clone(),
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
		)
		.await;

		let mut inst = instantiator.lock().await;

		let total_ct_sold_in_bids = bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);
		let total_ct_sold_in_community_contributions =
			community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
		let total_ct_sold_in_remainder_contributions =
			remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

		let total_ct_sold =
			total_ct_sold_in_bids + total_ct_sold_in_community_contributions + total_ct_sold_in_remainder_contributions;
		let total_ct_available = project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1;
		assert!(
			total_ct_sold <= total_ct_available,
			"Some CT buys are getting less than expected due to running out of CTs. This is ok in the runtime, but likely unexpected from the parameters of this instantiation"
		);

		match inst.get_project_details(project_id).status {
			ProjectStatus::FundingSuccessful => return project_id,
			ProjectStatus::RemainderRound if remainder_contributions.is_empty() => {
				inst.finish_funding(project_id).unwrap();
				return project_id;
			},
			_ => {},
		};

		let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = remainder_contributions.accounts();
		let new_contributors = contributors
			.clone()
			.into_iter()
			.filter(|account| {
				evaluations.accounts().contains(account).not() &&
					bids.accounts().contains(account).not() &&
					community_contributions.accounts().contains(account).not()
			})
			.collect_vec();
		let asset_id = remainder_contributions[0].asset.to_assethub_id();
		let prev_plmc_balances = inst.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&accepted_bids,
				project_metadata.clone(),
				None
			);
		let plmc_community_contribution_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_contributed_plmc_spent(
				community_contributions.clone(),
				ct_price,
			);
		let plmc_remainder_contribution_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_contributed_plmc_spent(
				remainder_contributions.clone(),
				ct_price,
			);

		let necessary_plmc_mint = Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
			vec![plmc_remainder_contribution_deposits.clone(), plmc_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![plmc_bid_deposits, plmc_community_contribution_deposits, plmc_remainder_contribution_deposits],
				MergeOperation::Add,
			);
		let plmc_existential_deposits = contributors.existential_deposits();
		let plmc_ct_account_deposits = new_contributors.ct_account_deposits();
		let funding_asset_deposits =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::calculate_contributed_funding_asset_spent(
				remainder_contributions.clone(),
				ct_price,
			);

		let contributor_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::sum_balance_mappings(vec![
				necessary_plmc_mint.clone(),
				plmc_existential_deposits.clone(),
				plmc_ct_account_deposits.clone(),
			]);

		let expected_free_plmc_balances =
			Instantiator::<T, AllPalletsWithoutSystem, RuntimeEvent>::generic_map_operation(
				vec![prev_plmc_balances, plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

		let prev_supply = inst.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		inst.mint_plmc_to(necessary_plmc_mint.clone());
		inst.mint_plmc_to(plmc_existential_deposits.clone());
		inst.mint_plmc_to(plmc_ct_account_deposits.clone());
		inst.mint_foreign_asset_to(funding_asset_deposits.clone());

		inst.contribute_for_users(project_id, remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		let merged = total_plmc_participation_locked.merge_accounts(MergeOperation::Add);

		inst.do_reserved_plmc_assertions(merged, HoldReason::Participation(project_id).into());

		inst.do_contribution_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(inst.get_plmc_total_supply(), post_supply);

		drop(inst);
		async_finish_funding(instantiator.clone(), block_orchestrator.clone(), project_id).await.unwrap();
		let mut inst = instantiator.lock().await;

		if inst.get_project_details(project_id).status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let project_details = inst.get_project_details(project_id);
			let auction_bought_tokens =
				accepted_bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);
			let community_bought_tokens =
				community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
			let remainder_bought_tokens =
				remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

			assert_eq!(
				project_details.remaining_contribution_tokens.0 + project_details.remaining_contribution_tokens.1,
				project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1 -
					auction_bought_tokens -
					community_bought_tokens -
					remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
		}

		project_id
	}

	pub async fn create_project_at<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		test_project_params: TestProjectParams<T>,
	) -> ProjectId {
		match test_project_params.expected_state {
			ProjectStatus::FundingSuccessful =>
				async_create_finished_project(
					instantiator,
					block_orchestrator,
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
					test_project_params.community_contributions,
					test_project_params.remainder_contributions,
				)
				.await,
			ProjectStatus::RemainderRound =>
				async_create_remainder_contributing_project(
					instantiator,
					block_orchestrator,
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
					test_project_params.community_contributions,
				)
				.map(|(project_id, _)| project_id)
				.await,
			ProjectStatus::CommunityRound =>
				async_create_community_contributing_project(
					instantiator,
					block_orchestrator,
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
				)
				.map(|(project_id, _)| project_id)
				.await,
			ProjectStatus::AuctionRound(AuctionPhase::English) =>
				async_create_auctioning_project(
					instantiator,
					block_orchestrator,
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
				)
				.await,
			ProjectStatus::EvaluationRound =>
				async_create_evaluating_project(instantiator, test_project_params.metadata, test_project_params.issuer)
					.await,
			ProjectStatus::Application =>
				async_create_new_project(instantiator, test_project_params.metadata, test_project_params.issuer).await,
			_ => panic!("unsupported project creation in that status"),
		}
	}

	pub async fn async_create_project_at<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		mutex_inst: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
		block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
		test_project_params: TestProjectParams<T>,
	) -> ProjectId {
		let time_to_new_project: BlockNumberFor<T> = Zero::zero();
		let time_to_evaluation: BlockNumberFor<T> = time_to_new_project + Zero::zero();
		// we immediately start the auction, so we dont wait for T::AuctionInitializePeriodDuration.
		let time_to_auction: BlockNumberFor<T> = time_to_evaluation + <T as Config>::EvaluationDuration::get();
		let time_to_community: BlockNumberFor<T> = time_to_auction +
			<T as Config>::EnglishAuctionDuration::get() +
			<T as Config>::CandleAuctionDuration::get();
		let time_to_remainder: BlockNumberFor<T> = time_to_community + <T as Config>::CommunityFundingDuration::get();
		let time_to_finish: BlockNumberFor<T> = time_to_remainder + <T as Config>::RemainderFundingDuration::get();
		let mut inst = mutex_inst.lock().await;
		let now = inst.current_block();
		drop(inst);

		match test_project_params.expected_state {
			ProjectStatus::Application => {
				let notify = Arc::new(Notify::new());
				block_orchestrator
					.add_awaiting_project(now + time_to_finish - time_to_new_project, notify.clone())
					.await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_new_project(mutex_inst.clone(), test_project_params.metadata, test_project_params.issuer)
					.await
			},
			ProjectStatus::EvaluationRound => {
				let notify = Arc::new(Notify::new());
				block_orchestrator
					.add_awaiting_project(now + time_to_finish - time_to_evaluation, notify.clone())
					.await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_evaluating_project(
					mutex_inst.clone(),
					test_project_params.metadata,
					test_project_params.issuer,
				)
				.await
			},
			ProjectStatus::AuctionRound(_) => {
				let notify = Arc::new(Notify::new());
				block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_auction, notify.clone()).await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_auctioning_project(
					mutex_inst.clone(),
					block_orchestrator.clone(),
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
				)
				.await
			},
			ProjectStatus::CommunityRound => {
				let notify = Arc::new(Notify::new());
				block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_community, notify.clone()).await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_community_contributing_project(
					mutex_inst.clone(),
					block_orchestrator.clone(),
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
				)
				.map(|(project_id, _)| project_id)
				.await
			},
			ProjectStatus::RemainderRound => {
				let notify = Arc::new(Notify::new());
				block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_remainder, notify.clone()).await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_remainder_contributing_project(
					mutex_inst.clone(),
					block_orchestrator.clone(),
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
					test_project_params.community_contributions,
				)
				.map(|(project_id, _)| project_id)
				.await
			},
			ProjectStatus::FundingSuccessful => {
				let notify = Arc::new(Notify::new());
				block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_finish, notify.clone()).await;
				// Wait for the notification that our desired block was reached to continue
				notify.notified().await;
				async_create_finished_project(
					mutex_inst.clone(),
					block_orchestrator.clone(),
					test_project_params.metadata,
					test_project_params.issuer,
					test_project_params.evaluations,
					test_project_params.bids,
					test_project_params.community_contributions,
					test_project_params.remainder_contributions,
				)
				.await
			},
			_ => unimplemented!("Unsupported project creation in that status"),
		}
	}

	pub fn create_multiple_projects_at<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>> + 'static + 'static,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	>(
		instantiator: Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>,
		projects: Vec<TestProjectParams<T>>,
	) -> (Vec<ProjectId>, Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>) {
		use tokio::runtime::Builder;
		let tokio_runtime = Builder::new_current_thread().enable_all().build().unwrap();
		let local = tokio::task::LocalSet::new();
		let execution = local.run_until(async move {
			let block_orchestrator = Arc::new(BlockOrchestrator::new());
			let mutex_inst = Arc::new(Mutex::new(instantiator));

			let project_futures = projects.into_iter().map(|project| {
				let block_orchestrator = block_orchestrator.clone();
				let mutex_inst = mutex_inst.clone();
				tokio::task::spawn_local(async {
					async_create_project_at(mutex_inst, block_orchestrator, project).await
				})
			});

			// Wait for all project creation tasks to complete
			let joined_project_futures = futures::future::join_all(project_futures);
			let controller_handle =
				tokio::task::spawn_local(block_controller(block_orchestrator.clone(), mutex_inst.clone()));
			let projects = joined_project_futures.await;

			// Now that all projects have been set up, signal the block_controller to stop
			block_orchestrator.should_continue.store(false, Ordering::SeqCst);

			// Wait for the block controller to finish
			controller_handle.await.unwrap();

			let inst = Arc::try_unwrap(mutex_inst).unwrap_or_else(|_| panic!("mutex in use")).into_inner();
			let project_ids = projects.into_iter().map(|project| project.unwrap()).collect_vec();

			(project_ids, inst)
		});
		tokio_runtime.block_on(execution)
	}
}

pub trait Accounts {
	type Account;

	fn accounts(&self) -> Vec<Self::Account>;
}

pub enum MergeOperation {
	Add,
	Subtract,
}
pub trait AccountMerge {
	/// The inner type of the Vec implementing this Trait.
	type Inner;
	/// Merge accounts in the list based on the operation.
	fn merge_accounts(&self, ops: MergeOperation) -> Self;
	/// Subtract amount of the matching accounts in the other list from the current list.
	/// If the account is not present in the current list, it is ignored.
	fn subtract_accounts(&self, other_list: Self) -> Self;
}

pub trait Deposits<T: Config> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>>;
	fn ct_account_deposits(&self) -> Vec<UserToPLMCBalance<T>>;
}

impl<T: Config + pallet_balances::Config> Deposits<T> for Vec<AccountIdOf<T>> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>> {
		self.iter()
			.map(|x| UserToPLMCBalance::new(x.clone(), <T as pallet_balances::Config>::ExistentialDeposit::get()))
			.collect::<Vec<_>>()
	}

	fn ct_account_deposits(&self) -> Vec<UserToPLMCBalance<T>> {
		self.iter()
			.map(|x| {
				UserToPLMCBalance::new(
					x.clone(),
					<T as crate::Config>::ContributionTokenCurrency::deposit_required(One::one()),
				)
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
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToPLMCBalance<T> {
	fn from((account, plmc_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToPLMCBalance::<T>::new(account, plmc_amount)
	}
}

impl<T: Config> AccountMerge for Vec<UserToPLMCBalance<T>> {
	type Inner = UserToPLMCBalance<T>;

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToPLMCBalance { account, plmc_amount } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut BalanceOf<T>| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*plmc_amount),
						MergeOperation::Subtract => e.saturating_sub(*plmc_amount),
					}
				})
				.or_insert(*plmc_amount);
		}
		btree.into_iter().map(|(account, plmc_amount)| UserToPLMCBalance::new(account, plmc_amount)).collect()
	}

	fn subtract_accounts(&self, other_list: Self) -> Self {
		let current_accounts = self.accounts();
		let filtered_list = other_list.into_iter().filter(|x| current_accounts.contains(&x.account)).collect_vec();
		let mut new_list = self.clone();
		new_list.extend(filtered_list);
		new_list.merge_accounts(MergeOperation::Subtract)
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
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToUSDBalance<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToUSDBalance::<T>::new(account, usd_amount)
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
impl<T: Config> AccountMerge for Vec<UserToUSDBalance<T>> {
	type Inner = UserToUSDBalance<T>;

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToUSDBalance { account, usd_amount } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut BalanceOf<T>| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*usd_amount),
						MergeOperation::Subtract => e.saturating_sub(*usd_amount),
					}
				})
				.or_insert(*usd_amount);
		}
		btree.into_iter().map(|(account, usd_amount)| UserToUSDBalance::new(account, usd_amount)).collect()
	}

	fn subtract_accounts(&self, other_list: Self) -> Self {
		let current_accounts = self.accounts();
		let filtered_list = other_list.into_iter().filter(|x| current_accounts.contains(&x.account)).collect_vec();
		let mut new_list = self.clone();
		new_list.extend(filtered_list);
		new_list.merge_accounts(MergeOperation::Subtract)
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserToForeignAssets<T: Config> {
	pub account: AccountIdOf<T>,
	pub asset_amount: BalanceOf<T>,
	pub asset_id: AssetIdOf<T>,
}
impl<T: Config> UserToForeignAssets<T> {
	pub fn new(account: AccountIdOf<T>, asset_amount: BalanceOf<T>, asset_id: AssetIdOf<T>) -> Self {
		Self { account, asset_amount, asset_id }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)> for UserToForeignAssets<T> {
	fn from((account, asset_amount, asset_id): (AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)) -> Self {
		UserToForeignAssets::<T>::new(account, asset_amount, asset_id)
	}
}
impl<T: Config> Accounts for Vec<UserToForeignAssets<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToForeignAssets { account, .. } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> AccountMerge for Vec<UserToForeignAssets<T>> {
	type Inner = UserToForeignAssets<T>;

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToForeignAssets { account, asset_amount, asset_id } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut (BalanceOf<T>, u32)| {
					e.0 = match ops {
						MergeOperation::Add => e.0.saturating_add(*asset_amount),
						MergeOperation::Subtract => e.0.saturating_sub(*asset_amount),
					}
				})
				.or_insert((*asset_amount, *asset_id));
		}
		btree.into_iter().map(|(account, info)| UserToForeignAssets::new(account, info.0, info.1)).collect()
	}

	fn subtract_accounts(&self, other_list: Self) -> Self {
		let current_accounts = self.accounts();
		let filtered_list = other_list.into_iter().filter(|x| current_accounts.contains(&x.account)).collect_vec();
		let mut new_list = self.clone();
		new_list.extend(filtered_list);
		new_list.merge_accounts(MergeOperation::Subtract)
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
	pub multiplier: MultiplierOf<T>,
	pub asset: AcceptedFundingAsset,
}
impl<T: Config> BidParams<T> {
	pub fn new(bidder: AccountIdOf<T>, amount: BalanceOf<T>, multiplier: u8, asset: AcceptedFundingAsset) -> Self {
		Self { bidder, amount, multiplier: multiplier.try_into().map_err(|_| ()).unwrap(), asset }
	}

	pub fn new_with_defaults(bidder: AccountIdOf<T>, amount: BalanceOf<T>) -> Self {
		Self {
			bidder,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for BidParams<T> {
	fn from((bidder, amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		Self {
			bidder,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)> for BidParams<T> {
	fn from((bidder, amount, multiplier): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)) -> Self {
		Self { bidder, amount, multiplier, asset: AcceptedFundingAsset::USDT }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset)> for BidParams<T> {
	fn from(
		(bidder, amount, multiplier, asset): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset),
	) -> Self {
		Self { bidder, amount, multiplier, asset }
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

	pub fn new_with_defaults(contributor: AccountIdOf<T>, amount: BalanceOf<T>) -> Self {
		Self {
			contributor,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for ContributionParams<T> {
	fn from((contributor, amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		Self {
			contributor,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)> for ContributionParams<T> {
	fn from((contributor, amount, multiplier): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)) -> Self {
		Self { contributor, amount, multiplier, asset: AcceptedFundingAsset::USDT }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset)> for ContributionParams<T> {
	fn from(
		(contributor, amount, multiplier, asset): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset),
	) -> Self {
		Self { contributor, amount, multiplier, asset }
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
	pub project_id: Option<ProjectId>,
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
	pub when: Option<BlockNumberFor<T>>,
	pub funds_released: Option<bool>,
	pub ct_minted: Option<bool>,
}
impl<T: Config> BidInfoFilter<T> {
	pub(crate) fn matches_bid(&self, bid: &BidInfoOf<T>) -> bool {
		if self.id.is_some() && self.id.unwrap() != bid.id {
			return false;
		}
		if self.project_id.is_some() && self.project_id.unwrap() != bid.project_id {
			return false;
		}
		if self.bidder.is_some() && self.bidder.clone().unwrap() != bid.bidder.clone() {
			return false;
		}
		if self.status.is_some() && self.status.as_ref().unwrap() != &bid.status {
			return false;
		}
		if self.original_ct_amount.is_some() && self.original_ct_amount.unwrap() != bid.original_ct_amount {
			return false;
		}
		if self.original_ct_usd_price.is_some() && self.original_ct_usd_price.unwrap() != bid.original_ct_usd_price {
			return false;
		}
		if self.final_ct_amount.is_some() && self.final_ct_amount.unwrap() != bid.final_ct_amount {
			return false;
		}
		if self.final_ct_usd_price.is_some() && self.final_ct_usd_price.unwrap() != bid.final_ct_usd_price {
			return false;
		}
		if self.funding_asset.is_some() && self.funding_asset.unwrap() != bid.funding_asset {
			return false;
		}
		if self.funding_asset_amount_locked.is_some() &&
			self.funding_asset_amount_locked.unwrap() != bid.funding_asset_amount_locked
		{
			return false;
		}
		if self.multiplier.is_some() && self.multiplier.unwrap() != bid.multiplier {
			return false;
		}
		if self.plmc_bond.is_some() && self.plmc_bond.unwrap() != bid.plmc_bond {
			return false;
		}
		if self.plmc_vesting_info.is_some() && self.plmc_vesting_info.unwrap() != bid.plmc_vesting_info {
			return false;
		}
		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false;
		}
		if self.funds_released.is_some() && self.funds_released.unwrap() != bid.funds_released {
			return false;
		}
		if self.ct_minted.is_some() && self.ct_minted.unwrap() != bid.ct_minted {
			return false;
		}

		true
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
	/// Example:
	/// ```
	/// use pallet_funding::assert_close_enough;
	/// use sp_arithmetic::Perquintill;
	///
	/// let real = 98u64;
	/// let desired = 100u64;
	/// assert_close_enough!(real, desired, Perquintill::from_float(0.02));
	/// // This would fail
	/// // assert_close_enough!(real, desired, Perquintill::from_float(0.01));
	/// ```
	///
	/// - Use this macro when you deal with operations with lots of decimals, and you are ok with the real value being an approximation of the desired one.
	/// - The max_approximation should be an upper bound such that 1-real/desired <= approximation in the case where the desired is smaller than the real,
	/// and 1-desired/real <= approximation in the case where the real is bigger than the desired.
	/// - You probably should define the max_approximation from a float number or a percentage, like in the example.
	macro_rules! assert_close_enough {
		// Match when a message is provided
		($real:expr, $desired:expr, $max_approximation:expr, $msg:expr) => {
			let real_parts;
			if $real <= $desired {
				real_parts = Perquintill::from_rational($real, $desired);
			} else {
				real_parts = Perquintill::from_rational($desired, $real);
			}
			let one = Perquintill::from_percent(100u64);
			let real_approximation = one - real_parts;
			assert!(real_approximation <= $max_approximation, $msg);
		};
		// Match when no message is provided
		($real:expr, $desired:expr, $max_approximation:expr) => {
			let real_parts;
			if $real <= $desired {
				real_parts = Perquintill::from_rational($real, $desired);
			} else {
				real_parts = Perquintill::from_rational($desired, $real);
			}
			let one = Perquintill::from_percent(100u64);
			let real_approximation = one - real_parts;
			assert!(
				real_approximation <= $max_approximation,
				"Approximation is too big: {:?} > {:?} for {:?} and {:?}",
				real_approximation,
				$max_approximation,
				$real,
				$desired
			);
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
	#[macro_export]
	macro_rules! find_event {
    ($runtime:ty, $pattern:pat, $($field_name:ident == $field_value:expr),+) => {
	    {
		    let events = frame_system::Pallet::<$runtime>::events();
	        events.iter().find_map(|event_record| {
			    let runtime_event = event_record.event.clone();
			    let runtime_event = <<$runtime as crate::Config>::RuntimeEvent>::from(runtime_event);
			    if let Ok(funding_event) = TryInto::<crate::Event<$runtime>>::try_into(runtime_event) {
				     if let $pattern = funding_event {
	                    let mut is_match = true;
	                    $(
	                        is_match &= $field_name == $field_value;
	                    )+
	                    if is_match {
	                        return Some(funding_event.clone());
	                    }
	                }
	                None
			    } else {
	                None
	            }
            })
	    }
    };
}

	#[macro_export]
	macro_rules! extract_from_event {
		($env: expr, $pattern:pat, $field:ident) => {
			$env.execute(|| {
				let events = System::events();

				events.iter().find_map(|event_record| {
					if let frame_system::EventRecord { event: RuntimeEvent::PolimecFunding($pattern), .. } =
						event_record
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
