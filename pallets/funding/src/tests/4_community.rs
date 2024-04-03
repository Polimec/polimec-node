use super::*;
use std::collections::HashMap;
pub const HOURS: BlockNumber = 300u64;

#[test]
fn community_round_completed() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let _ = inst.create_remainder_contributing_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
		default_community_buys(),
	);
}

#[test]
fn multiple_contribution_projects_completed() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project1 = default_project_metadata(ISSUER_1);
	let project2 = default_project_metadata(ISSUER_2);
	let project3 = default_project_metadata(ISSUER_3);
	let project4 = default_project_metadata(ISSUER_4);
	let evaluations = default_evaluations();
	let bids = default_bids();
	let community_buys = default_community_buys();

	inst.create_remainder_contributing_project(
		project1,
		ISSUER_1,
		evaluations.clone(),
		bids.clone(),
		community_buys.clone(),
	);
	inst.create_remainder_contributing_project(
		project2,
		ISSUER_2,
		evaluations.clone(),
		bids.clone(),
		community_buys.clone(),
	);
	inst.create_remainder_contributing_project(
		project3,
		ISSUER_3,
		evaluations.clone(),
		bids.clone(),
		community_buys.clone(),
	);
	inst.create_remainder_contributing_project(project4, ISSUER_4, evaluations, bids, community_buys);
}

#[test]
fn contribute_multiple_times_works() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let metadata = default_project_metadata(ISSUER_1);
	let issuer = ISSUER_1;
	let evaluations = default_evaluations();
	let bids = default_bids();
	let project_id = inst.create_community_contributing_project(metadata, issuer, evaluations, bids);

	const BOB: AccountId = 42;
	let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let contributions = vec![
		ContributionParams::new(BOB, 3 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BOB, 4 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];

	let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
	let plmc_existential_deposit = plmc_funding.accounts().existential_deposits();
	let foreign_funding =
		MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

	inst.mint_plmc_to(plmc_funding);
	inst.mint_plmc_to(plmc_existential_deposit);
	inst.mint_foreign_asset_to(foreign_funding);

	inst.contribute_for_users(project_id, vec![contributions[0].clone()])
		.expect("The Buyer should be able to buy multiple times");
	inst.advance_time(HOURS as BlockNumber).unwrap();

	inst.contribute_for_users(project_id, vec![contributions[1].clone()])
		.expect("The Buyer should be able to buy multiple times");

	let bob_total_contributions: BalanceOf<TestRuntime> = inst.execute(|| {
		Contributions::<TestRuntime>::iter_prefix_values((project_id, BOB)).map(|c| c.funding_asset_amount).sum()
	});

	let total_contributed = MockInstantiator::calculate_contributed_funding_asset_spent(contributions, token_price)
		.iter()
		.map(|item| item.asset_amount)
		.sum::<BalanceOf<TestRuntime>>();

	assert_eq!(bob_total_contributions, total_contributed);
}

#[test]
fn community_round_ends_on_all_ct_sold_exact() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let bids = vec![
		BidParams::new_with_defaults(BIDDER_1, 40_000 * ASSET_UNIT),
		BidParams::new_with_defaults(BIDDER_2, 10_000 * ASSET_UNIT),
	];
	let project_id = inst.create_community_contributing_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		bids,
	);
	const BOB: AccountId = 808;

	let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
	let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

	let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
	let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
	let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
	let foreign_asset_fundings =
		MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

	inst.mint_plmc_to(plmc_fundings.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

	// Buy remaining CTs
	inst.contribute_for_users(project_id, contributions)
		.expect("The Buyer should be able to buy the exact amount of remaining CTs");
	inst.advance_time(2u64).unwrap();
	// Check remaining CTs is 0
	assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0, "There are still remaining CTs");

	// Check project is in FundingEnded state
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

	inst.do_free_plmc_assertions(plmc_existential_deposits);
	inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
		BOB,
		0_u128,
		AcceptedFundingAsset::USDT.to_assethub_id(),
	)]);
	inst.do_reserved_plmc_assertions(vec![plmc_fundings[0].clone()], HoldReason::Participation(project_id).into());
	inst.do_contribution_transferred_foreign_asset_assertions(foreign_asset_fundings, project_id);
}

#[test]
fn community_round_ends_on_all_ct_sold_overbuy() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let bids = vec![
		BidParams::new(BIDDER_1, 40_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];
	let project_id = inst.create_community_contributing_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		bids,
	);
	const BOB: AccountId = 808;

	let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;

	let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

	let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
	let mut plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
	let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();
	let mut foreign_asset_fundings =
		MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

	inst.mint_plmc_to(plmc_fundings.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(foreign_asset_fundings.clone());

	// Buy remaining CTs
	inst.contribute_for_users(project_id, contributions)
		.expect("The Buyer should be able to buy the exact amount of remaining CTs");
	inst.advance_time(2u64).unwrap();

	// Check remaining CTs is 0
	assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0, "There are still remaining CTs");

	// Check project is in FundingEnded state
	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

	let reserved_plmc = plmc_fundings.swap_remove(0).plmc_amount;
	let _remaining_plmc: BalanceOf<TestRuntime> = plmc_fundings.iter().fold(0_u128, |acc, item| acc + item.plmc_amount);

	let actual_funding_transferred = foreign_asset_fundings.swap_remove(0).asset_amount;
	let remaining_foreign_assets: BalanceOf<TestRuntime> =
		foreign_asset_fundings.iter().fold(0_u128, |acc, item| acc + item.asset_amount);

	inst.do_free_plmc_assertions(plmc_existential_deposits);
	inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
		BOB,
		remaining_foreign_assets,
		AcceptedFundingAsset::USDT.to_assethub_id(),
	)]);
	inst.do_reserved_plmc_assertions(
		vec![UserToPLMCBalance::new(BOB, reserved_plmc)],
		HoldReason::Participation(project_id).into(),
	);
	inst.do_contribution_transferred_foreign_asset_assertions(
		vec![UserToForeignAssets::<TestRuntime>::new(
			BOB,
			actual_funding_transferred,
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)],
		project_id,
	);
}

#[test]
fn contribution_errors_if_limit_is_reached() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = inst.create_community_contributing_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
	);
	const CONTRIBUTOR: AccountIdOf<TestRuntime> = 420;

	let project_details = inst.get_project_details(project_id);
	let token_price = project_details.weighted_average_price.unwrap();

	// Create a contribution vector that will reach the limit of contributions for a user-project
	let token_amount: BalanceOf<TestRuntime> = ASSET_UNIT;
	let range = 0..<TestRuntime as Config>::MaxContributionsPerUser::get();
	let contributions: Vec<ContributionParams<_>> =
		range.map(|_| ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT)).collect();

	let plmc_funding = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), token_price);
	let plmc_existential_deposits = plmc_funding.accounts().existential_deposits();

	let foreign_funding =
		MockInstantiator::calculate_contributed_funding_asset_spent(contributions.clone(), token_price);

	inst.mint_plmc_to(plmc_funding.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());

	inst.mint_foreign_asset_to(foreign_funding.clone());

	// Reach up to the limit of contributions for a user-project
	assert!(inst.contribute_for_users(project_id, contributions).is_ok());

	// Try to contribute again, but it should fail because the limit of contributions for a user-project was reached.
	let over_limit_contribution = ContributionParams::new(CONTRIBUTOR, token_amount, 1u8, AcceptedFundingAsset::USDT);
	assert!(inst.contribute_for_users(project_id, vec![over_limit_contribution]).is_err());

	// Check that the right amount of PLMC is bonded, and funding currency is transferred
	let contributor_post_buy_plmc_balance =
		inst.execute(|| <TestRuntime as Config>::NativeCurrency::balance(&CONTRIBUTOR));
	let contributor_post_buy_foreign_asset_balance =
		inst.execute(|| <TestRuntime as Config>::FundingCurrency::balance(USDT_FOREIGN_ID, CONTRIBUTOR));

	assert_eq!(contributor_post_buy_plmc_balance, MockInstantiator::get_ed());
	assert_eq!(contributor_post_buy_foreign_asset_balance, 0);

	let plmc_bond_stored = inst.execute(|| {
		<TestRuntime as Config>::NativeCurrency::balance_on_hold(
			&HoldReason::Participation(project_id.into()).into(),
			&CONTRIBUTOR,
		)
	});
	let foreign_asset_contributions_stored = inst.execute(|| {
		Contributions::<TestRuntime>::iter_prefix_values((project_id, CONTRIBUTOR))
			.map(|c| c.funding_asset_amount)
			.sum::<BalanceOf<TestRuntime>>()
	});

	assert_eq!(plmc_bond_stored, MockInstantiator::sum_balance_mappings(vec![plmc_funding.clone()]));
	assert_eq!(
		foreign_asset_contributions_stored,
		MockInstantiator::sum_foreign_mappings(vec![foreign_funding.clone()])
	);
}

#[test]
fn retail_contributor_was_evaluator() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(issuer);
	let mut evaluations = default_evaluations();
	let evaluator_contributor = 69;
	let evaluation_amount = 420 * US_DOLLAR;
	let contribution =
		ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
	evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
	let bids = default_bids();

	let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);
	let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let already_bonded_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
		evaluator_contributor,
		evaluation_amount,
	)])[0]
		.plmc_amount;
	let plmc_available_for_participating =
		already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
	let necessary_plmc_for_contribution =
		MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;
	let necessary_usdt_for_contribution =
		MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

	inst.mint_plmc_to(vec![UserToPLMCBalance::new(
		evaluator_contributor,
		necessary_plmc_for_contribution - plmc_available_for_participating,
	)]);
	inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

	inst.contribute_for_users(project_id, vec![contribution]).unwrap();
}

#[test]
fn evaluator_cannot_use_slash_reserve_for_contributing_call_fail() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(issuer);
	let mut evaluations = default_evaluations();
	let bids = default_bids();

	let evaluator_contributor = 69;
	let evaluation_usd_amount = 400 * US_DOLLAR;
	let contribution_ct_amount =
		project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(evaluation_usd_amount) - 1 * ASSET_UNIT;

	let evaluation: UserToUSDBalance<TestRuntime> = (evaluator_contributor, evaluation_usd_amount).into();
	let contribution: ContributionParams<TestRuntime> = (evaluator_contributor, contribution_ct_amount).into();

	evaluations.push(evaluation.clone());

	let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);

	let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

	let plmc_evaluation_amount =
		MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation.clone()])[0].plmc_amount;
	let plmc_contribution_amount =
		MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;

	let evaluation_plmc_available_for_participating =
		plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;

	assert!(
		plmc_contribution_amount > evaluation_plmc_available_for_participating,
		"contribution should want to use slash reserve"
	);

	assert!(
		plmc_contribution_amount < plmc_evaluation_amount,
		"contribution should want to succeed by just using the slash reserve"
	);

	let necessary_usdt_for_contribution =
		MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);
	inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

	assert_matches!(inst.contribute_for_users(project_id, vec![contribution]), Err(_));
}

#[test]
fn evaluator_cannot_use_slash_reserve_for_contributing_call_success() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let issuer = ISSUER_1;
	let project_metadata = default_project_metadata(issuer);
	let mut evaluations = default_evaluations();
	let bids = default_bids();

	let evaluator_contributor = 69;
	let evaluation_usd_amount = 400 * US_DOLLAR;

	// We want to contribute in PLMC, less than what we used for evaluating, but more than what we have due to slash reserve
	let contribution_ct_amount =
		project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(evaluation_usd_amount) - 1 * ASSET_UNIT;

	let evaluation: UserToUSDBalance<TestRuntime> = (evaluator_contributor, evaluation_usd_amount).into();
	let contribution: ContributionParams<TestRuntime> = (evaluator_contributor, contribution_ct_amount).into();

	evaluations.push(evaluation.clone());

	let project_id = inst.create_community_contributing_project(project_metadata, issuer, evaluations, bids);

	let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();

	let plmc_evaluation_amount = MockInstantiator::calculate_evaluation_plmc_spent(vec![evaluation])[0].plmc_amount;
	let plmc_contribution_amount =
		MockInstantiator::calculate_contributed_plmc_spent(vec![contribution.clone()], ct_price)[0].plmc_amount;

	let evaluation_plmc_available_for_participating =
		plmc_evaluation_amount - <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount;

	assert!(
		plmc_contribution_amount > evaluation_plmc_available_for_participating,
		"contribution should want to use slash reserve"
	);

	assert!(
		plmc_contribution_amount < plmc_evaluation_amount,
		"contribution should want to succeed by just using the slash reserve"
	);

	let necessary_usdt_for_contribution =
		MockInstantiator::calculate_contributed_funding_asset_spent(vec![contribution.clone()], ct_price);

	// we mint what we would have taken from the reserve, to try and make the call pass
	inst.mint_plmc_to(vec![UserToPLMCBalance::new(
		evaluator_contributor,
		plmc_contribution_amount - evaluation_plmc_available_for_participating,
	)]);
	inst.mint_foreign_asset_to(necessary_usdt_for_contribution);
	inst.contribute_for_users(project_id, vec![contribution]).unwrap();

	let evaluation_locked = inst
		.get_reserved_plmc_balances_for(vec![evaluator_contributor], HoldReason::Evaluation(project_id).into())[0]
		.plmc_amount;
	let participation_locked = inst
		.get_reserved_plmc_balances_for(vec![evaluator_contributor], HoldReason::Participation(project_id).into())[0]
		.plmc_amount;

	assert_eq!(evaluation_locked, <TestRuntime as Config>::EvaluatorSlash::get() * plmc_evaluation_amount);
	assert_eq!(participation_locked, plmc_contribution_amount);
}

#[test]
fn round_has_total_ct_allocation_minus_auction_sold() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(ISSUER_1);
	let evaluations = default_evaluations();
	let bids = default_bids();

	let project_id = inst.create_community_contributing_project(
		project_metadata.clone(),
		ISSUER_1,
		evaluations.clone(),
		bids.clone(),
	);
	let project_details = inst.get_project_details(project_id);
	let bid_ct_sold: BalanceOf<TestRuntime> = inst.execute(|| {
		Bids::<TestRuntime>::iter_prefix_values((project_id,)).fold(Zero::zero(), |acc, bid| acc + bid.final_ct_amount)
	});
	assert_eq!(project_details.remaining_contribution_tokens, project_metadata.total_allocation_size - bid_ct_sold);

	let contributions = vec![(BUYER_1, project_details.remaining_contribution_tokens).into()];

	let plmc_contribution_funding = MockInstantiator::calculate_contributed_plmc_spent(
		contributions.clone(),
		project_details.weighted_average_price.unwrap(),
	);
	let plmc_existential_deposits = plmc_contribution_funding.accounts().existential_deposits();
	inst.mint_plmc_to(plmc_contribution_funding.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());

	let foreign_asset_contribution_funding = MockInstantiator::calculate_contributed_funding_asset_spent(
		contributions.clone(),
		project_details.weighted_average_price.unwrap(),
	);
	inst.mint_foreign_asset_to(foreign_asset_contribution_funding.clone());

	inst.contribute_for_users(project_id, contributions).unwrap();

	assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);
}

#[test]
fn per_credential_type_ticket_size_minimums() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = ProjectMetadata {
		token_information: default_token_information(),
		mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
		total_allocation_size: 1_000_000 * ASSET_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: PriceOf::<TestRuntime>::from_float(10.0),
		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(Some(8_000 * US_DOLLAR), None),
			institutional: TicketSize::new(Some(20_000 * US_DOLLAR), None),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(Some(10 * US_DOLLAR), None),
			professional: TicketSize::new(Some(100_000 * US_DOLLAR), None),
			institutional: TicketSize::new(Some(200_000 * US_DOLLAR), None),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: ISSUER_1,
		offchain_information_hash: Some(hashed(METADATA)),
	};

	let project_id = inst.create_community_contributing_project(
		project_metadata.clone(),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
	);

	inst.mint_plmc_to(vec![
		(BUYER_1, 50_000 * ASSET_UNIT).into(),
		(BUYER_2, 50_000 * ASSET_UNIT).into(),
		(BUYER_3, 50_000 * ASSET_UNIT).into(),
	]);

	inst.mint_foreign_asset_to(vec![
		(BUYER_1, 50_000 * US_DOLLAR).into(),
		(BUYER_2, 50_000 * US_DOLLAR).into(),
		(BUYER_3, 50_000 * US_DOLLAR).into(),
	]);

	// contribution below 1 CT (10 USD) should fail for retail
	let jwt = get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1));
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_1),
				jwt,
				project_id,
				ASSET_UNIT / 2,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooLow
		);
	});
	// contribution below 10_000 CT (100k USD) should fail for professionals
	let jwt = get_mock_jwt(BUYER_2, InvestorType::Professional, generate_did_from_account(BUYER_2));
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_2),
				jwt,
				project_id,
				9_999,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooLow
		);
	});

	// contribution below 20_000 CT (200k USD) should fail for institutionals
	let jwt = get_mock_jwt(BUYER_3, InvestorType::Professional, generate_did_from_account(BUYER_3));

	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_3),
				jwt,
				project_id,
				19_999,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooLow
		);
	});
}

#[test]
fn per_credential_type_ticket_size_maximums() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = ProjectMetadata {
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
			retail: TicketSize::new(None, Some(100_000 * US_DOLLAR)),
			professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
			institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: ISSUER_1,
		offchain_information_hash: Some(hashed(METADATA)),
	};

	let project_id = inst.create_community_contributing_project(
		project_metadata.clone(),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
	);

	inst.mint_plmc_to(vec![
		(BUYER_1, 500_000 * ASSET_UNIT).into(),
		(BUYER_2, 500_000 * ASSET_UNIT).into(),
		(BUYER_3, 500_000 * ASSET_UNIT).into(),
		(BUYER_4, 500_000 * ASSET_UNIT).into(),
		(BUYER_5, 500_000 * ASSET_UNIT).into(),
		(BUYER_6, 500_000 * ASSET_UNIT).into(),
	]);

	inst.mint_foreign_asset_to(vec![
		(BUYER_1, 500_000 * US_DOLLAR).into(),
		(BUYER_2, 500_000 * US_DOLLAR).into(),
		(BUYER_3, 500_000 * US_DOLLAR).into(),
		(BUYER_4, 500_000 * US_DOLLAR).into(),
		(BUYER_5, 500_000 * US_DOLLAR).into(),
		(BUYER_6, 500_000 * US_DOLLAR).into(),
	]);

	let buyer_1_jwt = get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1));
	let buyer_2_jwt_same_did = get_mock_jwt(BUYER_2, InvestorType::Retail, generate_did_from_account(BUYER_1));
	// total contributions with same DID above 10k CT (100k USD) should fail for retail
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_1),
			buyer_1_jwt,
			project_id,
			9000 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_2),
				buyer_2_jwt_same_did.clone(),
				project_id,
				1001 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooHigh
		);
	});
	// bidding 2k total works
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_2),
			buyer_2_jwt_same_did,
			project_id,
			1000 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});

	let buyer_3_jwt = get_mock_jwt(BUYER_3, InvestorType::Professional, generate_did_from_account(BUYER_3));
	let buyer_4_jwt_same_did = get_mock_jwt(BUYER_4, InvestorType::Professional, generate_did_from_account(BUYER_3));
	// total contributions with same DID above 2k CT (20k USD) should fail for professionals
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_3),
			buyer_3_jwt,
			project_id,
			1800 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_4),
				buyer_4_jwt_same_did.clone(),
				project_id,
				201 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooHigh
		);
	});
	// bidding 2k total works
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_4),
			buyer_4_jwt_same_did,
			project_id,
			200 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});

	let buyer_5_jwt = get_mock_jwt(BUYER_5, InvestorType::Institutional, generate_did_from_account(BUYER_5));
	let buyer_6_jwt_same_did = get_mock_jwt(BUYER_6, InvestorType::Institutional, generate_did_from_account(BUYER_5));
	// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_5),
			buyer_5_jwt,
			project_id,
			4690 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_6),
				buyer_6_jwt_same_did.clone(),
				project_id,
				311 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::ContributionTooHigh
		);
	});
	// bidding 5k total works
	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_6),
			buyer_6_jwt_same_did,
			project_id,
			310 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});
}

#[test]
fn contribute_with_multiple_currencies() {
	let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let mut project_metadata_all = default_project_metadata(ISSUER_1);
	project_metadata_all.participation_currencies =
		vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

	let mut project_metadata_usdt = default_project_metadata(ISSUER_2);
	project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

	let mut project_metadata_usdc = default_project_metadata(ISSUER_3);
	project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

	let mut project_metadata_dot = default_project_metadata(ISSUER_4);
	project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

	let evaluations = default_evaluations();

	let usdt_bids = default_bids()
		.into_iter()
		.map(|mut b| {
			b.asset = AcceptedFundingAsset::USDT;
			b
		})
		.collect::<Vec<_>>();

	let usdc_bids = default_bids()
		.into_iter()
		.map(|mut b| {
			b.asset = AcceptedFundingAsset::USDC;
			b
		})
		.collect::<Vec<_>>();

	let dot_bids = default_bids()
		.into_iter()
		.map(|mut b| {
			b.asset = AcceptedFundingAsset::DOT;
			b
		})
		.collect::<Vec<_>>();

	let projects = vec![
		TestProjectParams {
			expected_state: ProjectStatus::CommunityRound,
			metadata: project_metadata_all.clone(),
			issuer: ISSUER_1,
			evaluations: evaluations.clone(),
			bids: usdt_bids.clone(),
			community_contributions: vec![],
			remainder_contributions: vec![],
		},
		TestProjectParams {
			expected_state: ProjectStatus::CommunityRound,
			metadata: project_metadata_usdt,
			issuer: ISSUER_2,
			evaluations: evaluations.clone(),
			bids: usdt_bids.clone(),
			community_contributions: vec![],
			remainder_contributions: vec![],
		},
		TestProjectParams {
			expected_state: ProjectStatus::CommunityRound,
			metadata: project_metadata_usdc,
			issuer: ISSUER_3,
			evaluations: evaluations.clone(),
			bids: usdc_bids.clone(),
			community_contributions: vec![],
			remainder_contributions: vec![],
		},
		TestProjectParams {
			expected_state: ProjectStatus::CommunityRound,
			metadata: project_metadata_dot,
			issuer: ISSUER_4,
			evaluations: evaluations.clone(),
			bids: dot_bids.clone(),
			community_contributions: vec![],
			remainder_contributions: vec![],
		},
	];
	let (project_ids, mut inst) = create_multiple_projects_at(inst, projects);

	let project_id_all = project_ids[0];
	let project_id_usdt = project_ids[1];
	let project_id_usdc = project_ids[2];
	let project_id_dot = project_ids[3];

	let usdt_contribution = ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
	let usdc_contribution = ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
	let dot_contribution = ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

	let wap = inst.get_project_details(project_id_all).weighted_average_price.unwrap();

	let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(
		vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
		wap,
	);
	let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();

	let plmc_all_mints =
		MockInstantiator::generic_map_operation(vec![plmc_fundings, plmc_existential_deposits], MergeOperation::Add);
	inst.mint_plmc_to(plmc_all_mints.clone());
	inst.mint_plmc_to(plmc_all_mints.clone());
	inst.mint_plmc_to(plmc_all_mints.clone());

	let usdt_fundings = MockInstantiator::calculate_contributed_funding_asset_spent(
		vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
		wap,
	);
	inst.mint_foreign_asset_to(usdt_fundings.clone());
	inst.mint_foreign_asset_to(usdt_fundings.clone());
	inst.mint_foreign_asset_to(usdt_fundings.clone());

	assert_ok!(inst.contribute_for_users(
		project_id_all,
		vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()]
	));

	assert_ok!(inst.contribute_for_users(project_id_usdt, vec![usdt_contribution.clone()]));
	assert_err!(
		inst.contribute_for_users(project_id_usdt, vec![usdc_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
	assert_err!(
		inst.contribute_for_users(project_id_usdt, vec![dot_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);

	assert_err!(
		inst.contribute_for_users(project_id_usdc, vec![usdt_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
	assert_ok!(inst.contribute_for_users(project_id_usdc, vec![usdc_contribution.clone()]));
	assert_err!(
		inst.contribute_for_users(project_id_usdc, vec![dot_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);

	assert_err!(
		inst.contribute_for_users(project_id_dot, vec![usdt_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
	assert_err!(
		inst.contribute_for_users(project_id_dot, vec![usdc_contribution.clone()]),
		Error::<TestRuntime>::FundingAssetNotAccepted
	);
	assert_ok!(inst.contribute_for_users(project_id_dot, vec![dot_contribution.clone()]));
}

#[test]
fn issuer_cannot_contribute_his_project() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(ISSUER_1);
	let project_id = inst.create_community_contributing_project(
		project_metadata.clone(),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
	);
	assert_err!(
		inst.execute(|| crate::Pallet::<TestRuntime>::do_community_contribute(
			&(&ISSUER_1 + 1),
			project_id,
			500 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
			generate_did_from_account(ISSUER_1),
			InvestorType::Institutional
		)),
		Error::<TestRuntime>::ParticipationToThemselves
	);
}

#[test]
fn did_with_winning_bid_cannot_contribute() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(ISSUER_1);
	let bids = vec![
		BidParams::new(BIDDER_1, 400_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 50_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];

	let project_id =
		inst.create_community_contributing_project(project_metadata.clone(), ISSUER_1, default_evaluations(), bids);

	let bidder_2_jwt = get_mock_jwt(BIDDER_2, InvestorType::Retail, generate_did_from_account(BIDDER_2));
	let bidder_3_jwt_same_did = get_mock_jwt(BIDDER_3, InvestorType::Retail, generate_did_from_account(BIDDER_2));
	let bidder_3_jwt_different_did = get_mock_jwt(BIDDER_3, InvestorType::Retail, generate_did_from_account(BIDDER_3));

	let plmc_mints = vec![(BIDDER_2, 420 * PLMC).into(), (BIDDER_3, 420 * PLMC).into()];
	inst.mint_plmc_to(plmc_mints);
	let usdt_mints = vec![(BIDDER_2, 420 * ASSET_UNIT).into(), (BIDDER_3, 420 * ASSET_UNIT).into()];
	inst.mint_foreign_asset_to(usdt_mints);

	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BIDDER_2),
				bidder_2_jwt,
				project_id,
				10 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::UserHasWinningBids
		);
	});

	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BIDDER_3),
				bidder_3_jwt_same_did,
				project_id,
				10 * ASSET_UNIT,
				1u8.try_into().unwrap(),
				AcceptedFundingAsset::USDT,
			),
			Error::<TestRuntime>::UserHasWinningBids
		);
	});

	inst.execute(|| {
		assert_ok!(Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BIDDER_3),
			bidder_3_jwt_different_did,
			project_id,
			10 * ASSET_UNIT,
			1u8.try_into().unwrap(),
			AcceptedFundingAsset::USDT,
		));
	});
}

#[test]
fn non_retail_multiplier_limits() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = ProjectMetadata {
		token_information: default_token_information(),
		mainnet_token_max_supply: 80_000_000 * ASSET_UNIT,
		total_allocation_size: 10_000_000 * ASSET_UNIT,
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
	let evaluations = MockInstantiator::generate_successful_evaluations(
		project_metadata.clone(),
		default_evaluators(),
		default_weights(),
	);
	let bids = MockInstantiator::generate_bids_from_total_ct_percent(
		project_metadata.clone(),
		50,
		default_weights(),
		default_bidders(),
		default_multipliers(),
	);
	let project_id = inst.create_community_contributing_project(project_metadata.clone(), ISSUER_1, evaluations, bids);
	let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

	// Professional bids: 0x multiplier should fail
	let jwt = get_mock_jwt(BUYER_1, InvestorType::Professional, generate_did_from_account(BUYER_1));
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_1),
				jwt,
				project_id,
				1000 * ASSET_UNIT,
				Multiplier::force_new(0),
				AcceptedFundingAsset::USDT
			),
			Error::<TestRuntime>::ForbiddenMultiplier
		);
	});
	// Professional bids: 1 - 10x multiplier should work
	for multiplier in 1..=10u8 {
		let jwt = get_mock_jwt(BUYER_1, InvestorType::Professional, generate_did_from_account(BUYER_1));
		let bidder_plmc = MockInstantiator::calculate_contributed_plmc_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let bidder_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let ed = MockInstantiator::get_ed();
		inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
		inst.mint_plmc_to(bidder_plmc);
		inst.mint_foreign_asset_to(bidder_usdt);
		assert_ok!(inst.execute(|| Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_1),
			jwt,
			project_id,
			1000 * ASSET_UNIT,
			Multiplier::force_new(multiplier),
			AcceptedFundingAsset::USDT
		)));
	}
	// Professional bids: >=11x multiplier should fail
	for multiplier in 11..=50u8 {
		let jwt = get_mock_jwt(BUYER_1, InvestorType::Professional, generate_did_from_account(BUYER_1));
		let bidder_plmc = MockInstantiator::calculate_contributed_plmc_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let bidder_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let ed = MockInstantiator::get_ed();
		inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
		inst.mint_plmc_to(bidder_plmc);
		inst.mint_foreign_asset_to(bidder_usdt);
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_1),
					jwt,
					project_id,
					1000 * ASSET_UNIT,
					Multiplier::force_new(multiplier),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
		});
	}

	// Institutional bids: 0x multiplier should fail
	let jwt = get_mock_jwt(BUYER_2, InvestorType::Institutional, generate_did_from_account(BUYER_2));
	inst.execute(|| {
		assert_noop!(
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_2),
				jwt,
				project_id,
				1000 * ASSET_UNIT,
				Multiplier::force_new(0),
				AcceptedFundingAsset::USDT
			),
			Error::<TestRuntime>::ForbiddenMultiplier
		);
	});
	// Institutional bids: 1 - 25x multiplier should work
	for multiplier in 1..=25u8 {
		let jwt = get_mock_jwt(BUYER_2, InvestorType::Institutional, generate_did_from_account(BUYER_2));
		let bidder_plmc = MockInstantiator::calculate_contributed_plmc_spent(
			vec![(BUYER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let bidder_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![(BUYER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let ed = MockInstantiator::get_ed();
		inst.mint_plmc_to(vec![(BUYER_2, ed).into()]);
		inst.mint_plmc_to(bidder_plmc);
		inst.mint_foreign_asset_to(bidder_usdt);
		assert_ok!(inst.execute(|| Pallet::<TestRuntime>::community_contribute(
			RuntimeOrigin::signed(BUYER_2),
			jwt,
			project_id,
			1000 * ASSET_UNIT,
			multiplier.try_into().unwrap(),
			AcceptedFundingAsset::USDT
		)));
	}
	// Institutional bids: >=26x multiplier should fail
	for multiplier in 26..=50u8 {
		let jwt = get_mock_jwt(BUYER_2, InvestorType::Institutional, generate_did_from_account(BUYER_2));
		let bidder_plmc = MockInstantiator::calculate_contributed_plmc_spent(
			vec![(BUYER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let bidder_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![(BUYER_2, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let ed = MockInstantiator::get_ed();
		inst.mint_plmc_to(vec![(BUYER_2, ed).into()]);
		inst.mint_plmc_to(bidder_plmc);
		inst.mint_foreign_asset_to(bidder_usdt);
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_2),
					jwt,
					project_id,
					1000 * ASSET_UNIT,
					Multiplier::force_new(multiplier),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
		});
	}
}

#[test]
fn retail_multiplier_limits() {
	let _ = env_logger::try_init();
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let mut issuer: AccountId = 6969420;
	log::debug!("starting...");

	let mut create_project = |inst: &mut MockInstantiator| {
		issuer += 1;
		inst.create_community_contributing_project(
			default_project_metadata(issuer),
			issuer,
			default_evaluations(),
			default_bids(),
		)
	};
	let contribute = |inst: &mut MockInstantiator, project_id, multiplier| {
		let jwt = get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1));
		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let contributor_plmc = MockInstantiator::calculate_contributed_plmc_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let bidder_usdt = MockInstantiator::calculate_contributed_funding_asset_spent(
			vec![(BUYER_1, 1_000 * ASSET_UNIT, Multiplier::force_new(multiplier)).into()],
			wap,
		);
		let ed = MockInstantiator::get_ed();
		inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
		inst.mint_plmc_to(contributor_plmc);
		inst.mint_foreign_asset_to(bidder_usdt);
		inst.execute(|| {
			Pallet::<TestRuntime>::community_contribute(
				RuntimeOrigin::signed(BUYER_1),
				jwt,
				project_id,
				1000 * ASSET_UNIT,
				Multiplier::force_new(multiplier),
				AcceptedFundingAsset::USDT,
			)
		})
	};

	let max_allowed_multipliers_map = vec![(2, 1), (4, 2), (9, 4), (24, 7), (25, 10)];

	let mut previous_projects_created = 0;
	for (projects_participated_amount, max_allowed_multiplier) in max_allowed_multipliers_map {
		log::debug!("{projects_participated_amount:?}");

		log::debug!("{max_allowed_multiplier:?}");

		log::debug!("creating {} new projects", projects_participated_amount - previous_projects_created);

		(previous_projects_created..projects_participated_amount - 1).for_each(|_| {
			let project_id = create_project(&mut inst);
			log::debug!("created");
			assert_ok!(contribute(&mut inst, project_id, 1));
		});

		let project_id = create_project(&mut inst);
		log::debug!("created");
		previous_projects_created = projects_participated_amount;

		// 0x multiplier should fail
		// Professional bids: 0x multiplier should fail
		inst.execute(|| {
			assert_noop!(
				Pallet::<TestRuntime>::community_contribute(
					RuntimeOrigin::signed(BUYER_1),
					get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1)),
					project_id,
					1000 * ASSET_UNIT,
					Multiplier::force_new(0),
					AcceptedFundingAsset::USDT
				),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
		});

		// Multipliers that should work
		for multiplier in 1..=max_allowed_multiplier {
			log::debug!("success? - multiplier: {}", multiplier);
			assert_ok!(contribute(&mut inst, project_id, multiplier));
		}
		// dbg!

		// Multipliers that should NOT work
		for multiplier in max_allowed_multiplier + 1..=50 {
			log::debug!("error? - multiplier: {}", multiplier);
			assert_err!(contribute(&mut inst, project_id, multiplier), Error::<TestRuntime>::ForbiddenMultiplier);
		}
	}
}