#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod helpers;

#[test]
fn output_max_pallet_funding_values() {
	use crate::weights::pallet_funding::SubstrateWeight;
	use frame_support::pallet_prelude::Get;
	use pallet_funding::weights::WeightInfo;

	use crate::Runtime;

	let max_evaluations_per_user: u32 = <Runtime as pallet_funding::Config>::MaxEvaluationsPerUser::get();
	let max_bids_per_user: u32 = <Runtime as pallet_funding::Config>::MaxBidsPerUser::get();
	let max_contributions_per_user: u32 = <Runtime as pallet_funding::Config>::MaxContributionsPerUser::get();
	let max_bids_per_project: u32 = <Runtime as pallet_funding::Config>::MaxBidsPerProject::get();
	let max_evaluations_per_project: u32 = <Runtime as pallet_funding::Config>::MaxEvaluationsPerProject::get();

	let create_project = SubstrateWeight::<Runtime>::create_project();
	dbg!(create_project);

	let remove_project = SubstrateWeight::<Runtime>::remove_project();
	dbg!(remove_project);

	let edit_project = SubstrateWeight::<Runtime>::edit_project();
	dbg!(edit_project);

	let start_evaluation = SubstrateWeight::<Runtime>::start_evaluation(1);
	dbg!(start_evaluation);

	let start_auction_manually =
		SubstrateWeight::<Runtime>::start_auction_manually(1);
	dbg!(start_auction_manually);

	let evaluation = SubstrateWeight::<Runtime>::evaluation(max_evaluations_per_user - 1);
	dbg!(evaluation);

	let bid = SubstrateWeight::<Runtime>::bid(max_bids_per_user, 10);
	dbg!(bid);

	let contribution = SubstrateWeight::<Runtime>::contribution(max_contributions_per_user);
	dbg!(contribution);

	let contribution_ends_round = SubstrateWeight::<Runtime>::contribution_ends_round(
		max_contributions_per_user - 1,
		1,
	);
	dbg!(contribution_ends_round);

	let decide_project_outcome =
		SubstrateWeight::<Runtime>::decide_project_outcome(1);
	dbg!(decide_project_outcome);

	let settle_successful_evaluation = SubstrateWeight::<Runtime>::settle_successful_evaluation();
	dbg!(settle_successful_evaluation);

	let settle_failed_evaluation = SubstrateWeight::<Runtime>::settle_failed_evaluation();
	dbg!(settle_failed_evaluation);

	let settle_successful_bid = SubstrateWeight::<Runtime>::settle_successful_bid();
	dbg!(settle_successful_bid);

	let settle_failed_bid = SubstrateWeight::<Runtime>::settle_failed_bid();
	dbg!(settle_failed_bid);

	let settle_successful_contribution = SubstrateWeight::<Runtime>::settle_successful_contribution();
	dbg!(settle_successful_contribution);

	let settle_failed_contribution = SubstrateWeight::<Runtime>::settle_failed_contribution();
	dbg!(settle_failed_contribution);

	let end_evaluation_success =
		SubstrateWeight::<Runtime>::end_evaluation_success(1);
	dbg!(end_evaluation_success);

	let end_evaluation_failure =
		SubstrateWeight::<Runtime>::end_evaluation_failure(1);
	dbg!(end_evaluation_failure);

	let start_auction_closing_phase =
		SubstrateWeight::<Runtime>::start_auction_closing_phase(1);
	dbg!(start_auction_closing_phase);

	let end_auction_closing = SubstrateWeight::<Runtime>::end_auction_closing(
		1,
		max_bids_per_project,
		0,
	);
	dbg!(end_auction_closing);

	let start_community_funding = SubstrateWeight::<Runtime>::start_community_funding(
		1,
		max_bids_per_project,
		0,
	);
	dbg!(start_community_funding);

	let start_remainder_funding =
		SubstrateWeight::<Runtime>::start_remainder_funding(1);
	dbg!(start_remainder_funding);

	let end_funding_automatically_rejected_evaluators_slashed =
		SubstrateWeight::<Runtime>::end_funding_automatically_rejected_evaluators_slashed(
			1,
		);
	dbg!(end_funding_automatically_rejected_evaluators_slashed);

	let end_funding_awaiting_decision_evaluators_slashed =
		SubstrateWeight::<Runtime>::end_funding_awaiting_decision_evaluators_slashed(
			1,
		);
	dbg!(end_funding_awaiting_decision_evaluators_slashed);

	let end_funding_awaiting_decision_evaluators_unchanged =
		SubstrateWeight::<Runtime>::end_funding_awaiting_decision_evaluators_unchanged(
			1,
		);
	dbg!(end_funding_awaiting_decision_evaluators_unchanged);

	let end_funding_automatically_accepted_evaluators_rewarded =
		SubstrateWeight::<Runtime>::end_funding_automatically_accepted_evaluators_rewarded(
			1,
			max_evaluations_per_project,
		);
	dbg!(end_funding_automatically_accepted_evaluators_rewarded);

	let project_decision = SubstrateWeight::<Runtime>::project_decision();
	dbg!(project_decision);

	let start_settlement_funding_success = SubstrateWeight::<Runtime>::start_settlement_funding_success();
	dbg!(start_settlement_funding_success);

	let start_settlement_funding_failure = SubstrateWeight::<Runtime>::start_settlement_funding_failure();
	dbg!(start_settlement_funding_failure);

	let total_blockspace = <Runtime as frame_system::Config>::BlockWeights::get().max_block;
	dbg!(total_blockspace);
}
