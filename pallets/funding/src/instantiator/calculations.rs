use super::*;
use crate::{MultiplierOf, ParticipationMode};
use alloc::{vec, vec::Vec};
use itertools::{izip, GroupBy};
use polimec_common::{
	assets::{
		AcceptedFundingAsset,
		AcceptedFundingAsset::{DOT, ETH, USDC, USDT},
	},
	ProvideAssetPrice,
};
use sp_core::{blake2_256, ecdsa, hexdisplay::AsBytesRef, keccak_256, sr25519, Pair};
use sp_runtime::traits::TrailingZeroInput;
use InvestorType::{self, *};

impl<
		T: Config + cumulus_pallet_parachain_system::Config,
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

	pub fn calculate_evaluation_plmc_spent(
		&mut self,
		evaluations: Vec<EvaluationParams<T>>,
	) -> Vec<UserToPLMCBalance<T>> {
		evaluations.into_iter().map(|eval| UserToPLMCBalance::new(eval.account, eval.plmc_amount)).collect()
	}

	pub fn get_actual_price_charged_for_bucketed_bids(
		&self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
		maybe_bucket: Option<BucketOf<T>>,
		// ) -> Vec<(BidParams<T>, PriceOf<T>)> {
	) -> Vec<(BidParams<T>, PriceOf<T>, Balance /* funding asset spent */, Balance /* CTs bought */)> {
		let mut output = Vec::new();
		let mut bucket = if let Some(bucket) = maybe_bucket {
			bucket
		} else {
			Pallet::<T>::create_bucket_from_metadata(&project_metadata)
		};

		for bid in bids {
			let mut funding_asset_left = bid.amount;
			let funding_asset = bid.asset;
			let funding_asset_price =
				PriceProviderOf::<T>::get_decimals_aware_price(&funding_asset.id(), funding_asset.decimals()).unwrap();
			let mut i = 0;
			while !funding_asset_left.is_zero() {
				// 1. How many CTs can we buy with the remaining funding asset at this bucket's price?
				let funding_asset_value = funding_asset_price.checked_mul_int(funding_asset_left).unwrap();
				let ct_price = bucket.current_price;
				let ct_can_buy = ct_price.reciprocal().unwrap().checked_mul_int(funding_asset_value).unwrap();

				// 2. The bucket may not have enough CTs left
				let ct_to_buy = ct_can_buy.min(bucket.amount_left);

				if ct_to_buy.is_zero() {
					break; // nothing more to buy in this bucket
				}

				// 3. How much funding asset is actually needed for ct_to_buy at this price?
				// funding_asset_needed = (ct_to_buy * ct_price) / funding_asset_price
				let usd_needed = ct_price.checked_mul_int(ct_to_buy).unwrap();
				// let funding_asset_needed = usd_needed.checked_div(&funding_asset_price).unwrap().try_into().unwrap();
				let funding_asset_needed =
					funding_asset_price.reciprocal().unwrap().checked_mul_int(usd_needed).unwrap();

				// 4. Clamp to what we have left (in case of rounding)
				let funding_asset_spent = funding_asset_needed.min(funding_asset_left);

				output.push((
					BidParams::from((bid.bidder.clone(), bid.investor_type, funding_asset_spent, bid.mode, bid.asset)),
					bucket.current_price,
					funding_asset_spent,
					ct_to_buy,
				));

				bucket.update(ct_to_buy);
				funding_asset_left = funding_asset_left.saturating_sub(funding_asset_spent);
				i += 1;
				if i > 3 {
					// Prevent infinite loop in case of rounding errors
					break;
				}
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
			let funding_asset_price =
				PriceProviderOf::<T>::get_decimals_aware_price(&bid.asset.id(), bid.asset.decimals()).unwrap();
			let usd_ticket_size = funding_asset_price.saturating_mul_int(bid.amount);
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

		for (bid, _price, _funding_asset_spent, _ct_bought) in
			self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket)
		{
			// FIX: Compute USD value from funding asset amount
			let funding_asset_price =
				PriceProviderOf::<T>::get_decimals_aware_price(&bid.asset.id(), bid.asset.decimals()).unwrap();
			let usd_ticket_size = funding_asset_price.saturating_mul_int(bid.amount);

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
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let charged_bids = self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.into_iter().group_by(|&(_, price, _, _)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price_, _, _)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let mut remaining_cts = project_metadata.total_allocation_size;

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

				let actual_usd_ticket_size = price_charged.saturating_mul_int(bought_cts);
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

	pub fn calculate_auction_funding_asset_charged_with_given_price(
		&mut self,
		bids: &Vec<BidParams<T>>,
		ct_price: PriceOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();
		for bid in bids {
			let funding_asset_price =
				PriceProviderOf::<T>::get_decimals_aware_price(&bid.asset.id(), bid.asset.decimals()).unwrap();
			let usd_ticket_size = funding_asset_price.saturating_mul_int(bid.amount);

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

		// Use the updated version that returns (bid, price, funding_asset_spent, ct_bought)
		for (bid, _price, funding_asset_spent, _ct_bought) in
			self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, maybe_bucket)
		{
			let mut total_funding_asset_spent = funding_asset_spent;

			// For OTM, add the OTM fee (which is a % of the USD value of the funding asset spent)d
			if bid.mode == ParticipationMode::OTM {
				// Convert funding_asset_spent to USD using decimals-aware price
				let funding_asset_price =
					PriceProviderOf::<T>::get_decimals_aware_price(&bid.asset.id(), bid.asset.decimals()).unwrap();
				let usd_ticket_size = funding_asset_price.saturating_mul_int(funding_asset_spent);

				self.add_otm_fee_to(&mut total_funding_asset_spent, usd_ticket_size, bid.asset);
			}

			output.push(UserToFundingAsset::<T>::new(bid.bidder.clone(), total_funding_asset_spent, bid.asset.id()));
		}

		output.merge_accounts(MergeOperation::Add)
	}

	pub fn calculate_auction_funding_asset_returned_from_all_bids_made(
		&mut self,
		bids: &Vec<BidParams<T>>,
		project_metadata: ProjectMetadataOf<T>,
	) -> Vec<UserToFundingAsset<T>> {
		use std::collections::BTreeMap;

		// Map: (bidder, asset_id) -> (total_provided, total_spent)
		let mut provided: BTreeMap<(AccountIdOf<T>, AssetIdOf<T>), Balance> = BTreeMap::new();
		let mut spent: BTreeMap<(AccountIdOf<T>, AssetIdOf<T>), Balance> = BTreeMap::new();

		// 1. Record total provided per user/asset
		for bid in bids {
			let key = (bid.bidder.clone(), bid.asset.id());
			*provided.entry(key).or_default() += bid.amount;
		}

		// 2. Record total spent per user/asset from bucketed bids
		for (bid, _price, funding_asset_spent, _ct_bought) in
			self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata, None)
		{
			let key = (bid.bidder.clone(), bid.asset.id());
			*spent.entry(key).or_default() += funding_asset_spent;
		}

		// 3. Calculate returned = provided - spent
		let mut output = Vec::new();
		for ((bidder, asset_id), provided_amount) in provided {
			let spent_amount = spent.get(&(bidder.clone(), asset_id.clone())).copied().unwrap_or(0);
			let returned = provided_amount.saturating_sub(spent_amount);
			output.push(UserToFundingAsset::<T>::new(bidder, returned, asset_id));
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
		let plmc_usd_price =
			self.execute(|| <PriceProviderOf<T>>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap());
		let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(usd_ticket_size).unwrap();
		let plmc_bond = plmc_usd_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
		let otm_fee =
			self.execute(|| <pallet_proxy_bonding::Pallet<T>>::calculate_fee(plmc_bond, funding_asset.id())).unwrap();
		*balance += otm_fee;
	}

	pub fn add_required_plmc_to(&mut self, balance: &mut Balance, usd_ticket_size: Balance, multiplier: u8) {
		let multiplier: MultiplierOf<T> = multiplier.try_into().ok().unwrap();
		let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(usd_ticket_size).unwrap();
		let plmc_usd_price =
			self.execute(|| <PriceProviderOf<T>>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap());
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

	pub fn generate_evaluations_from_total_plmc(
		&self,
		total_plmc_amount: Balance, // This is the total PLMC to be distributed
		evaluations_count: u8,
	) -> Vec<EvaluationParams<T>> {
		if evaluations_count == 0 {
			return vec![];
		}

		let mut evaluations = Vec::with_capacity(evaluations_count as usize);
		let base_weight = 100 / evaluations_count;
		let remainder = 100 % evaluations_count;

		for i in 0..evaluations_count {
			let evaluator_account = self.account_from_u32(i as u32, "EVALUATOR");
			let weight_for_evaluator = base_weight + if i < remainder { 1 } else { 0 };

			// Calculate PLMC amount for this evaluator based on weight
			let plmc_for_evaluator = Percent::from_percent(weight_for_evaluator) * total_plmc_amount;

			evaluations.push(EvaluationParams::from((evaluator_account, plmc_for_evaluator)));
		}

		evaluations
	}

	pub fn generate_successful_evaluations(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		evaluations_count: u8,
	) -> Vec<EvaluationParams<T>> {
		let funding_target_usd =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		// if we use just the threshold, then for big usd targets we lose the evaluation due to PLMC conversion errors in `evaluation_end`
		let target_usd_for_success = Percent::from_percent(100) * funding_target_usd;

		let plmc_usd_price = <PriceProviderOf<T>>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap();
		// We want to find PLMC amount such that: PLMC_amount * price_of_plmc_in_usd = target_usd_for_success
		// So, PLMC_amount = target_usd_for_success / price_of_plmc_in_usd
		// Which is target_usd_for_success * (1 / price_of_plmc_in_usd)
		let price_reciprocal =
			plmc_usd_price.reciprocal().expect("Price reciprocal failed in test; price cannot be zero");

		let total_plmc_for_success = price_reciprocal
			.checked_mul_int(target_usd_for_success)
			.expect("Failed to calculate total PLMC for success in test (multiplication error)");

		self.generate_evaluations_from_total_plmc(total_plmc_for_success, evaluations_count)
	}

	pub fn generate_failing_evaluations(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		evaluations_count: u8,
	) -> Vec<EvaluationParams<T>> {
		let funding_target = project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		// if we use just the threshold, then for big usd targets we lose the evaluation due to PLMC conversion errors in `evaluation_end`
		let evaluation_fail_percent = <T as Config>::EvaluationSuccessThreshold::get().deconstruct() / 2;

		let usd_threshold = Percent::from_percent(evaluation_fail_percent) * funding_target;
		let plmc_usd_price = <PriceProviderOf<T>>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap();
		let price_reciprocal =
			plmc_usd_price.reciprocal().expect("Price reciprocal failed in test; price cannot be zero");

		let total_plmc_for_failure = price_reciprocal
			.checked_mul_int(usd_threshold)
			.expect("Failed to calculate total PLMC for failure in test (multiplication error)");

		self.generate_evaluations_from_total_plmc(total_plmc_for_failure, evaluations_count)
	}

	pub fn generate_bids_from_total_ct_amount(
		&self,
		bids_count: u32,
		total_ct_bid: Balance,
		project_metadata: &ProjectMetadataOf<T>,
	) -> Vec<BidParams<T>> {
		let mut multipliers = (1u8..=5u8).cycle();

		let modes = (0..bids_count)
			.map(|i| {
				if i % 2 == 0 {
					ParticipationMode::Classic(multipliers.next().unwrap())
				} else {
					ParticipationMode::OTM
				}
			})
			.collect_vec();

		let investor_types =
			vec![Retail, Professional, Institutional].into_iter().cycle().take(bids_count as usize).collect_vec();
		let funding_assets =
			vec![USDT, USDC, DOT, ETH, USDT].into_iter().cycle().take(bids_count as usize).collect_vec();

		let weights = {
			if bids_count == 0 {
				return vec![];
			}
			let one = Perquintill::from_percent(100);
			let per_bid = one / bids_count;
			let mut remaining = one;
			let mut result = Vec::with_capacity(bids_count as usize);

			for _ in 0..bids_count - 1 {
				result.push(per_bid);
				remaining = remaining - per_bid;
			}
			result.push(remaining);
			result
		};

		let bidders = (0..bids_count).map(|i| self.account_from_u32(i, "BIDDER")).collect_vec();

		// Use the current bucket price for the project
		let bucket = Pallet::<T>::create_bucket_from_metadata(project_metadata);
		let ct_price = bucket.current_price;

		izip!(weights, bidders, modes, investor_types, funding_assets)
			.map(|(weight, bidder, mode, investor_type, funding_asset)| {
				let ct_amount = weight * total_ct_bid;

				// Convert ct_amount to funding asset amount
				let funding_asset_price =
					PriceProviderOf::<T>::get_decimals_aware_price(&funding_asset.id(), funding_asset.decimals())
						.unwrap();
				// funding_asset_needed = (ct_amount * ct_price) / funding_asset_price
				let usd_needed = ct_price.saturating_mul_int(ct_amount);
				let funding_asset_needed =
					funding_asset_price.reciprocal().unwrap().checked_mul_int(usd_needed).unwrap();
				BidParams::from((bidder, investor_type, funding_asset_needed, mode, funding_asset))
			})
			.collect()
	}

	pub fn generate_bids_from_total_usd(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		usd_amount: Balance,
		bids_count: u32,
	) -> Vec<BidParams<T>> {
		let min_price = project_metadata.minimum_price;
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bid = min_price.reciprocal().unwrap().saturating_mul_int(usd_amount);
		if total_ct_bid > total_allocation_size {
			panic!("This function should be used for filling only the first bucket. usd_amount given was too high!")
		}
		self.generate_bids_from_total_ct_amount(bids_count, total_ct_bid, &project_metadata)
	}

	pub fn generate_bids_from_higher_usd_than_target(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		usd_target: Balance,
	) -> Vec<BidParams<T>> {
		let mut bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata);
		bucket.update(project_metadata.total_allocation_size);

		// Increase bucket price until we go past the target usd amount
		let mut usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		while usd_raised < usd_target {
			bucket.update(bucket.delta_amount);
			usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		}

		// Go one bucket back
		bucket.current_price = bucket.current_price.saturating_sub(bucket.delta_price);
		bucket.amount_left = bucket.delta_amount;

		// Start buying the min amount of tokens in this bucket until we reach or surpass the usd amount
		let mut bids = Vec::new();
		let mut starting_account = self.account_from_u32(0, "BIDDER");
		let increment_account = |acc: AccountIdOf<T>| -> AccountIdOf<T> {
			let acc_bytes = acc.encode();
			let account_string = String::from_utf8_lossy(&acc_bytes);
			let entropy = (0, account_string).using_encoded(blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		};

		let min_ticket = project_metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation;
		let funding_asset_price = PriceProviderOf::<T>::get_decimals_aware_price(
			&AcceptedFundingAsset::USDT.id(),
			AcceptedFundingAsset::USDT.decimals(),
		)
		.expect("Price must exist in test");

		let mut current_usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);

		while current_usd_raised < usd_target {
			let funding_asset_needed =
				funding_asset_price.checked_mul_int(min_ticket).unwrap().saturating_add(1u128).try_into().unwrap();

			// How many CTs will this buy at the current bucket price?
			let usd_value = funding_asset_price.saturating_mul_int(funding_asset_needed);
			let ct_amount = bucket.current_price.reciprocal().unwrap().saturating_mul_int(usd_value);

			let bid = BidParams::<T>::from((
				starting_account.clone(),
				Retail,
				funding_asset_needed,
				AcceptedFundingAsset::USDT,
			));
			bids.push(bid);
			starting_account = increment_account(starting_account.clone());
			bucket.update(ct_amount);

			current_usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		}

		bids
	}

	pub fn generate_bids_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		bids_count: u32,
	) -> Vec<BidParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bid = Percent::from_percent(percent_funding) * total_allocation_size;

		self.generate_bids_from_total_ct_amount(bids_count, total_ct_bid, &project_metadata)
	}

	pub fn slash_evaluator_balances(&self, mut balances: Vec<UserToPLMCBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		for UserToPLMCBalance { account: _acc, plmc_amount: balance } in balances.iter_mut() {
			*balance -= slash_percentage * *balance;
		}
		balances
	}

	// We assume a single bid can cover the whole first bucket. Make sure the ticket sizes allow this.
	// TODO: This function probably is the most problematic.
	pub fn generate_bids_from_bucket(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		bucket: BucketOf<T>,
		funding_asset: AcceptedFundingAsset,
	) -> Vec<BidParams<T>> {
		let mut new_bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata);
		assert_eq!(new_bucket.delta_amount, bucket.delta_amount, "Buckets must have the same delta amount");
		assert_eq!(new_bucket.delta_price, bucket.delta_price, "Buckets must have the same delta price");
		assert_eq!(new_bucket.initial_price, bucket.initial_price, "Buckets must have the same initial price");

		let auction_allocation = project_metadata.total_allocation_size;

		let mut starting_account = self.account_from_u32(0, "BIDDER");
		let increment_account = |acc: AccountIdOf<T>| -> AccountIdOf<T> {
			let acc_bytes = acc.encode();
			let account_string = String::from_utf8_lossy(&acc_bytes);
			let entropy = (0, account_string).using_encoded(blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		};

		// Helper: convert CT amount to funding asset amount at a given price
		let ct_to_funding_asset = |ct_amount: Balance, price: PriceOf<T>| -> Balance {
			let funding_asset_price =
				PriceProviderOf::<T>::get_decimals_aware_price(&funding_asset.id(), funding_asset.decimals()).unwrap();
			let usd_needed = price.saturating_mul_int(ct_amount);
			// let funding_asset_needed = usd_needed.checked_div(&funding_asset_price).unwrap().try_into().unwrap();
			let funding_asset_needed = funding_asset_price.reciprocal().unwrap().checked_mul_int(usd_needed).unwrap();
			funding_asset_needed
		};

		let mut generate_bid = |ct_amount, price| -> BidParams<T> {
			let funding_asset_amount = ct_to_funding_asset(ct_amount, price);
			let bid = BidParams::<T>::from((starting_account.clone(), Retail, funding_asset_amount, funding_asset));
			starting_account = increment_account(starting_account.clone());
			bid
		};

		let mut bids = Vec::new();

		if bucket.current_price > bucket.initial_price {
			let allocation_bid = generate_bid(auction_allocation, bucket.current_price);
			bids.push(allocation_bid);
			new_bucket.update(auction_allocation);
		}

		while bucket.current_price > new_bucket.current_price {
			let bucket_bid = generate_bid(bucket.delta_amount, new_bucket.current_price);
			bids.push(bucket_bid);
			new_bucket.update(bucket.delta_amount);
		}

		let last_bid_amount = bucket.delta_amount - bucket.amount_left;
		let last_usd_amount = bucket.current_price.saturating_mul_int(last_bid_amount);
		if last_usd_amount >= project_metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation {
			let last_bid = generate_bid(last_bid_amount, bucket.current_price);
			bids.push(last_bid);
			new_bucket.update(last_bid_amount);
		}

		assert_eq!(new_bucket, bucket, "Buckets must match after generating bids");

		bids
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

	pub fn account_from_u32(&self, x: u32, seed: &str) -> AccountIdOf<T> {
		let entropy = (x, seed).using_encoded(blake2_256);
		Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.expect("infinite length input; no invalid inputs for type; qed")
	}
}
