#![allow(clippy::too_many_arguments)]

#[allow(clippy::wildcard_imports)]
use super::*;

// general chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = Balance>,
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

	pub fn get_free_plmc_balance_for(&mut self, user: AccountIdOf<T>) -> Balance {
		self.execute(|| <T as Config>::NativeCurrency::balance(&user))
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

	pub fn get_reserved_plmc_balance_for(
		&mut self,
		user: AccountIdOf<T>,
		lock_type: <T as Config>::RuntimeHoldReason,
	) -> Balance {
		self.execute(|| <T as Config>::NativeCurrency::balance_on_hold(&lock_type, &user))
	}

	pub fn get_free_funding_asset_balances_for(
		&mut self,
		list: Vec<(AccountIdOf<T>, AssetIdOf<T>)>,
	) -> Vec<UserToFundingAsset<T>> {
		self.execute(|| {
			let mut balances: Vec<UserToFundingAsset<T>> = Vec::new();
			for (account, asset_id) in list {
				let asset_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				balances.push(UserToFundingAsset { account, asset_amount, asset_id });
			}
			balances.sort_by(|a, b| a.account.cmp(&b.account));
			balances
		})
	}

	pub fn get_free_funding_asset_balance_for(&mut self, asset_id: AssetIdOf<T>, user: AccountIdOf<T>) -> Balance {
		self.execute(|| <T as Config>::FundingCurrency::balance(asset_id, &user))
	}

	pub fn get_ct_asset_balances_for(&mut self, project_id: ProjectId, user_keys: Vec<AccountIdOf<T>>) -> Vec<Balance> {
		self.execute(|| {
			let mut balances: Vec<Balance> = Vec::new();
			for account in user_keys {
				let asset_amount = <T as Config>::ContributionTokenCurrency::balance(project_id, &account);
				balances.push(asset_amount);
			}
			balances
		})
	}

	pub fn get_ct_asset_balance_for(&mut self, project_id: ProjectId, user: AccountIdOf<T>) -> Balance {
		self.execute(|| <T as Config>::ContributionTokenCurrency::balance(project_id, &user))
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

	pub fn get_all_free_funding_asset_balances(&mut self, asset_id: AssetIdOf<T>) -> Vec<UserToFundingAsset<T>> {
		let user_keys = self.execute(|| frame_system::Account::<T>::iter_keys().map(|a| (a, asset_id)).collect());
		self.get_free_funding_asset_balances_for(user_keys)
	}

	pub fn get_plmc_total_supply(&mut self) -> Balance {
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
				assert_eq!(reserved, plmc_amount, "account {:?} has unexpected reserved plmc balance", account);
			});
		}
	}

	pub fn mint_plmc_to(&mut self, mapping: Vec<UserToPLMCBalance<T>>) {
		self.execute(|| {
			for UserToPLMCBalance { account, plmc_amount } in mapping {
				if plmc_amount > Zero::zero() {
					<T as Config>::NativeCurrency::mint_into(&account, plmc_amount).expect("Minting should work");
				}
			}
		});
	}

	pub fn mint_funding_asset_to(&mut self, mapping: Vec<UserToFundingAsset<T>>) {
		self.execute(|| {
			for UserToFundingAsset { account, asset_amount, asset_id } in mapping {
				<T as Config>::FundingCurrency::mint_into(asset_id, &account, asset_amount)
					.expect("Minting should work");
			}
		});
	}

	pub fn current_block(&mut self) -> BlockNumberFor<T> {
		self.execute(|| frame_system::Pallet::<T>::block_number())
	}

	pub fn advance_time(&mut self, amount: BlockNumberFor<T>) {
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
		})
	}

	pub fn jump_to_block(&mut self, block: BlockNumberFor<T>) {
		let current_block = self.current_block();
		if block > current_block {
			self.execute(|| frame_system::Pallet::<T>::set_block_number(block - One::one()));
			self.advance_time(One::one());
		} else {
			// panic!("Cannot jump to a block in the present or past")
		}
	}

	pub fn do_free_plmc_assertions(&mut self, correct_funds: Vec<UserToPLMCBalance<T>>) {
		for UserToPLMCBalance { account, plmc_amount } in correct_funds {
			self.execute(|| {
				let free = <T as Config>::NativeCurrency::balance(&account);
				assert_eq!(free, plmc_amount, "account has unexpected free plmc balance");
			});
		}
	}

	pub fn do_free_funding_asset_assertions(&mut self, correct_funds: Vec<UserToFundingAsset<T>>) {
		for UserToFundingAsset { account, asset_amount: expected_amount, asset_id } in correct_funds {
			self.execute(|| {
				let real_amount = <T as Config>::FundingCurrency::balance(asset_id, &account);
				assert_eq!(real_amount, expected_amount, "Wrong funding asset balance expected for user {:?}", account);
			});
		}
	}

	pub fn mint_plmc_ed_if_required(&mut self, accounts: Vec<AccountIdOf<T>>) {
		let ed = self.get_ed();
		for account in accounts {
			self.execute(|| {
				if <T as Config>::NativeCurrency::balance(&account) < ed {
					<T as Config>::NativeCurrency::mint_into(&account, ed).expect("Minting should work");
				}
			});
		}
	}

	pub fn mint_funding_asset_ed_if_required(&mut self, list: Vec<(AccountIdOf<T>, AssetIdOf<T>)>) {
		for (account, asset_id) in list {
			let ed = self.get_funding_asset_ed(asset_id);
			self.execute(|| {
				if <T as Config>::FundingCurrency::balance(asset_id, &account) < ed {
					<T as Config>::FundingCurrency::mint_into(asset_id, &account, ed).expect("Minting should work");
				}
			});
		}
	}
}

// assertions
impl<
		T: Config + pallet_balances::Config<Balance = Balance>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn test_ct_created_for(&mut self, project_id: ProjectId) {
		self.execute(|| {
			let metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
			assert!(
				<T as Config>::ContributionTokenCurrency::asset_exists(project_id),
				"Asset should exist, since funding was successful"
			);
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
		maybe_did: Option<Did>,
		expected_metadata: ProjectMetadataOf<T>,
	) {
		let metadata = self.get_project_metadata(project_id);
		let details = self.get_project_details(project_id);
		let issuer_did =
			if let Some(did) = maybe_did { did } else { generate_did_from_account(self.get_issuer(project_id)) };
		let expected_details = ProjectDetailsOf::<T> {
			issuer_account: self.get_issuer(project_id),
			issuer_did,
			is_frozen: false,
			weighted_average_price: None,
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
			migration_type: None,
		};
		assert_eq!(metadata, expected_metadata);
		assert_eq!(details, expected_details);
	}

	pub fn evaluation_assertions(
		&mut self,
		project_id: ProjectId,
		expected_free_plmc_balances: Vec<UserToPLMCBalance<T>>,
		expected_held_plmc_balances: Vec<UserToPLMCBalance<T>>,
		expected_total_plmc_supply: Balance,
	) {
		let project_details = self.get_project_details(project_id);

		// just in case we forgot to merge accounts:
		let expected_free_plmc_balances = expected_free_plmc_balances.merge_accounts(MergeOperation::Add);
		let expected_reserved_plmc_balances = expected_held_plmc_balances.merge_accounts(MergeOperation::Add);

		assert_eq!(project_details.status, ProjectStatus::EvaluationRound);
		assert_eq!(self.get_plmc_total_supply(), expected_total_plmc_supply);
		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_reserved_plmc_assertions(expected_reserved_plmc_balances, HoldReason::Evaluation.into());
	}

	pub fn finalized_bids_assertions(
		&mut self,
		project_id: ProjectId,
		bid_expectations: Vec<BidInfoFilter<T>>,
		expected_ct_sold: Balance,
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

	pub fn assert_plmc_free_balance(&mut self, account_id: AccountIdOf<T>, expected_balance: Balance) {
		let real_balance = self.get_free_plmc_balance_for(account_id.clone());
		assert_eq!(real_balance, expected_balance, "Unexpected PLMC balance for user {:?}", account_id);
	}

	pub fn assert_plmc_held_balance(
		&mut self,
		account_id: AccountIdOf<T>,
		expected_balance: Balance,
		hold_reason: <T as Config>::RuntimeHoldReason,
	) {
		let real_balance = self.get_reserved_plmc_balance_for(account_id.clone(), hold_reason);
		assert_eq!(real_balance, expected_balance, "Unexpected PLMC balance for user {:?}", account_id);
	}

	pub fn assert_funding_asset_free_balance(
		&mut self,
		account_id: AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		expected_balance: Balance,
	) {
		let real_balance = self.get_free_funding_asset_balance_for(asset_id, account_id.clone());
		assert_eq!(real_balance, expected_balance, "Unexpected funding asset balance for user {:?}", account_id);
	}

	pub fn assert_ct_balance(&mut self, project_id: ProjectId, account_id: AccountIdOf<T>, expected_balance: Balance) {
		let real_balance = self.get_ct_asset_balance_for(project_id, account_id.clone());
		assert_eq!(real_balance, expected_balance, "Unexpected CT balance for user {:?}", account_id);
	}
}

// project chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = Balance>,
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

	pub fn go_to_next_state(&mut self, project_id: ProjectId) -> ProjectStatus<BlockNumberFor<T>> {
		let project_details = self.get_project_details(project_id);
		let issuer = project_details.issuer_account;
		let original_state = project_details.status;
		if let Some(end_block) = project_details.round_duration.end() {
			self.jump_to_block(end_block + One::one());
		}
		let project_details = self.get_project_details(project_id);

		match project_details.status {
			ProjectStatus::Application => {
				self.execute(|| <Pallet<T>>::do_start_evaluation(issuer, project_id).unwrap());
			},
			ProjectStatus::EvaluationRound => {
				self.execute(|| <Pallet<T>>::do_end_evaluation(project_id).unwrap());
			},
			ProjectStatus::AuctionRound => {
				self.execute(|| <Pallet<T>>::do_end_auction(project_id).unwrap());
			},
			ProjectStatus::CommunityRound(..) => {
				self.execute(|| <Pallet<T>>::do_end_funding(project_id).unwrap());
			},
			ProjectStatus::FundingSuccessful | ProjectStatus::FundingFailed => {
				self.execute(|| <Pallet<T>>::do_start_settlement(project_id).unwrap());
			},
			_ => panic!("Unexpected project status"),
		}
		let new_details = self.get_project_details(project_id);
		assert_ne!(original_state, new_details.status, "Project should have transitioned to a new state");

		new_details.status
	}

	pub fn evaluate_for_users(
		&mut self,
		project_id: ProjectId,
		bonds: Vec<EvaluationParams<T>>,
	) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
		for EvaluationParams { account, usd_amount, receiving_account } in bonds {
			self.execute(|| {
				crate::Pallet::<T>::do_evaluate(
					&account.clone(),
					project_id,
					usd_amount,
					generate_did_from_account(account.clone()),
					project_policy.clone(),
					receiving_account,
				)
			})?;
		}
		Ok(().into())
	}

	pub fn bid_for_users(&mut self, project_id: ProjectId, bids: Vec<BidParams<T>>) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		for bid in bids {
			self.execute(|| {
				let did = generate_did_from_account(bid.bidder.clone());
				let params = DoBidParams::<T> {
					bidder: bid.bidder.clone(),
					project_id,
					ct_amount: bid.amount,
					mode: bid.mode,
					funding_asset: bid.asset,
					did,
					investor_type: InvestorType::Institutional,
					whitelisted_policy: project_policy.clone(),
					receiving_account: bid.receiving_account,
				};
				crate::Pallet::<T>::do_bid(params)
			})?;
		}
		Ok(().into())
	}

	pub fn contribute_for_users(
		&mut self,
		project_id: ProjectId,
		contributions: Vec<ContributionParams<T>>,
	) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		match self.get_project_details(project_id).status {
			ProjectStatus::CommunityRound(..) =>
				for cont in contributions {
					let did = generate_did_from_account(cont.contributor.clone());
					// We use institutional to be able to test most multipliers.
					let investor_type = InvestorType::Institutional;
					let params = DoContributeParams::<T> {
						contributor: cont.contributor.clone(),
						project_id,
						ct_amount: cont.amount,
						mode: cont.mode,
						funding_asset: cont.asset,
						did,
						investor_type,
						whitelisted_policy: project_policy.clone(),
						receiving_account: cont.receiving_account,
					};
					self.execute(|| crate::Pallet::<T>::do_contribute(params))?;
				},
			_ => panic!("Project should be in Community or Remainder status"),
		}

		Ok(().into())
	}

	pub fn settle_project(&mut self, project_id: ProjectId, mark_as_settled: bool) {
		self.execute(|| {
			Evaluations::<T>::iter_prefix((project_id,))
				.for_each(|(_, evaluation)| Pallet::<T>::do_settle_evaluation(evaluation, project_id).unwrap());

			Bids::<T>::iter_prefix((project_id,))
				.for_each(|(_, bid)| Pallet::<T>::do_settle_bid(bid, project_id).unwrap());

			Contributions::<T>::iter_prefix((project_id,))
				.for_each(|(_, contribution)| Pallet::<T>::do_settle_contribution(contribution, project_id).unwrap());

			if mark_as_settled {
				crate::Pallet::<T>::do_mark_project_as_settled(project_id).unwrap();
			}
		});
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

	// Used to check all the USDT/USDC/DOT was paid to the issuer funding account
	pub fn assert_total_funding_paid_out(
		&mut self,
		project_id: ProjectId,
		bids: Vec<BidInfoOf<T>>,
		contributions: Vec<ContributionInfoOf<T>>,
	) {
		let project_metadata = self.get_project_metadata(project_id);
		let mut total_expected_dot: Balance = Zero::zero();
		let mut total_expected_usdt: Balance = Zero::zero();
		let mut total_expected_usdc: Balance = Zero::zero();
		let mut total_expected_weth: Balance = Zero::zero();

		for bid in bids {
			match bid.funding_asset {
				AcceptedFundingAsset::DOT => total_expected_dot += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::USDT => total_expected_usdt += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::USDC => total_expected_usdc += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::WETH => total_expected_weth += bid.funding_asset_amount_locked,
			}
		}

		for contribution in contributions {
			match contribution.funding_asset {
				AcceptedFundingAsset::DOT => total_expected_dot += contribution.funding_asset_amount,
				AcceptedFundingAsset::USDT => total_expected_usdt += contribution.funding_asset_amount,
				AcceptedFundingAsset::USDC => total_expected_usdc += contribution.funding_asset_amount,
				AcceptedFundingAsset::WETH => total_expected_weth += contribution.funding_asset_amount,
			}
		}

		let total_stored_dot = self.get_free_funding_asset_balance_for(
			AcceptedFundingAsset::DOT.id(),
			project_metadata.funding_destination_account.clone(),
		);
		let total_stored_usdt = self.get_free_funding_asset_balance_for(
			AcceptedFundingAsset::USDT.id(),
			project_metadata.funding_destination_account.clone(),
		);
		let total_stored_usdc = self.get_free_funding_asset_balance_for(
			AcceptedFundingAsset::USDC.id(),
			project_metadata.funding_destination_account.clone(),
		);
		let total_stored_weth = self.get_free_funding_asset_balance_for(
			AcceptedFundingAsset::WETH.id(),
			project_metadata.funding_destination_account,
		);

		assert_eq!(total_expected_dot, total_stored_dot, "DOT amount is incorrect");
		assert_eq!(total_expected_usdt, total_stored_usdt, "USDT amount is incorrect");
		assert_eq!(total_expected_usdc, total_stored_usdc, "USDC amount is incorrect");
		assert_eq!(total_expected_weth, total_stored_weth, "WETH amount is incorrect");
	}

	// Used to check if all evaluations are settled correctly. We cannot check amount of
	// contributions minted for the user, as they could have received more tokens from other participations.
	pub fn assert_evaluations_migrations_created(
		&mut self,
		project_id: ProjectId,
		evaluations: Vec<EvaluationInfoOf<T>>,
		is_successful: bool,
	) {
		let details = self.get_project_details(project_id);
		assert!(matches!(details.status, ProjectStatus::SettlementFinished(_)));

		let evaluators_outcome = self.execute(|| {
			ProjectsDetails::<T>::get(project_id).unwrap().evaluation_round_info.evaluators_outcome.unwrap()
		});

		for evaluation in evaluations {
			let account = evaluation.evaluator.clone();
			assert_eq!(self.execute(|| { Evaluations::<T>::iter_prefix_values((&project_id, &account)).count() }), 0);

			let amount = if let EvaluatorsOutcome::Rewarded(ref info) = evaluators_outcome {
				assert!(is_successful);
				Pallet::<T>::calculate_evaluator_reward(&evaluation, info)
			} else {
				assert!(!is_successful);
				Zero::zero()
			};

			self.assert_migration(
				project_id,
				account,
				amount,
				evaluation.id,
				ParticipationType::Evaluation,
				evaluation.receiving_account,
				is_successful,
			);
		}
	}

	// Testing if a list of bids are settled correctly.
	pub fn assert_bids_migrations_created(
		&mut self,
		project_id: ProjectId,
		bids: Vec<BidInfoOf<T>>,
		is_successful: bool,
	) {
		for bid in bids {
			let account = bid.bidder.clone();
			assert_eq!(self.execute(|| { Bids::<T>::iter_prefix_values((&project_id, &account)).count() }), 0);
			let amount: Balance = bid.final_ct_amount();
			self.assert_migration(
				project_id,
				account,
				amount,
				bid.id,
				ParticipationType::Bid,
				bid.receiving_account,
				is_successful,
			);
		}
	}

	// Testing if a list of contributions are settled correctly.
	pub fn assert_contributions_migrations_created(
		&mut self,
		project_id: ProjectId,
		contributions: Vec<ContributionInfoOf<T>>,
		is_successful: bool,
	) {
		for contribution in contributions {
			let account = contribution.contributor.clone();
			assert_eq!(self.execute(|| { Bids::<T>::iter_prefix_values((&project_id, &account)).count() }), 0);
			let amount: Balance = if is_successful { contribution.ct_amount } else { 0u64.into() };
			self.assert_migration(
				project_id,
				account,
				amount,
				contribution.id,
				ParticipationType::Contribution,
				contribution.receiving_account,
				is_successful,
			);
		}
	}

	pub(crate) fn assert_migration(
		&mut self,
		project_id: ProjectId,
		account: AccountIdOf<T>,
		amount: Balance,
		id: u32,
		participation_type: ParticipationType,
		receiving_account: Junction,
		should_exist: bool,
	) {
		let Some((_migration_status, user_migrations)) =
			self.execute(|| UserMigrations::<T>::get((project_id, account.clone())))
		else {
			assert!(!should_exist);
			return;
		};
		let expected_migration_origin = MigrationOrigin { user: receiving_account, id, participation_type };

		let Some(migration) =
			user_migrations.into_iter().find(|migration| migration.origin == expected_migration_origin)
		else {
			assert!(!should_exist);
			return;
		};
		assert_close_enough!(
			migration.info.contribution_token_amount,
			amount,
			Perquintill::from_rational(999u64, 1000u64)
		);
	}

	pub fn create_new_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
	) -> ProjectId {
		// one ED for the issuer, one ED for the escrow account
		self.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), self.get_ed() * 2u128)]);

		let did = if let Some(did) = maybe_did.clone() { did } else { generate_did_from_account(issuer.clone()) };

		self.execute(|| {
			crate::Pallet::<T>::do_create_project(&issuer, project_metadata.clone(), did).unwrap();
			let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
			log::trace!("Last project metadata: {:?}", last_project_metadata);
		});

		let created_project_id = self.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
		self.creation_assertions(created_project_id, maybe_did, project_metadata);
		created_project_id
	}

	pub fn create_evaluating_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
	) -> ProjectId {
		let project_id = self.create_new_project(project_metadata, issuer.clone(), maybe_did);
		assert_eq!(self.go_to_next_state(project_id), ProjectStatus::EvaluationRound);

		project_id
	}

	pub fn create_auctioning_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
	) -> ProjectId {
		let project_id = self.create_evaluating_project(project_metadata, issuer.clone(), maybe_did);

		let evaluators = evaluations.accounts();
		self.mint_plmc_ed_if_required(evaluators.clone());

		let prev_supply = self.get_plmc_total_supply();
		let prev_free_plmc_balances = self.get_free_plmc_balances_for(evaluators.clone());
		let prev_held_plmc_balances =
			self.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Evaluation.into());

		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> =
			self.calculate_evaluation_plmc_spent(evaluations.clone());

		self.mint_plmc_to(plmc_evaluation_deposits.clone());

		self.evaluate_for_users(project_id, evaluations).unwrap();

		let expected_free_plmc_balances = prev_free_plmc_balances;
		let expected_held_plmc_balances = self.generic_map_operation(
			vec![prev_held_plmc_balances.clone(), plmc_evaluation_deposits.clone()],
			MergeOperation::Add,
		);
		let expected_total_plmc_supply =
			prev_supply + self.sum_balance_mappings(vec![plmc_evaluation_deposits.clone()]);

		self.evaluation_assertions(
			project_id,
			expected_free_plmc_balances,
			expected_held_plmc_balances,
			expected_total_plmc_supply,
		);

		assert_eq!(self.go_to_next_state(project_id), ProjectStatus::AuctionRound);

		project_id
	}

	pub fn create_community_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectId {
		let project_id =
			self.create_auctioning_project(project_metadata.clone(), issuer, maybe_did, evaluations.clone());
		if bids.is_empty() {
			assert!(matches!(self.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));
			return project_id
		}

		self.mint_plmc_ed_if_required(bids.accounts());
		self.mint_funding_asset_ed_if_required(bids.to_account_asset_map());

		let prev_plmc_supply = self.get_plmc_total_supply();
		let prev_free_plmc_balances = self.get_free_plmc_balances_for(bids.accounts());
		let prev_held_plmc_balances =
			self.get_reserved_plmc_balances_for(bids.accounts(), HoldReason::Participation.into());
		let prev_funding_asset_balances = self.get_free_funding_asset_balances_for(bids.to_account_asset_map());

		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> = self.calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> = self
			.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
		let reducible_evaluator_balances = self.slash_evaluator_balances(plmc_evaluation_deposits.clone());

		let necessary_plmc_mints = self.generic_map_operation(
			vec![plmc_bid_deposits.clone(), reducible_evaluator_balances],
			MergeOperation::Subtract,
		);
		let funding_asset_deposits = self.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);

		let expected_free_plmc_balances = prev_free_plmc_balances;
		let expected_held_plmc_balances =
			self.generic_map_operation(vec![prev_held_plmc_balances, plmc_bid_deposits], MergeOperation::Add);
		let expected_plmc_supply = prev_plmc_supply + necessary_plmc_mints.total();

		self.mint_plmc_to(necessary_plmc_mints.clone());
		self.mint_funding_asset_to(funding_asset_deposits.clone());

		self.bid_for_users(project_id, bids.clone()).unwrap();

		self.do_free_plmc_assertions(expected_free_plmc_balances);
		self.do_reserved_plmc_assertions(expected_held_plmc_balances, HoldReason::Participation.into());
		self.do_free_funding_asset_assertions(prev_funding_asset_balances);
		assert_eq!(self.get_plmc_total_supply(), expected_plmc_supply);

		assert!(matches!(self.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));

		project_id
	}

	pub fn create_remainder_contributing_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
		contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		let project_id = self.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			maybe_did,
			evaluations.clone(),
			bids.clone(),
		);

		if !contributions.is_empty() {
			let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();

			self.mint_plmc_ed_if_required(contributions.accounts());
			self.mint_funding_asset_ed_if_required(contributions.to_account_asset_map());

			let prev_free_plmc_balances = self.get_free_plmc_balances_for(contributions.accounts());
			let prev_held_plmc_balances =
				self.get_reserved_plmc_balances_for(contributions.accounts(), HoldReason::Participation.into());
			let prev_funding_asset_balances =
				self.get_free_funding_asset_balances_for(contributions.to_account_asset_map());
			let prev_plmc_supply = self.get_plmc_total_supply();

			let plmc_contribution_deposits = self.calculate_contributed_plmc_spent(contributions.clone(), ct_price);
			let funding_asset_contribution_deposits =
				self.calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

			let plmc_evaluation_deposits = self.calculate_evaluation_plmc_spent(evaluations.clone());
			let reducible_evaluator_balances = self.slash_evaluator_balances(plmc_evaluation_deposits.clone());
			let necessary_plmc_contribution_mint = self.generic_map_operation(
				vec![plmc_contribution_deposits.clone(), reducible_evaluator_balances],
				MergeOperation::Subtract,
			);

			let expected_free_plmc_balances = prev_free_plmc_balances;

			let expected_held_plmc_balances = self
				.generic_map_operation(vec![prev_held_plmc_balances, plmc_contribution_deposits], MergeOperation::Add);

			let expected_plmc_supply = prev_plmc_supply + necessary_plmc_contribution_mint.total();

			self.mint_plmc_to(necessary_plmc_contribution_mint.clone());
			self.mint_funding_asset_to(funding_asset_contribution_deposits.clone());

			self.contribute_for_users(project_id, contributions).expect("Contributing should work");

			self.do_free_plmc_assertions(expected_free_plmc_balances);
			self.do_reserved_plmc_assertions(expected_held_plmc_balances, HoldReason::Participation.into());

			self.do_free_funding_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
			assert_eq!(self.get_plmc_total_supply(), expected_plmc_supply);
		}

		let ProjectStatus::CommunityRound(remainder_block) = self.get_project_details(project_id).status else {
			panic!("Project should be in CommunityRound status");
		};
		self.jump_to_block(remainder_block);

		project_id
	}

	pub fn create_finished_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		let project_id = self.create_remainder_contributing_project(
			project_metadata.clone(),
			issuer,
			maybe_did,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
		);

		if !remainder_contributions.is_empty() {
			let ct_price = self.get_project_details(project_id).weighted_average_price.unwrap();

			self.mint_plmc_ed_if_required(remainder_contributions.accounts());
			self.mint_funding_asset_ed_if_required(remainder_contributions.to_account_asset_map());

			let prev_free_plmc_balances = self.get_free_plmc_balances_for(remainder_contributions.accounts());
			let prev_held_plmc_balances = self
				.get_reserved_plmc_balances_for(remainder_contributions.accounts(), HoldReason::Participation.into());
			let prev_funding_asset_balances =
				self.get_free_funding_asset_balances_for(remainder_contributions.to_account_asset_map());
			let prev_supply = self.get_plmc_total_supply();

			let plmc_evaluation_deposits = self.calculate_evaluation_plmc_spent(evaluations);
			let plmc_bid_deposits = self.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
			let plmc_remainder_contribution_deposits =
				self.calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);
			let reducible_evaluator_balances = self.slash_evaluator_balances(plmc_evaluation_deposits);
			let remaining_reducible_evaluator_balances = self.generic_map_operation(
				vec![reducible_evaluator_balances, plmc_bid_deposits.clone()],
				MergeOperation::Subtract,
			);

			let necessary_plmc_contribution_mint = self.generic_map_operation(
				vec![plmc_remainder_contribution_deposits.clone(), remaining_reducible_evaluator_balances],
				MergeOperation::Subtract,
			);

			let funding_asset_deposits =
				self.calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

			let expected_free_plmc_balances = prev_free_plmc_balances;
			let expected_held_plmc_balances = self.generic_map_operation(
				vec![prev_held_plmc_balances, plmc_remainder_contribution_deposits],
				MergeOperation::Add,
			);

			let expected_supply = prev_supply + necessary_plmc_contribution_mint.total();

			self.mint_plmc_to(necessary_plmc_contribution_mint.clone());
			self.mint_funding_asset_to(funding_asset_deposits.clone());

			self.contribute_for_users(project_id, remainder_contributions.clone())
				.expect("Remainder Contributing should work");

			self.do_free_plmc_assertions(expected_free_plmc_balances);
			self.do_reserved_plmc_assertions(expected_held_plmc_balances, HoldReason::Participation.into());
			self.do_free_funding_asset_assertions(prev_funding_asset_balances);
			assert_eq!(self.get_plmc_total_supply(), expected_supply);
		}

		let status = self.go_to_next_state(project_id);

		if status == ProjectStatus::FundingSuccessful {
			// Check that remaining CTs are updated
			let project_details = self.get_project_details(project_id);
			// if our bids were creating an oversubscription, then just take the total allocation size
			let auction_bought_tokens = bids
				.iter()
				.map(|bid| bid.amount)
				.fold(Balance::zero(), |acc, item| item + acc)
				.min(project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size);
			let community_bought_tokens =
				community_contributions.iter().map(|cont| cont.amount).fold(Balance::zero(), |acc, item| item + acc);
			let remainder_bought_tokens =
				remainder_contributions.iter().map(|cont| cont.amount).fold(Balance::zero(), |acc, item| item + acc);

			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size -
					auction_bought_tokens -
					community_bought_tokens -
					remainder_bought_tokens,
				"Remaining CTs are incorrect"
			);
		} else if status == ProjectStatus::FundingFailed {
			self.test_ct_not_created_for(project_id);
		} else {
			panic!("Project should be in FundingSuccessful or FundingFailed status");
		}

		project_id
	}

	pub fn create_settled_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
		mark_as_settled: bool,
	) -> ProjectId {
		let project_id = self.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			maybe_did,
			evaluations.clone(),
			bids.clone(),
			community_contributions.clone(),
			remainder_contributions.clone(),
		);

		assert!(matches!(self.go_to_next_state(project_id), ProjectStatus::SettlementStarted(_)));

		self.settle_project(project_id, mark_as_settled);
		project_id
	}

	pub fn create_project_at(
		&mut self,
		status: ProjectStatus<BlockNumberFor<T>>,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
		community_contributions: Vec<ContributionParams<T>>,
		remainder_contributions: Vec<ContributionParams<T>>,
	) -> ProjectId {
		match status {
			ProjectStatus::FundingSuccessful => self.create_finished_project(
				project_metadata,
				issuer,
				None,
				evaluations,
				bids,
				community_contributions,
				remainder_contributions,
			),
			ProjectStatus::CommunityRound(..) =>
				self.create_community_contributing_project(project_metadata, issuer, None, evaluations, bids),
			ProjectStatus::AuctionRound => self.create_auctioning_project(project_metadata, issuer, None, evaluations),
			ProjectStatus::EvaluationRound => self.create_evaluating_project(project_metadata, issuer, None),
			ProjectStatus::Application => self.create_new_project(project_metadata, issuer, None),
			_ => panic!("unsupported project creation in that status"),
		}
	}
}
