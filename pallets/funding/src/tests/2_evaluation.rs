use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn evaluation_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project_metadata, issuer, evaluations);
		}

		#[test]
		fn multiple_evaluating_projects() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(ISSUER_1);
			let project2 = default_project_metadata(ISSUER_2);
			let project3 = default_project_metadata(ISSUER_3);
			let project4 = default_project_metadata(ISSUER_4);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project1, ISSUER_1, evaluations.clone());
			inst.create_auctioning_project(project2, ISSUER_2, evaluations.clone());
			inst.create_auctioning_project(project3, ISSUER_3, evaluations.clone());
			inst.create_auctioning_project(project4, ISSUER_4, evaluations);
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
			let evaluation_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let evaluation_existential = evaluation_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(evaluation_plmc);
			inst.mint_plmc_to(evaluation_existential);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1);
			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let old_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			PRICE_MAP.with_borrow_mut(|map| map.insert(PLMC_FOREIGN_ID, old_price / 2.into()));

			inst.start_auction(project_id, ISSUER_1).unwrap();

			// Increasing the price before the end doesn't make a project under the threshold succeed.
			let evaluations = vec![(EVALUATOR_1, target_evaluation_usd / 2).into()];
			let evaluation_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let evaluation_existential = evaluation_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(evaluation_plmc);
			inst.mint_plmc_to(evaluation_existential);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_2);
			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let old_price = <TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap();
			PRICE_MAP.with_borrow_mut(|map| map.insert(PLMC_FOREIGN_ID, old_price * 2.into()));

			let update_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
			let now = inst.current_block();
			inst.advance_time(update_block - now + 1).unwrap();
			let project_status = inst.get_project_details(project_id).status;
			assert_eq!(project_status, ProjectStatus::FundingFailed);
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
				MockInstantiator::calculate_evaluation_plmc_spent(evaluations);
			let plmc_existential_deposits = plmc_eval_deposits.accounts().existential_deposits();

			let expected_evaluator_balances = MockInstantiator::generic_map_operation(
				vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()],
				MergeOperation::Add,
			);

			inst.mint_plmc_to(plmc_eval_deposits.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			let project_id = inst.create_evaluating_project(project_metadata, issuer);

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

			// Check that on_idle has unlocked the failed bonds
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

			let project_id = inst.create_new_project(project_metadata, issuer);
			let jwt = get_mock_jwt(issuer, InvestorType::Institutional, generate_did_from_account(issuer));
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

			let project_id = inst.create_new_project(project_metadata.clone(), issuer);
			let jwt = get_mock_jwt(issuer, InvestorType::Institutional, issuer_did.clone());
			let expected_details = ProjectDetailsOf::<TestRuntime> {
				issuer_account: ISSUER_1,
				issuer_did,
				is_frozen: true,
				weighted_average_price: None,
				status: ProjectStatus::EvaluationRound,
				phase_transition_points: PhaseTransitionPoints {
					application: BlockNumberPair { start: Some(1u64), end: Some(1u64) },
					evaluation: BlockNumberPair {
						start: Some(2u64),
						end: Some(1u64 + <TestRuntime as Config>::EvaluationDuration::get()),
					},
					auction_initialize_period: BlockNumberPair { start: None, end: None },
					auction_opening: BlockNumberPair { start: None, end: None },
					random_closing_ending: None,
					auction_closing: BlockNumberPair { start: None, end: None },
					community: BlockNumberPair { start: None, end: None },
					remainder: BlockNumberPair { start: None, end: None },
				},
				fundraising_target: project_metadata
					.minimum_price
					.saturating_mul_int(project_metadata.total_allocation_size),
				remaining_contribution_tokens: project_metadata.total_allocation_size,
				funding_amount_reached: 0u128,
				evaluation_round_info: EvaluationRoundInfoOf::<TestRuntime> {
					total_bonded_usd: 0u128,
					total_bonded_plmc: 0u128,
					evaluators_outcome: EvaluatorsOutcome::Unchanged,
				},
				funding_end_block: None,
				parachain_id: None,
				migration_readiness_check: None,
				hrmp_channel_status: HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Closed,
					polimec_to_project: ChannelStatus::Closed,
				},
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

		#[test]
		fn non_institutional_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata, issuer);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(
						RuntimeOrigin::signed(issuer),
						get_mock_jwt(issuer, InvestorType::Professional, generate_did_from_account(issuer)),
						project_id
					),
					Error::<TestRuntime>::NotAllowed
				);
			});

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(
						RuntimeOrigin::signed(issuer),
						get_mock_jwt(issuer, InvestorType::Retail, generate_did_from_account(issuer)),
						project_id
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		#[test]
		fn evaluation_started_already() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			let project_id = inst.create_new_project(project_metadata, issuer);
			let jwt = get_mock_jwt(issuer, InvestorType::Institutional, generate_did_from_account(issuer));
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
					Error::<TestRuntime>::ProjectNotInApplicationRound
				);
			});
		}

		#[test]
		fn no_offchain_hash_provided() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.offchain_information_hash = None;

			let project_id = inst.create_new_project(project_metadata, issuer);
			let jwt = get_mock_jwt(issuer, InvestorType::Institutional, generate_did_from_account(issuer));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::start_evaluation(RuntimeOrigin::signed(issuer), jwt, project_id),
					Error::<TestRuntime>::MetadataNotProvided
				);
			});
		}

		#[test]
		fn different_account() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			let project_id = inst.create_new_project(project_metadata, ISSUER_1);
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
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
						get_mock_jwt(ISSUER_2, InvestorType::Institutional, generate_did_from_account(ISSUER_2)),
						project_id
					),
					Error::<TestRuntime>::NotAllowed
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

		#[test]
		fn all_investor_types() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

			let evaluations = vec![
				(EVALUATOR_1, 500 * US_DOLLAR).into(),
				(EVALUATOR_2, 1000 * US_DOLLAR).into(),
				(EVALUATOR_3, 20_000 * US_DOLLAR).into(),
			];
			let necessary_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc);
			inst.mint_plmc_to(plmc_existential_deposits);

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[0].account),
				get_mock_jwt(
					evaluations[0].account,
					InvestorType::Institutional,
					generate_did_from_account(evaluations[0].account)
				),
				project_id,
				evaluations[0].usd_amount,
			)));

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[1].account),
				get_mock_jwt(
					evaluations[1].account,
					InvestorType::Professional,
					generate_did_from_account(evaluations[1].account)
				),
				project_id,
				evaluations[1].usd_amount,
			)));

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluations[2].account),
				get_mock_jwt(
					evaluations[2].account,
					InvestorType::Retail,
					generate_did_from_account(evaluations[2].account)
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
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * US_DOLLAR);
			let necessary_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_deposits);

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &EVALUATOR_1, necessary_plmc[0].plmc_amount).unwrap();
			});

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluation.account),
				get_mock_jwt(evaluation.account, InvestorType::Retail, generate_did_from_account(evaluation.account)),
				project_id,
				evaluation.usd_amount,
			)));
		}

		#[test]
		fn storage_check() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * US_DOLLAR);
			let necessary_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();
			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_deposits);

			inst.execute(|| {
				assert_eq!(Evaluations::<TestRuntime>::iter_values().collect_vec(), vec![]);
			});

			assert_ok!(inst.execute(|| PolimecFunding::evaluate(
				RuntimeOrigin::signed(evaluation.account),
				get_mock_jwt(evaluation.account, InvestorType::Retail, generate_did_from_account(evaluation.account)),
				project_id,
				evaluation.usd_amount,
			)));

			inst.execute(|| {
				let evaluations = Evaluations::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec();
				assert_eq!(evaluations.len(), 1);
				let stored_evaluation = &evaluations[0];
				let expected_evaluation_item = EvaluationInfoOf::<TestRuntime> {
					id: 0,
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
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn project_is_not_in_evaluation_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, default_evaluations());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(EVALUATOR_1),
						get_mock_jwt(EVALUATOR_1, InvestorType::Retail, generate_did_from_account(EVALUATOR_1)),
						project_id,
						500 * US_DOLLAR,
					),
					Error::<TestRuntime>::ProjectNotInEvaluationRound
				);
			});
		}

		#[test]
		fn insufficient_plmc_for_desired_evaluation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let insufficient_eval_deposits = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone())
				.iter()
				.map(|UserToPLMCBalance { account, plmc_amount }| UserToPLMCBalance::new(*account, plmc_amount / 2))
				.collect::<Vec<UserToPLMCBalance<_>>>();

			let plmc_existential_deposits = insufficient_eval_deposits.accounts().existential_deposits();

			inst.mint_plmc_to(insufficient_eval_deposits);
			inst.mint_plmc_to(plmc_existential_deposits);

			let project_id = inst.create_evaluating_project(project_metadata, issuer);

			let dispatch_error = inst.evaluate_for_users(project_id, evaluations);
			assert_err!(dispatch_error, TokenError::FundsUnavailable)
		}

		#[test]
		fn evaluation_placing_user_balance_under_ed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = vec![UserToUSDBalance::new(EVALUATOR_1, 1000 * US_DOLLAR)];
			let evaluating_plmc = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let mut plmc_insufficient_existential_deposit = evaluating_plmc.accounts().existential_deposits();

			plmc_insufficient_existential_deposit[0].plmc_amount =
				plmc_insufficient_existential_deposit[0].plmc_amount / 2;

			inst.mint_plmc_to(evaluating_plmc);
			inst.mint_plmc_to(plmc_insufficient_existential_deposit);

			let project_id = inst.create_evaluating_project(project_metadata, issuer);

			let dispatch_error = inst.evaluate_for_users(project_id, evaluations);
			assert_err!(dispatch_error, TokenError::FundsUnavailable)
		}

		#[test]
		fn cannot_evaluate_more_than_project_limit() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = (0u32..<TestRuntime as Config>::MaxEvaluationsPerProject::get())
				.map(|i| UserToUSDBalance::<TestRuntime>::new(i as u32 + 420u32, (10u128 * ASSET_UNIT).into()))
				.collect_vec();
			let failing_evaluation = UserToUSDBalance::new(EVALUATOR_1, 1000 * ASSET_UNIT);

			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1);

			let plmc_for_evaluating = MockInstantiator::calculate_evaluation_plmc_spent(evaluations.clone());
			let plmc_existential_deposits = evaluations.accounts().existential_deposits();

			inst.mint_plmc_to(plmc_for_evaluating.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			inst.evaluate_for_users(project_id, evaluations.clone()).unwrap();

			let plmc_for_failing_evaluating =
				MockInstantiator::calculate_evaluation_plmc_spent(vec![failing_evaluation.clone()]);
			let plmc_existential_deposits = plmc_for_failing_evaluating.accounts().existential_deposits();

			inst.mint_plmc_to(plmc_for_failing_evaluating.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			assert_err!(
				inst.evaluate_for_users(project_id, vec![failing_evaluation]),
				Error::<TestRuntime>::TooManyEvaluationsForProject
			);
		}

		#[test]
		fn cannot_use_balance_on_hold() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * US_DOLLAR);
			let necessary_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_deposits);

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
						get_mock_jwt(
							evaluation.account,
							InvestorType::Retail,
							generate_did_from_account(evaluation.account)
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
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1);
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_evaluate(
					&(&ISSUER_1 + 1),
					project_id,
					500 * US_DOLLAR,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional
				)),
				Error::<TestRuntime>::ParticipationToThemselves
			);
		}

		#[test]
		fn cannot_use_same_plmc_for_2_evaluations() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

			let evaluation = UserToUSDBalance::new(EVALUATOR_1, 500 * US_DOLLAR);
			let necessary_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
			let plmc_existential_deposits = necessary_plmc.accounts().existential_deposits();

			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_plmc_to(plmc_existential_deposits);

			inst.execute(|| {
				assert_ok!(PolimecFunding::evaluate(
					RuntimeOrigin::signed(evaluation.account),
					get_mock_jwt(
						evaluation.account,
						InvestorType::Retail,
						generate_did_from_account(evaluation.account)
					),
					project_id,
					evaluation.usd_amount,
				));
			});

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::evaluate(
						RuntimeOrigin::signed(evaluation.account),
						get_mock_jwt(
							evaluation.account,
							InvestorType::Retail,
							generate_did_from_account(evaluation.account)
						),
						project_id,
						evaluation.usd_amount,
					),
					TokenError::FundsUnavailable
				);
			});
		}

		#[test]
		fn cannot_evaluate_with_0_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

			let evaluator = EVALUATOR_1;
            let evaluation = (evaluator.clone(), 0).into();
            inst.mint_plmc_to(vec![(evaluator.clone(), 2000 * PLMC).into()]);
			assert_err!(inst.evaluate_for_users(project_id, vec![evaluation]), Error::<TestRuntime>::EvaluationBondTooLow)
		}
	}
}
