use super::*;

// Helper functions
// ATTENTION: if this is called directly, it will not be transactional
impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: ProjectId) -> AccountIdOf<T> {
		// since the project_id starts at 0, we need to add 1 to get a different sub_account than the pallet account.
		T::PalletId::get().into_sub_account_truncating(index.saturating_add(One::one()))
	}

	/// Adds a project to the ProjectsToUpdate storage, so it can be updated at some later point in time.
	pub fn add_to_update_store(block_number: BlockNumberFor<T>, store: (&ProjectId, UpdateType)) -> Result<u32, u32> {
		// Try to get the project into the earliest possible block to update.
		// There is a limit for how many projects can update each block, so we need to make sure we don't exceed that limit
		let mut block_number = block_number;
		for i in 1..T::MaxProjectsToUpdateInsertionAttempts::get() + 1 {
			if ProjectsToUpdate::<T>::get(block_number).is_some() {
				block_number += 1u32.into();
			} else {
				ProjectsToUpdate::<T>::insert(block_number, store);
				return Ok(i);
			}
		}
		Err(T::MaxProjectsToUpdateInsertionAttempts::get())
	}

	pub fn create_bucket_from_metadata(metadata: &ProjectMetadataOf<T>) -> Result<BucketOf<T>, DispatchError> {
		let auction_allocation_size = metadata.auction_round_allocation_percentage * metadata.total_allocation_size;
		let bucket_delta_amount = Percent::from_percent(10) * auction_allocation_size;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).ok_or(Error::<T>::BadMath)?;
		let bucket_delta_price: <T as Config>::Price = metadata.minimum_price.saturating_mul(ten_percent_in_price);

		let bucket: BucketOf<T> =
			Bucket::new(auction_allocation_size, metadata.minimum_price, bucket_delta_price, bucket_delta_amount);

		Ok(bucket)
	}

	pub fn calculate_plmc_bond(
		ticket_size: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		plmc_price: PriceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let usd_bond = multiplier.calculate_bonding_requirement::<T>(ticket_size).map_err(|_| Error::<T>::BadMath)?;
		plmc_price.reciprocal().ok_or(Error::<T>::BadMath)?.checked_mul_int(usd_bond).ok_or(Error::<T>::BadMath.into())
	}

	// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_info(
		_caller: &AccountIdOf<T>,
		multiplier: MultiplierOf<T>,
		bonded_amount: BalanceOf<T>,
	) -> Result<VestingInfo<BlockNumberFor<T>, BalanceOf<T>>, DispatchError> {
		let duration: BlockNumberFor<T> = multiplier.calculate_vesting_duration::<T>();
		let duration_as_balance = T::BlockNumberToBalance::convert(duration);
		let amount_per_block = if duration_as_balance == Zero::zero() {
			bonded_amount
		} else {
			bonded_amount.checked_div(&duration_as_balance).ok_or(Error::<T>::BadMath)?
		};

		Ok(VestingInfo { total_amount: bonded_amount, amount_per_block, duration })
	}

	pub fn decide_winning_bids(
		project_id: ProjectId,
		end_block: BlockNumberFor<T>,
		auction_allocation_size: BalanceOf<T>,
	) -> Result<(u32, u32), DispatchError> {
		// Get all the bids that were made before the end of the closing period.
		let mut bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = Zero::zero();
		// temp variable to store the total value of the bids (i.e price * amount = Cumulative Ticket Size)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();
		let project_account = Self::fund_account_id(project_id);

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let mut highest_accepted_price = project_metadata.minimum_price;

		// sort bids by price, and equal prices sorted by id
		bids.sort_by(|a, b| b.cmp(a));
		// accept only bids that were made before `end_block` i.e end of the the auction candle.
		let (accepted_bids, rejected_bids): (Vec<_>, Vec<_>) = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					bid.status = BidStatus::Rejected(RejectionReason::AfterClosingEnd);
					return bid;
				}
				let buyable_amount = auction_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount.is_zero() {
					bid.status = BidStatus::Rejected(RejectionReason::NoTokensLeft);
				} else if bid.original_ct_amount <= buyable_amount {
					let ticket_size = bid.original_ct_usd_price.saturating_mul_int(bid.original_ct_amount);
					bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
					bid_usd_value_sum.saturating_accrue(ticket_size);
					bid.final_ct_amount = bid.original_ct_amount;
					bid.status = BidStatus::Accepted;
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
					highest_accepted_price = highest_accepted_price.max(bid.original_ct_usd_price);
				} else {
					let ticket_size = bid.original_ct_usd_price.saturating_mul_int(buyable_amount);
					bid_usd_value_sum.saturating_accrue(ticket_size);
					bid_token_amount_sum.saturating_accrue(buyable_amount);
					bid.status = BidStatus::PartiallyAccepted(buyable_amount, RejectionReason::NoTokensLeft);
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
					bid.final_ct_amount = buyable_amount;
					highest_accepted_price = highest_accepted_price.max(bid.original_ct_usd_price);
				}
				Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
				bid
			})
			.partition(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));

		// Refund rejected bids. We do it here, so we don't have to calculate all the project
		// prices and then fail to refund the bids.
		let total_rejected_bids = rejected_bids.len() as u32;
		for bid in rejected_bids.into_iter() {
			Self::refund_bid(&bid, project_id, &project_account)?;
			Bids::<T>::remove((project_id, &bid.bidder, &bid.id));
		}

		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> DispatchResult {
			if let Some(info) = maybe_info {
				info.remaining_contribution_tokens.saturating_reduce(bid_token_amount_sum);
				if highest_accepted_price > project_metadata.minimum_price {
					info.usd_bid_on_oversubscription = Some(bid_usd_value_sum);
				}
				Ok(())
			} else {
				Err(Error::<T>::ProjectDetailsNotFound.into())
			}
		})?;

		Ok((accepted_bids.len() as u32, total_rejected_bids))
	}

	/// Calculates the price (in USD) of contribution tokens for the Community and Remainder Rounds
	pub fn calculate_weighted_average_price(project_id: ProjectId) -> Result<u32, DispatchError> {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// Rejected bids were deleted in the previous block.
		let accepted_bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		let project_account = Self::fund_account_id(project_id);
		let plmc_price = T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.ok_or(Error::<T>::PriceNotFound)?;

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

		// After reading from storage all accepted bids when calculating the weighted price of each bid, we store them here
		let mut weighted_token_price = if let Some(total_usd_bid) = project_details.usd_bid_on_oversubscription {
			let calc_weighted_price_fn = |bid: &BidInfoOf<T>| -> PriceOf<T> {
				let ticket_size = bid.original_ct_usd_price.saturating_mul_int(bid.final_ct_amount);
				let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(ticket_size, total_usd_bid);
				let weighted_price = bid.original_ct_usd_price.saturating_mul(bid_weight);
				weighted_price
			};

			accepted_bids
				.iter()
				.map(calc_weighted_price_fn)
				.fold(Zero::zero(), |a: PriceOf<T>, b: PriceOf<T>| a.saturating_add(b))
		} else {
			project_metadata.minimum_price
		};

		// We are 99% sure that the price cannot be less than the minimum if some accepted bids have higher price, but rounding
		// errors are strange, so we keep this just in case.
		if weighted_token_price < project_metadata.minimum_price {
			weighted_token_price = project_metadata.minimum_price;
		};

		let mut final_total_funding_reached_by_bids = BalanceOf::<T>::zero();

		let total_accepted_bids = accepted_bids.len() as u32;
		for mut bid in accepted_bids {
			if bid.final_ct_usd_price > weighted_token_price || matches!(bid.status, BidStatus::PartiallyAccepted(..)) {
				if bid.final_ct_usd_price > weighted_token_price {
					bid.final_ct_usd_price = weighted_token_price;
				}

				let new_ticket_size =
					bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;

				let funding_asset_id = bid.funding_asset.to_assethub_id();
				let funding_asset_decimals = T::FundingCurrency::decimals(funding_asset_id);
				let funding_asset_usd_price =
					T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
						.ok_or(Error::<T>::PriceNotFound)?;

				let funding_asset_amount_needed = funding_asset_usd_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(new_ticket_size)
					.ok_or(Error::<T>::BadMath)?;

				let amount_returned = bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed);
				let asset_id = bid.funding_asset.to_assethub_id();
				let min_amount = T::FundingCurrency::minimum_balance(asset_id);
				// Transfers of less than min_amount return an error
				if amount_returned > min_amount {
					T::FundingCurrency::transfer(
						bid.funding_asset.to_assethub_id(),
						&project_account,
						&bid.bidder,
						amount_returned,
						Preservation::Preserve,
					)?;
					bid.funding_asset_amount_locked = funding_asset_amount_needed;
				}

				let usd_bond_needed = bid
					.multiplier
					.calculate_bonding_requirement::<T>(new_ticket_size)
					.map_err(|_| Error::<T>::BadMath)?;
				let plmc_bond_needed = plmc_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(usd_bond_needed)
					.ok_or(Error::<T>::BadMath)?;

				let plmc_bond_returned = bid.plmc_bond.saturating_sub(plmc_bond_needed);
				// If the free balance of a user is zero and we want to send him less than ED, it will fail.
				if plmc_bond_returned > T::ExistentialDeposit::get() {
					T::NativeCurrency::release(
						&HoldReason::Participation.into(), // TODO: Check the `Reason`
						&bid.bidder,
						plmc_bond_returned,
						Precision::Exact,
					)?;
				}

				bid.plmc_bond = plmc_bond_needed;
			}
			let final_ticket_size =
				bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;
			final_total_funding_reached_by_bids.saturating_accrue(final_ticket_size);
			Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
		}

		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> DispatchResult {
			if let Some(info) = maybe_info {
				info.weighted_average_price = Some(weighted_token_price);
				info.funding_amount_reached_usd.saturating_accrue(final_total_funding_reached_by_bids);
				Ok(())
			} else {
				Err(Error::<T>::ProjectDetailsNotFound.into())
			}
		})?;

		Ok(total_accepted_bids)
	}

	/// Refund a bid because of `reason`.
	fn refund_bid(
		bid: &BidInfoOf<T>,
		project_id: ProjectId,
		project_account: &AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		T::FundingCurrency::transfer(
			bid.funding_asset.to_assethub_id(),
			project_account,
			&bid.bidder,
			bid.funding_asset_amount_locked,
			Preservation::Expendable,
		)?;
		T::NativeCurrency::release(
			&HoldReason::Participation.into(), // TODO: Check the `Reason`
			&bid.bidder,
			bid.plmc_bond,
			Precision::Exact,
		)?;

		// Refund bid should only be called when the bid is rejected, so this if let should
		// always match.
		if let BidStatus::Rejected(reason) = bid.status {
			Self::deposit_event(Event::BidRefunded {
				project_id,
				account: bid.bidder.clone(),
				bid_id: bid.id,
				reason,
				plmc_amount: bid.plmc_bond,
				funding_asset: bid.funding_asset,
				funding_amount: bid.funding_asset_amount_locked,
			});
		}

		Ok(())
	}

	pub fn select_random_block(
		closing_starting_block: BlockNumberFor<T>,
		closing_ending_block: BlockNumberFor<T>,
	) -> BlockNumberFor<T> {
		let nonce = Self::get_and_increment_nonce();
		let (random_value, _known_since) = T::Randomness::random(&nonce);
		let random_block = <BlockNumberFor<T>>::decode(&mut random_value.as_ref())
			.expect("secure hashes should always be bigger than the block number; qed");
		let block_range = closing_ending_block - closing_starting_block;

		closing_starting_block + (random_block % block_range)
	}

	fn get_and_increment_nonce() -> Vec<u8> {
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce.wrapping_add(1));
		nonce.encode()
	}

	/// People that contributed to the project during the Funding Round can claim their Contribution Tokens
	// This function is kept separate from the `do_claim_contribution_tokens` for easier testing the logic
	#[inline(always)]
	pub fn calculate_claimable_tokens(
		contribution_amount: BalanceOf<T>,
		weighted_average_price: BalanceOf<T>,
	) -> FixedU128 {
		FixedU128::saturating_from_rational(contribution_amount, weighted_average_price)
	}

	pub fn try_plmc_participation_lock(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		// Check if the user has already locked tokens in the evaluation period
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, who));

		let mut to_convert = amount;
		for mut evaluation in user_evaluations {
			if to_convert == Zero::zero() {
				break;
			}
			let slash_deposit = <T as Config>::EvaluatorSlash::get() * evaluation.original_plmc_bond;
			let available_to_convert = evaluation.current_plmc_bond.saturating_sub(slash_deposit);
			let converted = to_convert.min(available_to_convert);
			evaluation.current_plmc_bond = evaluation.current_plmc_bond.saturating_sub(converted);
			Evaluations::<T>::insert((project_id, who, evaluation.id), evaluation);
			T::NativeCurrency::release(&HoldReason::Evaluation.into(), who, converted, Precision::Exact) // TODO: Check the `Reason`
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&HoldReason::Participation.into(), who, converted) // TODO: Check the `Reason`
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&HoldReason::Participation.into(), who, to_convert) // TODO: Check the `Reason`
			.map_err(|_| Error::<T>::ParticipantNotEnoughFunds)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: BalanceOf<T>,
		asset_id: AssetIdOf<T>,
	) -> DispatchResult {
		let fund_account = Self::fund_account_id(project_id);
		// Why `Preservation::Expendable`?
		// the min_balance of funding assets (e.g USDT) are low enough so we don't expect users to care about their balance being dusted.
		// We do think the UX would be bad if they cannot use all of their available tokens.
		// Specially since a new funding asset account can be easily created by increasing the provider reference
		T::FundingCurrency::transfer(asset_id, who, &fund_account, amount, Preservation::Expendable)
			.map_err(|_| Error::<T>::ParticipantNotEnoughFunds)?;

		Ok(())
	}

	/// Calculate the total fees based on the funding reached.
	pub fn calculate_fees(funding_reached: BalanceOf<T>) -> Perquintill {
		let total_fee = Self::compute_total_fee_from_brackets(funding_reached);
		Perquintill::from_rational(total_fee, funding_reached)
	}

	/// Computes the total fee from all defined fee brackets.
	pub fn compute_total_fee_from_brackets(funding_reached: BalanceOf<T>) -> BalanceOf<T> {
		let mut remaining_for_fee = funding_reached;

		T::FeeBrackets::get()
			.into_iter()
			.map(|(fee, limit)| Self::compute_fee_for_bracket(&mut remaining_for_fee, fee, limit))
			.fold(BalanceOf::<T>::zero(), |acc, fee| acc.saturating_add(fee))
	}

	/// Calculate the fee for a particular bracket.
	pub fn compute_fee_for_bracket(
		remaining_for_fee: &mut BalanceOf<T>,
		fee: Percent,
		limit: BalanceOf<T>,
	) -> BalanceOf<T> {
		if let Some(amount_to_bid) = remaining_for_fee.checked_sub(&limit) {
			*remaining_for_fee = amount_to_bid;
			fee * limit
		} else {
			let fee_for_this_bracket = fee * *remaining_for_fee;
			*remaining_for_fee = BalanceOf::<T>::zero();
			fee_for_this_bracket
		}
	}

	/// Generate and return evaluator rewards based on a project's funding status.
	///
	/// The function calculates rewards based on several metrics: funding achieved,
	/// total allocations, and issuer fees. It also differentiates between early and
	/// normal evaluators for reward distribution.
	///
	/// Note: Consider refactoring the `RewardInfo` struct to make it more generic and
	/// reusable, not just for evaluator rewards.
	pub fn generate_evaluator_rewards_info(project_id: ProjectId) -> Result<(RewardInfoOf<T>, u32), DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let evaluations = Evaluations::<T>::iter_prefix((project_id,)).collect::<Vec<_>>();
		// used for weight calculation
		let evaluations_count = evaluations.len() as u32;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached_usd;
		let fundraising_target = project_details.fundraising_target_usd;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		let initial_token_allocation_size = project_metadata.total_allocation_size;
		let final_remaining_contribution_tokens = project_details.remaining_contribution_tokens;

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		// A.K.A variable "Y" in the documentation. We mean it to saturate to 1 even if the ratio is above 1 when funding raised
		// is above the target.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);

		// Calculate rewards.
		let evaluator_rewards = percentage_of_target_funding * Perquintill::from_percent(30) * total_fee_allocation;

		// Distribute rewards between early and normal evaluators.
		let early_evaluator_reward_pot = Perquintill::from_percent(20) * evaluator_rewards;
		let normal_evaluator_reward_pot = Perquintill::from_percent(80) * evaluator_rewards;

		// Sum up the total bonded USD amounts for both early and late evaluators.
		let early_evaluator_total_bonded_usd =
			evaluations.iter().fold(BalanceOf::<T>::zero(), |acc, ((_evaluator, _id), evaluation)| {
				acc.saturating_add(evaluation.early_usd_amount)
			});
		let late_evaluator_total_bonded_usd =
			evaluations.iter().fold(BalanceOf::<T>::zero(), |acc, ((_evaluator, _id), evaluation)| {
				acc.saturating_add(evaluation.late_usd_amount)
			});

		let normal_evaluator_total_bonded_usd =
			early_evaluator_total_bonded_usd.saturating_add(late_evaluator_total_bonded_usd);

		// Construct the reward information object.
		let reward_info = RewardInfo {
			early_evaluator_reward_pot,
			normal_evaluator_reward_pot,
			early_evaluator_total_bonded_usd,
			normal_evaluator_total_bonded_usd,
		};

		Ok((reward_info, evaluations_count))
	}

	pub fn generate_liquidity_pools_and_long_term_holder_rewards(
		project_id: ProjectId,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached_usd;
		let fundraising_target = project_details.fundraising_target_usd;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		let initial_token_allocation_size = project_metadata.total_allocation_size;
		let final_remaining_contribution_tokens = project_details.remaining_contribution_tokens;

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		// A.K.A variable "Y" in the documentation. We mean it to saturate to 1 even if the ratio is above 1 when funding raised
		// is above the target.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);
		let inverse_percentage_of_target_funding = Perquintill::from_percent(100) - percentage_of_target_funding;

		let liquidity_pools_percentage = Perquintill::from_percent(50);
		let liquidity_pools_reward_pot = liquidity_pools_percentage * total_fee_allocation;

		let long_term_holder_percentage = if percentage_of_target_funding < Perquintill::from_percent(90) {
			Perquintill::from_percent(50)
		} else {
			Perquintill::from_percent(20) + Perquintill::from_percent(30) * inverse_percentage_of_target_funding
		};
		let long_term_holder_reward_pot = long_term_holder_percentage * total_fee_allocation;

		Ok((liquidity_pools_reward_pot, long_term_holder_reward_pot))
	}

	pub fn migrations_per_xcm_message_allowed() -> u32 {
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		let one_migration_bytes = (0u128, 0u64).encode().len() as u32;

		// our encoded call starts with pallet index 51, and call index 0
		let mut encoded_call = vec![51u8, 0];
		let encoded_first_param = [0u8; 32].encode();
		let encoded_second_param = Vec::<MigrationInfo>::new().encode();
		// we append the encoded parameters, with our migrations vec being empty for now
		encoded_call.extend_from_slice(encoded_first_param.as_slice());
		encoded_call.extend_from_slice(encoded_second_param.as_slice());

		let base_xcm_message: Xcm<()> = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Native, require_weight_at_most: MAX_WEIGHT, call: encoded_call.into() },
			ReportTransactStatus(QueryResponseInfo {
				destination: Parachain(3344).into(),
				query_id: 0,
				max_weight: MAX_WEIGHT,
			}),
		]);
		let xcm_size = base_xcm_message.encode().len();

		let available_bytes_for_migration_per_message =
			T::RequiredMaxMessageSize::get().saturating_sub(xcm_size as u32);

		available_bytes_for_migration_per_message.saturating_div(one_migration_bytes)
	}

	/// Check if the user has no participations (left) in the project.
	pub fn user_has_no_participations(project_id: ProjectId, user: AccountIdOf<T>) -> bool {
		Evaluations::<T>::iter_prefix_values((project_id, user.clone())).next().is_none() &&
			Bids::<T>::iter_prefix_values((project_id, user.clone())).next().is_none() &&
			Contributions::<T>::iter_prefix_values((project_id, user)).next().is_none()
	}

	pub fn construct_migration_xcm_message(
		migrations: BoundedVec<Migration, MaxParticipationsPerUser<T>>,
		query_id: QueryId,
		pallet_index: PalletIndex,
	) -> Xcm<()> {
		// TODO: adjust this as benchmarks for polimec-receiver are written
		const MAX_WEIGHT: Weight = Weight::from_parts(10_000, 0);
		const MAX_RESPONSE_WEIGHT: Weight = Weight::from_parts(1_000_000_000, 50_000);
		let migrations_item = Migrations::from(migrations.into());

		// First byte is the pallet index, second byte is the call index
		let mut encoded_call = vec![pallet_index, 0];

		// migrations_item can contain a Maximum of MaxParticipationsPerUser migrations which
		// is 48. So we know that there is an upper limit to this encoded call, namely 48 *
		// Migration encode size.
		encoded_call.extend_from_slice(migrations_item.encode().as_slice());
		Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Native, require_weight_at_most: MAX_WEIGHT, call: encoded_call.into() },
			ReportTransactStatus(QueryResponseInfo {
				destination: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into(),
				query_id,
				max_weight: MAX_RESPONSE_WEIGHT,
			}),
		])
	}

	pub fn change_migration_status(
		project_id: ProjectId,
		user: T::AccountId,
		status: MigrationStatus,
	) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let (current_status, migrations) =
			UserMigrations::<T>::get((project_id, user.clone())).ok_or(Error::<T>::NoMigrationsFound)?;

		let status = match status {
			MigrationStatus::Sent(_)
				if matches!(current_status, MigrationStatus::NotStarted | MigrationStatus::Failed) =>
				status,
			MigrationStatus::Confirmed
				if matches!(project_details.migration_type, Some(MigrationType::Offchain)) ||
					(matches!(project_details.migration_type, Some(MigrationType::Pallet(_))) &&
						matches!(current_status, MigrationStatus::Sent(_))) =>
			{
				UnmigratedCounter::<T>::mutate(project_id, |counter| *counter = counter.saturating_sub(1));
				status
			},
			MigrationStatus::Failed if matches!(current_status, MigrationStatus::Sent(_)) => status,

			_ => return Err(Error::<T>::NotAllowed.into()),
		};
		UserMigrations::<T>::insert((project_id, user), (status, migrations));
		ProjectsDetails::<T>::insert(project_id, project_details);

		Ok(())
	}
}
