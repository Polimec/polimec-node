use sp_runtime::traits::CheckedAdd;

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
	) -> Result<BalanceOf<T>, DispatchError> {
		let plmc_usd_price = T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.ok_or(Error::<T>::PriceNotFound)?;
		let usd_bond = multiplier.calculate_bonding_requirement::<T>(ticket_size).map_err(|_| Error::<T>::BadMath)?;
		plmc_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_bond)
			.ok_or(Error::<T>::BadMath.into())
	}

	pub fn calculate_funding_asset_amount(
		ticket_size: BalanceOf<T>,
		asset_id: AcceptedFundingAsset,
	) -> Result<BalanceOf<T>, DispatchError> {
		let asset_id = asset_id.to_assethub_id();
		let asset_decimals = T::FundingCurrency::decimals(asset_id);
		let asset_usd_price = T::PriceProvider::get_decimals_aware_price(asset_id, USD_DECIMALS, asset_decimals)
			.ok_or(Error::<T>::PriceNotFound)?;
		asset_usd_price
			.reciprocal()
			.and_then(|recip| recip.checked_mul_int(ticket_size))
			.ok_or(Error::<T>::BadMath.into())
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
		auction_allocation_size: BalanceOf<T>,
		wap: PriceOf<T>,
	) -> Result<(u32, u32), DispatchError> {
		let mut bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = Zero::zero();
		// sort bids by price, and equal prices sorted by id
		bids.sort_by(|a, b| b.cmp(a));
		let (accepted_bids, rejected_bids): (Vec<_>, Vec<_>) = bids
			.into_iter()
			.map(|mut bid| {
				let buyable_amount = auction_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount.is_zero() {
					bid.status = BidStatus::Rejected;
					bid.final_ct_amount = Zero::zero();
				} else if bid.original_ct_amount <= buyable_amount {
					if bid.final_ct_usd_price > wap {
						bid.final_ct_usd_price = wap;
					}
					bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
					bid.final_ct_amount = bid.original_ct_amount;
					bid.status = BidStatus::Accepted;
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
				} else {
					bid_token_amount_sum.saturating_accrue(buyable_amount);
					bid.final_ct_amount = buyable_amount;
					bid.status = BidStatus::PartiallyAccepted(buyable_amount);
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
				}
				Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
				bid
			})
			.partition(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));

		let accepted_bid_len = accepted_bids.len() as u32;
		let total_auction_allocation_usd: BalanceOf<T> = accepted_bids
			.into_iter()
			.try_fold(Zero::zero(), |acc: BalanceOf<T>, bid: BidInfoOf<T>| {
				bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).and_then(|ticket| acc.checked_add(&ticket))
			})
			.ok_or(Error::<T>::BadMath)?;

		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> DispatchResult {
			if let Some(info) = maybe_info {
				info.remaining_contribution_tokens.saturating_reduce(bid_token_amount_sum);
				info.funding_amount_reached_usd.saturating_accrue(total_auction_allocation_usd);
				info.weighted_average_price = Some(wap);

				Ok(())
			} else {
				Err(Error::<T>::ProjectDetailsNotFound.into())
			}
		})?;

		Ok((accepted_bid_len, rejected_bids.len() as u32))
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
			T::NativeCurrency::release(&HoldReason::Evaluation(project_id).into(), who, converted, Precision::Exact)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&HoldReason::Participation(project_id).into(), who, converted)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&HoldReason::Participation(project_id).into(), who, to_convert)
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

	// Calculate the total fee allocation for a project, based on the funding reached.
	fn calculate_fee_allocation(project_id: ProjectId) -> Result<BalanceOf<T>, DispatchError> {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;

		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached_usd;
		let fee_usd = Self::compute_total_fee_from_brackets(funding_amount_reached);
		let fee_percentage = Perquintill::from_rational(fee_usd, funding_amount_reached);

		let initial_token_allocation_size = project_metadata.total_allocation_size;
		let final_remaining_contribution_tokens = project_details.remaining_contribution_tokens;

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = fee_percentage * token_sold;

		Ok(total_fee_allocation)
	}

	/// Computes the total fee from all defined fee brackets.
	fn compute_total_fee_from_brackets(funding_reached: BalanceOf<T>) -> BalanceOf<T> {
		let mut remaining_for_fee = funding_reached;

		T::FeeBrackets::get()
			.into_iter()
			.map(|(fee, limit)| Self::compute_fee_for_bracket(&mut remaining_for_fee, fee, limit))
			.fold(BalanceOf::<T>::zero(), |acc, fee| acc.saturating_add(fee))
	}

	/// Calculate the fee for a particular bracket.
	fn compute_fee_for_bracket(
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
	pub fn generate_evaluator_rewards_info(project_id: ProjectId) -> Result<RewardInfoOf<T>, DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let total_fee_allocation = Self::calculate_fee_allocation(project_id)?;

		// Calculate rewards.
		let evaluator_rewards = Perquintill::from_percent(30) * total_fee_allocation;

		// Distribute rewards between early and normal evaluators.
		let early_evaluator_reward_pot = Perquintill::from_percent(20) * evaluator_rewards;
		let normal_evaluator_reward_pot = Perquintill::from_percent(80) * evaluator_rewards;

		let normal_evaluator_total_bonded_usd = project_details.evaluation_round_info.total_bonded_usd;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
		let early_evaluator_total_bonded_usd =
			normal_evaluator_total_bonded_usd.min(early_evaluation_reward_threshold_usd);

		// Construct the reward information object.
		let reward_info = RewardInfo {
			early_evaluator_reward_pot,
			normal_evaluator_reward_pot,
			early_evaluator_total_bonded_usd,
			normal_evaluator_total_bonded_usd,
		};

		Ok(reward_info)
	}

	pub fn generate_liquidity_pools_and_long_term_holder_rewards(
		project_id: ProjectId,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		let details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let total_fee_allocation = Self::calculate_fee_allocation(project_id)?;

		let percentage_of_target_funding =
			Perquintill::from_rational(details.funding_amount_reached_usd, details.fundraising_target_usd);

		let liquidity_pools_percentage = Perquintill::from_percent(50);
		let liquidity_pools_reward_pot = liquidity_pools_percentage * total_fee_allocation;

		let long_term_holder_percentage = if percentage_of_target_funding < Perquintill::from_percent(90) {
			Perquintill::from_percent(50)
		} else {
			Perquintill::from_percent(20)
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
		const MAX_RESPONSE_WEIGHT: Weight = Weight::from_parts(700_000_000, 10_000);
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
				destination: ParentThen(X1(Parachain(POLIMEC_PARA_ID))).into(),
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

	pub(crate) fn transition_project(
		project_id: ProjectId,
		mut project_details: ProjectDetailsOf<T>,
		current_round: ProjectStatus<BlockNumberFor<T>>,
		next_round: ProjectStatus<BlockNumberFor<T>>,
		round_duration: BlockNumberFor<T>,
		skip_end_check: bool,
	) -> DispatchResult {
		/* Verify */
		let now = <frame_system::Pallet<T>>::block_number();
		ensure!(project_details.round_duration.ended(now) || skip_end_check, Error::<T>::TooEarlyForRound);
		ensure!(project_details.status == current_round, Error::<T>::IncorrectRound);

		let round_end = now.saturating_add(round_duration).saturating_sub(One::one());
		project_details.round_duration.update(Some(now), Some(round_end));
		project_details.status = next_round;

		// * Update storage *
		ProjectsDetails::<T>::insert(project_id, project_details);

		// // * Emit events *
		// Self::deposit_event(Event::ProjectPhaseTransition { project_id, phase: next_round });

		Ok(())
	}
}
