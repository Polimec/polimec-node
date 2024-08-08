use super::*;
use frame_support::traits::fungibles::Inspect;
use sp_runtime::bounded_vec;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn can_fully_settle_accepted_project() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);
			let evaluations = inst.get_evaluations(project_id);
			let bids = inst.get_bids(project_id);
			let contributions = inst.get_contributions(project_id);

			inst.settle_project(project_id, true);

			inst.assert_total_funding_paid_out(project_id, bids.clone(), contributions.clone());
			inst.assert_evaluations_migrations_created(project_id, evaluations, true);
			inst.assert_bids_migrations_created(project_id, bids, true);
			inst.assert_contributions_migrations_created(project_id, contributions, true);
		}

		#[test]
		fn can_fully_settle_failed_project() {
			let percentage = 32u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);
			let evaluations = inst.get_evaluations(project_id);
			let bids = inst.get_bids(project_id);
			let contributions = inst.get_contributions(project_id);

			inst.settle_project(project_id, true);

			inst.assert_evaluations_migrations_created(project_id, evaluations, false);
			inst.assert_bids_migrations_created(project_id, bids, false);
			inst.assert_contributions_migrations_created(project_id, contributions, false);
		}
	}
}

#[cfg(test)]
mod start_settlement_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn funding_success_settlement() {
			let (mut inst, project_id) = create_project_with_funding_percentage(40, false);
			let ct_treasury = <TestRuntime as Config>::ContributionTreasury::get();
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_amount_reached_usd, 4_000_000 * USD_UNIT);
			let usd_fee = Percent::from_percent(10u8) * (1_000_000 * USD_UNIT) +
				Percent::from_percent(8u8) * (3_000_000 * USD_UNIT);
			let ct_fee =
				project_details.weighted_average_price.unwrap().reciprocal().unwrap().saturating_mul_int(usd_fee);
			// Liquidity Pools and Long Term Holder Bonus treasury allocation
			let treasury_allocation = Percent::from_percent(50) * ct_fee + Percent::from_percent(20) * ct_fee;

			assert_eq!(project_details.funding_end_block, None);
			assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);
			inst.execute(|| {
				assert_eq!(<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id), false)
			});

			inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get());
			inst.execute(|| {
				assert_ok!(PolimecFunding::start_settlement(RuntimeOrigin::signed(80085), project_id));
			});
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, Some(inst.current_block()));
			assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Success));
			inst.execute(|| {
				assert_eq!(<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id), true)
			});

			inst.assert_ct_balance(project_id, ct_treasury, treasury_allocation);
		}

		#[test]
		fn funding_failed_settlement() {
			let (mut inst, project_id) = create_project_with_funding_percentage(32, false);
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, None);
			assert_eq!(project_details.status, ProjectStatus::FundingFailed);
			inst.execute(|| {
				assert_eq!(<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id), false)
			});

			inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get());
			inst.execute(|| {
				assert_ok!(PolimecFunding::start_settlement(RuntimeOrigin::signed(80085), project_id));
			});
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, Some(inst.current_block()));
			assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Failure));
			inst.execute(|| {
				assert_eq!(<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id), false)
			});
		}

		#[test]
		fn vesting_schedules_are_merged() {
			let mut inst =
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn called_too_early() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_remainder_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				None,
				default_evaluations(),
				vec![],
				vec![],
			);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::end_funding(RuntimeOrigin::signed(42), project_id),
					Error::<TestRuntime>::TooEarlyForRound
				);
			});
		}

		#[test]
		fn called_twice() {
			let (mut inst, project_id) = create_project_with_funding_percentage(95, true);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_settlement(RuntimeOrigin::signed(42), project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}

#[cfg(test)]
mod settle_evaluation_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn evaluation_rewarded() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				vec![
					UserToUSDBalance::new(EVALUATOR_1, 500_000 * USD_UNIT),
					UserToUSDBalance::new(EVALUATOR_2, 250_000 * USD_UNIT),
					UserToUSDBalance::new(EVALUATOR_3, 320_000 * USD_UNIT),
				],
				inst.generate_bids_from_total_ct_percent(
					project_metadata.clone(),
					50,
					default_weights(),
					default_bidders(),
					default_multipliers(),
				),
				inst.generate_contributions_from_total_ct_percent(
					project_metadata.clone(),
					50,
					default_weights(),
					default_community_contributors(),
					default_community_contributor_multipliers(),
				),
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			// The rewards are calculated as follows:
			// Data:
			// - Funding USD reached: 10_000_000 USD
			// - Total CTs sold: 1_000_000 CT
			// - USD target reached percent: 100%

			// Step 1) Calculate the total USD fee:
			// USD fee 1 = 0.1 * 1_000_000 = 100_000 USD
			// USD fee 2 = 0.08 * 4_000_000 = 320_000 USD
			// USD fee 3 = 0.06 * 5_000_000 = 300_000 USD
			// Total USD fee = 100_000 + 320_000 + 300_000 = 720_000 USD

			// Step 2) Calculate CT fee as follows:
			// Percent fee = Total USD fee / Funding USD reached = 720_000 / 10_000_000 = 0.072
			// CT fee = Percent fee * Total CTs sold = 0.072 * 1_000_000 = 72_000 CT

			// Step 3) Calculate Early and Normal evaluator reward pots:
			// Total evaluators reward pot = CT fee * 0.3 * USD target reached percent = 72_000 * 0.3 * 1 = 21_600 CT
			// Early evaluators reward pot = Total evaluators reward pot * 0.2 = 21_600 * 0.2 = 4_320 CT
			// Normal evaluators reward pot = Total evaluators reward pot * 0.8 = 21_600 * 0.8 = 17_280 CT

			// Step 4) Calculate the early and normal weights of each evaluation:
			// Evaluation 1 = 500_000 USD
			// Evaluation 2 = 250_000 USD
			// Evaluation 3 = 320_000 USD

			// Early amount 1 = 500_000 USD
			// Early amount 2 = 250_000 USD
			// Early amount 3 = 250_000 USD

			// Total Normal amount = Evaluation 1 + Evaluation 2 + Evaluation 3 = 500_000 + 250_000 + 320_000 = 1_070_000 USD
			// Total Early amount = 10% of USD target = 1_000_000 USD

			// Early weight 1 = Early amount 1 / Total Early amount = 500_000 / 1_000_000 = 0.5
			// Early weight 2 = Early amount 2 / Total Early amount = 250_000 / 1_000_000 = 0.25
			// Early weight 3 = Early amount 3 / Total Early amount = 250_000 / 1_000_000 = 0.25

			// Normal weight 1 = Evaluation 1 / Total Normal amount = 500_000 / 1_070_000 = 0.467289719626168
			// Normal weight 2 = Evaluation 2 / Total Normal amount = 250_000 / 1_070_000 = 0.233644859813084
			// Normal weight 3 = Evaluation 3 / Total Normal amount = 320_000 / 1_070_000 = 0.299065420560748

			// Step 5) Calculate the rewards for each evaluation:
			// Evaluation 1 Early reward = Early weight 1 * Early evaluators reward pot = 0.5 * 4_320 = 2_160 CT
			// Evaluation 2 Early reward = Early weight 2 * Early evaluators reward pot = 0.25 * 4_320 = 1_080 CT
			// Evaluation 3 Early reward = Early weight 3 * Early evaluators reward pot = 0.25 * 4_320 = 1_080 CT

			// Evaluation 1 Normal reward = Normal weight 1 * Normal evaluators reward pot = 0.467289719626168 * 17_280 = 8'074.766355140186916 CT
			// Evaluation 2 Normal reward = Normal weight 2 * Normal evaluators reward pot = 0.233644859813084 * 17_280 = 4'037.383177570093458 CT
			// Evaluation 3 Normal reward = Normal weight 3 * Normal evaluators reward pot = 0.299065420560748 * 17_280 = 5'167.850467289719626 CT

			// Evaluation 1 Total reward = Evaluation 1 Early reward + Evaluation 1 Normal reward = 2_160 + 8_066 = 10'234.766355140186916 CT
			// Evaluation 2 Total reward = Evaluation 2 Early reward + Evaluation 2 Normal reward = 1_080 + 4_033 = 5'117.383177570093458 CT
			// Evaluation 3 Total reward = Evaluation 3 Early reward + Evaluation 3 Normal reward = 1_080 + 5_201 = 6'247.850467289719626 CT

			const EVAL_1_REWARD: u128 = 10_234_766355140186916;
			const EVAL_2_REWARD: u128 = 5_117_383177570093458;
			const EVAL_3_REWARD: u128 = 6_247_850467289719626;

			let prev_ct_balances = inst.get_ct_asset_balances_for(project_id, vec![ISSUER_1, ISSUER_2, ISSUER_3]);
			assert!(prev_ct_balances.iter().all(|x| *x == Zero::zero()));

			let evals = vec![(EVALUATOR_1, EVAL_1_REWARD), (EVALUATOR_2, EVAL_2_REWARD), (EVALUATOR_3, EVAL_3_REWARD)];

			for (evaluator, expected_reward) in evals {
				let evaluation_locked_plmc =
					inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation(project_id).into());
				let free_plmc = inst.get_free_plmc_balance_for(evaluator);
				assert_ok!(inst.execute(|| PolimecFunding::settle_evaluation(
					RuntimeOrigin::signed(evaluator),
					project_id,
					evaluator,
					evaluator - 21 // The First evaluation index is 0, the first evaluator account is 21
				)));
				let ct_rewarded = inst.get_ct_asset_balance_for(project_id, evaluator);
				assert_close_enough!(ct_rewarded, expected_reward, Perquintill::from_float(0.9999));
				assert_eq!(inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation(project_id).into()), 0);
				assert_eq!(inst.get_free_plmc_balance_for(evaluator), free_plmc + evaluation_locked_plmc);
				inst.assert_migration(project_id, evaluator, expected_reward, 0, ParticipationType::Evaluation, true);
			}
		}

		#[test]
		fn evaluation_slashed() {
			let percentage = 20u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);

			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
			let evaluator = first_evaluation.evaluator;
			let prev_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;

			assert_eq!(
				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
				Some(EvaluatorsOutcomeOf::<TestRuntime>::Slashed)
			);

			assert_ok!(inst.execute(|| PolimecFunding::settle_evaluation(
				RuntimeOrigin::signed(evaluator),
				project_id,
				evaluator,
				first_evaluation.id
			)));

			let post_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;
			assert_eq!(
				post_balance,
				prev_balance +
					(Percent::from_percent(100) - <TestRuntime as Config>::EvaluatorSlash::get()) *
						first_evaluation.current_plmc_bond
			);
		}

		#[test]
		fn evaluation_round_failed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 1_000 * USD_UNIT);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

			let evaluation_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], true);
			inst.mint_plmc_to(evaluation_plmc.clone());
			inst.evaluate_for_users(project_id, vec![evaluation]).unwrap();

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			let evaluation_locked_plmc =
				inst.get_reserved_plmc_balance_for(EVALUATOR_1, HoldReason::Evaluation(project_id).into());
			let free_plmc = inst.get_free_plmc_balance_for(EVALUATOR_1);

			assert_ok!(inst.execute(|| PolimecFunding::settle_evaluation(
				RuntimeOrigin::signed(EVALUATOR_1),
				project_id,
				EVALUATOR_1,
				0
			)));

			assert_eq!(inst.get_ct_asset_balance_for(project_id, EVALUATOR_1), 0);
			assert_eq!(inst.get_reserved_plmc_balance_for(EVALUATOR_1, HoldReason::Evaluation(project_id).into()), 0);
			assert_eq!(inst.get_free_plmc_balance_for(EVALUATOR_1), free_plmc + evaluation_locked_plmc);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_settle_twice() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);

			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
			inst.execute(|| {
				let evaluator = first_evaluation.evaluator;
				assert_ok!(crate::Pallet::<TestRuntime>::settle_evaluation(
					RuntimeOrigin::signed(evaluator),
					project_id,
					evaluator,
					first_evaluation.id
				));
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					),
					Error::<TestRuntime>::ParticipationNotFound
				);
			});
		}

		#[test]
		fn cannot_be_called_before_settlement_started() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, false);

			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
			let evaluator = first_evaluation.evaluator;

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::settle_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					),
					Error::<TestRuntime>::SettlementNotStarted
				);
			});
		}
	}
}

#[cfg(test)]
mod settle_bid_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn accepted_bid_with_refund_on_project_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let partial_amount_bid_params =
				BidParams::new(BIDDER_1, auction_allocation, 1u8, AcceptedFundingAsset::USDT);
			let lower_price_bid_params = BidParams::new(BIDDER_2, 2000 * CT_UNIT, 5u8, AcceptedFundingAsset::DOT);
			let bids = vec![partial_amount_bid_params.clone(), lower_price_bid_params.clone()];

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
				default_community_contributions(),
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			// Partial amount bid assertions
			let partial_amount_bid_stored =
				inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
			let mut final_partial_amount_bid_params = partial_amount_bid_params.clone();
			final_partial_amount_bid_params.amount = auction_allocation - 2000 * CT_UNIT;
			let expected_final_plmc_bonded = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![final_partial_amount_bid_params.clone()],
				project_metadata.minimum_price,
				false,
			)[0]
			.plmc_amount;
			let expected_final_usdt_paid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![final_partial_amount_bid_params],
				project_metadata.minimum_price,
			)[0]
			.asset_amount;

			let expected_plmc_refund = partial_amount_bid_stored.plmc_bond - expected_final_plmc_bonded;
			let expected_usdt_refund = partial_amount_bid_stored.funding_asset_amount_locked - expected_final_usdt_paid;

			let pre_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(BIDDER_1, AcceptedFundingAsset::USDT.id(), expected_usdt_refund);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance + expected_final_usdt_paid);

			inst.assert_plmc_free_balance(BIDDER_1, expected_plmc_refund + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, auction_allocation - 2000 * CT_UNIT);

			inst.assert_migration(
				project_id,
				BIDDER_1,
				auction_allocation - 2000 * CT_UNIT,
				0,
				ParticipationType::Bid,
				true,
			);

			// Multiplier one should be fully unbonded the next block
			inst.advance_time(1_u64);

			let hold_reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_1), hold_reason).expect("Vesting failed"));

			inst.assert_plmc_free_balance(BIDDER_1, expected_plmc_refund + expected_final_plmc_bonded + ed);

			// Price > wap bid assertions
			let lower_price_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_2, 1)).unwrap());
			let expected_final_plmc_bonded =
				inst.calculate_auction_plmc_charged_with_given_price(&vec![lower_price_bid_params.clone()], wap, false)
					[0]
				.plmc_amount;
			let expected_final_dot_paid = inst
				.calculate_auction_funding_asset_charged_with_given_price(&vec![lower_price_bid_params.clone()], wap)[0]
				.asset_amount;
			let expected_plmc_refund = lower_price_bid_stored.plmc_bond - expected_final_plmc_bonded;
			let expected_dot_refund = lower_price_bid_stored.funding_asset_amount_locked - expected_final_dot_paid;

			let pre_issuer_dot_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::DOT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BUYER_1), project_id, BIDDER_2, 1));
			});

			let post_issuer_dot_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::DOT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(BIDDER_2, AcceptedFundingAsset::DOT.id(), expected_dot_refund);
			assert_eq!(post_issuer_dot_balance, pre_issuer_dot_balance + expected_final_dot_paid);

			inst.assert_plmc_free_balance(BIDDER_2, expected_plmc_refund + ed);
			inst.assert_ct_balance(project_id, BIDDER_2, 2000 * CT_UNIT);

			inst.assert_migration(project_id, BIDDER_2, 2000 * CT_UNIT, 1, ParticipationType::Bid, true);

			// Multiplier 5 should be unbonded no earlier than after 8.67 weeks (i.e. 436'867 blocks)
			let vesting_time = lower_price_bid_params.multiplier.calculate_vesting_duration::<TestRuntime>();

			// Sanity check, 5 blocks should not be enough
			inst.advance_time(5u64);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason).expect("Vesting failed"));
			assert_ne!(
				inst.get_free_plmc_balance_for(BIDDER_2),
				expected_plmc_refund + expected_final_plmc_bonded + ed
			);

			// After the vesting time, the full amount should be vested
			let current_block = inst.current_block();
			inst.jump_to_block(current_block + vesting_time - 5u64);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason).expect("Vesting failed"));
			inst.assert_plmc_free_balance(BIDDER_2, expected_plmc_refund + expected_final_plmc_bonded + ed);
		}

		#[test]
		fn accepted_bid_without_refund_on_project_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let no_refund_bid_params =
				BidParams::new(BIDDER_1, auction_allocation / 2, 16u8, AcceptedFundingAsset::USDT);

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				vec![no_refund_bid_params.clone()],
				default_community_contributions(),
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let no_refund_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());

			let pre_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(BIDDER_1, AcceptedFundingAsset::USDT.id(), Zero::zero());
			assert_eq!(
				post_issuer_usdc_balance,
				pre_issuer_usdc_balance + no_refund_bid_stored.funding_asset_amount_locked
			);

			inst.assert_plmc_free_balance(BIDDER_1, ed);
			inst.assert_ct_balance(project_id, BIDDER_1, auction_allocation / 2);

			inst.assert_migration(project_id, BIDDER_1, auction_allocation / 2, 0, ParticipationType::Bid, true);

			let hold_reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();

			let vesting_time = no_refund_bid_params.multiplier.calculate_vesting_duration::<TestRuntime>();

			// Sanity check, 5 blocks should not be enough
			inst.advance_time(5u64);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_1), hold_reason).expect("Vesting failed"));
			assert_ne!(inst.get_free_plmc_balance_for(BIDDER_1), no_refund_bid_stored.plmc_bond + ed);

			// After the vesting time, the full amount should be vested
			let current_block = inst.current_block();
			inst.jump_to_block(current_block + vesting_time - 5u64 + 1u64);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_1), hold_reason).expect("Vesting failed"));
			inst.assert_plmc_free_balance(BIDDER_1, no_refund_bid_stored.plmc_bond + ed);
		}

		#[test]
		fn accepted_bid_with_refund_on_project_failure() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT];
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(10);
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let partial_amount_bid_params =
				BidParams::new(BIDDER_1, auction_allocation, 1u8, AcceptedFundingAsset::USDC);
			let lower_price_bid_params = BidParams::new(BIDDER_2, 2000 * CT_UNIT, 5u8, AcceptedFundingAsset::DOT);
			let bids = vec![partial_amount_bid_params.clone(), lower_price_bid_params.clone()];

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
				vec![],
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));
			let hold_reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();

			// Partial amount bid assertions
			let partial_amount_bid_stored =
				inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());

			let pre_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDC.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDC.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDC.id(),
				partial_amount_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_usdc_balance, pre_issuer_usdc_balance);

			inst.assert_plmc_free_balance(BIDDER_1, partial_amount_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			inst.assert_migration(project_id, BIDDER_1, Zero::zero(), 0, ParticipationType::Bid, false);

			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_1), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});

			// Price > wap bid assertions
			let lower_price_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_2, 1)).unwrap());

			let pre_issuer_dot_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::DOT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BUYER_1), project_id, BIDDER_2, 1));
			});

			let post_issuer_dot_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::DOT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_2,
				AcceptedFundingAsset::DOT.id(),
				lower_price_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_dot_balance, pre_issuer_dot_balance);

			inst.assert_plmc_free_balance(BIDDER_2, lower_price_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_2, Zero::zero());

			inst.assert_migration(project_id, BIDDER_2, Zero::zero(), 1, ParticipationType::Bid, false);

			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});
		}

		#[test]
		fn accepted_bid_without_refund_on_project_failure() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let no_refund_bid_params = BidParams::new(BIDDER_1, 500 * CT_UNIT, 16u8, AcceptedFundingAsset::USDT);

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				vec![no_refund_bid_params.clone()],
				vec![],
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			// Partial amount bid assertions
			let no_refund_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());

			let pre_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				no_refund_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_usdc_balance, pre_issuer_usdc_balance);

			inst.assert_plmc_free_balance(BIDDER_1, ed + no_refund_bid_stored.plmc_bond);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});
		}

		#[test]
		fn rejected_bid_on_community_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let rejected_bid_params = BidParams::new(BIDDER_1, auction_allocation, 4u8, AcceptedFundingAsset::USDT);
			let accepted_bid_params = BidParams::new(BIDDER_2, auction_allocation, 1u8, AcceptedFundingAsset::DOT);

			let bids = vec![rejected_bid_params.clone(), accepted_bid_params.clone()];
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
			);

			let rejected_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
			assert_eq!(rejected_bid_stored.status, BidStatus::Rejected);

			let pre_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				rejected_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance);

			inst.assert_plmc_free_balance(BIDDER_1, rejected_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason = HoldReason::Participation(project_id).into();
			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});
		}

		#[test]
		fn rejected_bid_on_project_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let rejected_bid_params = BidParams::new(BIDDER_1, auction_allocation, 4u8, AcceptedFundingAsset::USDT);
			let accepted_bid_params = BidParams::new(BIDDER_2, auction_allocation, 1u8, AcceptedFundingAsset::DOT);

			let bids = vec![rejected_bid_params.clone(), accepted_bid_params.clone()];
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
				default_community_contributions(),
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let rejected_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
			assert_eq!(rejected_bid_stored.status, BidStatus::Rejected);

			let pre_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				rejected_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance);

			inst.assert_plmc_free_balance(BIDDER_1, rejected_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason = HoldReason::Participation(project_id).into();
			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});
		}

		#[test]
		fn rejected_bid_on_project_failure() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(10);
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let rejected_bid_params = BidParams::new(BIDDER_1, auction_allocation, 4u8, AcceptedFundingAsset::USDT);
			let accepted_bid_params = BidParams::new(BIDDER_2, auction_allocation, 1u8, AcceptedFundingAsset::DOT);

			let bids = vec![rejected_bid_params.clone(), accepted_bid_params.clone()];
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
				vec![],
				vec![],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			let rejected_bid_stored = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 0)).unwrap());
			assert_eq!(rejected_bid_stored.status, BidStatus::Rejected);

			let pre_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0));
			});

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				rejected_bid_stored.funding_asset_amount_locked,
			);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance);

			inst.assert_plmc_free_balance(BIDDER_1, rejected_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason = HoldReason::Participation(project_id).into();
			inst.execute(|| {
				assert_noop!(
					LinearRelease::vest(RuntimeOrigin::signed(BIDDER_2), hold_reason),
					pallet_linear_release::Error::<TestRuntime>::NotVesting
				);
			});
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_settle_twice() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);

			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
			inst.execute(|| {
				let bidder = first_bid.bidder;
				assert_ok!(crate::Pallet::<TestRuntime>::settle_bid(
					RuntimeOrigin::signed(bidder),
					project_id,
					bidder,
					first_bid.id
				));
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_bid(
						RuntimeOrigin::signed(bidder),
						project_id,
						bidder,
						first_bid.id
					),
					Error::<TestRuntime>::ParticipationNotFound
				);
			});
		}

		#[test]
		fn cannot_be_called_before_settlement_started() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, false);

			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
			let bidder = first_bid.bidder;
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_bid(
						RuntimeOrigin::signed(bidder),
						project_id,
						bidder,
						first_bid.id
					),
					Error::<TestRuntime>::SettlementNotStarted
				);
			});
		}
	}
}

#[cfg(test)]
mod settle_contribution_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn contribution_on_successful_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let project_metadata = default_project_metadata(ISSUER_1);

			let contribution =
				ContributionParams::<TestRuntime>::new(BUYER_1, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				default_bids(),
				vec![contribution.clone()],
				vec![],
			);

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			// First contribution assertions
			let stored_contribution =
				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_1, 0)).unwrap());
			let hold_reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();

			inst.assert_plmc_free_balance(BUYER_1, ed);
			inst.assert_plmc_held_balance(BUYER_1, stored_contribution.plmc_bond, hold_reason);
			inst.assert_ct_balance(project_id, BUYER_1, Zero::zero());
			inst.assert_funding_asset_free_balance(BUYER_1, AcceptedFundingAsset::USDT.id(), Zero::zero());
			inst.assert_funding_asset_free_balance(
				project_metadata.funding_destination_account,
				AcceptedFundingAsset::USDT.id(),
				Zero::zero(),
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_contribution(RuntimeOrigin::signed(BUYER_1), project_id, BUYER_1, 0));
			});

			inst.assert_plmc_free_balance(BUYER_1, ed);
			inst.assert_plmc_held_balance(BUYER_1, stored_contribution.plmc_bond, hold_reason);
			inst.assert_ct_balance(project_id, BUYER_1, stored_contribution.ct_amount);
			inst.assert_funding_asset_free_balance(BUYER_1, AcceptedFundingAsset::USDT.id(), Zero::zero());
			inst.assert_funding_asset_free_balance(
				project_metadata.funding_destination_account,
				AcceptedFundingAsset::USDT.id(),
				stored_contribution.funding_asset_amount,
			);
			inst.assert_migration(
				project_id,
				BUYER_1,
				stored_contribution.ct_amount,
				0,
				ParticipationType::Contribution,
				true,
			);

			let vesting_time = contribution.multiplier.calculate_vesting_duration::<TestRuntime>();
			let current_block = inst.current_block();
			inst.jump_to_block(current_block + vesting_time + 1);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BUYER_1), hold_reason).expect("Vesting failed"));
			inst.assert_plmc_free_balance(BUYER_1, ed + stored_contribution.plmc_bond);
			inst.assert_plmc_held_balance(BUYER_1, Zero::zero(), hold_reason);
		}

		#[test]
		fn contribution_on_failed_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let issuer = ISSUER_1;
			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
			let bids = inst.generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				10,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let mut community_contributions = inst.generate_contributions_from_total_ct_percent(
				project_metadata.clone(),
				10,
				default_weights(),
				default_community_contributors(),
				default_community_contributor_multipliers(),
			);

			let contribution_mul_1 =
				ContributionParams::<TestRuntime>::new(BUYER_6, 1000 * CT_UNIT, 1, AcceptedFundingAsset::USDT);
			let contribution_mul_2 =
				ContributionParams::<TestRuntime>::new(BUYER_7, 1000 * CT_UNIT, 2, AcceptedFundingAsset::USDT);

			community_contributions.push(contribution_mul_1);

			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				issuer,
				None,
				evaluations,
				bids,
				community_contributions,
			);
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let plmc_required = inst.calculate_contributed_plmc_spent(vec![contribution_mul_2.clone()], wap, false);
			let plmc_ed = plmc_required.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_required.clone());
			inst.mint_plmc_to(plmc_ed);

			let usdt_required = inst.calculate_contributed_funding_asset_spent(vec![contribution_mul_2.clone()], wap);
			inst.mint_funding_asset_to(usdt_required.clone());

			inst.execute(|| {
				assert_ok!(PolimecFunding::contribute(
					RuntimeOrigin::signed(BUYER_7),
					get_mock_jwt_with_cid(
						BUYER_7,
						InvestorType::Professional,
						generate_did_from_account(BUYER_7),
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					),
					project_id,
					contribution_mul_2.amount,
					contribution_mul_2.multiplier,
					contribution_mul_2.asset
				));
			});

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			// First contribution assertions
			let stored_contribution =
				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 5)).unwrap());
			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
			let plmc_held_amount =
				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
			let issuer_usdt_balance =
				inst.get_free_funding_asset_balance_for(stored_contribution.funding_asset.id(), issuer);
			let unvested_amount = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&BUYER_6,
					HoldReason::Participation(project_id).into(),
				)
			});

			assert_eq!(plmc_free_amount, inst.get_ed());
			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
			assert_eq!(ct_amount, 0u128);
			assert_eq!(issuer_usdt_balance, 0u128);
			assert!(unvested_amount.is_none());

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_contribution(RuntimeOrigin::signed(BUYER_6), project_id, BUYER_6, 5));
			});

			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_6, 6)).is_none()));
			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_6);
			let plmc_held_amount =
				inst.get_reserved_plmc_balance_for(BUYER_6, HoldReason::Participation(project_id).into());
			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_6);
			let issuer_usdt_balance =
				inst.get_free_funding_asset_balance_for(stored_contribution.funding_asset.id(), issuer);
			let unvested_amount = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&BUYER_6,
					HoldReason::Participation(project_id).into(),
				)
			});

			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
			assert_eq!(plmc_held_amount, 0u128);
			assert_eq!(ct_amount, Zero::zero());
			assert_eq!(issuer_usdt_balance, Zero::zero());
			assert!(unvested_amount.is_none());
			inst.assert_migration(
				project_id,
				BUYER_6,
				stored_contribution.ct_amount,
				5,
				ParticipationType::Contribution,
				false,
			);

			// Second contribution assertions
			let stored_contribution =
				inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 6)).unwrap());
			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
			let plmc_held_amount =
				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
			let issuer_usdt_balance_2 =
				inst.get_free_funding_asset_balance_for(stored_contribution.funding_asset.id(), issuer);
			let unvested_amount = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&BUYER_7,
					HoldReason::Participation(project_id).into(),
				)
			});
			assert_eq!(plmc_free_amount, inst.get_ed());
			assert_eq!(plmc_held_amount, stored_contribution.plmc_bond);
			assert_eq!(ct_amount, 0u128);
			assert_eq!(issuer_usdt_balance_2, issuer_usdt_balance);
			assert!(unvested_amount.is_none());

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_contribution(RuntimeOrigin::signed(BUYER_7), project_id, BUYER_7, 6));
			});

			assert!(inst.execute(|| Contributions::<TestRuntime>::get((project_id, BUYER_7, 7)).is_none()));
			let plmc_free_amount = inst.get_free_plmc_balance_for(BUYER_7);
			let plmc_held_amount =
				inst.get_reserved_plmc_balance_for(BUYER_7, HoldReason::Participation(project_id).into());
			let ct_amount = inst.get_ct_asset_balance_for(project_id, BUYER_7);
			let issuer_usdt_balance_2 =
				inst.get_free_funding_asset_balance_for(stored_contribution.funding_asset.id(), issuer);
			let unvested_amount = inst.execute(|| {
				<TestRuntime as Config>::Vesting::total_scheduled_amount(
					&BUYER_7,
					HoldReason::Participation(project_id).into(),
				)
			});

			assert_eq!(plmc_free_amount, inst.get_ed() + stored_contribution.plmc_bond);
			assert_eq!(plmc_held_amount, 0u128);
			assert_eq!(ct_amount, Zero::zero());
			assert_eq!(issuer_usdt_balance_2, Zero::zero());
			assert!(unvested_amount.is_none());

			inst.assert_migration(
				project_id,
				BUYER_7,
				stored_contribution.ct_amount,
				6,
				ParticipationType::Contribution,
				false,
			);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_settle_twice() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);

			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
			inst.execute(|| {
				let contributor = first_contribution.contributor;
				assert_ok!(crate::Pallet::<TestRuntime>::settle_contribution(
					RuntimeOrigin::signed(contributor),
					project_id,
					contributor,
					first_contribution.id
				));
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_contribution(
						RuntimeOrigin::signed(contributor),
						project_id,
						contributor,
						first_contribution.id
					),
					Error::<TestRuntime>::ParticipationNotFound
				);
			});
		}

		#[test]
		fn cannot_be_called_before_settlement_started() {
			let percentage = 100u64;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, false);
			let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
			let contributor = first_contribution.contributor;
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_contribution(
						RuntimeOrigin::signed(contributor),
						project_id,
						contributor,
						first_contribution.id
					),
					Error::<TestRuntime>::SettlementNotStarted
				);
			});
		}
	}
}

#[cfg(test)]
mod mark_project_as_settled_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn funding_failed_marked_as_settled() {
			let (mut inst, project_id) = create_project_with_funding_percentage(10, true);
			inst.settle_project(project_id, false);

			inst.execute(|| {
				assert_ok!(PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id));
			});

			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementFinished(FundingOutcome::Failure)
			);
		}

		#[test]
		fn funding_successful_marked_as_settled() {
			let (mut inst, project_id) = create_project_with_funding_percentage(34, true);
			inst.settle_project(project_id, false);

			inst.execute(|| {
				assert_ok!(PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id));
			});

			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementFinished(FundingOutcome::Success)
			);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn funding_failed_marked_twice() {
			let (mut inst, project_id) = create_project_with_funding_percentage(10, true);
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn funding_failed_too_early() {
			let (mut inst, project_id) = create_project_with_funding_percentage(10, false);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn funding_successful_marked_twice() {
			let (mut inst, project_id) = create_project_with_funding_percentage(35, true);
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn funding_successful_too_early() {
			let (mut inst, project_id) = create_project_with_funding_percentage(35, false);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::mark_project_as_settled(RuntimeOrigin::signed(80085), project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}
