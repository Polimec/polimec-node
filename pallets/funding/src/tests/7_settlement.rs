use super::*;

#[test]
fn can_settle_accepted_project() {
	let percentage = 100u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let evaluations = inst.get_evaluations(project_id);
	let bids = inst.get_bids(project_id);
	let contributions = inst.get_contributions(project_id);

	inst.settle_project(project_id).unwrap();

	inst.assert_evaluations_migrations_created(project_id, evaluations, percentage);
	inst.assert_bids_migrations_created(project_id, bids, true);
	inst.assert_contributions_migrations_created(project_id, contributions, true);
}

#[test]
fn can_settle_failed_project() {
	let percentage = 33u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let evaluations = inst.get_evaluations(project_id);
	let bids = inst.get_bids(project_id);
	let contributions = inst.get_contributions(project_id);

	inst.settle_project(project_id).unwrap();

	inst.assert_evaluations_migrations_created(project_id, evaluations, percentage);
	inst.assert_bids_migrations_created(project_id, bids, false);
	inst.assert_contributions_migrations_created(project_id, contributions, false);
}

#[test]
fn cannot_settle_successful_project_twice() {
	let percentage = 100u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);

	let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
	let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
	let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();

	inst.execute(|| {
		let evaluator = first_evaluation.evaluator;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_evaluation(
			RuntimeOrigin::signed(evaluator),
			project_id,
			evaluator,
			first_evaluation.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_successful_evaluation(
				RuntimeOrigin::signed(evaluator),
				project_id,
				evaluator,
				first_evaluation.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);

		let bidder = first_bid.bidder;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_bid(
			RuntimeOrigin::signed(bidder),
			project_id,
			bidder,
			first_bid.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_successful_bid(
				RuntimeOrigin::signed(bidder),
				project_id,
				bidder,
				first_bid.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);

		let contributor = first_contribution.contributor;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_contribution(
			RuntimeOrigin::signed(contributor),
			project_id,
			contributor,
			first_contribution.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_successful_contribution(
				RuntimeOrigin::signed(contributor),
				project_id,
				contributor,
				first_contribution.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);
	});
}

#[test]
fn cannot_settle_failed_project_twice() {
	let percentage = 33u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);

	let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
	let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
	let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();

	inst.execute(|| {
		let evaluator = first_evaluation.evaluator;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_evaluation(
			RuntimeOrigin::signed(evaluator),
			project_id,
			evaluator,
			first_evaluation.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_failed_evaluation(
				RuntimeOrigin::signed(evaluator),
				project_id,
				evaluator,
				first_evaluation.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);

		let bidder = first_bid.bidder;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_bid(
			RuntimeOrigin::signed(bidder),
			project_id,
			bidder,
			first_bid.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_failed_bid(
				RuntimeOrigin::signed(bidder),
				project_id,
				bidder,
				first_bid.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);

		let contributor = first_contribution.contributor;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_contribution(
			RuntimeOrigin::signed(contributor),
			project_id,
			contributor,
			first_contribution.id
		));
		assert_noop!(
			crate::Pallet::<TestRuntime>::settle_failed_contribution(
				RuntimeOrigin::signed(contributor),
				project_id,
				contributor,
				first_contribution.id
			),
			Error::<TestRuntime>::ParticipationNotFound
		);
	});
}

/// Test that the correct amount of PLMC is slashed from the evaluator independent of the
/// project outcome.
#[test]
fn evaluator_slashed_if_between_33_and_75() {
	let percentage = 50u64;
	let project_1 = create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::AcceptFunding));
	let project_2 = create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::RejectFunding));
	let projects = vec![project_1, project_2];

	for (mut inst, project_id) in projects {
		let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
		let evaluator = first_evaluation.evaluator;

		inst.execute(|| {
			let prev_balance = <TestRuntime as Config>::NativeCurrency::balance(&evaluator);
			match ProjectsDetails::<TestRuntime>::get(project_id).unwrap().status {
				ProjectStatus::FundingSuccessful => {
					assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					));
				},
				ProjectStatus::FundingFailed => {
					assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					));
				},
				_ => panic!("unexpected project status"),
			}
			let balance = <TestRuntime as Config>::NativeCurrency::balance(&evaluator);
			assert_eq!(
				balance,
				prev_balance +
					(Percent::from_percent(100) - <TestRuntime as Config>::EvaluatorSlash::get()) *
						first_evaluation.current_plmc_bond
			);
		});
	}
}

// Test that the evaluators PLMC bond is not slashed if the project is between 76 and 89
// percent funded independent of the project outcome.
#[test]
fn evaluator_plmc_unchanged_between_76_and_89() {
	let percentage = 80u64;
	let project_1 = create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::AcceptFunding));
	let project_2 = create_project_with_funding_percentage(percentage, Some(FundingOutcomeDecision::RejectFunding));
	let projects = vec![project_1, project_2];

	for (mut inst, project_id) in projects {
		let first_evaluation = inst.get_evaluations(project_id).into_iter().next().unwrap();
		let evaluator = first_evaluation.evaluator;

		inst.execute(|| {
			let prev_balance = <TestRuntime as Config>::NativeCurrency::balance(&evaluator);
			match ProjectsDetails::<TestRuntime>::get(project_id).unwrap().status {
				ProjectStatus::FundingSuccessful => {
					assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					));
				},
				ProjectStatus::FundingFailed => {
					assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_evaluation(
						RuntimeOrigin::signed(evaluator),
						project_id,
						evaluator,
						first_evaluation.id
					));
				},
				_ => panic!("unexpected project status"),
			}
			let balance = <TestRuntime as Config>::NativeCurrency::balance(&evaluator);
			assert_eq!(balance, prev_balance + first_evaluation.current_plmc_bond);
		});
	}
}

#[test]
fn bid_is_correctly_settled_for_successful_project() {
	let percentage = 100u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
	let issuer = &inst.get_issuer(project_id);
	inst.execute(|| {
		let bidder = first_bid.bidder;

		assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_bid(
			RuntimeOrigin::signed(bidder),
			project_id,
			bidder,
			first_bid.id
		));

		let reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
		let held_bidder = <TestRuntime as Config>::NativeCurrency::balance_on_hold(&reason, &bidder);
		assert_eq!(held_bidder, 0u32.into());

		let balance_issuer =
			<TestRuntime as Config>::FundingCurrency::balance(first_bid.funding_asset.to_assethub_id(), issuer);
		assert_eq!(balance_issuer, first_bid.funding_asset_amount_locked);

		let ct_amount = <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &bidder);
		assert_eq!(ct_amount, first_bid.final_ct_amount);
	});
}

#[test]
fn bid_is_correctly_settled_for_failed_project() {
	let percentage = 33u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let first_bid = inst.get_bids(project_id).into_iter().next().unwrap();
	inst.execute(|| {
		let bidder = first_bid.bidder;
		assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_bid(
			RuntimeOrigin::signed(bidder),
			project_id,
			bidder,
			first_bid.id
		));

		let reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
		let held_bidder = <TestRuntime as Config>::NativeCurrency::balance_on_hold(&reason, &bidder);
		assert_eq!(held_bidder, 0u32.into());

		let funding_asset_bidder =
			<TestRuntime as Config>::FundingCurrency::balance(first_bid.funding_asset.to_assethub_id(), &bidder);
		assert_eq!(funding_asset_bidder, first_bid.funding_asset_amount_locked);

		let ct_amount = <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &bidder);
		assert_eq!(ct_amount, Zero::zero());
	});
}

#[test]
fn contribution_is_correctly_settled_for_successful_project() {
	let percentage = 100u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
	let issuer = &inst.get_issuer(project_id);
	inst.execute(|| {
		let contributor = first_contribution.contributor;

		assert_ok!(crate::Pallet::<TestRuntime>::settle_successful_contribution(
			RuntimeOrigin::signed(contributor),
			project_id,
			contributor,
			first_contribution.id
		));

		let reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
		let held_contributor = <TestRuntime as Config>::NativeCurrency::balance_on_hold(&reason, &contributor);
		assert_eq!(held_contributor, 0u32.into());

		let balance_issuer = <TestRuntime as Config>::FundingCurrency::balance(
			first_contribution.funding_asset.to_assethub_id(),
			issuer,
		);
		assert_eq!(balance_issuer, first_contribution.usd_contribution_amount);

		let ct_amount = <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor);
		assert_eq!(ct_amount, first_contribution.ct_amount);
	});
}

#[test]
fn contribution_is_correctly_settled_for_failed_project() {
	let percentage = 33u64;
	let (mut inst, project_id) = create_project_with_funding_percentage(percentage, None);
	let first_contribution = inst.get_contributions(project_id).into_iter().next().unwrap();
	inst.execute(|| {
		let contributor = first_contribution.contributor;

		assert_ok!(crate::Pallet::<TestRuntime>::settle_failed_contribution(
			RuntimeOrigin::signed(contributor),
			project_id,
			contributor,
			first_contribution.id
		));

		let reason: RuntimeHoldReason = HoldReason::Participation(project_id).into();
		let held_contributor = <TestRuntime as Config>::NativeCurrency::balance_on_hold(&reason, &contributor);
		assert_eq!(held_contributor, 0u32.into());

		let funding_asset_contributor = <TestRuntime as Config>::FundingCurrency::balance(
			first_contribution.funding_asset.to_assethub_id(),
			&contributor,
		);
		assert_eq!(funding_asset_contributor, first_contribution.usd_contribution_amount);

		let ct_amount = <TestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor);
		assert_eq!(ct_amount, Zero::zero());
	});
}
#[test]
fn unsuccessful_bids_dont_get_vest_schedule() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(issuer);
	let evaluations = default_evaluations();
	let auction_token_allocation =
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

	let mut bids = inst.generate_bids_from_total_usd(
		Percent::from_percent(80) * project_metadata.minimum_price.saturating_mul_int(auction_token_allocation),
		project_metadata.minimum_price,
		vec![60, 40],
		vec![BIDDER_1, BIDDER_2],
		vec![1u8, 1u8],
	);

	let available_tokens = auction_token_allocation.saturating_sub(bids.iter().fold(0, |acc, bid| acc + bid.amount));

	let rejected_bid = vec![BidParams::new(BIDDER_5, available_tokens, 1u8, AcceptedFundingAsset::USDT)];
	let accepted_bid = vec![BidParams::new(BIDDER_4, available_tokens, 2u8, AcceptedFundingAsset::USDT)];
	bids.extend(rejected_bid.clone());
	bids.extend(accepted_bid.clone());

	let community_contributions = default_community_buys();

	let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

	let bidders_plmc =
		inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
	let bidders_existential_deposits = bidders_plmc.accounts().existential_deposits();
	inst.mint_plmc_to(bidders_plmc.clone());
	inst.mint_plmc_to(bidders_existential_deposits);

	let bidders_funding_assets = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata.clone(),
		None,
	);
	inst.mint_foreign_asset_to(bidders_funding_assets);

	inst.bid_for_users(project_id, bids).unwrap();

	inst.start_community_funding(project_id).unwrap();

	let final_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let contributors_plmc = inst.calculate_contributed_plmc_spent(community_contributions.clone(), final_price);
	let contributors_existential_deposits = contributors_plmc.accounts().existential_deposits();
	inst.mint_plmc_to(contributors_plmc.clone());
	inst.mint_plmc_to(contributors_existential_deposits);

	let contributors_funding_assets =
		inst.calculate_contributed_funding_asset_spent(community_contributions.clone(), final_price);
	inst.mint_foreign_asset_to(contributors_funding_assets);

	inst.contribute_for_users(project_id, community_contributions).unwrap();
	inst.start_remainder_or_end_funding(project_id).unwrap();
	inst.finish_funding(project_id).unwrap();

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
	inst.settle_project(project_id).unwrap();

	let plmc_locked_for_accepted_bid = inst.calculate_auction_plmc_charged_with_given_price(&accepted_bid, final_price);
	let plmc_locked_for_rejected_bid = inst.calculate_auction_plmc_charged_with_given_price(&rejected_bid, final_price);

	let UserToPLMCBalance { account: accepted_user, plmc_amount: accepted_plmc_amount } =
		plmc_locked_for_accepted_bid[0];
	let schedule = inst.execute(|| {
		<TestRuntime as Config>::Vesting::total_scheduled_amount(
			&accepted_user,
			HoldReason::Participation(project_id).into(),
		)
	});
	assert_close_enough!(schedule.unwrap(), accepted_plmc_amount, Perquintill::from_float(0.99));

	let UserToPLMCBalance { account: rejected_user, .. } = plmc_locked_for_rejected_bid[0];
	assert!(inst
		.execute(|| {
			<TestRuntime as Config>::Vesting::total_scheduled_amount(
				&rejected_user,
				HoldReason::Participation(project_id).into(),
			)
		})
		.is_none());
}
