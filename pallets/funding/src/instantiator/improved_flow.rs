use super::*;
use alloc::vec::Vec;

/// Improved project creation methods that use pallet logic directly and provide
/// clear separation of concerns. These methods are composable and easier to debug.
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	/// Create a project in Application phase using pallet's do_create_project
	pub fn create_project_with_pallet(
		&mut self,
		metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
	) -> ProjectId {
		// Mint required PLMC for issuer (ED for issuer + ED for escrow account)
		self.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), self.get_ed() * 2u128)]);

		let did = if let Some(did) = maybe_did.clone() { did } else { generate_did_from_account(issuer.clone()) };

		// Use pallet's actual project creation logic
		let project_id = self.execute(|| {
			Pallet::<T>::do_create_project(&issuer, metadata.clone(), did).unwrap();
			NextProjectId::<T>::get().saturating_sub(One::one())
		});

		// Verify project was created correctly using pallet state
		self.assert_project_created_correctly(project_id, maybe_did, metadata);
		project_id
	}

	/// Start evaluation round using pallet's do_start_evaluation
	pub fn start_evaluation_with_pallet(&mut self, project_id: ProjectId) -> DispatchResultWithPostInfo {
		let issuer = self.get_issuer(project_id);
		let result = self.execute(|| Pallet::<T>::do_start_evaluation(issuer, project_id));

		if result.is_ok() {
			self.assert_project_state(project_id, ProjectStatus::EvaluationRound);
		}

		result
	}

	/// End evaluation round using pallet's do_end_evaluation
	pub fn end_evaluation_with_pallet(&mut self, project_id: ProjectId) -> DispatchResult {
		let result = self.execute(|| Pallet::<T>::do_end_evaluation(project_id));

		if result.is_ok() {
			let new_status = self.get_project_details(project_id).status;
			// Could be AuctionRound or FundingFailed depending on evaluation success
			assert!(
				matches!(new_status, ProjectStatus::AuctionRound | ProjectStatus::FundingFailed),
				"Expected AuctionRound or FundingFailed, got {:?}",
				new_status
			);
		}

		result
	}

	/// Perform evaluations using pallet's do_evaluate with exact token requirements
	pub fn perform_evaluations_with_pallet(
		&mut self,
		project_id: ProjectId,
		evaluations: Vec<EvaluationParams<T>>,
	) -> DispatchResult {
		// Calculate exact PLMC requirements (no custom logic, just direct mapping)
		let plmc_needed: Vec<UserToPLMCBalance<T>> = evaluations
			.iter()
			.map(|eval| UserToPLMCBalance::new(eval.account.clone(), eval.plmc_amount))
			.collect();

		// Ensure accounts have existential deposits
		self.mint_plmc_ed_if_required(plmc_needed.accounts());
		
		// Mint the exact PLMC needed
		self.mint_plmc_to(plmc_needed);

		// Get project policy for validation
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		// Perform evaluations using pallet's do_evaluate
		for EvaluationParams { account, plmc_amount, receiving_account } in evaluations {
			let result = self.execute(|| {
				Pallet::<T>::do_evaluate(
					&account,
					project_id,
					plmc_amount,
					generate_did_from_account(account.clone()),
					project_policy.clone(),
					receiving_account,
				)
			});

			// Stop on first error to provide clear feedback
			if let Err(e) = result {
				return Err(e);
			}
		}

		Ok(())
	}

	/// Perform bids using pallet's do_bid with exact token requirements
	pub fn perform_bids_with_pallet(
		&mut self,
		project_id: ProjectId,
		bids: Vec<BidParams<T>>,
	) -> DispatchResult {
		// Calculate exact funding requirements using pallet logic
		let (plmc_requirements, funding_asset_requirements) =
			self.get_exact_funding_requirements_for_bids(project_id, &bids);

		// Ensure accounts have existential deposits
		self.mint_plmc_ed_if_required(plmc_requirements.accounts());
		self.mint_funding_asset_ed_if_required(funding_asset_requirements.to_account_asset_map());

		// Precise minting: only mint exact amounts needed
		self.mint_precise_plmc_if_needed(plmc_requirements);
		self.mint_precise_funding_assets_if_needed(funding_asset_requirements);

		// Get project policy for validation
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		// Perform bids using pallet logic
		for bid in bids {
			let did = generate_did_from_account(bid.bidder.clone());

			// Validate bid parameters first (same validation as pallet)
			self.validate_bid_parameters(project_id, &bid, did.clone(), project_policy.clone())?;

			// Perform the bid using pallet's do_bid
			let _created_bids = self.perform_bid_with_pallet_logic(project_id, bid, did, project_policy.clone())?;
		}

		Ok(())
	}

	/// End funding/auction round using pallet's do_end_funding
	pub fn end_funding_with_pallet(&mut self, project_id: ProjectId) -> DispatchResult {
		// Process any remaining oversubscribed bids first
		self.process_oversubscribed_bids(project_id);

		let result = self.execute(|| Pallet::<T>::do_end_funding(project_id));

		if result.is_ok() {
			let new_status = self.get_project_details(project_id).status;
			assert!(
				matches!(new_status, ProjectStatus::FundingSuccessful | ProjectStatus::FundingFailed),
				"Expected FundingSuccessful or FundingFailed, got {:?}",
				new_status
			);
		}

		result
	}

	/// Start settlement using pallet's do_start_settlement
	pub fn start_settlement_with_pallet(&mut self, project_id: ProjectId) -> DispatchResult {
		let result = self.execute(|| Pallet::<T>::do_start_settlement(project_id));

		if result.is_ok() {
			let new_status = self.get_project_details(project_id).status;
			assert!(
				matches!(new_status, ProjectStatus::SettlementStarted(_)),
				"Expected SettlementStarted, got {:?}",
				new_status
			);
		}

		result
	}

	/// Settle all evaluations and bids using pallet's settlement logic
	pub fn settle_project_with_pallet(&mut self, project_id: ProjectId, mark_as_settled: bool) {
		self.execute(|| {
			// Settle evaluations using pallet logic
			Evaluations::<T>::iter_prefix((project_id,)).for_each(|((_, id), evaluation)| {
				Pallet::<T>::do_settle_evaluation(evaluation, project_id, id).unwrap()
			});

			// Settle bids using pallet logic
			Bids::<T>::iter_prefix(project_id)
				.for_each(|(_, bid)| Pallet::<T>::do_settle_bid(project_id, bid.id).unwrap());

			// Mark as settled if requested
			if mark_as_settled {
				Pallet::<T>::do_mark_project_as_settled(project_id).unwrap();
			}
		});
	}

	/// Create a complete project from Application to Settlement using pallet logic at each step
	pub fn create_complete_project_with_pallet(
		&mut self,
		metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
		settle: bool,
	) -> ProjectId {
		// Step 1: Create project
		let project_id = self.create_project_with_pallet(metadata, issuer, maybe_did);

		// Step 2: Start evaluation
		self.start_evaluation_with_pallet(project_id).unwrap();

		// Step 3: Perform evaluations
		self.perform_evaluations_with_pallet(project_id, evaluations).unwrap();

		// Step 4: End evaluation (transitions to auction or fails)
		self.end_evaluation_with_pallet(project_id).unwrap();

		// Only continue if we reached auction round
		if matches!(self.get_project_details(project_id).status, ProjectStatus::AuctionRound) {
			// Step 5: Perform bids
			self.perform_bids_with_pallet(project_id, bids).unwrap();

			// Step 6: End funding
			self.end_funding_with_pallet(project_id).unwrap();

			// Step 7: Start settlement if project was successful
			if matches!(self.get_project_details(project_id).status, ProjectStatus::FundingSuccessful) {
				self.start_settlement_with_pallet(project_id).unwrap();

				// Step 8: Settle project if requested
				if settle {
					self.settle_project_with_pallet(project_id, settle);
				}
			}
		}

		project_id
	}

	/// Assert project was created correctly by checking pallet state
	fn assert_project_created_correctly(
		&mut self,
		project_id: ProjectId,
		maybe_did: Option<Did>,
		expected_metadata: ProjectMetadataOf<T>,
	) {
		let metadata = self.get_project_metadata(project_id);
		let details = self.get_project_details(project_id);
		let issuer_did = if let Some(did) = maybe_did { 
			did 
		} else { 
			generate_did_from_account(self.get_issuer(project_id)) 
		};

		let expected_details = ProjectDetailsOf::<T> {
			issuer_account: self.get_issuer(project_id),
			issuer_did,
			is_frozen: false,
			status: ProjectStatus::Application,
			round_duration: BlockNumberPair::new(None, None),
			fundraising_target_usd: expected_metadata
				.minimum_price
				.checked_mul_int(expected_metadata.total_allocation_size)
				.unwrap(),
			remaining_contribution_tokens: expected_metadata.total_allocation_size,
			funding_amount_reached_usd: Balance::zero(),
			evaluation_round_info: EvaluationRoundInfo {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: None,
			},
			usd_bid_on_oversubscription: None,
			funding_end_block: None,
		};

		assert_eq!(metadata, expected_metadata, "Project metadata mismatch");
		assert_eq!(details, expected_details, "Project details mismatch");
	}

	/// Assert project is in expected state
	pub fn assert_project_state(&mut self, project_id: ProjectId, expected_status: ProjectStatus) {
		let actual_status = self.get_project_details(project_id).status;
		assert_eq!(
			actual_status, expected_status,
			"Project {} has status {:?} but expected {:?}",
			project_id, actual_status, expected_status
		);
	}
}