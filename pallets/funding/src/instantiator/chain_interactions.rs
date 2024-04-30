use super::*;

// general chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn new(ext: OptionalExternalities) -> Self {
		Self { ext, nonce: RefCell::new(0u64), _marker: PhantomData }
	}

	pub fn set_ext(&mut self, ext: OptionalExternalities) {
		self.ext = ext;
	}

	pub fn execute<R>(&mut self, execution: impl FnOnce() -> R) -> R {
		#[cfg(feature = "std")]
		if let Some(ext) = &self.ext {
			return ext.borrow_mut().execute_with(execution);
		}
		execution()
	}

	pub fn get_new_nonce(&self) -> u64 {
		let nonce = *self.nonce.borrow_mut();
		self.nonce.replace(nonce + 1);
		nonce
	}

	pub fn get_free_plmc_balances_for(&mut self, user_keys: Vec<AccountIdOf<T>>) -> Vec<UserToPLMCBalance<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToPLMCBalance<T>> = Vec::new();
			for account in user_keys {
				let plmc_amount = <T as Config>::NativeCurrency::balance(&account);
				balances.push(UserToPLMCBalance { account, plmc_amount });
			}
			balances.sort_by_key(|a| a.account.clone());
			balances
		})
	}

	pub fn get_reserved_plmc_balances_for(
		&mut self,
		user_keys: Vec<AccountIdOf<T>>,
		lock_type: <T as Config>::RuntimeHoldReason,
	) -> Vec<UserToPLMCBalance<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToPLMCBalance<T>> = Vec::new();
			for account in user_keys {
				let plmc_amount = <T as Config>::NativeCurrency::balance_on_hold(&lock_type, &account);
				balances.push(UserToPLMCBalance { account, plmc_amount });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_free_foreign_asset_balances_for(
		&mut self,
		asset_id: AssetIdOf<T>,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<UserToForeignAssets<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToForeignAssets<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				balances.push(UserToForeignAssets { account, asset_amount, asset_id });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_ct_asset_balances_for(
		&mut self,
		project_id: ProjectId,
		user_keys: Vec<AccountIdOf<T>>,
	) -> Vec<BalanceOf<T>> {
		self.execute(|| {
			let mut balances: Vec<BalanceOf<T>> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::ContributionTokenCurrency::balance(project_id, &account);
				balances.push(asset_amount);
			}
			balances
		})
	}

	pub fn get_all_free_plmc_balances(&mut self) -> Vec<UserToPLMCBalance<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_free_plmc_balances_for(user_keys)
	}

	pub fn get_all_reserved_plmc_balances(
		&mut self,
		reserve_type: <T as Config>::RuntimeHoldReason,
	) -> Vec<UserToPLMCBalance<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_reserved_plmc_balances_for(user_keys, reserve_type)
	}

	pub fn get_all_free_foreign_asset_balances(&mut self, asset_id: AssetIdOf<T>) -> Vec<UserToForeignAssets<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().collect());
		self.get_free_foreign_asset_balances_for(asset_id, user_keys)
	}

	pub fn get_plmc_total_supply(&mut self) -> BalanceOf<T> {
		self.execute(<T as Config>::NativeCurrency::total_issuance)
	}

	pub fn do_reserved_plmc_assertions(
		&mut self,
		correct_funds: Vec<UserToPLMCBalance<T>>,
		reserve_type: <T as Config>::RuntimeHoldReason,
	) {
		for UserToPLMCBalance { account, plmc_amount } in correct_funds {
			self.execute(|| {
				let reserved = <T as Config>::NativeCurrency::balance_on_hold(&reserve_type, &account);
				assert_eq!(reserved, plmc_amount, "account has unexpected reserved plmc balance");
			});
		}
	}

	pub fn mint_plmc_to(&mut self, mapping: Vec<UserToPLMCBalance<T>>) {
		self.execute(|| {
			for UserToPLMCBalance { account, plmc_amount } in mapping {
				<T as Config>::NativeCurrency::mint_into(&account, plmc_amount).expect("Minting should work");
			}
		});
	}

	pub fn mint_foreign_asset_to(&mut self, mapping: Vec<UserToForeignAssets<T>>) {
		self.execute(|| {
			for UserToForeignAssets { account, asset_amount, asset_id } in mapping {
				<T as Config>::FundingCurrency::mint_into(asset_id, &account, asset_amount)
					.expect("Minting should work");
			}
		});
	}

	pub fn current_block(&mut self) -> BlockNumberFor<T> {
		self.execute(|| frame_system::Pallet::<T>::block_number())
	}

	pub fn advance_time(&mut self, amount: BlockNumberFor<T>) -> Result<(), DispatchError> {
		self.execute(|| {
			for _block in 0u32..amount.saturated_into() {
				let mut current_block = frame_system::Pallet::<T>::block_number();

				<AllPalletsWithoutSystem as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);
				<frame_system::Pallet<T> as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);

				<AllPalletsWithoutSystem as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);
				<frame_system::Pallet<T> as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);

				current_block += One::one();
				frame_system::Pallet::<T>::set_block_number(current_block);

				<frame_system::Pallet<T> as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
				<AllPalletsWithoutSystem as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
			}
			Ok(())
		})
	}

	pub fn do_free_plmc_assertions(&mut self, correct_funds: Vec<UserToPLMCBalance<T>>) {
		for UserToPLMCBalance { account, plmc_amount } in correct_funds {
			self.execute(|| {
				let free = <T as Config>::NativeCurrency::balance(&account);
				assert_eq!(free, plmc_amount, "account has unexpected free plmc balance");
			});
		}
	}

	pub fn do_free_foreign_asset_assertions(&mut self, correct_funds: Vec<UserToForeignAssets<T>>) {
		for UserToForeignAssets { account, asset_amount, asset_id } in correct_funds {
			self.execute(|| {
				let real_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				assert_eq!(asset_amount, real_amount, "Wrong foreign asset balance expected for user {:?}", account);
			});
		}
	}

	pub fn do_bid_transferred_foreign_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToForeignAssets<T>>,
		project_id: ProjectId,
	) {
		for UserToForeignAssets { account, asset_amount, .. } in correct_funds {
			self.execute(|| {
				// total amount of contributions for this user for this project stored in the mapping
				let contribution_total: <T as Config>::Balance =
					Bids::<T>::iter_prefix_values((project_id, account.clone()))
						.map(|c| c.funding_asset_amount_locked)
						.fold(Zero::zero(), |a, b| a + b);
				assert_eq!(
					contribution_total, asset_amount,
					"Wrong funding balance expected for stored auction info on user {:?}",
					account
				);
			});
		}
	}

	// Check if a Contribution storage item exists for the given funding asset transfer
	pub fn do_contribution_transferred_foreign_asset_assertions(
		&mut self,
		correct_funds: Vec<UserToForeignAssets<T>>,
		project_id: ProjectId,
	) {
		for UserToForeignAssets { account, asset_amount, .. } in correct_funds {
			self.execute(|| {
				Contributions::<T>::iter_prefix_values((project_id, account.clone()))
					.find(|c| c.funding_asset_amount == asset_amount)
					.expect("Contribution not found in storage");
			});
		}
	}
}

// assertions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn test_ct_created_for(&mut self, project_id: ProjectId) {
		self.execute(|| {
			let metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
			assert_eq!(
				<T as Config>::ContributionTokenCurrency::name(project_id),
				metadata.token_information.name.to_vec()
			);
			let escrow_account = Pallet::<T>::fund_account_id(project_id);

			assert_eq!(<T as Config>::ContributionTokenCurrency::admin(project_id).unwrap(), escrow_account);
		});
	}

	pub fn test_ct_not_created_for(&mut self, project_id: ProjectId) {
		self.execute(|| {
			assert!(
				!<T as Config>::ContributionTokenCurrency::asset_exists(project_id),
				"Asset shouldn't exist, since funding failed"
			);
		});
	}

	pub fn creation_assertions(
		&mut self,
		project_id: ProjectId,
		expected_metadata: ProjectMetadataOf<T>,
		creation_start_block: BlockNumberFor<T>,
	) {
		let metadata = self.get_project_metadata(project_id);
		let details = self.get_project_details(project_id);
		let expected_details = ProjectDetailsOf::<T> {
			issuer_account: self.get_issuer(project_id),
			issuer_did: generate_did_from_account(self.get_issuer(project_id)),
			is_frozen: false,
			weighted_average_price: None,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair { start: Some(creation_start_block), end: None },
				..Default::default()
			},
			fundraising_target_usd: expected_metadata
				.minimum_price
				.checked_mul_int(expected_metadata.total_allocation_size)
				.unwrap(),
			remaining_contribution_tokens: expected_metadata.total_allocation_size,
			funding_amount_reached_usd: BalanceOf::<T>::zero(),
			evaluation_round_info: EvaluationRoundInfoOf::<T> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
			parachain_id: None,
			migration_readiness_check: None,
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: crate::ChannelStatus::Closed,
				polimec_to_project: crate::ChannelStatus::Closed,
			},
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	pub fn evaluation_assertions(
		&mut self,
		project_id: ProjectId,
		expected_free_plmc_balances: Vec<UserToPLMCBalance<T>>,
		expected_reserved_plmc_balances: Vec<UserToPLMCBalance<T>>,
		total_plmc_supply: BalanceOf<T>,
	) {
		// just in case we forgot to merge accounts:
		let expected_free_plmc_balances =
			Self::generic_map_operation(vec![expected_free_plmc_balances], MergeOperation::Add);
		let expected_reserved_plmc_balances =
			Self::generic_map_operation(vec![expected_reserved_plmc_balances], MergeOperation::Add);

		let project_details = self.get_project_details(project_id);

		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		assert_eq!(self.get_plmc_total_supply(), total_plmc_supply);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_reserved_plmc_assertions(expected_reserved_plmc_balances, HoldReason::Evaluation(project_id).into());
	}

	pub fn finalized_bids_assertions(
		&mut self,
		project_id: ProjectId,
		bid_expectations: Vec<BidInfoFilter<T>>,
		expected_ct_sold: BalanceOf<T>,
	) {
		let project_metadata = self.get_project_metadata(project_id);
		let project_details = self.get_project_details(project_id);
		let project_bids = self.execute(|| Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
		assert!(project_details.weighted_average_price.is_some(), "Weighted average price should exist");

		for filter in bid_expectations {
			let _found_bid = project_bids.iter().find(|bid| filter.matches_bid(bid)).unwrap();
		}

		// Remaining CTs are updated
		assert_eq!(
			project_details.remaining_contribution_tokens,
			project_metadata.total_allocation_size - expected_ct_sold,
			"Remaining CTs are incorrect"
		);
	}
}

// project chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn get_issuer(&mut self, project_id: ProjectId) -> AccountIdOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).unwrap().issuer_account)
	}

	pub fn get_project_metadata(&mut self, project_id: ProjectId) -> ProjectMetadataOf<T> {
		self.execute(|| ProjectsMetadata::<T>::get(project_id).expect("Project metadata exists"))
	}

	pub fn get_project_details(&mut self, project_id: ProjectId) -> ProjectDetailsOf<T> {
		self.execute(|| ProjectsDetails::<T>::get(project_id).expect("Project details exists"))
	}

	pub fn get_update_block(&mut self, project_id: ProjectId, update_type: &UpdateType) -> Option<BlockNumberFor<T>> {
		self.execute(|| {
			ProjectsToUpdate::<T>::iter().find_map(|(block, update_tup)| {
				if project_id == update_tup.0 && update_type == &update_tup.1 {
					Some(block)
				} else {
					None
				}
			})
		})
	}

	pub fn create_new_project(&mut self, project_metadata: ProjectMetadataOf<T>, issuer: AccountIdOf<T>) -> ProjectId {
		let now = self.current_block();
		// one ED for the issuer, one ED for the escrow account
		self.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), Self::get_ed() * 2u64.into())]);

		self.execute(|| {
			crate::Pallet::<T>::do_create_project(
				&issuer,
				project_metadata.clone(),
				generate_did_from_account(issuer.clone()),
			)
			.unwrap();
			let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
			log::trace!("Last project metadata: {:?}", last_project_metadata);
		});

		let created_project_id = self.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
		self.creation_assertions(created_project_id, project_metadata, now);
		created_project_id
	}

	pub fn start_evaluation(&mut self, project_id: ProjectId, caller: AccountIdOf<T>) -> Result<(), DispatchError> {
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::Application);
		self.execute(|| crate::Pallet::<T>::do_start_evaluation(caller, project_id).unwrap());
		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::EvaluationRound);

		Ok(())
	}

	pub fn create_evaluating_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
	) -> ProjectId {
		let project_id = self.create_new_project(project_metadata, issuer.clone());
		self.start_evaluation(project_id, issuer).unwrap();
		project_id
	}

	pub fn evaluate_for_users(
		&mut self,
		project_id: ProjectId,
		bonds: Vec<UserToUSDBalance<T>>,
	) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
		for UserToUSDBalance { account, usd_amount } in bonds {
			self.execute(|| {
				crate::Pallet::<T>::do_evaluate(
					&account.clone(),
					project_id,
					usd_amount,
					generate_did_from_account(account),
					InvestorType::Professional,
					project_policy.clone(),
				)
			})?;
		}
		Ok(().into())
	}

	pub fn start_auction(&mut self, project_id: ProjectId, caller: AccountIdOf<T>) -> Result<(), DispatchError> {
		let project_details = self.get_project_details(project_id);

		if project_details.status == ProjectStatus::EvaluationRound {
			let evaluation_end = project_details.phase_transition_points.evaluation.end().unwrap();
			let auction_start = evaluation_end.saturating_add(2u32.into());
			let blocks_to_start = auction_start.saturating_sub(self.current_block());
			self.advance_time(blocks_to_start + 1u32.into()).unwrap();
		};

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		self.execute(|| crate::Pallet::<T>::do_auction_opening(caller, project_id).unwrap());

		assert_eq!(self.get_project_details(project_id).status, ProjectStatus::AuctionOpening);

		Ok(())
	}

	pub fn create_auctioning_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
	) -> ProjectId {
		let project_id = self.create_evaluating_project(project_metadata, issuer.clone());

		let evaluators = evaluations.accounts();
		let prev_supply = self.get_plmc_total_supply();
		let prev_plmc_balances = self.get_free_plmc_balances_for(evaluators.clone());

		let plmc_eval_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = evaluators.existential_deposits();

		let expected_remaining_plmc: Vec<UserToPLMCBalance<T>> = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		self.mint_plmc_to(plmc_eval_deposits.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());

		self.evaluate_for_users(project_id, evaluations).unwrap();

		let expected_evaluator_balances =
			Self::sum_balance_mappings(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

		let expected_total_supply = prev_supply + expected_evaluator_balances;

		self.evaluation_assertions(project_id, expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

		self.start_auction(project_id, issuer).unwrap();
		project_id
	}

	pub fn bid_for_users(&mut self, project_id: ProjectId, bids: Vec<BidParams<T>>) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		for bid in bids {
			self.execute(|| {
				let did = generate_did_from_account(bid.bidder.clone());
				crate::Pallet::<T>::do_bid(
					&bid.bidder,
					project_id,
					bid.amount,
					bid.multiplier,
					bid.asset,
					did,
					InvestorType::Institutional,
					project_policy.clone(),
				)
			})?;
		}
		Ok(().into())
	}

	pub fn start_community_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		let opening_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.auction_opening
			.end()
			.expect("Auction Opening end point should exist");

		self.execute(|| frame_system::Pallet::<T>::set_block_number(opening_end));
		// run on_initialize
		self.advance_time(2u32.into()).unwrap();

		let closing_end = self
			.get_project_details(project_id)
			.phase_transition_points
			.auction_closing
			.end()
			.expect("closing end point should exist");

		self.execute(|| frame_system::Pallet::<T>::set_block_number(closing_end));
		// run on_initialize
		self.advance_time(1u32.into()).unwrap();

		ensure!(
			self.get_project_details(project_id).status == ProjectStatus::CommunityRound,
			DispatchError::from("Auction failed")
		);

		Ok(())
	}

	pub fn create_community_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectId {
		if bids.is_empty() {
			panic!("Cannot start community funding without bids")
		}

		let project_id = self.create_auctioning_project(project_metadata.clone(), issuer, evaluations.clone());
		let bidders = bids.accounts();
		let asset_id = bids[0].asset.to_assethub_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(bidders.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, bidders.clone());
		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> = Self::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> =
			Self::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
		let participation_usable_evaluation_deposits = plmc_evaluation_deposits
			.into_iter()
			.map(|mut x| {
				x.plmc_amount = x.plmc_amount.saturating_sub(<T as Config>::EvaluatorSlash::get() * x.plmc_amount);
				x
			})
			.collect::<Vec<UserToPLMCBalance<T>>>();
		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_bid_deposits.clone(), participation_usable_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked = plmc_bid_deposits;
		let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bidders.existential_deposits();
		let funding_asset_deposits = Self::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);

		let bidder_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + bidder_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.bid_for_users(project_id, bids.clone()).unwrap();

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		self.do_bid_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.start_community_funding(project_id).unwrap();

		project_id
	}

	pub fn contribute_for_users(
		&mut self,
		project_id: ProjectId,
		contributions: Vec<ContributionParams<T>>,
	) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		match self.get_project_details(project_id).status {
			ProjectStatus::CommunityRound =>
				for cont in contributions {
					let did = generate_did_from_account(cont.contributor.clone());
					let investor_type = InvestorType::Retail;
					self.execute(|| {
						crate::Pallet::<T>::do_community_contribute(
							&cont.contributor,
							project_id,
							cont.amount,
							cont.multiplier,
							cont.asset,
							did,
							investor_type,
							project_policy.clone(),
						)
					})?;
				},
			ProjectStatus::RemainderRound =>
				for cont in contributions {
					let did = generate_did_from_account(cont.contributor.clone());
					let investor_type = InvestorType::Professional;
					self.execute(|| {
						crate::Pallet::<T>::do_remaining_contribute(
							&cont.contributor,
							project_id,
							cont.amount,
							cont.multiplier,
							cont.asset,
							did,
							investor_type,
							project_policy.clone(),
						)
					})?;
				},
			_ => panic!("Project should be in Community or Remainder status"),
		}

		Ok(().into())
	}

	pub fn start_remainder_or_end_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		let details = self.get_project_details(project_id);
		assert_eq!(details.status, ProjectStatus::CommunityRound);
		let remaining_tokens = details.remaining_contribution_tokens;
		let update_type =
			if remaining_tokens > Zero::zero() { UpdateType::RemainderFundingStart } else { UpdateType::FundingEnd };
		if let Some(transition_block) = self.get_update_block(project_id, &update_type) {
			self.execute(|| frame_system::Pallet::<T>::set_block_number(transition_block - One::one()));
			self.advance_time(1u32.into()).unwrap();
			match self.get_project_details(project_id).status {
				ProjectStatus::RemainderRound | ProjectStatus::FundingSuccessful => Ok(()),
				_ => panic!("Bad state"),
			}
		} else {
			panic!("Bad state")
		}
	}

	pub fn finish_funding(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		if let Some(update_block) = self.get_update_block(project_id, &UpdateType::RemainderFundingStart) {
			self.execute(|| frame_system::Pallet::<T>::set_block_number(update_block - One::one()));
			self.advance_time(1u32.into()).unwrap();
		}
		let update_block =
			self.get_update_block(project_id, &UpdateType::FundingEnd).expect("Funding end block should exist");
		self.execute(|| frame_system::Pallet::<T>::set_block_number(update_block - One::one()));
		self.advance_time(1u32.into()).unwrap();
		let project_details = self.get_project_details(project_id);
		assert!(
			matches!(
				project_details.status,
				ProjectStatus::FundingSuccessful |
					ProjectStatus::FundingFailed |
					ProjectStatus::AwaitingProjectDecision
			),
			"Project should be in Finished status"
		);
		Ok(())
	}

	pub fn settle_project(&mut self, project_id: ProjectId) -> Result<(), DispatchError> {
		let details = self.get_project_details(project_id);
		self.execute(|| match details.status {
			ProjectStatus::FundingSuccessful => Self::settle_successful_project(project_id),
			ProjectStatus::FundingFailed => Self::settle_failed_project(project_id),
			_ => panic!("Project should be in FundingSuccessful or FundingFailed status"),
		})
	}

	fn settle_successful_project(project_id: ProjectId) -> Result<(), DispatchError> {
		Evaluations::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, evaluation)| Pallet::<T>::do_settle_successful_evaluation(evaluation, project_id))?;

		Bids::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, bid)| Pallet::<T>::do_settle_successful_bid(bid, project_id))?;

		Contributions::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, contribution)| Pallet::<T>::do_settle_successful_contribution(contribution, project_id))
	}

	fn settle_failed_project(project_id: ProjectId) -> Result<(), DispatchError> {
		Evaluations::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, evaluation)| Pallet::<T>::do_settle_failed_evaluation(evaluation, project_id))?;

		Bids::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, bid)| Pallet::<T>::do_settle_failed_bid(bid, project_id))?;

		Contributions::<T>::iter_prefix((project_id,))
			.try_for_each(|(_, contribution)| Pallet::<T>::do_settle_failed_contribution(contribution, project_id))?;

		Ok(())
	}

	pub fn get_evaluations(&mut self, project_id: ProjectId) -> Vec<EvaluationInfoOf<T>> {
		self.execute(|| Evaluations::<T>::iter_prefix_values((project_id,)).collect())
	}

	pub fn get_bids(&mut self, project_id: ProjectId) -> Vec<BidInfoOf<T>> {
		self.execute(|| Bids::<T>::iter_prefix_values((project_id,)).collect())
	}

	pub fn get_contributions(&mut self, project_id: ProjectId) -> Vec<ContributionInfoOf<T>> {
		self.execute(|| Contributions::<T>::iter_prefix_values((project_id,)).collect())
	}

	// Used to check if all evaluations are settled correctly. We cannot check amount of
	// contributions minted for the user, as they could have received more tokens from other participations.
	pub fn assert_evaluations_migrations_created(
		&mut self,
		project_id: ProjectId,
		evaluations: Vec<EvaluationInfoOf<T>>,
		percentage: u64,
	) {
		let details = self.get_project_details(project_id);
		assert!(matches!(details.status, ProjectStatus::FundingSuccessful | ProjectStatus::FundingFailed));

		self.execute(|| {
			for evaluation in evaluations {
				let reward_info =
					ProjectsDetails::<T>::get(project_id).unwrap().evaluation_round_info.evaluators_outcome;
				let account = evaluation.evaluator.clone();
				assert_eq!(Evaluations::<T>::iter_prefix_values((&project_id, &account)).count(), 0);

				let (amount, should_exist) = match percentage {
					0..=75 => {
						assert!(matches!(reward_info, EvaluatorsOutcome::Slashed));
						(0u64.into(), false)
					},
					76..=89 => {
						assert!(matches!(reward_info, EvaluatorsOutcome::Unchanged));
						(0u64.into(), false)
					},
					90..=100 => {
						let reward = match reward_info {
							EvaluatorsOutcome::Rewarded(info) =>
								Pallet::<T>::calculate_evaluator_reward(&evaluation, &info),
							_ => panic!("Evaluators should be rewarded"),
						};
						(reward, true)
					},
					_ => panic!("Percentage should be between 0 and 100"),
				};
				Self::assert_migration(
					project_id,
					account,
					amount,
					evaluation.id,
					ParticipationType::Evaluation,
					should_exist,
				);
			}
		});
	}

	// Testing if a list of bids are settled correctly.
	pub fn assert_bids_migrations_created(
		&mut self,
		project_id: ProjectId,
		bids: Vec<BidInfoOf<T>>,
		is_successful: bool,
	) {
		self.execute(|| {
			for bid in bids {
				let account = bid.bidder.clone();
				assert_eq!(Bids::<T>::iter_prefix_values((&project_id, &account)).count(), 0);
				let amount: BalanceOf<T> = if is_successful { bid.final_ct_amount } else { 0u64.into() };
				Self::assert_migration(project_id, account, amount, bid.id, ParticipationType::Bid, is_successful);
			}
		});
	}

	// Testing if a list of contributions are settled correctly.
	pub fn assert_contributions_migrations_created(
		&mut self,
		project_id: ProjectId,
		contributions: Vec<ContributionInfoOf<T>>,
		is_successful: bool,
	) {
		self.execute(|| {
			for contribution in contributions {
				let account = contribution.contributor.clone();
				assert_eq!(Bids::<T>::iter_prefix_values((&project_id, &account)).count(), 0);
				let amount: BalanceOf<T> = if is_successful { contribution.ct_amount } else { 0u64.into() };
				Self::assert_migration(
					project_id,
					account,
					amount,
					contribution.id,
					ParticipationType::Contribution,
					is_successful,
				);
			}
		});
	}

	fn assert_migration(
		project_id: ProjectId,
		account: AccountIdOf<T>,
		amount: BalanceOf<T>,
		id: u32,
		participation_type: ParticipationType,
		should_exist: bool,
	) {
		let correct = match (should_exist, UserMigrations::<T>::get(project_id, account.clone())) {
			// User has migrations, so we need to check if any matches our criteria
			(_, Some((_, migrations))) => {
				let maybe_migration = migrations.into_iter().find(|migration| {
                    let user = T::AccountId32Conversion::convert(account.clone());
                    matches!(migration.origin, MigrationOrigin { user: m_user, id: m_id, participation_type: m_participation_type } if m_user == user && m_id == id && m_participation_type == participation_type)
                });
				match maybe_migration {
					// Migration exists so we check if the amount is correct and if it should exist
					Some(migration) => migration.info.contribution_token_amount == amount.into() && should_exist,
					// Migration doesn't exist so we check if it should not exist
					None => !should_exist,
				}
			},
			// User does not have any migrations, so the migration should not exist
			(false, None) => true,
			(true, None) => false,
		};
		assert!(correct);
	}

	pub fn create_remainder_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		let project_id = self.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
		);

		if contributions.is_empty() {
			self.start_remainder_or_end_funding(project_id).unwrap();
			return project_id;
		}

		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();

		let contributors = contributions.accounts();

		let asset_id = contributions[0].asset.to_assethub_id();

		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_post_wap(&bids, project_metadata.clone(), ct_price);
		let plmc_contribution_deposits = Self::calculate_contributed_plmc_spent(contributions.clone(), ct_price);

		let reducible_evaluator_balances = Self::slash_evaluator_balances(plmc_evaluation_deposits.clone());
		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_contribution_deposits.clone(), reducible_evaluator_balances],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked =
			Self::generic_map_operation(vec![plmc_bid_deposits, plmc_contribution_deposits], MergeOperation::Add);
		let plmc_existential_deposits = contributors.existential_deposits();

		let funding_asset_deposits = Self::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);
		let contributor_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, contributions).expect("Contributing should work");

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);

		self.do_contribution_transferred_foreign_asset_assertions(funding_asset_deposits, project_id);

		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.start_remainder_or_end_funding(project_id).unwrap();

		project_id
	}

	pub fn create_finished_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		let project_id = self.create_remainder_contributing_project(
			project_metadata.clone(),
			issuer,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
		);

		match self.get_project_details(project_id).status {
			ProjectStatus::FundingSuccessful => return project_id,
			ProjectStatus::RemainderRound if remainder_contributions.is_empty() => {
				self.finish_funding(project_id).unwrap();
				return project_id;
			},
			_ => {},
		};

		let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();
		let contributors = remainder_contributions.accounts();
		let asset_id = remainder_contributions[0].asset.to_assethub_id();
		let prev_plmc_balances = self.get_free_plmc_balances_for(contributors.clone());
		let prev_funding_asset_balances = self.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

		let plmc_evaluation_deposits = Self::calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits = Self::calculate_auction_plmc_spent_post_wap(&bids, project_metadata.clone(), ct_price);
		let plmc_community_contribution_deposits =
			Self::calculate_contributed_plmc_spent(community_contributions.clone(), ct_price);
		let plmc_remainder_contribution_deposits =
			Self::calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);

		let necessary_plmc_mint = Self::generic_map_operation(
			vec![plmc_remainder_contribution_deposits.clone(), plmc_evaluation_deposits],
			MergeOperation::Subtract,
		);
		let total_plmc_participation_locked = Self::generic_map_operation(
			vec![plmc_bid_deposits, plmc_community_contribution_deposits, plmc_remainder_contribution_deposits],
			MergeOperation::Add,
		);
		let plmc_existential_deposits = contributors.existential_deposits();
		let funding_asset_deposits =
			Self::calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

		let contributor_balances =
			Self::sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

		let expected_free_plmc_balances = Self::generic_map_operation(
			vec![prev_plmc_balances, plmc_existential_deposits.clone()],
			MergeOperation::Add,
		);

		let prev_supply = self.get_plmc_total_supply();
		let post_supply = prev_supply + contributor_balances;

		self.mint_plmc_to(necessary_plmc_mint.clone());
		self.mint_plmc_to(plmc_existential_deposits.clone());
		self.mint_foreign_asset_to(funding_asset_deposits.clone());

		self.contribute_for_users(project_id, remainder_contributions.clone())
			.expect("Remainder Contributing should work");

		self.do_reserved_plmc_assertions(
			total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
			HoldReason::Participation(project_id).into(),
		);
		self.do_contribution_transferred_foreign_asset_assertions(
			funding_asset_deposits.merge_accounts(MergeOperation::Add),
			project_id,
		);
		self.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
		self.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
		assert_eq!(self.get_plmc_total_supply(), post_supply);

		self.finish_funding(project_id).unwrap();

		if self.get_project_details(project_id).status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let project_details = self.get_project_details(project_id);
			// if our bids were creating an oversubscription, then just take the total allocation size
			let auction_bought_tokens = bids
				.iter()
				.map(|bid| bid.amount)
				.fold(Zero::zero(), |acc, item| item + acc)
				.min(project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size);
			let community_bought_tokens =
				community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
			let remainder_bought_tokens =
				remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size -
					auction_bought_tokens -
					community_bought_tokens -
					remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
		}

		project_id
	}

	pub fn create_project_at(
		&mut self,
		status: ProjectStatus,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<UserToUSDBalance<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		match status {
			ProjectStatus::FundingSuccessful => self.create_finished_project(
				project_metadata,
				issuer,
				evaluations,
				bids,
				community_contributions,
				remainder_contributions,
			),
			ProjectStatus::RemainderRound => self.create_remainder_contributing_project(
				project_metadata,
				issuer,
				evaluations,
				bids,
				community_contributions,
			),
			ProjectStatus::CommunityRound =>
				self.create_community_contributing_project(project_metadata, issuer, evaluations, bids),
			ProjectStatus::AuctionOpening => self.create_auctioning_project(project_metadata, issuer, evaluations),
			ProjectStatus::EvaluationRound => self.create_evaluating_project(project_metadata, issuer),
			ProjectStatus::Application => self.create_new_project(project_metadata, issuer),
			_ => panic!("unsupported project creation in that status"),
		}
	}
}
