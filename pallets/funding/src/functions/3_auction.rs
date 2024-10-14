#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_end_auction(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;

		// * Calculate WAP *
		let auction_allocation_size =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let weighted_token_price = bucket.calculate_wap(auction_allocation_size);

		// * Update Storage *
		let calculation_result = Self::decide_winning_bids(
			project_id,
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			weighted_token_price,
		);
		let updated_project_details =
			ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		match calculation_result {
			Err(e) => return Err(DispatchErrorWithPostInfo { post_info: ().into(), error: e }),
			Ok((accepted_bids_count, rejected_bids_count)) => {
				let now = <frame_system::Pallet<T>>::block_number();
				// * Transition Round *
				Self::transition_project(
					project_id,
					updated_project_details,
					ProjectStatus::AuctionRound,
					ProjectStatus::CommunityRound(now.saturating_add(T::CommunityRoundDuration::get())),
					Some(T::CommunityRoundDuration::get() + T::RemainderRoundDuration::get()),
					false,
				)?;
				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::end_auction(accepted_bids_count, rejected_bids_count)),
					pays_fee: Pays::Yes,
				})
			},
		}
	}

	#[transactional]
	pub fn do_bid(params: DoBidParams<T>) -> DispatchResultWithPostInfo {
		// * Get variables *
		let DoBidParams { bidder, project_id, ct_amount, mode, funding_asset, investor_type, did, whitelisted_policy } =
			params;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// Fetch current bucket details and other required info
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let mut amount_to_bid = ct_amount;
		let total_bids_for_project = BidCounts::<T>::get(project_id);
		let project_policy = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;

		// User will spend at least this amount of USD for his bid(s). More if the bid gets split into different buckets
		let min_total_ticket_size =
			current_bucket.current_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		// weight return variables
		let mut perform_bid_calls = 0;

		let existing_bids = Bids::<T>::iter_prefix_values((project_id, bidder.clone())).collect::<Vec<_>>();
		let existing_bids_amount = existing_bids.len() as u32;

		let metadata_ticket_size_bounds = match investor_type {
			InvestorType::Institutional => project_metadata.bidding_ticket_sizes.institutional,
			InvestorType::Professional => project_metadata.bidding_ticket_sizes.professional,
			_ => return Err(Error::<T>::WrongInvestorType.into()),
		};
		let max_multiplier = match investor_type {
			InvestorType::Professional => PROFESSIONAL_MAX_MULTIPLIER,
			InvestorType::Institutional => INSTITUTIONAL_MAX_MULTIPLIER,
			// unreachable
			_ => return Err(Error::<T>::ImpossibleState.into()),
		};

		// * Validity checks *
		ensure!(project_policy == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(
			matches!(investor_type, InvestorType::Institutional | InvestorType::Professional),
			DispatchError::from("Retail investors are not allowed to bid")
		);

		ensure!(ct_amount > Zero::zero(), Error::<T>::TooLow);
		ensure!(did != project_details.issuer_did, Error::<T>::ParticipationToOwnProject);
		ensure!(matches!(project_details.status, ProjectStatus::AuctionRound), Error::<T>::IncorrectRound);
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
		ensure!(
			ct_amount <= project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			Error::<T>::TooHigh
		);
		ensure!(existing_bids.len() < T::MaxBidsPerUser::get() as usize, Error::<T>::TooManyUserParticipations);

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
				total_bids_by_bidder: existing_bids_amount.saturating_add(perform_bid_calls),
				total_bids_for_project: total_bids_for_project.saturating_add(perform_bid_calls),
			};
			Self::do_perform_bid(perform_params)?;

			perform_bid_calls = perform_bid_calls.saturating_add(1);

			// Update the current bucket and reduce the amount to bid by the amount we just bid
			current_bucket.update(ct_amount);
			amount_to_bid.saturating_reduce(ct_amount);
		}

		// Note: If the bucket has been exhausted, the 'update' function has already made the 'current_bucket' point to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::bid(existing_bids_amount, perform_bid_calls)),
			pays_fee: Pays::Yes,
		})
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
			total_bids_by_bidder,
			total_bids_for_project,
		} = do_perform_bid_params;

		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let total_usd_bid_by_did = AuctionBoughtUSD::<T>::get((project_id, did.clone()));
		let multiplier: MultiplierOf<T> = mode.multiplier().try_into().map_err(|_| Error::<T>::BadMath)?;

		ensure!(
			metadata_ticket_size_bounds
				.usd_ticket_below_maximum_per_did(total_usd_bid_by_did.saturating_add(ticket_size)),
			Error::<T>::TooHigh
		);
		ensure!(total_bids_by_bidder < T::MaxBidsPerUser::get(), Error::<T>::TooManyUserParticipations);
		ensure!(total_bids_for_project < T::MaxBidsPerProject::get(), Error::<T>::TooManyProjectParticipations);

		// * Calculate new variables *
		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier).map_err(|_| Error::<T>::BadMath)?;
		let funding_asset_amount_locked = Self::calculate_funding_asset_amount(ticket_size, funding_asset)?;

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
		};

		Self::bond_plmc_with_mode(&bidder, project_id, plmc_bond, mode, funding_asset)?;
		Self::try_funding_asset_hold(&bidder, project_id, funding_asset_amount_locked, funding_asset.id())?;

		Bids::<T>::insert((project_id, bidder.clone(), bid_id), &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));
		BidCounts::<T>::mutate(project_id, |c| *c = c.saturating_add(1));
		AuctionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount = amount.saturating_add(ticket_size));

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
}
