use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[test]
	fn auction_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(ISSUER_1);
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 60, 10);
		let _project_id = inst.create_finished_project(project_metadata, ISSUER_1, None, evaluations, bids);
	}

	#[test]
	fn multiple_auction_projects_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project1 = default_project_metadata(ISSUER_1);
		let project2 = default_project_metadata(ISSUER_2);
		let project3 = default_project_metadata(ISSUER_3);
		let project4 = default_project_metadata(ISSUER_4);
		let evaluations = inst.generate_successful_evaluations(project1.clone(), 5);
		let bids = inst.generate_bids_from_total_ct_percent(project1.clone(), 60, 10);

		inst.create_finished_project(project1, ISSUER_1, None, evaluations.clone(), bids.clone());
		inst.create_finished_project(project2, ISSUER_2, None, evaluations.clone(), bids.clone());
		inst.create_finished_project(project3, ISSUER_3, None, evaluations.clone(), bids.clone());
		inst.create_finished_project(project4, ISSUER_4, None, evaluations, bids);
	}

	#[test]
	fn no_bids_made() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER_1;
		let project_metadata = default_project_metadata(issuer);
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

		assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed));
	}

	#[test]
	fn different_decimals_ct_works_as_expected() {
		// Setup some base values to compare different decimals
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let ed = inst.get_ed();
		let default_project_metadata = default_project_metadata(ISSUER_1);
		let original_decimal_aware_price = default_project_metadata.minimum_price;
		let original_price = <TestRuntime as Config>::PriceProvider::convert_back_to_normal_price(
			original_decimal_aware_price,
			USD_DECIMALS,
			default_project_metadata.token_information.decimals,
		)
		.unwrap();
		let usable_plmc_price = inst.execute(|| {
			<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(&Location::here(), PLMC_DECIMALS).unwrap()
		});
		let usdt_price = inst
			.execute(|| PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::USDT).unwrap());
		let usdc_price = inst
			.execute(|| PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::USDC).unwrap());
		let dot_price = inst
			.execute(|| PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::DOT).unwrap());

		let mut funding_assets_cycle =
			vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].into_iter().cycle();

		let mut min_bid_amounts_ct = Vec::new();
		let mut min_bid_amounts_usd = Vec::new();
		let mut auction_allocations_ct = Vec::new();
		let mut auction_allocations_usd = Vec::new();

		let mut decimal_test = |decimals: u8| {
			let funding_asset = funding_assets_cycle.next().unwrap();
			let funding_asset_usd_price = match funding_asset {
				AcceptedFundingAsset::USDT => usdt_price,
				AcceptedFundingAsset::USDC => usdc_price,
				AcceptedFundingAsset::DOT => dot_price,
				AcceptedFundingAsset::ETH => todo!(),
			};

			let mut project_metadata = default_project_metadata.clone();
			project_metadata.token_information.decimals = decimals;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				original_price,
				USD_DECIMALS,
				decimals,
			)
			.unwrap();

			project_metadata.total_allocation_size = 1_000_000 * 10u128.pow(decimals as u32);
			project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
			project_metadata.participation_currencies = bounded_vec!(funding_asset);

			let issuer: AccountIdOf<TestRuntime> = (10_000 + inst.get_new_nonce()).try_into().unwrap();
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let auction_allocation_ct = project_metadata.total_allocation_size;
			auction_allocations_ct.push(auction_allocation_ct);
			let auction_allocation_usd = project_metadata.minimum_price.saturating_mul_int(auction_allocation_ct);
			auction_allocations_usd.push(auction_allocation_usd);

			let min_professional_bid_usd =
				project_metadata.bidding_ticket_sizes.professional.usd_minimum_per_participation + 1000000;
			min_bid_amounts_usd.push(min_professional_bid_usd);
			let min_professional_bid_ct =
				project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);
			let min_professional_bid_plmc =
				usable_plmc_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);
			min_bid_amounts_ct.push(min_professional_bid_ct);
			let min_professional_bid_funding_asset =
				funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);

			// Every project should want to raise 10MM USD on the auction round regardless of CT decimals
			assert_eq!(auction_allocation_usd, 10_000_000 * USD_UNIT);

			// A minimum bid goes through. This is a fixed USD value, but the extrinsic amount depends on CT decimals.
			inst.mint_plmc_ed_if_required(vec![BIDDER_1]);
			inst.mint_funding_asset_ed_if_required(vec![(BIDDER_1, funding_asset.id())]);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(BIDDER_1, min_professional_bid_plmc + ed)]);
			inst.mint_funding_asset_to(vec![UserToFundingAsset::new(
				BIDDER_1,
				min_professional_bid_funding_asset,
				funding_asset.id(),
			)]);

			let fundig_asset_amount =
				funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);

			assert_ok!(inst.execute(|| PolimecFunding::bid(
				RuntimeOrigin::signed(BIDDER_1),
				get_mock_jwt_with_cid(
					BIDDER_1,
					InvestorType::Professional,
					generate_did_from_account(BIDDER_1),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				fundig_asset_amount, // TODO: Probably with the .reciprocal() we're missing some 0.00000x amount with the rounding.
				ParticipationMode::Classic(1u8),
				funding_asset,
			)));

			// The bucket should have 1MM * 10^decimals CT minus what we just bid
			let bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());
			assert_close_enough!(
				bucket.amount_left,
				1_000_000u128 * 10u128.pow(decimals as u32) - min_professional_bid_ct,
				Perquintill::from_float(0.9999)
			);
		};

		for decimals in 6..=18 {
			decimal_test(decimals);
		}

		// Since we use the same original price and allocation size and adjust for decimals,
		// the USD amounts should be the same
		assert!(min_bid_amounts_usd.iter().all(|x| *x == min_bid_amounts_usd[0]));
		assert!(auction_allocations_usd.iter().all(|x| *x == auction_allocations_usd[0]));

		// CT amounts however should be different from each other
		let mut hash_set_1 = HashSet::new();
		for amount in min_bid_amounts_ct {
			assert!(!hash_set_1.contains(&amount));
			hash_set_1.insert(amount);
		}
		let mut hash_set_2 = HashSet::new();
		for amount in auction_allocations_ct {
			assert!(!hash_set_2.contains(&amount));
			hash_set_2.insert(amount);
		}
	}

	#[test]
	fn auction_oversubscription() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(ISSUER_1);
		let mut bucket = Pallet::<TestRuntime>::create_bucket_from_metadata(&project_metadata);
		bucket.current_price =
			bucket.initial_price + (bucket.delta_price.mul(PriceOf::<TestRuntime>::from_float(10.0)));
		bucket.amount_left = bucket.delta_amount;
		let bids = inst.generate_bids_from_bucket(project_metadata.clone(), bucket, USDT);

		let _project_id = inst.create_finished_project(
			project_metadata.clone(),
			ISSUER_1,
			None,
			inst.generate_successful_evaluations(project_metadata.clone(), 5),
			bids,
		);
	}

	#[test]
	fn on_idle_clears_oversubscribed_bids() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(ISSUER_1);
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 100, 10);

		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
		inst.mint_necessary_tokens_for_bids(project_id, bids.clone());
		inst.bid_for_users(project_id, bids.clone()).unwrap();

		// Check full rejection of one bid by another
		let last_bid_amount = bids[9].amount;
		let oversubscribed_bid =
			BidParams::<TestRuntime>::from((BIDDER_1, Retail, last_bid_amount, Classic(1u8), USDT));
		inst.mint_necessary_tokens_for_bids(project_id, vec![oversubscribed_bid.clone()]);
		inst.bid_for_users(project_id, vec![oversubscribed_bid.clone()]).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), last_bid_amount);
		inst.advance_time(1);
		let rejected_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 9)).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), Zero::zero());
		assert_eq!(rejected_bid.status, BidStatus::Rejected);
		let yet_unknown_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 8)).unwrap();
		assert_eq!(yet_unknown_bid.status, BidStatus::YetUnknown);

		// Check multiple bid rejections by one bid
		let multiple_bids_amount = bids[8].amount + bids[7].amount + bids[6].amount;
		let multiple_bids =
			BidParams::<TestRuntime>::from((BIDDER_1, Retail, multiple_bids_amount, Classic(1u8), USDT));
		inst.mint_necessary_tokens_for_bids(project_id, vec![multiple_bids.clone()]);
		inst.bid_for_users(project_id, vec![multiple_bids.clone()]).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), multiple_bids_amount);
		inst.advance_time(1);
		let rejected_bid_1 = inst.execute(|| Bids::<TestRuntime>::get(project_id, 8)).unwrap();
		let rejected_bid_2 = inst.execute(|| Bids::<TestRuntime>::get(project_id, 7)).unwrap();
		let rejected_bid_3 = inst.execute(|| Bids::<TestRuntime>::get(project_id, 6)).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), Zero::zero());
		assert_eq!(rejected_bid_1.status, BidStatus::Rejected);
		assert_eq!(rejected_bid_2.status, BidStatus::Rejected);
		assert_eq!(rejected_bid_3.status, BidStatus::Rejected);
		let yet_unknown_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 5)).unwrap();
		assert_eq!(yet_unknown_bid.status, BidStatus::YetUnknown);

		// Check partial rejection of one bid by another
		let partial_bid_amount = last_bid_amount / 2;
		let partial_bid = BidParams::<TestRuntime>::from((BIDDER_1, Retail, partial_bid_amount, Classic(1u8), USDT));
		inst.mint_necessary_tokens_for_bids(project_id, vec![partial_bid.clone()]);
		inst.bid_for_users(project_id, vec![partial_bid.clone()]).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), partial_bid_amount);
		inst.advance_time(1);
		let rejected_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 5)).unwrap();
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id)), Zero::zero());
		assert_eq!(rejected_bid.status, BidStatus::PartiallyAccepted(last_bid_amount - partial_bid_amount));
		let yet_unknown_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 4)).unwrap();
		assert_eq!(yet_unknown_bid.status, BidStatus::YetUnknown);
	}

	#[test]
	fn on_idle_clears_multiple_oversubscribed_projects() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		// Create two projects with different metadata
		let project_metadata_1 = default_project_metadata(ISSUER_1);
		let project_metadata_2 = default_project_metadata(ISSUER_2);

		// Generate evaluations and bids for both projects
		let evaluations_1 = inst.generate_successful_evaluations(project_metadata_1.clone(), 5);
		let evaluations_2 = inst.generate_successful_evaluations(project_metadata_2.clone(), 5);
		let bids_1 = inst.generate_bids_from_total_ct_percent(project_metadata_1.clone(), 100, 5);
		let bids_2 = inst.generate_bids_from_total_ct_percent(project_metadata_2.clone(), 100, 5);

		// Create two projects in auctioning state
		let project_id_1 = inst.create_auctioning_project(project_metadata_1.clone(), ISSUER_1, None, evaluations_1);
		let project_id_2 = inst.create_auctioning_project(project_metadata_2.clone(), ISSUER_2, None, evaluations_2);

		// Place initial bids for both projects
		inst.mint_necessary_tokens_for_bids(project_id_1, bids_1.clone());
		inst.mint_necessary_tokens_for_bids(project_id_2, bids_2.clone());
		inst.bid_for_users(project_id_1, bids_1.clone()).unwrap();
		inst.bid_for_users(project_id_2, bids_2.clone()).unwrap();

		// Create oversubscribed bids for both projects
		let oversubscribed_bid_1 =
			BidParams::<TestRuntime>::from((BIDDER_1, Retail, bids_1[4].amount, Classic(1u8), USDT));
		let oversubscribed_bid_2 =
			BidParams::<TestRuntime>::from((BIDDER_2, Retail, bids_2[4].amount, Classic(1u8), USDT));

		// Place oversubscribed bids
		inst.mint_necessary_tokens_for_bids(project_id_1, vec![oversubscribed_bid_1.clone()]);
		inst.mint_necessary_tokens_for_bids(project_id_2, vec![oversubscribed_bid_2.clone()]);
		inst.bid_for_users(project_id_1, vec![oversubscribed_bid_1.clone()]).unwrap();
		inst.bid_for_users(project_id_2, vec![oversubscribed_bid_2.clone()]).unwrap();

		// Verify both projects are oversubscribed
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id_1)), bids_1[4].amount);
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id_2)), bids_2[4].amount);

		// Advance time to trigger on_idle
		inst.advance_time(1);

		// Verify oversubscribed amounts are cleared for both projects
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id_1)), Zero::zero());
		assert_eq!(inst.execute(|| CTAmountOversubscribed::<TestRuntime>::get(project_id_2)), Zero::zero());

		// Verify bid statuses for both projects
		let rejected_bid_1 = inst.execute(|| Bids::<TestRuntime>::get(project_id_1, 4)).unwrap();
		let rejected_bid_2 = inst.execute(|| Bids::<TestRuntime>::get(project_id_2, 9)).unwrap();
		assert_eq!(rejected_bid_1.status, BidStatus::Rejected);
		assert_eq!(rejected_bid_2.status, BidStatus::Rejected);

		let yet_unknown_bid_1 = inst.execute(|| Bids::<TestRuntime>::get(project_id_1, 3)).unwrap();
		let yet_unknown_bid_2 = inst.execute(|| Bids::<TestRuntime>::get(project_id_2, 8)).unwrap();
		assert_eq!(yet_unknown_bid_1.status, BidStatus::YetUnknown);
		assert_eq!(yet_unknown_bid_2.status, BidStatus::YetUnknown);

		inst.go_to_next_state(project_id_1);
		inst.go_to_next_state(project_id_2);

		assert!(inst.execute(|| ProjectsInAuctionRound::<TestRuntime>::iter_keys().next().is_none()))
	}
}

#[cfg(test)]
mod bid_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use frame_support::dispatch::DispatchResultWithPostInfo;

		use super::*;

		#[test]
		fn evaluation_bond_counts_towards_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let evaluator_bidder = 69u64;
			let evaluation_amount = 420 * PLMC_UNIT;
			let evaluator_bid = BidParams::from((
				evaluator_bidder,
				Retail,
				600 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			));
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let already_bonded_plmc =
				inst.calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0].plmc_amount;

			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;

			let necessary_plmc_for_bid = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			)[0]
			.plmc_amount;

			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_plmc_ed_if_required(vec![evaluator_bidder]);
			inst.mint_funding_asset_ed_if_required(vec![(evaluator_bidder, USDT.id())]);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_bidder,
				necessary_plmc_for_bid.saturating_sub(usable_evaluation_plmc),
			)]);
			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();

			let evaluation_items = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, evaluator_bidder)).collect_vec()
			});
			assert_eq!(evaluation_items.len(), 1);
			assert_eq!(evaluation_items[0].current_plmc_bond, already_bonded_plmc - usable_evaluation_plmc);

			let bid_items = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values(project_id).collect_vec());
			assert_eq!(bid_items.len(), 1);
			assert_eq!(bid_items[0].plmc_bond, necessary_plmc_for_bid);

			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, necessary_plmc_for_bid)],
				HoldReason::Participation.into(),
			);
			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, already_bonded_plmc - usable_evaluation_plmc)],
				HoldReason::Evaluation.into(),
			);
		}

		#[test]
		fn bid_with_multiple_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata_all = default_project_metadata(ISSUER_1);
			project_metadata_all.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.try_into()
					.unwrap();

			let mut project_metadata_usdt = default_project_metadata(ISSUER_2);
			project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

			let mut project_metadata_usdc = default_project_metadata(ISSUER_3);
			project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

			let mut project_metadata_dot = default_project_metadata(ISSUER_4);
			project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

			let evaluations = inst.generate_successful_evaluations(project_metadata_all.clone(), 5);

			let usdt_bid = BidParams::from((
				BIDDER_1,
				Retail,
				1_000 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			));
			let usdc_bid = BidParams::from((
				BIDDER_1,
				Retail,
				1_000 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDC,
			));
			let dot_bid = BidParams::from((
				BIDDER_1,
				Retail,
				1000 * DOT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::DOT,
			));

			let plmc_fundings = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);

			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_fundings.clone());

			let usdt_fundings = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);
			inst.mint_funding_asset_to(usdt_fundings.clone());
			inst.mint_funding_asset_to(usdt_fundings.clone());
			inst.mint_funding_asset_to(usdt_fundings.clone());

			let project_id_all =
				inst.create_auctioning_project(project_metadata_all, ISSUER_1, None, evaluations.clone());
			assert_ok!(inst.bid_for_users(project_id_all, vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()]));

			let project_id_usdt =
				inst.create_auctioning_project(project_metadata_usdt, ISSUER_2, None, evaluations.clone());
			assert_ok!(inst.bid_for_users(project_id_usdt, vec![usdt_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![dot_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);

			let project_id_usdc =
				inst.create_auctioning_project(project_metadata_usdc, ISSUER_3, None, evaluations.clone());
			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_ok!(inst.bid_for_users(project_id_usdc, vec![usdc_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![dot_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);

			let project_id_dot =
				inst.create_auctioning_project(project_metadata_dot, ISSUER_4, None, evaluations.clone());
			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_ok!(inst.bid_for_users(project_id_dot, vec![dot_bid.clone()]));
		}

		fn test_bid_setup(
			inst: &mut MockInstantiator,
			project_id: ProjectId,
			bidder: AccountIdOf<TestRuntime>,
			investor_type: InvestorType,
			u8_multiplier: u8,
		) -> DispatchResultWithPostInfo {
			let project_policy = inst.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
			let jwt = get_mock_jwt_with_cid(bidder, investor_type, generate_did_from_account(BIDDER_1), project_policy);
			let amount = 1000 * USDT_UNIT;
			let mode = ParticipationMode::Classic(u8_multiplier);

			if u8_multiplier > 0 {
				// We cannot use helper functions because some multipliers are invalid
				let necessary_plmc = vec![(bidder, 1_000_000 * PLMC).into()];
				let necessary_usdt = vec![(bidder, 1_000_000 * USDT_UNIT).into()];

				inst.mint_plmc_to(necessary_plmc.clone());
				inst.mint_funding_asset_to(necessary_usdt.clone());
			}
			inst.execute(|| {
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(bidder),
					jwt,
					project_id,
					amount,
					mode,
					AcceptedFundingAsset::USDT,
				)
			})
		}

		#[test]
		fn multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			// Professional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, 0),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
			// Professional bids: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier));
			}
			// Professional bids: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier),
					Error::<TestRuntime>::ForbiddenMultiplier
				);
			}

			// Institutional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, 0),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
			// Institutional bids: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier));
			}
			// Institutional bids: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier),
					Error::<TestRuntime>::ForbiddenMultiplier
				);
			}
		}

		#[test]
		fn bid_split_into_multiple_buckets() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				project_metadata.clone().token_information.decimals,
			)
			.unwrap();

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

			// bid that fills 90% of the first bucket
			let bid_90_percent = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 90u8, 1);

			// This bid fills last 20% of the first bucket,
			// and gets split into 3 more bids of 2 more full and one partially full buckets.
			// 10% + 10% + 10% + 3% = 33%
			let bid_33_percent = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 33u8, 1);

			let all_bids = vec![bid_90_percent[0].clone(), bid_33_percent[0].clone()];

			inst.mint_plmc_ed_if_required(all_bids.accounts());
			inst.mint_funding_asset_ed_if_required(all_bids.to_account_asset_map());

			let necessary_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			let necessary_usdt = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_funding_asset_to(necessary_usdt.clone());

			inst.bid_for_users(project_id, bid_90_percent.clone()).unwrap();
			let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values(project_id).collect_vec());
			assert_eq!(stored_bids.len(), 1);

			inst.bid_for_users(project_id, bid_33_percent.clone()).unwrap();
			let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values(project_id).collect_vec());
			stored_bids.sort_by(|a, b| a.id.cmp(&b.id));
			// 90% + 10% + 10% + 10% + 3% = 5 total bids
			assert_eq!(stored_bids.len(), 5);

			let normalize_price = |decimal_aware_price| {
				PriceProviderOf::<TestRuntime>::convert_back_to_normal_price(
					decimal_aware_price,
					USD_DECIMALS,
					project_metadata.clone().token_information.decimals,
				)
				.unwrap()
			};
			assert_eq!(normalize_price(stored_bids[1].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.0));
			assert_eq!(
				stored_bids[1].original_ct_amount,
				Percent::from_percent(10) * project_metadata.total_allocation_size
			);
			assert_eq!(
				normalize_price(stored_bids[2].original_ct_usd_price),
				PriceOf::<TestRuntime>::from_rational(11, 10)
			);
			assert_eq!(
				stored_bids[2].original_ct_amount,
				Percent::from_percent(10) * project_metadata.total_allocation_size
			);

			assert_eq!(normalize_price(stored_bids[3].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.2));
			assert_eq!(
				stored_bids[3].original_ct_amount,
				Percent::from_percent(10) * project_metadata.total_allocation_size
			);

			assert_eq!(normalize_price(stored_bids[4].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(
				stored_bids[4].original_ct_amount,
				Percent::from_percent(3) * project_metadata.total_allocation_size
			);
			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			assert_eq!(normalize_price(current_bucket.current_price), PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(current_bucket.amount_left, Percent::from_percent(7) * project_metadata.total_allocation_size);
			assert_eq!(normalize_price(current_bucket.delta_price), PriceOf::<TestRuntime>::from_float(0.1));
		}

		#[test]
		fn can_bid_with_frozen_tokens_funding_failed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let bid = BidParams::from((
				BIDDER_4,
				Retail,
				500 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			));
			let plmc_required = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			let frozen_amount = plmc_required[0].plmc_amount;

			inst.mint_plmc_ed_if_required(plmc_required.accounts());
			inst.mint_plmc_to(plmc_required.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &BIDDER_4, plmc_required[0].plmc_amount).unwrap();
			});

			let usdt_required = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(vec![bid.clone()].to_account_asset_map());
			inst.mint_funding_asset_to(usdt_required);

			inst.execute(|| {
				assert_noop!(
					Balances::transfer_allow_death(RuntimeOrigin::signed(BIDDER_4), ISSUER_1, frozen_amount,),
					TokenError::Frozen
				);
			});

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_4),
					get_mock_jwt_with_cid(
						BIDDER_4,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_4),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					bid.amount,
					bid.mode,
					bid.asset
				));
			});

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			inst.execute(|| {
				PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_4), project_id, 0).unwrap();
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Evaluation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed() + frozen_amount);
			assert_eq!(bid_held_balance, Zero::zero());
			assert_eq!(frozen_balance, frozen_amount);
		}

		#[test]
		fn can_bid_with_frozen_tokens_funding_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let mut project_metadata = default_project_metadata(issuer);
			let base_price = PriceOf::<TestRuntime>::from_float(1.0);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let bid = BidParams::from((
				BIDDER_4,
				Retail,
				300_000 * USDT_UNIT,
				ParticipationMode::Classic(5u8),
				AcceptedFundingAsset::USDT,
			));
			let plmc_required = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			let frozen_amount = plmc_required[0].plmc_amount;

			inst.mint_plmc_ed_if_required(plmc_required.accounts());
			inst.mint_plmc_to(plmc_required.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &BIDDER_4, plmc_required[0].plmc_amount).unwrap();
			});

			let usdt_required = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(vec![bid.clone()].to_account_asset_map());
			inst.mint_funding_asset_to(usdt_required);

			inst.execute(|| {
				assert_noop!(
					Balances::transfer_allow_death(RuntimeOrigin::signed(BIDDER_4), ISSUER_1, frozen_amount,),
					TokenError::Frozen
				);
			});

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_4),
					get_mock_jwt_with_cid(
						BIDDER_4,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_4),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					bid.amount,
					bid.mode,
					bid.asset
				));
			});

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			inst.execute(|| {
				PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_4), project_id, 0).unwrap();
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			let vest_duration =
				MultiplierOf::<TestRuntime>::try_from(5u8).unwrap().calculate_vesting_duration::<TestRuntime>();
			let now = inst.current_block();
			inst.jump_to_block(now + vest_duration + 1u64);
			inst.execute(|| {
				assert_ok!(mock::LinearRelease::vest(
					RuntimeOrigin::signed(BIDDER_4),
					HoldReason::Participation.into()
				));
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed() + frozen_amount);
			assert_eq!(bid_held_balance, Zero::zero());
			assert_eq!(frozen_balance, frozen_amount);
		}

		#[test]
		fn one_token_mode_bid_funding_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;

			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.mainnet_token_max_supply = 50_000 * CT_UNIT;
			project_metadata.total_allocation_size = 10_000 * CT_UNIT;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);
			let otm_multiplier: MultiplierOf<TestRuntime> =
				ParticipationMode::OTM.multiplier().try_into().ok().unwrap();
			let otm_duration = otm_multiplier.calculate_vesting_duration::<TestRuntime>();

			let usdt_id = AcceptedFundingAsset::USDT.id();
			const USDT_PARTICIPATION: u128 = 5000 * USDT_UNIT;

			let otm_usdt_fee: u128 = (FeePercentage::get() / ParticipationMode::OTM.multiplier()) * USDT_PARTICIPATION;

			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let required_usdt =
				UserToFundingAsset::new(BIDDER_1, USDT_PARTICIPATION + otm_usdt_fee + usdt_ed, usdt_id.clone());
			inst.mint_funding_asset_to(vec![required_usdt.clone()]);

			// USDT has the same decimals and price as our baseline USD
			let expected_plmc_bond =
				<Pallet<TestRuntime>>::calculate_plmc_bond(USDT_PARTICIPATION, otm_multiplier).unwrap();

			let otm_escrow_account = pallet_proxy_bonding::Pallet::<TestRuntime>::get_bonding_account(project_id);
			let otm_treasury_account = <TestRuntime as pallet_proxy_bonding::Config>::Treasury::get();
			let otm_fee_recipient_account = <TestRuntime as pallet_proxy_bonding::Config>::FeeRecipient::get();
			let funding_project_escrow = PolimecFunding::fund_account_id(project_id);

			assert_ne!(funding_project_escrow, otm_escrow_account);

			let pre_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let pre_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let pre_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let pre_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let pre_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_1),
					get_mock_jwt_with_cid(
						BIDDER_1,
						InvestorType::Professional,
						generate_did_from_account(BIDDER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					USDT_PARTICIPATION,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT
				));
			});

			let post_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let post_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let post_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let post_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);

			assert_eq!(
				post_participation_treasury_free_plmc,
				pre_participation_treasury_free_plmc - expected_plmc_bond - inst.get_ed()
			);
			assert_eq!(
				post_participation_otm_escrow_held_plmc,
				pre_participation_otm_escrow_held_plmc + expected_plmc_bond
			);
			assert_eq!(post_participation_otm_escrow_usdt, pre_participation_otm_escrow_usdt + otm_usdt_fee,);
			assert_eq!(post_participation_otm_fee_recipient_usdt, pre_participation_otm_fee_recipient_usdt,);
			assert_eq!(post_participation_buyer_usdt, pre_participation_buyer_usdt - USDT_PARTICIPATION - otm_usdt_fee);

			let post_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let post_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let post_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let post_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);

			assert_eq!(
				post_participation_treasury_free_plmc,
				pre_participation_treasury_free_plmc - expected_plmc_bond - inst.get_ed()
			);
			assert_eq!(
				post_participation_otm_escrow_held_plmc,
				pre_participation_otm_escrow_held_plmc + expected_plmc_bond
			);
			assert_close_enough!(
				post_participation_otm_escrow_usdt,
				pre_participation_otm_escrow_usdt + otm_usdt_fee,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_otm_fee_recipient_usdt,
				pre_participation_otm_fee_recipient_usdt,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_buyer_usdt,
				pre_participation_buyer_usdt - USDT_PARTICIPATION - otm_usdt_fee,
				Perquintill::from_float(0.999)
			);

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_fees_to_recipient(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into(),
					usdt_id.clone()
				));
				assert_noop!(
					<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
						RuntimeOrigin::signed(BIDDER_1),
						project_id,
						HoldReason::Participation.into()
					),
					pallet_proxy_bonding::Error::<TestRuntime>::TooEarlyToUnlock
				);
			});
			let now = inst.current_block();
			inst.jump_to_block(otm_duration + now);
			inst.execute(|| {
				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into()
				));
			});

			let post_settlement_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_settlement_otm_escrow_held_plmc = inst.get_free_plmc_balance_for(otm_escrow_account);
			let post_settlement_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let post_settlement_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let post_settlement_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);
			let issuer_funding_account = inst.get_free_funding_asset_balance_for(usdt_id, issuer);

			assert_eq!(post_settlement_treasury_free_plmc, post_participation_treasury_free_plmc + expected_plmc_bond);
			assert_eq!(post_settlement_otm_escrow_held_plmc, inst.get_ed());
			assert_eq!(post_settlement_otm_escrow_usdt, Zero::zero());
			assert_close_enough!(post_settlement_otm_fee_recipient_usdt, otm_usdt_fee, Perquintill::from_float(0.999));
			assert_close_enough!(post_settlement_buyer_usdt, usdt_ed, Perquintill::from_float(0.999));
			assert_close_enough!(issuer_funding_account, USDT_PARTICIPATION, Perquintill::from_float(0.999));
		}

		#[test]
		fn one_token_mode_bid_funding_failed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;

			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.mainnet_token_max_supply = 50_000 * CT_UNIT;
			project_metadata.total_allocation_size = 20_000 * CT_UNIT;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);
			let otm_multiplier: MultiplierOf<TestRuntime> =
				ParticipationMode::OTM.multiplier().try_into().ok().unwrap();

			let usdt_id = AcceptedFundingAsset::USDT.id();
			const USDT_PARTICIPATION: u128 = 5000 * USDT_UNIT;

			let otm_usdt_fee: u128 = (FeePercentage::get() / ParticipationMode::OTM.multiplier()) * USDT_PARTICIPATION;
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let required_usdt =
				UserToFundingAsset::new(BIDDER_1, USDT_PARTICIPATION + otm_usdt_fee + usdt_ed, usdt_id.clone());
			inst.mint_funding_asset_to(vec![required_usdt.clone()]);

			// USDT has the same decimals and price as our baseline USD
			let expected_plmc_bond =
				<Pallet<TestRuntime>>::calculate_plmc_bond(USDT_PARTICIPATION, otm_multiplier).unwrap();

			let otm_escrow_account =
				<TestRuntime as pallet_proxy_bonding::Config>::RootId::get().into_sub_account_truncating(project_id);
			let otm_treasury_account = <TestRuntime as pallet_proxy_bonding::Config>::Treasury::get();
			let otm_fee_recipient_account = <TestRuntime as pallet_proxy_bonding::Config>::FeeRecipient::get();
			let funding_project_escrow = PolimecFunding::fund_account_id(project_id);

			assert!(funding_project_escrow != otm_escrow_account);

			let pre_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let pre_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let pre_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let pre_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let pre_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_1),
					get_mock_jwt_with_cid(
						BIDDER_1,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					USDT_PARTICIPATION,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT
				));
			});

			let post_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let post_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let post_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let post_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);

			assert_eq!(
				post_participation_treasury_free_plmc,
				pre_participation_treasury_free_plmc - expected_plmc_bond - inst.get_ed()
			);
			assert_eq!(
				post_participation_otm_escrow_held_plmc,
				pre_participation_otm_escrow_held_plmc + expected_plmc_bond
			);
			assert_close_enough!(
				post_participation_otm_escrow_usdt,
				pre_participation_otm_escrow_usdt + otm_usdt_fee,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_otm_fee_recipient_usdt,
				pre_participation_otm_fee_recipient_usdt,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_buyer_usdt,
				pre_participation_buyer_usdt - USDT_PARTICIPATION - otm_usdt_fee,
				Perquintill::from_float(0.999)
			);

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_noop!(
					<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_fees_to_recipient(
						RuntimeOrigin::signed(BIDDER_1),
						project_id,
						HoldReason::Participation.into(),
						usdt_id.clone()
					),
					pallet_proxy_bonding::Error::<TestRuntime>::FeeToRecipientDisallowed
				);

				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into()
				));
			});

			let post_settlement_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_settlement_otm_escrow_held_plmc = inst.get_free_plmc_balance_for(otm_escrow_account);
			let post_settlement_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_escrow_account);
			let post_settlement_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(usdt_id.clone(), otm_fee_recipient_account);
			let post_settlement_buyer_usdt = inst.get_free_funding_asset_balance_for(usdt_id.clone(), BIDDER_1);
			let issuer_funding_account = inst.get_free_funding_asset_balance_for(usdt_id, issuer);

			assert_eq!(post_settlement_treasury_free_plmc, post_participation_treasury_free_plmc + expected_plmc_bond);
			assert_eq!(post_settlement_otm_escrow_held_plmc, inst.get_ed());
			assert_eq!(post_settlement_otm_escrow_usdt, Zero::zero());
			assert_eq!(post_settlement_otm_fee_recipient_usdt, Zero::zero());
			assert_eq!(post_settlement_buyer_usdt, usdt_ed + USDT_PARTICIPATION + otm_usdt_fee);
			assert_eq!(issuer_funding_account, Zero::zero());
		}

		#[test]
		fn bid_on_ethereum_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participants_account_type = ParticipantsAccountType::Ethereum;

			let mut eth_evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			for eval in &mut eth_evaluations {
				let mut key = [0u8; 20];
				key[..8].copy_from_slice(&eval.account.to_le_bytes());
				eval.receiving_account = Junction::AccountKey20 { network: None, key };
			}

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, eth_evaluations.clone());
			let jwt = get_mock_jwt_with_cid(
				BIDDER_1,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			let (eth_acc, eth_sig) = inst.eth_key_and_sig_from("//BIDDER1", project_id, BIDDER_1);
			let bid = BidParams::from((
				BIDDER_1,
				Retail,
				500 * USDT_UNIT,
				ParticipationMode::OTM,
				AcceptedFundingAsset::USDT,
			));
			let mint_amount = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(mint_amount.to_account_asset_map());
			inst.mint_funding_asset_to(mint_amount.clone());

			assert_ok!(inst.execute(|| {
				PolimecFunding::bid_with_receiving_account(
					RuntimeOrigin::signed(BIDDER_1),
					jwt,
					project_id,
					500 * USDT_UNIT,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT,
					eth_acc,
					eth_sig,
				)
			}));
		}

		#[test]
		fn bid_with_different_receiver_polkadot_account() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			let jwt = get_mock_jwt_with_cid(
				BIDDER_1,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			let (dot_acc, dot_sig) = inst.dot_key_and_sig_from("//BIDDER1", project_id, BIDDER_1);
			let bid = BidParams::from((
				BIDDER_1,
				Retail,
				500 * USDT_UNIT,
				ParticipationMode::OTM,
				AcceptedFundingAsset::USDT,
			));
			let mint_amount = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(mint_amount.to_account_asset_map());
			inst.mint_funding_asset_to(mint_amount.clone());

			assert_ok!(inst.execute(|| {
				PolimecFunding::bid_with_receiving_account(
					RuntimeOrigin::signed(BIDDER_1),
					jwt,
					project_id,
					500 * USDT_UNIT,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT,
					dot_acc,
					dot_sig,
				)
			}));
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_use_all_of_evaluation_bond_on_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let evaluator_bidder = 69;
			let evaluation_amount = 420 * PLMC_UNIT;
			let evaluator_bid = BidParams::from((
				evaluator_bidder,
				Retail,
				600 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			));
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			assert_err!(inst.bid_for_users(project_id, vec![evaluator_bid]), TokenError::NotExpendable);
		}

		#[test]
		fn cannot_use_evaluation_bond_on_another_project_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			let mut evaluations_1 = inst.generate_successful_evaluations(project_metadata_1.clone(), 5);
			let evaluations_2 = evaluations_1.clone();

			let evaluator_bidder = 69;
			let evaluation_amount = 420 * PLMC_UNIT;
			let evaluator_bid = BidParams::from((
				evaluator_bidder,
				Retail,
				600 * USDT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			));
			evaluations_1.push((evaluator_bidder, evaluation_amount).into());

			let _project_id_1 =
				inst.create_auctioning_project(project_metadata_1.clone(), ISSUER_1, None, evaluations_1);
			let project_id_2 =
				inst.create_auctioning_project(project_metadata_2.clone(), ISSUER_2, None, evaluations_2);

			// Necessary Mints
			let already_bonded_plmc =
				inst.calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0].plmc_amount;
			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_bid = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata_2.minimum_price,
			)[0]
			.plmc_amount;
			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata_2.minimum_price,
			);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_bidder,
				necessary_plmc_for_bid.saturating_sub(usable_evaluation_plmc),
			)]);
			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(evaluator_bidder),
						get_mock_jwt_with_cid(
							evaluator_bidder,
							InvestorType::Professional,
							generate_did_from_account(evaluator_bidder),
							project_metadata_2.clone().policy_ipfs_cid.unwrap()
						),
						project_id_2,
						evaluator_bid.amount,
						evaluator_bid.mode,
						evaluator_bid.asset
					),
					TokenError::FundsUnavailable
				);
			});
		}

		#[test]
		fn cannot_bid_before_auction_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let _ = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			let did = generate_did_from_account(BIDDER_2);
			let investor_type = InvestorType::Institutional;

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_2,
						project_id: 0,
						funding_asset_amount: 1,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did,
						investor_type,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
						receiving_account: polkadot_junction!(BIDDER_2)
					}),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * CT_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, None),
				institutional: TicketSize::new(20_000 * USD_UNIT, None),
				retail: TicketSize::new(100 * USD_UNIT, None),
				phantom: Default::default(),
			};

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![(BIDDER_1, 50_000 * CT_UNIT).into(), (BIDDER_2, 50_000 * CT_UNIT).into()]);

			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 50_000 * USD_UNIT).into(),
				(BIDDER_2, 50_000 * USD_UNIT).into(),
			]);

			// bid below 800 CT (8k USD) should fail for professionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_1,
						project_id,
						funding_asset_amount: 7_999 * USDT_UNIT,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did: generate_did_from_account(BIDDER_1),
						investor_type: InvestorType::Professional,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
						receiving_account: polkadot_junction!(BIDDER_1)
					}),
					Error::<TestRuntime>::TooLow
				);
			});
			// bid below 2000 CT (20k USD) should fail for institutionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_2,
						project_id,
						funding_asset_amount: 19_999 * USDT_UNIT,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did: generate_did_from_account(BIDDER_1),
						investor_type: InvestorType::Institutional,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
						receiving_account: polkadot_junction!(BIDDER_2)
					}),
					Error::<TestRuntime>::TooLow
				);
			});
		}

		#[test]
		fn ticket_size_minimums_use_current_bucket_price() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 50_000 * CT_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, None),
				institutional: TicketSize::new(20_000 * USD_UNIT, None),
				retail: TicketSize::new(100 * USD_UNIT, None),
				phantom: Default::default(),
			};
			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				project_metadata.clone().token_information.decimals,
			)
			.unwrap();

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 200_000 * PLMC).into(),
				(BIDDER_2, 200_000 * PLMC).into(),
				(BIDDER_3, 200_000 * PLMC).into(),
			]);
			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 200_000 * USDT_UNIT).into(),
				(BIDDER_2, 200_000 * USDT_UNIT).into(),
				(BIDDER_3, 200_000 * USDT_UNIT).into(),
			]);

			// First bucket is covered by one bidder
			let big_bid: BidParams<TestRuntime> = (BIDDER_1, 50_000 * USDT_UNIT).into();
			inst.bid_for_users(project_id, vec![big_bid.clone()]).unwrap();

			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: BIDDER_2,
					project_id,
					funding_asset_amount: 8001 * USD_UNIT,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(BIDDER_1),
					investor_type: InvestorType::Professional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					receiving_account: polkadot_junction!(BIDDER_2)
				}));
			});

			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: BIDDER_3,
					project_id,
					funding_asset_amount: 20_001 * USD_UNIT,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(BIDDER_1),
					investor_type: InvestorType::Institutional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					receiving_account: polkadot_junction!(BIDDER_3)
				}));
			});
		}

		#[test]
		fn per_credential_type_ticket_size_maximums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, Some(17_000 * USD_UNIT)),
				institutional: TicketSize::new(20_000 * USD_UNIT, Some(500_000 * USD_UNIT)),
				retail: TicketSize::new(100 * USD_UNIT, None),
				phantom: Default::default(),
			};
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 500_000 * CT_UNIT).into(),
				(BIDDER_2, 500_000 * CT_UNIT).into(),
				(BIDDER_3, 500_000 * CT_UNIT).into(),
				(BIDDER_4, 500_000 * CT_UNIT).into(),
			]);

			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 500_000 * USD_UNIT).into(),
				(BIDDER_2, 500_000 * USD_UNIT).into(),
				(BIDDER_3, 500_000 * USD_UNIT).into(),
				(BIDDER_4, 500_000 * USD_UNIT).into(),
			]);

			let bidder_1_jwt = get_mock_jwt_with_cid(
				BIDDER_1,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let bidder_2_jwt_same_did = get_mock_jwt_with_cid(
				BIDDER_2,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total bids with same DID above 10k CT (100k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_1),
					bidder_1_jwt,
					project_id,
					8_000 * USDT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_2),
						bidder_2_jwt_same_did.clone(),
						project_id,
						10_000 * USDT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::TooHigh
				);
			});
			// bidding 17k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_2),
					bidder_2_jwt_same_did,
					project_id,
					9_000 * USDT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});

			let bidder_3_jwt = get_mock_jwt_with_cid(
				BIDDER_3,
				InvestorType::Institutional,
				generate_did_from_account(BIDDER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let bidder_4_jwt_same_did = get_mock_jwt_with_cid(
				BIDDER_4,
				InvestorType::Institutional,
				generate_did_from_account(BIDDER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total bids with same DID above 50k CT (500k USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_3),
					bidder_3_jwt,
					project_id,
					40_000 * USDT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_4),
						bidder_4_jwt_same_did.clone(),
						project_id,
						500_001 * USDT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::TooHigh
				);
			});
			// bidding 60k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_4),
					bidder_4_jwt_same_did,
					project_id,
					20_000 * USDT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn issuer_cannot_bid_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: ISSUER_1,
					project_id,
					funding_asset_amount: 5000 * USDT_UNIT,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(ISSUER_1),
					investor_type: InvestorType::Professional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					receiving_account: polkadot_junction!(ISSUER_1)
				})),
				Error::<TestRuntime>::ParticipationToOwnProject
			);
		}

		#[test]
		fn bid_with_asset_not_accepted() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies = bounded_vec![USDT];

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			let bids = [BidParams::<TestRuntime>::from((
				BIDDER_1,
				Retail,
				10_000,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDC,
			))];

			let did = generate_did_from_account(bids[0].bidder);
			let investor_type = InvestorType::Institutional;

			let outcome = inst.execute(|| {
				Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: bids[0].bidder,
					project_id,
					funding_asset_amount: bids[0].amount,
					mode: bids[0].mode,
					funding_asset: bids[0].asset,
					did,
					investor_type,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					receiving_account: polkadot_junction!(bids[0].bidder),
				})
			});
			frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
		}

		#[test]
		fn wrong_policy_on_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(BIDDER_1),
						get_mock_jwt_with_cid(
							BIDDER_1,
							InvestorType::Professional,
							generate_did_from_account(BIDDER_1),
							"wrong_cid".as_bytes().to_vec().try_into().unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::PolicyMismatch
				);
			});
		}

		#[test]
		fn bid_after_end_block_before_transitioning_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			let end_block = inst.get_project_details(project_id).round_duration.end.unwrap();
			inst.jump_to_block(end_block + 1);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(BIDDER_1),
						get_mock_jwt_with_cid(
							BIDDER_1,
							InvestorType::Professional,
							generate_did_from_account(BIDDER_1),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}

#[cfg(test)]
mod end_auction_extrinsic {
	use super::*;

	// Partial acceptance at price <= wap (refund due to less CT bought)
	// Rejection due to no more tokens left (full refund)
	#[test]
	fn bids_get_rejected_and_refunded() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let issuer = ISSUER_1;
		let mut project_metadata = default_project_metadata(issuer);
		project_metadata.total_allocation_size = 500_000 * CT_UNIT;
		project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
		project_metadata.minimum_price = ConstPriceProvider::calculate_decimals_aware_price(
			FixedU128::from_float(10.0f64),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap();
		project_metadata.participation_currencies =
			bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT];

		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

		// We use multiplier > 1 so after settlement, only the refunds defined above are done. The rest will be done
		// through the linear release pallet
		let bid_1 = BidParams::from((
			BIDDER_1,
			Retail,
			500 * USDT_UNIT,
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::USDT,
		));
		let bid_2 = BidParams::from((
			BIDDER_2,
			Institutional,
			4_00 * USDT_UNIT,
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::USDC,
		));
		let bid_3 = BidParams::from((
			BIDDER_5,
			Professional,
			100 * DOT_UNIT,
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::DOT,
		));
		let bid_4 = BidParams::from((
			BIDDER_3,
			Retail,
			600 * USDT_UNIT,
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::USDT,
		));
		let bid_5 = BidParams::from((
			BIDDER_4,
			Retail,
			10 * DOT_UNIT,
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::DOT,
		));
		// post bucketing, the bids look like this:
		// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_5, 5k)   - (BIDDER_5, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
		// | -------------------- 10 USD ----------------------|---- 11 USD ---|---- 12 USD ----|----------- 13 USD -------------|
		// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

		let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

		let plmc_amounts = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);
		let funding_asset_amounts = vec![
			UserToFundingAsset::new(BIDDER_1, bids[0].amount, AcceptedFundingAsset::USDT.id()),
			UserToFundingAsset::new(BIDDER_2, bids[1].amount, AcceptedFundingAsset::USDC.id()),
			UserToFundingAsset::new(BIDDER_3, bids[3].amount, AcceptedFundingAsset::USDT.id()),
			UserToFundingAsset::new(BIDDER_4, bids[4].amount, AcceptedFundingAsset::DOT.id()),
			UserToFundingAsset::new(BIDDER_5, bids[2].amount, AcceptedFundingAsset::DOT.id()),
		];

		inst.mint_plmc_ed_if_required(bids.accounts());
		inst.mint_funding_asset_ed_if_required(bids.to_account_asset_map());

		let prev_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());
		let prev_funding_asset_balances = inst.get_free_funding_asset_balances_for(bids.to_account_asset_map());

		inst.mint_plmc_to(plmc_amounts.clone());
		inst.mint_funding_asset_to(funding_asset_amounts.clone());

		inst.bid_for_users(project_id, bids.clone()).unwrap();

		inst.do_free_plmc_assertions(vec![
			UserToPLMCBalance::new(BIDDER_1, inst.get_ed()),
			UserToPLMCBalance::new(BIDDER_2, inst.get_ed()),
		]);
		inst.do_reserved_plmc_assertions(plmc_amounts.clone(), HoldReason::Participation.into());

		assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful));

		let bidder_5_rejected_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 2)).unwrap();
		let _bidder_5_accepted_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 3)).unwrap();
		let bidder_5_plmc_pre_balance = inst.get_free_plmc_balance_for(bidder_5_rejected_bid.bidder);
		let bidder_5_funding_asset_pre_balance = inst
			.get_free_funding_asset_balance_for(bidder_5_rejected_bid.funding_asset.id(), bidder_5_rejected_bid.bidder);

		assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success)));
		inst.settle_project(project_id, true);

		let returned_auction_plmc =
			inst.calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone());
		let returned_funding_assets =
			inst.calculate_auction_funding_asset_returned_from_all_bids_made(&bids, project_metadata.clone());

		let expected_free_plmc =
			inst.generic_map_operation(vec![returned_auction_plmc.clone(), prev_plmc_balances], MergeOperation::Add);
		let expected_free_funding_assets = inst.generic_map_operation(
			vec![returned_funding_assets.clone(), prev_funding_asset_balances],
			MergeOperation::Add,
		);
		let expected_reserved_plmc = inst
			.generic_map_operation(vec![plmc_amounts.clone(), returned_auction_plmc.clone()], MergeOperation::Subtract);
		let expected_final_funding_spent = inst.generic_map_operation(
			vec![funding_asset_amounts.clone(), returned_funding_assets.clone()],
			MergeOperation::Subtract,
		);
		let expected_issuer_funding = inst.sum_funding_asset_mappings(vec![expected_final_funding_spent]);

		// Assertions about rejected bid
		let bidder_5_plmc_post_balance = inst.get_free_plmc_balance_for(bidder_5_rejected_bid.bidder);
		let bidder_5_funding_asset_post_balance = inst
			.get_free_funding_asset_balance_for(bidder_5_rejected_bid.funding_asset.id(), bidder_5_rejected_bid.bidder);

		// Bidder 5's accepted bid should have some refunds due to paying the wap in the end instead of the bucket price.
		// Bidder 5's rejected bid should have a full refund
		let bidder_5_returned_plmc = returned_auction_plmc.iter().find(|x| x.account == BIDDER_5).unwrap().plmc_amount;
		let bidder_5_returned_funding_asset =
			returned_funding_assets.iter().find(|x| x.account == BIDDER_5).unwrap().asset_amount;

		assert!(inst.execute(|| Bids::<TestRuntime>::get(project_id, 2)).is_none());
		assert_eq!(bidder_5_plmc_post_balance, bidder_5_plmc_pre_balance + bidder_5_returned_plmc);
		assert_eq!(
			bidder_5_funding_asset_post_balance,
			bidder_5_funding_asset_pre_balance + bidder_5_returned_funding_asset
		);

		inst.do_free_plmc_assertions(expected_free_plmc);
		inst.do_reserved_plmc_assertions(expected_reserved_plmc, HoldReason::Participation.into());
		inst.do_free_funding_asset_assertions(expected_free_funding_assets);

		for (asset, expected_amount) in expected_issuer_funding {
			let real_amount = inst.get_free_funding_asset_balance_for(asset, ISSUER_1);
			assert_eq!(real_amount, expected_amount);
		}
	}

	#[test]
	fn oversubscribed_bid_can_get_refund_and_bid_again_in_auction_round() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(ISSUER_1);
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 100, 10);

		let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

		inst.mint_necessary_tokens_for_bids(project_id, bids.clone());
		inst.bid_for_users(project_id, bids.clone()).unwrap();

		// First bid is OTM
		let first_bid_to_refund = bids[9].clone();
		// Second is classic
		let second_bid_to_refund = bids[8].clone();

		let oversubscribing_bids = vec![
			(
				BIDDER_1,
				Retail,
				first_bid_to_refund.amount + 200 * CT_UNIT,
				ParticipationMode::Classic(1),
				AcceptedFundingAsset::USDT,
			)
				.into(),
			(BIDDER_2, Retail, second_bid_to_refund.amount, ParticipationMode::Classic(1), AcceptedFundingAsset::USDT)
				.into(),
		];

		inst.mint_necessary_tokens_for_bids(project_id, oversubscribing_bids.clone());
		inst.bid_for_users(project_id, vec![oversubscribing_bids[0].clone()]).unwrap();

		inst.process_oversubscribed_bids(project_id);

		let pre_first_refund_bidder_plmc_balance = inst.get_free_plmc_balance_for(first_bid_to_refund.bidder);
		let pre_first_refund_bidder_funding_asset_balance =
			inst.get_free_funding_asset_balance_for(first_bid_to_refund.asset.id(), first_bid_to_refund.bidder);

		let first_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 9)).unwrap();
		assert!(matches!(first_bid.status, BidStatus::Rejected));
		let second_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 8)).unwrap();
		assert!(matches!(second_bid.status, BidStatus::PartiallyAccepted(_)));

		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_settle_bid(project_id, 9));
			assert_noop!(
				Pallet::<TestRuntime>::do_settle_bid(project_id, 8),
				Error::<TestRuntime>::SettlementNotStarted
			);
		});

		let post_first_refund_bidder_plmc_balance = inst.get_free_plmc_balance_for(first_bid_to_refund.bidder);
		let post_first_refund_bidder_funding_asset_balance =
			inst.get_free_funding_asset_balance_for(first_bid_to_refund.asset.id(), first_bid_to_refund.bidder);

		// OTM bid doesnt give PLMC refund to bidder
		assert_eq!(post_first_refund_bidder_plmc_balance, pre_first_refund_bidder_plmc_balance);
		let mut funding_asset_refund = first_bid.funding_asset_amount_locked;
		let usd_ticket = first_bid.original_ct_usd_price.saturating_mul_int(first_bid.original_ct_amount);
		inst.add_otm_fee_to(&mut funding_asset_refund, usd_ticket, first_bid_to_refund.asset);
		assert_eq!(
			post_first_refund_bidder_funding_asset_balance,
			pre_first_refund_bidder_funding_asset_balance + funding_asset_refund
		);

		inst.bid_for_users(project_id, vec![oversubscribing_bids[1].clone()]).unwrap();

		inst.process_oversubscribed_bids(project_id);
		let pre_second_refund_bidder_plmc_balance = inst.get_free_plmc_balance_for(second_bid_to_refund.bidder);
		let pre_second_refund_bidder_funding_asset_balance =
			inst.get_free_funding_asset_balance_for(second_bid_to_refund.asset.id(), second_bid_to_refund.bidder);

		let second_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 8)).unwrap();
		assert!(matches!(second_bid.status, BidStatus::Rejected));
		let third_bid = inst.execute(|| Bids::<TestRuntime>::get(project_id, 7)).unwrap();
		assert!(matches!(third_bid.status, BidStatus::PartiallyAccepted(_)));

		inst.execute(|| {
			assert_ok!(Pallet::<TestRuntime>::do_settle_bid(project_id, 8));
			assert_noop!(
				Pallet::<TestRuntime>::do_settle_bid(project_id, 7),
				Error::<TestRuntime>::SettlementNotStarted
			);
		});

		let post_second_refund_bidder_plmc_balance = inst.get_free_plmc_balance_for(second_bid_to_refund.bidder);
		let post_second_refund_bidder_funding_asset_balance =
			inst.get_free_funding_asset_balance_for(second_bid_to_refund.asset.id(), second_bid_to_refund.bidder);

		// Classic bid
		assert_eq!(
			post_second_refund_bidder_plmc_balance,
			pre_second_refund_bidder_plmc_balance + second_bid.plmc_bond
		);
		assert_eq!(
			post_second_refund_bidder_funding_asset_balance,
			pre_second_refund_bidder_funding_asset_balance + second_bid.funding_asset_amount_locked
		);
	}
}
