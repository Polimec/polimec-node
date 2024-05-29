use super::*;
use polimec_common::USD_DECIMALS;

impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn get_ed(&self) -> BalanceOf<T> {
		T::ExistentialDeposit::get()
	}

	pub fn get_ct_account_deposit(&self) -> BalanceOf<T> {
		<T as crate::Config>::ContributionTokenCurrency::deposit_required(One::one())
	}

	pub fn calculate_evaluation_plmc_spent(
		&mut self,
		evaluations: Vec<UserToUSDBalance<T>>,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_usd_price = self.execute(|| {
			T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});

		let mut output = Vec::new();
		for eval in evaluations {
			let usd_bond = eval.usd_amount;
			let mut plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			if with_ed {
				plmc_bond = plmc_bond.saturating_add(self.get_ed());
			}
			output.push(UserToPLMCBalance::new(eval.account, plmc_bond));
		}
		output
	}

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
		&mut self,
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_usd_price = self.execute(|| {
			T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});

		let mut output = Vec::new();
		for bid in bids {
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let mut plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			if with_ed {
				plmc_bond = plmc_bond.saturating_add(self.get_ed());
			}
			output.push(UserToPLMCBalance::new(bid.bidder.clone(), plmc_bond));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let plmc_usd_price = self.execute(|| {
			T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});

		for (bid, price) in self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let usd_bond = bid.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let mut plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			if with_ed {
				plmc_bond = plmc_bond.saturating_add(self.get_ed());
			}
			output.push(UserToPLMCBalance::<T>::new(bid.bidder.clone(), plmc_bond));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	// WARNING: Only put bids that you are sure will be done before the random end of the closing auction
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

		let plmc_usd_price = self.execute(|| {
			T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});
		let mut remaining_cts =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		for (price_charged, bids) in grouped_by_price_bids {
			for bid in bids {
				let charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let charged_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(charged_usd_ticket_size).unwrap();
				let charged_plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(charged_usd_bond);

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
				let actual_plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(actual_usd_bond);

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
		let plmc_charged = self.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			bids,
			project_metadata.clone(),
			None,
			false,
		);
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
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let funding_asset_id = bid.asset.to_assethub_id();
			let funding_asset_decimals = self.execute(|| T::FundingCurrency::decimals(funding_asset_id));
			let funding_asset_usd_price = self.execute(|| {
				T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
					.unwrap()
			});
			let usd_ticket_size = ct_price.saturating_mul_int(bid.amount);
			let funding_asset_spent = funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::new(bid.bidder.clone(), funding_asset_spent, bid.asset.to_assethub_id()));
		}
		output
	}

	// Make sure you give it all the bids made for the project. It doesn't require a ct_price, since it will simulate the bucket prices itself
	pub fn calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();

		for (bid, price) in self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket) {
			let funding_asset_id = bid.asset.to_assethub_id();
			let funding_asset_decimals = self.execute(|| T::FundingCurrency::decimals(funding_asset_id));
			let funding_asset_usd_price = self.execute(|| {
				T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
					.ok_or(Error::<T>::PriceNotFound)
					.unwrap()
			});
			let usd_ticket_size = price.saturating_mul_int(bid.amount);
			let funding_asset_spent = funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::<T>::new(
				bid.bidder.clone(),
				funding_asset_spent,
				bid.asset.to_assethub_id(),
			));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	// WARNING: Only put bids that you are sure will be done before the random end of the closing auction
	pub fn calculate_auction_funding_asset_returned_from_all_bids_made(
		&mut self,
		// bids in the order they were made
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
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
				let funding_asset_id = bid.asset.to_assethub_id();
				let funding_asset_decimals = self.execute(|| T::FundingCurrency::decimals(funding_asset_id));
				let funding_asset_usd_price = self.execute(|| {
					T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
						.ok_or(Error::<T>::PriceNotFound)
						.unwrap()
				});
				let charged_usd_ticket_size = price_charged.saturating_mul_int(bid.amount);
				let charged_usd_bond =
					bid.multiplier.calculate_bonding_requirement::<T>(charged_usd_ticket_size).unwrap();
				let charged_funding_asset =
					funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(charged_usd_bond);

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
					funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(actual_usd_bond);

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
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		weighted_average_price: PriceOf<T>,
	) -> Vec<UserToForeignAssets<T>> {
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
	pub fn filter_bids_after_auction(&self, bids: Vec<BidParams<T>>, total_cts: BalanceOf<T>) -> Vec<BidParams<T>> {
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
		&mut self,
		contributions: Vec<ContributionParams<T>>,
		token_usd_price: PriceOf<T>,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let plmc_usd_price = self.execute(|| {
			T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS).unwrap()
		});

		let mut output = Vec::new();
		for cont in contributions {
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let usd_bond = cont.multiplier.calculate_bonding_requirement::<T>(usd_ticket_size).unwrap();
			let mut plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
			if with_ed {
				plmc_bond = plmc_bond.saturating_add(self.get_ed());
			}
			output.push(UserToPLMCBalance::new(cont.contributor, plmc_bond));
		}
		output
	}

	pub fn calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
		&mut self,
		evaluations: Vec<UserToUSDBalance<T>>,
		contributions: Vec<ContributionParams<T>>,
		price: PriceOf<T>,
		slashed: bool,
		with_ed: bool,
	) -> Vec<UserToPLMCBalance<T>> {
		let evaluation_locked_plmc_amounts = self.calculate_evaluation_plmc_spent(evaluations, false);
		// how much new plmc would be locked without considering evaluation bonds
		let theoretical_contribution_locked_plmc_amounts =
			self.calculate_contributed_plmc_spent(contributions, price, false);

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
	) -> Vec<UserToForeignAssets<T>> {
		let mut output = Vec::new();
		for cont in contributions {
			let funding_asset_id = cont.asset.to_assethub_id();
			let funding_asset_decimals = self.execute(|| T::FundingCurrency::decimals(funding_asset_id));
			let funding_asset_usd_price = self.execute(|| {
				T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
					.ok_or(Error::<T>::PriceNotFound)
					.unwrap()
			});
			let usd_ticket_size = token_usd_price.saturating_mul_int(cont.amount);
			let funding_asset_spent = funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(usd_ticket_size);
			output.push(UserToForeignAssets::new(cont.contributor, funding_asset_spent, cont.asset.to_assethub_id()));
		}
		output
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

	pub fn sum_balance_mappings(&self, mut mappings: Vec<Vec<UserToPLMCBalance<T>>>) -> BalanceOf<T> {
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

	pub fn sum_foreign_mappings(&self, mut mappings: Vec<Vec<UserToForeignAssets<T>>>) -> BalanceOf<T> {
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

	pub fn generate_successful_evaluations(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		evaluators: Vec<AccountIdOf<T>>,
		weights: Vec<u8>,
	) -> Vec<UserToUSDBalance<T>> {
		let funding_target = project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		let evaluation_success_threshold = <T as Config>::EvaluationSuccessThreshold::get(); // if we use just the threshold, then for big usd targets we lose the evaluation due to PLMC conversion errors in `evaluation_end`
		let usd_threshold = evaluation_success_threshold * funding_target * 2u32.into();

		zip(evaluators, weights)
			.map(|(evaluator, weight)| {
				let ticket_size = Percent::from_percent(weight) * usd_threshold;
				(evaluator, ticket_size).into()
			})
			.collect()
	}

	pub fn generate_bids_from_total_usd(
		&self,
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

	pub fn generate_bids_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		weights: Vec<u8>,
		bidders: Vec<AccountIdOf<T>>,
		multipliers: Vec<u8>,
	) -> Vec<BidParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bid = Percent::from_percent(percent_funding) * total_allocation_size;

		assert_eq!(weights.len(), bidders.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, bidders), multipliers)
			.map(|((weight, bidder), multiplier)| {
				let token_amount = Percent::from_percent(weight) * total_ct_bid;
				BidParams::new(bidder, token_amount, multiplier, AcceptedFundingAsset::USDT)
			})
			.collect()
	}

	pub fn generate_contributions_from_total_usd(
		&self,
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

	pub fn generate_contributions_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		weights: Vec<u8>,
		contributors: Vec<AccountIdOf<T>>,
		multipliers: Vec<u8>,
	) -> Vec<ContributionParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bought = Percent::from_percent(percent_funding) * total_allocation_size;

		assert_eq!(weights.len(), contributors.len(), "Should have enough weights for all the bidders");

		zip(zip(weights, contributors), multipliers)
			.map(|((weight, contributor), multiplier)| {
				let token_amount = Percent::from_percent(weight) * total_ct_bought;
				ContributionParams::new(contributor, token_amount, multiplier, AcceptedFundingAsset::USDT)
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
