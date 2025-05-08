use super::*;
use crate::{MultiplierOf, ParticipationMode};
use alloc::{vec, vec::Vec};
use itertools::{izip, GroupBy};
#[allow(clippy::wildcard_imports)]
use polimec_common::assets::AcceptedFundingAsset;
use polimec_common::{
	assets::AcceptedFundingAsset::{DOT, ETH, USDC, USDT},
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
					BidParams::from((bid.bidder.clone(), bid.investor_type, bid_amount, bid.mode, bid.asset)),
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
	) -> Vec<UserToPLMCBalance<T>> {
		let mut output = Vec::new();
		let charged_bids = self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price_)| bid).collect()))
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
	) -> Vec<UserToFundingAsset<T>> {
		let mut output = Vec::new();
		let charged_bids = self.get_actual_price_charged_for_bucketed_bids(bids, project_metadata.clone(), None);
		let grouped_by_price_bids = charged_bids.into_iter().group_by(|&(_, price)| price);
		let mut grouped_by_price_bids: Vec<(PriceOf<T>, Vec<BidParams<T>>)> = grouped_by_price_bids
			.into_iter()
			.map(|(key, group)| (key, group.map(|(bid, _price)| bid).collect()))
			.collect();
		grouped_by_price_bids.reverse();

		let mut remaining_cts = project_metadata.total_allocation_size;

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

				let actual_usd_ticket_size = price_charged.saturating_mul_int(bought_cts);
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

	pub fn generate_bids_from_total_ct_amount(&self, bids_count: u32, total_ct_bid: Balance) -> Vec<BidParams<T>> {
		// Use u128 for multipliers to allow for larger values
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

		// Use Perquintill for precise weight distribution
		let weights = {
			if bids_count == 0 {
				return vec![];
			}
			// Convert to Perquintill for higher precision division
			let one = Perquintill::from_percent(100);
			let per_bid = one / bids_count;
			let mut remaining = one;
			let mut result = Vec::with_capacity(bids_count as usize);

			// Distribute weights evenly with maximum precision
			for _ in 0..bids_count - 1 {
				result.push(per_bid);
				remaining = remaining - per_bid;
			}
			// Add remaining weight to the last bid to ensure total is exactly 100%
			result.push(remaining);
			result
		};

		let bidders = (0..bids_count).map(|i| self.account_from_u32(i, "BIDDER")).collect_vec();

		izip!(weights, bidders, modes, investor_types, funding_assets)
			.map(|(weight, bidder, mode, investor_type, funding_asset)| {
				let token_amount = weight * total_ct_bid;
				BidParams::from((bidder, investor_type, token_amount, mode, funding_asset))
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
		self.generate_bids_from_total_ct_amount(bids_count, total_ct_bid)
	}

	pub fn generate_bids_from_higher_usd_than_target(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		usd_target: Balance,
	) -> Vec<BidParams<T>> {
		let mut bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata).unwrap();
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
		while bucket.calculate_usd_raised(project_metadata.total_allocation_size) < usd_target {
			let min_ticket = project_metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation;
			let ct_min_ticket = bucket.current_price.reciprocal().unwrap().saturating_mul_int(min_ticket);
			bucket.update(ct_min_ticket);
		}

		self.generate_bids_from_bucket(project_metadata, bucket, AcceptedFundingAsset::USDT)
	}

	pub fn generate_bids_from_total_ct_percent(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		percent_funding: u8,
		bids_count: u32,
	) -> Vec<BidParams<T>> {
		let total_allocation_size = project_metadata.total_allocation_size;
		let total_ct_bid = Percent::from_percent(percent_funding) * total_allocation_size;

		self.generate_bids_from_total_ct_amount(bids_count, total_ct_bid)
	}

	pub fn slash_evaluator_balances(&self, mut balances: Vec<UserToPLMCBalance<T>>) -> Vec<UserToPLMCBalance<T>> {
		let slash_percentage = <T as Config>::EvaluatorSlash::get();
		for UserToPLMCBalance { account: _acc, plmc_amount: balance } in balances.iter_mut() {
			*balance -= slash_percentage * *balance;
		}
		balances
	}

	// We assume a single bid can cover the whole first bucket. Make sure the ticket sizes allow this.
	pub fn generate_bids_from_bucket(
		&self,
		project_metadata: ProjectMetadataOf<T>,
		bucket: BucketOf<T>,
		funding_asset: AcceptedFundingAsset,
	) -> Vec<BidParams<T>> {
		let mut new_bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata).unwrap();
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

		let mut generate_bid = |ct_amount| -> BidParams<T> {
			let bid = BidParams::<T>::from((starting_account.clone(), Retail, ct_amount, funding_asset));
			starting_account = increment_account(starting_account.clone());
			bid
		};

		let mut bids = Vec::new();

		if bucket.current_price > bucket.initial_price {
			let allocation_bid = generate_bid(auction_allocation);
			bids.push(allocation_bid);
			new_bucket.update(auction_allocation);
		}

		while bucket.current_price > new_bucket.current_price {
			let bucket_bid = generate_bid(bucket.delta_amount);
			bids.push(bucket_bid);
			new_bucket.update(bucket.delta_amount);
		}

		let last_bid_amount = bucket.delta_amount - bucket.amount_left;
		let last_usd_amount = bucket.current_price.saturating_mul_int(last_bid_amount);
		if last_usd_amount >= project_metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation {
			let last_bid = generate_bid(last_bid_amount);
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
