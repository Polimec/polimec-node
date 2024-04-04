use super::*;

#[test]
fn automatic_fail_less_eq_33_percent() {
	for funding_percent in (1..=33).step_by(5) {
		let _ = create_project_with_funding_percentage(funding_percent, None);
	}
}

#[test]
fn automatic_success_bigger_eq_90_percent() {
	for funding_percent in (90..=100).step_by(2) {
		let _ = create_project_with_funding_percentage(funding_percent, None);
	}
}

#[test]
fn manual_acceptance_percentage_between_34_89() {
	for funding_percent in (34..=89).step_by(5) {
		let _ = create_project_with_funding_percentage(funding_percent, Some(FundingOutcomeDecision::AcceptFunding));
	}
}

#[test]
fn manual_rejection_percentage_between_34_89() {
	for funding_percent in (34..=89).step_by(5) {
		let _ = create_project_with_funding_percentage(funding_percent, Some(FundingOutcomeDecision::RejectFunding));
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
	let bids = MockInstantiator::generate_bids_from_total_usd(
		Percent::from_percent(50u8) * twenty_percent_funding_usd,
		min_price,
		default_weights(),
		default_bidders(),
		default_multipliers(),
	);
	let contributions = MockInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(50u8) * twenty_percent_funding_usd,
		min_price,
		default_weights(),
		default_community_contributors(),
		default_multipliers(),
	);
	let project_id = inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

	let project_id = project_id;
	inst.advance_time(1u64 + <TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.test_ct_created_for(project_id);

	inst.settle_project(project_id).unwrap();
}
