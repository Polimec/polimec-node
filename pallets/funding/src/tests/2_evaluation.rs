use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use std::collections::HashSet;

		#[test]
		fn evaluation_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project_metadata, issuer, None, evaluations);
		}

		#[test]
		fn multiple_evaluating_projects() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(ISSUER_1);
			let project2 = default_project_metadata(ISSUER_2);
			let project3 = default_project_metadata(ISSUER_3);
			let project4 = default_project_metadata(ISSUER_4);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project1, ISSUER_1, None, evaluations.clone());
			inst.create_auctioning_project(project2, ISSUER_2, None, evaluations.clone());
			inst.create_auctioning_project(project3, ISSUER_3, None, evaluations.clone());
			inst.create_auctioning_project(project4, ISSUER_4, None, evaluations);
		}

		#[test]
		fn plmc_price_change_doesnt_affect_evaluation_end() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			// Decreasing the price before the end doesn't make a project over the threshold fail.
			let target_funding =
				project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
			let target_evaluation_usd = Percent::from_percent(10) * target_funding;

			let evaluations = vec![(EVALUATOR_1, target_evaluation_usd).into()];
			let evaluation_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);
			inst.mint_plmc_to(evaluation_plmc);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let old_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			PRICE_MAP.with_borrow_mut(|map| map.insert(PLMC_FOREIGN_ID, old_price / 2.into()));

			inst.start_auction(project_id, ISSUER_1).unwrap();

			// Increasing the price before the end doesn't make a project under the threshold succeed.
			let evaluations = vec![(EVALUATOR_1, target_evaluation_usd / 2).into()];
			let evaluation_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);
			inst.mint_plmc_to(evaluation_plmc);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_2, None);
			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let old_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			PRICE_MAP.with_borrow_mut(|map| map.insert(PLMC_FOREIGN_ID, old_price * 2.into()));

			let update_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
			let now = inst.current_block();
			inst.advance_time(update_block - now + 1).unwrap();
			let project_status = inst.get_project_details(project_id).status;
			assert_eq!(project_status, ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed));
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
			let min_evaluation_amount_usd = <TestRuntime as Config>::MinUsdPerEvaluation::get();
			let stored_plmc_price =
				inst.execute(|| <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap());
			let usable_plmc_price = inst.execute(|| {
				<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(
					PLMC_FOREIGN_ID,
					USD_DECIMALS,
					PLMC_DECIMALS,
				)
				.unwrap()
			});
			let min_evaluation_amount_plmc =
				usable_plmc_price.reciprocal().unwrap().checked_mul_int(min_evaluation_amount_usd).unwrap();

			// Test independent of CT decimals - Right PLMC conversion is stored.
			// We move comma 4 places to the left since PLMC has 4 more decimals than USD.
			assert_eq!(stored_plmc_price, FixedU128::from_float(8.4));
			assert_eq!(usable_plmc_price, FixedU128::from_float(0.00084));

			let mut evaluation_ct_thresholds = Vec::new();
			let mut evaluation_usd_thresholds = Vec::new();
			let mut evaluation_plmc_thresholds = Vec::new();

			let mut decimal_test = |decimals: u8| {
				let mut project_metadata = default_project_metadata.clone();
				project_metadata.token_information.decimals = decimals;
				project_metadata.minimum_price =
					<TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
						original_price,
						USD_DECIMALS,
						decimals,
					)
					.unwrap();
				project_metadata.total_allocation_size = 1_000_000 * 10u128.pow(decimals as u32);
				project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;

				let issuer: AccountIdOf<TestRuntime> = (10_000 + inst.get_new_nonce()).try_into().unwrap();
				let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

				let evaluation_threshold = inst.execute(|| <TestRuntime as Config>::EvaluationSuccessThreshold::get());
				let evaluation_threshold_ct = evaluation_threshold * project_metadata.total_allocation_size;
				evaluation_ct_thresholds.push(evaluation_threshold_ct);

				let evaluation_threshold_usd =
					project_metadata.minimum_price.saturating_mul_int(evaluation_threshold_ct);
				evaluation_usd_thresholds.push(evaluation_threshold_usd);

				let evaluation_threshold_plmc =
					usable_plmc_price.reciprocal().unwrap().checked_mul_int(evaluation_threshold_usd).unwrap();
				evaluation_plmc_thresholds.push(evaluation_threshold_plmc);

				// CT price should be multiplied or divided by the amount of decimal difference with USD.
				let decimal_abs_diff = USD_DECIMALS.abs_diff(decimals);
				let original_price_as_usd = original_price.saturating_mul_int(10u128.pow(USD_DECIMALS as u32));
				let min_price_as_usd = project_metadata.minimum_price.saturating_mul_int(USD_UNIT);
				if decimals < USD_DECIMALS {
					assert_eq!(min_price_as_usd, original_price_as_usd * 10u128.pow(decimal_abs_diff as u32));
				} else {
					assert_eq!(min_price_as_usd, original_price_as_usd / 10u128.pow(decimal_abs_diff as u32));
				}

				// A minimum evaluation goes through. This is a fixed USD/PLMC value, so independent of CT decimals.
				inst.mint_plmc_to(vec![UserToPLMCBalance::new(EVALUATOR_1, min_evaluation_amount_plmc + ed)]);
				assert_ok!(inst.execute(|| PolimecFunding::evaluate(
					RuntimeOrigin::signed(EVALUATOR_1),
					get_mock_jwt_with_cid(
						EVALUATOR_1,
						InvestorType::Retail,
						generate_did_from_account(EVALUATOR_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					min_evaluation_amount_usd
				)));

				// Try bonding up to the threshold with a second evaluation
				inst.mint_plmc_to(vec![UserToPLMCBalance::new(
					EVALUATOR_2,
					evaluation_threshold_plmc + ed - min_evaluation_amount_plmc,
				)]);
				assert_ok!(inst.execute(|| PolimecFunding::evaluate(
					RuntimeOrigin::signed(EVALUATOR_2),
					get_mock_jwt_with_cid(
						EVALUATOR_2,
						InvestorType::Retail,
						generate_did_from_account(EVALUATOR_2),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					evaluation_threshold_usd - min_evaluation_amount_usd
				)));

				// The evaluation should succeed when we bond the threshold PLMC amount in total.
				inst.start_auction(project_id, issuer).unwrap();
			};

			for decimals in 6..=18 {
				decimal_test(decimals);
			}

			// Since we use the same original price and allocation size and adjust for decimals,
			// the USD and PLMC amounts should be the same
			assert!(evaluation_usd_thresholds.iter().all(|x| *x == evaluation_usd_thresholds[0]));
			assert!(evaluation_plmc_thresholds.iter().all(|x| *x == evaluation_plmc_thresholds[0]));

			// CT amounts however should be different from each other
			let mut hash_set = HashSet::new();
			for amount in evaluation_ct_thresholds {
				assert!(!hash_set.contains(&amount));
				hash_set.insert(amount);
			}
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn round_fails_after_not_enough_bonds() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let now = inst.current_block();
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_failing_evaluations();
			let plmc_eval_deposits: Vec<UserToPLMCBalance<_>> =
				inst.calculate_evaluation_plmc_spent(evaluations, false);
			let plmc_existential_deposits = plmc_eval_deposits.accounts().existential_deposits();

			let expected_evaluator_balances = inst.generic_map_operation(
				vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

			inst.mint_plmc_to(plmc_eval_deposits.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			let project_id = inst.create_evaluating_project(project_metadata, issuer, None);

			let evaluation_end = inst
				.get_project_details(project_id)
				.phase_transition_points
				.evaluation
				.end
				.expect("Evaluation round end block should be set");

			inst.evaluate_for_users(project_id, default_failing_evaluations()).expect("Bonding should work");

			inst.do_free_plmc_assertions(plmc_existential_deposits);
			inst.do_reserved_plmc_assertions(plmc_eval_deposits, HoldReason::Evaluation(project_id).into());

			inst.advance_time(evaluation_end - now + 1).unwrap();

			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
			inst.jump_to_block(settlement_block);

			inst.settle_project(project_id).unwrap();
			inst.do_free_plmc_assertions(expected_evaluator_balances);
		}
	}
}

#[cfg(test)]
mod start_evaluation_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn evaluation_starts() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata.clone(), issuer, None);
			let jwt = get_mock_jwt_with_cid(
				issuer,
				InvestorType::Institutional,
				generate_did_from_account(issuer),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			assert_ok!(inst.execute(|| PolimecFunding::start_evaluation(
				RuntimeOrigin::signed(issuer),
				jwt,
				project_id
			)));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationRound);
		}

		#[test]
		fn storage_is_updated() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let issuer_did = generate_did_from_account(issuer);
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata.clone(), issuer, None);
			let jwt = get_mock_jwt_with_cid(
				issuer,
				InvestorType::Institutional,
				issuer_did.clone(),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let expected_details = ProjectDetailsOf::<TestRuntime> {
				issuer_account: ISSUER_1,
				issuer_did,
				is_frozen: true,
				weighted_average_price: None,
				status: ProjectStatus::EvaluationRound,
				phase_transition_points: PhaseTransitionPoints {
					application: BlockNumberPair { start: Some(1u64), end: Some(1u64) },
					evaluation: BlockNumberPair {
						start: Some(1u64),
						end: Some(<TestRuntime as Config>::EvaluationDuration::get()),
					},
					auction_initialize_period: BlockNumberPair { start: None, end: None },
					auction_opening: BlockNumberPair { start: None, end: None },
					random_closing_ending: None,
					auction_closing: BlockNumberPair { start: None, end: None },
					community: BlockNumberPair { start: None, end: None },
					remainder: BlockNumberPair { start: None, end: None },
				},
				fundraising_target_usd: project_metadata
					.minimum_price
					.saturating_mul_int(project_metadata.total_allocation_size),
				remaining_contribution_tokens: project_metadata.total_allocation_size,
				funding_amount_reached_usd: 0u128,
				evaluation_round_info: EvaluationRoundInfoOf::<TestRuntime> {
					total_bonded_usd: 0u128,
					total_bonded_plmc: 0u128,
					evaluators_outcome: EvaluatorsOutcome::Unchanged,
				},
				usd_bid_on_oversubscription: None,
				funding_end_block: None,
				migration_type: None,
			};
			assert_ok!(inst.execute(|| PolimecFunding::start_evaluation(
				RuntimeOrigin::signed(issuer),
				jwt,
				project_id
			)));

			assert_eq!(inst.get_project_details(project_id), expected_details);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;
		use polimec_common_test_utils::get_mock_jwt;

		#[test]
		fn non_institutional_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata.clone(), issuer, None);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(
						RuntimeOrigin::signed(issuer),
						get_mock_jwt_with_cid(
							issuer,
							InvestorType::Professional,
							generate_did_from_account(issuer),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(
						RuntimeOrigin::signed(issuer),
						get_mock_jwt_with_cid(
							issuer,
							InvestorType::Retail,
							generate_did_from_account(issuer),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});
		}

		#[test]
		fn evaluation_started_already() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata.clone(), issuer, None);
			let jwt = get_mock_jwt_with_cid(
				issuer,
				InvestorType::Institutional,
				generate_did_from_account(issuer),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			assert_ok!(inst.execute(|| PolimecFunding::start_evaluation(
				RuntimeOrigin::signed(issuer),
				jwt.clone(),
				project_id
			)));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationRound);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(RuntimeOrigin::signed(issuer), jwt, project_id),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn no_policy_provided() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.policy_ipfs_cid = None;

			let project_id = inst.create_new_project(project_metadata.clone(), issuer, None);
			let jwt = get_mock_jwt(issuer, InvestorType::Institutional, generate_did_from_account(issuer));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(RuntimeOrigin::signed(issuer), jwt, project_id),
					Error::<TestRuntime>::CidNotProvided
				);
			});
		}

		#[test]
		fn different_account() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			assert_ok!(inst.execute(|| PolimecFunding::start_evaluation(
				RuntimeOrigin::signed(ISSUER_1),
				jwt,
				project_id
			)));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationRound);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(
						RuntimeOrigin::signed(ISSUER_2),
						get_mock_jwt_with_cid(
							ISSUER_2,
							InvestorType::Institutional,
							generate_did_from_account(ISSUER_2),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
					),
					Error::<TestRuntime>::NotIssuer
				);
			});
		}
	}
}

#[cfg(test)]
mod evaluate_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use frame_support::traits::fungible::InspectFreeze;
		use pallet_balances::AccountData;

		#[test]
		fn all_investor_types() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

			let evaluations = vec![
				(EVALUATOR_1, 500 * USD_UNIT).into(),
				(EVALUATOR_2, 1000 * USD_UNIT).into(),
				(EVALUATOR_3, 20_000 * USD_UNIT).into(),
			];
			let necessary_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

			inst.mint_plmc_to(necessary_plmc);

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[0].account),
				get_mock_jwt_with_cid(
					evaluations[0].account,
					InvestorType::Institutional,
					generate_did_from_account(evaluations[0].account),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				evaluations[0].usd_amount,
			)));

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[1].account),
				get_mock_jwt_with_cid(
					evaluations[1].account,
					InvestorType::Professional,
					generate_did_from_account(evaluations[1].account),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				evaluations[1].usd_amount,
			)));

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[2].account),
				get_mock_jwt_with_cid(
					evaluations[2].account,
					InvestorType::Retail,
					generate_did_from_account(evaluations[2].account),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				evaluations[2].usd_amount,
			)));
		}

		#[test]
		fn using_frozen_tokens() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * USD_UNIT);
			let necessary_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], true);

			inst.mint_plmc_to(necessary_plmc.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &EVALUATOR_1, necessary_plmc[0].plmc_amount).unwrap();
			});

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluation.account),
				get_mock_jwt_with_cid(
					evaluation.account,
					InvestorType::Retail,
					generate_did_from_account(evaluation.account),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				evaluation.usd_amount
			)));
		}

		#[test]
		fn storage_check() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * USD_UNIT);
			let necessary_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], false);
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			inst.execute(|| {
				assert_eq!(Evaluations::<TestRuntime>::iter_values().collect_vec(), vec![]);
			});

			let did = generate_did_from_account(evaluation.account);

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluation.account),
				get_mock_jwt_with_cid(
					evaluation.account,
					InvestorType::Retail,
					did.clone(),
					project_metadata.clone().policy_ipfs_cid.unwrap()
				),
				project_id,
				evaluation.usd_amount
			)));

			inst.execute(|| {
				let evaluations = Evaluations::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec();
				assert_eq!(evaluations.len(), 1);
				let stored_evaluation = &evaluations[0];
				let expected_evaluation_item = EvaluationInfoOf::<TestRuntime> {
					id: 0,
					did,
					project_id: 0,
					evaluator: EVALUATOR_1,
					original_plmc_bond: necessary_plmc[0].plmc_amount,
					current_plmc_bond: necessary_plmc[0].plmc_amount,
					early_usd_amount: evaluation.usd_amount,
					late_usd_amount: 0,
					when: 1,
				};
				assert_eq!(stored_evaluation, &expected_evaluation_item);
			});
		}

		#[test]
		fn can_evaluate_with_frozen_tokens() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let evaluation = UserToUSDBalance::new(EVALUATOR_4, 1_000_000 * USD_UNIT);
			let plmc_required = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], false);
			let plmc_existential_deposits = plmc_required.accounts().existential_deposits();
			let frozen_amount = plmc_required[0].plmc_amount;

			inst.mint_plmc_to(plmc_required.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &EVALUATOR_4, plmc_required[0].plmc_amount).unwrap();
			});

			inst.execute(|| {
				assert_noop!(
					Balances::transfer_allow_death(RuntimeOrigin::signed(EVALUATOR_4), ISSUER_1, frozen_amount,),
					TokenError::Frozen
				);
			});

			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);
			inst.execute(|| {
				assert_ok!(PolimecFunding::evaluate(
					RuntimeOrigin::signed(EVALUATOR_4),
					get_mock_jwt_with_cid(
						EVALUATOR_4,
						InvestorType::Retail,
						generate_did_from_account(EVALUATOR_4),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					evaluation.usd_amount
				));
			});

			let new_evaluations = default_evaluations();
			let new_plmc_required = inst.calculate_evaluation_plmc_spent(new_evaluations.clone(), true);
			inst.mint_plmc_to(new_plmc_required.clone());
			inst.evaluate_for_users(project_id, new_evaluations).unwrap();

			inst.start_auction(project_id, ISSUER_1).unwrap();
			inst.start_community_funding(project_id).unwrap();
			inst.start_remainder_or_end_funding(project_id).unwrap();
			inst.finish_funding(project_id, None).unwrap();

			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

			let free_balance = inst.get_free_plmc_balance_for(EVALUATOR_4);
			let evaluation_held_balance =
				inst.get_reserved_plmc_balance_for(EVALUATOR_4, HoldReason::Evaluation(project_id).into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &EVALUATOR_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(evaluation_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			let treasury_account = <TestRuntime as Config>::BlockchainOperationTreasury::get();
			let pre_slash_treasury_balance = inst.get_free_plmc_balance_for(treasury_account);

			let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
			inst.jump_to_block(settlement_block);

			inst.execute(|| {
				PolimecFunding::settle_failed_evaluation(
					RuntimeOrigin::signed(EVALUATOR_4),
					project_id,
					EVALUATOR_4,
					0,
				)
				.unwrap();
			});

			let post_slash_treasury_balance = inst.get_free_plmc_balance_for(treasury_account);
			let free_balance = inst.get_free_plmc_balance_for(EVALUATOR_4);
			let evaluation_held_balance =
				inst.get_reserved_plmc_balance_for(EVALUATOR_4, HoldReason::Evaluation(project_id).into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &EVALUATOR_4));
			let account_data = inst.execute(|| System::account(&EVALUATOR_4)).data;

			let post_slash_evaluation_plmc =
				frozen_amount - (<TestRuntime as Config>::EvaluatorSlash::get() * frozen_amount);
			assert_eq!(free_balance, inst.get_ed() + post_slash_evaluation_plmc);
			assert_eq!(evaluation_held_balance, Zero::zero());
			assert_eq!(frozen_balance, frozen_amount);
			let expected_account_data = AccountData {
				free: inst.get_ed() + post_slash_evaluation_plmc,
				reserved: Zero::zero(),
				frozen: frozen_amount,
				flags: Default::default(),
			};
			assert_eq!(account_data, expected_account_data);

			assert!(account_data.frozen > account_data.free);
			assert_eq!(
				post_slash_treasury_balance,
				pre_slash_treasury_balance + <TestRuntime as Config>::EvaluatorSlash::get() * frozen_amount
			);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn project_is_not_in_evaluation_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), issuer, None, default_evaluations());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(EVALUATOR_1),
						get_mock_jwt_with_cid(
							EVALUATOR_1,
							InvestorType::Retail,
							generate_did_from_account(EVALUATOR_1),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						500 * USD_UNIT,
					),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn insufficient_plmc_for_desired_evaluation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let insufficient_eval_deposits = inst
				.calculate_evaluation_plmc_spent(evaluations.clone(), false)
				.iter()
				.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance::new(*account, plmc_amount / 2))
				.collect::<Vec<UserToPLMCBalance<_>>>();

			let plmc_existential_deposits = insufficient_eval_deposits.accounts().existential_deposits();

			inst.mint_plmc_to(insufficient_eval_deposits);
			inst.mint_plmc_to(plmc_existential_deposits);

			let project_id = inst.create_evaluating_project(project_metadata, issuer, None);

			let dispatch_error = inst.evaluate_for_users(project_id, evaluations);
			assert_err!(dispatch_error, TokenError::FundsUnavailable)
		}

		#[test]
		fn evaluation_placing_user_balance_under_ed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = vec![UserToUSDBalance::new(EVALUATOR_1, 1000 * USD_UNIT)];
			let evaluating_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone(), false);
			let mut plmc_insufficient_existential_deposit = evaluating_plmc.accounts().existential_deposits();

			plmc_insufficient_existential_deposit[0].plmc_amount =
				plmc_insufficient_existential_deposit[0].plmc_amount / 2;

			inst.mint_plmc_to(evaluating_plmc);
			inst.mint_plmc_to(plmc_insufficient_existential_deposit);

			let project_id = inst.create_evaluating_project(project_metadata, issuer, None);

			let dispatch_error = inst.evaluate_for_users(project_id, evaluations);
			assert_err!(dispatch_error, TokenError::FundsUnavailable)
		}

		#[test]
		fn cannot_evaluate_more_than_project_limit() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = (0u32..<TestRuntime as Config>::MaxEvaluationsPerProject::get())
				.map(|i| UserToUSDBalance::<TestRuntime>::new(i as u32 + 420u32, (100u128 * CT_UNIT).into()))
				.collect_vec();
			let failing_evaluation = UserToUSDBalance::new(EVALUATOR_1, 1000 * CT_UNIT);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

			let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

			inst.mint_plmc_to(plmc_for_evaluating.clone());

			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let plmc_for_failing_evaluating =
				inst.calculate_evaluation_plmc_spent(vec![failing_evaluation.clone()], true);

			inst.mint_plmc_to(plmc_for_failing_evaluating.clone());

			assert_err!(
				inst.evaluate_for_users(project_id, vec![failing_evaluation]),
				Error::<TestRuntime>::TooManyProjectParticipations
			);
		}

		#[test]
		fn cannot_evaluate_more_than_user_limit() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = (0u32..<TestRuntime as Config>::MaxEvaluationsPerUser::get())
				.map(|_| UserToUSDBalance::<TestRuntime>::new(EVALUATOR_1, (100u128 * USD_UNIT).into()))
				.collect_vec();
			let failing_evaluation = UserToUSDBalance::new(EVALUATOR_1, 100 * USD_UNIT);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

			let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), false);
			let plmc_existential_deposits = evaluations.accounts().existential_deposits();

			inst.mint_plmc_to(plmc_for_evaluating.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let plmc_for_failing_evaluating =
				inst.calculate_evaluation_plmc_spent(vec![failing_evaluation.clone()], true);

			inst.mint_plmc_to(plmc_for_failing_evaluating.clone());

			assert_err!(
				inst.evaluate_for_users(project_id, vec![failing_evaluation]),
				Error::<TestRuntime>::TooManyUserParticipations
			);
		}

		#[test]
		fn cannot_use_balance_on_hold() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * USD_UNIT);
			let necessary_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], false);
			let ed = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(ed.clone());

			inst.execute(|| {
				<TestRuntime as Config>::NativeCurrency::hold(
					&RuntimeHoldReason::PolimecFunding(HoldReason::Evaluation(69)),
					&EVALUATOR_1,
					necessary_plmc[0].plmc_amount,
				)
				.unwrap();
			});

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(evaluation.account),
						get_mock_jwt_with_cid(
							evaluation.account,
							InvestorType::Retail,
							generate_did_from_account(evaluation.account),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						evaluation.usd_amount,
					),
					TokenError::FundsUnavailable
				);
			});
		}

		#[test]
		fn issuer_cannot_evaluate_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_evaluate(
					&(&ISSUER_1 + 1),
					project_id,
					500 * USD_UNIT,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				)),
				Error::<TestRuntime>::ParticipationToOwnProject
			);
		}

		#[test]
		fn cannot_use_same_plmc_for_2_evaluations() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * USD_UNIT);
			let necessary_plmc = inst.calculate_evaluation_plmc_spent(vec![evaluation.clone()], true);

			inst.mint_plmc_to(necessary_plmc.clone());

			inst.execute(|| {
				assert_ok!(PolimecFunding::evaluate(
					RuntimeOrigin::signed(evaluation.account),
					get_mock_jwt_with_cid(
						evaluation.account,
						InvestorType::Retail,
						generate_did_from_account(evaluation.account),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					evaluation.usd_amount,
				));
			});

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(evaluation.account),
						get_mock_jwt_with_cid(
							evaluation.account,
							InvestorType::Retail,
							generate_did_from_account(evaluation.account),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						evaluation.usd_amount,
					),
					TokenError::FundsUnavailable
				);
			});
		}

		#[test]
		fn cannot_evaluate_with_less_than_100_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);
			let evaluator = EVALUATOR_1;
			let jwt = get_mock_jwt_with_cid(
				evaluator,
				InvestorType::Retail,
				generate_did_from_account(evaluator),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			inst.mint_plmc_to(vec![(evaluator.clone(), 2000 * PLMC).into()]);

			// Cannot evaluate with 0 USD
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::evaluate(
						RuntimeOrigin::signed(evaluator.clone()),
						jwt.clone(),
						project_id,
						0
					),
					Error::<TestRuntime>::TooLow
				);
			});

			// Cannot evaluate with less than 99 USD
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::evaluate(
						RuntimeOrigin::signed(evaluator.clone()),
						jwt.clone(),
						project_id,
						99 * USD_UNIT
					),
					Error::<TestRuntime>::TooLow
				);
			});
		}

		#[test]
		fn wrong_policy_on_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(EVALUATOR_1),
						get_mock_jwt_with_cid(
							EVALUATOR_1,
							InvestorType::Retail,
							generate_did_from_account(EVALUATOR_1),
							"wrong_cid".as_bytes().to_vec().try_into().unwrap()
						),
						project_id,
						500 * USD_UNIT,
					),
					Error::<TestRuntime>::PolicyMismatch
				);
			});
		}
	}
}
