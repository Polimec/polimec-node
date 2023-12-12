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
use super::*;

impl<T: Config> Pallet<T> {

    /// Called automatically by on_initialize
	/// Starts the community round for a project.
	/// Retail users now buy tokens instead of bidding on them. The price of the tokens are calculated
	/// based on the available bids, using the function [`calculate_weighted_average_price`](Self::calculate_weighted_average_price).
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block is after the candle auction end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Community Funding round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// round.
	///
	/// # Next step
	/// Retail users buy tokens at the price set on the auction round.
	/// Later on, `on_initialize` ends the community round by calling [`do_remainder_funding`](Self::do_remainder_funding) and
	/// starts the remainder round, where anyone can buy at that price point.
	pub fn do_community_funding(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_candle_start_block =
			project_details.phase_transition_points.candle_auction.start().ok_or(Error::<T>::FieldIsNone)?;
		let auction_candle_end_block =
			project_details.phase_transition_points.candle_auction.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > auction_candle_end_block, Error::<T>::TooEarlyForCommunityRoundStart);
		ensure!(
			project_details.status == ProjectStatus::AuctionRound(AuctionPhase::Candle),
			Error::<T>::ProjectNotInCandleAuctionRound
		);

		// * Calculate new variables *
		let end_block = Self::select_random_block(auction_candle_start_block, auction_candle_end_block);
		let community_start_block = now + 1u32.into();
		let community_end_block = now + T::CommunityFundingDuration::get();

		// * Update Storage *
		let calculation_result =
			Self::calculate_weighted_average_price(project_id, end_block, project_metadata.total_allocation_size.0);
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		match calculation_result {
			Err(pallet_error) if pallet_error == Error::<T>::NoBidsFound.into() => {
				project_details.status = ProjectStatus::FundingFailed;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Self::add_to_update_store(
					<frame_system::Pallet<T>>::block_number() + 1u32.into(),
					(&project_id, UpdateType::FundingEnd),
				);

				// * Emit events *
				Self::deposit_event(Event::AuctionFailed { project_id });

				Ok(())
			},
			e @ Err(_) => e,
			Ok(()) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_candle_ending = Some(end_block);
				project_details
					.phase_transition_points
					.community
					.update(Some(community_start_block), Some(community_end_block));
				project_details.status = ProjectStatus::CommunityRound;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Self::add_to_update_store(
					community_end_block + 1u32.into(),
					(&project_id, UpdateType::RemainderFundingStart),
				);

				// * Emit events *
				Self::deposit_event(Event::CommunityFundingStarted { project_id });

				Ok(())
			},
		}
	}

	/// Called automatically by on_initialize
	/// Starts the remainder round for a project.
	/// Anyone can now buy tokens, until they are all sold out, or the time is reached.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, the current block is after the community funding end period, and there are still tokens left to sell.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Remainder Funding round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// round.
	///
	/// # Next step
	/// Any users can now buy tokens at the price set on the auction round.
	/// Later on, `on_initialize` ends the remainder round, and finalizes the project funding, by calling
	/// [`do_end_funding`](Self::do_end_funding).
	pub fn do_remainder_funding(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block =
			project_details.phase_transition_points.community.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::ProjectNotInCommunityRound);

		// * Calculate new variables *
		let remainder_start_block = now + 1u32.into();
		let remainder_end_block = now + T::RemainderFundingDuration::get();

		// * Update Storage *
		project_details
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block));
		project_details.status = ProjectStatus::RemainderRound;
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic transition by `on_initialize`
		Self::add_to_update_store(remainder_end_block + 1u32.into(), (&project_id, UpdateType::FundingEnd));

		// * Emit events *
		Self::deposit_event(Event::RemainderFundingStarted { project_id });

		Ok(())
	}

	/// Called automatically by on_initialize
	/// Ends the project funding, and calculates if the project was successfully funded or not.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, the current block is after the remainder funding end period.
	/// Update the project information with the new round status.
	///
	/// # Success Path
	/// The validity checks pass, and either of 2 paths happen:
	///
	/// * Project achieves its funding target - the project info is set to a successful funding state,
	/// and the contribution token asset class is created with the same id as the project.
	///
	/// * Project doesn't achieve its funding target - the project info is set to an unsuccessful funding state.
	///
	/// # Next step
	/// If **successful**, bidders can claim:
	///	* Contribution tokens with [`vested_contribution_token_bid_mint_for`](Self::vested_contribution_token_bid_mint_for)
	/// * Bonded plmc with [`vested_plmc_bid_unbond_for`](Self::vested_plmc_bid_unbond_for)
	///
	/// And contributors can claim:
	/// * Contribution tokens with [`vested_contribution_token_purchase_mint_for`](Self::vested_contribution_token_purchase_mint_for)
	/// * Bonded plmc with [`vested_plmc_purchase_unbond_for`](Self::vested_plmc_purchase_unbond_for)
	///
	/// If **unsuccessful**, users every user should have their PLMC vesting unbonded.
	pub fn do_end_funding(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let remaining_cts = project_details
			.remaining_contribution_tokens
			.0
			.saturating_add(project_details.remaining_contribution_tokens.1);
		let remainder_end_block = project_details.phase_transition_points.remainder.end();
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(
			remaining_cts == Zero::zero() ||
				project_details.status == ProjectStatus::FundingFailed ||
				matches!(remainder_end_block, Some(end_block) if now > end_block),
			Error::<T>::TooEarlyForFundingEnd
		);

		// * Calculate new variables *
		let funding_target = project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size.0)
			.ok_or(Error::<T>::BadMath)?;
		let funding_reached = project_details.funding_amount_reached;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		if funding_ratio <= Perquintill::from_percent(33u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			Self::make_project_funding_fail(project_id, project_details, FailureReason::TargetNotReached, 1u32.into())
		} else if funding_ratio <= Perquintill::from_percent(75u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			);
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(())
		} else if funding_ratio < Perquintill::from_percent(90u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Unchanged;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			);
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(())
		} else {
			let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Rewarded(reward_info);
			Self::make_project_funding_successful(
				project_id,
				project_details,
				SuccessReason::ReachedTarget,
				T::SuccessToSettlementTime::get(),
			)
		}
	}

    /// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens to buy
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the issuer is not a contributor
	/// * [`ProjectsDetails`] - Check that the project is in the Community Round, and the amount is big
	/// enough to buy at least 1 token
	/// * [`Contributions`] - Update storage with the new contribution
	/// * [`T::NativeCurrency`] - Update the balance of the contributor and the project pot
	pub fn do_contribute(
		contributor: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
	) -> DispatchResultWithPostInfo {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_metadata.participation_currencies == asset, Error::<T>::FundingAssetNotAccepted);
		ensure!(contributor.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(
			project_details.status == ProjectStatus::CommunityRound ||
				project_details.status == ProjectStatus::RemainderRound,
			Error::<T>::AuctionNotStarted
		);

		let now = <frame_system::Pallet<T>>::block_number();

		let ct_usd_price = project_details.weighted_average_price.ok_or(Error::<T>::AuctionNotStarted)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PriceNotFound)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;

		// * Calculate variables *
		let buyable_tokens = Self::calculate_buyable_amount(
			&project_details.status,
			token_amount,
			project_details.remaining_contribution_tokens,
		);
		let ticket_size = ct_usd_price.checked_mul_int(buyable_tokens).ok_or(Error::<T>::BadMath)?;
		if let Some(minimum_ticket_size) = project_metadata.ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(ticket_size >= minimum_ticket_size, Error::<T>::ContributionTooLow);
		};
		if let Some(maximum_ticket_size) = project_metadata.ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(ticket_size <= maximum_ticket_size, Error::<T>::ContributionTooHigh);
		};

		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price)?;
		let funding_asset_amount =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = asset.to_statemint_id();

		let contribution_id = Self::next_contribution_id();
		let new_contribution = ContributionInfoOf::<T> {
			id: contribution_id,
			project_id,
			contributor: contributor.clone(),
			ct_amount: buyable_tokens,
			usd_contribution_amount: ticket_size,
			multiplier,
			funding_asset: asset,
			funding_asset_amount,
			plmc_bond,
			plmc_vesting_info: None,
			funds_released: false,
			ct_minted: false,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		// * Update storage *
		// Try adding the new contribution to the system
		let existing_contributions =
			Contributions::<T>::iter_prefix_values((project_id, contributor)).collect::<Vec<_>>();
		if existing_contributions.len() < T::MaxContributionsPerUser::get() as usize {
			Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
			Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;
		} else {
			let lowest_contribution = existing_contributions
				.iter()
				.min_by_key(|contribution| contribution.plmc_bond)
				.ok_or(Error::<T>::ImpossibleState)?;

			ensure!(new_contribution.plmc_bond > lowest_contribution.plmc_bond, Error::<T>::ContributionTooLow);

			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&lowest_contribution.contributor,
				lowest_contribution.plmc_bond,
				Precision::Exact,
			)?;
			T::FundingCurrency::transfer(
				asset_id,
				&Self::fund_account_id(project_id),
				&lowest_contribution.contributor,
				lowest_contribution.funding_asset_amount,
				Preservation::Expendable,
			)?;
			Contributions::<T>::remove((project_id, &lowest_contribution.contributor, &lowest_contribution.id));

			Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
			Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;

			project_details.remaining_contribution_tokens.1.saturating_accrue(lowest_contribution.ct_amount);
			project_details.funding_amount_reached.saturating_reduce(lowest_contribution.usd_contribution_amount);
		}

		Contributions::<T>::insert((project_id, contributor, contribution_id), &new_contribution);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));

		// Update remaining contribution tokens
		if project_details.status == ProjectStatus::CommunityRound {
			project_details.remaining_contribution_tokens.1.saturating_reduce(new_contribution.ct_amount);
		} else {
			let before = project_details.remaining_contribution_tokens.0;
			let remaining_cts_in_round = before.saturating_sub(new_contribution.ct_amount);
			project_details.remaining_contribution_tokens.0 = remaining_cts_in_round;

			// If the entire ct_amount could not be subtracted from remaining_contribution_tokens.0, subtract the difference from remaining_contribution_tokens.1
			if remaining_cts_in_round.is_zero() {
				let difference = new_contribution.ct_amount.saturating_sub(before);
				project_details.remaining_contribution_tokens.1.saturating_reduce(difference);
			}
		}

		let remaining_cts_after_purchase = project_details
			.remaining_contribution_tokens
			.0
			.saturating_add(project_details.remaining_contribution_tokens.1);
		project_details.funding_amount_reached.saturating_accrue(new_contribution.usd_contribution_amount);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// If no CTs remain, end the funding phase
		if remaining_cts_after_purchase.is_zero() {
			Self::remove_from_update_store(&project_id)?;
			Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::FundingEnd));
		}

		// * Emit events *
		Self::deposit_event(Event::Contribution {
			project_id,
			contributor: contributor.clone(),
			amount: token_amount,
			multiplier,
		});

		Ok(Pays::No.into())
	}

    fn calculate_buyable_amount(
		status: &ProjectStatus,
		amount: BalanceOf<T>,
		remaining_contribution_tokens: (BalanceOf<T>, BalanceOf<T>),
	) -> BalanceOf<T> {
		match status {
			ProjectStatus::CommunityRound =>
				if amount <= remaining_contribution_tokens.1 {
					amount
				} else {
					remaining_contribution_tokens.1
				},
			ProjectStatus::RemainderRound => {
				let sum = remaining_contribution_tokens.0.saturating_add(remaining_contribution_tokens.1);
				if sum >= amount {
					amount
				} else {
					sum
				}
			},
			_ => Zero::zero(),
		}
	}
	
    pub fn do_set_para_id_for_project(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		para_id: ParaId,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(&(project_details.issuer) == caller, Error::<T>::NotAllowed);

		// * Update storage *
		project_details.parachain_id = Some(para_id);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::ProjectParaIdSet { project_id, para_id, caller: caller.clone() });

		Ok(())
	}


    /// Calculates the price (in USD) of contribution tokens for the Community and Remainder Rounds
	pub fn calculate_weighted_average_price(
		project_id: T::ProjectIdentifier,
		end_block: BlockNumberFor<T>,
		total_allocation_size: BalanceOf<T>,
	) -> DispatchResult {
		// Get all the bids that were made before the end of the candle
		let mut bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = Zero::zero();
		// temp variable to store the total value of the bids (i.e price * amount = Cumulative Ticket Size)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();
		let project_account = Self::fund_account_id(project_id);
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		// sort bids by price, and equal prices sorted by id
		bids.sort_by(|a, b| b.cmp(a));
		// accept only bids that were made before `end_block` i.e end of candle auction
		let bids: Result<Vec<_>, DispatchError> = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::AfterCandleEnd)
						.and(Ok(bid))
				}
				let buyable_amount = total_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount.is_zero() {
					return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::NoTokensLeft)
						.and(Ok(bid))
				} else if bid.original_ct_amount <= buyable_amount {
					let maybe_ticket_size = bid.original_ct_usd_price.checked_mul_int(bid.original_ct_amount);
					if let Some(ticket_size) = maybe_ticket_size {
						bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
						bid_usd_value_sum.saturating_accrue(ticket_size);
						bid.status = BidStatus::Accepted;
					} else {
						return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::BadMath)
							.and(Ok(bid))
					}
				} else {
					let maybe_ticket_size = bid.original_ct_usd_price.checked_mul_int(buyable_amount);
					if let Some(ticket_size) = maybe_ticket_size {
						bid_usd_value_sum.saturating_accrue(ticket_size);
						bid_token_amount_sum.saturating_accrue(buyable_amount);
						bid.status = BidStatus::PartiallyAccepted(buyable_amount, RejectionReason::NoTokensLeft);
						bid.final_ct_amount = buyable_amount;

						let funding_asset_price = T::PriceProvider::get_price(bid.funding_asset.to_statemint_id())
							.ok_or(Error::<T>::PriceNotFound)?;
						let funding_asset_amount_needed = funding_asset_price
							.reciprocal()
							.ok_or(Error::<T>::BadMath)?
							.checked_mul_int(ticket_size)
							.ok_or(Error::<T>::BadMath)?;
						T::FundingCurrency::transfer(
							bid.funding_asset.to_statemint_id(),
							&project_account,
							&bid.bidder,
							bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed),
							Preservation::Preserve,
						)?;

						let usd_bond_needed = bid
							.multiplier
							.calculate_bonding_requirement::<T>(ticket_size)
							.map_err(|_| Error::<T>::BadMath)?;
						let plmc_bond_needed = plmc_price
							.reciprocal()
							.ok_or(Error::<T>::BadMath)?
							.checked_mul_int(usd_bond_needed)
							.ok_or(Error::<T>::BadMath)?;
						T::NativeCurrency::release(
							&LockType::Participation(project_id),
							&bid.bidder,
							bid.plmc_bond.saturating_sub(plmc_bond_needed),
							Precision::Exact,
						)?;

						bid.funding_asset_amount_locked = funding_asset_amount_needed;
						bid.plmc_bond = plmc_bond_needed;
					} else {
						return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::BadMath)
							.and(Ok(bid))
					}
				}

				Ok(bid)
			})
			.collect();
		let bids = bids?;
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
		let current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let is_first_bucket = current_bucket.current_price == project_metadata.minimum_price;

		let calc_weighted_price_fn = |bid: &BidInfoOf<T>, amount: BalanceOf<T>| -> Option<PriceOf<T>> {
			let ticket_size = bid.original_ct_usd_price.saturating_mul_int(amount);
			let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(ticket_size, bid_usd_value_sum);
			let weighted_price = bid.original_ct_usd_price.saturating_mul(bid_weight);
			Some(weighted_price)
		};
		let weighted_token_price = match is_first_bucket && !bids.is_empty() {
			true => project_metadata.minimum_price,
			false => bids
				.iter()
				.filter_map(|bid| match bid.status {
					BidStatus::Accepted => calc_weighted_price_fn(bid, bid.original_ct_amount),
					BidStatus::PartiallyAccepted(amount, _) => calc_weighted_price_fn(bid, amount),
					_ => None,
				})
				.reduce(|a, b| a.saturating_add(b))
				.ok_or(Error::<T>::NoBidsFound)?,
		};

		let mut final_total_funding_reached_by_bids = BalanceOf::<T>::zero();
		// Update the bid in the storage
		for mut bid in bids.into_iter() {
			if bid.final_ct_usd_price > weighted_token_price {
				bid.final_ct_usd_price = weighted_token_price;
				let new_ticket_size =
					weighted_token_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;

				let funding_asset_price = T::PriceProvider::get_price(bid.funding_asset.to_statemint_id())
					.ok_or(Error::<T>::PriceNotFound)?;
				let funding_asset_amount_needed = funding_asset_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(new_ticket_size)
					.ok_or(Error::<T>::BadMath)?;

				T::FundingCurrency::transfer(
					bid.funding_asset.to_statemint_id(),
					&project_account,
					&bid.bidder,
					bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed),
					Preservation::Preserve,
				)?;

				bid.funding_asset_amount_locked = funding_asset_amount_needed;

				let usd_bond_needed = bid
					.multiplier
					.calculate_bonding_requirement::<T>(new_ticket_size)
					.map_err(|_| Error::<T>::BadMath)?;
				let plmc_bond_needed = plmc_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(usd_bond_needed)
					.ok_or(Error::<T>::BadMath)?;

				T::NativeCurrency::release(
					&LockType::Participation(project_id),
					&bid.bidder,
					bid.plmc_bond.saturating_sub(plmc_bond_needed),
					Precision::Exact,
				)?;

				bid.plmc_bond = plmc_bond_needed;
			}
			let final_ticket_size =
				bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;
			final_total_funding_reached_by_bids.saturating_accrue(final_ticket_size);
			Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
		}

		// Update storage
		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> DispatchResult {
			if let Some(info) = maybe_info {
				info.weighted_average_price = Some(weighted_token_price);
				info.remaining_contribution_tokens.0.saturating_reduce(bid_token_amount_sum);
				info.funding_amount_reached.saturating_accrue(final_total_funding_reached_by_bids);
				Ok(())
			} else {
				Err(Error::<T>::ProjectNotFound.into())
			}
		})?;

		Ok(())
	}

    /// Refund a bid because of `reason`.
	fn refund_bid<'a>(
		bid: &'a mut BidInfoOf<T>,
		project_id: T::ProjectIdentifier,
		project_account: &'a AccountIdOf<T>,
		reason: RejectionReason,
	) -> Result<(), DispatchError> {
		bid.status = BidStatus::Rejected(reason);
		bid.final_ct_amount = Zero::zero();
		bid.final_ct_usd_price = Zero::zero();

		T::FundingCurrency::transfer(
			bid.funding_asset.to_statemint_id(),
			project_account,
			&bid.bidder,
			bid.funding_asset_amount_locked,
			Preservation::Preserve,
		)?;
		T::NativeCurrency::release(&LockType::Participation(project_id), &bid.bidder, bid.plmc_bond, Precision::Exact)?;
		bid.funding_asset_amount_locked = Zero::zero();
		bid.plmc_bond = Zero::zero();

		Ok(())
	}

    pub fn select_random_block(
		candle_starting_block: BlockNumberFor<T>,
		candle_ending_block: BlockNumberFor<T>,
	) -> BlockNumberFor<T> {
		let nonce = Self::get_and_increment_nonce();
		let (random_value, _known_since) = T::Randomness::random(&nonce);
		let random_block = <BlockNumberFor<T>>::decode(&mut random_value.as_ref())
			.expect("secure hashes should always be bigger than the block number; qed");
		let block_range = candle_ending_block - candle_starting_block;

		candle_starting_block + (random_block % block_range)
	}

	fn get_and_increment_nonce() -> Vec<u8> {
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce.wrapping_add(1));
		nonce.encode()
	}


    /// Calculate the total fees based on the funding reached.
	pub fn calculate_fees(funding_reached: BalanceOf<T>) -> Perquintill {
		let total_fee = Self::compute_total_fee_from_brackets(funding_reached);
		Perquintill::from_rational(total_fee, funding_reached)
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
	pub fn generate_evaluator_rewards_info(
		project_id: <T as Config>::ProjectIdentifier,
	) -> Result<RewardInfoOf<T>, DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let evaluations = Evaluations::<T>::iter_prefix((project_id,)).collect::<Vec<_>>();

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached;
		let fundraising_target = project_details.fundraising_target;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		let initial_token_allocation_size =
			project_metadata.total_allocation_size.0.saturating_add(project_metadata.total_allocation_size.1);
		let final_remaining_contribution_tokens = project_details
			.remaining_contribution_tokens
			.0
			.saturating_add(project_details.remaining_contribution_tokens.1);

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);

		// Calculate rewards.
		let evaluator_rewards = percentage_of_target_funding * Perquintill::from_percent(30) * total_fee_allocation;
		// Placeholder allocations (intended for future use).
		let _liquidity_pool = Perquintill::from_percent(50) * total_fee_allocation;
		let _long_term_holder_bonus = _liquidity_pool.saturating_sub(evaluator_rewards);

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

		Ok(reward_info)
	}


}



