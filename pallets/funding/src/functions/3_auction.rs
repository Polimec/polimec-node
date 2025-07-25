use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_bid(params: DoBidParams<T>) -> DispatchResultWithPostInfo {
		let DoBidParams {
			bidder,
			project_id,
			funding_asset_amount, // Total funding asset provided by the bidder
			mode,
			funding_asset,
			investor_type,
			did,
			whitelisted_policy,
			receiving_account,
		} = params;

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let now = BlockProviderFor::<T>::current_block_number();

		let funding_asset_price =
			PriceProviderOf::<T>::get_decimals_aware_price(&funding_asset.id(), funding_asset.decimals())
				.ok_or(Error::<T>::BadMath)?;

		let price_ratio = funding_asset_price.checked_div(&current_bucket.current_price).ok_or(Error::<T>::BadMath)?;

		let raw_ct_amount = price_ratio.checked_mul_int(funding_asset_amount).ok_or(Error::<T>::BadMath)?;

		let rounding_step = 10u128
			.checked_pow(project_metadata.token_information.decimals as u32 - USD_DECIMALS as u32)
			.ok_or(Error::<T>::BadMath)?;

		// Now, round the raw amount to the nearest UNIT number.
		let total_ct_amount =
			polimec_common::round_to_nearest(raw_ct_amount, rounding_step).ok_or(Error::<T>::BadMath)?;

		// println!("Rounded CT Amount: {:?}", total_ct_amount);

		let min_total_ticket_size_usd =
			current_bucket.current_price.checked_mul_int(total_ct_amount).ok_or(Error::<T>::BadMath)?;

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

		let project_policy_cid = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;
		ensure!(project_policy_cid == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(total_ct_amount > Zero::zero(), Error::<T>::TooLow);
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
			metadata_ticket_size_bounds.usd_ticket_above_minimum_per_participation(min_total_ticket_size_usd),
			Error::<T>::TooLow
		);
		ensure!(mode.multiplier() <= max_multiplier && mode.multiplier() > 0u8, Error::<T>::ForbiddenMultiplier);
		ensure!(
			project_metadata.participants_account_type.junction_is_supported(&receiving_account),
			Error::<T>::UnsupportedReceiverAccountJunction
		);

		let mut perform_bid_calls = 0u8;
		let mut remaining_funding_asset_for_bid = funding_asset_amount;
		let mut amount_to_bid_total = total_ct_amount;

		while !amount_to_bid_total.is_zero() {
			perform_bid_calls.saturating_accrue(1);

			let ct_for_this_bucket = amount_to_bid_total.min(current_bucket.amount_left);
			let bid_id = NextBidId::<T>::get();
			let auction_oversubscribed = current_bucket.current_price > current_bucket.initial_price;

			// Calculate the funding asset amount required for ct_for_this_bucket at the current bucket's price.
			let funding_asset_needed_for_chunk_at_current_price =
				if amount_to_bid_total == ct_for_this_bucket && remaining_funding_asset_for_bid > Zero::zero() {
					remaining_funding_asset_for_bid
				} else {
					// Otherwise, calculate proportionally based on current bucket's price.
					let usd_cost_for_this_bucket =
						current_bucket.current_price.checked_mul_int(ct_for_this_bucket).ok_or(Error::<T>::BadMath)?;
					funding_asset_price
						.reciprocal()
						.ok_or(Error::<T>::BadMath)?
						.checked_mul_int(usd_cost_for_this_bucket)
						.ok_or(Error::<T>::BadMath)?
				};

			let funding_asset_for_this_bucket =
				funding_asset_needed_for_chunk_at_current_price.min(remaining_funding_asset_for_bid);

			// If the calculated funding for this bucket chunk is zero:
			if funding_asset_for_this_bucket.is_zero() {
				if amount_to_bid_total > ct_for_this_bucket {
					break;
				}
			}

			let perform_params = DoPerformBidParams {
				bidder: bidder.clone(),
				project_id,
				ct_amount: ct_for_this_bucket,
				ct_usd_price: current_bucket.current_price,
				mode,
				funding_asset,
				funding_asset_amount: funding_asset_for_this_bucket,
				bid_id,
				now,
				did: did.clone(),
				metadata_ticket_size_bounds,
				receiving_account,
				auction_oversubscribed,
			};

			BidsBucketBounds::<T>::mutate(project_id, current_bucket.current_price, |maybe_bounds| {
				if let Some(bucket_bounds) = maybe_bounds {
					bucket_bounds.last_bid_index = bid_id;
				} else {
					*maybe_bounds = Some(BidBucketBounds { first_bid_index: bid_id, last_bid_index: bid_id });
				}
			});

			Self::do_perform_bid(perform_params)?;

			remaining_funding_asset_for_bid =
				remaining_funding_asset_for_bid.saturating_sub(funding_asset_for_this_bucket);
			amount_to_bid_total = amount_to_bid_total.saturating_sub(ct_for_this_bucket);

			current_bucket.update(ct_for_this_bucket);
		}
		// If the bucket was exhausted, current_bucket.update() already advanced it to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(PostDispatchInfo {
			actual_weight: Some(<T as Config>::WeightInfo::bid(perform_bid_calls as u32)),
			pays_fee: Pays::No,
		})
	}

	/// Inner function to perform bids within a bucket. do_bid makes sure to split the bid into buckets and call this as
	/// many times as necessary
	#[transactional]
	fn do_perform_bid(do_perform_bid_params: DoPerformBidParams<T>) -> Result<BidInfoOf<T>, DispatchError> {
		let DoPerformBidParams {
			bidder,
			project_id,
			ct_amount,
			ct_usd_price,
			mode,
			funding_asset,
			funding_asset_amount,
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

		let plmc_bond = Self::calculate_plmc_bond(usd_ticket_size, multiplier).map_err(|_| Error::<T>::BadMath)?;
		let funding_asset_amount_locked = funding_asset_amount;

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

		Self::bond_plmc_with_mode(&bidder, project_id, new_bid.plmc_bond, mode, funding_asset)?;
		Self::try_funding_asset_hold(&bidder, project_id, funding_asset_amount_locked, funding_asset.id())?;
		Bids::<T>::insert(project_id, bid_id, &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));
		AuctionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount = amount.saturating_add(usd_ticket_size));

		if auction_oversubscribed {
			CTAmountOversubscribed::<T>::mutate(project_id, |amount| *amount = amount.saturating_add(ct_amount));
		}

		Self::deposit_event(Event::Bid {
			project_id,
			bidder,
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

	/// Process a bid that was outbid by a new bid. This will set it to Rejected so the user can get their funds back with `settle_bid` and bid again.
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
					let (new_price, new_index) =
						Self::get_next_cutoff(project_id, bucket.delta_price, bid_price, bid_index)?;
					let new_cutoff = OutbidBidsCutoff { bid_price: new_price, bid_index: new_index };
					OutbidBidsCutoffs::<T>::set(project_id, Some(new_cutoff));
					new_cutoff
				}
			},
			None => {
				let first_price = project_metadata.minimum_price;
				let first_bounds =
					BidsBucketBounds::<T>::get(project_id, first_price).ok_or(Error::<T>::ImpossibleState)?;
				let initial_cutoff =
					OutbidBidsCutoff { bid_price: first_price, bid_index: first_bounds.last_bid_index };
				OutbidBidsCutoffs::<T>::set(project_id, Some(initial_cutoff));
				initial_cutoff
			},
		};

		// Process the bid at the cutoff
		let mut bid = Bids::<T>::get(project_id, current_cutoff.bid_index).ok_or(Error::<T>::ImpossibleState)?;

		let ct_amount = match bid.status {
			BidStatus::PartiallyAccepted(ct_amount) => ct_amount,
			_ => bid.original_ct_amount,
		};

		// Update bid status and oversubscribed amount
		// TODO: Sync this with the buffer amount in the bucket.update() method
		if ct_amount - 1 > ct_amount_oversubscribed {
			bid.status = BidStatus::PartiallyAccepted(ct_amount.saturating_sub(ct_amount_oversubscribed));
			ct_amount_oversubscribed = Zero::zero();
		} else {
			bid.status = BidStatus::Rejected;
			ct_amount_oversubscribed = ct_amount_oversubscribed.saturating_sub(ct_amount);
		}

		// Save state changes
		Bids::<T>::insert(project_id, bid.id, bid.clone());
		OutbidBidsCutoffs::<T>::set(project_id, Some(current_cutoff));
		CTAmountOversubscribed::<T>::insert(project_id, ct_amount_oversubscribed);

		Self::deposit_event(Event::OversubscribedBidProcessed { project_id, bid_id: bid.id });

		Ok(())
	}

	/// Get the next bid that should be processed by do_process_next_oversubscribed_bid
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
