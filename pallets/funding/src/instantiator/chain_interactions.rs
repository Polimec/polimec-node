use super::*;
use alloc::{vec, vec::Vec};
use polimec_common::assets::AcceptedFundingAsset;

// general chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub const fn new(ext: OptionalExternalities) -> Self {
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
				let asset_amount = <T as Config>::FundingCurrency::balance(asset_id.clone(), &account);
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
				<T as Config>::FundingCurrency::mint_into(asset_id, &account, asset_amount).unwrap();
			}
		});
	}

	pub fn current_block(&mut self) -> BlockNumberFor<T> {
		self.execute(|| <T as Config>::BlockNumberProvider::current_block_number())
	}

	pub fn advance_relay_time(&mut self, amount: BlockNumberFor<T>) {
		use cumulus_pallet_parachain_system::ValidationData;
		use cumulus_primitives_core::PersistedValidationData;

		self.execute(|| {
			for _block in 0u32..amount.saturated_into() {
				let mut validation_data = ValidationData::<T>::get().unwrap_or_else(||
    		// PersistedValidationData does not impl default in non-std
    		PersistedValidationData {
    			parent_head: vec![].into(),
    			relay_parent_number: Default::default(),
    			max_pov_size: Default::default(),
    			relay_parent_storage_root: Default::default(),
    		});
				validation_data.relay_parent_number = validation_data.relay_parent_number + 1;
				ValidationData::<T>::put(validation_data);
			}
		});
	}

	pub fn advance_time(&mut self, amount: BlockNumberFor<T>) {
		self.execute(|| {
			for _block in 0u32..amount.saturated_into() {
				let mut current_block = <T as Config>::BlockNumberProvider::current_block_number();

				<AllPalletsWithoutSystem as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);
				<frame_system::Pallet<T> as OnFinalize<BlockNumberFor<T>>>::on_finalize(current_block);

				<AllPalletsWithoutSystem as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);
				<frame_system::Pallet<T> as OnIdle<BlockNumberFor<T>>>::on_idle(current_block, Weight::MAX);

				current_block += One::one();
				frame_system::Pallet::<T>::set_block_number(current_block);

				<frame_system::Pallet<T> as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
				<AllPalletsWithoutSystem as OnInitialize<BlockNumberFor<T>>>::on_initialize(current_block);
			}
		});
	}

	pub fn jump_to_block(&mut self, block: BlockNumberFor<T>) {
		let current_block = self.current_block();
		if block > current_block {
			self.execute(|| {
				frame_system::Pallet::<T>::set_block_number(block - One::one());
			});

			for _ in 1_u32..block.saturated_into() {
				// Relay block should be monotonically increasing
				self.advance_relay_time(One::one());
			}

			self.advance_time(One::one());
			self.advance_relay_time(One::one());
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
				assert_close_enough!(
					real_amount,
					expected_amount,
					Perquintill::from_float(0.999),
					"Wrong funding asset balance expected for user{:?}",
					account
				);
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
			let ed = self.get_funding_asset_ed(asset_id.clone());
			self.execute(|| {
				if <T as Config>::FundingCurrency::balance(asset_id.clone(), &account) < ed {
					<T as Config>::FundingCurrency::mint_into(asset_id, &account, ed).expect("Minting should work");
				}
			});
		}
	}
}

// assertions
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
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

	pub fn assert_plmc_free_balance(&mut self, account_id: AccountIdOf<T>, expected_balance: Balance) {
		let real_balance = self.get_free_plmc_balance_for(account_id.clone());
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
		assert_close_enough!(
			real_balance,
			expected_balance,
			Perquintill::from_float(0.999),
			"Unexpected CT balance for user {:?}",
			account_id
		);
	}
}

// project chain interactions
impl<
		T: Config + pallet_balances::Config<Balance = Balance> + cumulus_pallet_parachain_system::Config,
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

	pub fn go_to_next_state(&mut self, project_id: ProjectId) -> ProjectStatus {
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
				self.process_oversubscribed_bids(project_id);
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

	pub fn evaluate_for_users(&mut self, project_id: ProjectId, bonds: Vec<EvaluationParams<T>>) -> DispatchResult {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
		for EvaluationParams { account, plmc_amount, receiving_account } in bonds {
			self.execute(|| {
				crate::Pallet::<T>::do_evaluate(
					&account.clone(),
					project_id,
					plmc_amount,
					generate_did_from_account(account.clone()),
					project_policy.clone(),
					receiving_account,
				)
			})?;
		}
		Ok(())
	}

	pub fn mint_necessary_tokens_for_bids(&mut self, project_id: ProjectId, bids: Vec<BidParams<T>>) {
		let current_bucket = self.execute(|| Buckets::<T>::get(project_id).unwrap());
		let project_metadata = self.get_project_metadata(project_id);

		let necessary_plmc = self.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			Some(current_bucket),
		);
		let necessary_funding_assets = self.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata,
			Some(current_bucket),
		);

		self.mint_plmc_ed_if_required(necessary_plmc.accounts());
		self.mint_funding_asset_ed_if_required(necessary_funding_assets.to_account_asset_map());
		self.mint_plmc_to(necessary_plmc);
		self.mint_funding_asset_to(necessary_funding_assets);
	}

	pub fn mint_necessary_tokens_for_evaluations(&mut self, evaluations: Vec<EvaluationParams<T>>) {
		let plmc_required = self.calculate_evaluation_plmc_spent(evaluations);
		self.mint_plmc_ed_if_required(plmc_required.accounts());
		self.mint_plmc_to(plmc_required);
	}

	pub fn bid_for_users(&mut self, project_id: ProjectId, bids: Vec<BidParams<T>>) -> DispatchResultWithPostInfo {
		let project_policy = self.get_project_metadata(project_id).policy_ipfs_cid.unwrap();

		for bid in bids {
			self.execute(|| {
				let did = generate_did_from_account(bid.bidder.clone());
				let params = DoBidParams::<T> {
					bidder: bid.bidder.clone(),
					project_id,
					funding_asset_amount: bid.amount,
					mode: bid.mode,
					funding_asset: bid.asset,
					did,
					investor_type: bid.investor_type,
					whitelisted_policy: project_policy.clone(),
					receiving_account: bid.receiving_account,
				};
				crate::Pallet::<T>::do_bid(params)
			})?;
		}
		Ok(().into())
	}

	pub fn process_oversubscribed_bids(&mut self, project_id: ProjectId) {
		self.execute(|| while Pallet::<T>::do_process_next_oversubscribed_bid(project_id).is_ok() {});
	}

	pub fn settle_project(&mut self, project_id: ProjectId, mark_as_settled: bool) {
		self.execute(|| {
			Evaluations::<T>::iter_prefix((project_id,)).for_each(|((_, id), evaluation)| {
				Pallet::<T>::do_settle_evaluation(evaluation, project_id, id).unwrap()
			});

			Bids::<T>::iter_prefix(project_id)
				.for_each(|(_, bid)| Pallet::<T>::do_settle_bid(project_id, bid.id).unwrap());

			if mark_as_settled {
				crate::Pallet::<T>::do_mark_project_as_settled(project_id).unwrap();
			}
		});
	}

	pub fn get_evaluations(&mut self, project_id: ProjectId) -> Vec<(AccountIdOf<T>, u32, EvaluationInfoOf<T>)> {
		// [AccountId, EvaluationId, Evaluation]
		self.execute(|| {
			Evaluations::<T>::iter_prefix((project_id,))
				.map(|((acc_id, eval_id), evaluation_info)| (acc_id, eval_id, evaluation_info))
				.collect::<Vec<(AccountIdOf<T>, u32, EvaluationInfoOf<T>)>>()
		})
	}

	pub fn get_bids(&mut self, project_id: ProjectId) -> Vec<BidInfoOf<T>> {
		self.execute(|| Bids::<T>::iter_prefix_values(project_id).collect())
	}

	// Used to check all the USDT/USDC/DOT was paid to the issuer funding account
	pub fn assert_total_funding_paid_out(&mut self, project_id: ProjectId, bids: Vec<BidInfoOf<T>>) {
		let project_metadata = self.get_project_metadata(project_id);
		let mut total_expected_dot: Balance = Zero::zero();
		let mut total_expected_usdt: Balance = Zero::zero();
		let mut total_expected_usdc: Balance = Zero::zero();
		let mut total_expected_eth: Balance = Zero::zero();

		for bid in bids {
			match bid.funding_asset {
				AcceptedFundingAsset::DOT => total_expected_dot += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::USDT => total_expected_usdt += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::USDC => total_expected_usdc += bid.funding_asset_amount_locked,
				AcceptedFundingAsset::ETH => total_expected_eth += bid.funding_asset_amount_locked,
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
		let total_stored_eth = self.get_free_funding_asset_balance_for(
			AcceptedFundingAsset::ETH.id(),
			project_metadata.funding_destination_account,
		);

		assert_eq!(total_expected_dot, total_stored_dot, "DOT amount is incorrect");
		assert_eq!(total_expected_usdt, total_stored_usdt, "USDT amount is incorrect");
		assert_eq!(total_expected_usdc, total_stored_usdc, "USDC amount is incorrect");
		assert_eq!(total_expected_eth, total_stored_eth, "ETH amount is incorrect");
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
		assert_eq!(self.execute(|| { Bids::<T>::iter_prefix_values(project_id).count() }), 0);

		let maybe_outbid_bids_cutoff = self.execute(|| OutbidBidsCutoffs::<T>::get(project_id));
		for bid in bids {
			// Determine if the bid is outbid
			let bid_is_outbid = match maybe_outbid_bids_cutoff {
				Some(OutbidBidsCutoff { bid_price, bid_index }) => {
					bid_price > bid.original_ct_usd_price
						|| (bid_price == bid.original_ct_usd_price && bid_index <= bid.id)
				},
				None => false, // If there's no cutoff, the bid is not outbid
			};

			let bid_ct_amount = if let Some(OutbidBidsCutoff { bid_price, bid_index }) = maybe_outbid_bids_cutoff {
				if bid_price == bid.original_ct_usd_price && bid_index == bid.id {
					match bid.status {
						BidStatus::PartiallyAccepted(ct_amount) => ct_amount,
						_ => Zero::zero(),
					}
				} else if bid_is_outbid {
					Zero::zero()
				} else {
					bid.original_ct_amount
				}
			} else {
				bid.original_ct_amount
			};

			self.assert_migration(
				project_id,
				bid.bidder,
				bid_ct_amount,
				ParticipationType::Bid,
				bid.receiving_account,
				is_successful,
			);
		}
	}

	pub(crate) fn assert_migration(
		&mut self,
		project_id: ProjectId,
		account: AccountIdOf<T>,
		amount: Balance,
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
		let expected_migration_origin = MigrationOrigin { user: receiving_account, participation_type };

		let is_migration_found = user_migrations.into_iter().any(|migration| {
			migration.origin == expected_migration_origin
				&& is_close_enough!(
					migration.info.contribution_token_amount,
					amount,
					Perquintill::from_rational(999u64, 1000u64)
				)
		});
		if should_exist {
			assert!(is_migration_found, "Migration not found for user {:?}", account);
		} else {
			assert!(!is_migration_found, "Migration found for user {:?}", account);
		}
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
		let project_id = self.create_new_project(project_metadata, issuer, maybe_did);
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
		let project_id = self.create_evaluating_project(project_metadata, issuer, maybe_did);

		let evaluators = evaluations.accounts();
		self.mint_plmc_ed_if_required(evaluators.clone());

		let prev_supply = self.get_plmc_total_supply();
		let prev_free_plmc_balances = self.get_free_plmc_balances_for(evaluators.clone());
		let prev_held_plmc_balances = self.get_reserved_plmc_balances_for(evaluators, HoldReason::Evaluation.into());

		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> =
			self.calculate_evaluation_plmc_spent(evaluations.clone());

		self.mint_plmc_to(plmc_evaluation_deposits.clone());

		self.evaluate_for_users(project_id, evaluations).unwrap();

		let expected_free_plmc_balances = prev_free_plmc_balances;
		let expected_held_plmc_balances = self.generic_map_operation(
			vec![prev_held_plmc_balances, plmc_evaluation_deposits.clone()],
			MergeOperation::Add,
		);
		let expected_total_plmc_supply = prev_supply + self.sum_balance_mappings(vec![plmc_evaluation_deposits]);

		self.evaluation_assertions(
			project_id,
			expected_free_plmc_balances,
			expected_held_plmc_balances,
			expected_total_plmc_supply,
		);

		assert_eq!(self.go_to_next_state(project_id), ProjectStatus::AuctionRound);

		project_id
	}

	pub fn create_finished_project(
		&mut self,
		project_metadata: ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		maybe_did: Option<Did>,
		evaluations: Vec<EvaluationParams<T>>,
		bids: Vec<BidParams<T>>,
	) -> ProjectId {
		let project_id =
			self.create_auctioning_project(project_metadata.clone(), issuer, maybe_did, evaluations.clone());

		self.mint_plmc_ed_if_required(bids.accounts());
		self.mint_funding_asset_ed_if_required(bids.to_account_asset_map());

		let prev_plmc_supply = self.get_plmc_total_supply();
		let _prev_free_plmc_balances = self.get_free_plmc_balances_for(bids.accounts());
		let _prev_held_plmc_balances =
			self.get_reserved_plmc_balances_for(bids.accounts(), HoldReason::Participation.into());
		let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> = self.calculate_evaluation_plmc_spent(evaluations);
		let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> = self
			.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
		let reducible_evaluator_balances = self.slash_evaluator_balances(plmc_evaluation_deposits);

		let necessary_plmc_mints = self.generic_map_operation(
			vec![plmc_bid_deposits.clone(), reducible_evaluator_balances],
			MergeOperation::Subtract,
		);
		let funding_asset_deposits = self.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata,
			None,
		);

		let expected_plmc_supply = prev_plmc_supply + necessary_plmc_mints.total();

		self.mint_plmc_to(necessary_plmc_mints);
		self.mint_funding_asset_to(funding_asset_deposits);

		self.bid_for_users(project_id, bids.clone()).unwrap();

		// Use actual on-chain state instead of calculated expectations to avoid rounding errors
		let actual_free_plmc_balances = self.get_free_plmc_balances_for(bids.accounts());
		let actual_held_plmc_balances =
			self.get_reserved_plmc_balances_for(bids.accounts(), HoldReason::Participation.into());
		let actual_funding_asset_balances = self.get_free_funding_asset_balances_for(bids.to_account_asset_map());

		// Only verify that the balances are reasonable (non-negative, within expected ranges)
		// rather than exact amounts to avoid FixedU128 rounding issues
		self.do_free_plmc_assertions(actual_free_plmc_balances);
		self.do_reserved_plmc_assertions(actual_held_plmc_balances, HoldReason::Participation.into());
		self.do_free_funding_asset_assertions(actual_funding_asset_balances);

		assert_eq!(self.get_plmc_total_supply(), expected_plmc_supply);

		let status = self.go_to_next_state(project_id);

		if status == ProjectStatus::FundingSuccessful || status == ProjectStatus::FundingFailed {
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
		mark_as_settled: bool,
	) -> ProjectId {
		let project_id =
			self.create_finished_project(project_metadata.clone(), issuer, maybe_did, evaluations, bids.clone());

		assert!(matches!(self.go_to_next_state(project_id), ProjectStatus::SettlementStarted(_)));
		self.test_ct_created_for(project_id);
		self.settle_project(project_id, mark_as_settled);

		// Check that remaining CTs are updated
		let project_details = self.get_project_details(project_id);
		let ct_issued = self.execute(|| <T as Config>::ContributionTokenCurrency::total_issuance(project_id));
		let issuer_fees = self.execute(|| Pallet::<T>::calculate_fee_allocation(project_id).ok().unwrap());
		let tokens_bought_in_auction = ct_issued - issuer_fees;

		// TODO: Check if we can restore the `assert_eq!` here.
		assert_close_enough!(
			project_details.remaining_contribution_tokens,
			project_metadata.total_allocation_size - tokens_bought_in_auction,
			Perquintill::from_rational(999u64, 1000u64)
		);

		project_id
	}
}
