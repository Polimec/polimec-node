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
	/// round, and the current block is after the auction closing end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Community Funding round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// round.
	///
	/// # Next step
	/// Retail users buy tokens at the price set on the auction round.
	/// Later on, `on_initialize` ends the community round by calling [`do_remainder_funding`](Self::do_start_remainder_funding) and
	/// starts the remainder round, where anyone can buy at that price point.
	#[transactional]
	pub fn do_start_community_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_closing_start_block =
			project_details.phase_transition_points.auction_closing.start().ok_or(Error::<T>::TransitionPointNotSet)?;
		let auction_closing_end_block =
			project_details.phase_transition_points.auction_closing.end().ok_or(Error::<T>::TransitionPointNotSet)?;

		// * Validity checks *
		ensure!(now > auction_closing_end_block, Error::<T>::TooEarlyForRound);
		ensure!(project_details.status == ProjectStatus::CalculatingWAP, Error::<T>::IncorrectRound);

		// * Calculate new variables *
		let end_block = Self::select_random_block(auction_closing_start_block, auction_closing_end_block);
		let community_start_block = now;
		let community_end_block = now.saturating_add(T::CommunityFundingDuration::get()).saturating_sub(One::one());
		let auction_allocation_size =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

		// * Update Storage *
		let wap_result = Self::calculate_weighted_average_price(project_id);

		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		match wap_result {
			Err(e) => return Err(DispatchErrorWithPostInfo { post_info: ().into(), error: e }),
			Ok(winning_bids_count) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_closing_ending = Some(end_block);
				project_details
					.phase_transition_points
					.community
					.update(Some(community_start_block), Some(community_end_block));
				project_details.status = ProjectStatus::CommunityRound;
				ProjectsDetails::<T>::insert(project_id, project_details);

				let insertion_iterations = match Self::add_to_update_store(
					community_end_block + 1u32.into(),
					(&project_id, UpdateType::RemainderFundingStart),
				) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

				// * Emit events *
				Self::deposit_event(Event::<T>::ProjectPhaseTransition {
					project_id,
					phase: ProjectPhases::CommunityFunding,
				});

				//TODO: address this
				let rejected_bids_count = 0;
				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::start_community_funding(
						insertion_iterations,
						winning_bids_count,
						rejected_bids_count,
					)),
					pays_fee: Pays::Yes,
				})
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
	#[transactional]
	pub fn do_start_remainder_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block =
			project_details.phase_transition_points.community.end().ok_or(Error::<T>::TransitionPointNotSet)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRound);
		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::IncorrectRound);

		// Transition to remainder round was initiated by `do_community_funding`, but the ct
		// tokens where already sold in the community round. This transition is obsolete.
		ensure!(
			project_details.remaining_contribution_tokens > 0u32.into(),
			Error::<T>::RoundTransitionAlreadyHappened
		);

		// * Calculate new variables *
		let remainder_start_block = now;
		let remainder_end_block = now.saturating_add(T::RemainderFundingDuration::get()).saturating_sub(One::one());

		// * Update Storage *
		project_details
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block));
		project_details.status = ProjectStatus::RemainderRound;
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic transition by `on_initialize`
		let insertion_iterations =
			match Self::add_to_update_store(remainder_end_block + 1u32.into(), (&project_id, UpdateType::FundingEnd)) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};

		// * Emit events *
		Self::deposit_event(Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::RemainderFunding });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_remainder_funding(insertion_iterations)),
			pays_fee: Pays::Yes,
		})
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens the contributor tries to buy. Tokens
	///   are limited by the total amount of tokens available in the Community Round.
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	/// * asset: The asset used for the contribution
	#[transactional]
	pub fn do_community_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
		whitelisted_policy: Cid,
	) -> DispatchResultWithPostInfo {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let did_has_winning_bid = DidWithWinningBids::<T>::get(project_id, did.clone());

		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::IncorrectRound);
		ensure!(!did_has_winning_bid, Error::<T>::UserHasWinningBid);

		let buyable_tokens = token_amount.min(project_details.remaining_contribution_tokens);
		project_details.remaining_contribution_tokens.saturating_reduce(buyable_tokens);

		Self::do_contribute(
			contributor,
			project_id,
			&mut project_details,
			buyable_tokens,
			multiplier,
			asset,
			investor_type,
			did,
			whitelisted_policy,
		)
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens the contributor tries to buy. Tokens
	///   are limited by the total amount of tokens available after the Auction and Community rounds.
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	/// * asset: The asset used for the contribution
	#[transactional]
	pub fn do_remaining_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
		whitelisted_policy: Cid,
	) -> DispatchResultWithPostInfo {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.status == ProjectStatus::RemainderRound, Error::<T>::IncorrectRound);
		let buyable_tokens = token_amount.min(project_details.remaining_contribution_tokens);

		let before = project_details.remaining_contribution_tokens;
		let remaining_cts_in_round = before.saturating_sub(buyable_tokens);
		project_details.remaining_contribution_tokens = remaining_cts_in_round;

		Self::do_contribute(
			contributor,
			project_id,
			&mut project_details,
			token_amount,
			multiplier,
			asset,
			investor_type,
			did,
			whitelisted_policy,
		)
	}

	#[transactional]
	fn do_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		project_details: &mut ProjectDetailsOf<T>,
		buyable_tokens: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		investor_type: InvestorType,
		did: Did,
		whitelisted_policy: Cid,
	) -> DispatchResultWithPostInfo {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let caller_existing_contributions =
			Contributions::<T>::iter_prefix_values((project_id, contributor)).collect::<Vec<_>>();
		let total_usd_bought_by_did = ContributionBoughtUSD::<T>::get((project_id, did.clone()));
		let now = <frame_system::Pallet<T>>::block_number();
		let ct_usd_price = project_details.weighted_average_price.ok_or(Error::<T>::WapNotSet)?;
		let plmc_usd_price = T::PriceProvider::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.ok_or(Error::<T>::PriceNotFound)?;

		let funding_asset_id = funding_asset.to_assethub_id();
		let funding_asset_decimals = T::FundingCurrency::decimals(funding_asset_id);
		let funding_asset_usd_price =
			T::PriceProvider::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
				.ok_or(Error::<T>::PriceNotFound)?;

		let project_policy = project_metadata.policy_ipfs_cid.ok_or(Error::<T>::ImpossibleState)?;

		let ticket_size = ct_usd_price.checked_mul_int(buyable_tokens).ok_or(Error::<T>::BadMath)?;
		let contributor_ticket_size = match investor_type {
			InvestorType::Institutional => project_metadata.contributing_ticket_sizes.institutional,
			InvestorType::Professional => project_metadata.contributing_ticket_sizes.professional,
			InvestorType::Retail => project_metadata.contributing_ticket_sizes.retail,
		};
		let max_multiplier = match investor_type {
			InvestorType::Retail => {
				RetailParticipations::<T>::mutate(&did, |project_participations| {
					if project_participations.contains(&project_id).not() {
						// We don't care if it fails, since it means the user already has access to the max multiplier
						let _ = project_participations.try_push(project_id);
					}
					retail_max_multiplier_for_participations(project_participations.len() as u8)
				})
			},

			InvestorType::Professional => PROFESSIONAL_MAX_MULTIPLIER,
			InvestorType::Institutional => INSTITUTIONAL_MAX_MULTIPLIER,
		};
		// * Validity checks *
		ensure!(project_policy == whitelisted_policy, Error::<T>::PolicyMismatch);
		ensure!(multiplier.into() <= max_multiplier && multiplier.into() > 0u8, Error::<T>::ForbiddenMultiplier);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);
		ensure!(did.clone() != project_details.issuer_did, Error::<T>::ParticipationToOwnProject);
		ensure!(
			caller_existing_contributions.len() < T::MaxContributionsPerUser::get() as usize,
			Error::<T>::TooManyUserParticipations
		);
		ensure!(contributor_ticket_size.usd_ticket_above_minimum_per_participation(ticket_size), Error::<T>::TooLow);
		ensure!(
			contributor_ticket_size.usd_ticket_below_maximum_per_did(total_usd_bought_by_did + ticket_size),
			Error::<T>::TooHigh
		);

		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price)?;
		let funding_asset_amount =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_assethub_id();

		let contribution_id = NextContributionId::<T>::get();
		let new_contribution = ContributionInfoOf::<T> {
			did: did.clone(),
			id: contribution_id,
			project_id,
			contributor: contributor.clone(),
			ct_amount: buyable_tokens,
			usd_contribution_amount: ticket_size,
			multiplier,
			funding_asset,
			funding_asset_amount,
			plmc_bond,
		};

		// Try adding the new contribution to the system
		Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;

		Contributions::<T>::insert((project_id, contributor, contribution_id), &new_contribution);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));
		ContributionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount += ticket_size);

		let remaining_cts_after_purchase = project_details.remaining_contribution_tokens;
		project_details.funding_amount_reached_usd.saturating_accrue(new_contribution.usd_contribution_amount);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// If no CTs remain, end the funding phase

		let mut weight_round_end_flag: Option<u32> = None;
		if remaining_cts_after_purchase.is_zero() {
			let fully_filled_vecs_from_insertion =
				match Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::FundingEnd)) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

			weight_round_end_flag = Some(fully_filled_vecs_from_insertion);
		}

		// * Emit events *
		Self::deposit_event(Event::Contribution {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			ct_amount: buyable_tokens,
			funding_asset,
			funding_amount: funding_asset_amount,
			plmc_bond,
			multiplier,
		});

		// return correct weight function
		let actual_weight = match weight_round_end_flag {
			None => Some(WeightInfoOf::<T>::contribution(caller_existing_contributions.len() as u32)),
			Some(fully_filled_vecs_from_insertion) => Some(WeightInfoOf::<T>::contribution_ends_round(
				caller_existing_contributions.len() as u32,
				fully_filled_vecs_from_insertion,
			)),
		};

		Ok(PostDispatchInfo { actual_weight, pays_fee: Pays::Yes })
	}
}
