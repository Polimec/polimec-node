// use super::*;
// #[cfg(test)]
// mod round_flow {
// 	use super::*;
// 	#[cfg(test)]
// 	mod success {
// 		use super::*;
//
// 		#[test]
// 		fn evaluator_slash_is_decided() {
// 			let (mut inst, project_id) = create_project_with_funding_percentage(20, None, true);
// 			assert_eq!(
// 				inst.get_project_details(project_id).status,
// 				ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)
// 			);
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcome::Slashed
// 			);
// 		}
//
// 		#[test]
// 		fn evaluator_unchanged_is_decided() {
// 			let (mut inst, project_id) =
// 				create_project_with_funding_percentage(80, Some(FundingOutcomeDecision::AcceptFunding), true);
// 			assert_eq!(
// 				inst.get_project_details(project_id).status,
// 				ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful)
// 			);
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcome::Unchanged
// 			);
// 		}
//
// 		#[test]
// 		fn evaluator_reward_is_decided() {
// 			let (mut inst, project_id) = create_project_with_funding_percentage(95, None, true);
// 			let project_details = inst.get_project_details(project_id);
// 			let project_metadata = inst.get_project_metadata(project_id);
// 			assert_eq!(
// 				inst.get_project_details(project_id).status,
// 				ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful)
// 			);
//
// 			// We want to test rewards over the 3 brackets, which means > 5MM USD funded
// 			const USD_REACHED: u128 = 9_500_000 * USD_UNIT;
// 			const FEE_1: Percent = Percent::from_percent(10u8);
// 			const FEE_2: Percent = Percent::from_percent(8);
// 			const FEE_3: Percent = Percent::from_percent(6);
//
// 			let fee_1 = FEE_1 * 1_000_000 * USD_UNIT;
// 			let fee_2 = FEE_2 * 4_000_000 * USD_UNIT;
// 			let fee_3 = FEE_3 * 4_500_000 * USD_UNIT;
//
// 			let total_fee = Perquintill::from_rational(fee_1 + fee_2 + fee_3, USD_REACHED);
//
// 			let total_ct_fee =
// 				total_fee * (project_metadata.total_allocation_size - project_details.remaining_contribution_tokens);
//
// 			let total_evaluator_reward =
// 				Perquintill::from_percent(95u64) * Perquintill::from_percent(30) * total_ct_fee;
//
// 			let early_evaluator_reward = Perquintill::from_percent(20u64) * total_evaluator_reward;
//
// 			let normal_evaluator_reward = Perquintill::from_percent(80u64) * total_evaluator_reward;
// 			const EARLY_EVALUATOR_TOTAL_USD_BONDED: u128 = 1_000_000 * USD_UNIT;
// 			const NORMAL_EVALUATOR_TOTAL_USD_BONDED: u128 = 1_070_000 * USD_UNIT;
//
// 			let expected_reward_info = RewardInfoOf::<TestRuntime> {
// 				early_evaluator_reward_pot: early_evaluator_reward,
// 				normal_evaluator_reward_pot: normal_evaluator_reward,
// 				early_evaluator_total_bonded_usd: EARLY_EVALUATOR_TOTAL_USD_BONDED,
// 				normal_evaluator_total_bonded_usd: NORMAL_EVALUATOR_TOTAL_USD_BONDED,
// 			};
// 			assert_eq!(
// 				inst.get_project_details(project_id).evaluation_round_info.evaluators_outcome,
// 				EvaluatorsOutcome::Rewarded(expected_reward_info)
// 			);
// 		}
//
// 		#[test]
// 		fn auction_oversubscription() {
// 			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 			let project_metadata = default_project_metadata(ISSUER_1);
// 			let auction_allocation =
// 				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
// 			let bucket_size = Percent::from_percent(10) * auction_allocation;
// 			let bids = vec![
// 				(BIDDER_1, auction_allocation).into(),
// 				(BIDDER_2, bucket_size).into(),
// 				(BIDDER_3, bucket_size).into(),
// 				(BIDDER_4, bucket_size).into(),
// 				(BIDDER_5, bucket_size).into(),
// 				(BIDDER_6, bucket_size).into(),
// 			];
//
// 			let project_id = inst.create_finished_project(
// 				project_metadata.clone(),
// 				ISSUER_1,
// 				None,
// 				default_evaluations(),
// 				bids,
// 				default_community_buys(),
// 				default_remainder_buys(),
// 			);
//
// 			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
// 			dbg!(wap);
// 			assert!(wap > project_metadata.minimum_price);
// 		}
// 	}
// }
