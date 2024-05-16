use super::*;

impl<T: Config> Pallet<T> {
	/// Called by user extrinsic
	/// Starts the auction round for a project. From the next block forward, any professional or
	/// institutional user can set bids for a token_amount/token_price pair.
	/// Any bids from this point until the auction_closing starts, will be considered as valid.
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
	/// The validity checks pass, and the project is transitioned to the Auction Opening round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// auction opening round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the [`bid`](Self::bid) extrinsic.
	/// Later on, `on_initialize` transitions the project into the closing auction round, by calling
	/// [`do_auction_closing`](Self::do_auction_closing).
	#[transactional]
	pub fn do_start_auction_opening(caller: AccountIdOf<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		let auction_initialize_period_start_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.start()
			.ok_or(Error::<T>::TransitionPointNotSet)?;

		// * Validity checks *
		ensure!(
			caller == T::PalletId::get().into_account_truncating() || caller == project_details.issuer_account,
			Error::<T>::NotIssuer
		);

		ensure!(now >= auction_initialize_period_start_block, Error::<T>::TooEarlyForRound);
		// If the auction is first manually started, the automatic transition fails here. This
		// behaviour is intended, as it gracefully skips the automatic transition if the
		// auction was started manually.
		ensure!(project_details.status == ProjectStatus::AuctionInitializePeriod, Error::<T>::IncorrectRound);

		// * Calculate new variables *
		let opening_start_block = now;
		let opening_end_block = now.saturating_add(T::AuctionOpeningDuration::get()).saturating_sub(One::one());

		// * Update Storage *
		project_details
			.phase_transition_points
			.auction_opening
			.update(Some(opening_start_block), Some(opening_end_block));
		project_details.status = ProjectStatus::AuctionOpening;
		ProjectsDetails::<T>::insert(project_id, project_details);

		let insertion_attempts;
		// Schedule for automatic transition to auction closing round
		match Self::add_to_update_store(opening_end_block + 1u32.into(), (&project_id, UpdateType::AuctionClosingStart))
		{
			Ok(iterations) => {
				insertion_attempts = iterations;
			},
			Err(insertion_attempts) =>
				return Err(DispatchErrorWithPostInfo {
					post_info: PostDispatchInfo {
						actual_weight: Some(WeightInfoOf::<T>::start_auction_manually(insertion_attempts)),
						pays_fee: Pays::Yes,
					},
					error: Error::<T>::TooManyInsertionAttempts.into(),
				}),
		};

		// * Emit events *
		Self::deposit_event(Event::ProjectPhaseTransition { project_id, phase: ProjectPhases::AuctionOpening });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_auction_manually(insertion_attempts)),
			pays_fee: Pays::Yes,
		})
	}

	/// Called automatically by on_initialize
	/// Starts the auction closing round for a project.
	/// Any bids from this point until the auction closing round ends, are not guaranteed. Only bids
	/// made before the random ending block between the auction closing start and end will be considered
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block after the opening auction end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the auction closing round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// auction closing round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the `bid` extrinsic,
	/// but now their bids are not guaranteed.
	/// Later on, `on_initialize` ends the auction closing round and starts the community round,
	/// by calling [`do_community_funding`](Self::do_start_community_funding).
	#[transactional]
	pub fn do_start_auction_closing(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let opening_end_block =
			project_details.phase_transition_points.auction_opening.end().ok_or(Error::<T>::TransitionPointNotSet)?;

		// * Validity checks *
		ensure!(now > opening_end_block, Error::<T>::TooEarlyForRound);
		ensure!(project_details.status == ProjectStatus::AuctionOpening, Error::<T>::IncorrectRound);

		// * Calculate new variables *
		let closing_start_block = now;
		let closing_end_block = now.saturating_add(T::AuctionClosingDuration::get()).saturating_sub(One::one());

		// * Update Storage *
		project_details
			.phase_transition_points
			.auction_closing
			.update(Some(closing_start_block), Some(closing_end_block));
		project_details.status = ProjectStatus::AuctionClosing;
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic check by on_initialize. Success depending on enough funding reached
		let insertion_iterations = match Self::add_to_update_store(
			closing_end_block + 1u32.into(),
			(&project_id, UpdateType::AuctionClosingEnd),
		) {
			Ok(iterations) => iterations,
			Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
		};

		// * Emit events *
		Self::deposit_event(Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::AuctionClosing });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_auction_closing_phase(insertion_iterations)),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	pub fn do_end_auction_closing(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_closing_start_block =
			project_details.phase_transition_points.auction_closing.start().ok_or(Error::<T>::TransitionPointNotSet)?;
		let auction_closing_end_block =
			project_details.phase_transition_points.auction_closing.end().ok_or(Error::<T>::TransitionPointNotSet)?;

		// * Validity checks *
		ensure!(now > auction_closing_end_block, Error::<T>::TooEarlyForRound);
		ensure!(project_details.status == ProjectStatus::AuctionClosing, Error::<T>::IncorrectRound);

		// * Calculate new variables *
		let end_block = Self::select_random_block(auction_closing_start_block, auction_closing_end_block);

		// * Update Storage *
		let calculation_result = Self::decide_winning_bids(
			project_id,
			end_block,
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
		);

		match calculation_result {
			Err(e) => return Err(DispatchErrorWithPostInfo { post_info: ().into(), error: e }),
			Ok((accepted_bids_count, rejected_bids_count)) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_closing_ending = Some(end_block);
				project_details.status = ProjectStatus::CalculatingWAP;
				ProjectsDetails::<T>::insert(project_id, project_details);

				let insertion_iterations = match Self::add_to_update_store(
					now + 1u32.into(),
					(&project_id, UpdateType::CommunityFundingStart),
				) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

				// * Emit events *
				Self::deposit_event(Event::<T>::ProjectPhaseTransition {
					project_id,
					phase: ProjectPhases::CalculatingWAP,
				});

				Ok(PostDispatchInfo {
					// TODO: make new benchmark
					actual_weight: Some(WeightInfoOf::<T>::start_community_funding(
						insertion_iterations,
						accepted_bids_count,
						rejected_bids_count,
					)),
					pays_fee: Pays::Yes,
				})
			},
		}
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
	/// * [`ProjectsDetails`] - Check that the project is in the bidding stage
	/// * [`BiddingBonds`] - Update the storage with the bidder's PLMC bond for that bid
	/// * [`Bids`] - Check previous bids by that user, and update the storage with the new bid
	#[transactional]
	pub fn do_bid(
		bidder: &AccountIdOf<T>,
		project_id: ProjectId,
		ct_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
		whitelisted_policy: Cid,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let plmc_usd_price = T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.ok_or(Error::<T>::PriceNotFound)?;

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

		let existing_bids = Bids::<T>::iter_prefix_values((project_id, bidder)).collect::<Vec<_>>();
		let existing_bids_amount = existing_bids.len() as u32;

		let metadata_bidder_ticket_size_bounds = match investor_type {
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
		ensure!(
			matches!(project_details.status, ProjectStatus::AuctionOpening | ProjectStatus::AuctionClosing),
			Error::<T>::IncorrectRound
		);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);

		ensure!(
			metadata_bidder_ticket_size_bounds.usd_ticket_above_minimum_per_participation(min_total_ticket_size),
			Error::<T>::TooLow
		);
		ensure!(multiplier.into() <= max_multiplier && multiplier.into() > 0u8, Error::<T>::ForbiddenMultiplier);

		// Note: We limit the CT Amount to the auction allocation size, to avoid long running loops.
		ensure!(
			ct_amount <= project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			Error::<T>::TooHigh
		);
		ensure!(existing_bids.len() < T::MaxBidsPerUser::get() as usize, Error::<T>::TooManyUserParticipations);

		// While there's a remaining amount to bid for
		while !amount_to_bid.is_zero() {
			let bid_amount = if amount_to_bid <= current_bucket.amount_left {
				// Simple case, the bucket has enough to cover the bid
				amount_to_bid
			} else {
				// The bucket doesn't have enough to cover the bid, so we bid the remaining amount of the current bucket
				current_bucket.amount_left
			};
			let bid_id = NextBidId::<T>::get();

			Self::perform_do_bid(
				bidder,
				project_id,
				bid_amount,
				current_bucket.current_price,
				multiplier,
				funding_asset,
				bid_id,
				now,
				plmc_usd_price,
				did.clone(),
				metadata_bidder_ticket_size_bounds,
				existing_bids_amount.saturating_add(perform_bid_calls),
				total_bids_for_project.saturating_add(perform_bid_calls),
			)?;

			perform_bid_calls += 1;

			// Update the current bucket and reduce the amount to bid by the amount we just bid
			current_bucket.update(bid_amount);
			amount_to_bid.saturating_reduce(bid_amount);
		}

		// Note: If the bucket has been exhausted, the 'update' function has already made the 'current_bucket' point to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::bid(existing_bids_amount, perform_bid_calls)),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	fn perform_do_bid(
		bidder: &AccountIdOf<T>,
		project_id: ProjectId,
		ct_amount: BalanceOf<T>,
		ct_usd_price: T::Price,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		bid_id: u32,
		now: BlockNumberFor<T>,
		plmc_usd_price: T::Price,
		did: Did,
		metadata_ticket_size_bounds: TicketSizeOf<T>,
		total_bids_by_bidder: u32,
		total_bids_for_project: u32,
	) -> Result<BidInfoOf<T>, DispatchError> {
		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let total_usd_bid_by_did = AuctionBoughtUSD::<T>::get((project_id, did.clone()));

		ensure!(
			metadata_ticket_size_bounds
				.usd_ticket_below_maximum_per_did(total_usd_bid_by_did.saturating_add(ticket_size)),
			Error::<T>::TooHigh
		);
		ensure!(total_bids_by_bidder < T::MaxBidsPerUser::get(), Error::<T>::TooManyUserParticipations);
		ensure!(total_bids_for_project < T::MaxBidsPerProject::get(), Error::<T>::TooManyProjectParticipations);

		let funding_asset_id = funding_asset.to_assethub_id();
		let funding_asset_decimals = T::FundingCurrency::decimals(funding_asset_id);
		let funding_asset_usd_price =
			T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
				.ok_or(Error::<T>::PriceNotFound)?;

		// * Calculate new variables *
		let plmc_bond =
			Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price).map_err(|_| Error::<T>::BadMath)?;

		let funding_asset_amount_locked =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_assethub_id();

		let new_bid = BidInfoOf::<T> {
			id: bid_id,
			project_id,
			bidder: bidder.clone(),
			did: did.clone(),
			status: BidStatus::YetUnknown,
			original_ct_amount: ct_amount,
			original_ct_usd_price: ct_usd_price,
			final_ct_amount: ct_amount,
			final_ct_usd_price: ct_usd_price,
			funding_asset,
			funding_asset_amount_locked,
			multiplier,
			plmc_bond,
			when: now,
		};

		Self::try_plmc_participation_lock(bidder, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(bidder, project_id, funding_asset_amount_locked, asset_id)?;

		Bids::<T>::insert((project_id, bidder, bid_id), &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));
		BidCounts::<T>::mutate(project_id, |c| *c += 1);
		AuctionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount += ticket_size);

		Self::deposit_event(Event::Bid {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			ct_amount,
			ct_price: ct_usd_price,
			funding_asset,
			funding_amount: funding_asset_amount_locked,
			plmc_bond,
			multiplier,
		});

		Ok(new_bid)
	}
}
