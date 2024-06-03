use super::*;
#[cfg(test)]
mod round_flow {
	use super::*;
	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn evaluator_slash_is_decided() {
			let (mut inst, project_id) = create_project_with_funding_percentage(20, None);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
			assert_eq!(
				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
				EvaluatorsOutcome::Slashed
			);
		}

		#[test]
		fn evaluator_unchanged_is_decided() {
			let (mut inst, project_id) =
				create_project_with_funding_percentage(80, Some(FundingOutcomeDecision::AcceptFunding));
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
			assert_eq!(
				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
				EvaluatorsOutcome::Unchanged
			);
		}

		#[test]
		fn evaluator_reward_is_decided() {
			let (mut inst, project_id) = create_project_with_funding_percentage(95, None);
			let project_details = inst.get_project_details(project_id);
			let project_metadata = inst.get_project_metadata(project_id);
			assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);

			// We want to test rewards over the 3 brackets, which means > 5MM USD funded
			const USD_REACHED: u128 = 9_500_000 * USD_UNIT;
			const FEE_1: Percent = Percent::from_percent(10u8);
			const FEE_2: Percent = Percent::from_percent(8);
			const FEE_3: Percent = Percent::from_percent(6);

			let fee_1 = FEE_1 * 1_000_000 * USD_UNIT;
			let fee_2 = FEE_2 * 4_000_000 * USD_UNIT;
			let fee_3 = FEE_3 * 4_500_000 * USD_UNIT;

			let x = PolimecFunding::calculate_fees(USD_REACHED);
			dbg!(x);
			let total_fee = Perquintill::from_rational(fee_1 + fee_2 + fee_3, USD_REACHED);

			let total_ct_fee =
				total_fee * (project_metadata.total_allocation_size - project_details.remaining_contribution_tokens);

			let total_evaluator_reward =
				Perquintill::from_percent(95u64) * Perquintill::from_percent(30) * total_ct_fee;

			let early_evaluator_reward = Perquintill::from_percent(20u64) * total_evaluator_reward;

			let normal_evaluator_reward = Perquintill::from_percent(80u64) * total_evaluator_reward;
			const EARLY_EVALUATOR_TOTAL_USD_BONDED: u128 = 1_000_000 * USD_UNIT;
			const NORMAL_EVALUATOR_TOTAL_USD_BONDED: u128 = 1_070_000 * USD_UNIT;

			let expected_reward_info = RewardInfoOf::<TestRuntime> {
				early_evaluator_reward_pot: early_evaluator_reward,
				normal_evaluator_reward_pot: normal_evaluator_reward,
				early_evaluator_total_bonded_usd: EARLY_EVALUATOR_TOTAL_USD_BONDED,
				normal_evaluator_total_bonded_usd: NORMAL_EVALUATOR_TOTAL_USD_BONDED,
			};
			assert_eq!(
				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
				EvaluatorsOutcome::Rewarded(expected_reward_info)
			);
		}
	}
}

#[cfg(test)]
mod decide_project_outcome {
	use super::*;
	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn manual_acceptance_percentage_between_34_89() {
			for funding_percent in (34..=89).step_by(5) {
				let _ = create_project_with_funding_percentage(
					funding_percent,
					Some(FundingOutcomeDecision::AcceptFunding),
				);
			}
		}

		#[test]
		fn manual_rejection_percentage_between_34_89() {
			for funding_percent in (34..=89).step_by(5) {
				let _ = create_project_with_funding_percentage(
					funding_percent,
					Some(FundingOutcomeDecision::RejectFunding),
				);
			}
		}

		#[test]
		fn automatic_fail_less_eq_33_percent() {
			for funding_percent in (1..=33).step_by(5) {
				let _ = create_project_with_funding_percentage(funding_percent, None);
			}
		}

		#[test]
		fn automatic_acceptance_on_manual_decision_after_time_delta() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let min_price = project_metadata.minimum_price;
			let twenty_percent_funding_usd = Perquintill::from_percent(55) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids = inst.generate_bids_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let contributions = inst.generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * twenty_percent_funding_usd,
				min_price,
				default_weights(),
				default_community_contributors(),
				default_multipliers(),
			);
			let project_id =
				inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

			inst.advance_time(1u64 + <TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
			inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

			inst.test_ct_created_for(project_id);

			inst.settle_project(project_id).unwrap();
		}

		#[test]
		fn automatic_success_bigger_eq_90_percent() {
			for funding_percent in (90..=100).step_by(2) {
				let _ = create_project_with_funding_percentage(funding_percent, None);
			}
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn called_by_non_issuer() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let funding_percentage = 40u64;
			let min_price = project_metadata.minimum_price;
			let percentage_funded_usd = Perquintill::from_percent(funding_percentage) *
				(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
			let evaluations = default_evaluations();
			let bids = inst.generate_bids_from_total_usd(
				Percent::from_percent(50u8) * percentage_funded_usd,
				min_price,
				default_weights(),
				default_bidders(),
				default_multipliers(),
			);
			let contributions = inst.generate_contributions_from_total_usd(
				Percent::from_percent(50u8) * percentage_funded_usd,
				min_price,
				default_weights(),
				default_community_contributors(),
				default_multipliers(),
			);
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				evaluations,
				bids,
				contributions,
				vec![],
			);

			inst.execute(|| {
				// Accepting doesn't work
				assert_noop!(
					PolimecFunding::decide_project_outcome(
						RuntimeOrigin::signed(BUYER_1),
						get_mock_jwt_with_cid(
							BUYER_1,
							InvestorType::Institutional,
							generate_did_from_account(BUYER_1),
							project_metadata.clone().policy_ipfs_cid.unwrap(),
						),
						project_id,
						FundingOutcomeDecision::AcceptFunding
					),
					Error::<TestRuntime>::NotIssuer
				);
				// Rejecting doesn't work
				assert_noop!(
					PolimecFunding::decide_project_outcome(
						RuntimeOrigin::signed(BUYER_1),
						get_mock_jwt_with_cid(
							BUYER_1,
							InvestorType::Institutional,
							generate_did_from_account(BUYER_1),
							project_metadata.clone().policy_ipfs_cid.unwrap(),
						),
						project_id,
						FundingOutcomeDecision::AcceptFunding
					),
					Error::<TestRuntime>::NotIssuer
				);

				// But Issuer can accept or reject
				assert_ok!(PolimecFunding::decide_project_outcome(
					RuntimeOrigin::signed(ISSUER_1),
					get_mock_jwt_with_cid(
						ISSUER_1,
						InvestorType::Institutional,
						generate_did_from_account(ISSUER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					),
					project_id,
					FundingOutcomeDecision::AcceptFunding
				));
			})
		}

		#[test]
		fn called_on_incorrect_project_status() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);

			let call_fails = |project_id, issuer, inst: &mut MockInstantiator| {
				let jwt = |issuer| {
					get_mock_jwt_with_cid(
						issuer,
						InvestorType::Institutional,
						generate_did_from_account(issuer),
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					)
				};

				inst.execute(|| {
					assert_noop!(
						PolimecFunding::decide_project_outcome(
							RuntimeOrigin::signed(issuer),
							jwt(issuer),
							project_id,
							FundingOutcomeDecision::AcceptFunding
						),
						Error::<TestRuntime>::IncorrectRound
					);
					assert_noop!(
						PolimecFunding::decide_project_outcome(
							RuntimeOrigin::signed(issuer),
							jwt(issuer),
							project_id,
							FundingOutcomeDecision::RejectFunding
						),
						Error::<TestRuntime>::IncorrectRound
					);
				});
			};

			// Application
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::Application);
			call_fails(project_id, ISSUER_1, &mut inst);

			// Evaluation
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_2);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::EvaluationRound);
			call_fails(project_id, ISSUER_2, &mut inst);

			// EvaluationFailed
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_3);
			let transition_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
			inst.jump_to_block(transition_block);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
			call_fails(project_id, ISSUER_3, &mut inst);

			// Auction
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_4, default_evaluations());
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionOpening);
			call_fails(project_id, ISSUER_4, &mut inst);

			// Community
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_5,
				default_evaluations(),
				default_bids(),
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);
			call_fails(project_id, ISSUER_5, &mut inst);

			// Remainder
			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_6,
				default_evaluations(),
				default_bids(),
				vec![],
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::RemainderRound);
			call_fails(project_id, ISSUER_6, &mut inst);

			// FundingSuccessful
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_7,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
				default_remainder_buys(),
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
			call_fails(project_id, ISSUER_7, &mut inst);

			// FundingFailed
			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_8,
				default_evaluations(),
				vec![default_bids()[1].clone()],
				vec![],
				vec![],
			);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
			call_fails(project_id, ISSUER_8, &mut inst);
		}
	}
}
