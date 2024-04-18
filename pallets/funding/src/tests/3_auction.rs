use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use sp_core::bounded_vec;
		use std::ops::Not;

		#[test]
		fn auction_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let bids = default_bids();
			let _project_id = inst.create_community_contributing_project(project_metadata, ISSUER_1, evaluations, bids);
		}

		#[test]
		fn multiple_auction_projects_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(ISSUER_1);
			let project2 = default_project_metadata(ISSUER_2);
			let project3 = default_project_metadata(ISSUER_3);
			let project4 = default_project_metadata(ISSUER_4);
			let evaluations = default_evaluations();
			let bids = default_bids();

			inst.create_community_contributing_project(project1, ISSUER_1, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project2, ISSUER_2, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project3, ISSUER_3, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project4, ISSUER_4, evaluations, bids);
		}

		#[test]
		fn wap_is_accurate() {
			// From the knowledge hub: https://hub.polimec.org/learn/calculation-example#auction-round-calculation-example
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			const ADAM: u32 = 60;
			const TOM: u32 = 61;
			const SOFIA: u32 = 62;
			const FRED: u32 = 63;
			const ANNA: u32 = 64;
			const DAMIAN: u32 = 65;

			let accounts = vec![ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];

			let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
			let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
			let metadata_hash = hashed(format!("{}-{}", METADATA, 0));
			let project_metadata = ProjectMetadata {
				token_information: CurrencyMetadata {
					name: bounded_name,
					symbol: bounded_symbol,
					decimals: ASSET_DECIMALS,
				},
				mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
				total_allocation_size: 100_000 * ASSET_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(50u8),
				minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
					institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
					phantom: Default::default(),
				},
				contributing_ticket_sizes: ContributingTicketSizes {
					retail: TicketSize::new(None, None),
					professional: TicketSize::new(None, None),
					institutional: TicketSize::new(None, None),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
				funding_destination_account: ISSUER_1,
				offchain_information_hash: Some(metadata_hash),
			};

			// overfund with plmc
			let plmc_fundings = accounts
				.iter()
				.map(|acc| UserToPLMCBalance { account: acc.clone(), plmc_amount: PLMC * 1_000_000 })
				.collect_vec();
			let usdt_fundings = accounts
				.iter()
				.map(|acc| UserToForeignAssets {
					account: acc.clone(),
					asset_amount: US_DOLLAR * 1_000_000,
					asset_id: AcceptedFundingAsset::USDT.to_assethub_id(),
				})
				.collect_vec();
			inst.mint_plmc_to(plmc_fundings);
			inst.mint_foreign_asset_to(usdt_fundings);

			let project_id = inst.create_auctioning_project(project_metadata, ISSUER_1, default_evaluations());

			let bids = vec![
				(ADAM, 10_000 * ASSET_UNIT).into(),
				(TOM, 20_000 * ASSET_UNIT).into(),
				(SOFIA, 20_000 * ASSET_UNIT).into(),
				(FRED, 10_000 * ASSET_UNIT).into(),
				(ANNA, 5_000 * ASSET_UNIT).into(),
				(DAMIAN, 5_000 * ASSET_UNIT).into(),
			];

			inst.bid_for_users(project_id, bids).unwrap();

			inst.start_community_funding(project_id).unwrap();

			let token_price =
				inst.get_project_details(project_id).weighted_average_price.unwrap().saturating_mul_int(ASSET_UNIT);

			let desired_price = PriceOf::<TestRuntime>::from_float(11.1818f64).saturating_mul_int(ASSET_UNIT);

			assert_close_enough!(token_price, desired_price, Perquintill::from_float(0.99));
		}

		#[test]
		fn bids_at_higher_price_than_weighted_average_use_average() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let mut bids: Vec<BidParams<_>> = MockInstantiator::generate_bids_from_total_usd(
				project_metadata.minimum_price.saturating_mul_int(
					project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
				),
				project_metadata.minimum_price,
				default_weights(),
				default_bidders(),
				default_bidder_multipliers(),
			);

			let second_bucket_bid = (BIDDER_6, 500 * ASSET_UNIT).into();
			bids.push(second_bucket_bid);

			let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
			let bidder_5_bid =
				inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, BIDDER_6)).next().unwrap());
			let wabgp = inst.get_project_details(project_id).weighted_average_price.unwrap();
			assert_eq!(bidder_5_bid.original_ct_usd_price.to_float(), 11.0);
			assert_eq!(bidder_5_bid.final_ct_usd_price, wabgp);
		}

		#[test]
		fn auction_gets_percentage_of_ct_total_allocation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let auction_percentage = project_metadata.auction_round_allocation_percentage;
			let total_allocation = project_metadata.total_allocation_size;

			let auction_allocation = auction_percentage * total_allocation;

			let bids = vec![(BIDDER_1, auction_allocation).into()];
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				evaluations.clone(),
				bids,
			);
			let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
			let bid_info = inst.execute(|| bid_infos.next().unwrap());
			assert!(inst.execute(|| bid_infos.next().is_none()));
			assert_eq!(bid_info.final_ct_amount, auction_allocation);

			let project_metadata = default_project_metadata(ISSUER_2);
			let bids = vec![(BIDDER_1, auction_allocation).into(), (BIDDER_1, 1000 * ASSET_UNIT).into()];
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_2,
				evaluations.clone(),
				bids,
			);
			let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
			let bid_info_1 = inst.execute(|| bid_infos.next().unwrap());
			let bid_info_2 = inst.execute(|| bid_infos.next().unwrap());
			assert!(inst.execute(|| bid_infos.next().is_none()));
			assert_eq!(
				bid_info_1.final_ct_amount + bid_info_2.final_ct_amount,
				auction_allocation,
				"Should not be able to buy more than auction allocation"
			);
		}

		// Partial acceptance at price <= wap (refund due to less CT bought)
		// Full Acceptance at price > wap (refund due to final price lower than original price paid)
		// Rejection due to no more tokens left (full refund)
		#[test]
		fn bids_get_rejected_and_refunded_part_one() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();

			let bid_1 = BidParams::new(BIDDER_1, 5000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let bid_2 = BidParams::new(BIDDER_2, 40_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let bid_3 = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let bid_4 = BidParams::new(BIDDER_3, 6000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let bid_5 = BidParams::new(BIDDER_4, 2000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			// post bucketing, the bids look like this:
			// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_1, 5k) - (BIDDER_1, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
			// | -------------------- 1USD ----------------------|---- 1.1 USD ---|---- 1.2 USD ----|----------- 1.3 USD -------------|
			// post wap ~ 1.0557252:
			// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

			let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
			let usdt_fundings =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&bids,
					project_metadata.clone(),
					None,
				);

			let plmc_existential_amounts = plmc_fundings.accounts().existential_deposits();

			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_existential_amounts.clone());
			inst.mint_foreign_asset_to(usdt_fundings.clone());

			inst.bid_for_users(project_id, bids.clone()).unwrap();

			inst.do_free_plmc_assertions(vec![
				UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
			]);
			inst.do_reserved_plmc_assertions(plmc_fundings.clone(), HoldReason::Participation(project_id).into());
			inst.do_bid_transferred_foreign_asset_assertions(usdt_fundings.clone(), project_id);

			inst.start_community_funding(project_id).unwrap();

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let returned_auction_plmc = MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(
				&bids,
				project_metadata.clone(),
				wap,
			);
			let returned_funding_assets = MockInstantiator::calculate_auction_funding_asset_returned_from_all_bids_made(
				&bids,
				project_metadata,
				wap,
			);

			let expected_free_plmc = MockInstantiator::generic_map_operation(
				vec![returned_auction_plmc.clone(), plmc_existential_amounts],
				MergeOperation::Add,
			);
			let expected_free_funding_assets =
				MockInstantiator::generic_map_operation(vec![returned_funding_assets.clone()], MergeOperation::Add);
			let expected_reserved_plmc = MockInstantiator::generic_map_operation(
				vec![plmc_fundings.clone(), returned_auction_plmc],
				MergeOperation::Subtract,
			);
			let expected_held_funding_assets = MockInstantiator::generic_map_operation(
				vec![usdt_fundings.clone(), returned_funding_assets],
				MergeOperation::Subtract,
			);

			inst.do_free_plmc_assertions(expected_free_plmc);

			inst.do_reserved_plmc_assertions(expected_reserved_plmc, HoldReason::Participation(project_id).into());

			inst.do_free_foreign_asset_assertions(expected_free_funding_assets);
			inst.do_bid_transferred_foreign_asset_assertions(expected_held_funding_assets, project_id);
		}

		#[test]
		// Partial acceptance at price > wap (refund due to less CT bought, and final price lower than original price paid)
		// Rejection due to bid being made after random end (full refund)
		fn bids_get_rejected_and_refunded_part_two() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, default_evaluations());

			let total_auction_ct_amount =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

			let full_ct_bid_rejected =
				BidParams::new(BIDDER_1, total_auction_ct_amount, 1u8, AcceptedFundingAsset::USDT);
			let full_ct_bid_partially_accepted =
				BidParams::new(BIDDER_2, total_auction_ct_amount, 1u8, AcceptedFundingAsset::USDT);
			let oversubscription_bid = BidParams::new(BIDDER_3, 100_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let after_random_end_bid = BidParams::new(BIDDER_4, 100_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

			let all_bids = vec![
				full_ct_bid_rejected.clone(),
				full_ct_bid_partially_accepted.clone(),
				oversubscription_bid.clone(),
				after_random_end_bid.clone(),
			];
			let all_included_bids =
				vec![full_ct_bid_rejected.clone(), full_ct_bid_partially_accepted.clone(), oversubscription_bid];

			let necessary_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			let plmc_existential_amounts = necessary_plmc.accounts().existential_deposits();
			let necessary_usdt =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&all_bids,
					project_metadata.clone(),
					None,
				);

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_amounts.clone());
			inst.mint_foreign_asset_to(necessary_usdt.clone());
			inst.bid_for_users(project_id, all_included_bids.clone()).unwrap();
			inst.advance_time(
				<TestRuntime as Config>::AuctionOpeningDuration::get() +
					<TestRuntime as Config>::AuctionClosingDuration::get() -
					1,
			)
			.unwrap();

			inst.bid_for_users(project_id, vec![after_random_end_bid]).unwrap();
			inst.do_free_plmc_assertions(vec![
				UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_3, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_4, MockInstantiator::get_ed()),
			]);
			inst.do_reserved_plmc_assertions(necessary_plmc.clone(), HoldReason::Participation(project_id).into());
			inst.do_bid_transferred_foreign_asset_assertions(necessary_usdt.clone(), project_id);
			inst.start_community_funding(project_id).unwrap();

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let plmc_returned = MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(
				&all_included_bids,
				project_metadata.clone(),
				wap,
			);
			let usdt_returned = MockInstantiator::calculate_auction_funding_asset_returned_from_all_bids_made(
				&all_included_bids,
				project_metadata.clone(),
				wap,
			);

			let rejected_bid_necessary_plmc = &necessary_plmc[3];
			let rejected_bid_necessary_usdt = &necessary_usdt[3];

			let expected_free = MockInstantiator::generic_map_operation(
				vec![plmc_returned.clone(), plmc_existential_amounts, vec![rejected_bid_necessary_plmc.clone()]],
				MergeOperation::Add,
			);
			inst.do_free_plmc_assertions(expected_free);
			let expected_reserved = MockInstantiator::generic_map_operation(
				vec![necessary_plmc.clone(), plmc_returned.clone(), vec![rejected_bid_necessary_plmc.clone()]],
				MergeOperation::Subtract,
			);
			inst.do_reserved_plmc_assertions(expected_reserved, HoldReason::Participation(project_id).into());
			let expected_reserved = MockInstantiator::generic_map_operation(
				vec![necessary_usdt.clone(), usdt_returned.clone(), vec![rejected_bid_necessary_usdt.clone()]],
				MergeOperation::Subtract,
			);
			inst.do_bid_transferred_foreign_asset_assertions(expected_reserved, project_id);
		}

		#[test]
		fn no_bids_made() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let details = inst.get_project_details(project_id);
			let opening_end = details.phase_transition_points.auction_opening.end().unwrap();
			let now = inst.current_block();
			inst.advance_time(opening_end - now + 2).unwrap();

			let details = inst.get_project_details(project_id);
			let closing_end = details.phase_transition_points.auction_closing.end().unwrap();
			let now = inst.current_block();
			inst.advance_time(closing_end - now + 2).unwrap();

			let details = inst.get_project_details(project_id);
			assert_eq!(details.status, ProjectStatus::CommunityRound);
			assert_eq!(details.weighted_average_price, Some(project_metadata.minimum_price));
		}

		#[test]
		fn all_bids_rejected() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let bids = default_bids();
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let necessary_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
			let plmc_existential_amounts = necessary_plmc.accounts().existential_deposits();
			let necessary_usdt =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&bids,
					project_metadata.clone(),
					None,
				);

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_amounts.clone());
			inst.mint_foreign_asset_to(necessary_usdt.clone());
			inst.advance_time(
				<TestRuntime as Config>::AuctionOpeningDuration::get() +
					<TestRuntime as Config>::AuctionClosingDuration::get() -
					1,
			)
			.unwrap();

			// We bid at the last block, which we assume will be after the random end
			inst.bid_for_users(project_id, bids.clone()).unwrap();

			inst.start_community_funding(project_id).unwrap();

			let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
			let non_rejected_bids = stored_bids
				.into_iter()
				.filter(|bid| {
					(bid.final_ct_amount == 0 && bid.status == BidStatus::Rejected(RejectionReason::AfterClosingEnd))
						.not()
				})
				.count();
			assert_eq!(non_rejected_bids, 0);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);
		}

		#[test]
		fn wap_from_different_funding_assets() {
			// From the knowledge hub: https://hub.polimec.org/learn/calculation-example#auction-round-calculation-example
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			const ADAM: u32 = 60;
			const TOM: u32 = 61;
			const SOFIA: u32 = 62;
			const FRED: u32 = 63;
			const ANNA: u32 = 64;
			const DAMIAN: u32 = 65;

			let accounts = vec![ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * ASSET_UNIT;
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT,];

			// overfund with plmc
			let plmc_fundings = accounts
				.iter()
				.map(|acc| UserToPLMCBalance { account: acc.clone(), plmc_amount: PLMC * 1_000_000 })
				.collect_vec();

			let fundings = [AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT];
			assert_eq!(fundings.len(), AcceptedFundingAsset::VARIANT_COUNT);
			let mut fundings = fundings.into_iter().cycle();

			let usdt_fundings = accounts
				.iter()
				.map(|acc| UserToForeignAssets {
					account: acc.clone(),
					asset_amount: ASSET_UNIT * 1_000_000,
					asset_id: fundings.next().unwrap().to_assethub_id(),
				})
				.collect_vec();
			inst.mint_plmc_to(plmc_fundings);
			inst.mint_foreign_asset_to(usdt_fundings);

			let project_id = inst.create_auctioning_project(project_metadata, ISSUER_1, default_evaluations());

			let bids = vec![
				(ADAM, 10_000 * ASSET_UNIT, 1, AcceptedFundingAsset::USDT).into(),
				(TOM, 20_000 * ASSET_UNIT, 1, AcceptedFundingAsset::USDC).into(),
				(SOFIA, 20_000 * ASSET_UNIT, 1, AcceptedFundingAsset::DOT).into(),
				(FRED, 10_000 * ASSET_UNIT, 1, AcceptedFundingAsset::USDT).into(),
				(ANNA, 5_000 * ASSET_UNIT, 1, AcceptedFundingAsset::USDC).into(),
				(DAMIAN, 5_000 * ASSET_UNIT, 1, AcceptedFundingAsset::DOT).into(),
			];

			inst.bid_for_users(project_id, bids).unwrap();

			inst.start_community_funding(project_id).unwrap();

			let token_price =
				inst.get_project_details(project_id).weighted_average_price.unwrap().saturating_mul_int(ASSET_UNIT);

			let desired_price = PriceOf::<TestRuntime>::from_float(11.1818f64).saturating_mul_int(ASSET_UNIT);

			assert_close_enough!(token_price, desired_price, Perquintill::from_float(0.99));
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn contribute_does_not_work() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let did = generate_did_from_account(ISSUER_1);
			let investor_type = InvestorType::Retail;
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_community_contribute(
						&BIDDER_1,
						project_id,
						100,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						did,
						investor_type
					),
					Error::<TestRuntime>::ProjectRoundError(RoundError::IncorrectRound)
				);
			});
		}
	}
}

#[cfg(test)]
mod start_auction_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn pallet_can_start_auction_automatically() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluations = default_evaluations();
			let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let ed_plmc = required_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(required_plmc);
			inst.mint_plmc_to(ed_plmc);
			inst.evaluate_for_users(project_id, evaluations).unwrap();
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
			inst.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionOpening);
		}

		#[test]
		fn issuer_can_start_auction_manually() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluations = default_evaluations();
			let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let ed_plmc = required_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(required_plmc);
			inst.mint_plmc_to(ed_plmc);
			inst.evaluate_for_users(project_id, evaluations).unwrap();
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
			inst.advance_time(1).unwrap();
			inst.execute(|| Pallet::<TestRuntime>::do_auction_opening(ISSUER_1, project_id)).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionOpening);
		}

		#[test]
		fn stranger_cannot_start_auction_manually() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluations = default_evaluations();
			let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let ed_plmc = required_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(required_plmc);
			inst.mint_plmc_to(ed_plmc);
			inst.evaluate_for_users(project_id, evaluations).unwrap();
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);
			inst.advance_time(1).unwrap();

			for account in 6000..6010 {
				inst.execute(|| {
					let response = Pallet::<TestRuntime>::do_auction_opening(account, project_id);
					assert_noop!(response, Error::<TestRuntime>::IssuerError(IssuerErrorReason::NotIssuer));
				});
			}
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_start_auction_manually_before_evaluation_finishes() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_auction_opening(ISSUER_1, project_id),
					Error::<TestRuntime>::ProjectRoundError(RoundError::TransitionPointNotSet)
				);
			});
		}

		#[test]
		fn cannot_start_auction_manually_if_evaluation_fails() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_auction_opening(ISSUER_1, project_id),
					Error::<TestRuntime>::ProjectRoundError(RoundError::TransitionPointNotSet)
				);
			});
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		}

		#[test]
		fn auction_doesnt_start_automatically_if_evaluation_fails() {
			// Test our success assumption is ok
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluations = default_evaluations();
			let required_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let ed_plmc = required_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(required_plmc);
			inst.mint_plmc_to(ed_plmc);
			inst.evaluate_for_users(project_id, evaluations).unwrap();
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			inst.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionOpening);

			// Main test with failed evaluation
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			inst.advance_time(<TestRuntime as Config>::AuctionInitializePeriodDuration::get() + 2).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
		}
	}
}

#[cfg(test)]
mod bid_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use frame_support::dispatch::DispatchResultWithPostInfo;

		#[test]
		fn evaluation_bond_counts_towards_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = default_evaluations();
			let evaluator_bidder = 69;
			let evaluation_amount = 420 * US_DOLLAR;
			let evaluator_bid = BidParams::new(evaluator_bidder, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let already_bonded_plmc =
				MockInstantiator::calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])
					[0]
				.plmc_amount;

			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;

			let necessary_plmc_for_bid = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			)[0]
			.plmc_amount;

			let necessary_usdt_for_bid = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_bidder,
				necessary_plmc_for_bid - usable_evaluation_plmc,
			)]);
			inst.mint_foreign_asset_to(necessary_usdt_for_bid);

			inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();

			let evaluation_items = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, evaluator_bidder)).collect_vec()
			});
			assert_eq!(evaluation_items.len(), 1);
			assert_eq!(evaluation_items[0].current_plmc_bond, already_bonded_plmc - usable_evaluation_plmc);

			let bid_items =
				inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, evaluator_bidder)).collect_vec());
			assert_eq!(bid_items.len(), 1);
			assert_eq!(bid_items[0].plmc_bond, necessary_plmc_for_bid);

			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, necessary_plmc_for_bid)],
				HoldReason::Participation(project_id).into(),
			);
			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, already_bonded_plmc - usable_evaluation_plmc)],
				HoldReason::Evaluation(project_id).into(),
			);
		}

		#[test]
		fn bid_with_multiple_currencies() {
			let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
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

			let evaluations = default_evaluations();

			let projects = vec![
				TestProjectParams {
					expected_state: ProjectStatus::AuctionOpening,
					metadata: project_metadata_all.clone(),
					issuer: ISSUER_1,
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::AuctionOpening,
					metadata: project_metadata_usdt,
					issuer: ISSUER_2,
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::AuctionOpening,
					metadata: project_metadata_usdc,
					issuer: ISSUER_3,
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::AuctionOpening,
					metadata: project_metadata_dot,
					issuer: ISSUER_4,
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
			];
			let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

			let project_id_all = project_ids[0];
			let project_id_usdt = project_ids[1];
			let project_id_usdc = project_ids[2];
			let project_id_dot = project_ids[3];

			let usdt_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let usdc_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
			let dot_bid = BidParams::new(BIDDER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

			let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();

			let plmc_all_mints = MockInstantiator::generic_map_operation(
				vec![plmc_fundings, plmc_existential_deposits],
				MergeOperation::Add,
			);
			inst.mint_plmc_to(plmc_all_mints.clone());
			inst.mint_plmc_to(plmc_all_mints.clone());
			inst.mint_plmc_to(plmc_all_mints.clone());

			let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);
			inst.mint_foreign_asset_to(usdt_fundings.clone());
			inst.mint_foreign_asset_to(usdt_fundings.clone());
			inst.mint_foreign_asset_to(usdt_fundings.clone());

			assert_ok!(inst.bid_for_users(project_id_all, vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()]));

			assert_ok!(inst.bid_for_users(project_id_usdt, vec![usdt_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![dot_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);

			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_ok!(inst.bid_for_users(project_id_usdc, vec![usdc_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![dot_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);

			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
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
			let jwt = get_mock_jwt(bidder.clone(), investor_type, generate_did_from_account(BIDDER_1));
			let amount = 1000 * ASSET_UNIT;
			let multiplier = Multiplier::force_new(u8_multiplier);

			if u8_multiplier > 0 {
				let bid = BidParams::<TestRuntime> {
					bidder: bidder.clone(),
					amount,
					multiplier,
					asset: AcceptedFundingAsset::USDT,
				};

				let necessary_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
					&vec![bid.clone()],
					inst.get_project_metadata(project_id).minimum_price,
				);
				let plmc_existential_amounts = necessary_plmc.accounts().existential_deposits();
				let necessary_usdt = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
					&vec![bid.clone()],
					inst.get_project_metadata(project_id).minimum_price,
				);

				inst.mint_plmc_to(necessary_plmc.clone());
				inst.mint_plmc_to(plmc_existential_amounts.clone());
				inst.mint_foreign_asset_to(necessary_usdt.clone());
			}
			inst.execute(|| {
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(bidder),
					jwt,
					project_id,
					amount,
					multiplier,
					AcceptedFundingAsset::USDT,
				)
			})
		}

		#[test]
		fn multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators(),
				default_weights(),
			);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations);
			// Professional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, 0),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
			);
			// Professional bids: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier));
			}
			// Professional bids: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			}

			// Institutional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, 0),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
			);
			// Institutional bids: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier));
			}
			// Institutional bids: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			}
		}

		#[test]
		fn bid_split_into_multiple_buckets() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(1.0);
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(50u8);

			let evaluations = default_evaluations();
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations);

			// bid that fills 80% of the first bucket
			let bid_40_percent = MockInstantiator::generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				40u8,
				vec![100],
				vec![BIDDER_1],
				vec![8u8],
			);

			// Note: 5% of total CTs is one bucket, i.e 10% of the auction allocation
			// This bid fills last 20% of the first bucket,
			// and gets split into 3 more bids of 2 more full and one partially full buckets.
			// 10% + 5% + 5% + 3% = 23%
			let bid_23_percent = MockInstantiator::generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				23u8,
				vec![100],
				vec![BIDDER_2],
				vec![7u8],
			);

			let all_bids = vec![bid_40_percent[0].clone(), bid_23_percent[0].clone()];

			let necessary_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			let ed_plmc = necessary_plmc.accounts().existential_deposits();
			let necessary_usdt =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&all_bids,
					project_metadata.clone(),
					None,
				);
			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(ed_plmc.clone());
			inst.mint_foreign_asset_to(necessary_usdt.clone());

			inst.bid_for_users(project_id, bid_40_percent.clone()).unwrap();
			let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
			assert_eq!(stored_bids.len(), 1);

			inst.bid_for_users(project_id, bid_23_percent.clone()).unwrap();
			let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
			stored_bids.sort_by(|a, b| a.id.cmp(&b.id));
			// 40% + 10% + 5% + 5% + 3% = 5 total bids
			assert_eq!(stored_bids.len(), 5);

			assert_eq!(stored_bids[1].original_ct_usd_price, PriceOf::<TestRuntime>::from_float(1.0));
			assert_eq!(
				stored_bids[1].original_ct_amount,
				Percent::from_percent(10) * project_metadata.total_allocation_size
			);
			assert_eq!(stored_bids[2].original_ct_usd_price, PriceOf::<TestRuntime>::from_rational(11, 10));
			assert_eq!(
				stored_bids[2].original_ct_amount,
				Percent::from_percent(5) * project_metadata.total_allocation_size
			);

			assert_eq!(stored_bids[3].original_ct_usd_price, PriceOf::<TestRuntime>::from_float(1.2));
			assert_eq!(
				stored_bids[3].original_ct_amount,
				Percent::from_percent(5) * project_metadata.total_allocation_size
			);

			assert_eq!(stored_bids[4].original_ct_usd_price, PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(
				stored_bids[4].original_ct_amount,
				Percent::from_percent(3) * project_metadata.total_allocation_size
			);
			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			assert_eq!(current_bucket.current_price, PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(current_bucket.amount_left, Percent::from_percent(2) * project_metadata.total_allocation_size);
			assert_eq!(current_bucket.delta_price, PriceOf::<TestRuntime>::from_float(0.1));
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
			let mut evaluations = default_evaluations();
			let evaluator_bidder = 69;
			let evaluation_amount = 420 * US_DOLLAR;
			let evaluator_bid = BidParams::new(evaluator_bidder, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let necessary_usdt_for_bid = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_foreign_asset_to(necessary_usdt_for_bid);

			assert_err!(inst.bid_for_users(project_id, vec![evaluator_bid]), TokenError::FundsUnavailable);
		}

		#[test]
		fn cannot_bid_before_auction_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let _ = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let did = generate_did_from_account(BIDDER_2);
			let investor_type = InvestorType::Institutional;
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_bid(
						&BIDDER_2,
						0,
						1,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						did,
						investor_type
					),
					Error::<TestRuntime>::ProjectRoundError(RoundError::IncorrectRound)
				);
			});
		}

		#[test]
		fn cannot_bid_more_than_project_limit_count() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = 1_000_000_000 * ASSET_UNIT;
			project_metadata.total_allocation_size = 100_000_000 * ASSET_UNIT;

			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				vec![EVALUATOR_1],
				vec![100u8],
			);
			let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
			let bids =
				(0u32..max_bids_per_project - 1).map(|i| (i as u32 + 420u32, 5000 * ASSET_UNIT).into()).collect_vec();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations);

			let plmc_for_bidding = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);
			let plmc_existential_deposits = bids.accounts().existential_deposits();
			let usdt_for_bidding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);

			inst.mint_plmc_to(plmc_for_bidding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(usdt_for_bidding.clone());
			inst.bid_for_users(project_id, bids.clone()).unwrap();

			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			let remaining_ct = current_bucket.amount_left;

			// This bid should be split in 2, but the second one should fail, making the whole extrinsic fail and roll back storage
			let failing_bid = BidParams::<TestRuntime>::new(
				BIDDER_1,
				remaining_ct + 5000 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			);
			let plmc_for_failing_bid =
				MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
					&vec![failing_bid.clone()],
					project_metadata.clone(),
					Some(current_bucket),
				);
			let plmc_existential_deposits = plmc_for_failing_bid.accounts().existential_deposits();
			let usdt_for_bidding =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&vec![failing_bid.clone()],
					project_metadata.clone(),
					Some(current_bucket),
				);

			inst.mint_plmc_to(plmc_for_failing_bid.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(usdt_for_bidding.clone());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder)
						),
						project_id,
						failing_bid.amount,
						failing_bid.multiplier,
						failing_bid.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooManyProjectParticipations)
				);
			});

			// Now we test that after reaching the limit, just one bid is also not allowed
			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(failing_bid.bidder),
					get_mock_jwt(
						failing_bid.bidder,
						InvestorType::Professional,
						generate_did_from_account(failing_bid.bidder)
					),
					project_id,
					remaining_ct,
					failing_bid.multiplier,
					failing_bid.asset
				));
			});
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder)
						),
						project_id,
						5000 * ASSET_UNIT,
						failing_bid.multiplier,
						failing_bid.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooManyProjectParticipations)
				);
			});
		}

		#[test]
		fn cannot_bid_more_than_user_limit_count() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = 1_000_000_000 * ASSET_UNIT;
			project_metadata.total_allocation_size = 100_000_000 * ASSET_UNIT;

			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				vec![EVALUATOR_1],
				vec![100u8],
			);
			let max_bids_per_user: u32 = <TestRuntime as Config>::MaxBidsPerUser::get();
			let bids = (0u32..max_bids_per_user - 1u32).map(|_| (BIDDER_1, 5000 * ASSET_UNIT).into()).collect_vec();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations);

			let plmc_for_bidding = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);
			let plmc_existential_deposits = bids.accounts().existential_deposits();
			let usdt_for_bidding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);

			inst.mint_plmc_to(plmc_for_bidding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(usdt_for_bidding.clone());
			inst.bid_for_users(project_id, bids.clone()).unwrap();

			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			let remaining_ct = current_bucket.amount_left;

			// This bid should be split in 2, but the second one should fail, making the whole extrinsic fail and roll back storage
			let failing_bid = BidParams::<TestRuntime>::new(
				BIDDER_1,
				remaining_ct + 5000 * ASSET_UNIT,
				1u8,
				AcceptedFundingAsset::USDT,
			);
			let plmc_for_failing_bid =
				MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
					&vec![failing_bid.clone()],
					project_metadata.clone(),
					Some(current_bucket),
				);
			let plmc_existential_deposits = plmc_for_failing_bid.accounts().existential_deposits();
			let usdt_for_bidding =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&vec![failing_bid.clone()],
					project_metadata.clone(),
					Some(current_bucket),
				);
			inst.mint_plmc_to(plmc_for_failing_bid.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(usdt_for_bidding.clone());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder)
						),
						project_id,
						failing_bid.amount,
						failing_bid.multiplier,
						failing_bid.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooManyUserParticipations)
				);
			});

			// Now we test that after reaching the limit, just one bid is also not allowed
			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(failing_bid.bidder),
					get_mock_jwt(
						failing_bid.bidder,
						InvestorType::Professional,
						generate_did_from_account(failing_bid.bidder)
					),
					project_id,
					remaining_ct,
					failing_bid.multiplier,
					failing_bid.asset
				));
			});
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder)
						),
						project_id,
						5000 * ASSET_UNIT,
						failing_bid.multiplier,
						failing_bid.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooManyUserParticipations)
				);
			});
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * ASSET_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
				phantom: Default::default(),
			};

			let evaluations = default_evaluations();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations.clone());

			inst.mint_plmc_to(vec![(BIDDER_1, 50_000 * ASSET_UNIT).into(), (BIDDER_2, 50_000 * ASSET_UNIT).into()]);

			inst.mint_foreign_asset_to(vec![
				(BIDDER_1, 50_000 * US_DOLLAR).into(),
				(BIDDER_2, 50_000 * US_DOLLAR).into(),
			]);

			// bid below 800 CT (8k USD) should fail for professionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(
						&BIDDER_1,
						project_id,
						799 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						generate_did_from_account(BIDDER_1),
						InvestorType::Professional
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
			// bid below 2000 CT (20k USD) should fail for institutionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(
						&BIDDER_2,
						project_id,
						1999 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						generate_did_from_account(BIDDER_1),
						InvestorType::Institutional
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
		}

		#[test]
		fn ticket_size_minimums_use_current_bucket_price() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * ASSET_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
				phantom: Default::default(),
			};
			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(1.0);

			let evaluations = default_evaluations();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 200_000 * PLMC).into(),
				(BIDDER_2, 200_000 * PLMC).into(),
				(BIDDER_3, 200_000 * PLMC).into(),
			]);
			inst.mint_foreign_asset_to(vec![
				(BIDDER_1, 200_000 * ASSET_UNIT).into(),
				(BIDDER_2, 200_000 * ASSET_UNIT).into(),
				(BIDDER_3, 200_000 * ASSET_UNIT).into(),
			]);

			// First bucket is covered by one bidder
			let big_bid: BidParams<TestRuntime> = (BIDDER_1, 50_000 * ASSET_UNIT).into();
			inst.bid_for_users(project_id, vec![big_bid.clone()]).unwrap();

			// A bid at the min price of 1 should require a min of 8k CT, but with a new price of 1.1, we can now bid with less
			let smallest_ct_amount_at_8k_usd = PriceOf::<TestRuntime>::from_float(1.1)
				.reciprocal()
				.unwrap()
				.checked_mul_int(8000 * US_DOLLAR)
				// add 1 because result could be .99999 of what we expect
				.unwrap() + 1;
			assert!(smallest_ct_amount_at_8k_usd < 8000 * ASSET_UNIT);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(
					&BIDDER_1,
					project_id,
					smallest_ct_amount_at_8k_usd,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BIDDER_1),
					InvestorType::Professional
				));
			});
			let smallest_ct_amount_at_20k_usd = PriceOf::<TestRuntime>::from_float(1.1)
				.reciprocal()
				.unwrap()
				.checked_mul_int(20_000 * US_DOLLAR)
				// add 1 because result could be .99999 of what we expect
				.unwrap() + 1;
			assert!(smallest_ct_amount_at_20k_usd < 20_000 * ASSET_UNIT);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(
					&BIDDER_2,
					project_id,
					smallest_ct_amount_at_20k_usd,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BIDDER_1),
					InvestorType::Institutional
				));
			});
		}

		#[test]
		fn per_credential_type_ticket_size_maximums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(Some(8_000 * US_DOLLAR), Some(100_000 * US_DOLLAR)),
				institutional: TicketSize::new(Some(20_000 * US_DOLLAR), Some(500_000 * US_DOLLAR)),
				phantom: Default::default(),
			};
			project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
				retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
				professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
				institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
				phantom: Default::default(),
			};
			let evaluations = default_evaluations();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 500_000 * ASSET_UNIT).into(),
				(BIDDER_2, 500_000 * ASSET_UNIT).into(),
				(BIDDER_3, 500_000 * ASSET_UNIT).into(),
				(BIDDER_4, 500_000 * ASSET_UNIT).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BIDDER_1, 500_000 * US_DOLLAR).into(),
				(BIDDER_2, 500_000 * US_DOLLAR).into(),
				(BIDDER_3, 500_000 * US_DOLLAR).into(),
				(BIDDER_4, 500_000 * US_DOLLAR).into(),
			]);

			let bidder_1_jwt = get_mock_jwt(BIDDER_1, InvestorType::Professional, generate_did_from_account(BIDDER_1));
			let bidder_2_jwt_same_did =
				get_mock_jwt(BIDDER_2, InvestorType::Professional, generate_did_from_account(BIDDER_1));
			// total bids with same DID above 10k CT (100k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_1),
					bidder_1_jwt,
					project_id,
					8000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_2),
						bidder_2_jwt_same_did.clone(),
						project_id,
						3000 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 10k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_2),
					bidder_2_jwt_same_did,
					project_id,
					2000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});

			let bidder_3_jwt = get_mock_jwt(BIDDER_3, InvestorType::Institutional, generate_did_from_account(BIDDER_3));
			let bidder_4_jwt_same_did =
				get_mock_jwt(BIDDER_4, InvestorType::Institutional, generate_did_from_account(BIDDER_3));
			// total bids with same DID above 50k CT (500k USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_3),
					bidder_3_jwt,
					project_id,
					40_000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_4),
						bidder_4_jwt_same_did.clone(),
						project_id,
						11_000 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 50k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_4),
					bidder_4_jwt_same_did,
					project_id,
					10_000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn issuer_cannot_bid_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, default_evaluations());
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_bid(
					&(&ISSUER_1 + 1),
					project_id,
					500 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional
				)),
				Error::<TestRuntime>::IssuerError(IssuerErrorReason::ParticipationToOwnProject)
			);
		}

		#[test]
		fn bid_with_asset_not_accepted() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id =
				inst.create_auctioning_project(default_project_metadata(ISSUER_1), ISSUER_1, default_evaluations());
			let bids = vec![BidParams::<TestRuntime>::new(BIDDER_1, 10_000, 1u8, AcceptedFundingAsset::USDC)];

			let did = generate_did_from_account(bids[0].bidder);
			let investor_type = InvestorType::Institutional;

			let outcome = inst.execute(|| {
				Pallet::<TestRuntime>::do_bid(
					&bids[0].bidder,
					project_id,
					bids[0].amount,
					bids[0].multiplier,
					bids[0].asset,
					did,
					investor_type,
				)
			});
			frame_support::assert_err!(
				outcome,
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
		}
	}
}
