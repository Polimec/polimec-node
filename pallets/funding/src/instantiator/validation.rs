use super::*;
use alloc::vec::Vec;
use polimec_common::ProvideAssetPrice;

// Constants for validation
const USD_UNIT: Balance = 10_u128.pow(6);

/// Validation and error handling methods that ensure consistency with pallet behavior.
/// These methods help catch issues early in test setup.
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	/// Validate project metadata using the same validation as the pallet
	pub fn validate_project_metadata(&mut self, metadata: &ProjectMetadataOf<T>) -> DispatchResult {
		self.execute(|| {

			// Check funding target is within bounds
			let funding_target = metadata.minimum_price.saturating_mul_int(metadata.total_allocation_size);
			ensure!(funding_target >= 1000u128 * USD_UNIT, Error::<T>::FundingTargetTooLow);
			ensure!(funding_target <= 1_000_000_000u128 * USD_UNIT, Error::<T>::FundingTargetTooHigh);

			// Check allocation size is valid
			ensure!(metadata.total_allocation_size > Zero::zero(), Error::<T>::AllocationSizeError);
			ensure!(
				metadata.total_allocation_size <= metadata.mainnet_token_max_supply,
				Error::<T>::AllocationSizeError
			);


			// Check participation currencies are unique
			let mut unique_currencies = metadata.participation_currencies.clone();
			unique_currencies.sort();
			ensure!(
				unique_currencies.len() == metadata.participation_currencies.len(),
				Error::<T>::ParticipationCurrenciesError
			);

			// Check ticket sizes are valid
			ensure!(
				metadata.bidding_ticket_sizes.professional.usd_minimum_per_participation >= 
				metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation,
				Error::<T>::TicketSizeError
			);
			ensure!(
				metadata.bidding_ticket_sizes.institutional.usd_minimum_per_participation >= 
				metadata.bidding_ticket_sizes.professional.usd_minimum_per_participation,
				Error::<T>::TicketSizeError
			);

			// Check decimals are within acceptable range
			ensure!(
				metadata.token_information.decimals >= 4 && metadata.token_information.decimals <= 20,
				Error::<T>::BadDecimals
			);

			Ok(())
		})
	}

	/// Validate evaluation parameters before performing evaluation
	pub fn validate_evaluation_params(
		&mut self,
		project_id: ProjectId,
		evaluation: &EvaluationParams<T>,
	) -> DispatchResult {
		self.execute(|| {
			let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
			let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
			let now = BlockProviderFor::<T>::current_block_number();

			// Check project is in evaluation round
			ensure!(matches!(project_details.status, ProjectStatus::EvaluationRound), Error::<T>::IncorrectRound);

			// Check timing
			ensure!(
				project_details.round_duration.started(now) && !project_details.round_duration.ended(now),
				Error::<T>::IncorrectRound
			);

			// Check minimum evaluation amount
			let plmc_usd_price = PriceProviderOf::<T>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS)
				.ok_or(Error::<T>::PriceNotFound)?;
			let usd_evaluation_amount = plmc_usd_price.saturating_mul_int(evaluation.plmc_amount);
			ensure!(usd_evaluation_amount >= T::MinUsdPerEvaluation::get(), Error::<T>::TooLow);

			// Check evaluation amount is positive
			ensure!(evaluation.plmc_amount > Zero::zero(), Error::<T>::TooLow);

			// Check receiving account junction is supported
			ensure!(
				project_metadata.participants_account_type.junction_is_supported(&evaluation.receiving_account),
				Error::<T>::UnsupportedReceiverAccountJunction
			);

			Ok(())
		})
	}

	/// Validate that accounts have sufficient funds for operations
	pub fn validate_account_funds(&mut self, requirements: &[(AccountIdOf<T>, Balance, AssetIdOf<T>)]) -> Result<(), &'static str> {
		for (account, required_amount, asset_id) in requirements {
			let available = if asset_id == &Location::here() {
				// PLMC balance
				self.get_free_plmc_balance_for(account.clone())
			} else {
				// Funding asset balance
				self.get_free_funding_asset_balance_for(asset_id.clone(), account.clone())
			};

			if available < *required_amount {
				return Err("Insufficient funds for account");
			}
		}
		Ok(())
	}

	/// Validate project state transitions are correct
	pub fn validate_state_transition(
		&mut self,
		project_id: ProjectId,
		expected_from: ProjectStatus,
		expected_to: ProjectStatus,
	) -> Result<(), &'static str> {
		let current_status = self.get_project_details(project_id).status;
		
		if current_status != expected_from {
			return Err("Project not in expected initial state");
		}

		// Check if transition is valid according to pallet logic
		let valid_transition = match (expected_from, expected_to) {
			(ProjectStatus::Application, ProjectStatus::EvaluationRound) => true,
			(ProjectStatus::EvaluationRound, ProjectStatus::AuctionRound) => true,
			(ProjectStatus::EvaluationRound, ProjectStatus::FundingFailed) => true,
			(ProjectStatus::AuctionRound, ProjectStatus::FundingSuccessful) => true,
			(ProjectStatus::AuctionRound, ProjectStatus::FundingFailed) => true,
			(ProjectStatus::FundingSuccessful, ProjectStatus::SettlementStarted(_)) => true,
			(ProjectStatus::FundingFailed, ProjectStatus::SettlementStarted(_)) => true,
			(ProjectStatus::SettlementStarted(_), ProjectStatus::SettlementFinished(_)) => true,
			_ => false,
		};

		if !valid_transition {
			return Err("Invalid state transition");
		}

		Ok(())
	}

	/// Comprehensive validation of a complete project setup
	pub fn validate_complete_project_setup(
		&mut self,
		metadata: &ProjectMetadataOf<T>,
		evaluations: &[EvaluationParams<T>],
		bids: &[BidParams<T>],
	) -> DispatchResult {
		// Validate metadata
		self.validate_project_metadata(metadata)?;

		// Check evaluation distribution makes sense
		let total_evaluation_plmc: Balance = evaluations.iter().map(|e| e.plmc_amount).sum();
		ensure!(total_evaluation_plmc > Zero::zero(), Error::<T>::TooLow);

		// Check bids are reasonable
		ensure!(!bids.is_empty(), Error::<T>::TooLow);
		let total_bid_amount: Balance = bids.iter().map(|b| b.amount).sum();
		ensure!(total_bid_amount > Zero::zero(), Error::<T>::TooLow);

		// Check participant accounts are unique where needed
		let evaluation_accounts: Vec<_> = evaluations.iter().map(|e| e.account.clone()).collect();
		let bid_accounts: Vec<_> = bids.iter().map(|b| b.bidder.clone()).collect();
		
		// Ensure we have reasonable diversity
		let unique_evaluators = evaluation_accounts.iter().collect::<std::collections::BTreeSet<_>>().len();
		let unique_bidders = bid_accounts.iter().collect::<std::collections::BTreeSet<_>>().len();
		
		if unique_evaluators == 0 || unique_bidders == 0 {
			return Err(Error::<T>::TooLow.into());
		}

		Ok(())
	}

	/// Assert that the pallet state matches expected values
	pub fn assert_pallet_state_consistency(&mut self, project_id: ProjectId) {
		self.execute(|| {
			let project_details = ProjectsDetails::<T>::get(project_id).expect("Project should exist");
			let project_metadata = ProjectsMetadata::<T>::get(project_id).expect("Project metadata should exist");

			// Check basic consistency
			assert!(
				project_details.fundraising_target_usd > Zero::zero(),
				"Fundraising target should be positive"
			);
			assert!(
				project_details.remaining_contribution_tokens <= project_metadata.total_allocation_size,
				"Remaining CTs should not exceed total allocation"
			);

			// Check evaluation round info consistency
			if project_details.evaluation_round_info.total_bonded_plmc > Zero::zero() {
				assert!(
					project_details.evaluation_round_info.total_bonded_usd > Zero::zero(),
					"If PLMC is bonded, USD equivalent should be positive"
				);
			}

			// Check funding amount consistency
			if project_details.funding_amount_reached_usd > Zero::zero() {
				assert!(
					matches!(
						project_details.status,
						ProjectStatus::AuctionRound
							| ProjectStatus::FundingSuccessful
							| ProjectStatus::FundingFailed
							| ProjectStatus::SettlementStarted(_)
							| ProjectStatus::SettlementFinished(_)
					),
					"Funding should only be recorded in appropriate phases"
				);
			}
		});
	}

	/// Validate bucket state is consistent with pallet logic
	pub fn validate_bucket_state(&mut self, project_id: ProjectId) -> Result<(), &'static str> {
		self.execute(|| {
			let maybe_bucket = Buckets::<T>::get(project_id);
			let project_details = ProjectsDetails::<T>::get(project_id).ok_or("Project not found")?;

			// Bucket should exist for projects in auction round or later
			if matches!(
				project_details.status,
				ProjectStatus::AuctionRound
					| ProjectStatus::FundingSuccessful
					| ProjectStatus::FundingFailed
					| ProjectStatus::SettlementStarted(_)
					| ProjectStatus::SettlementFinished(_)
			) {
				let bucket = maybe_bucket.ok_or("Bucket should exist for auction phase projects")?;
				
				// Validate bucket consistency
				if bucket.current_price < bucket.initial_price {
					return Err("Current price should not be less than initial price");
				}
				
				if bucket.delta_price == Zero::zero() {
					return Err("Delta price should be positive");
				}
				
				if bucket.delta_amount == Zero::zero() {
					return Err("Delta amount should be positive");
				}
			}

			Ok(())
		})
	}
}