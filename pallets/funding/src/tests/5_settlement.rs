use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn can_fully_settle_accepted_project() {
			let percentage = 100u8;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);
			let evaluations = inst.get_evaluations(project_id);

			let bids = inst.get_bids(project_id);
			inst.settle_project(project_id, true);

			inst.assert_total_funding_paid_out(project_id, bids.clone());
			inst.assert_evaluations_migrations_created(project_id, evaluations, true);
			inst.assert_bids_migrations_created(project_id, bids, true);
		}

		#[test]
		fn can_fully_settle_failed_project() {
			let percentage = 32u8;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);
			let evaluations = inst.get_evaluations(project_id);
			let bids = inst.get_bids(project_id);

			inst.settle_project(project_id, true);

			inst.assert_evaluations_migrations_created(project_id, evaluations, false);
			inst.assert_bids_migrations_created(project_id, bids, false);
		}

		#[test]
		fn ethereum_project_can_be_settled() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			let base_price = PriceOf::<TestRuntime>::from_float(1.0);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;

			project_metadata.participants_account_type = ParticipantsAccountType::Ethereum;

			let evaluations = vec![
				EvaluationParams::from((
					EVALUATOR_1,
					500_000 * USD_UNIT,
					Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [0u8; 20] },
				)),
				EvaluationParams::from((
					EVALUATOR_2,
					250_000 * USD_UNIT,
					Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [1u8; 20] },
				)),
				EvaluationParams::from((
					EVALUATOR_3,
					300_000 * USD_UNIT,
					Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [2u8; 20] },
				)),
			];
			let bids = vec![
				BidParams::from((
					BIDDER_1,
					Retail,
					120_000 * CT_UNIT,
					ParticipationMode::Classic(3u8),
					AcceptedFundingAsset::USDT,
					Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [3u8; 20] },
				)),
				BidParams::from((
					BIDDER_2,
					Retail,
					420_000 * CT_UNIT,
					ParticipationMode::Classic(5u8),
					AcceptedFundingAsset::USDT,
					Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [4u8; 20] },
				)),
			];

			let project_id =
				inst.create_settled_project(project_metadata.clone(), ISSUER_1, None, evaluations, bids, false);

			let evaluations = inst.get_evaluations(project_id);
			let bids = inst.get_bids(project_id);

			inst.settle_project(project_id, true);

			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementFinished(FundingOutcome::Success)
			);

			inst.assert_evaluations_migrations_created(project_id, evaluations, true);
			inst.assert_bids_migrations_created(project_id, bids, true);

			let _user_migrations =
				inst.execute(|| UserMigrations::<TestRuntime>::iter_prefix((project_id,)).collect_vec());
		}

		#[test]
		fn polkadot_project_with_different_receiving_accounts_can_be_settled() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			let base_price = PriceOf::<TestRuntime>::from_float(1.0);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;

			let evaluations = vec![
				EvaluationParams::from((EVALUATOR_1, 500_000 * USD_UNIT, polkadot_junction!(EVALUATOR_1 + 420))),
				EvaluationParams::from((EVALUATOR_2, 250_000 * USD_UNIT, polkadot_junction!([1u8; 32]))),
				EvaluationParams::from((EVALUATOR_3, 300_000 * USD_UNIT, polkadot_junction!([2u8; 32]))),
			];
			let bids = vec![
				BidParams::from((
					BIDDER_1,
					Retail,
					120_000 * CT_UNIT,
					ParticipationMode::Classic(3u8),
					AcceptedFundingAsset::USDT,
					polkadot_junction!([3u8; 32]),
				)),
				BidParams::from((
					BIDDER_2,
					Retail,
					420_000 * CT_UNIT,
					ParticipationMode::Classic(5u8),
					AcceptedFundingAsset::USDT,
					polkadot_junction!([4u8; 32]),
				)),
			];
			let project_id =
				inst.create_settled_project(project_metadata.clone(), ISSUER_1, None, evaluations, bids, true);
			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementFinished(FundingOutcome::Success)
			);
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
			let project_metadata = inst.get_project_metadata(project_id);

			assert_eq!(project_details.funding_amount_reached_usd, 4_000_000 * USD_UNIT);
			let usd_fee = Percent::from_percent(10u8) * (1_000_000 * USD_UNIT) +
				Percent::from_percent(8u8) * (3_000_000 * USD_UNIT);
			let ct_fee = project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(usd_fee);
			// Liquidity Pools and Long Term Holder Bonus treasury allocation
			let treasury_allocation = Percent::from_percent(50) * ct_fee + Percent::from_percent(20) * ct_fee;

			assert_eq!(project_details.funding_end_block, None);
			assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);
			inst.execute(|| assert!(!<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id)));

			inst.execute(|| {
				assert_ok!(PolimecFunding::start_settlement(RuntimeOrigin::signed(80085), project_id));
			});
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, Some(inst.current_block()));
			assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Success));
			inst.execute(|| assert!(<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id)));

			inst.assert_ct_balance(project_id, ct_treasury, treasury_allocation);
		}

		#[test]
		fn funding_failed_settlement() {
			let (mut inst, project_id) = create_project_with_funding_percentage(32, false);
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, None);
			assert_eq!(project_details.status, ProjectStatus::FundingFailed);
			inst.execute(|| assert!(!<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id)));

			inst.execute(|| {
				assert_ok!(PolimecFunding::start_settlement(RuntimeOrigin::signed(80085), project_id));
			});
			let project_details = inst.get_project_details(project_id);

			assert_eq!(project_details.funding_end_block, Some(inst.current_block()));
			assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Failure));
			inst.execute(|| assert!(!<TestRuntime as Config>::ContributionTokenCurrency::asset_exists(project_id)));
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn called_too_early() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id =
				inst.create_auctioning_project(default_project_metadata(ISSUER_1), ISSUER_1, None, evaluations);
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
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 1_000_000 * CT_UNIT;
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				vec![
					EvaluationParams::from((EVALUATOR_1, 500_000 * USD_UNIT)),
					EvaluationParams::from((EVALUATOR_2, 250_000 * USD_UNIT)),
					EvaluationParams::from((EVALUATOR_3, 320_000 * USD_UNIT)),
				],
				inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 100, 30),
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

			for (_index, (evaluator, expected_reward)) in evals.into_iter().enumerate() {
				let evaluation_locked_plmc =
					inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation.into());
				let free_plmc = inst.get_free_plmc_balance_for(evaluator);
				assert_ok!(inst.execute(|| PolimecFunding::settle_evaluation(
					RuntimeOrigin::signed(evaluator),
					project_id,
					evaluator,
					(evaluator - 21) as u32 // The First evaluation index is 0, the first evaluator account is 21
				)));
				let ct_rewarded = inst.get_ct_asset_balance_for(project_id, evaluator);
				assert_close_enough!(ct_rewarded, expected_reward, Perquintill::from_float(0.9999));
				assert_eq!(inst.get_reserved_plmc_balance_for(evaluator, HoldReason::Evaluation.into()), 0);
				assert_eq!(inst.get_free_plmc_balance_for(evaluator), free_plmc + evaluation_locked_plmc);
				inst.assert_migration(
					project_id,
					evaluator,
					expected_reward,
					ParticipationType::Evaluation,
					polkadot_junction!(evaluator),
					true,
				);
			}
		}

		#[test]
		fn evaluation_slashed() {
			let percentage = 20u8;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);

			let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
			let evaluator = first_evaluation.evaluator;
			let prev_balance = inst.get_free_plmc_balances_for(vec![evaluator])[0].plmc_amount;

			assert_eq!(
				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
				Some(EvaluatorsOutcome::Slashed)
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
			let evaluation = EvaluationParams::from((EVALUATOR_1, 1_000 * USD_UNIT));
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

			let evaluation_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
			inst.mint_plmc_ed_if_required(vec![EVALUATOR_1]);
			inst.mint_plmc_to(evaluation_plmc.clone());
			inst.evaluate_for_users(project_id, vec![evaluation]).unwrap();

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			let evaluation_locked_plmc = inst.get_reserved_plmc_balance_for(EVALUATOR_1, HoldReason::Evaluation.into());
			let free_plmc = inst.get_free_plmc_balance_for(EVALUATOR_1);

			assert_ok!(inst.execute(|| PolimecFunding::settle_evaluation(
				RuntimeOrigin::signed(EVALUATOR_1),
				project_id,
				EVALUATOR_1,
				0
			)));

			assert_eq!(inst.get_ct_asset_balance_for(project_id, EVALUATOR_1), 0);
			assert_eq!(inst.get_reserved_plmc_balance_for(EVALUATOR_1, HoldReason::Evaluation.into()), 0);
			assert_eq!(inst.get_free_plmc_balance_for(EVALUATOR_1), free_plmc + evaluation_locked_plmc);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_settle_twice() {
			let percentage = 100u8;
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
			let percentage = 100u8;
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
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let mut project_metadata = default_project_metadata(ISSUER_1);
			let base_price = PriceOf::<TestRuntime>::from_float(1.0);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation = project_metadata.total_allocation_size;
			let partial_amount_bid_params = BidParams::from((
				BIDDER_1,
				Retail,
				auction_allocation,
				ParticipationMode::Classic(3u8),
				AcceptedFundingAsset::USDT,
			));
			let accepted_bid_params = BidParams::from((
				BIDDER_2,
				Retail,
				2000 * CT_UNIT,
				ParticipationMode::Classic(5u8),
				AcceptedFundingAsset::DOT,
			));
			let bids = vec![partial_amount_bid_params.clone(), accepted_bid_params.clone()];
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_finished_project(project_metadata.clone(), ISSUER_1, None, evaluations, bids);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			// Partial amount bid assertions
			let partial_amount_bid_stored = inst.execute(|| Bids::<TestRuntime>::get(project_id, 0)).unwrap();
			let mut final_partial_amount_bid_params = partial_amount_bid_params.clone();
			final_partial_amount_bid_params.amount = auction_allocation - 2000 * CT_UNIT;
			let expected_final_plmc_bonded = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![final_partial_amount_bid_params.clone()],
				project_metadata.minimum_price,
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

			inst.settle_project(project_id, true);

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				expected_usdt_refund + usdt_ed,
			);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance + expected_final_usdt_paid);

			inst.assert_plmc_free_balance(BIDDER_1, expected_plmc_refund + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, auction_allocation - 2000 * CT_UNIT);

			inst.assert_migration(
				project_id,
				BIDDER_1,
				auction_allocation - 2000 * CT_UNIT,
				ParticipationType::Bid,
				polkadot_junction!(BIDDER_1),
				true,
			);

			let hold_reason: RuntimeHoldReason = HoldReason::Participation.into();
			let vesting_time = Multiplier::force_new(3).calculate_vesting_duration::<TestRuntime>();
			let now = inst.current_block();
			inst.jump_to_block(now + vesting_time + 1u64);
			inst.execute(|| LinearRelease::vest(RuntimeOrigin::signed(BIDDER_1), hold_reason).expect("Vesting failed"));

			inst.assert_plmc_free_balance(BIDDER_1, expected_plmc_refund + expected_final_plmc_bonded + ed);
		}

		#[test]
		fn accepted_bid_without_refund_on_project_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());

			let mut project_metadata = default_project_metadata(ISSUER_1);
			let base_price = PriceOf::<TestRuntime>::from_float(1.0);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation = project_metadata.total_allocation_size;
			let no_refund_bid_params = BidParams::from((
				BIDDER_1,
				Institutional,
				auction_allocation / 2,
				ParticipationMode::Classic(16u8),
				AcceptedFundingAsset::USDT,
			));
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				evaluations,
				vec![no_refund_bid_params.clone()],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let no_refund_bid_stored = inst.execute(|| Bids::<TestRuntime>::get(project_id, 0)).unwrap();

			let pre_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, 0));
			});

			let post_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(BIDDER_1, AcceptedFundingAsset::USDT.id(), usdt_ed);
			assert_eq!(
				post_issuer_usdc_balance,
				pre_issuer_usdc_balance + no_refund_bid_stored.funding_asset_amount_locked
			);

			inst.assert_plmc_free_balance(BIDDER_1, ed);
			inst.assert_ct_balance(project_id, BIDDER_1, auction_allocation / 2);

			inst.assert_migration(
				project_id,
				BIDDER_1,
				auction_allocation / 2,
				ParticipationType::Bid,
				polkadot_junction!(BIDDER_1),
				true,
			);

			let hold_reason: RuntimeHoldReason = HoldReason::Participation.into();
			let multiplier: MultiplierOf<TestRuntime> = no_refund_bid_params.mode.multiplier().try_into().ok().unwrap();
			let vesting_time = multiplier.calculate_vesting_duration::<TestRuntime>();

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
		fn accepted_bid_without_refund_on_project_failure() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let no_refund_bid_params = BidParams::from((
				BIDDER_1,
				Institutional,
				500 * CT_UNIT,
				ParticipationMode::Classic(16u8),
				AcceptedFundingAsset::USDT,
			));

			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				evaluations,
				vec![no_refund_bid_params.clone()],
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			// Partial amount bid assertions
			let no_refund_bid_stored = inst.execute(|| Bids::<TestRuntime>::get(project_id, 0)).unwrap();

			let pre_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.execute(|| {
				assert_ok!(PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, 0));
			});

			let post_issuer_usdc_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				no_refund_bid_stored.funding_asset_amount_locked + usdt_ed,
			);
			assert_eq!(post_issuer_usdc_balance, pre_issuer_usdc_balance);

			inst.assert_plmc_free_balance(BIDDER_1, ed + no_refund_bid_stored.plmc_bond);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason: RuntimeHoldReason = HoldReason::Participation.into();
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
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let mut project_metadata = default_project_metadata(ISSUER_1);
			let base_price = PriceOf::<TestRuntime>::from_float(0.5);
			let decimal_aware_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				base_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.minimum_price = decimal_aware_price;
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT];
			let auction_allocation = project_metadata.total_allocation_size;
			let rejected_bid_params = BidParams::from((
				BIDDER_1,
				Retail,
				auction_allocation,
				ParticipationMode::Classic(4u8),
				AcceptedFundingAsset::USDT,
			));
			let accepted_bid_params = BidParams::from((
				BIDDER_2,
				Retail,
				auction_allocation,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::DOT,
			));

			let bids = vec![rejected_bid_params.clone(), accepted_bid_params.clone()];
			let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let project_id = inst.create_finished_project(project_metadata.clone(), ISSUER_1, None, evaluations, bids);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let rejected_bid_stored = inst.execute(|| Bids::<TestRuntime>::get(project_id, 0)).unwrap();

			let pre_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.settle_project(project_id, true);

			let post_issuer_usdt_balance = inst.get_free_funding_asset_balance_for(
				AcceptedFundingAsset::USDT.id(),
				project_metadata.funding_destination_account,
			);

			inst.assert_funding_asset_free_balance(
				BIDDER_1,
				AcceptedFundingAsset::USDT.id(),
				rejected_bid_stored.funding_asset_amount_locked + usdt_ed,
			);
			assert_eq!(post_issuer_usdt_balance, pre_issuer_usdt_balance);

			inst.assert_plmc_free_balance(BIDDER_1, rejected_bid_stored.plmc_bond + ed);
			inst.assert_ct_balance(project_id, BIDDER_1, Zero::zero());

			let hold_reason = HoldReason::Participation.into();
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
			let percentage = 100u8;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, true);
			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
			inst.execute(|| {
				let bidder = first_bid.bidder;
				assert_ok!(crate::Pallet::<TestRuntime>::settle_bid(
					RuntimeOrigin::signed(bidder),
					project_id,
					first_bid.id
				));
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_bid(RuntimeOrigin::signed(bidder), project_id, first_bid.id),
					Error::<TestRuntime>::ParticipationNotFound
				);
			});
		}

		#[test]
		fn cannot_be_called_before_settlement_started() {
			let percentage = 100u8;
			let (mut inst, project_id) = create_project_with_funding_percentage(percentage, false);
			let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
			let bidder = first_bid.bidder;
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::settle_bid(RuntimeOrigin::signed(bidder), project_id, first_bid.id),
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
