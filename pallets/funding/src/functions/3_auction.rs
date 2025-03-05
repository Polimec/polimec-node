#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_bid(params: DoBidParams<T>) -> DispatchResult {
		// * Get variables *
		let DoBidParams {
			bidder,
			project_id,
			ct_amount,
			mode,
			funding_asset,
			investor_type,
			did,
			whitelisted_policy,
			receiving_account,
		} = params;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// Fetch current bucket details and other required info
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let mut amount_to_bid = ct_amount;
		let project_policy = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;

		// User will spend at least this amount of USD for his bid(s). More if the bid gets split into different buckets
		let min_total_ticket_size =
			current_bucket.current_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		// weight return variables
		let mut perform_bid_calls = 0;

		let metadata_ticket_size_bounds = match investor_type {
			InvestorType::Institutional => project_metadata.bidding_ticket_sizes.institutional,
			InvestorType::Professional => project_metadata.bidding_ticket_sizes.professional,
			InvestorType::Retail => project_metadata.bidding_ticket_sizes.retail,
		};
		let max_multiplier = match investor_type {
			InvestorType::Professional => PROFESSIONAL_MAX_MULTIPLIER,
			InvestorType::Institutional => INSTITUTIONAL_MAX_MULTIPLIER,
			InvestorType::Retail => RETAIL_MAX_MULTIPLIER,
		};

		// * Validity checks *
		ensure!(project_policy == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(ct_amount > Zero::zero(), Error::<T>::TooLow);
		ensure!(did != project_details.issuer_did, Error::<T>::ParticipationToOwnProject);
		ensure!(matches!(project_details.status, ProjectStatus::AuctionRound), Error::<T>::IncorrectRound);
		ensure!(
			project_details.round_duration.started(now) && !project_details.round_duration.ended(now),
			Error::<T>::IncorrectRound
		);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);

		ensure!(
			metadata_ticket_size_bounds.usd_ticket_above_minimum_per_participation(min_total_ticket_size),
			Error::<T>::TooLow
		);
		ensure!(mode.multiplier() <= max_multiplier && mode.multiplier() > 0u8, Error::<T>::ForbiddenMultiplier);

		// Note: We limit the CT Amount to the auction allocation size, to avoid long-running loops.
		ensure!(ct_amount <= project_metadata.total_allocation_size, Error::<T>::TooHigh);
		ensure!(
			project_metadata.participants_account_type.junction_is_supported(&receiving_account),
			Error::<T>::UnsupportedReceiverAccountJunction
		);

		// While there's a remaining amount to bid for
		while !amount_to_bid.is_zero() {
			let ct_amount = if amount_to_bid <= current_bucket.amount_left {
				// Simple case, the bucket has enough to cover the bid
				amount_to_bid
			} else {
				// The bucket doesn't have enough to cover the bid, so we bid the remaining amount of the current bucket
				current_bucket.amount_left
			};
			let bid_id = NextBidId::<T>::get();
			let auction_oversubscribed = current_bucket.current_price > current_bucket.initial_price;

			let perform_params = DoPerformBidParams {
				bidder: bidder.clone(),
				project_id,
				ct_amount,
				ct_usd_price: current_bucket.current_price,
				mode,
				funding_asset,
				bid_id,
				now,
				did: did.clone(),
				metadata_ticket_size_bounds,
				receiving_account,
				auction_oversubscribed,
			};

			BidsBucketBounds::<T>::mutate(project_id, current_bucket.current_price, |maybe_indexes| {
				if let Some(bucket_bounds) = maybe_indexes {
					bucket_bounds.last_bid_index = bid_id;
				} else {
					*maybe_indexes = Some(BidBucketBounds { first_bid_index: bid_id, last_bid_index: bid_id });
				}
			});

			Self::do_perform_bid(perform_params)?;

			perform_bid_calls = perform_bid_calls.saturating_add(1);

			// Update the current bucket and reduce the amount to bid by the amount we just bid
			current_bucket.update(ct_amount);
			amount_to_bid.saturating_reduce(ct_amount);
		}

		// Note: If the bucket has been exhausted, the 'update' function has already made the 'current_bucket' point to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(())
	}

	#[transactional]
	fn do_perform_bid(do_perform_bid_params: DoPerformBidParams<T>) -> Result<BidInfoOf<T>, DispatchError> {
		let DoPerformBidParams {
			bidder,
			project_id,
			ct_amount,
			ct_usd_price,
			mode,
			funding_asset,
			bid_id,
			now,
			did,
			metadata_ticket_size_bounds,
			receiving_account,
			auction_oversubscribed,
		} = do_perform_bid_params;

		let usd_ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let total_usd_bid_by_did = AuctionBoughtUSD::<T>::get((project_id, did.clone()));
		let multiplier: MultiplierOf<T> = mode.multiplier().try_into().map_err(|_| Error::<T>::BadMath)?;

		ensure!(
			metadata_ticket_size_bounds
				.usd_ticket_below_maximum_per_did(total_usd_bid_by_did.saturating_add(usd_ticket_size)),
			Error::<T>::TooHigh
		);

		// * Calculate new variables *
		let plmc_bond = Self::calculate_plmc_bond(usd_ticket_size, multiplier).map_err(|_| Error::<T>::BadMath)?;
		let funding_asset_amount_locked = Self::calculate_funding_asset_amount(usd_ticket_size, funding_asset)?;

		let new_bid = BidInfoOf::<T> {
			id: bid_id,
			project_id,
			bidder: bidder.clone(),
			did: did.clone(),
			status: BidStatus::YetUnknown,
			original_ct_amount: ct_amount,
			original_ct_usd_price: ct_usd_price,
			funding_asset,
			funding_asset_amount_locked,
			mode,
			plmc_bond,
			when: now,
			receiving_account,
		};

		Self::bond_plmc_with_mode(&bidder, project_id, plmc_bond, mode, funding_asset)?;
		Self::try_funding_asset_hold(&bidder, project_id, funding_asset_amount_locked, funding_asset.id())?;

		Bids::<T>::insert(project_id, bid_id, &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));
		AuctionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount = amount.saturating_add(usd_ticket_size));

		if auction_oversubscribed {
			CTAmountOversubscribed::<T>::mutate(project_id, |amount| *amount = amount.saturating_add(ct_amount));
		}

		Self::deposit_event(Event::Bid {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			ct_amount,
			ct_price: ct_usd_price,
			funding_asset,
			funding_amount: funding_asset_amount_locked,
			plmc_bond,
			mode,
		});

		Ok(new_bid)
	}

	pub fn do_process_next_oversubscribed_bid(project_id: ProjectId) -> DispatchResult {
		// Load and validate initial state
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let mut ct_amount_oversubscribed = CTAmountOversubscribed::<T>::get(project_id);
		ensure!(ct_amount_oversubscribed > Zero::zero(), Error::<T>::NoBidsOversubscribed);

		// Determine the current cutoff
		let current_cutoff = match OutbidBidsCutoffs::<T>::get(project_id) {
			Some(cutoff @ OutbidBidsCutoff { bid_price, bid_index }) => {
				let bid = Bids::<T>::get(project_id, bid_index).ok_or(Error::<T>::ImpossibleState)?;
				if matches!(bid.status, BidStatus::PartiallyAccepted(_)) {
					cutoff
				} else {
					let (new_price, new_index) = Self::get_next_cutoff(project_id, bucket.delta_price, bid_price, bid_index)?;
					OutbidBidsCutoff { bid_price: new_price, bid_index: new_index }
				}
			},
			None => {
				let first_price = project_metadata.minimum_price;
				let first_bounds = BidsBucketBounds::<T>::get(project_id, first_price)
					.ok_or(Error::<T>::ImpossibleState)?;
				OutbidBidsCutoff { bid_price: first_price, bid_index: first_bounds.last_bid_index }
			}
		};

		// Process the bid at the cutoff
		let mut bid = Bids::<T>::get(project_id, current_cutoff.bid_index)
			.ok_or(Error::<T>::ImpossibleState)?;

		let bid_amount = match bid.status {
			BidStatus::PartiallyAccepted(amount) => amount,
			_ => bid.original_ct_amount,
		};

		// Update bid status and oversubscribed amount
		if bid_amount > ct_amount_oversubscribed {
			bid.status = BidStatus::PartiallyAccepted(bid_amount.saturating_sub(ct_amount_oversubscribed));
			ct_amount_oversubscribed = Zero::zero();
		} else {
			bid.status = BidStatus::Rejected;
			ct_amount_oversubscribed = ct_amount_oversubscribed.saturating_sub(bid_amount);
		}

		// Save state changes
		Bids::<T>::insert(project_id, bid.id, bid);
		OutbidBidsCutoffs::<T>::set(project_id, Some(current_cutoff));
		CTAmountOversubscribed::<T>::insert(project_id, ct_amount_oversubscribed);

		Ok(())
	}

	pub fn get_next_cutoff(
		project_id: ProjectId,
		delta_price: PriceOf<T>,
		current_price: PriceOf<T>,
		current_index: u32,
	) -> Result<(PriceOf<T>, u32), DispatchError> {
		let bounds = BidsBucketBounds::<T>::get(project_id, current_price).ok_or(Error::<T>::ImpossibleState)?;
		if current_index == bounds.first_bid_index {
			let new_price = current_price.saturating_add(delta_price);
			let new_bounds = BidsBucketBounds::<T>::get(project_id, new_price).ok_or(Error::<T>::ImpossibleState)?;
			Ok((new_price, new_bounds.last_bid_index))
		} else {
			Ok((current_price, current_index.saturating_sub(1)))
		}
	}
}
