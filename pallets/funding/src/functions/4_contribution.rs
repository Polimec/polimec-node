use super::*;

impl<T: Config> Pallet<T> {

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
	pub fn do_contribute(
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
		let round_start_block = project_details.round_duration.start().ok_or(Error::<T>::ImpossibleState)?;
		
		let remainder_start = match project_details.status {
			ProjectStatus::CommunityRound(remainder_start) => remainder_start,
			_ => return Err(Error::<T>::IncorrectRound.into()),
		};
		
		let now = <frame_system::Pallet<T>>::block_number();
		let remainder_started = now > remainder_start;
		ensure!(!did_has_winning_bid || remainder_started, Error::<T>::UserHasWinningBid);

		let buyable_tokens = token_amount.min(project_details.remaining_contribution_tokens);
		project_details.remaining_contribution_tokens.saturating_reduce(buyable_tokens);

		Self::do_perform_contribution(
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

	#[transactional]
	fn do_perform_contribution(
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
		ensure!(contributor_ticket_size.usd_ticket_above_minimum_per_participation(ticket_size) || project_details.remaining_contribution_tokens.is_zero(), Error::<T>::TooLow);
		ensure!(
			contributor_ticket_size.usd_ticket_below_maximum_per_did(total_usd_bought_by_did + ticket_size),
			Error::<T>::TooHigh
		);

		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier)?;
		let funding_asset_amount = Self::calculate_funding_asset_amount(ticket_size, funding_asset)?;

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
			when: now,
		};

		// Try adding the new contribution to the system
		Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, funding_asset.to_assethub_id())?;

		Contributions::<T>::insert((project_id, contributor, contribution_id), &new_contribution);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));
		ContributionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount += ticket_size);

		project_details.funding_amount_reached_usd.saturating_accrue(new_contribution.usd_contribution_amount);
		ProjectsDetails::<T>::insert(project_id, project_details);

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
		let actual_weight = Some(WeightInfoOf::<T>::contribution(caller_existing_contributions.len() as u32));
		Ok(PostDispatchInfo { actual_weight, pays_fee: Pays::Yes })
	}
}
