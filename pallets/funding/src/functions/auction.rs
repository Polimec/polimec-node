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
    /// Called by user extrinsic
	/// Starts the auction round for a project. From the next block forward, any professional or
	/// institutional user can set bids for a token_amount/token_price pair.
	/// Any bids from this point until the candle_auction starts, will be considered as valid.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block is between the defined start and end blocks of the initialize period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the English Auction round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// english auction round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the [`bid`](Self::bid) extrinsic.
	/// Later on, `on_initialize` transitions the project into the candle auction round, by calling
	/// [`do_candle_auction`](Self::do_candle_auction).
	pub fn do_english_auction(caller: AccountIdOf<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_initialize_period_start_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.start()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;
		let auction_initialize_period_end_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.end()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;

		// * Validity checks *
		ensure!(
			caller == project_details.issuer || caller == T::PalletId::get().into_account_truncating(),
			Error::<T>::NotAllowed
		);
		ensure!(now >= auction_initialize_period_start_block, Error::<T>::TooEarlyForEnglishAuctionStart);
		ensure!(
			project_details.status == ProjectStatus::AuctionInitializePeriod,
			Error::<T>::ProjectNotInAuctionInitializePeriodRound
		);

		// * Calculate new variables *
		let english_start_block = now + 1u32.into();
		let english_end_block = now + T::EnglishAuctionDuration::get();

		// * Update Storage *
		project_details
			.phase_transition_points
			.english_auction
			.update(Some(english_start_block), Some(english_end_block));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::English);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// If this function was called inside the period, then it was called by the extrinsic and we need to
		// remove the scheduled automatic transition
		if now <= auction_initialize_period_end_block {
			Self::remove_from_update_store(&project_id)?;
		}
		// Schedule for automatic transition to candle auction round
		Self::add_to_update_store(english_end_block + 1u32.into(), (&project_id, UpdateType::CandleAuctionStart));

		// * Emit events *
		Self::deposit_event(Event::EnglishAuctionStarted { project_id, when: now });

		Ok(())
	}

    /// Called automatically by on_initialize
	/// Starts the candle round for a project.
	/// Any bids from this point until the candle round ends, are not guaranteed. Only bids
	/// made before the random ending block between the candle start and end will be considered
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block after the english auction end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Candle Auction round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// candle auction round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the `bid` extrinsic,
	/// but now their bids are not guaranteed.
	/// Later on, `on_initialize` ends the candle auction round and starts the community round,
	/// by calling [`do_community_funding`](Self::do_community_funding).
	pub fn do_candle_auction(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let english_end_block =
			project_details.phase_transition_points.english_auction.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > english_end_block, Error::<T>::TooEarlyForCandleAuctionStart);
		ensure!(
			project_details.status == ProjectStatus::AuctionRound(AuctionPhase::English),
			Error::<T>::ProjectNotInEnglishAuctionRound
		);

		// * Calculate new variables *
		let candle_start_block = now + 1u32.into();
		let candle_end_block = now + T::CandleAuctionDuration::get();

		// * Update Storage *
		project_details.phase_transition_points.candle_auction.update(Some(candle_start_block), Some(candle_end_block));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic check by on_initialize. Success depending on enough funding reached
		Self::add_to_update_store(candle_end_block + 1u32.into(), (&project_id, UpdateType::CommunityFundingStart));

		// * Emit events *
		Self::deposit_event(Event::CandleAuctionStarted { project_id, when: now });

		Ok(())
	}

    /// Bid for a project in the bidding stage.
	///
	/// # Arguments
	/// * `bidder` - The account that is bidding
	/// * `project_id` - The project to bid for
	/// * `amount` - The amount of tokens that the bidder wants to buy
	/// * `multiplier` - Used for calculating how much PLMC needs to be bonded to spend this much money (in USD)
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the bidder is not the project issuer
	/// * [`ProjectsDetails`] - Check that the project is in the bidding stage
	/// * [`BiddingBonds`] - Update the storage with the bidder's PLMC bond for that bid
	/// * [`Bids`] - Check previous bids by that user, and update the storage with the new bid
	pub fn do_bid(
		bidder: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		ct_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
	) -> DispatchResult {
		// * Get variables *
		ensure!(ct_amount > Zero::zero(), Error::<T>::BidTooLow);
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PriceNotFound)?;

		// * Validity checks *
		ensure!(bidder.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(matches!(project_details.status, ProjectStatus::AuctionRound(_)), Error::<T>::AuctionNotStarted);
		ensure!(funding_asset == project_metadata.participation_currencies, Error::<T>::FundingAssetNotAccepted);
		// Note: We limit the CT Amount to the total allocation size, to avoid long running loops.
		ensure!(ct_amount <= project_metadata.total_allocation_size.0, Error::<T>::NotAllowed);

		// Fetch current bucket details and other required info
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let mut amount_to_bid = ct_amount;

		// While there's a remaining amount to bid for
		while !amount_to_bid.is_zero() {
			let bid_amount = if amount_to_bid <= current_bucket.amount_left {
				// Simple case, the bucket has enough to cover the bid
				amount_to_bid
			} else {
				// The bucket doesn't have enough to cover the bid, so we bid the remaining amount of the current bucket
				current_bucket.amount_left
			};
			let bid_id = Self::next_bid_id();

			Self::perform_do_bid(
				bidder,
				project_id,
				bid_amount,
				current_bucket.current_price,
				multiplier,
				funding_asset,
				project_metadata.ticket_size,
				bid_id,
				now,
				plmc_usd_price,
			)?;
			// Update current bucket, and reduce the amount to bid by the amount we just bid
			current_bucket.update(bid_amount);
			amount_to_bid.saturating_reduce(bid_amount);
		}

		// Note: If the bucket has been exhausted, the 'update' function has already made the 'current_bucket' point to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(())
	}

	fn perform_do_bid(
		bidder: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		ct_amount: BalanceOf<T>,
		ct_usd_price: T::Price,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		project_ticket_size: TicketSize<BalanceOf<T>>,
		bid_id: u32,
		now: BlockNumberFor<T>,
		plmc_usd_price: T::Price,
	) -> Result<BidInfoOf<T>, DispatchError> {
		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(funding_asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;
		let existing_bids = Bids::<T>::iter_prefix_values((project_id, bidder)).collect::<Vec<_>>();

		if let Some(minimum_ticket_size) = project_ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(ticket_size >= minimum_ticket_size, Error::<T>::BidTooLow);
		};
		if let Some(maximum_ticket_size) = project_ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(ticket_size <= maximum_ticket_size, Error::<T>::BidTooLow);
		};
		// * Calculate new variables *
		let plmc_bond =
			Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price).map_err(|_| Error::<T>::BadMath)?;

		let funding_asset_amount_locked =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_statemint_id();

		let new_bid = BidInfoOf::<T> {
			id: bid_id,
			project_id,
			bidder: bidder.clone(),
			status: BidStatus::YetUnknown,
			original_ct_amount: ct_amount,
			original_ct_usd_price: ct_usd_price,
			final_ct_amount: ct_amount,
			final_ct_usd_price: ct_usd_price,
			funding_asset,
			funding_asset_amount_locked,
			multiplier,
			plmc_bond,
			plmc_vesting_info: None,
			when: now,
			funds_released: false,
			ct_minted: false,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		// * Update storage *
		if existing_bids.len() >= T::MaxBidsPerUser::get() as usize {
			let lowest_bid = existing_bids.iter().min_by_key(|bid| &bid.id).ok_or(Error::<T>::ImpossibleState)?;

			// TODO: Check how to handle this
			// ensure!(new_bid.plmc_bond > lowest_bid.plmc_bond, Error::<T>::BidTooLow);

			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&lowest_bid.bidder,
				lowest_bid.plmc_bond,
				Precision::Exact,
			)?;
			T::FundingCurrency::transfer(
				asset_id,
				&Self::fund_account_id(project_id),
				&lowest_bid.bidder,
				lowest_bid.funding_asset_amount_locked,
				Preservation::Expendable,
			)?;
			Bids::<T>::remove((project_id, &lowest_bid.bidder, lowest_bid.id));
		}

		Self::try_plmc_participation_lock(bidder, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(bidder, project_id, funding_asset_amount_locked, asset_id)?;

		Bids::<T>::insert((project_id, bidder, bid_id), &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));

		Self::deposit_event(Event::Bid { project_id, amount: ct_amount, price: ct_usd_price, multiplier });

		Ok(new_bid)
	}

}