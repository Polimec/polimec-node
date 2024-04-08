use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
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
		fn price_calculation() {
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
				token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: ASSET_DECIMALS },
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
			let bidder_5_bid = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, BIDDER_6)).next().unwrap());
			let wabgp = inst.get_project_details(project_id).weighted_average_price.unwrap();
			assert_eq!(bidder_5_bid.original_ct_usd_price.to_float(), 11.0);
			assert_eq!(bidder_5_bid.final_ct_usd_price, wabgp);
		}

		#[test]
		fn after_random_end_bid_gets_refunded() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, default_evaluations());

			let (bid_in, bid_out) = (default_bids()[0].clone(), default_bids()[1].clone());

			let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&vec![bid_in.clone(), bid_out.clone()],
				project_metadata.minimum_price,
			);
			let plmc_existential_amounts = plmc_fundings.accounts().existential_deposits();

			let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_in.clone(), bid_out.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_existential_amounts.clone());
			inst.mint_foreign_asset_to(usdt_fundings.clone());

			inst.bid_for_users(project_id, vec![bid_in]).unwrap();
			inst.advance_time(
				<TestRuntime as Config>::AuctionOpeningDuration::get() + <TestRuntime as Config>::AuctionClosingDuration::get() -
					1,
			)
				.unwrap();

			inst.bid_for_users(project_id, vec![bid_out]).unwrap();

			inst.do_free_plmc_assertions(vec![
				UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_2, MockInstantiator::get_ed()),
			]);
			inst.do_reserved_plmc_assertions(
				vec![
					UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount),
					UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount),
				],
				HoldReason::Participation(project_id).into(),
			);
			inst.do_bid_transferred_foreign_asset_assertions(
				vec![
					UserToForeignAssets::<TestRuntime>::new(
						BIDDER_1,
						usdt_fundings[0].asset_amount,
						AcceptedFundingAsset::USDT.to_assethub_id(),
					),
					UserToForeignAssets::<TestRuntime>::new(
						BIDDER_2,
						usdt_fundings[1].asset_amount,
						AcceptedFundingAsset::USDT.to_assethub_id(),
					),
				],
				project_id,
			);
			inst.start_community_funding(project_id).unwrap();
			inst.do_free_plmc_assertions(vec![
				UserToPLMCBalance::new(BIDDER_1, MockInstantiator::get_ed()),
				UserToPLMCBalance::new(BIDDER_2, plmc_fundings[1].plmc_amount + MockInstantiator::get_ed()),
			]);

			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(BIDDER_1, plmc_fundings[0].plmc_amount), UserToPLMCBalance::new(BIDDER_2, 0)],
				HoldReason::Participation(project_id).into(),
			);

			inst.do_bid_transferred_foreign_asset_assertions(
				vec![
					UserToForeignAssets::<TestRuntime>::new(
						BIDDER_1,
						usdt_fundings[0].asset_amount,
						AcceptedFundingAsset::USDT.to_assethub_id(),
					),
					UserToForeignAssets::<TestRuntime>::new(BIDDER_2, 0, AcceptedFundingAsset::USDT.to_assethub_id()),
				],
				project_id,
			);
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
			let project_id =
				inst.create_community_contributing_project(project_metadata.clone(), ISSUER_1, evaluations.clone(), bids);
			let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
			let bid_info = inst.execute(|| bid_infos.next().unwrap());
			assert!(inst.execute(|| bid_infos.next().is_none()));
			assert_eq!(bid_info.final_ct_amount, auction_allocation);

			let project_metadata = default_project_metadata(ISSUER_2);
			let bids = vec![(BIDDER_1, auction_allocation).into(), (BIDDER_1, 1000 * ASSET_UNIT).into()];
			let project_id =
				inst.create_community_contributing_project(project_metadata.clone(), ISSUER_2, evaluations.clone(), bids);
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

	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn bids_after_random_block_get_rejected() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.total_allocation_size = 1_000_000 * ASSET_UNIT;
			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators(),
				default_weights(),
			);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);
			let opening_end_block = inst
				.get_project_details(project_id)
				.phase_transition_points
				.auction_opening
				.end()
				.expect("Auction start point should exist");
			// The block following the end of the opening auction, is used to transition the project into closing auction.
			// We move past that transition, into the start of the closing auction.
			let now = inst.current_block();
			inst.advance_time(opening_end_block - now + 1).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionClosing);

			let closing_end_block = inst
				.get_project_details(project_id)
				.phase_transition_points
				.auction_closing
				.end()
				.expect("closing auction end point should exist");

			let bid_info = BidParams::new(0, 500u128 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

			let plmc_necessary_funding = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&vec![bid_info.clone()],
				project_metadata.minimum_price,
			)[0]
				.plmc_amount;

			let foreign_asset_necessary_funding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_info.clone()],
				project_metadata.minimum_price,
			)[0]
				.asset_amount;

			let mut bids_made: Vec<BidParams<TestRuntime>> = vec![];
			let starting_bid_block = inst.current_block();
			let blocks_to_bid = inst.current_block()..closing_end_block;

			let mut bidding_account = 1000;

			// Do one closing bid for each block until the end of closing auction with a new user
			for _block in blocks_to_bid {
				assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionClosing);
				inst.mint_plmc_to(vec![UserToPLMCBalance::new(bidding_account, plmc_necessary_funding * 10)]);
				inst.mint_plmc_to(vec![bidding_account].existential_deposits());

				inst.mint_foreign_asset_to(vec![UserToForeignAssets::new(
					bidding_account,
					foreign_asset_necessary_funding * 10,
					bid_info.asset.to_assethub_id(),
				)]);
				let bids: Vec<BidParams<_>> = vec![BidParams {
					bidder: bidding_account,
					amount: bid_info.amount,
					multiplier: bid_info.multiplier,
					asset: bid_info.asset,
				}];
				inst.bid_for_users(project_id, bids.clone()).unwrap();

				bids_made.push(bids[0].clone());
				bidding_account += 1;

				inst.advance_time(1).unwrap();
			}
			let now = inst.current_block();
			inst.advance_time(closing_end_block - now + 1).unwrap();

			let random_end = inst
				.get_project_details(project_id)
				.phase_transition_points
				.random_closing_ending
				.expect("Random auction end point should exist");

			let split = (random_end - starting_bid_block + 1) as usize;
			let excluded_bids = bids_made.split_off(split);
			let included_bids = bids_made;
			let _weighted_price =
				inst.get_project_details(project_id).weighted_average_price.expect("Weighted price should exist");

			for bid in included_bids {
				let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, bid.bidder)));
				let desired_bid: BidInfoFilter<TestRuntime> = BidInfoFilter {
					project_id: Some(project_id),
					bidder: Some(bid.bidder),
					original_ct_amount: Some(bid.amount),
					original_ct_usd_price: None,
					status: Some(BidStatus::Accepted),
					..Default::default()
				};

				assert!(
					inst.execute(|| stored_bids.any(|bid| desired_bid.matches_bid(&bid))),
					"Stored bid does not match the given filter"
				)
			}

			for bid in excluded_bids {
				assert!(inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, bid.bidder)).count() == 0));
			}
		}

		#[test]
		fn unsuccessful_bids_dont_get_vest_schedule() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let auction_token_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

			let mut bids = MockInstantiator::generate_bids_from_total_usd(
				Percent::from_percent(80) * project_metadata.minimum_price.saturating_mul_int(auction_token_allocation),
				project_metadata.minimum_price,
				vec![60, 40],
				vec![BIDDER_1, BIDDER_2],
				vec![1u8, 1u8],
			);

			let available_tokens = auction_token_allocation.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));

			let rejected_bid = vec![BidParams::new(BIDDER_5, available_tokens, 1u8, AcceptedFundingAsset::USDT)];
			let accepted_bid = vec![BidParams::new(BIDDER_4, available_tokens, 2u8, AcceptedFundingAsset::USDT)];
			bids.extend(rejected_bid.clone());
			bids.extend(accepted_bid.clone());

			let community_contributions = default_community_buys();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

			let bidders_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
			let bidders_existential_deposits = bidders_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(bidders_plmc.clone());
			inst.mint_plmc_to(bidders_existential_deposits);

			let bidders_funding_assets =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&bids,
					project_metadata.clone(),
					None,
				);
			inst.mint_foreign_asset_to(bidders_funding_assets);

			inst.bid_for_users(project_id, bids).unwrap();

			inst.start_community_funding(project_id).unwrap();

			let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let contributors_plmc =
				MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
			let contributors_existential_deposits = contributors_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(contributors_plmc.clone());
			inst.mint_plmc_to(contributors_existential_deposits);

			let contributors_funding_assets =
				MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
			inst.mint_foreign_asset_to(contributors_funding_assets);

			inst.contribute_for_users(project_id, community_contributions).unwrap();
			inst.start_remainder_or_end_funding(project_id).unwrap();
			inst.finish_funding(project_id).unwrap();

			inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
			inst.settle_project(project_id).unwrap();

			let plmc_locked_for_accepted_bid =
				MockInstantiator::calculate_auction_plmc_charged_with_given_price(&accepted_bid, final_price);
			let plmc_locked_for_rejected_bid =
				MockInstantiator::calculate_auction_plmc_charged_with_given_price(&rejected_bid, final_price);

			let UserToPLMCBalance { account: accepted_user, plmc_amount: accepted_plmc_amount } =
				plmc_locked_for_accepted_bid[0];
			let schedule = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&accepted_user,
					HoldReason::Participation(project_id).into(),
				)
			});
			assert_close_enough!(schedule.unwrap(), accepted_plmc_amount, Perquintill::from_float(0.99));

			let UserToPLMCBalance { account: rejected_user, .. } = plmc_locked_for_rejected_bid[0];
			assert!(inst
				.execute(|| {
					<TestRuntime as Config>::Vesting::total_scheduled_amount(
						&rejected_user,
						HoldReason::Participation(project_id).into(),
					)
				})
				.is_none());
		}

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
			Error::<TestRuntime>::AuctionNotStarted
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
					assert_noop!(response, Error::<TestRuntime>::NotAllowed);
				});
			}
		}

		// We use the already tested instantiator functions to calculate the correct post-wap returns
		#[test]
		fn refund_on_partial_acceptance_and_price_above_wap_and_ct_sold_out_bids() {
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
			let usdt_fundings = MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
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
			let returned_auction_plmc =
				MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
			let returned_funding_assets =
				MockInstantiator::calculate_auction_funding_asset_returned_from_all_bids_made(&bids, project_metadata, wap);

			let expected_free_plmc = MockInstantiator::generic_map_operation(
				vec![returned_auction_plmc.clone(), plmc_existential_amounts],
				MergeOperation::Add,
			);
			let expected_free_funding_assets =
				MockInstantiator::generic_map_operation(vec![returned_funding_assets.clone()], MergeOperation::Add);
			dbg!(&expected_free_plmc);
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
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_start_auction_before_evaluation_finishes() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			inst.execute(|| {
				assert_noop!(
			PolimecFunding::do_auction_opening(ISSUER_1, project_id),
			Error::<TestRuntime>::EvaluationPeriodNotEnded
		);
			});
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
			frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
		}
	}
}

#[cfg(test)]
mod bid_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

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
				MockInstantiator::calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0]
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

			inst.mint_plmc_to(vec![UserToPLMCBalance::new(evaluator_bidder, necessary_plmc_for_bid - usable_evaluation_plmc)]);

			inst.mint_foreign_asset_to(necessary_usdt_for_bid);

			inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();
		}

		#[test]
		fn bidder_was_evaluator() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let mut bids = default_bids();
			let evaluator = evaluations[0].account;
			bids.push(BidParams::new(evaluator, 500 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT));
			let _ = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
		}

		#[test]
		fn bid_with_multiple_currencies() {
			let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata_all = default_project_metadata(ISSUER_1);
			project_metadata_all.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

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

			let plmc_all_mints =
				MockInstantiator::generic_map_operation(vec![plmc_fundings, plmc_existential_deposits], MergeOperation::Add);
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
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
			assert_err!(
		inst.bid_for_users(project_id_usdt, vec![dot_bid.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);

			assert_err!(
		inst.bid_for_users(project_id_usdc, vec![usdt_bid.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
			assert_ok!(inst.bid_for_users(project_id_usdc, vec![usdc_bid.clone()]));
			assert_err!(
		inst.bid_for_users(project_id_usdc, vec![dot_bid.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);

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

		#[test]
		fn multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = ProjectMetadata {
				token_information: default_token_information(),
				mainnet_token_max_supply: 80_000_000 * ASSET_UNIT,
				total_allocation_size: 10_000_000 * ASSET_UNIT,
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
				offchain_information_hash: Some(hashed(METADATA)),
			};
			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators(),
				default_weights(),
			);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations);
			// Professional bids: 0x multiplier should fail
			let jwt = get_mock_jwt(BIDDER_1, InvestorType::Professional, generate_did_from_account(BIDDER_1));
			inst.execute(|| {
				assert_noop!(
			Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_1),
				jwt,
				project_id,
				1000 * ASSET_UNIT,
				Multiplier::force_new(0),
				AcceptedFundingAsset::USDT
			),
			Error::<TestRuntime>::ForbiddenMultiplier
		);
			});
			// Professional bids: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				let jwt = get_mock_jwt(BIDDER_1, InvestorType::Professional, generate_did_from_account(BIDDER_1));
				let bidder_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
					&vec![(BIDDER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let bidder_usdt = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
					&vec![(BIDDER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let ed = MockInstantiator::get_ed();
				inst.mint_plmc_to(vec![(BIDDER_1, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::bid(
			RuntimeOrigin::signed(BIDDER_1),
			jwt,
			project_id,
			1000 * ASSET_UNIT,
			Multiplier::force_new(multiplier),
			AcceptedFundingAsset::USDT
		)));
			}
			// Professional bids: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				let jwt = get_mock_jwt(BIDDER_1, InvestorType::Professional, generate_did_from_account(BIDDER_1));
				let bidder_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
					&vec![(BIDDER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let bidder_usdt = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
					&vec![(BIDDER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let ed = MockInstantiator::get_ed();
				inst.mint_plmc_to(vec![(BIDDER_1, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				inst.execute(|| {
					assert_noop!(
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_1),
					jwt,
					project_id,
					1000 * ASSET_UNIT,
					Multiplier::force_new(multiplier),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
				});
			}

			// Institutional bids: 0x multiplier should fail
			let jwt = get_mock_jwt(BIDDER_2, InvestorType::Institutional, generate_did_from_account(BIDDER_2));
			inst.execute(|| {
				assert_noop!(
			Pallet::<TestRuntime>::bid(
				RuntimeOrigin::signed(BIDDER_2),
				jwt,
				project_id,
				1000 * ASSET_UNIT,
				Multiplier::force_new(0),
				AcceptedFundingAsset::USDT
			),
			Error::<TestRuntime>::ForbiddenMultiplier
		);
			});
			// Institutional bids: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				let jwt = get_mock_jwt(BIDDER_2, InvestorType::Institutional, generate_did_from_account(BIDDER_2));
				let bidder_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
					&vec![(BIDDER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let bidder_usdt = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
					&vec![(BIDDER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let ed = MockInstantiator::get_ed();
				inst.mint_plmc_to(vec![(BIDDER_2, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::bid(
			RuntimeOrigin::signed(BIDDER_2),
			jwt,
			project_id,
			1000 * ASSET_UNIT,
			multiplier.try_into().unwrap(),
			AcceptedFundingAsset::USDT
		)));
			}
			// Institutional bids: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				let jwt = get_mock_jwt(BIDDER_2, InvestorType::Institutional, generate_did_from_account(BIDDER_2));
				let bidder_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
					&vec![(BIDDER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let bidder_usdt = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
					&vec![(BIDDER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
					project_metadata.minimum_price,
				);
				let ed = MockInstantiator::get_ed();
				inst.mint_plmc_to(vec![(BIDDER_2, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				inst.execute(|| {
					assert_noop!(
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_2),
					jwt,
					project_id,
					1000 * ASSET_UNIT,
					Multiplier::force_new(multiplier),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
				});
			}
		}

	}

	#[cfg(test)]
	mod failure {
		use super::*;

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
			Error::<TestRuntime>::AuctionNotStarted
		);
			});
		}

		#[test]
		fn cannot_bid_more_than_project_limit_count() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = ProjectMetadata {
				token_information: default_token_information(),
				mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
				total_allocation_size: 1_000_000 * ASSET_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(50u8),
				minimum_price: PriceOf::<TestRuntime>::from_float(100.0),
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
				offchain_information_hash: Some(hashed(METADATA)),
			};
			let evaluations =
				MockInstantiator::generate_successful_evaluations(project_metadata.clone(), vec![EVALUATOR_1], vec![100u8]);
			let bids = (0u32..<TestRuntime as Config>::MaxBidsPerProject::get())
				.map(|i| (i as u32 + 420u32, 50 * ASSET_UNIT).into())
				.collect_vec();
			let failing_bid = BidParams::<TestRuntime>::new(BIDDER_1, 50 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);

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

			let plmc_for_failing_bid = MockInstantiator::calculate_auction_plmc_charged_with_given_price(
				&vec![failing_bid.clone()],
				project_metadata.minimum_price,
			);
			let plmc_existential_deposits = plmc_for_failing_bid.accounts().existential_deposits();
			let usdt_for_bidding = MockInstantiator::calculate_auction_funding_asset_charged_with_given_price(
				&vec![failing_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_plmc_to(plmc_for_failing_bid.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(usdt_for_bidding.clone());

			assert_err!(inst.bid_for_users(project_id, vec![failing_bid]), Error::<TestRuntime>::TooManyBidsForProject);
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = ProjectMetadata {
				token_information: default_token_information(),
				mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
				total_allocation_size: 100_000 * ASSET_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(50u8),
				minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
					institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
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
				offchain_information_hash: Some(hashed(METADATA)),
			};
			let evaluations = default_evaluations();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, evaluations.clone());

			inst.mint_plmc_to(vec![(BIDDER_1, 50_000 * ASSET_UNIT).into(), (BIDDER_2, 50_000 * ASSET_UNIT).into()]);

			inst.mint_foreign_asset_to(vec![(BIDDER_1, 50_000 * US_DOLLAR).into(), (BIDDER_2, 50_000 * US_DOLLAR).into()]);

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
			Error::<TestRuntime>::BidTooLow
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
			Error::<TestRuntime>::BidTooLow
		);
			});
		}

		#[test]
		fn per_credential_type_ticket_size_maximums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = ProjectMetadata {
				token_information: default_token_information(),
				mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
				total_allocation_size: 100_000 * ASSET_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(80u8),
				minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(Some(8_000 * US_DOLLAR), Some(100_000 * US_DOLLAR)),
					institutional: TicketSize::new(Some(20_000 * US_DOLLAR), Some(500_000 * US_DOLLAR)),
					phantom: Default::default(),
				},
				contributing_ticket_sizes: ContributingTicketSizes {
					retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
					professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
					institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
				funding_destination_account: ISSUER_1,
				offchain_information_hash: Some(hashed(METADATA)),
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
			let bidder_2_jwt_same_did = get_mock_jwt(BIDDER_2, InvestorType::Professional, generate_did_from_account(BIDDER_1));
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
			Error::<TestRuntime>::BidTooHigh
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
			Error::<TestRuntime>::BidTooHigh
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
		Error::<TestRuntime>::ParticipationToThemselves
	);
		}
	}
}




















