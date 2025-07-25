use super::*;
use crate::{DoBidParams, MultiplierOf, ParticipationMode};
use alloc::vec::Vec;
use polimec_common::assets::AcceptedFundingAsset;
use polimec_common::ProvideAssetPrice;

/// Pallet-aligned methods that use the actual pallet logic instead of reimplementing it.
/// This ensures consistency between test setup and actual pallet behavior.
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	/// Calculate PLMC bond using the pallet's actual calculation logic
	pub fn calculate_plmc_bond_with_pallet(
		&mut self,
		usd_amount: Balance,
		multiplier: MultiplierOf<T>,
	) -> Balance {
		self.execute(|| Pallet::<T>::calculate_plmc_bond(usd_amount, multiplier).unwrap_or_default())
	}

	/// Calculate funding asset amount using the pallet's actual calculation logic  
	pub fn calculate_funding_asset_with_pallet(
		&mut self,
		usd_amount: Balance,
		asset: AcceptedFundingAsset,
	) -> Balance {
		self.execute(|| Pallet::<T>::calculate_funding_asset_amount(usd_amount, asset).unwrap_or_default())
	}

	/// Get the current bucket state for a project
	pub fn get_current_bucket(&mut self, project_id: ProjectId) -> BucketOf<T> {
		self.execute(|| Buckets::<T>::get(project_id).expect("Project bucket should exist"))
	}

	/// Update bucket state using pallet's bucket logic
	pub fn update_bucket(&mut self, project_id: ProjectId, ct_amount: Balance) {
		self.execute(|| {
			let mut bucket = Buckets::<T>::get(project_id).expect("Project bucket should exist");
			bucket.update(ct_amount);
			Buckets::<T>::insert(project_id, bucket);
		});
	}

	/// Simulate what CT amount and price would result from a funding asset amount
	/// Uses the same logic as the pallet's do_bid function
	pub fn simulate_ct_amount_from_funding_asset(
		&mut self,
		project_id: ProjectId,
		funding_asset_amount: Balance,
		funding_asset: AcceptedFundingAsset,
	) -> Result<(Balance, PriceOf<T>), &'static str> {
		self.execute(|| {
			let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or("Project not found")?;
			let bucket = Buckets::<T>::get(project_id).ok_or("Bucket not found")?;

			let funding_asset_price = PriceProviderOf::<T>::get_decimals_aware_price(
				&funding_asset.id(),
				funding_asset.decimals(),
			)
			.ok_or("Price not found")?;

			// Use the same calculation as the pallet
			let price_ratio = funding_asset_price.checked_div(&bucket.current_price).ok_or("Division error")?;
			let raw_ct_amount = price_ratio.checked_mul_int(funding_asset_amount).ok_or("Multiplication error")?;

			let rounding_step = 10u128
				.checked_pow(project_metadata.token_information.decimals as u32 - USD_DECIMALS as u32)
				.ok_or("Rounding step calculation failed")?;

			let total_ct_amount = polimec_common::round_to_nearest(raw_ct_amount, rounding_step)
				.ok_or("Rounding failed")?;

			Ok((total_ct_amount, bucket.current_price))
		})
	}

	/// Get exact funding requirements for a list of bids using pallet calculations
	/// This version simulates bucket updates without actually modifying the bucket state
	pub fn get_exact_funding_requirements_for_bids(
		&mut self,
		project_id: ProjectId,
		bids: &[BidParams<T>],
	) -> (Vec<UserToPLMCBalance<T>>, Vec<UserToFundingAsset<T>>) {
		let mut plmc_requirements = Vec::new();
		let mut funding_asset_requirements = Vec::new();

		// Start with the current bucket state
		let mut simulated_bucket = self.get_current_bucket(project_id);

		for bid in bids {
			// Get the CT amount and price using simulated bucket (no state modification)
			if let Ok((ct_amount, ct_price)) = self.simulate_ct_amount_from_funding_asset_with_bucket(
				project_id, bid.amount, bid.asset, &simulated_bucket
			) {
				// Calculate PLMC bond using pallet logic
				let usd_ticket_size = ct_price.saturating_mul_int(ct_amount);
				if let ParticipationMode::Classic(multiplier) = bid.mode {
					let multiplier_typed: MultiplierOf<T> = multiplier.try_into().unwrap_or_default();
					let plmc_bond = self.calculate_plmc_bond_with_pallet(usd_ticket_size, multiplier_typed);
					plmc_requirements.push(UserToPLMCBalance::new(bid.bidder.clone(), plmc_bond));
				}

				// Calculate funding asset requirement using pallet logic
				let mut funding_asset_needed = self.calculate_funding_asset_with_pallet(usd_ticket_size, bid.asset);

				// Add OTM fee if needed
				if bid.mode == ParticipationMode::OTM {
					let otm_fee = self.calculate_otm_fee_with_pallet(usd_ticket_size, bid.asset);
					funding_asset_needed = funding_asset_needed.saturating_add(otm_fee);
				}

				funding_asset_requirements.push(UserToFundingAsset::new(
					bid.bidder.clone(),
					funding_asset_needed,
					bid.asset.id(),
				));

				// Update the simulated bucket for the next bid calculation (no state change)
				simulated_bucket.update(ct_amount);
			}
		}

		// Merge accounts to handle multiple bids from same user
		let merged_plmc = plmc_requirements.merge_accounts(MergeOperation::Add);
		let merged_funding = funding_asset_requirements.merge_accounts(MergeOperation::Add);

		(merged_plmc, merged_funding)
	}

	/// Simulate CT amount calculation with a provided bucket (no state modification)
	pub fn simulate_ct_amount_from_funding_asset_with_bucket(
		&mut self,
		project_id: ProjectId,
		funding_asset_amount: Balance,
		funding_asset: AcceptedFundingAsset,
		bucket: &BucketOf<T>,
	) -> Result<(Balance, PriceOf<T>), &'static str> {
		self.execute(|| {
			let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or("Project not found")?;

			let funding_asset_price = PriceProviderOf::<T>::get_decimals_aware_price(
				&funding_asset.id(),
				funding_asset.decimals(),
			)
			.ok_or("Price not found")?;

			// Use the provided bucket instead of querying state
			let price_ratio = funding_asset_price.checked_div(&bucket.current_price).ok_or("Division error")?;
			let raw_ct_amount = price_ratio.checked_mul_int(funding_asset_amount).ok_or("Multiplication error")?;

			let rounding_step = 10u128
				.checked_pow(project_metadata.token_information.decimals as u32 - USD_DECIMALS as u32)
				.ok_or("Rounding step calculation failed")?;

			let total_ct_amount = polimec_common::round_to_nearest(raw_ct_amount, rounding_step)
				.ok_or("Rounding failed")?;

			Ok((total_ct_amount, bucket.current_price))
		})
	}

	/// Calculate OTM fee using pallet logic
	pub fn calculate_otm_fee_with_pallet(&mut self, usd_ticket_size: Balance, funding_asset: AcceptedFundingAsset) -> Balance {
		self.execute(|| {
			let multiplier: MultiplierOf<T> = ParticipationMode::OTM.multiplier().try_into().unwrap_or_default();
			let plmc_usd_price = PriceProviderOf::<T>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS)
				.unwrap_or_default();
			let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(usd_ticket_size).unwrap_or_default();
			let plmc_bond = plmc_usd_price.reciprocal().unwrap_or_default().saturating_mul_int(usd_bond);
			pallet_proxy_bonding::Pallet::<T>::calculate_fee(plmc_bond, funding_asset.id()).unwrap_or_default()
		})
	}

	/// Perform bid using pallet's actual do_bid logic and return the created bid info
	pub fn perform_bid_with_pallet_logic(
		&mut self,
		project_id: ProjectId,
		bid: BidParams<T>,
		did: Did,
		whitelisted_policy: Cid,
	) -> Result<Vec<BidInfoOf<T>>, DispatchError> {
		self.execute(|| {
			// Count existing bids before the new bid
			let bids_before: Vec<BidInfoOf<T>> = Bids::<T>::iter_prefix_values(project_id).collect();
			let bids_count_before = bids_before.len();

			// Perform the bid using pallet logic
			let params = DoBidParams::<T> {
				bidder: bid.bidder.clone(),
				project_id,
				funding_asset_amount: bid.amount,
				mode: bid.mode,
				funding_asset: bid.asset,
				did,
				investor_type: bid.investor_type,
				whitelisted_policy,
				receiving_account: bid.receiving_account,
			};

			Pallet::<T>::do_bid(params).expect("Bid should succeed");

			// Get the newly created bids
			let bids_after: Vec<BidInfoOf<T>> = Bids::<T>::iter_prefix_values(project_id).collect();
			let new_bids = bids_after.into_iter().skip(bids_count_before).collect();

			Ok(new_bids)
		})
	}

	/// Validate bid parameters using the same checks as the pallet
	pub fn validate_bid_parameters(
		&mut self,
		project_id: ProjectId,
		bid: &BidParams<T>,
		did: Did,
		whitelisted_policy: Cid,
	) -> DispatchResult {
		self.execute(|| {
			let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
			let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
			let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
			let now = BlockProviderFor::<T>::current_block_number();

			// Simulate CT amount calculation
			let funding_asset_price = PriceProviderOf::<T>::get_decimals_aware_price(
				&bid.asset.id(),
				bid.asset.decimals(),
			)
			.ok_or(Error::<T>::PriceNotFound)?;

			let price_ratio = funding_asset_price.checked_div(&bucket.current_price).ok_or(Error::<T>::BadMath)?;
			let raw_ct_amount = price_ratio.checked_mul_int(bid.amount).ok_or(Error::<T>::BadMath)?;

			let rounding_step = 10u128
				.checked_pow(project_metadata.token_information.decimals as u32 - USD_DECIMALS as u32)
				.ok_or(Error::<T>::BadMath)?;

			let total_ct_amount = polimec_common::round_to_nearest(raw_ct_amount, rounding_step)
				.ok_or(Error::<T>::BadMath)?;

			let min_total_ticket_size_usd = bucket.current_price.checked_mul_int(total_ct_amount).ok_or(Error::<T>::BadMath)?;

			// Validate the same way as the pallet
			let metadata_ticket_size_bounds = match bid.investor_type {
				InvestorType::Institutional => project_metadata.bidding_ticket_sizes.institutional,
				InvestorType::Professional => project_metadata.bidding_ticket_sizes.professional,
				InvestorType::Retail => project_metadata.bidding_ticket_sizes.retail,
			};

			let max_multiplier = match bid.investor_type {
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
				project_metadata.participation_currencies.contains(&bid.asset),
				Error::<T>::FundingAssetNotAccepted
			);
			ensure!(
				metadata_ticket_size_bounds.usd_ticket_above_minimum_per_participation(min_total_ticket_size_usd),
				Error::<T>::TooLow
			);
			ensure!(bid.mode.multiplier() <= max_multiplier && bid.mode.multiplier() > 0u8, Error::<T>::ForbiddenMultiplier);
			ensure!(
				project_metadata.participants_account_type.junction_is_supported(&bid.receiving_account),
				Error::<T>::UnsupportedReceiverAccountJunction
			);

			Ok(())
		})
	}

	/// Calculate bid requirements using pallet's actual logic
	pub fn calculate_bid_requirements_with_pallet(
		&mut self,
		project_id: ProjectId,
		bids: &[BidParams<T>],
	) -> (Vec<UserToPLMCBalance<T>>, Vec<UserToFundingAsset<T>>) {
		self.get_exact_funding_requirements_for_bids(project_id, bids)
	}
}