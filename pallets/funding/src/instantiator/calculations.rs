use super::*;
use crate::{MultiplierOf, ParticipationMode};
use core::cmp::Ordering;
use itertools::GroupBy;
#[allow(clippy::wildcard_imports)]
use polimec_common::assets::AcceptedFundingAsset;
use polimec_common::{ProvideAssetPrice, USD_DECIMALS};
use sp_core::{ecdsa, hexdisplay::AsBytesRef, keccak_256, sr25519, Pair};

impl<
		T: Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn get_ed(&self) -> Balance {
		T::ExistentialDeposit::get()
	}

	pub fn get_funding_asset_ed(&mut self, asset_id: AssetIdOf<T>) -> Balance {
		self.execute(|| T::FundingCurrency::minimum_balance(asset_id))
	}

	pub fn get_funding_asset_unit(&mut self, asset_id: AssetIdOf<T>) -> Balance {
		self.execute(|| {
			let decimals = T::FundingCurrency::decimals(asset_id);
			10u128.pow(decimals as u32)
		})
	}

	pub fn get_ct_account_deposit(&self) -> Balance {
		<T as crate::Config>::ContributionTokenCurrency::deposit_required(One::one())
	}

	pub fn calculate_evaluation_plmc_spent(
		&mut self,
		evaluations: Vec<EvaluationParams<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_usd_price = self.execute(|| {
			<PriceProviderOf<T>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});

		let mut output = Vec::new();
		for eval in evaluations {
			let usd_bond = eval.usd_amount;
			let plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);

			output.push(UserToPLMCBalance::new(eval.account, plmc_bond));
		}
		output
	}

	// A single bid can be split into multiple buckets. This function splits the bid into multiple ones at different prices.
	pub fn get_actual_price_charged_for_bucketed_bids(
		&self,
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
					BidParams::from((bid.bidder.clone(), bid_amount, bid.mode, bid.asset)),
					bucket.current_price,
				));
				bucket.update(bid_amount);
				amount_to_bid.saturating_reduce(bid_amount);
			}
		}
		output
	}

	pub fn calculate_auction_plmc_charged_with_given_price(
		&mut self,
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let mut plmc_required = Balance::zero();
			if let ParticipationMode::Classic(multiplier) = bid.mode {
				self.add_required_plmc_to(&mut plmc_required, usd_ticket_size, multiplier)
			}

			output.push(UserToPLMCBalance::new(bid.bidder.clone(), plmc_required));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();

		for (bid, price) in self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let mut plmc_required = Balance::zero();
			if let ParticipationMode::Classic(multiplier) = bid.mode {
				self.add_required_plmc_to(&mut plmc_required, usd_ticket_size, multiplier)
			}

			output.push(UserToPLMCBalance::<T>::new(bid.bidder.clone(), plmc_required));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_plmc_returned_from_all_bids_made(
		&mut self,
		// bids in the order they were made
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let charged_bids = self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.clone().into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price_)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let mut remaining_cts =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		for (price_charged, bids) in grouped_by_price_bids {
			for bid in bids {
				let charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let mut charged_plmc_bond = Balance::zero();
				if let ParticipationMode::Classic(multiplier) = bid.mode {
					self.add_required_plmc_to(&mut charged_plmc_bond, charged_usd_ticket_size, multiplier);
				}

				if remaining_cts <= Zero::zero() {
					output.push(UserToPLMCBalance::new(bid.bidder, charged_plmc_bond));
					continue
				}

				let bought_cts = if remaining_cts < bid.amount { remaining_cts } else { bid.amount };
				remaining_cts = remaining_cts.saturating_sub(bought_cts);

				let final_price =
					if weighted_average_price > price_charged { price_charged } else { weighted_average_price };

				let actual_usd_ticket_size = final_price.saturating_mul_int(bought_cts);
				let mut actual_plmc_bond = Balance::zero();
				if let ParticipationMode::Classic(multiplier) = bid.mode {
					self.add_required_plmc_to(&mut actual_plmc_bond, actual_usd_ticket_size, multiplier);
				}

				let returned_plmc_bond = charged_plmc_bond - actual_plmc_bond;

				output.push(UserToPLMCBalance::<T>::new(bid.bidder, returned_plmc_bond));
			}
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_plmc_spent_post_wap(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_charged =
			self.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(bids, project_metadata.clone(), None);
		let plmc_returned = self.calculate_auction_plmc_returned_from_all_bids_made(
			bids,
			project_metadata.clone(),
			weighted_average_price,
		);

		plmc_charged.subtract_accounts(plmc_returned)
	}

	pub fn calculate_auction_funding_asset_charged_with_given_price(
		&mut self,
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let mut funding_asset_spent = Balance::zero();
			self.add_required_funding_asset_to(&mut funding_asset_spent, usd_ticket_size, bid.asset);
			if bid.mode == ParticipationMode::OTM {
				self.add_otm_fee_to(&mut funding_asset_spent, usd_ticket_size, bid.asset);
			}
			output.push(UserToFundingAsset::new(bid.bidder.clone(), funding_asset_spent, bid.asset.id()));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();

		for (bid, price) in self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let mut funding_asset_spent = Balance::zero();
			self.add_required_funding_asset_to(&mut funding_asset_spent, usd_ticket_size, bid.asset);
			if bid.mode == ParticipationMode::OTM {
				self.add_otm_fee_to(&mut funding_asset_spent, usd_ticket_size, bid.asset);
			}

			output.push(UserToFundingAsset::<T>::new(bid.bidder.clone(), funding_asset_spent, bid.asset.id()));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_funding_asset_returned_from_all_bids_made(
		&mut self,
		// bids in the order they were made
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();
		let charged_bids = self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.clone().into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let mut remaining_cts =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		for (price_charged, bids) in grouped_by_price_bids {
			for bid in bids {
				let mut charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let mut charged_funding_asset = Balance::zero();
				self.add_required_funding_asset_to(&mut charged_funding_asset, charged_usd_ticket_size, bid.asset);
				if bid.mode == ParticipationMode::OTM {
					self.add_otm_fee_to(&mut charged_usd_ticket_size, bid.amount, bid.asset);
				}

				if remaining_cts <= Zero::zero() {
					output.push(UserToFundingAsset::new(bid.bidder, charged_funding_asset, bid.asset.id()));
					continue
				}

				let bought_cts = if remaining_cts < bid.amount { remaining_cts } else { bid.amount };
				remaining_cts = remaining_cts.saturating_sub(bought_cts);

				let final_price =
					if weighted_average_price > price_charged { price_charged } else { weighted_average_price };

				let actual_usd_ticket_size = final_price.saturating_mul_int(bought_cts);
				let mut actual_funding_asset_spent = Balance::zero();
				self.add_required_funding_asset_to(&mut actual_funding_asset_spent, actual_usd_ticket_size, bid.asset);
				if bid.mode == ParticipationMode::OTM {
					self.add_otm_fee_to(&mut actual_funding_asset_spent, actual_usd_ticket_size, bid.asset);
				}

				let returned_foreign_asset = charged_funding_asset - actual_funding_asset_spent;

				output.push(UserToFundingAsset::<T>::new(bid.bidder, returned_foreign_asset, bid.asset.id()));
			}
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_funding_asset_spent_post_wap(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		let funding_asset_charged = self.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			bids,
			project_metadata.clone(),
			None,
		);
		let funding_asset_returned = self.calculate_auction_funding_asset_returned_from_all_bids_made(
			bids,
			project_metadata.clone(),
			weighted_average_price,
		);

		funding_asset_charged.subtract_accounts(funding_asset_returned)
	}

	/// Filters the bids that would be rejected after the auction ends.
	pub fn filter_bids_after_auction(&self, bids: Vec<BidParams<T>>, total_cts: Balance) -> Vec<BidParams<T>> {
		let mut filtered_bids: Vec<BidParams<T>> = Vec::new();
		let sorted_bids = bids;
		let mut total_cts_left = total_cts;
		for bid in sorted_bids {
			if total_cts_left >= bid.amount {
				total_cts_left.saturating_reduce(bid.amount);
				filtered_bids.push(bid);
			} else if !total_cts_left.is_zero() {
				filtered_bids.push(BidParams::from((bid.bidder.clone(), total_cts_left, bid.mode, bid.asset)));
				total_cts_left = Zero::zero();
			}
		}
		filtered_bids
	}

	pub fn calculate_contributed_plmc_spent(
		&mut self,
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		for cont in contributions {
			let mut plmc_bond = 0u128;
			if let ParticipationMode::Classic(multiplier) = cont.mode {
				let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
				self.add_required_plmc_to(&mut plmc_bond, usd_ticket_size, multiplier);
			}

			output.push(UserToPLMCBalance::new(cont.contributor, plmc_bond));
		}
		output
	}

	pub fn calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
		&mut self,
		evaluations: Vec<EvaluationParams<T>>,
		contributions: Vec<ContributionParams<T>>,
		price: PriceOf<T>,
		slashed: bool,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let evaluation_locked_plmc_amounts = self.calculate_evaluation_plmc_spent(evaluations);
		// how much new plmc would be locked without considering evaluation bonds
		let theoretical_contribution_locked_plmc_amounts = self.calculate_contributed_plmc_spent(contributions, price);

		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		let slashable_min_deposits = evaluation_locked_plmc_amounts
			.iter()
			.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance {
				account: account.clone(),
				plmc_amount: slash_percentage * *plmc_amount,
			})
			.collect::<Vec<_>>();
		let available_evaluation_locked_plmc_for_lock_transfer = self.generic_map_operation(
			vec![evaluation_locked_plmc_amounts.clone(), slashable_min_deposits.clone()],
			MergeOperation::Subtract,
		);

		// how much new plmc was actually locked, considering already evaluation bonds used
		// first.
		let actual_contribution_locked_plmc_amounts = self.generic_map_operation(
			vec![theoretical_contribution_locked_plmc_amounts, available_evaluation_locked_plmc_for_lock_transfer],
			MergeOperation::Subtract,
		);
		let mut result = self.generic_map_operation(
			vec![evaluation_locked_plmc_amounts, actual_contribution_locked_plmc_amounts],
			MergeOperation::Add,
		);

		if slashed {
			result = self.generic_map_operation(vec![result, slashable_min_deposits], MergeOperation::Subtract);
		}
		if with_ed {
			for UserToPLMCBalance { account: _, plmc_amount } in result.iter_mut() {
				*plmc_amount += self.get_ed();
			}
		}
		result
	}

	pub fn calculate_contributed_funding_asset_spent(
		&mut self,
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();
		for cont in contributions {
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let mut funding_asset_spent = Balance::zero();
			self.add_required_funding_asset_to(&mut funding_asset_spent, usd_ticket_size, cont.asset);
			if cont.mode == ParticipationMode::OTM {
				self.add_otm_fee_to(&mut funding_asset_spent, usd_ticket_size, cont.asset);
			}

			output.push(UserToFundingAsset::new(cont.contributor, funding_asset_spent, cont.asset.id()));
		}
		output
	}

	pub fn add_otm_fee_to(
		&mut self,
		balance: &mut Balance,
		usd_ticket_size: Balance,
		funding_asset: AcceptedFundingAsset,
	) {
		let multiplier: MultiplierOf<T> = ParticipationMode::OTM.multiplier().try_into().ok().unwrap();
		let plmc_usd_price = self.execute(|| {
			<PriceProviderOf<T>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});
		let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(usd_ticket_size).unwrap();
		let plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
		let otm_fee =
			self.execute(|| <pallet_proxy_bonding::Pallet<T>>::calculate_fee(plmc_bond, funding_asset.id())).unwrap();
		*balance += otm_fee;
	}

	pub fn add_required_plmc_to(&mut self, balance: &mut Balance, usd_ticket_size: Balance, multiplier: u8) {
		let multiplier: MultiplierOf<T> = multiplier.try_into().ok().unwrap();
		let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(usd_ticket_size).unwrap();
		let plmc_usd_price = self.execute(|| {
			<PriceProviderOf<T>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});
		let plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
		*balance += plmc_bond;
	}

	pub fn add_required_funding_asset_to(
		&mut self,
		balance: &mut Balance,
		usd_ticket_size: Balance,
		funding_asset: AcceptedFundingAsset,
	) {
		let funding_asset_usd_price =
			self.execute(|| Pallet::<T>::get_decimals_aware_funding_asset_price(&funding_asset).unwrap());
		let funding_asset_bond = funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
		*balance += funding_asset_bond;
	}

	pub fn generic_map_merge_reduce<M: Clone, K: Ord + Clone, S: Clone>(
		&self,
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
		&self,
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

	pub fn sum_balance_mappings(&self, mut mappings: Vec<Vec<UserToPLMCBalance<T>>>) -> Balance {
		let mut output = mappings
			.swap_remove(0)
			.into_iter()
			.map(|user_to_plmc| user_to_plmc.plmc_amount)
			.fold(Zero::zero(), |a, b| a + b);
		for map in mappings {
			output += map.into_iter().map(|user_to_plmc| user_to_plmc.plmc_amount).fold(Balance::zero(), |a, b| a + b);
		}
		output
	}

	pub fn sum_funding_asset_mappings(
		&self,
		mappings: Vec<Vec<UserToFundingAsset<T>>>,
	) -> Vec<(AssetIdOf<T>, Balance)> {
		let flattened_list = mappings.into_iter().flatten().collect_vec();

		let ordered_list = flattened_list.into_iter().sorted_by(|a, b| a.asset_id.cmp(&b.asset_id)).collect_vec();

		#[allow(clippy::type_complexity)]
		let asset_lists: GroupBy<AssetIdOf<T>, _, fn(&UserToFundingAsset<T>) -> AssetIdOf<T>> =
			ordered_list.into_iter().group_by(|item| item.asset_id.clone());

		let mut output = Vec::new();

		for (asset_id, asset_list) in &asset_lists {
			let sum = asset_list.fold(Zero::zero(), |acc, item| acc + item.asset_amount);
			output.push((asset_id, sum));
		}
		output
	}

	pub fn generate_successful_evaluations(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		evaluators: Vec<AccountIdOf<T>>,
		weights: Vec<u8>,
	) -> Vec<EvaluationParams<T>> {
		let funding_target = project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		let evaluation_success_threshold = <T as Config>::EvaluationSuccessThreshold::get(); // if we use just the threshold, then for big usd targets we lose the evaluation due to PLMC conversion errors in `evaluation_end`
		let usd_threshold = evaluation_success_threshold * funding_target * 2u128;

		zip(evaluators, weights)
			.map(|(evaluator, weight)| {
				let ticket_size = Percent::from_percent(weight) * usd_threshold;
				(evaluator, ticket_size).into()
			})
			.collect()
	}

	pub fn generate_bids_from_total_usd(
		&self,
		usd_amount: Balance,
		min_price: PriceOf<T>,
		weights: Vec<u8>,
		bidders: Vec<AccountIdOf<T>>,
		modes: Vec<ParticipationMode>,
	) -> Vec<BidParams<T>> {
		assert_eq!(weights.len(), bidders.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, bidders), modes)
			.map(|((weight, bidder), mode)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = min_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				BidParams::from((bidder, token_amount, mode, AcceptedFundingAsset::USDT))
			})
			.collect()
	}

	pub fn generate_bids_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		weights: Vec<u8>,
		bidders: Vec<AccountIdOf<T>>,
		modes: Vec<ParticipationMode>,
	) -> Vec<BidParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bid = Percent::from_percent(percent_funding) * total_allocation_size;

		assert_eq!(weights.len(), bidders.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, bidders), modes)
			.map(|((weight, bidder), mode)| {
				let token_amount = Percent::from_percent(weight) * total_ct_bid;
				BidParams::from((bidder, token_amount, mode, AcceptedFundingAsset::USDT))
			})
			.collect()
	}

	pub fn generate_contributions_from_total_usd(
		&self,
		usd_amount: Balance,
		final_price: PriceOf<T>,
		weights: Vec<u8>,
		contributors: Vec<AccountIdOf<T>>,
		modes: Vec<ParticipationMode>,
	) -> Vec<ContributionParams<T>> {
		zip(zip(weights, contributors), modes)
			.map(|((weight, bidder), mode)| {
				let ticket_size = Percent::from_percent(weight) * usd_amount;
				let token_amount = final_price.reciprocal().unwrap().saturating_mul_int(ticket_size);

				ContributionParams::from((bidder, token_amount, mode, AcceptedFundingAsset::USDT))
			})
			.collect()
	}

	pub fn generate_contributions_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		weights: Vec<u8>,
		contributors: Vec<AccountIdOf<T>>,
		modes: Vec<ParticipationMode>,
	) -> Vec<ContributionParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bought = Percent::from_percent(percent_funding) * total_allocation_size;

		assert_eq!(weights.len(), contributors.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, contributors), modes)
			.map(|((weight, contributor), mode)| {
				let token_amount = Percent::from_percent(weight) * total_ct_bought;
				ContributionParams::from((contributor, token_amount, mode, AcceptedFundingAsset::USDT))
			})
			.collect()
	}

	pub fn slash_evaluator_balances(&self, mut balances: Vec<UserToPLMCBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		for UserToPLMCBalance { account: _acc, plmc_amount: balance } in balances.iter_mut() {
			*balance -= slash_percentage * *balance;
		}
		balances
	}

	pub fn calculate_total_reward_for_evaluation(
		&self,
		evaluation: EvaluationInfoOf<T>,
		reward_info: RewardInfo,
	) -> Balance {
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

	pub fn find_bucket_for_wap(&self, project_metadata: ProjectMetadataOf<T>, target_wap: PriceOf<T>) -> BucketOf<T> {
		let mut bucket = <Pallet<T>>::create_bucket_from_metadata(&project_metadata).unwrap();
		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		if target_wap == bucket.initial_price {
			return bucket
		}

		// Fill first bucket
		bucket.update(bucket.delta_amount * 10u128);

		// Fill remaining buckets till we pass by the wap
		loop {
			let wap = bucket.calculate_wap(auction_allocation);

			if wap == target_wap {
				return bucket
			}
			if wap < target_wap {
				bucket.update(bucket.delta_amount);
			} else {
				break
			}
		}

		// Go back one bucket
		bucket.amount_left = bucket.delta_amount;
		bucket.current_price = bucket.current_price - bucket.delta_price;

		// Do a binary search on the amount to reach the desired wap
		let mut lower_bound: Balance = Zero::zero();
		let mut upper_bound: Balance = bucket.delta_amount;

		while lower_bound <= upper_bound {
			let mid_point = (lower_bound + upper_bound) / 2u128;
			bucket.amount_left = mid_point;
			let new_wap = bucket.calculate_wap(auction_allocation);

			// refactor as match
			match new_wap.cmp(&target_wap) {
				Ordering::Equal => return bucket,
				Ordering::Less => upper_bound = mid_point - 1u128,
				Ordering::Greater => lower_bound = mid_point + 1u128,
			}
		}

		bucket
	}

	// We assume a single bid can cover the whole first bucket. Make sure the ticket sizes allow this.
	pub fn generate_bids_from_bucket<F>(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		bucket: BucketOf<T>,
		mut starting_account: AccountIdOf<T>,
		mut increment_account: F,
		funding_asset: AcceptedFundingAsset,
	) -> Vec<BidParams<T>>
	where
		F: FnMut(AccountIdOf<T>) -> AccountIdOf<T>,
	{
		if bucket.current_price == bucket.initial_price {
			return vec![]
		}
		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		let mut generate_bid = |ct_amount| -> BidParams<T> {
			let bid = (starting_account.clone(), ct_amount, funding_asset).into();
			starting_account = increment_account(starting_account.clone());
			bid
		};

		let step_amounts = ((bucket.current_price - bucket.initial_price) / bucket.delta_price).saturating_mul_int(1u8);
		let last_bid_amount = bucket.delta_amount - bucket.amount_left;

		let mut bids = Vec::new();

		let first_bid = generate_bid(auction_allocation);
		bids.push(first_bid);

		for _i in 0u8..step_amounts - 1u8 {
			let full_bucket_bid = generate_bid(bucket.delta_amount);
			bids.push(full_bucket_bid);
		}

		// A CT amount can be so low that the PLMC required is less than the minimum mintable amount. We estimate all bids
		// should be at least 1% of a bucket.
		let min_bid_amount = Percent::from_percent(1) * bucket.delta_amount;
		if last_bid_amount > min_bid_amount {
			let last_bid = generate_bid(last_bid_amount);
			bids.push(last_bid);
		}

		bids
	}

	pub fn generate_bids_that_take_price_to<F>(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		desired_price: PriceOf<T>,
		bidder_account: AccountIdOf<T>,
		next_bidder_account: F,
	) -> Vec<BidParams<T>>
	where
		F: FnMut(AccountIdOf<T>) -> AccountIdOf<T>,
	{
		let necessary_bucket = self.find_bucket_for_wap(project_metadata.clone(), desired_price);
		self.generate_bids_from_bucket(
			project_metadata,
			necessary_bucket,
			bidder_account,
			next_bidder_account,
			AcceptedFundingAsset::USDT,
		)
	}

	// Make sure the bids are in the order they were made
	pub fn calculate_wap_from_all_bids_made(
		&self,
		project_metadata: &ProjectMetadataOf<T>,
		bids: &Vec<BidParams<T>>,
	) -> PriceOf<T> {
		let mut bucket = Pallet::<T>::create_bucket_from_metadata(project_metadata).unwrap();

		for bid in bids {
			bucket.update(bid.amount);
		}

		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		bucket.calculate_wap(auction_allocation)
	}

	pub fn remainder_round_block(&self) -> BlockNumberFor<T> {
		T::EvaluationRoundDuration::get() +
			T::AuctionRoundDuration::get() +
			T::CommunityRoundDuration::get() +
			One::one()
	}

	#[cfg(feature = "std")]
	pub fn eth_key_and_sig_from(
		&mut self,
		seed_string: &str,
		project_id: ProjectId,
		polimec_account: AccountIdOf<T>,
	) -> (Junction, [u8; 65]) {
		let polimec_account_ss58_string = T::SS58Conversion::convert(polimec_account.clone());
		let nonce = self.execute(|| frame_system::Pallet::<T>::account_nonce(polimec_account));
		let message_to_sign =
			crate::functions::misc::typed_data_v4::get_eip_712_message(&polimec_account_ss58_string, project_id, nonce);
		let ecdsa_pair = ecdsa::Pair::from_string(seed_string, None).unwrap();
		let signature = ecdsa_pair.sign_prehashed(&message_to_sign);
		let mut signature_bytes = [0u8; 65];
		signature_bytes[..65].copy_from_slice(signature.as_bytes_ref());

		match signature_bytes[64] {
			0x00 => signature_bytes[64] = 27,
			0x01 => signature_bytes[64] = 28,
			_v => unreachable!("Recovery bit should be always either 0 or 1"),
		}

		let compressed_public_key = ecdsa_pair.public().to_raw();
		let public_uncompressed = k256::ecdsa::VerifyingKey::from_sec1_bytes(&compressed_public_key).unwrap();
		let public_uncompressed_point = public_uncompressed.to_encoded_point(false).to_bytes();
		let derived_ethereum_account: [u8; 20] =
			keccak_256(&public_uncompressed_point[1..])[12..32].try_into().unwrap();
		let junction = Junction::AccountKey20 { network: None, key: derived_ethereum_account };

		(junction, signature_bytes)
	}

	#[cfg(feature = "std")]
	pub fn dot_key_and_sig_from(
		&mut self,
		seed_string: &str,
		project_id: ProjectId,
		polimec_account: AccountIdOf<T>,
	) -> (Junction, [u8; 65]) {
		let message_to_sign =
			self.execute(|| Pallet::<T>::get_substrate_message_to_sign(polimec_account, project_id)).unwrap();
		let message_to_sign = message_to_sign.into_bytes();

		let sr_pair = sr25519::Pair::from_string(seed_string, None).unwrap();
		let signature = sr_pair.sign(&message_to_sign);
		let mut signature_bytes = [0u8; 65];
		signature_bytes[..64].copy_from_slice(signature.as_bytes_ref());
		let junction = Junction::AccountId32 { network: Some(Polkadot), id: sr_pair.public().to_raw() };
		(junction, signature_bytes)
	}
}
