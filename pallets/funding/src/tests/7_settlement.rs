// use super::*;
//
// #[cfg(test)]
// mod round_flow {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn can_fully_settle_accepted_project() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
// 			let evaluations = inst.get_evaluations(project_id);
// 			let bids = inst.get_bids(project_id);
// 			let contributions = inst.get_contributions(project_id);
//
// 			inst.settle_project(project_id).unwrap();
//
// 			inst.assert_total_funding_paid_out(project_id, bids.clone(), contributions.clone());
// 			inst.assert_evaluations_migrations_created(project_id, evaluations, percentage);
// 			inst.assert_bids_migrations_created(project_id, bids, true);
// 			inst.assert_contributions_migrations_created(project_id, contributions, true);
// 		}
//
// 		#[test]
// 		fn can_fully_settle_failed_project() {
// 			let percentage = 33u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
// 			let evaluations = inst.get_evaluations(project_id);
// 			let bids = inst.get_bids(project_id);
// 			let contributions = inst.get_contributions(project_id);
//
// 			inst.settle_project(project_id).unwrap();
//
// 			inst.assert_evaluations_migrations_created(project_id, evaluations, percentage);
// 			inst.assert_bids_migrations_created(project_id, bids, false);
// 			inst.assert_contributions_migrations_created(project_id, contributions, false);
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_successful_evaluation_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn evaluation_unchanged() {
// 			let percentage = 89u64;
//
// 			let (mut inst, project_id) =
// 				create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::AcceptFunding), true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
// 			let prev_balance = inst.get_free_plmc_balance_for(evaluator);
//
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcomeOf::<TestRuntime>::Unchanged
// 			);
//
// 			assert_ok!(inst.execute(|| PolimecFunding::settle_successful_evaluation(
// 				RuntimeOrigin::signed(evaluator),
// 				project_id,
// 				evaluator,
// 				first_evaluation.id
// 			)));
//
// 			let post_balance = inst.get_free_plmc_balance_for(evaluator);
// 			assert_eq!(post_balance, prev_balance + first_evaluation.current_plmc_bond);
// 		}
//
// 		#[test]
// 		fn evaluation_slashed() {
// 			let percentage = 50u64;
// 			let (mut inst, project_id) =
// 				create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::AcceptFunding), true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
// 			let prev_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;
//
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcomeOf::<TestRuntime>::Slashed
// 			);
//
// 			assert_ok!(inst.execute(|| PolimecFunding::settle_successful_evaluation(
// 				RuntimeOrigin::signed(evaluator),
// 				project_id,
// 				evaluator,
// 				first_evaluation.id
// 			)));
//
// 			let post_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;
// 			assert_eq!(
// 				post_balance,
// 				prev_balance +
// 					(Percent::from_percent(100) - <TestRuntime as Config>::EvaluatorSlash::get()) *
// 						first_evaluation.current_plmc_bond
// 			);
// 		}
//
// 		#[test]
// 		fn evaluation_rewarded() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let project_id = inst.create_finished_project(
// 				project_metadata.clone(),
// 				ISSUER_1,
// 				None,
// 				vec![
// 					UserToUSDBalance::new(EVALUATOR_1, 500_000 * USD_UNIT),
// 					UserToUSDBalance::new(EVALUATOR_2, 250_000 * USD_UNIT),
// 					UserToUSDBalance::new(EVALUATOR_3, 320_000 * USD_UNIT),
// 				],
// 				inst.generate_bids_from_total_ct_percent(
// 					project_metadata.clone(),
// 					50,
// 					default_weights(),
// 					default_bidders(),
// 					default_multipliers(),
// 				),
// 				inst.generate_contributions_from_total_ct_percent(
// 					project_metadata.clone(),
// 					50,
// 					default_weights(),
// 					default_community_contributors(),
// 					default_community_contributor_multipliers(),
// 				),
// 				vec![],
// 			);
// 			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
// 			inst.jump_to_block(settlement_block);
//
// 			// The rewards are calculated as follows:
// 			// Data:
// 			// - Funding USD reached: 10_000_000 USD
// 			// - Total CTs sold: 1_000_000 CT
// 			// - USD target reached percent: 100%
//
// 			// Step 1) Calculate the total USD fee:
// 			// USD fee 1 = 0.1 * 1_000_000 = 100_000 USD
// 			// USD fee 2 = 0.08 * 4_000_000 = 320_000 USD
// 			// USD fee 3 = 0.06 * 5_000_000 = 300_000 USD
// 			// Total USD fee = 100_000 + 320_000 + 300_000 = 720_000 USD
//
// 			// Step 2) Calculate CT fee as follows:
// 			// Percent fee = Total USD fee / Funding USD reached = 720_000 / 10_000_000 = 0.072
// 			// CT fee = Percent fee * Total CTs sold = 0.072 * 1_000_000 = 72_000 CT
//
// 			// Step 3) Calculate Early and Normal evaluator reward pots:
// 			// Total evaluators reward pot = CT fee * 0.3 * USD target reached percent = 72_000 * 0.3 * 1 = 21_600 CT
// 			// Early evaluators reward pot = Total evaluators reward pot * 0.2 = 21_600 * 0.2 = 4_320 CT
// 			// Normal evaluators reward pot = Total evaluators reward pot * 0.8 = 21_600 * 0.8 = 17_280 CT
//
// 			// Step 4) Calculate the early and normal weights of each evaluation:
// 			// Evaluation 1 = 500_000 USD
// 			// Evaluation 2 = 250_000 USD
// 			// Evaluation 3 = 320_000 USD
//
// 			// Early amount 1 = 500_000 USD
// 			// Early amount 2 = 250_000 USD
// 			// Early amount 3 = 250_000 USD
//
// 			// Total Normal amount = Evaluation 1 + Evaluation 2 + Evaluation 3 = 500_000 + 250_000 + 320_000 = 1_070_000 USD
// 			// Total Early amount = 10% of USD target = 1_000_000 USD
//
// 			// Early weight 1 = Early amount 1 / Total Early amount = 500_000 / 1_000_000 = 0.5
// 			// Early weight 2 = Early amount 2 / Total Early amount = 250_000 / 1_000_000 = 0.25
// 			// Early weight 3 = Early amount 3 / Total Early amount = 250_000 / 1_000_000 = 0.25
//
// 			// Normal weight 1 = Evaluation 1 / Total Normal amount = 500_000 / 1_070_000 = 0.467289719626168
// 			// Normal weight 2 = Evaluation 2 / Total Normal amount = 250_000 / 1_070_000 = 0.233644859813084
// 			// Normal weight 3 = Evaluation 3 / Total Normal amount = 320_000 / 1_070_000 = 0.299065420560748
//
// 			// Step 5) Calculate the rewards for each evaluation:
// 			// Evaluation 1 Early reward = Early weight 1 * Early evaluators reward pot = 0.5 * 4_320 = 2_160 CT
// 			// Evaluation 2 Early reward = Early weight 2 * Early evaluators reward pot = 0.25 * 4_320 = 1_080 CT
// 			// Evaluation 3 Early reward = Early weight 3 * Early evaluators reward pot = 0.25 * 4_320 = 1_080 CT
//
// 			// Evaluation 1 Normal reward = Normal weight 1 * Normal evaluators reward pot = 0.467289719626168 * 17_280 = 8'074.766355140186916 CT
// 			// Evaluation 2 Normal reward = Normal weight 2 * Normal evaluators reward pot = 0.233644859813084 * 17_280 = 4'037.383177570093458 CT
// 			// Evaluation 3 Normal reward = Normal weight 3 * Normal evaluators reward pot = 0.299065420560748 * 17_280 = 5'167.850467289719626 CT
//
// 			// Evaluation 1 Total reward = Evaluation 1 Early reward + Evaluation 1 Normal reward = 2_160 + 8_066 = 10'234.766355140186916 CT
// 			// Evaluation 2 Total reward = Evaluation 2 Early reward + Evaluation 2 Normal reward = 1_080 + 4_033 = 5'117.383177570093458 CT
// 			// Evaluation 3 Total reward = Evaluation 3 Early reward + Evaluation 3 Normal reward = 1_080 + 5_201 = 6'247.850467289719626 CT
//
// 			const EVAL_1_REWARD: u128 = 10_234_766355140186916;
// 			const EVAL_2_REWARD: u128 = 5_117_383177570093458;
// 			const EVAL_3_REWARD: u128 = 6_247_850467289719626;
//
// 			let prev_ct_balances = inst.get_ct_asset_balances_for(project_id, vec![ISSUER_1, ISSUER_2, ISSUER_3]);
// 			assert!(prev_ct_balances.iter().all(|x| *x == Zero::zero()));
//
// 			let evals = vec![(EVALUATOR_1, EVAL_1_REWARD), (EVALUATOR_2, EVAL_2_REWARD), (EVALUATOR_3, EVAL_3_REWARD)];
//
// 			for (evaluator, expected_reward) in evals {
// 				let evaluation_locked_plmc =
// 					inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation(project_id).into());
// 				let free_plmc = inst.get_free_plmc_balance_for(evaluator);
// 				assert_ok!(inst.execute(|| PolimecFunding::settle_successful_evaluation(
// 					RuntimeOrigin::signed(evaluator),
// 					project_id,
// 					evaluator,
// 					evaluator - 21 // The First evaluation index is 0, the first evaluator account is 21
// 				)));
// 				let ct_rewarded = inst.get_ct_asset_balance_for(project_id, evaluator);
// 				assert_close_enough!(ct_rewarded, expected_reward, Perquintill::from_float(0.9999));
// 				assert_eq!(inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation(project_id).into()), 0);
// 				assert_eq!(inst.get_free_plmc_balance_for(evaluator), free_plmc + evaluation_locked_plmc);
// 				inst.assert_migration(project_id, evaluator, expected_reward, 0, ParticipationType::Evaluation, true);
// 			}
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let evaluator = first_evaluation.evaluator;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_evaluation(
// 					RuntimeOrigin::signed(evaluator),
// 					project_id,
// 					evaluator,
// 					first_evaluation.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
//
// 			inst.execute(|| {
// 				assert_noop!(
// 					PolimecFunding::settle_successful_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
//
// 			inst.execute(|| {
// 				assert_noop!(
// 					PolimecFunding::settle_successful_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_successful_bid_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn bid_is_correctly_settled() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let issuer = ISSUER_1;
// 			let evaluations =
// 				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
// 			let bid_1 = BidParams::new(BIDDER_1, 1000 * CT_UNIT, 1, AcceptedFundingAsset::USDT);
// 			let bid_2 = BidParams::new(BIDDER_2, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);
//
// 			let community_contributions = inst.generate_contributions_from_total_ct_percent(
// 				project_metadata.clone(),
// 				90,
// 				default_weights(),
// 				default_community_contributors(),
// 				default_community_contributor_multipliers(),
// 			);
//
// 			let project_id = inst.create_finished_project(
// 				project_metadata.clone(),
// 				issuer,
// 				None,
// 				evaluations,
// 				vec![bid_1, bid_2],
// 				community_contributions,
// 				vec![],
// 			);
//
// 			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
// 			inst.jump_to_block(settlement_block);
//
// 			// First bid assertions
// 			let stored_bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_1);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_1, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_1);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_1,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_bid.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance, 0u128);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_successful_bid(
// 					RuntimeOrigin::signed(BIDDER_1),
// 					project_id,
// 					BIDDER_1,
// 					0
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BIDDER_1, 0)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_1);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_1, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_1);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_1,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_bid.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			assert_eq!(ct_amount, stored_bid.final_ct_amount);
// 			assert_eq!(
// 				issuer_usdt_balance,
// 				stored_bid.final_ct_usd_price.saturating_mul_int(stored_bid.final_ct_amount)
// 			);
// 			assert!(unvested_amount.is_none());
// 			inst.assert_migration(project_id, BIDDER_1, stored_bid.final_ct_amount, 0, ParticipationType::Bid, true);
//
// 			// Second bid assertions
// 			let stored_bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_2, 1)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_2);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_2, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_2);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_2,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_bid.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_successful_bid(
// 					RuntimeOrigin::signed(BIDDER_2),
// 					project_id,
// 					BIDDER_2,
// 					1
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BIDDER_2, 1)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_2);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_2, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_2);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst
// 				.execute(|| {
// 					<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 						&BIDDER_2,
// 						HoldReason::Participation(project_id).into(),
// 					)
// 				})
// 				.unwrap();
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_bid.plmc_bond);
// 			assert_eq!(ct_amount, stored_bid.final_ct_amount);
// 			assert_eq!(
// 				issuer_usdt_balance_2,
// 				issuer_usdt_balance + stored_bid.final_ct_usd_price.saturating_mul_int(stored_bid.final_ct_amount)
// 			);
// 			assert_eq!(unvested_amount, stored_bid.plmc_bond);
//
// 			let vesting_time = stored_bid.multiplier.calculate_vesting_duration::<TestRuntime>();
// 			let now = inst.current_block();
// 			inst.jump_to_block(vesting_time + now + 1u64);
// 			inst.execute(|| {
// 				assert_ok!(<TestRuntime as Config>::Vesting::vest(
// 					RuntimeOrigin::signed(BIDDER_2),
// 					HoldReason::Participation(project_id).into()
// 				));
// 			});
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_2,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert!(unvested_amount.is_none());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_2);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_2, HoldReason::Participation(project_id).into());
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_bid.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			inst.assert_migration(project_id, BIDDER_2, stored_bid.final_ct_amount, 1, ParticipationType::Bid, true);
// 		}
//
// 		#[test]
// 		fn rejected_bids_dont_get_vest_schedule() {
// 			// * Test Setup *
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
//
// 			// Project variables
// 			let issuer = ISSUER_1;
// 			let project_metadata = default_project_metadata(issuer);
// 			let evaluations = default_evaluations();
// 			let auction_token_allocation =
// 				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
// 			let mut bids = inst.generate_bids_from_total_usd(
// 				Percent::from_percent(80) * project_metadata.minimum_price.saturating_mul_int(auction_token_allocation),
// 				project_metadata.minimum_price,
// 				vec![60, 40],
// 				vec![BIDDER_1, BIDDER_2],
// 				vec![1u8, 1u8],
// 			);
// 			let community_contributions = default_community_buys();
//
// 			// Add rejected and accepted bids to test our vesting schedule assertions
// 			let available_tokens =
// 				auction_token_allocation.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));
//
// 			let rejected_bid = vec![BidParams::new(BIDDER_5, available_tokens, 1u8, AcceptedFundingAsset::USDT)];
// 			let accepted_bid = vec![BidParams::new(BIDDER_4, available_tokens, 2u8, AcceptedFundingAsset::USDT)];
// 			bids.extend(rejected_bid.clone());
// 			bids.extend(accepted_bid.clone());
//
// 			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);
//
// 			// Mint the necessary bidding balances
// 			let bidders_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
// 				&bids,
// 				project_metadata.clone(),
// 				None,
// 				true,
// 			);
// 			let bidders_existential_deposits = bidders_plmc.accounts().existential_deposits();
// 			inst.mint_plmc_to(bidders_plmc.clone());
// 			inst.mint_plmc_to(bidders_existential_deposits);
// 			let bidders_funding_assets = inst
// 				.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
// 					&bids,
// 					project_metadata.clone(),
// 					None,
// 				);
// 			inst.mint_foreign_asset_to(bidders_funding_assets);
//
// 			inst.bid_for_users(project_id, bids).unwrap();
//
// 			inst.start_community_funding(project_id).unwrap();
//
// 			// Mint the necessary community contribution balances
// 			let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
// 			let contributors_plmc =
// 				inst.calculate_contributed_plmc_spent(community_contributions.clone(), final_price, false);
// 			let contributors_existential_deposits = contributors_plmc.accounts().existential_deposits();
// 			inst.mint_plmc_to(contributors_plmc.clone());
// 			inst.mint_plmc_to(contributors_existential_deposits);
// 			let contributors_funding_assets =
// 				inst.calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
// 			inst.mint_foreign_asset_to(contributors_funding_assets);
//
// 			inst.contribute_for_users(project_id, community_contributions).unwrap();
//
// 			// Finish and Settle project
// 			inst.start_remainder_or_end_funding(project_id).unwrap();
// 			inst.finish_funding(project_id, None).unwrap();
// 			inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 			inst.settle_project(project_id).unwrap();
//
// 			let plmc_locked_for_accepted_bid =
// 				inst.calculate_auction_plmc_charged_with_given_price(&accepted_bid, final_price, false);
// 			let plmc_locked_for_rejected_bid =
// 				inst.calculate_auction_plmc_charged_with_given_price(&rejected_bid, final_price, false);
//
// 			let UserToPLMCBalance { account: accepted_user, plmc_amount: accepted_plmc_amount } =
// 				plmc_locked_for_accepted_bid[0];
// 			let UserToPLMCBalance { account: rejected_user, .. } = plmc_locked_for_rejected_bid[0];
//
// 			// * Assertions *
// 			let schedule = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&accepted_user,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_close_enough!(schedule.unwrap(), accepted_plmc_amount, Perquintill::from_float(0.99));
// 			assert!(inst
// 				.execute(|| {
// 					<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 						&rejected_user,
// 						HoldReason::Participation(project_id).into(),
// 					)
// 				})
// 				.is_none());
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let bidder = first_bid.bidder;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_bid(
// 					RuntimeOrigin::signed(bidder),
// 					project_id,
// 					bidder,
// 					first_bid.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			let bidder = first_bid.bidder;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
//
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			let bidder = first_bid.bidder;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_successful_contribution_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn contribution_is_correctly_settled() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let issuer = ISSUER_1;
// 			let evaluations =
// 				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
// 			let bids = inst.generate_bids_from_total_ct_percent(
// 				project_metadata.clone(),
// 				50,
// 				default_weights(),
// 				default_bidders(),
// 				default_multipliers(),
// 			);
// 			let mut community_contributions = inst.generate_contributions_from_total_ct_percent(
// 				project_metadata.clone(),
// 				40,
// 				default_weights(),
// 				default_community_contributors(),
// 				default_community_contributor_multipliers(),
// 			);
//
// 			let contribution_mul_1 =
// 				ContributionParams::<TestRuntime>::new(BUYER_6, 1000 * CT_UNIT, 1, AcceptedFundingAsset::USDT);
// 			let contribution_mul_2 =
// 				ContributionParams::<TestRuntime>::new(BUYER_7, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);
//
// 			community_contributions.push(contribution_mul_1);
//
// 			let project_id = inst.create_remainder_contributing_project(
// 				project_metadata.clone(),
// 				issuer,
// 				None,
// 				evaluations,
// 				bids,
// 				community_contributions,
// 			);
// 			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
//
// 			let plmc_required = inst.calculate_contributed_plmc_spent(vec![contribution_mul_2.clone()], wap, false);
// 			let plmc_ed = plmc_required.accounts().existential_deposits();
// 			inst.mint_plmc_to(plmc_required.clone());
// 			inst.mint_plmc_to(plmc_ed);
//
// 			let usdt_required = inst.calculate_contributed_funding_asset_spent(vec![contribution_mul_2.clone()], wap);
// 			inst.mint_foreign_asset_to(usdt_required.clone());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::contribute(
// 					RuntimeOrigin::signed(BUYER_7),
// 					get_mock_jwt_with_cid(
// 						BUYER_7,
// 						InvestorType::Professional,
// 						generate_did_from_account(BUYER_7),
// 						project_metadata.clone().policy_ipfs_cid.unwrap(),
// 					),
// 					project_id,
// 					contribution_mul_2.amount,
// 					contribution_mul_2.multiplier,
// 					contribution_mul_2.asset
// 				));
// 			});
//
// 			inst.finish_funding(project_id, None).unwrap();
// 			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
// 			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
// 			inst.jump_to_block(settlement_block);
//
// 			// First contribution assertions
// 			let stored_contribution =
// 				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 5)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_6,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance, 0u128);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_successful_contribution(
// 					RuntimeOrigin::signed(BUYER_6),
// 					project_id,
// 					BUYER_6,
// 					5
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 6)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_6,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			assert_eq!(ct_amount, stored_contribution.ct_amount);
// 			assert_eq!(issuer_usdt_balance, stored_contribution.usd_contribution_amount);
// 			assert!(unvested_amount.is_none());
// 			inst.assert_migration(
// 				project_id,
// 				BUYER_6,
// 				stored_contribution.ct_amount,
// 				5,
// 				ParticipationType::Contribution,
// 				true,
// 			);
//
// 			// Second contribution assertions
// 			let stored_contribution =
// 				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 6)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_7,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_successful_contribution(
// 					RuntimeOrigin::signed(BUYER_7),
// 					project_id,
// 					BUYER_7,
// 					6
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 7)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst
// 				.execute(|| {
// 					<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 						&BUYER_7,
// 						HoldReason::Participation(project_id).into(),
// 					)
// 				})
// 				.unwrap();
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
// 			assert_eq!(ct_amount, stored_contribution.ct_amount);
// 			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance + stored_contribution.usd_contribution_amount);
// 			assert_eq!(unvested_amount, stored_contribution.plmc_bond);
//
// 			let vesting_time = stored_contribution.multiplier.calculate_vesting_duration::<TestRuntime>();
// 			let now = inst.current_block();
// 			inst.jump_to_block(vesting_time + now + 1u64);
// 			inst.execute(|| {
// 				assert_ok!(<TestRuntime as Config>::Vesting::vest(
// 					RuntimeOrigin::signed(BUYER_7),
// 					HoldReason::Participation(project_id).into()
// 				));
// 			});
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_7,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert!(unvested_amount.is_none());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			inst.assert_migration(
// 				project_id,
// 				BUYER_7,
// 				stored_contribution.ct_amount,
// 				6,
// 				ParticipationType::Contribution,
// 				true,
// 			);
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let contributor = first_contribution.contributor;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_contribution(
// 					RuntimeOrigin::signed(contributor),
// 					project_id,
// 					contributor,
// 					first_contribution.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			let contributor = first_contribution.contributor;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			let contributor = first_contribution.contributor;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_successful_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::FundingSuccessSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_failed_evaluation_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn evaluation_unchanged() {
// 			let percentage = 89u64;
//
// 			let (mut inst, project_id) =
// 				create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::RejectFunding), true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
// 			let prev_balance = inst.get_free_plmc_balance_for(evaluator);
//
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcomeOf::<TestRuntime>::Unchanged
// 			);
//
// 			assert_ok!(inst.execute(|| PolimecFunding::settle_failed_evaluation(
// 				RuntimeOrigin::signed(evaluator),
// 				project_id,
// 				evaluator,
// 				first_evaluation.id
// 			)));
//
// 			let post_balance = inst.get_free_plmc_balance_for(evaluator);
// 			assert_eq!(post_balance, prev_balance + first_evaluation.current_plmc_bond);
// 		}
//
// 		#[test]
// 		fn evaluation_slashed() {
// 			let percentage = 50u64;
// 			let (mut inst, project_id) =
// 				create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::RejectFunding), true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
// 			let prev_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;
//
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcomeOf::<TestRuntime>::Slashed
// 			);
//
// 			assert_ok!(inst.execute(|| PolimecFunding::settle_failed_evaluation(
// 				RuntimeOrigin::signed(evaluator),
// 				project_id,
// 				evaluator,
// 				first_evaluation.id
// 			)));
//
// 			let post_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;
// 			assert_eq!(
// 				post_balance,
// 				prev_balance +
// 					(Percent::from_percent(100) - <TestRuntime as Config>::EvaluatorSlash::get()) *
// 						first_evaluation.current_plmc_bond
// 			);
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 33u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let evaluator = first_evaluation.evaluator;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_evaluation(
// 					RuntimeOrigin::signed(evaluator),
// 					project_id,
// 					evaluator,
// 					first_evaluation.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
//
// 			inst.execute(|| {
// 				assert_noop!(
// 					PolimecFunding::settle_failed_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
//
// 			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
// 			let evaluator = first_evaluation.evaluator;
//
// 			inst.execute(|| {
// 				assert_noop!(
// 					PolimecFunding::settle_failed_evaluation(
// 						RuntimeOrigin::signed(evaluator),
// 						project_id,
// 						evaluator,
// 						first_evaluation.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_failed_bid_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn bid_is_correctly_settled() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let issuer = ISSUER_1;
// 			let evaluations =
// 				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
// 			let bid_1 = BidParams::new(BIDDER_1, 1000 * CT_UNIT, 1, AcceptedFundingAsset::USDT);
// 			let bid_2 = BidParams::new(BIDDER_2, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);
//
// 			let community_contributions = inst.generate_contributions_from_total_ct_percent(
// 				project_metadata.clone(),
// 				20,
// 				default_weights(),
// 				default_community_contributors(),
// 				default_community_contributor_multipliers(),
// 			);
//
// 			let project_id = inst.create_finished_project(
// 				project_metadata.clone(),
// 				issuer,
// 				None,
// 				evaluations,
// 				vec![bid_1, bid_2],
// 				community_contributions,
// 				vec![],
// 			);
// 			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
// 			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
// 			inst.jump_to_block(settlement_block);
//
// 			// First bid assertions
// 			let stored_bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_1);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_1, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_1);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_1,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_bid.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance, 0u128);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_failed_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BIDDER_1, 0)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_1);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_1, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_1);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_1,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_bid.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			assert_eq!(ct_amount, Zero::zero());
// 			assert_eq!(issuer_usdt_balance, Zero::zero());
// 			assert!(unvested_amount.is_none());
// 			inst.assert_migration(project_id, BIDDER_1, stored_bid.final_ct_amount, 0, ParticipationType::Bid, false);
//
// 			// Second bid assertions
// 			let stored_bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_2, 1)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_2);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_2, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_2);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_2,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_bid.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_failed_bid(RuntimeOrigin::signed(BIDDER_2), project_id, BIDDER_2, 1));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BIDDER_2, 1)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BIDDER_2);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BIDDER_2, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BIDDER_2);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_bid.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BIDDER_2,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_bid.plmc_bond);
// 			assert_eq!(plmc_held_amount, Zero::zero());
// 			assert_eq!(ct_amount, Zero::zero());
// 			assert_eq!(issuer_usdt_balance_2, Zero::zero());
// 			assert!(unvested_amount.is_none());
//
// 			inst.assert_migration(project_id, BIDDER_2, stored_bid.final_ct_amount, 1, ParticipationType::Bid, false);
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 33u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let bidder = first_bid.bidder;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_bid(
// 					RuntimeOrigin::signed(bidder),
// 					project_id,
// 					bidder,
// 					first_bid.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			let bidder = first_bid.bidder;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
//
// 			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
// 			let bidder = first_bid.bidder;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_bid(
// 						RuntimeOrigin::signed(bidder),
// 						project_id,
// 						bidder,
// 						first_bid.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
//
// #[cfg(test)]
// mod settle_failed_contribution_extrinsic {
// 	use super::*;
//
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn contribution_is_correctly_settled() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let issuer = ISSUER_1;
// 			let evaluations =
// 				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
// 			let bids = inst.generate_bids_from_total_ct_percent(
// 				project_metadata.clone(),
// 				10,
// 				default_weights(),
// 				default_bidders(),
// 				default_multipliers(),
// 			);
// 			let mut community_contributions = inst.generate_contributions_from_total_ct_percent(
// 				project_metadata.clone(),
// 				10,
// 				default_weights(),
// 				default_community_contributors(),
// 				default_community_contributor_multipliers(),
// 			);
//
// 			let contribution_mul_1 =
// 				ContributionParams::<TestRuntime>::new(BUYER_6, 1000 * CT_UNIT, 1, AcceptedFundingAsset::USDT);
// 			let contribution_mul_2 =
// 				ContributionParams::<TestRuntime>::new(BUYER_7, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);
//
// 			community_contributions.push(contribution_mul_1);
//
// 			let project_id = inst.create_remainder_contributing_project(
// 				project_metadata.clone(),
// 				issuer,
// 				None,
// 				evaluations,
// 				bids,
// 				community_contributions,
// 			);
// 			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
//
// 			let plmc_required = inst.calculate_contributed_plmc_spent(vec![contribution_mul_2.clone()], wap, false);
// 			let plmc_ed = plmc_required.accounts().existential_deposits();
// 			inst.mint_plmc_to(plmc_required.clone());
// 			inst.mint_plmc_to(plmc_ed);
//
// 			let usdt_required = inst.calculate_contributed_funding_asset_spent(vec![contribution_mul_2.clone()], wap);
// 			inst.mint_foreign_asset_to(usdt_required.clone());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::contribute(
// 					RuntimeOrigin::signed(BUYER_7),
// 					get_mock_jwt_with_cid(
// 						BUYER_7,
// 						InvestorType::Professional,
// 						generate_did_from_account(BUYER_7),
// 						project_metadata.clone().policy_ipfs_cid.unwrap(),
// 					),
// 					project_id,
// 					contribution_mul_2.amount,
// 					contribution_mul_2.multiplier,
// 					contribution_mul_2.asset
// 				));
// 			});
//
// 			inst.finish_funding(project_id, None).unwrap();
// 			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
// 			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
// 			inst.jump_to_block(settlement_block);
//
// 			// First contribution assertions
// 			let stored_contribution =
// 				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 5)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_6,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance, 0u128);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_failed_contribution(
// 					RuntimeOrigin::signed(BUYER_6),
// 					project_id,
// 					BUYER_6,
// 					5
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 6)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
// 			let issuer_usdt_balance =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_6,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			assert_eq!(ct_amount, Zero::zero());
// 			assert_eq!(issuer_usdt_balance, Zero::zero());
// 			assert!(unvested_amount.is_none());
// 			inst.assert_migration(
// 				project_id,
// 				BUYER_6,
// 				stored_contribution.ct_amount,
// 				5,
// 				ParticipationType::Contribution,
// 				false,
// 			);
//
// 			// Second contribution assertions
// 			let stored_contribution =
// 				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 6)).unwrap());
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_7,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
// 			assert_eq!(plmc_free_amount, inst.get_ed());
// 			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
// 			assert_eq!(ct_amount, 0u128);
// 			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance);
// 			assert!(unvested_amount.is_none());
//
// 			inst.execute(|| {
// 				assert_ok!(PolimecFunding::settle_failed_contribution(
// 					RuntimeOrigin::signed(BUYER_7),
// 					project_id,
// 					BUYER_7,
// 					6
// 				));
// 			});
//
// 			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 7)).is_none()));
// 			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
// 			let plmc_held_amount =
// 				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
// 			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
// 			let issuer_usdt_balance_2 =
// 				inst.get_free_foreign_asset_balance_for(stored_contribution.funding_asset.to_assethub_id(), issuer);
// 			let unvested_amount = inst.execute(|| {
// 				<TestRuntime as Config>::Vesting::total_scheduled_amount(
// 					&BUYER_7,
// 					HoldReason::Participation(project_id).into(),
// 				)
// 			});
//
// 			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
// 			assert_eq!(plmc_held_amount, 0u128);
// 			assert_eq!(ct_amount, Zero::zero());
// 			assert_eq!(issuer_usdt_balance_2, Zero::zero());
// 			assert!(unvested_amount.is_none());
//
// 			inst.assert_migration(
// 				project_id,
// 				BUYER_7,
// 				stored_contribution.ct_amount,
// 				6,
// 				ParticipationType::Contribution,
// 				false,
// 			);
// 		}
// 	}
//
// 	#[cfg(test)]
// 	mod failure {
// 		use super::*;
//
// 		#[test]
// 		fn cannot_settle_twice() {
// 			let percentage = 33u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			inst.execute(|| {
// 				let contributor = first_contribution.contributor;
// 				assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_contribution(
// 					RuntimeOrigin::signed(contributor),
// 					project_id,
// 					contributor,
// 					first_contribution.id
// 				));
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::ParticipationNotFound
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_on_wrong_outcome() {
// 			let percentage = 100u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, true);
//
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			let contributor = first_contribution.contributor;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
//
// 		#[test]
// 		fn cannot_be_called_before_settlement_started() {
// 			let percentage = 10u64;
// 			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None, false);
//
// 			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
// 			let contributor = first_contribution.contributor;
// 			inst.execute(|| {
// 				assert_noop!(
// 					crate::Pallet::<TestRuntime>::settle_failed_contribution(
// 						RuntimeOrigin::signed(contributor),
// 						project_id,
// 						contributor,
// 						first_contribution.id
// 					),
// 					Error::<TestRuntime>::FundingFailedSettlementNotStarted
// 				);
// 			});
// 		}
// 	}
// }
