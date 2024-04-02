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
			let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project_metadata, issuer, evaluations);
		}

		#[test]
		fn multiple_evaluating_projects() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
			let project2 = default_project_metadata(inst.get_new_nonce(), ISSUER_2);
			let project3 = default_project_metadata(inst.get_new_nonce(), ISSUER_3);
			let project4 = default_project_metadata(inst.get_new_nonce(), ISSUER_4);
			let evaluations = default_evaluations();

			inst.create_auctioning_project(project1, ISSUER_1, evaluations.clone());
			inst.create_auctioning_project(project2, ISSUER_2, evaluations.clone());
			inst.create_auctioning_project(project3, ISSUER_3, evaluations.clone());
			inst.create_auctioning_project(project4, ISSUER_4, evaluations);
		}

		#[test]
		fn plmc_price_change_doesnt_affect_evaluation_end() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);

			// Decreasing the price before the end doesn't make a project over the threshold fail.
			let target_funding = project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
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
			assert_eq!(project_status, ProjectStatus::EvaluationFailed);
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
			let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
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

			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationFailed);

			// Check that on_idle has unlocked the failed bonds
			inst.advance_time(10).unwrap();
			inst.do_free_plmc_assertions(expected_evaluator_balances);
		}
	}
}

#[cfg(test)]
mod evaluate_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn evaluation_fails_on_insufficient_balance() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
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
		fn cannot_evaluate_more_than_project_limit() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(0, ISSUER_1);
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
		fn issuer_cannot_evaluate_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(0, ISSUER_1);
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
	}
}




