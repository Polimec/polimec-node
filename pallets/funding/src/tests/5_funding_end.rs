use super::*;
use sp_runtime::PerThing;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn auction_oversubscription() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let bucket_size = Percent::from_percent(10) * auction_allocation;
			let bids = vec![
				(BIDDER_1, auction_allocation).into(),
				(BIDDER_2, bucket_size).into(),
				(BIDDER_3, bucket_size).into(),
				(BIDDER_4, bucket_size).into(),
				(BIDDER_5, bucket_size).into(),
				(BIDDER_6, bucket_size).into(),
			];

			let project_id = inst.create_finished_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
				default_community_contributions(),
				default_remainder_contributions(),
			);

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
			dbg!(wap);
			assert!(wap > project_metadata.minimum_price);
		}
	}
}

#[cfg(test)]
mod end_funding_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn evaluator_reward_is_correct() {
			let (mut inst, project_id) = create_project_with_funding_percentage(95, true);
			let project_details = inst.get_project_details(project_id);
			let project_metadata = inst.get_project_metadata(project_id);
			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementStarted(FundingOutcome::Success)
			);

			// We want to test rewards over the 3 brackets, which means > 5MM USD funded
			const USD_REACHED: u128 = 9_500_000 * USD_UNIT;
			const FEE_1: Percent = Percent::from_percent(10u8);
			const FEE_2: Percent = Percent::from_percent(8);
			const FEE_3: Percent = Percent::from_percent(6);

			let fee_1 = FEE_1 * 1_000_000 * USD_UNIT;
			let fee_2 = FEE_2 * 4_000_000 * USD_UNIT;
			let fee_3 = FEE_3 * 4_500_000 * USD_UNIT;

			let total_fee = Perquintill::from_rational(fee_1 + fee_2 + fee_3, USD_REACHED);

			let total_ct_fee =
				total_fee * (project_metadata.total_allocation_size - project_details.remaining_contribution_tokens);

			let total_evaluator_reward = Perquintill::from_percent(30) * total_ct_fee;

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
				Some(EvaluatorsOutcome::Rewarded(expected_reward_info))
			);
		}

		#[test]
		fn evaluator_outcome_bounds() {
			let try_for_percentage = |percentage: u8, should_slash: bool| {
				let (mut inst, project_id) = create_project_with_funding_percentage(percentage.into(), true);
				dbg!(percentage);
				if should_slash {
					assert_eq!(
						inst.get_project_details(project_id).status,
						ProjectStatus::SettlementStarted(FundingOutcome::Failure)
					);
					assert_eq!(
						inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
						Some(EvaluatorsOutcome::Slashed)
					);
				} else {
					assert_eq!(
						inst.get_project_details(project_id).status,
						ProjectStatus::SettlementStarted(FundingOutcome::Success)
					);
					assert!(matches!(
						inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
						Some(EvaluatorsOutcome::Rewarded(..))
					));
				}
			};
			for i in 1..=32u8 {
				try_for_percentage(i, true);
			}
			for i in 33..130u8 {
				try_for_percentage(i, false);
			}
		}

		#[test]
		fn round_end_is_set() {
			let (mut inst, project_id) = create_project_with_funding_percentage(95, true);
			let project_details = inst.get_project_details(project_id);
			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementStarted(FundingOutcome::Success)
			);
			assert_eq!(
				project_details.funding_end_block,
				Some(
					EvaluationRoundDuration::get() +
						AuctionRoundDuration::get() +
						CommunityRoundDuration::get() +
						RemainderRoundDuration::get() +
						SuccessToSettlementTime::get() +
						1
				)
			);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn called_too_early() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_community_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				None,
				default_evaluations(),
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
			let (mut inst, project_id) = create_project_with_funding_percentage(95, false);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::end_funding(RuntimeOrigin::signed(42), project_id),
					// We don't expect a specific previous state for this transition, so we cannot assert on IncorrectRound error.
					Error::<TestRuntime>::TooEarlyForRound
				);
			});
		}

		#[test]
		fn project_fails_if_not_enough_funding() {
			let funding_threshold = <TestRuntime as Config>::FundingSuccessThreshold::get();
			let funding_threshold: u128 =
				funding_threshold.deconstruct() as u128 * 100u128 / Perquintill::ACCURACY as u128;

			dbg!(funding_threshold);
			let (mut inst, project_id) = create_project_with_funding_percentage(funding_threshold as u64 - 1, true);
			assert_eq!(
				inst.get_project_details(project_id).status,
				ProjectStatus::SettlementStarted(FundingOutcome::Failure)
			);
		}
	}
}
