use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn community_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let _ = inst.create_remainder_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
			);
		}

		#[test]
		fn multiple_contribution_projects_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(ISSUER_1);
			let project2 = default_project_metadata(ISSUER_2);
			let project3 = default_project_metadata(ISSUER_3);
			let project4 = default_project_metadata(ISSUER_4);
			let evaluations = default_evaluations();
			let bids = default_bids();
			let community_buys = default_community_buys();

			inst.create_remainder_contributing_project(
				project1,
				ISSUER_1,
				evaluations.clone(),
				bids.clone(),
				community_buys.clone(),
			);
			inst.create_remainder_contributing_project(
				project2,
				ISSUER_2,
				evaluations.clone(),
				bids.clone(),
				community_buys.clone(),
			);
			inst.create_remainder_contributing_project(
				project3,
				ISSUER_3,
				evaluations.clone(),
				bids.clone(),
				community_buys.clone(),
			);
			inst.create_remainder_contributing_project(project4, ISSUER_4, evaluations, bids, community_buys);
		}

		#[test]
		fn community_round_ends_on_all_ct_sold_exact() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let bids = vec![
				BidParams::new_with_defaults(BIDDER_1, 40_000 * ASSET_UNIT),
				BidParams::new_with_defaults(BIDDER_2, 10_000 * ASSET_UNIT),
			];
			let project_id = inst.create_community_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				default_evaluations(),
				bids,
			);
			const BOB: AccountId = 808;

			let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
			let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

			let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
			let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
			let foreign_asset_fundings =
				MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

			// Buy remaining CTs
			inst.contribute_for_users(project_id, contributions)
				.expect("The Buyer should be able to buy the exact amount of remaining CTs");
			inst.advance_time(2u64).unwrap();
			// Check remaining CTs is 0
			assert_eq!(
				inst.get_project_details(project_id).remaining_contribution_tokens,
				0,
				"There are still remaining CTs"
			);

			// Check project is in FundingEnded state
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

			inst.do_free_plmc_assertions(plmc_existential_deposits);
			inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
				BOB,
				0_u128,
				AcceptedFundingAsset::USDT.to_assethub_id(),
			)]);
			inst.do_reserved_plmc_assertions(
				vec![plmc_fundings[0].clone()],
				HoldReason::Participation(project_id).into(),
			);
			inst.do_contribution_transferred_foreign_asset_assertions(foreign_asset_fundings, project_id);
		}

		#[test]
		fn round_has_total_ct_allocation_minus_auction_sold() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);
			let project_details = inst.get_project_details(project_id);
			let bid_ct_sold: BalanceOf<TestRuntime> = inst.execute(|| {
				Bids::<TestRuntime>::iter_prefix_values((project_id,))
					.fold(Zero::zero(), |acc, bid| acc + bid.final_ct_amount)
			});
			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size - bid_ct_sold
			);

			let contributions = vec![(BUYER_1, project_details.remaining_contribution_tokens).into()];

			let plmc_contribution_funding = MockInstantiator::calculate_contributed_plmc_spent(
				contributions.clone(),
				project_details.weighted_average_price.unwrap(),
			);
			let plmc_existential_deposits = plmc_contribution_funding.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_contribution_funding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			let foreign_asset_contribution_funding = MockInstantiator::calculate_contributed_funding_asset_spent(
				contributions.clone(),
				project_details.weighted_average_price.unwrap(),
			);
			inst.mint_foreign_asset_to(foreign_asset_contribution_funding.clone());

			inst.contribute_for_users(project_id, contributions).unwrap();

			assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);
		}
	}
}

#[cfg(test)]
mod community_contribute_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use frame_support::dispatch::DispatchResultWithPostInfo;

		#[test]
		fn evaluation_bond_counts_towards_contribution() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			const BOB: AccountId = 42069;
			const CARL: AccountId = 420691;
			let mut evaluations = default_evaluations();
			let bob_evaluation: UserToUSDBalance<TestRuntime> = (BOB, 1337 * US_DOLLAR).into();
			let carl_evaluation: UserToUSDBalance<TestRuntime> = (CARL, 1337 * US_DOLLAR).into();
			evaluations.push(bob_evaluation.clone());
			evaluations.push(carl_evaluation.clone());

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				evaluations,
				default_bids(),
			);

			let evaluation_bond =
				inst.execute(|| Balances::balance_on_hold(&HoldReason::Evaluation(project_id).into(), &BOB));
			let slashable_bond = <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;
			let usable_bond = evaluation_bond - slashable_bond;

			let plmc_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let usable_usd = plmc_price.saturating_mul_int(usable_bond);
			let usable_ct = ct_price.reciprocal().unwrap().saturating_mul_int(usable_usd);

			let slashable_usd = plmc_price.saturating_mul_int(slashable_bond);
			let slashable_ct = ct_price.reciprocal().unwrap().saturating_mul_int(slashable_usd);

			// Can't contribute with only the evaluation bond
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BOB),
						get_mock_jwt_with_cid(
							BOB,
							InvestorType::Retail,
							generate_did_from_account(BOB),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						usable_ct + slashable_ct,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::NotEnoughFunds)
				);
			});

			// Can partially use the usable evaluation bond (half in this case)
			let contribution_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
				vec![(BOB, usable_ct / 2).into()],
				ct_price,
			);
			inst.mint_foreign_asset_to(contribution_usdt.clone());
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BOB),
					get_mock_jwt_with_cid(
						BOB,
						InvestorType::Retail,
						generate_did_from_account(BOB),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					usable_ct / 2,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});

			// Can use the full evaluation bond
			let contribution_usdt =
				MockInstantiator::calculate_contributed_funding_asset_spent(vec![(CARL, usable_ct).into()], ct_price);
			inst.mint_foreign_asset_to(contribution_usdt.clone());
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(CARL),
					get_mock_jwt_with_cid(
						CARL,
						InvestorType::Retail,
						generate_did_from_account(CARL),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					usable_ct,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn evaluation_bond_used_on_failed_bid_can_be_reused_on_contribution() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let bob = 42069;
			let project_metadata = default_project_metadata(ISSUER_1);
			// An evaluator that did a bid but it was not accepted at the end of the auction, can use that PLMC for contributing
			let mut evaluations = default_evaluations();
			let bob_evaluation = (bob, 1337 * US_DOLLAR).into();
			evaluations.push(bob_evaluation);

			let bids = default_bids();
			let bob_bid: BidParams<TestRuntime> = (bob, 1337 * ASSET_UNIT).into();
			let all_bids = bids.iter().chain(vec![bob_bid.clone()].iter()).cloned().collect_vec();

			let project_id = inst.create_auctioning_project(default_project_metadata(ISSUER_2), ISSUER_2, evaluations);

			let evaluation_bond =
				inst.execute(|| Balances::balance_on_hold(&HoldReason::Evaluation(project_id).into(), &bob));
			let slashable_bond = <TestRuntime as Config>::EvaluatorSlash::get() * evaluation_bond;
			let usable_bond = evaluation_bond - slashable_bond;

			let bids_plmc = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			let bids_existential_deposits = bids_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(bids_plmc.clone());
			inst.mint_plmc_to(bids_existential_deposits.clone());

			let bids_foreign =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&all_bids,
					project_metadata.clone(),
					None,
				);
			inst.mint_foreign_asset_to(bids_foreign.clone());

			inst.bid_for_users(project_id, bids).unwrap();

			let auction_end = <TestRuntime as Config>::AuctionOpeningDuration::get() +
				<TestRuntime as Config>::AuctionClosingDuration::get();
			inst.advance_time(auction_end - 1).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionClosing);
			inst.bid_for_users(project_id, vec![bob_bid]).unwrap();

			inst.start_community_funding(project_id).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

			let plmc_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let usable_usd = plmc_price.saturating_mul_int(usable_bond);
			let usable_ct = wap.reciprocal().unwrap().saturating_mul_int(usable_usd);

			let bob_contribution = (bob, 1337 * ASSET_UNIT).into();
			let contribution_usdt =
				MockInstantiator::calculate_contributed_funding_asset_spent(vec![bob_contribution], wap);
			inst.mint_foreign_asset_to(contribution_usdt.clone());
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(bob),
					get_mock_jwt_with_cid(
						bob,
						InvestorType::Retail,
						generate_did_from_account(bob),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					usable_ct,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn contribute_with_multiple_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata_all = default_project_metadata(ISSUER_1);
			project_metadata_all.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.try_into()
					.unwrap();

			let project_id = inst.create_community_contributing_project(
				project_metadata_all.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);
			let usdt_contribution =
				ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let usdc_contribution =
				ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
			let dot_contribution =
				ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(
				vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
				wap,
			);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();

			let plmc_all_mints = MockInstantiator::generic_map_operation(
				vec![plmc_fundings, plmc_existential_deposits],
				MergeOperation::Add,
			);
			inst.mint_plmc_to(plmc_all_mints.clone());

			let asset_hub_fundings = MockInstantiator::calculate_contributed_funding_asset_spent(
				vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
				wap,
			);
			inst.mint_foreign_asset_to(asset_hub_fundings.clone());

			assert_ok!(inst.contribute_for_users(
				project_id,
				vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()]
			));
		}

		fn test_contribution_setup(
			inst: &mut MockInstantiator,
			project_id: ProjectId,
			contributor: AccountIdOf<TestRuntime>,
			investor_type: InvestorType,
			u8_multiplier: u8,
		) -> DispatchResultWithPostInfo {
			let project_policy = inst.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
			let jwt = get_mock_jwt_with_cid(
				contributor.clone(),
				investor_type,
				generate_did_from_account(contributor),
				project_policy,
			);
			let amount = 1000 * ASSET_UNIT;
			let multiplier = Multiplier::force_new(u8_multiplier);
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			if u8_multiplier > 0 {
				let contribution = ContributionParams::<TestRuntime> {
					contributor: contributor.clone(),
					amount,
					multiplier,
					asset: AcceptedFundingAsset::USDT,
				};

				let necessary_plmc =
					MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], wap);
				let plmc_existential_amounts = necessary_plmc.accounts().existential_deposits();
				let necessary_usdt =
					MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], wap);

				inst.mint_plmc_to(necessary_plmc.clone());
				inst.mint_plmc_to(plmc_existential_amounts.clone());
				inst.mint_foreign_asset_to(necessary_usdt.clone());
			}
			inst.execute(|| {
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(contributor),
					jwt,
					project_id,
					amount,
					multiplier,
					AcceptedFundingAsset::USDT,
				)
			})
		}

		#[test]
		fn non_retail_multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = 80_000_000 * ASSET_UNIT;
			project_metadata.total_allocation_size = 10_000_000 * ASSET_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			};
			project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			};
			let evaluations = MockInstantiator::generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators(),
				default_weights(),
			);
			let bids = MockInstantiator::generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				50,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let project_id =
				inst.create_community_contributing_project(project_metadata.clone(), ISSUER_1, evaluations, bids);

			// Professional contributions: 0x multiplier should fail
			assert_err!(
				test_contribution_setup(&mut inst, project_id, BUYER_1, InvestorType::Professional, 0),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
			);
			// Professional contributions: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				assert_ok!(test_contribution_setup(
					&mut inst,
					project_id,
					BUYER_1,
					InvestorType::Professional,
					multiplier
				));
			}
			// Professional contributions: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				assert_err!(
					test_contribution_setup(&mut inst, project_id, BUYER_1, InvestorType::Professional, multiplier),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			}

			// Institutional contributions: 0x multiplier should fail
			assert_err!(
				test_contribution_setup(&mut inst, project_id, BUYER_2, InvestorType::Institutional, 0),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
			);
			// Institutional contributions: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				assert_ok!(test_contribution_setup(
					&mut inst,
					project_id,
					BUYER_2,
					InvestorType::Institutional,
					multiplier
				));
			}
			// Institutional contributions: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				assert_err!(
					test_contribution_setup(&mut inst, project_id, BUYER_2, InvestorType::Institutional, multiplier),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			}
		}

		#[test]
		fn retail_multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut issuer: AccountId = 6969420;

			let mut create_project = |inst: &mut MockInstantiator| {
				issuer += 1;
				inst.create_community_contributing_project(
					default_project_metadata(issuer),
					issuer,
					default_evaluations(),
					default_bids(),
				)
			};

			let max_allowed_multipliers_map = vec![(2, 1), (4, 2), (9, 4), (24, 7), (25, 10)];

			let mut previous_projects_created = 0;
			for (projects_participated_amount, max_allowed_multiplier) in max_allowed_multipliers_map {
				(previous_projects_created..projects_participated_amount - 1).for_each(|_| {
					let project_id = create_project(&mut inst);
					assert_ok!(test_contribution_setup(&mut inst, project_id, BUYER_1, InvestorType::Retail, 1));
				});

				let project_id = create_project(&mut inst);
				previous_projects_created = projects_participated_amount;

				// 0x multiplier should fail
				assert_err!(
					test_contribution_setup(&mut inst, project_id, BUYER_1, InvestorType::Retail, 0),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);

				// Multipliers that should work
				for multiplier in 1..=max_allowed_multiplier {
					assert_ok!(test_contribution_setup(
						&mut inst,
						project_id,
						BUYER_1,
						InvestorType::Retail,
						multiplier
					));
				}

				// Multipliers that should NOT work
				for multiplier in max_allowed_multiplier + 1..=50 {
					assert_err!(
						test_contribution_setup(&mut inst, project_id, BUYER_1, InvestorType::Retail, multiplier),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
					);
				}
			}
		}

		#[test]
		fn did_with_losing_bid_can_contribute() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let mut evaluations = default_evaluations();
			evaluations.push((BIDDER_4, 1337 * US_DOLLAR).into());

			let successful_bids = vec![
				BidParams::new(BIDDER_1, 400_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
				BidParams::new(BIDDER_2, 100_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			];
			let failing_bids_after_random_end =
				vec![(BIDDER_3, 25_000 * ASSET_UNIT).into(), (BIDDER_4, 25_000 * ASSET_UNIT).into()];
			// This bids should fill the first bucket.
			let failing_bids_sold_out =
				vec![(BIDDER_5, 250_000 * ASSET_UNIT).into(), (BIDDER_6, 250_000 * ASSET_UNIT).into()];

			let all_bids = failing_bids_sold_out
				.iter()
				.chain(successful_bids.iter())
				.chain(failing_bids_after_random_end.iter())
				.cloned()
				.collect_vec();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, default_evaluations());

			let plmc_fundings = MockInstantiator::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids.clone(),
				project_metadata.clone(),
				None,
			);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			let foreign_funding =
				MockInstantiator::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
					&all_bids.clone(),
					project_metadata.clone(),
					None,
				);
			inst.mint_foreign_asset_to(foreign_funding.clone());

			inst.bid_for_users(project_id, failing_bids_sold_out).unwrap();
			inst.bid_for_users(project_id, successful_bids).unwrap();
			inst.advance_time(
				<TestRuntime as Config>::AuctionOpeningDuration::get() +
					<TestRuntime as Config>::AuctionClosingDuration::get() +
					1,
			)
			.unwrap();
			inst.bid_for_users(project_id, failing_bids_after_random_end).unwrap();
			inst.advance_time(2).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

			// Some low amount of plmc and usdt to cover a purchase of 10CTs.
			let plmc_mints = vec![
				(BIDDER_3, 42069 * PLMC).into(),
				(BIDDER_4, 42069 * PLMC).into(),
				(BIDDER_5, 42069 * PLMC).into(),
				(BIDDER_6, 42069 * PLMC).into(),
				(BUYER_3, 42069 * PLMC).into(),
				(BUYER_4, 42069 * PLMC).into(),
				(BUYER_5, 42069 * PLMC).into(),
				(BUYER_6, 42069 * PLMC).into(),
			];
			inst.mint_plmc_to(plmc_mints);
			let usdt_mints = vec![
				(BIDDER_3, 42069 * ASSET_UNIT).into(),
				(BIDDER_4, 42069 * ASSET_UNIT).into(),
				(BIDDER_5, 42069 * ASSET_UNIT).into(),
				(BIDDER_6, 42069 * ASSET_UNIT).into(),
				(BUYER_3, 42069 * ASSET_UNIT).into(),
				(BUYER_4, 42069 * ASSET_UNIT).into(),
				(BUYER_5, 42069 * ASSET_UNIT).into(),
				(BUYER_6, 42069 * ASSET_UNIT).into(),
			];
			inst.mint_foreign_asset_to(usdt_mints);

			let mut bid_should_succeed = |account, investor_type, did_acc| {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(account),
						get_mock_jwt_with_cid(
							account,
							investor_type,
							generate_did_from_account(did_acc),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						10 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					));
				});
			};

			// Bidder 3 has a losing bid due to bidding after the random end. His did should be able to contribute regardless of what investor type
			// or account he uses to sign the transaction
			bid_should_succeed(BIDDER_3, InvestorType::Institutional, BIDDER_3);
			bid_should_succeed(BUYER_3, InvestorType::Institutional, BIDDER_3);
			bid_should_succeed(BIDDER_3, InvestorType::Professional, BIDDER_3);
			bid_should_succeed(BUYER_3, InvestorType::Professional, BIDDER_3);
			bid_should_succeed(BIDDER_3, InvestorType::Retail, BIDDER_3);
			bid_should_succeed(BUYER_3, InvestorType::Retail, BIDDER_3);

			// Bidder 4 has a losing bid due to bidding after the random end, and he was also an evaluator. Same conditions as before should apply.
			bid_should_succeed(BIDDER_4, InvestorType::Institutional, BIDDER_4);
			bid_should_succeed(BUYER_4, InvestorType::Institutional, BIDDER_4);
			bid_should_succeed(BIDDER_4, InvestorType::Professional, BIDDER_4);
			bid_should_succeed(BUYER_4, InvestorType::Professional, BIDDER_4);
			bid_should_succeed(BIDDER_4, InvestorType::Retail, BIDDER_4);
			bid_should_succeed(BUYER_4, InvestorType::Retail, BIDDER_4);

			// Bidder 5 has a losing bid due to CTs being sold out at his price point. Same conditions as before should apply.
			bid_should_succeed(BIDDER_5, InvestorType::Institutional, BIDDER_5);
			bid_should_succeed(BUYER_5, InvestorType::Institutional, BIDDER_5);
			bid_should_succeed(BIDDER_5, InvestorType::Professional, BIDDER_5);
			bid_should_succeed(BUYER_5, InvestorType::Professional, BIDDER_5);
			bid_should_succeed(BIDDER_5, InvestorType::Retail, BIDDER_5);
			bid_should_succeed(BUYER_5, InvestorType::Retail, BIDDER_5);

			// Bidder 6 has a losing bid due to CTs being sold out at his price point, and he was also an evaluator. Same conditions as before should apply.
			bid_should_succeed(BIDDER_6, InvestorType::Institutional, BIDDER_6);
			bid_should_succeed(BUYER_6, InvestorType::Institutional, BIDDER_6);
			bid_should_succeed(BIDDER_6, InvestorType::Professional, BIDDER_6);
			bid_should_succeed(BUYER_6, InvestorType::Professional, BIDDER_6);
			bid_should_succeed(BIDDER_6, InvestorType::Retail, BIDDER_6);
			bid_should_succeed(BUYER_6, InvestorType::Retail, BIDDER_6);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;
		use frame_support::traits::{
			fungible::Mutate as MutateFungible,
			fungibles::Mutate as MutateFungibles,
			tokens::{Fortitude, Precision},
		};

		#[test]
		fn contribution_errors_if_user_limit_is_reached() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_community_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);
			const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

			let project_details = inst.get_project_details(project_id);
			let token_price = project_details.weighted_average_price.unwrap();

			// Create a contribution vector that will reach the limit of contributions for a user-project
			let token_amount: BalanceOf<TestRuntime> = ASSET_UNIT;
			let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
			let contributions: Vec<ContributionParams<_>> = range
				.map(|_| ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT))
				.collect();

			let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
			let plmc_existential_deposits = plmc_funding.accounts().existential_deposits();

			let foreign_funding =
				MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

			inst.mint_plmc_to(plmc_funding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			inst.mint_foreign_asset_to(foreign_funding.clone());

			// Reach up to the limit of contributions for a user-project
			assert!(inst.contribute_for_users(project_id, contributions).is_ok());

			// Try to contribute again, but it should fail because the limit of contributions for a user-project was reached.
			let over_limit_contribution =
				ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT);
			assert!(inst.contribute_for_users(project_id, vec![over_limit_contribution]).is_err());

			// Check that the right amount of PLMC is bonded, and funding currency is transferred
			let contributor_post_buy_plmc_balance =
				inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&CONTRIBUTOR));
			let contributor_post_buy_foreign_asset_balance =
				inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_FOREIGN_ID, CONTRIBUTOR));

			assert_eq!(contributor_post_buy_plmc_balance, MockInstantiator::get_ed());
			assert_eq!(contributor_post_buy_foreign_asset_balance, 0);

			let plmc_bond_stored = inst.execute(|| {
				<TestRuntime as Config>::NativeCurrency::balance_on_hold(
					&HoldReason::Participation(project_id.into()).into(),
					&CONTRIBUTOR,
				)
			});
			let foreign_asset_contributions_stored = inst.execute(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
					.map(|c| c.funding_asset_amount)
					.sum::<BalanceOf<TestRuntime>>()
			});

			assert_eq!(plmc_bond_stored, MockInstantiator::sum_balance_mappings(vec![plmc_funding.clone()]));
			assert_eq!(
				foreign_asset_contributions_stored,
				MockInstantiator::sum_foreign_mappings(vec![foreign_funding.clone()])
			);
		}

		#[test]
		fn issuer_cannot_contribute_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_community_contribute(
					&(&ISSUER_1 + 1),
					project_id,
					500 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				)),
				Error::<TestRuntime>::IssuerError(IssuerErrorReason::ParticipationToOwnProject)
			);
		}

		#[test]
		fn did_with_winning_bid_cannot_contribute() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let mut evaluations = default_evaluations();
			evaluations.push((BIDDER_2, 1337 * US_DOLLAR).into());
			let bids = vec![
				BidParams::new(BIDDER_1, 400_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
				BidParams::new(BIDDER_2, 50_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
				// Partially accepted bid. Only the 50k of the second bid will be accepted.
				BidParams::new(BIDDER_3, 100_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
			];

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				bids,
			);

			let mut bid_should_fail = |account, investor_type, did_acc| {
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::community_contribute(
							RuntimeOrigin::signed(account),
							get_mock_jwt_with_cid(
								account,
								investor_type,
								generate_did_from_account(did_acc),
								project_metadata.clone().policy_ipfs_cid.unwrap()
							),
							project_id,
							10 * ASSET_UNIT,
							1u8.try_into().unwrap(),
							AcceptedFundingAsset::USDT,
						),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::UserHasWinningBid)
					);
				});
			};

			// Bidder 1 has a winning bid, his did should not be able to contribute regardless of what investor type
			// or account he uses to sign the transaction
			bid_should_fail(BIDDER_1, InvestorType::Institutional, BIDDER_1);
			bid_should_fail(BUYER_1, InvestorType::Institutional, BIDDER_1);
			bid_should_fail(BIDDER_1, InvestorType::Professional, BIDDER_1);
			bid_should_fail(BUYER_1, InvestorType::Professional, BIDDER_1);
			bid_should_fail(BIDDER_1, InvestorType::Retail, BIDDER_1);
			bid_should_fail(BUYER_1, InvestorType::Retail, BIDDER_1);

			// Bidder 2 has a winning bid, and he was also an evaluator. Same conditions as before should apply.
			bid_should_fail(BIDDER_2, InvestorType::Institutional, BIDDER_2);
			bid_should_fail(BUYER_2, InvestorType::Institutional, BIDDER_2);
			bid_should_fail(BIDDER_2, InvestorType::Professional, BIDDER_2);
			bid_should_fail(BUYER_2, InvestorType::Professional, BIDDER_2);
			bid_should_fail(BIDDER_2, InvestorType::Retail, BIDDER_2);
			bid_should_fail(BUYER_2, InvestorType::Retail, BIDDER_2);

			// Bidder 3 has a partial winning bid. Same conditions as before should apply.
			bid_should_fail(BIDDER_3, InvestorType::Institutional, BIDDER_3);
			bid_should_fail(BUYER_3, InvestorType::Institutional, BIDDER_3);
			bid_should_fail(BIDDER_3, InvestorType::Professional, BIDDER_3);
			bid_should_fail(BUYER_3, InvestorType::Professional, BIDDER_3);
			bid_should_fail(BIDDER_3, InvestorType::Retail, BIDDER_3);
			bid_should_fail(BUYER_3, InvestorType::Retail, BIDDER_3);
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
				retail: TicketSize::new(Some(10 * US_DOLLAR), None),
				professional: TicketSize::new(Some(100_000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(200_000 * US_DOLLAR), None),
				phantom: Default::default(),
			};

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);

			inst.mint_plmc_to(vec![
				(BUYER_1, 50_000 * ASSET_UNIT).into(),
				(BUYER_2, 50_000 * ASSET_UNIT).into(),
				(BUYER_3, 50_000 * ASSET_UNIT).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_1, 50_000 * US_DOLLAR).into(),
				(BUYER_2, 50_000 * US_DOLLAR).into(),
				(BUYER_3, 50_000 * US_DOLLAR).into(),
			]);

			// contribution below 1 CT (10 USD) should fail for retail
			let jwt = get_mock_jwt_with_cid(
				BUYER_1,
				InvestorType::Retail,
				generate_did_from_account(BUYER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt,
						project_id,
						ASSET_UNIT / 2,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
			// contribution below 10_000 CT (100k USD) should fail for professionals
			let jwt = get_mock_jwt_with_cid(
				BUYER_2,
				InvestorType::Professional,
				generate_did_from_account(BUYER_2),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_2),
						jwt,
						project_id,
						9_999,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});

			// contribution below 20_000 CT (200k USD) should fail for institutionals
			let jwt = get_mock_jwt_with_cid(
				BUYER_3,
				InvestorType::Professional,
				generate_did_from_account(BUYER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_3),
						jwt,
						project_id,
						19_999,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
		}

		#[test]
		fn per_credential_type_ticket_size_maximums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
				retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
				professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
				institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
				phantom: Default::default(),
			};

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);

			inst.mint_plmc_to(vec![
				(BUYER_1, 500_000 * ASSET_UNIT).into(),
				(BUYER_2, 500_000 * ASSET_UNIT).into(),
				(BUYER_3, 500_000 * ASSET_UNIT).into(),
				(BUYER_4, 500_000 * ASSET_UNIT).into(),
				(BUYER_5, 500_000 * ASSET_UNIT).into(),
				(BUYER_6, 500_000 * ASSET_UNIT).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_1, 500_000 * US_DOLLAR).into(),
				(BUYER_2, 500_000 * US_DOLLAR).into(),
				(BUYER_3, 500_000 * US_DOLLAR).into(),
				(BUYER_4, 500_000 * US_DOLLAR).into(),
				(BUYER_5, 500_000 * US_DOLLAR).into(),
				(BUYER_6, 500_000 * US_DOLLAR).into(),
			]);

			let buyer_1_jwt = get_mock_jwt_with_cid(
				BUYER_1,
				InvestorType::Retail,
				generate_did_from_account(BUYER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let buyer_2_jwt_same_did = get_mock_jwt_with_cid(
				BUYER_2,
				InvestorType::Retail,
				generate_did_from_account(BUYER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total contributions with same DID above 10k CT (100k USD) should fail for retail
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_1),
					buyer_1_jwt,
					project_id,
					9000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_2),
						buyer_2_jwt_same_did.clone(),
						project_id,
						1001 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_2),
					buyer_2_jwt_same_did,
					project_id,
					1000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});

			let buyer_3_jwt = get_mock_jwt_with_cid(
				BUYER_3,
				InvestorType::Professional,
				generate_did_from_account(BUYER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let buyer_4_jwt_same_did = get_mock_jwt_with_cid(
				BUYER_4,
				InvestorType::Professional,
				generate_did_from_account(BUYER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total contributions with same DID above 2k CT (20k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_3),
					buyer_3_jwt,
					project_id,
					1800 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_4),
						buyer_4_jwt_same_did.clone(),
						project_id,
						201 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_4),
					buyer_4_jwt_same_did,
					project_id,
					200 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});

			let buyer_5_jwt = get_mock_jwt_with_cid(
				BUYER_5,
				InvestorType::Institutional,
				generate_did_from_account(BUYER_5),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let buyer_6_jwt_same_did = get_mock_jwt_with_cid(
				BUYER_6,
				InvestorType::Institutional,
				generate_did_from_account(BUYER_5),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_5),
					buyer_5_jwt,
					project_id,
					4690 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_6),
						buyer_6_jwt_same_did.clone(),
						project_id,
						311 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 5k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_6),
					buyer_6_jwt_same_did,
					project_id,
					310 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn insufficient_funds() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);

			let jwt = get_mock_jwt_with_cid(
				BUYER_1,
				InvestorType::Retail,
				generate_did_from_account(BUYER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let contribution = ContributionParams::new(BUYER_1, 1_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			// 1 unit less native asset than needed
			let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], wap);
			let plmc_existential_deposits = plmc_funding.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_funding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			inst.execute(|| Balances::burn_from(&BUYER_1, 1, Precision::BestEffort, Fortitude::Force)).unwrap();

			let foreign_funding =
				MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], wap);
			inst.mint_foreign_asset_to(foreign_funding.clone());
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt.clone(),
						project_id,
						1_000 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::NotEnoughFunds)
				);
			});

			// 1 unit less funding asset than needed
			let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], wap);
			let plmc_existential_deposits = plmc_funding.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_funding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());
			let foreign_funding =
				MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], wap);

			inst.execute(|| ForeignAssets::set_balance(AcceptedFundingAsset::USDT.to_assethub_id(), &BUYER_1, 0));
			inst.mint_foreign_asset_to(foreign_funding.clone());

			inst.execute(|| {
				ForeignAssets::burn_from(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					&BUYER_1,
					100,
					Precision::BestEffort,
					Fortitude::Force,
				)
			})
			.unwrap();

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::community_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt,
						project_id,
						1_000 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::NotEnoughFunds)
				);
			});
		}

		#[test]
		fn called_outside_community_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let created_project = inst.create_new_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluating_project = inst.create_evaluating_project(default_project_metadata(ISSUER_2), ISSUER_2);
			let auctioning_project =
				inst.create_auctioning_project(default_project_metadata(ISSUER_3), ISSUER_3, default_evaluations());
			let remaining_project = inst.create_remainder_contributing_project(
				default_project_metadata(ISSUER_4),
				ISSUER_4,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
			);
			let finished_project = inst.create_finished_project(
				default_project_metadata(ISSUER_5),
				ISSUER_5,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
				default_remainder_buys(),
			);

			let projects =
				vec![created_project, evaluating_project, auctioning_project, remaining_project, finished_project];
			for project in projects {
				let project_policy = inst.get_project_metadata(project).policy_ipfs_cid.unwrap();
				inst.execute(|| {
					assert_noop!(
						PolimecFunding::community_contribute(
							RuntimeOrigin::signed(BUYER_1),
							get_mock_jwt_with_cid(
								BUYER_1,
								InvestorType::Retail,
								generate_did_from_account(BUYER_1),
								project_policy
							),
							project,
							1000 * ASSET_UNIT,
							1u8.try_into().unwrap(),
							AcceptedFundingAsset::USDT
						),
						Error::<TestRuntime>::ProjectRoundError(RoundError::IncorrectRound)
					);
				});
			}
		}

		#[test]
		fn contribute_with_unaccepted_currencies() {
			let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata_usdt = default_project_metadata(ISSUER_2);
			project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

			let mut project_metadata_usdc = default_project_metadata(ISSUER_3);
			project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

			let evaluations = default_evaluations();

			let usdt_bids = default_bids()
				.into_iter()
				.map(|mut b| {
					b.asset = AcceptedFundingAsset::USDT;
					b
				})
				.collect::<Vec<_>>();

			let usdc_bids = default_bids()
				.into_iter()
				.map(|mut b| {
					b.asset = AcceptedFundingAsset::USDC;
					b
				})
				.collect::<Vec<_>>();

			let projects = vec![
				TestProjectParams {
					expected_state: ProjectStatus::CommunityRound,
					metadata: project_metadata_usdt,
					issuer: ISSUER_2,
					evaluations: evaluations.clone(),
					bids: usdt_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::CommunityRound,
					metadata: project_metadata_usdc,
					issuer: ISSUER_3,
					evaluations: evaluations.clone(),
					bids: usdc_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
			];
			let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

			let project_id_usdt = project_ids[0];
			let project_id_usdc = project_ids[1];

			let usdt_contribution =
				ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let usdc_contribution =
				ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
			let dot_contribution =
				ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

			assert_err!(
				inst.contribute_for_users(project_id_usdc, vec![usdt_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.contribute_for_users(project_id_usdt, vec![usdc_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.contribute_for_users(project_id_usdt, vec![dot_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
		}

		#[test]
		fn cannot_use_evaluation_bond_on_another_project_contribution() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			let mut evaluations_1 = default_evaluations();
			let evaluations_2 = default_evaluations();

			let evaluator_contributor = 69;
			let evaluation_amount = 420 * US_DOLLAR;
			let evaluator_contribution =
				ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			evaluations_1.push((evaluator_contributor, evaluation_amount).into());

			let _project_id_1 = inst.create_community_contributing_project(
				project_metadata_1.clone(),
				ISSUER_1,
				evaluations_1,
				default_bids(),
			);
			let project_id_2 = inst.create_community_contributing_project(
				project_metadata_2.clone(),
				ISSUER_2,
				evaluations_2,
				default_bids(),
			);

			let wap = inst.get_project_details(project_id_2).weighted_average_price.unwrap();

			// Necessary Mints
			let already_bonded_plmc =
				MockInstantiator::calculate_evaluation_plmc_spent(vec![
					(evaluator_contributor, evaluation_amount).into()
				])[0]
					.plmc_amount;
			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_contribution =
				MockInstantiator::calculate_contributed_plmc_spent(vec![evaluator_contribution.clone()], wap)[0]
					.plmc_amount;
			let necessary_usdt_for_contribution =
				MockInstantiator::calculate_contributed_funding_asset_spent(vec![evaluator_contribution.clone()], wap);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_contributor,
				necessary_plmc_for_contribution - usable_evaluation_plmc,
			)]);
			inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::community_contribute(
						RuntimeOrigin::signed(evaluator_contributor),
						get_mock_jwt_with_cid(
							evaluator_contributor,
							InvestorType::Retail,
							generate_did_from_account(evaluator_contributor),
							project_metadata_2.clone().policy_ipfs_cid.unwrap()
						),
						project_id_2,
						evaluator_contribution.amount,
						evaluator_contribution.multiplier,
						evaluator_contribution.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::NotEnoughFunds)
				);
			});
		}

		#[test]
		fn wrong_policy_on_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
			);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::community_contribute(
						RuntimeOrigin::signed(BUYER_1),
						get_mock_jwt_with_cid(
							BUYER_1,
							InvestorType::Retail,
							generate_did_from_account(BUYER_1),
							"wrong_cid".as_bytes().to_vec().try_into().unwrap()
						),
						project_id,
						5000 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::PolicyMismatch)
				);
			});
		}
	}
}
