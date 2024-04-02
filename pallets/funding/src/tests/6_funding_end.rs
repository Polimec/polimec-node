use super::*;

#[test]
fn failed_auction_is_settled() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(0, ISSUER_1);
	let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, default_evaluations());
	inst.start_community_funding(project_id).unwrap_err();
	// execute `do_end_funding`
	inst.advance_time(1).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
	// execute `do_start_settlement`
	inst.advance_time(1).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Initialized(PhantomData)));
	// run settlement machine a.k.a cleaner
	inst.advance_time(1).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));
}

#[test]
fn automatic_fail_less_eq_33_percent() {
	for funding_percent in (1..=33).step_by(5) {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			vec![100u8],
			vec![BIDDER_1],
			vec![10u8],
		);
		let contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_community_contributors(),
			default_multipliers(),
		);
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
	}
}

#[test]
fn automatic_success_bigger_eq_90_percent() {
	for funding_percent in (90..=100).step_by(2) {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
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
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
	}
}

#[test]
fn manual_outcome_above33_to_below90() {
	for funding_percent in (34..90).step_by(5) {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
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
		let project_id =
			inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);
		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);
	}
}

#[test]
fn manual_acceptance() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
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
	inst.execute(|| {
		PolimecFunding::do_decide_project_outcome(ISSUER_1, project_id, FundingOutcomeDecision::AcceptFunding)
	})
	.unwrap();

	inst.advance_time(1u64).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	assert_matches!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(_)));
	inst.test_ct_created_for(project_id);

	inst.advance_time(10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Success(CleanerState::Finished(PhantomData))
	);
}

#[test]
fn manual_rejection() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
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
	inst.execute(|| {
		PolimecFunding::do_decide_project_outcome(ISSUER_1, project_id, FundingOutcomeDecision::RejectFunding)
	})
	.unwrap();

	inst.advance_time(1u64).unwrap();

	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Failure(CleanerState::Initialized(PhantomData))
	);

	inst.test_ct_not_created_for(project_id);

	inst.advance_time(10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Failure(CleanerState::Finished(PhantomData))
	);
}

#[test]
fn automatic_acceptance_on_manual_decision_after_time_delta() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(inst.get_new_nonce(), ISSUER_1);
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

	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Success(CleanerState::Initialized(PhantomData))
	);
	inst.test_ct_created_for(project_id);

	inst.advance_time(10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Success(CleanerState::Finished(PhantomData))
	);
}

#[test]
fn evaluators_get_slashed_funding_accepted() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = project_from_funding_reached(&mut inst, 43u64);
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

	let old_evaluation_locked_plmc = inst
		.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
		.into_iter()
		.filter(|item| item.plmc_amount > Zero::zero())
		.collect::<Vec<UserToPLMCBalance<_>>>();

	let evaluators = old_evaluation_locked_plmc.accounts();

	let old_participation_locked_plmc =
		inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
	let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

	call_and_is_ok!(
		inst,
		PolimecFunding::do_decide_project_outcome(ISSUER_1, project_id, FundingOutcomeDecision::AcceptFunding)
	);
	inst.advance_time(1u64).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Success(CleanerState::Finished(PhantomData))
	);

	let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
	let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
		vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
		MergeOperation::Add,
	);

	let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

	assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
}

#[test]
fn evaluators_get_slashed_funding_funding_rejected() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = project_from_funding_reached(&mut inst, 56u64);
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

	let old_evaluation_locked_plmc = inst
		.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
		.into_iter()
		.filter(|item| item.plmc_amount > Zero::zero())
		.collect::<Vec<UserToPLMCBalance<_>>>();

	let evaluators = old_evaluation_locked_plmc.accounts();

	let old_participation_locked_plmc =
		inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
	let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

	call_and_is_ok!(
		inst,
		PolimecFunding::do_decide_project_outcome(ISSUER_1, project_id, FundingOutcomeDecision::RejectFunding)
	);
	inst.advance_time(1u64).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Failure(CleanerState::Finished(PhantomData))
	);

	let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
	let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
		vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
		MergeOperation::Add,
	);
	let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

	assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
}

#[test]
fn evaluators_get_slashed_funding_failed() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = project_from_funding_reached(&mut inst, 24u64);
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

	let old_evaluation_locked_plmc = inst
		.get_all_reserved_plmc_balances(HoldReason::Evaluation(project_id).into())
		.into_iter()
		.filter(|item| item.plmc_amount > Zero::zero())
		.collect::<Vec<_>>();

	let evaluators = old_evaluation_locked_plmc.accounts();

	let old_participation_locked_plmc =
		inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Participation(project_id).into());
	let old_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 10u64).unwrap();
	assert_matches!(
		inst.get_project_details(project_id).cleanup,
		Cleaner::Failure(CleanerState::Finished(PhantomData))
	);

	let slashed_evaluation_locked_plmc = MockInstantiator::slash_evaluator_balances(old_evaluation_locked_plmc);
	let expected_evaluator_free_balances = MockInstantiator::generic_map_operation(
		vec![slashed_evaluation_locked_plmc, old_participation_locked_plmc, old_free_plmc],
		MergeOperation::Add,
	);

	let actual_evaluator_free_balances = inst.get_free_plmc_balances_for(evaluators.clone());

	assert_eq!(actual_evaluator_free_balances, expected_evaluator_free_balances);
}

#[test]
fn ct_minted_automatically() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::NotReady);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.advance_time(10u64).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	let evaluators = evaluations.accounts();
	let evaluator_ct_amounts = evaluators
		.iter()
		.map(|account| {
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			let total_evaluator_ct_rewarded =
				evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

			(account, total_evaluator_ct_rewarded)
		})
		.collect_vec();

	let bidders = bids.accounts();
	let bidder_ct_amounts = bidders
		.iter()
		.map(|account| {
			let bids = inst
				.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>());
			let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

			(account, total_bidder_ct_rewarded)
		})
		.collect_vec();

	let community_accounts = community_contributions.accounts();
	let remainder_accounts = remainder_contributions.accounts();
	let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
	let contributor_ct_amounts = all_contributors
		.map(|account| {
			let contributions = inst.execute(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			let total_contributor_ct_rewarded =
				contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

			(account, total_contributor_ct_rewarded)
		})
		.collect_vec();

	let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
		vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
		|item| item.0,
		Zero::zero(),
		|item, accumulator| accumulator + item.1,
	);

	for (account, amount) in all_ct_expectations {
		let minted = inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
		assert_eq!(minted, amount);
	}
}

#[test]
fn ct_minted_manually() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::NotReady);
	// do_end_funding
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

	let evaluators = evaluations.accounts();
	let evaluator_ct_amounts = evaluators
		.iter()
		.map(|account| {
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for evaluation in evaluations.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::evaluation_reward_payout_for(
						RuntimeOrigin::signed(evaluation.evaluator),
						project_id,
						evaluation.evaluator,
						evaluation.id,
					));
				});
			}
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			let total_evaluator_ct_rewarded =
				evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

			(account, total_evaluator_ct_rewarded)
		})
		.collect_vec();

	let bidders = bids.accounts();
	let bidder_ct_amounts = bidders
		.iter()
		.map(|account| {
			let bids = inst
				.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>());
			for bid in bids.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					));
				});
			}

			let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

			(account, total_bidder_ct_rewarded)
		})
		.collect_vec();

	let community_accounts = community_contributions.accounts();
	let remainder_accounts = remainder_contributions.accounts();
	let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
	let contributor_ct_amounts = all_contributors
		.map(|account| {
			let contributions = inst.execute(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for contribution in contributions.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					));
				});
			}

			let total_contributor_ct_rewarded =
				contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

			(account, total_contributor_ct_rewarded)
		})
		.collect_vec();

	let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
		vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
		|item| item.0,
		Zero::zero(),
		|item, accumulator| accumulator + item.1,
	);

	for (account, amount) in all_ct_expectations {
		let minted = inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
		assert_eq!(minted, amount, "Account: {}", account);
	}

	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
}

#[test]
fn cannot_mint_ct_twice_manually() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::NotReady);
	// do_end_funding
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

	let evaluators = evaluations.accounts();
	let evaluator_ct_amounts = evaluators
		.iter()
		.map(|account| {
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for evaluation in evaluations.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::evaluation_reward_payout_for(
						RuntimeOrigin::signed(evaluation.evaluator),
						project_id,
						evaluation.evaluator,
						evaluation.id,
					));
					assert_noop!(
						Pallet::<TestRuntime>::evaluation_reward_payout_for(
							RuntimeOrigin::signed(evaluation.evaluator),
							project_id,
							evaluation.evaluator,
							evaluation.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			let total_evaluator_ct_rewarded =
				evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

			(account, total_evaluator_ct_rewarded)
		})
		.collect_vec();

	let bidders = bids.accounts();
	let bidder_ct_amounts = bidders
		.iter()
		.map(|account| {
			let bids = inst
				.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>());
			for bid in bids.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::bid_ct_mint_for(
						RuntimeOrigin::signed(bid.bidder),
						project_id,
						bid.bidder,
						bid.id,
					));
				});
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::bid_ct_mint_for(
							RuntimeOrigin::signed(bid.bidder),
							project_id,
							bid.bidder,
							bid.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}

			let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

			(account, total_bidder_ct_rewarded)
		})
		.collect_vec();

	let community_accounts = community_contributions.accounts();
	let remainder_accounts = remainder_contributions.accounts();
	let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
	let contributor_ct_amounts = all_contributors
		.map(|account| {
			let contributions = inst.execute(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for contribution in contributions.iter() {
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::contribution_ct_mint_for(
						RuntimeOrigin::signed(contribution.contributor),
						project_id,
						contribution.contributor,
						contribution.id,
					));
				});
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::contribution_ct_mint_for(
							RuntimeOrigin::signed(contribution.contributor),
							project_id,
							contribution.contributor,
							contribution.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}

			let total_contributor_ct_rewarded =
				contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

			(account, total_contributor_ct_rewarded)
		})
		.collect_vec();

	let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
		vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
		|item| item.0,
		Zero::zero(),
		|item, accumulator| accumulator + item.1,
	);

	for (account, amount) in all_ct_expectations {
		let minted = inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
		assert_eq!(minted, amount, "Account: {}", account);
	}

	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
}

#[test]
fn cannot_mint_ct_manually_after_automatic_mint() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let details = inst.get_project_details(project_id);
	assert_eq!(details.status, ProjectStatus::FundingSuccessful);
	assert_eq!(details.cleanup, Cleaner::NotReady);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	inst.advance_time(1).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	let evaluators = evaluations.accounts();
	let evaluator_ct_amounts = evaluators
		.iter()
		.map(|account| {
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for evaluation in evaluations.iter() {
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::evaluation_reward_payout_for(
							RuntimeOrigin::signed(evaluation.evaluator),
							project_id,
							evaluation.evaluator,
							evaluation.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}
			let evaluations = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			let total_evaluator_ct_rewarded =
				evaluations
					.iter()
					.map(|evaluation| evaluation.rewarded_or_slashed)
					.map(|reward_or_slash| {
						if let Some(RewardOrSlash::Reward(balance)) = reward_or_slash {
							balance
						} else {
							Zero::zero()
						}
					})
					.sum::<u128>();

			(account, total_evaluator_ct_rewarded)
		})
		.collect_vec();

	let bidders = bids.accounts();
	let bidder_ct_amounts = bidders
		.iter()
		.map(|account| {
			let bids = inst
				.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>());
			for bid in bids.iter() {
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::bid_ct_mint_for(
							RuntimeOrigin::signed(bid.bidder),
							project_id,
							bid.bidder,
							bid.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}

			let total_bidder_ct_rewarded = bids.iter().map(|bid| bid.final_ct_amount).sum::<u128>();

			(account, total_bidder_ct_rewarded)
		})
		.collect_vec();

	let community_accounts = community_contributions.accounts();
	let remainder_accounts = remainder_contributions.accounts();
	let all_contributors = community_accounts.iter().chain(remainder_accounts.iter()).unique();
	let contributor_ct_amounts = all_contributors
		.map(|account| {
			let contributions = inst.execute(|| {
				Contributions::<TestRuntime>::iter_prefix_values((project_id, account.clone())).collect::<Vec<_>>()
			});
			for contribution in contributions.iter() {
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::contribution_ct_mint_for(
							RuntimeOrigin::signed(contribution.contributor),
							project_id,
							contribution.contributor,
							contribution.id,
						),
						Error::<TestRuntime>::NotAllowed
					);
				});
			}

			let total_contributor_ct_rewarded =
				contributions.iter().map(|contribution| contribution.ct_amount).sum::<u128>();

			(account, total_contributor_ct_rewarded)
		})
		.collect_vec();

	let all_ct_expectations = MockInstantiator::generic_map_merge_reduce(
		vec![evaluator_ct_amounts, bidder_ct_amounts, contributor_ct_amounts],
		|item| item.0,
		Zero::zero(),
		|item, accumulator| accumulator + item.1,
	);

	for (account, amount) in all_ct_expectations {
		let minted = inst.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, account));
		assert_eq!(minted, amount, "Account: {}", account);
	}
}

#[test]
fn multiplier_gets_correct_vesting_duration() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = vec![
		BidParams::new(BIDDER_1, 325_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 75_000 * ASSET_UNIT, 2u8, AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_3, 50_000 * ASSET_UNIT, 3u8, AcceptedFundingAsset::USDT),
	];
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions,
		remainder_contributions,
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.advance_time(10u64).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	stored_bids.sort_by_key(|bid| bid.bidder);
	let one_week_in_blocks = DAYS * 7;
	assert_eq!(stored_bids[0].plmc_vesting_info.unwrap().duration, 1u64);
	assert_eq!(
		stored_bids[1].plmc_vesting_info.unwrap().duration,
		FixedU128::from_rational(2167, 1000).saturating_mul_int(one_week_in_blocks as u64)
	);
	assert_eq!(
		stored_bids[2].plmc_vesting_info.unwrap().duration,
		FixedU128::from_rational(4334, 1000).saturating_mul_int(one_week_in_blocks as u64)
	);
}

#[test]
fn funding_assets_are_paid_manually_to_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions,
		remainder_contributions,
	);

	let final_winning_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.collect::<Vec<_>>()
	});
	let final_bid_payouts = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.map(|bid| {
				UserToForeignAssets::new(
					bid.bidder,
					bid.funding_asset_amount_locked,
					bid.funding_asset.to_assethub_id(),
				)
			})
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});
	let final_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
	let final_contribution_payouts = inst.execute(|| {
		Contributions::<TestRuntime>::iter_prefix_values((project_id,))
			.map(|contribution| {
				UserToForeignAssets::new(
					contribution.contributor,
					contribution.funding_asset_amount,
					contribution.funding_asset.to_assethub_id(),
				)
			})
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});

	let total_expected_bid_payout =
		final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
	let total_expected_contribution_payout =
		final_contribution_payouts.iter().map(|contribution| contribution.asset_amount).sum::<BalanceOf<TestRuntime>>();

	let prev_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

	let prev_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		final_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	for bid in final_winning_bids {
		inst.execute(|| {
			Pallet::<TestRuntime>::payout_bid_funds_for(RuntimeOrigin::signed(issuer), project_id, bid.bidder, bid.id)
		})
		.unwrap();
	}
	for contribution in final_contributions {
		inst.execute(|| {
			Pallet::<TestRuntime>::payout_contribution_funds_for(
				RuntimeOrigin::signed(issuer),
				project_id,
				contribution.contributor,
				contribution.id,
			)
		})
		.unwrap();
	}
	let post_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

	let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		final_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;
	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
	let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

	assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
	assert_eq!(issuer_funding_delta, project_pot_funding_delta);

	assert_eq!(post_project_pot_funding_balance, 0u128);
}

#[test]
fn funding_assets_are_paid_automatically_to_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions,
		remainder_contributions,
	);

	let final_bid_payouts = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.map(|bid| {
				UserToForeignAssets::new(
					bid.bidder,
					bid.funding_asset_amount_locked,
					bid.funding_asset.to_assethub_id(),
				)
			})
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});
	let final_contribution_payouts = inst.execute(|| {
		Contributions::<TestRuntime>::iter_prefix_values((project_id,))
			.map(|contribution| {
				UserToForeignAssets::new(
					contribution.contributor,
					contribution.funding_asset_amount,
					contribution.funding_asset.to_assethub_id(),
				)
			})
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});

	let total_expected_bid_payout =
		final_bid_payouts.iter().map(|bid| bid.asset_amount).sum::<BalanceOf<TestRuntime>>();
	let total_expected_contribution_payout =
		final_contribution_payouts.iter().map(|contribution| contribution.asset_amount).sum::<BalanceOf<TestRuntime>>();

	let prev_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

	let prev_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		final_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	inst.advance_time(1u64).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	let post_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(final_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;

	let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		final_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;
	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
	let project_pot_funding_delta = prev_project_pot_funding_balance - post_project_pot_funding_balance;

	assert_eq!(issuer_funding_delta - total_expected_bid_payout, total_expected_contribution_payout);
	assert_eq!(issuer_funding_delta, project_pot_funding_delta);

	assert_eq!(post_project_pot_funding_balance, 0u128);
}

#[test]
fn funding_assets_are_released_automatically_on_funding_fail() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);

	let auction_allocation =
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
	let evaluations = default_evaluations();
	let bids = MockInstantiator::generate_bids_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(auction_allocation),
		project_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions.clone(),
		remainder_contributions.clone(),
	);

	let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let expected_bid_payouts = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.map(|bid| {
				UserToForeignAssets::<TestRuntime>::new(
					bid.bidder,
					bid.funding_asset_amount_locked,
					bid.funding_asset.to_assethub_id(),
				)
			})
			.sorted_by_key(|bid| bid.account)
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});
	let expected_community_contribution_payouts =
		MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions, final_price);
	let expected_remainder_contribution_payouts =
		MockInstantiator::calculate_contributed_funding_asset_spent(remainder_contributions, final_price);
	let all_expected_payouts = MockInstantiator::generic_map_operation(
		vec![
			expected_bid_payouts.clone(),
			expected_community_contribution_payouts,
			expected_remainder_contribution_payouts,
		],
		MergeOperation::Add,
	);

	let prev_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
	let all_participants = all_expected_payouts.accounts();
	let prev_participants_funding_balances =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

	call_and_is_ok!(
		inst,
		Pallet::<TestRuntime>::decide_project_outcome(
			RuntimeOrigin::signed(issuer),
			get_mock_jwt(issuer.clone(), InvestorType::Institutional, generate_did_from_account(issuer.clone())),
			project_id,
			FundingOutcomeDecision::RejectFunding
		)
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	inst.advance_time(10).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

	let post_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
	let post_participants_funding_balances =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
	let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		expected_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;

	let all_participants_funding_delta = MockInstantiator::generic_map_operation(
		vec![prev_participants_funding_balances, post_participants_funding_balances],
		MergeOperation::Add,
	);

	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

	assert_eq!(issuer_funding_delta, 0);
	assert_eq!(post_project_pot_funding_balance, 0u128);
	assert_eq!(all_expected_payouts, all_participants_funding_delta);
}

#[test]
fn funding_assets_are_released_manually_on_funding_fail() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let auction_allocation =
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
	let evaluations = default_evaluations();
	let bids = MockInstantiator::generate_bids_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(auction_allocation),
		project_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let expected_bid_payouts = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.map(|bid| {
				UserToForeignAssets::<TestRuntime>::new(
					bid.bidder,
					bid.funding_asset_amount_locked,
					bid.funding_asset.to_assethub_id(),
				)
			})
			.sorted_by_key(|item| item.account)
			.collect::<Vec<UserToForeignAssets<TestRuntime>>>()
	});
	let expected_community_contribution_payouts =
		MockInstantiator::calculate_contributed_funding_asset_spent(community_contributions, final_price);
	let expected_remainder_contribution_payouts =
		MockInstantiator::calculate_contributed_funding_asset_spent(remainder_contributions, final_price);
	let all_expected_payouts = MockInstantiator::generic_map_operation(
		vec![
			expected_bid_payouts.clone(),
			expected_community_contribution_payouts,
			expected_remainder_contribution_payouts,
		],
		MergeOperation::Add,
	);

	let prev_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
	let all_participants = all_expected_payouts.accounts();
	let prev_participants_funding_balances =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants.clone());

	call_and_is_ok!(
		inst,
		Pallet::<TestRuntime>::decide_project_outcome(
			RuntimeOrigin::signed(issuer),
			get_mock_jwt(issuer.clone(), InvestorType::Institutional, generate_did_from_account(issuer.clone())),
			project_id,
			FundingOutcomeDecision::RejectFunding
		)
	);

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	let stored_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.collect::<Vec<_>>()
	});
	let stored_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	for bid in stored_bids {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::release_bid_funds_for(RuntimeOrigin::signed(issuer), project_id, bid.bidder, bid.id,)
		)
	}

	for contribution in stored_contributions {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::release_contribution_funds_for(
				RuntimeOrigin::signed(issuer),
				project_id,
				contribution.contributor,
				contribution.id,
			)
		)
	}

	let post_issuer_funding_balance =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, vec![issuer])[0].asset_amount;
	let post_participants_funding_balances =
		inst.get_free_foreign_asset_balances_for(expected_bid_payouts[0].asset_id, all_participants);
	let post_project_pot_funding_balance = inst.get_free_foreign_asset_balances_for(
		expected_bid_payouts[0].asset_id,
		vec![Pallet::<TestRuntime>::fund_account_id(project_id)],
	)[0]
	.asset_amount;

	let all_participants_funding_delta = MockInstantiator::generic_map_operation(
		vec![prev_participants_funding_balances, post_participants_funding_balances],
		MergeOperation::Add,
	);

	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

	assert_eq!(issuer_funding_delta, 0);
	assert_eq!(post_project_pot_funding_balance, 0u128);
	assert_eq!(all_expected_payouts, all_participants_funding_delta);
}

#[test]
fn plmc_bonded_is_returned_automatically_on_funding_fail() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let auction_allocation =
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
	let evaluations = default_evaluations();
	let bids = MockInstantiator::generate_bids_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(auction_allocation),
		project_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

	let expected_evaluators_and_contributors_payouts =
		MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
			evaluations.clone(),
			remainder_contributions.clone(),
			final_price,
			true,
		);
	let expected_bid_payouts = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, final_price);
	let expected_community_contribution_payouts =
		MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
	let all_expected_payouts = MockInstantiator::generic_map_operation(
		vec![
			expected_evaluators_and_contributors_payouts.clone(),
			expected_bid_payouts,
			expected_community_contribution_payouts,
		],
		MergeOperation::Add,
	);
	println!("all expected payouts {:?}", all_expected_payouts);
	for payout in all_expected_payouts.clone() {
		let evaluation_hold = inst.execute(|| {
			<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
                    &HoldReason::Evaluation(project_id).into(),
                    &payout.account,
                )
		});
		let participation_hold = inst.execute(|| {
			<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
                    &HoldReason::Participation(project_id).into(),
                    &payout.account,
                )
		});
		println!("account {:?} has evaluation hold {:?}", payout.account, evaluation_hold);
		println!("account {:?} has participation hold {:?}", payout.account, participation_hold);
	}
	let deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
		ProjectId,
		AccountIdOf<TestRuntime>,
	>>::deposit_required(project_id);
	let all_expected_payouts = all_expected_payouts
		.into_iter()
		.map(|UserToPLMCBalance { account, plmc_amount }| {
			UserToPLMCBalance::new(account, plmc_amount + deposit_required)
		})
		.collect::<Vec<_>>();

	let prev_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;

	let all_participants = all_expected_payouts.accounts();
	let prev_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants.clone());

	call_and_is_ok!(
		inst,
		Pallet::<TestRuntime>::decide_project_outcome(
			RuntimeOrigin::signed(issuer),
			get_mock_jwt(issuer.clone(), InvestorType::Institutional, generate_did_from_account(issuer.clone())),
			project_id,
			FundingOutcomeDecision::RejectFunding
		)
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	inst.advance_time(10).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Finished(PhantomData)));

	let post_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
	let post_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants);

	let all_participants_plmc_deltas = MockInstantiator::generic_map_operation(
		vec![post_participants_plmc_balances, prev_participants_plmc_balances],
		MergeOperation::Subtract,
	);

	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;

	assert_eq!(issuer_funding_delta, 0);
	assert_eq!(all_participants_plmc_deltas, all_expected_payouts);
}

#[test]
fn plmc_bonded_is_returned_manually_on_funding_fail() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let auction_allocation =
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
	let evaluations = default_evaluations();
	let bids = MockInstantiator::generate_bids_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(auction_allocation),
		project_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
		project_metadata.minimum_price.saturating_mul_int(
			Percent::from_percent(50u8) * (project_metadata.total_allocation_size - auction_allocation) / 2,
		),
		project_metadata.minimum_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);
	let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

	let expected_evaluators_and_contributors_payouts =
		MockInstantiator::calculate_total_plmc_locked_from_evaluations_and_remainder_contributions(
			evaluations.clone(),
			remainder_contributions.clone(),
			final_price,
			true,
		);
	let expected_bid_payouts = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, final_price);
	let expected_community_contribution_payouts =
		MockInstantiator::calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
	let all_expected_payouts = MockInstantiator::generic_map_operation(
		vec![
			expected_evaluators_and_contributors_payouts.clone(),
			expected_bid_payouts,
			expected_community_contribution_payouts,
		],
		MergeOperation::Add,
	);
	println!("all expected payouts {:?}", all_expected_payouts);
	for payout in all_expected_payouts.clone() {
		let evaluation_hold = inst.execute(|| {
			<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
                    &HoldReason::Evaluation(project_id).into(),
                    &payout.account,
                )
		});
		let participation_hold = inst.execute(|| {
			<<TestRuntime as Config>::NativeCurrency as fungible::InspectHold<AccountIdOf<TestRuntime>>>::balance_on_hold(
                    &HoldReason::Participation(project_id).into(),
                    &payout.account,
                )
		});
		println!("account {:?} has evaluation hold {:?}", payout.account, evaluation_hold);
		println!("account {:?} has participation hold {:?}", payout.account, participation_hold);
	}
	let _deposit_required = <<TestRuntime as Config>::ContributionTokenCurrency as AccountTouch<
		ProjectId,
		AccountIdOf<TestRuntime>,
	>>::deposit_required(project_id);

	let prev_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
	let all_participants = all_expected_payouts.accounts();
	let prev_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants.clone());

	call_and_is_ok!(
		inst,
		Pallet::<TestRuntime>::decide_project_outcome(
			RuntimeOrigin::signed(issuer),
			get_mock_jwt(issuer.clone(), InvestorType::Institutional, generate_did_from_account(issuer.clone())),
			project_id,
			FundingOutcomeDecision::RejectFunding
		)
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Initialized(PhantomData)));

	let stored_evaluations =
		inst.execute(|| Evaluations::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());
	let stored_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.collect::<Vec<_>>()
	});
	let stored_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	for evaluation in stored_evaluations {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::evaluation_slash_for(
				RuntimeOrigin::signed(evaluation.evaluator),
				project_id,
				evaluation.evaluator,
				evaluation.id,
			),
			Pallet::<TestRuntime>::evaluation_unbond_for(
				RuntimeOrigin::signed(evaluation.evaluator),
				project_id,
				evaluation.evaluator,
				evaluation.id,
			)
		)
	}

	for bid in stored_bids {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::release_bid_funds_for(RuntimeOrigin::signed(issuer), project_id, bid.bidder, bid.id,),
			Pallet::<TestRuntime>::bid_unbond_for(RuntimeOrigin::signed(bid.bidder), project_id, bid.bidder, bid.id,)
		)
	}

	for contribution in stored_contributions {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::release_contribution_funds_for(
				RuntimeOrigin::signed(issuer),
				project_id,
				contribution.contributor,
				contribution.id,
			),
			Pallet::<TestRuntime>::contribution_unbond_for(
				RuntimeOrigin::signed(contribution.contributor),
				project_id,
				contribution.contributor,
				contribution.id,
			)
		)
	}

	let post_issuer_funding_balance = inst.get_free_plmc_balances_for(vec![issuer])[0].plmc_amount;
	let post_participants_plmc_balances = inst.get_free_plmc_balances_for(all_participants);

	let all_participants_plmc_deltas = MockInstantiator::generic_map_operation(
		vec![post_participants_plmc_balances, prev_participants_plmc_balances],
		MergeOperation::Subtract,
	);

	let issuer_funding_delta = post_issuer_funding_balance - prev_issuer_funding_balance;
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Failure(CleanerState::Initialized(PhantomData)));
	assert_eq!(issuer_funding_delta, 0);
	assert_eq!(all_participants_plmc_deltas, all_expected_payouts);
}

// i.e consumer increase bug fixed with touch on pallet-assets
#[test]
fn no_limit_on_project_contributions_per_user() {
	let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let project = |x| TestProjectParams {
		expected_state: ProjectStatus::FundingSuccessful,
		metadata: default_project_metadata(x, ISSUER_1),
		evaluations: default_evaluations(),
		bids: default_bids(),
		community_contributions: default_community_buys(),
		remainder_contributions: default_remainder_buys(),
		issuer: x as u32,
	};
	let projects = (0..20).into_iter().map(|x| project(x)).collect_vec();
	async_features::create_multiple_projects_at(inst, projects);
}

#[test]
fn evaluation_plmc_unbonded_after_funding_success() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let evaluations = default_evaluations();
	let evaluators = evaluations.accounts();

	let project_id = inst.create_remainder_contributing_project(
		default_project_metadata(inst.get_new_nonce(), ISSUER_1),
		ISSUER_1,
		evaluations.clone(),
		default_bids(),
		default_community_buys(),
	);

	let prev_reserved_plmc =
		inst.get_reserved_plmc_balances_for(evaluators.clone(), HoldReason::Evaluation(project_id).into());

	let prev_free_plmc = inst.get_free_plmc_balances_for(evaluators.clone());

	inst.finish_funding(project_id).unwrap();
	inst.advance_time(<TestRuntime as Config>::ManualAcceptanceDuration::get() + 1).unwrap();
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
	assert_eq!(inst.get_project_details(project_id).cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
	inst.advance_time(10).unwrap();
	let post_unbond_amounts: Vec<UserToPLMCBalance<_>> = prev_reserved_plmc
		.iter()
		.map(|UserToPLMCBalance { account, .. }| UserToPLMCBalance::new(*account, Zero::zero()))
		.collect();

	inst.do_reserved_plmc_assertions(post_unbond_amounts.clone(), HoldReason::Evaluation(project_id).into());
	inst.do_reserved_plmc_assertions(post_unbond_amounts, HoldReason::Participation(project_id).into());

	let post_free_plmc = inst.get_free_plmc_balances_for(evaluators);

	let increased_amounts =
		MockInstantiator::generic_map_operation(vec![post_free_plmc, prev_free_plmc], MergeOperation::Subtract);

	assert_eq!(increased_amounts, MockInstantiator::calculate_evaluation_plmc_spent(evaluations))
}

#[test]
fn plmc_vesting_schedule_starts_automatically() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let mut bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);

	let price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
	bids = stored_bids.into_iter().map(|bid| BidParams::new_with_defaults(bid.bidder, bid.final_ct_amount)).collect();
	let auction_locked_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, price);
	let community_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(community_contributions, price);
	let remainder_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(remainder_contributions, price);
	let all_plmc_locks = MockInstantiator::generic_map_operation(
		vec![auction_locked_plmc, community_locked_plmc, remainder_locked_plmc],
		MergeOperation::Add,
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.advance_time(10u64).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	for UserToPLMCBalance { account, plmc_amount } in all_plmc_locks {
		let schedule = inst.execute(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&account,
				HoldReason::Participation(project_id).into(),
			)
		});

		assert_eq!(schedule.unwrap(), plmc_amount);
	}
}

#[test]
fn plmc_vesting_schedule_starts_manually() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids.clone(),
		community_contributions.clone(),
		remainder_contributions.clone(),
	);

	let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));

	let stored_successful_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
			.collect::<Vec<_>>()
	});
	let stored_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	for bid in stored_successful_bids.clone() {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::do_start_bid_vesting_schedule_for(&bid.bidder, project_id, &bid.bidder, bid.id,)
		);
	}
	for contribution in stored_contributions {
		call_and_is_ok!(
			inst,
			Pallet::<TestRuntime>::do_start_contribution_vesting_schedule_for(
				&contribution.contributor,
				project_id,
				&contribution.contributor,
				contribution.id,
			)
		);
	}

	let auction_locked_plmc = MockInstantiator::calculate_auction_plmc_charged_with_given_price(&bids, price);
	let community_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(community_contributions, price);
	let remainder_locked_plmc = MockInstantiator::calculate_contributed_plmc_spent(remainder_contributions, price);
	let all_plmc_locks = MockInstantiator::generic_map_operation(
		vec![auction_locked_plmc, community_locked_plmc, remainder_locked_plmc],
		MergeOperation::Add,
	);

	for UserToPLMCBalance { account, plmc_amount } in all_plmc_locks {
		let schedule = inst.execute(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&account,
				HoldReason::Participation(project_id).into(),
			)
		});

		assert_eq!(schedule.unwrap(), plmc_amount);
	}
}

#[test]
fn plmc_vesting_full_amount() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions,
		remainder_contributions,
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.advance_time(10u64).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));

	let stored_successful_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
			.collect::<Vec<_>>()
	});

	let stored_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	let total_bid_plmc_in_vesting: Vec<UserToPLMCBalance<TestRuntime>> = stored_successful_bids
		.iter()
		.map(|bid| (bid.bidder, bid.plmc_vesting_info.unwrap().total_amount).into())
		.collect_vec();

	let total_contribution_plmc_in_vesting: Vec<UserToPLMCBalance<TestRuntime>> = stored_contributions
		.iter()
		.map(|contribution| (contribution.contributor, contribution.plmc_vesting_info.unwrap().total_amount).into())
		.collect_vec();

	let total_participant_plmc_in_vesting = MockInstantiator::generic_map_operation(
		vec![total_bid_plmc_in_vesting, total_contribution_plmc_in_vesting],
		MergeOperation::Add,
	);

	inst.advance_time((10 * DAYS).into()).unwrap();

	for UserToPLMCBalance { account, plmc_amount } in total_participant_plmc_in_vesting {
		let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&account));

		inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(account, project_id, account)).unwrap();

		let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&account));
		assert_eq!(plmc_amount, post_free_balance - prev_free_balance);
	}
}

#[test]
fn plmc_vesting_partial_amount() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(inst.get_new_nonce(), issuer);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_contributions = default_community_buys();
	let remainder_contributions = default_remainder_buys();

	let project_id = inst.create_finished_project(
		project_metadata,
		issuer,
		evaluations,
		bids,
		community_contributions,
		remainder_contributions,
	);

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	inst.advance_time(15u64).unwrap();
	let details = inst.get_project_details(project_id);
	assert_eq!(details.cleanup, Cleaner::Success(CleanerState::Finished(PhantomData)));
	let vest_start_block = details.funding_end_block.unwrap();
	let stored_successful_bids = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,))
			.filter(|bid| matches!(bid.status, BidStatus::Rejected(_)).not())
			.collect::<Vec<_>>()
	});
	let stored_contributions =
		inst.execute(|| Contributions::<TestRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>());

	let bidder_vesting =
		stored_successful_bids.iter().map(|bid| (bid.bidder, bid.plmc_vesting_info.unwrap())).collect_vec();
	let contributor_vesting = stored_contributions
		.iter()
		.map(|contribution| (contribution.contributor, contribution.plmc_vesting_info.unwrap()))
		.collect_vec();

	let participant_vesting_infos: Vec<(AccountIdOf<TestRuntime>, Vec<VestingInfoOf<TestRuntime>>)> =
		MockInstantiator::generic_map_merge_reduce(
			vec![bidder_vesting, contributor_vesting],
			|map| map.0,
			Vec::new(),
			|map, mut vestings| {
				vestings.push(map.1);
				vestings
			},
		);

	let now = inst.current_block();
	for (participant, vesting_infos) in participant_vesting_infos {
		let vested_amount = vesting_infos.into_iter().fold(0u128, |acc, vesting_info| {
			acc + vesting_info.amount_per_block * min(vesting_info.duration, now - vest_start_block) as u128
		});

		let prev_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&participant));

		inst.execute(|| Pallet::<TestRuntime>::do_vest_plmc_for(participant, project_id, participant)).unwrap();

		let post_free_balance = inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&participant));
		assert_eq!(vested_amount, post_free_balance - prev_free_balance);
	}
}

#[test]
fn ct_treasury_mints() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let treasury_account = <TestRuntime as Config>::ContributionTreasury::get();

	let project_metadata = ProjectMetadataOf::<TestRuntime> {
		token_information: default_token_information(),
		mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
		total_allocation_size: 1_000_000 * ASSET_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
			institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(None, None),
			professional: TicketSize::new(None, None),
			institutional: TicketSize::new(None, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: ISSUER_1,
		offchain_information_hash: Some(hashed(METADATA)),
	};
	let mut counter: u8 = 0u8;
	let mut with_different_metadata = |mut project: ProjectMetadataOf<TestRuntime>| {
		let mut binding = project.offchain_information_hash.unwrap();
		let h256_bytes = binding.as_fixed_bytes_mut();
		h256_bytes[0] = counter;
		counter += 1u8;
		project.offchain_information_hash = Some(binding);
		project
	};

	let price = project_metadata.minimum_price;

	// Failed project has no mints on the treasury
	let project_20_percent = inst.create_finished_project(
		with_different_metadata(project_metadata.clone()),
		ISSUER_1,
		default_evaluations(),
		default_bids_from_ct_percent(10),
		default_community_contributions_from_ct_percent(10),
		vec![],
	);
	inst.advance_time(<TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	let ct_balance = inst
		.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_20_percent, treasury_account));
	assert_eq!(ct_balance, 0);

	// 50% funded project can have mints on the treasury if issuer accepts or enough time passes for automatic acceptance
	let fee_10_percent = Percent::from_percent(10) * 1_000_000 * US_DOLLAR;
	let fee_8_percent = Percent::from_percent(8) * 4_000_000 * US_DOLLAR;
	let fee_6_percent = Percent::from_percent(6) * 0 * US_DOLLAR;
	let total_usd_fee = fee_10_percent + fee_8_percent + fee_6_percent;
	let total_ct_fee = price.reciprocal().unwrap().saturating_mul_int(total_usd_fee);

	let project_50_percent = inst.create_finished_project(
		with_different_metadata(project_metadata.clone()),
		ISSUER_1,
		default_evaluations(),
		default_bids_from_ct_percent(25),
		default_community_contributions_from_ct_percent(20),
		default_remainder_contributions_from_ct_percent(5),
	);
	inst.advance_time(<TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	let ct_balance = inst
		.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_50_percent, treasury_account));
	let expected_liquidity_pool_minted = Percent::from_percent(50) * total_ct_fee;
	let expected_long_term_holder_bonus_minted = Percent::from_percent(50) * total_ct_fee;
	assert_eq!(ct_balance, expected_liquidity_pool_minted + expected_long_term_holder_bonus_minted);

	// 80% funded project can have mints on the treasury if issuer accepts or enough time passes for automatic acceptance
	let fee_10_percent = Percent::from_percent(10) * 1_000_000 * US_DOLLAR;
	let fee_8_percent = Percent::from_percent(8) * 5_000_000 * US_DOLLAR;
	let fee_6_percent = Percent::from_percent(6) * 2_000_000 * US_DOLLAR;
	let total_usd_fee = fee_10_percent + fee_8_percent + fee_6_percent;
	let total_ct_fee = price.reciprocal().unwrap().saturating_mul_int(total_usd_fee);

	let project_80_percent = inst.create_finished_project(
		with_different_metadata(project_metadata.clone()),
		ISSUER_1,
		default_evaluations(),
		default_bids_from_ct_percent(40),
		default_community_contributions_from_ct_percent(30),
		default_remainder_contributions_from_ct_percent(10),
	);
	inst.advance_time(<TestRuntime as Config>::ManualAcceptanceDuration::get()).unwrap();
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	let ct_balance = inst
		.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_80_percent, treasury_account));
	let expected_liquidity_pool_minted = Percent::from_percent(50) * total_ct_fee;
	let expected_long_term_holder_bonus_minted = Percent::from_percent(50) * total_ct_fee;
	assert_eq!(ct_balance, expected_liquidity_pool_minted + expected_long_term_holder_bonus_minted);

	// 98% funded project always has mints on the treasury
	let fee_10_percent = Percent::from_percent(10) * 1_000_000 * US_DOLLAR;
	let fee_8_percent = Percent::from_percent(8) * 5_000_000 * US_DOLLAR;
	let fee_6_percent = Percent::from_percent(6) * 3_800_000 * US_DOLLAR;
	let total_usd_fee = fee_10_percent + fee_8_percent + fee_6_percent;
	let total_ct_fee = price.reciprocal().unwrap().saturating_mul_int(total_usd_fee);

	let project_98_percent = inst.create_finished_project(
		with_different_metadata(project_metadata.clone()),
		ISSUER_1,
		default_evaluations(),
		default_bids_from_ct_percent(49),
		default_community_contributions_from_ct_percent(39),
		default_remainder_contributions_from_ct_percent(10),
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	let ct_balance = inst
		.execute(|| <TestRuntime as Config>::ContributionTokenCurrency::balance(project_98_percent, treasury_account));
	let expected_liquidity_pool_minted = Percent::from_percent(50) * total_ct_fee;
	let lthb_percent = Perquintill::from_percent(20) + Perquintill::from_percent(30) * Perquintill::from_percent(2);
	let expected_long_term_holder_bonus_minted = lthb_percent * total_ct_fee;
	assert_eq!(ct_balance, expected_liquidity_pool_minted + expected_long_term_holder_bonus_minted);

	// Test the touch on the treasury ct account by the issuer.
	// We create more CT accounts that the account can use provider references for,
	// so if it succeeds, then it means the touch was successful.
	let consumer_limit: u32 = <TestRuntime as frame_system::Config>::MaxConsumers::get();

	// we want to test ct mints on treasury of 1 over the consumer limit,
	// and we already minted 3 contribution tokens on previous tests.
	for i in 0..consumer_limit + 1u32 - 3u32 {
		let _ = inst.create_finished_project(
			with_different_metadata(project_metadata.clone()),
			ISSUER_1 + i + 1000,
			default_evaluations(),
			default_bids_from_ct_percent(49),
			default_community_contributions_from_ct_percent(39),
			default_remainder_contributions_from_ct_percent(10),
		);
		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 1).unwrap();
	}
}

#[test]
fn evaluation_rewards_are_paid_full_funding() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let project_metadata = knowledge_hub_project(0);
	let evaluations = knowledge_hub_evaluations();
	let bids = knowledge_hub_bids();
	let contributions = knowledge_hub_buys();

	let project_id = inst.create_finished_project(project_metadata, ISSUER_1, evaluations, bids, contributions, vec![]);

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	inst.advance_time(10).unwrap();

	let actual_reward_balances = inst.execute(|| {
		vec![
			(EVALUATOR_1, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_1)),
			(EVALUATOR_2, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_2)),
			(EVALUATOR_3, <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, EVALUATOR_3)),
		]
	});
	let expected_ct_rewards =
		vec![(EVALUATOR_1, 1_332_4_500_000_000), (EVALUATOR_2, 917_9_100_000_000), (EVALUATOR_3, 710_6_400_000_000)];

	for (real, desired) in zip(actual_reward_balances.iter(), expected_ct_rewards.iter()) {
		assert_close_enough!(real.1, desired.1, Perquintill::from_float(0.99));
	}
}
