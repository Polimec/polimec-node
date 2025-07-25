use super::*;
use alloc::vec::Vec;
use polimec_common::{assets::AcceptedFundingAsset, ProvideAssetPrice};
use polimec_common::assets::AcceptedFundingAsset::{USDT, USDC, DOT, ETH};
use crate::InvestorType::{self, *};

// Common constants for testing
const USD_UNIT: Balance = 10_u128.pow(6);
const CT_UNIT: Balance = 10_u128.pow(18);

/// Focused test helper methods for common testing scenarios.
/// These methods provide simple, composable building blocks for tests.
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	/// Create a minimal project for testing basic functionality
	pub fn create_minimal_project(&mut self, issuer: AccountIdOf<T>) -> ProjectId {
		let metadata = self.get_default_project_metadata(issuer.clone());
		self.create_project_with_pallet(metadata, issuer, None)
	}

	/// Create a project and immediately start evaluation
	pub fn create_project_in_evaluation(&mut self, issuer: AccountIdOf<T>) -> ProjectId {
		let project_id = self.create_minimal_project(issuer);
		self.start_evaluation_with_pallet(project_id).unwrap();
		project_id
	}

	pub fn generate_successful_evaluations_with_pallet(
		&mut self,
		metadata: ProjectMetadataOf<T>,
		count: u8,
	) -> Vec<EvaluationParams<T>> {
		let evaluators = self.create_test_accounts(count as u32, "EVALUATOR");
		let total_evaluation_plmc = self.calculate_successful_evaluation_amount(&metadata);

		let mut evaluations = Vec::new();
		let base_amount = total_evaluation_plmc / (count as u128);

		for (i, evaluator) in evaluators.iter().enumerate() {
			let variance_factor = if i == 0 {
				150
			} else if i < count as usize / 2 {
				120
			} else {
				80
			};
			let evaluation_amount = (base_amount * variance_factor) / 100;

			evaluations.push(EvaluationParams::from((evaluator.clone(), evaluation_amount)));
		}

		evaluations
	}

	/// Create a project with successful evaluation that reaches auction
	pub fn create_project_in_auction(&mut self, issuer: AccountIdOf<T>, evaluation_count: u8) -> ProjectId {
		let metadata = self.get_default_project_metadata(issuer.clone());
		let project_id = self.create_project_with_pallet(metadata.clone(), issuer, None);

		self.start_evaluation_with_pallet(project_id).unwrap();

		let evaluations = self.generate_successful_evaluations_with_pallet(metadata, evaluation_count);
		self.perform_evaluations_with_pallet(project_id, evaluations).unwrap();

		// Wait for evaluation period to end before calling end_evaluation
		let project_details = self.get_project_details(project_id);
		if let Some(end_block) = project_details.round_duration.end() {
			self.jump_to_block(end_block + One::one());
		}

		self.end_evaluation_with_pallet(project_id).unwrap();
		self.assert_project_state(project_id, ProjectStatus::AuctionRound);

		project_id
	}

	/// Create a project that has completed funding (successful or failed)
	pub fn create_completed_project(
		&mut self,
		issuer: AccountIdOf<T>,
		evaluation_count: u8,
		bid_count: u32,
		funding_percentage: u8, // Percentage of total allocation to fund
	) -> ProjectId {
		let metadata = self.get_default_project_metadata(issuer.clone());
		let project_id = self.create_project_in_auction(issuer, evaluation_count);

		// Generate appropriate amount of bids using existing method that handles funding
		let bids = self.generate_bids_from_total_ct_percent(metadata.clone(), funding_percentage, bid_count);

		self.perform_bids_with_pallet(project_id, bids).unwrap();

		// Wait for auction period to end before calling end_funding
		let project_details = self.get_project_details(project_id);
		if let Some(end_block) = project_details.round_duration.end() {
			self.jump_to_block(end_block + One::one());
		}

		self.end_funding_with_pallet(project_id).unwrap();

		project_id
	}

	/// Create a successfully funded project ready for settlement
	pub fn create_successful_project(&mut self, issuer: AccountIdOf<T>) -> ProjectId {
		let project_id = self.create_completed_project(issuer, 5, 10, 70); // 70% funding
		self.assert_project_state(project_id, ProjectStatus::FundingSuccessful);
		project_id
	}

	/// Create a failed project (insufficient funding)
	pub fn create_failed_project(&mut self, issuer: AccountIdOf<T>) -> ProjectId {
		let project_id = self.create_completed_project(issuer, 5, 10, 20); // 20% funding (below threshold)
		self.assert_project_state(project_id, ProjectStatus::FundingFailed);
		project_id
	}

	/// Create a settled project with all participations settled
	pub fn create_fully_settled_project(&mut self, issuer: AccountIdOf<T>) -> ProjectId {
		let project_id = self.create_successful_project(issuer);
		self.start_settlement_with_pallet(project_id).unwrap();
		self.settle_project_with_pallet(project_id, true);
		project_id
	}

	pub fn bounded_name(&self) -> BoundedVec<u8, StringLimitOf<T>> {
		BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap()
	}

	pub fn bounded_symbol(&self) -> BoundedVec<u8, StringLimitOf<T>> {
		BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap()
	}

	pub fn ipfs_hash(&self) -> Cid {
		const IPFS_CID: &str = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";
		Cid::try_from(IPFS_CID.as_bytes().to_vec()).unwrap()
	}

	/// Get default project metadata suitable for testing - delegates to existing method
	pub fn get_default_project_metadata(&self, issuer: AccountIdOf<T>) -> ProjectMetadataOf<T> {
		ProjectMetadata {
			token_information: CurrencyMetadata {
				name: self.bounded_name(),
				symbol: self.bounded_symbol(),
				decimals: 18,
			},
			mainnet_token_max_supply: 8_000_000 * CT_UNIT,
			total_allocation_size: 1_000_000 * CT_UNIT,
			minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(
				PriceOf::<T>::saturating_from_rational(10,1),
				USD_DECIMALS,
				18,
			)
			.unwrap(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(5000 * USD_UNIT, None),
				institutional: TicketSize::new(5000 * USD_UNIT, None),
				retail: TicketSize::new(100 * USD_UNIT, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![USDT, USDC, DOT, ETH].try_into().unwrap(),
			funding_destination_account: issuer,
			policy_ipfs_cid: Some(self.ipfs_hash()),
			participants_account_type: ParticipantsAccountType::Polkadot,
		}
	}

	/// Create test accounts with proper naming
	pub fn create_test_accounts(&self, count: u32, prefix: &str) -> Vec<AccountIdOf<T>> {
		(0..count).map(|i| self.account_from_u32(i, prefix)).collect()
	}

	/// Setup accounts with PLMC and funding assets for testing
	pub fn setup_test_accounts_with_funds(
		&mut self,
		accounts: &[AccountIdOf<T>],
		plmc_amount: Balance,
		funding_assets: &[(AcceptedFundingAsset, Balance)],
	) {
		// Mint PLMC to accounts
		let plmc_balances: Vec<UserToPLMCBalance<T>> =
			accounts.iter().map(|account| UserToPLMCBalance::new(account.clone(), plmc_amount)).collect();
		self.mint_plmc_to(plmc_balances);

		// Mint funding assets to accounts
		for (asset, amount) in funding_assets {
			let asset_balances: Vec<UserToFundingAsset<T>> =
				accounts.iter().map(|account| UserToFundingAsset::new(account.clone(), *amount, asset.id())).collect();
			self.mint_funding_asset_to(asset_balances);
		}
	}

	/// Create a realistic evaluation scenario with multiple evaluators
	pub fn create_realistic_evaluations(
		&mut self,
		project_metadata: &ProjectMetadataOf<T>,
		evaluator_count: u8,
	) -> Vec<EvaluationParams<T>> {
		let evaluators = self.create_test_accounts(evaluator_count as u32, "EVALUATOR");
		let total_evaluation_plmc = self.calculate_successful_evaluation_amount(project_metadata);

		// Distribute evaluation amounts with some variance
		let mut evaluations = Vec::new();
		let base_amount = total_evaluation_plmc / (evaluator_count as u128);

		for (i, evaluator) in evaluators.iter().enumerate() {
			// Add some variance: larger evaluators get more, smaller get less
			let variance_factor = if i == 0 {
				150
			} else if i < evaluator_count as usize / 2 {
				120
			} else {
				80
			};
			let evaluation_amount = (base_amount * variance_factor) / 100;

			evaluations.push(EvaluationParams::from((evaluator.clone(), evaluation_amount)));
		}

		// Setup accounts with funds
		let evaluation_amounts: Vec<UserToPLMCBalance<T>> =
			evaluations.iter().map(|e: &EvaluationParams<T>| UserToPLMCBalance::new(e.account.clone(), e.plmc_amount)).collect();
		self.mint_plmc_ed_if_required(evaluation_amounts.accounts());
		self.mint_plmc_to(evaluation_amounts);

		evaluations
	}

	/// Create a realistic bid scenario with diverse participants
	pub fn create_realistic_bids(
		&mut self,
		project_id: ProjectId,
		bid_count: u32,
		total_funding_percentage: u8,
	) -> Vec<BidParams<T>> {
		let project_metadata = self.get_project_metadata(project_id);
		let total_ct_to_buy = (total_funding_percentage as u128 * project_metadata.total_allocation_size) / 100u128;

		let bidders = self.create_test_accounts(bid_count, "BIDDER");
		let mut bids = Vec::new();

		// Create diverse bid sizes and types
		for (i, bidder) in bidders.iter().enumerate() {
			let investor_type = match i % 3 {
				0 => InvestorType::Retail,
				1 => InvestorType::Professional,
				_ => InvestorType::Institutional,
			};

			let mode = if i % 4 == 0 { ParticipationMode::OTM } else { ParticipationMode::Classic((i % 5 + 1) as u8) };

			let asset = match i % 2 {
				0 => AcceptedFundingAsset::USDT,
				_ => AcceptedFundingAsset::USDT, // For now, just USDT
			};

			// Vary bid sizes: some large, some medium, some small
			let size_factor = match i % 4 {
				0 => 200, // Large bids
				1 => 100, // Medium bids
				2 => 50,  // Small bids
				_ => 25,  // Very small bids
			};

			let ct_amount = (total_ct_to_buy * size_factor) / (bid_count as u128 * 100);
			if ct_amount > 0 {
				// Calculate funding asset needed for this CT amount
				let bucket = self.get_current_bucket(project_id);
				let usd_needed = bucket.current_price.saturating_mul_int(ct_amount);

				let funding_asset_price = self
					.execute(|| PriceProviderOf::<T>::get_decimals_aware_price(&asset.id(), asset.decimals()).unwrap());
				let funding_asset_amount =
					funding_asset_price.reciprocal().unwrap().checked_mul_int(usd_needed).unwrap();

				let bid = BidParams::from((bidder.clone(), investor_type, funding_asset_amount, mode, asset));

				bids.push(bid);

				// Update bucket for next calculation
				self.update_bucket(project_id, ct_amount);
			}
		}

		// Setup accounts with required funds
		let (plmc_requirements, funding_requirements) = self.calculate_bid_requirements_with_pallet(project_id, &bids);
		self.mint_plmc_ed_if_required(plmc_requirements.accounts());
		self.mint_funding_asset_ed_if_required(funding_requirements.to_account_asset_map());
		self.mint_plmc_to(plmc_requirements);
		self.mint_funding_asset_to(funding_requirements);

		bids
	}


	/// Calculate the PLMC amount needed for successful evaluation
	fn calculate_successful_evaluation_amount(&mut self, project_metadata: &ProjectMetadataOf<T>) -> Balance {
		let funding_target_usd =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		let target_usd_for_success = Percent::from_percent(100) * funding_target_usd;

		let plmc_usd_price =
			self.execute(|| PriceProviderOf::<T>::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap());

		plmc_usd_price.reciprocal().unwrap().checked_mul_int(target_usd_for_success).unwrap()
	}

	/// Run a complete project lifecycle test with validation at each step
	pub fn run_full_project_lifecycle_test(
		&mut self,
		issuer: AccountIdOf<T>,
		evaluation_count: u8,
		bid_count: u32,
		funding_percentage: u8,
	) -> ProjectId {
		// Step 1: Create and validate project
		let metadata = self.get_default_project_metadata(issuer.clone());
		self.validate_project_metadata(&metadata).unwrap();
		let project_id = self.create_project_with_pallet(metadata.clone(), issuer, None);
		self.assert_pallet_state_consistency(project_id);

		// Step 2: Start evaluation and validate state
		self.start_evaluation_with_pallet(project_id).unwrap();
		self.assert_project_state(project_id, ProjectStatus::EvaluationRound);

		// Step 3: Perform evaluations with validation
		let evaluations = self.create_realistic_evaluations(&metadata, evaluation_count);
		for evaluation in &evaluations {
			self.validate_evaluation_params(project_id, evaluation).unwrap();
		}
		self.perform_evaluations_with_pallet(project_id, evaluations).unwrap();

		// Step 4: Wait for evaluation to end, then end evaluation and validate transition
		let project_details = self.get_project_details(project_id);
		if let Some(end_block) = project_details.round_duration.end() {
			self.jump_to_block(end_block + One::one());
		}
		self.end_evaluation_with_pallet(project_id).unwrap();
		let new_status = self.get_project_details(project_id).status;
		assert!(matches!(new_status, ProjectStatus::AuctionRound | ProjectStatus::FundingFailed));

		if matches!(new_status, ProjectStatus::AuctionRound) {
			// Step 5: Perform bids with validation
			self.validate_bucket_state(project_id).unwrap();
			let bids = self.create_realistic_bids(project_id, bid_count, funding_percentage);
			self.perform_bids_with_pallet(project_id, bids).unwrap();

			// Step 6: Wait for auction to end, then end funding and validate
			let project_details = self.get_project_details(project_id);
			if let Some(end_block) = project_details.round_duration.end() {
				self.jump_to_block(end_block + One::one());
			}
			self.end_funding_with_pallet(project_id).unwrap();
			let final_status = self.get_project_details(project_id).status;
			assert!(matches!(final_status, ProjectStatus::FundingSuccessful | ProjectStatus::FundingFailed));

			// Step 7: If successful, test settlement
			if matches!(final_status, ProjectStatus::FundingSuccessful) {
				self.start_settlement_with_pallet(project_id).unwrap();
				self.settle_project_with_pallet(project_id, true);

				let settled_status = self.get_project_details(project_id).status;
				assert!(matches!(settled_status, ProjectStatus::SettlementFinished(_)));
			}
		}

		// Final state consistency check
		self.assert_pallet_state_consistency(project_id);
		project_id
	}

	// We assume a single bid can cover the whole first bucket. Make sure the ticket sizes allow this.
	pub fn generate_bids_from_bucket(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		bucket: BucketOf<T>, // This is the target state
		funding_asset: AcceptedFundingAsset,
	) -> Vec<BidParams<T>> {
		let mut new_bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata);
		assert_eq!(new_bucket.delta_amount, bucket.delta_amount, "Buckets must have the same delta amount");
		assert_eq!(new_bucket.delta_price, bucket.delta_price, "Buckets must have the same delta price");
		assert_eq!(new_bucket.initial_price, bucket.initial_price, "Buckets must have the same initial price");

		// Get the funding asset price once to avoid multiple mutable borrows
		let funding_asset_price = self.execute(|| {
			<PriceProviderOf<T>>::get_decimals_aware_price(&funding_asset.id(), funding_asset.decimals()).unwrap()
		});

		let mut account_counter = 0u32;
		let mut bids = Vec::new();

		// Helper function to convert CT amount to funding asset amount
		let ct_to_funding_asset = |ct_amount: Balance, ct_price: PriceOf<T>| -> Balance {
			let usd_needed = ct_price.saturating_mul_int(ct_amount);
			funding_asset_price.reciprocal().unwrap().checked_mul_int(usd_needed).unwrap()
		};

		// Loop to fill full buckets until new_bucket's price tier matches bucket's price tier
		while bucket.current_price > new_bucket.current_price {
			let ct_amount = new_bucket.amount_left;
			let ct_price = new_bucket.current_price;
			let funding_asset_amount = ct_to_funding_asset(ct_amount, ct_price);
			let account = self.account_from_u32(account_counter, "BIDDER");
			account_counter += 1;

			let bucket_bid = BidParams::<T>::from((account, Retail, funding_asset_amount, funding_asset));
			bids.push(bucket_bid);
			new_bucket.update(new_bucket.amount_left); // Consumes full amount_left, calls .next()
		}

		// Handle the final (potentially partially filled) bucket
		if new_bucket.current_price == bucket.current_price {
			if new_bucket.amount_left > bucket.amount_left {
				let amount_to_consume_in_final_tier = new_bucket.amount_left.saturating_sub(bucket.amount_left);

				// Ensure we are actually consuming a positive amount
				if amount_to_consume_in_final_tier > Balance::zero() {
					let ct_amount = amount_to_consume_in_final_tier;
					let ct_price = new_bucket.current_price;
					let funding_asset_amount = ct_to_funding_asset(ct_amount, ct_price);
					let account = self.account_from_u32(account_counter, "BIDDER");

					let partial_bid = BidParams::<T>::from((account, Retail, funding_asset_amount, funding_asset));
					bids.push(partial_bid);
					new_bucket.update(amount_to_consume_in_final_tier);
				}
			}
		}

		assert_eq!(new_bucket, bucket, "Buckets must match after generating bids");

		bids
	}

	pub fn generate_bids_from_higher_usd_than_target(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		usd_target: Balance,
	) -> Vec<BidParams<T>> {
		let mut bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata);
		bucket.update(project_metadata.total_allocation_size);

		// Increase bucket price until we go past the target usd amount
		let mut usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		while usd_raised < usd_target {
			bucket.update(bucket.delta_amount);
			usd_raised = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		}

		// Go one bucket back
		bucket.current_price = bucket.current_price.saturating_sub(bucket.delta_price);
		bucket.amount_left = bucket.delta_amount;

		// Start buying the min amount of tokens in this bucket until we reach or surpass the usd amount
		while bucket.calculate_usd_raised(project_metadata.total_allocation_size) < usd_target {
			let min_ticket = project_metadata.bidding_ticket_sizes.retail.usd_minimum_per_participation;
			let ct_min_ticket = bucket.current_price.reciprocal().unwrap().saturating_mul_int(min_ticket);
			bucket.update(ct_min_ticket);
		}

		self.generate_bids_from_bucket(project_metadata.clone(), bucket, AcceptedFundingAsset::USDT)
	}
}
