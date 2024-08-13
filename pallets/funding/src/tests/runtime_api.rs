use super::*;
use crate::runtime_api::{ExtrinsicHelpers, Leaderboards, ProjectInformation, UserInformation};

#[test]
fn top_evaluations() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let evaluations = vec![
		UserToUSDBalance::new(EVALUATOR_1, 500_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_2, 250_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_3, 320_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_4, 1_000_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_1, 1_000 * USD_UNIT),
	];
	let project_id = inst.create_auctioning_project(default_project_metadata(ISSUER_1), ISSUER_1, None, evaluations);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let top_1 = TestRuntime::top_evaluations(&TestRuntime, block_hash, project_id, 1).unwrap();
		let evaluator_4_evaluation = Evaluations::<TestRuntime>::get((project_id, EVALUATOR_4, 3)).unwrap();
		assert!(top_1.len() == 1 && top_1[0] == evaluator_4_evaluation);

		let top_4_evaluators = TestRuntime::top_evaluations(&TestRuntime, block_hash, project_id, 4)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.evaluator)
			.collect_vec();
		assert_eq!(top_4_evaluators, vec![EVALUATOR_4, EVALUATOR_1, EVALUATOR_3, EVALUATOR_2]);

		let top_6_evaluators = TestRuntime::top_evaluations(&TestRuntime, block_hash, project_id, 6)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.evaluator)
			.collect_vec();
		assert_eq!(top_6_evaluators, vec![EVALUATOR_4, EVALUATOR_1, EVALUATOR_3, EVALUATOR_2, EVALUATOR_1]);
	});
}

#[test]
fn top_bids() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let bids = vec![
		(BIDDER_1, 8000 * CT_UNIT).into(),
		(BIDDER_2, 501 * CT_UNIT).into(),
		(BIDDER_3, 1200 * CT_UNIT).into(),
		(BIDDER_4, 10400 * CT_UNIT).into(),
		(BIDDER_1, 500 * CT_UNIT).into(),
	];
	let project_id = inst.create_community_contributing_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		None,
		default_evaluations(),
		bids,
	);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let top_1 = TestRuntime::top_bids(&TestRuntime, block_hash, project_id, 1).unwrap();
		let bidder_4_evaluation = Bids::<TestRuntime>::get((project_id, BIDDER_4, 3)).unwrap();
		assert!(top_1.len() == 1 && top_1[0] == bidder_4_evaluation);

		let top_4_bidders = TestRuntime::top_bids(&TestRuntime, block_hash, project_id, 4)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.bidder)
			.collect_vec();
		assert_eq!(top_4_bidders, vec![BIDDER_4, BIDDER_1, BIDDER_3, BIDDER_2]);

		let top_6_bidders = TestRuntime::top_bids(&TestRuntime, block_hash, project_id, 6)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.bidder)
			.collect_vec();
		assert_eq!(top_6_bidders, vec![BIDDER_4, BIDDER_1, BIDDER_3, BIDDER_2, BIDDER_1]);
	});
}

#[test]
fn top_contributions() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let community_contributors =
		vec![(BUYER_1, 8000 * CT_UNIT).into(), (BUYER_2, 501 * CT_UNIT).into(), (BUYER_3, 1200 * CT_UNIT).into()];
	let remainder_contributors = vec![(BUYER_4, 10400 * CT_UNIT).into(), (BUYER_1, 500 * CT_UNIT).into()];
	let project_id = inst.create_finished_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		None,
		default_evaluations(),
		default_bids(),
		community_contributors,
		remainder_contributors,
	);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let top_1 = TestRuntime::top_contributions(&TestRuntime, block_hash, project_id, 1).unwrap();
		let contributor_4_evaluation = Contributions::<TestRuntime>::get((project_id, BUYER_4, 3)).unwrap();
		assert!(top_1.len() == 1 && top_1[0] == contributor_4_evaluation);

		let top_4_contributors = TestRuntime::top_contributions(&TestRuntime, block_hash, project_id, 4)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.contributor)
			.collect_vec();
		assert_eq!(top_4_contributors, vec![BUYER_4, BUYER_1, BUYER_3, BUYER_2]);

		let top_6_contributors = TestRuntime::top_contributions(&TestRuntime, block_hash, project_id, 6)
			.unwrap()
			.into_iter()
			.map(|evaluation| evaluation.contributor)
			.collect_vec();
		assert_eq!(top_6_contributors, vec![BUYER_4, BUYER_1, BUYER_3, BUYER_2, BUYER_1]);
	});
}

#[test]
fn top_projects_by_usd_raised() {
	let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let (inst, project_id_1) = create_finished_project_with_usd_raised(inst, 400_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_2) =
		create_finished_project_with_usd_raised(inst, 1_200_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_3) =
		create_finished_project_with_usd_raised(inst, 3_000_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_4) = create_finished_project_with_usd_raised(inst, 840_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (mut inst, project_id_5) =
		create_finished_project_with_usd_raised(inst, 980_000 * USD_UNIT, 1_000_000 * USD_UNIT);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let top_1 = TestRuntime::top_projects_by_usd_raised(&TestRuntime, block_hash, 1u32).unwrap();
		let project_3_details = ProjectsDetails::<TestRuntime>::get(project_id_3).unwrap();
		let project_3_metadata = ProjectsMetadata::<TestRuntime>::get(project_id_3).unwrap();
		assert!(top_1.len() == 1 && top_1[0] == (project_id_3, project_3_metadata, project_3_details));

		let top_4 = TestRuntime::top_projects_by_usd_raised(&TestRuntime, block_hash, 4u32)
			.unwrap()
			.into_iter()
			.map(|(project_id, project_metadata, project_details)| {
				let stored_metadata = ProjectsMetadata::<TestRuntime>::get(project_id).unwrap();
				let stored_details = ProjectsDetails::<TestRuntime>::get(project_id).unwrap();
				assert!(project_metadata == stored_metadata && project_details == stored_details);
				project_id
			})
			.collect_vec();

		assert_eq!(top_4, vec![project_id_3, project_id_2, project_id_5, project_id_4]);

		let top_6 = TestRuntime::top_projects_by_usd_raised(&TestRuntime, block_hash, 6u32)
			.unwrap()
			.into_iter()
			.map(|(project_id, project_metadata, project_details)| {
				let stored_metadata = ProjectsMetadata::<TestRuntime>::get(project_id).unwrap();
				let stored_details = ProjectsDetails::<TestRuntime>::get(project_id).unwrap();
				assert!(project_metadata == stored_metadata && project_details == stored_details);
				project_id
			})
			.collect_vec();

		assert_eq!(top_6, vec![project_id_3, project_id_2, project_id_5, project_id_4, project_id_1]);
	});
}

#[test]
fn top_projects_by_usd_target_percent_reached() {
	let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let (inst, project_id_1) =
		create_finished_project_with_usd_raised(inst, 2_000_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_2) = create_finished_project_with_usd_raised(inst, 945_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_3) = create_finished_project_with_usd_raised(inst, 500_000 * USD_UNIT, 100_000 * USD_UNIT);

	let (mut inst, project_id_4) = create_finished_project_with_usd_raised(inst, 50_000 * USD_UNIT, 100_000 * USD_UNIT);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let top_1 = TestRuntime::top_projects_by_usd_target_percent_reached(&TestRuntime, block_hash, 1u32).unwrap();
		let project_3_details = ProjectsDetails::<TestRuntime>::get(project_id_3).unwrap();
		let project_3_metadata = ProjectsMetadata::<TestRuntime>::get(project_id_3).unwrap();
		assert!(top_1.len() == 1 && top_1[0] == (project_id_3, project_3_metadata, project_3_details));

		let top_3 = TestRuntime::top_projects_by_usd_target_percent_reached(&TestRuntime, block_hash, 3u32)
			.unwrap()
			.into_iter()
			.map(|(project_id, project_metadata, project_details)| {
				let stored_metadata = ProjectsMetadata::<TestRuntime>::get(project_id).unwrap();
				let stored_details = ProjectsDetails::<TestRuntime>::get(project_id).unwrap();
				assert!(project_metadata == stored_metadata && project_details == stored_details);
				project_id
			})
			.collect_vec();

		assert_eq!(top_3, vec![project_id_3, project_id_1, project_id_2]);

		let top_6 = TestRuntime::top_projects_by_usd_target_percent_reached(&TestRuntime, block_hash, 6u32)
			.unwrap()
			.into_iter()
			.map(|(project_id, project_metadata, project_details)| {
				let stored_metadata = ProjectsMetadata::<TestRuntime>::get(project_id).unwrap();
				let stored_details = ProjectsDetails::<TestRuntime>::get(project_id).unwrap();
				assert!(project_metadata == stored_metadata && project_details == stored_details);
				project_id
			})
			.collect_vec();

		assert_eq!(top_6, vec![project_id_3, project_id_1, project_id_2, project_id_4]);
	});
}

#[test]
fn contribution_tokens() {
	let bob = 420;
	let mut contributions_with_bob_1 = default_community_contributions();
	let bob_amount_1 = 10_000 * CT_UNIT;
	contributions_with_bob_1.last_mut().unwrap().contributor = bob;
	contributions_with_bob_1.last_mut().unwrap().amount = bob_amount_1;

	let mut contributions_with_bob_2 = default_community_contributions();
	let bob_amount_2 = 25_000 * CT_UNIT;
	contributions_with_bob_2.last_mut().unwrap().contributor = bob;
	contributions_with_bob_2.last_mut().unwrap().amount = bob_amount_2;

	let mut contributions_with_bob_3 = default_community_contributions();
	let bob_amount_3 = 5_020 * CT_UNIT;
	contributions_with_bob_3.last_mut().unwrap().contributor = bob;
	contributions_with_bob_3.last_mut().unwrap().amount = bob_amount_3;

	let mut contributions_with_bob_4 = default_community_contributions();
	let bob_amount_4 = 420 * CT_UNIT;
	contributions_with_bob_4.last_mut().unwrap().contributor = bob;
	contributions_with_bob_4.last_mut().unwrap().amount = bob_amount_4;

	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id_1 = inst.create_settled_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		None,
		default_evaluations(),
		default_bids(),
		contributions_with_bob_1,
		default_remainder_contributions(),
		true,
	);
	let _project_id_2 = inst.create_settled_project(
		default_project_metadata(ISSUER_2),
		ISSUER_2,
		None,
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);
	let _project_id_3 = inst.create_settled_project(
		default_project_metadata(ISSUER_3),
		ISSUER_3,
		None,
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);
	let project_id_4 = inst.create_settled_project(
		default_project_metadata(ISSUER_4),
		ISSUER_4,
		None,
		default_evaluations(),
		default_bids(),
		contributions_with_bob_2,
		default_remainder_contributions(),
		true,
	);
	let _project_id_5 = inst.create_settled_project(
		default_project_metadata(ISSUER_5),
		ISSUER_5,
		None,
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);
	let project_id_6 = inst.create_settled_project(
		default_project_metadata(ISSUER_6),
		ISSUER_6,
		None,
		default_evaluations(),
		default_bids(),
		contributions_with_bob_3,
		default_remainder_contributions(),
		true,
	);
	let project_id_7 = inst.create_settled_project(
		default_project_metadata(ISSUER_7),
		ISSUER_7,
		None,
		default_evaluations(),
		default_bids(),
		contributions_with_bob_4,
		default_remainder_contributions(),
		true,
	);

	let expected_items = vec![
		(project_id_4, bob_amount_2),
		(project_id_1, bob_amount_1),
		(project_id_6, bob_amount_3),
		(project_id_7, bob_amount_4),
	];

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let bob_items = TestRuntime::contribution_tokens(&TestRuntime, block_hash, bob).unwrap();
		assert_eq!(bob_items, expected_items);
	});
}

#[test]
fn funding_asset_to_ct_amount() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	// We want to use a funding asset that is not equal to 1 USD
	// Sanity check
	assert_eq!(
		PriceProviderOf::<TestRuntime>::get_price(AcceptedFundingAsset::DOT.id()).unwrap(),
		PriceOf::<TestRuntime>::from_float(69.0f64)
	);

	let dot_amount: u128 = 13_500_000_000_000;
	// USD Ticket = 93_150 USD

	// Easy case, wap is already calculated, we want to know how many tokens at wap we can buy with `x` USDT
	let project_metadata_1 = default_project_metadata(ISSUER_1);
	let project_id_1 = inst.create_community_contributing_project(
		project_metadata_1.clone(),
		ISSUER_1,
		None,
		default_evaluations(),
		vec![],
	);
	let wap = project_metadata_1.minimum_price;
	assert_eq!(inst.get_project_details(project_id_1).weighted_average_price.unwrap(), wap);

	// Price of ct is min price = 10 USD/CT
	let expected_ct_amount_contribution = 9_315 * CT_UNIT;
	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount(
			&TestRuntime,
			block_hash,
			project_id_1,
			AcceptedFundingAsset::DOT,
			dot_amount,
		)
		.unwrap();
		assert_eq!(ct_amount, expected_ct_amount_contribution);
	});

	// Medium case, contribution at a wap that is not the minimum price.
	let project_metadata_2 = default_project_metadata(ISSUER_2);
	let new_price = PriceOf::<TestRuntime>::from_float(16.3f64);
	let decimal_aware_price =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(new_price, USD_DECIMALS, CT_DECIMALS).unwrap();

	let bids =
		inst.generate_bids_that_take_price_to(project_metadata_2.clone(), decimal_aware_price, 420u32, |acc| acc + 1);
	let project_id_2 = inst.create_community_contributing_project(
		project_metadata_2.clone(),
		ISSUER_2,
		None,
		default_evaluations(),
		bids,
	);
	// Sanity check
	let project_details = inst.get_project_details(project_id_2);
	assert_eq!(project_details.weighted_average_price.unwrap(), decimal_aware_price);

	// 5'714.72... rounded down
	let expected_ct_amount_contribution = 5_714_720_000_000_000_000;
	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount(
			&TestRuntime,
			block_hash,
			project_id_2,
			AcceptedFundingAsset::DOT,
			dot_amount,
		)
		.unwrap();
		assert_close_enough!(ct_amount, expected_ct_amount_contribution, Perquintill::from_float(0.999f64));
	});

	// Medium case, a bid goes over part of a bucket (bucket after the first one)
	let project_metadata_3 = default_project_metadata(ISSUER_3);
	let project_id_3 =
		inst.create_auctioning_project(project_metadata_3.clone(), ISSUER_3, None, default_evaluations());
	let mut bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id_3)).unwrap();

	// We want a full bucket after filling 6 buckets. (first bucket has full allocation and initial price)
	// Price should be at 16 USD/CT
	bucket.current_price = bucket.initial_price + bucket.delta_price * FixedU128::from_float(6.0f64);
	bucket.amount_left = bucket.delta_amount;
	let bids = inst.generate_bids_from_bucket(
		project_metadata_3.clone(),
		bucket,
		420u32,
		|acc| acc + 1,
		AcceptedFundingAsset::USDT,
	);
	let necessary_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata_3.clone(),
		None,
		true,
	);
	let necessary_usdt = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata_3.clone(),
		None,
	);
	inst.mint_plmc_to(necessary_plmc);
	inst.mint_funding_asset_to(necessary_usdt);
	inst.bid_for_users(project_id_3, bids).unwrap();

	// Sanity check
	let expected_price = PriceOf::<TestRuntime>::from_float(16.0f64);
	let decimal_aware_expected_price =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(expected_price, USD_DECIMALS, CT_DECIMALS)
			.unwrap();
	let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id_3).unwrap());
	assert_eq!(current_bucket.current_price, decimal_aware_expected_price);

	dbg!(current_bucket.current_price.saturating_mul_int(current_bucket.amount_left));

	let dot_amount: u128 = 2_170_000_000_000;
	let expected_ct_amount: u128 = 935_812_500_000_000_000;

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount(
			&TestRuntime,
			block_hash,
			project_id_3,
			AcceptedFundingAsset::DOT,
			dot_amount,
		)
		.unwrap();
		assert_eq!(ct_amount, expected_ct_amount);
	});

	// Hard case, a bid goes over multiple buckets
	// We take the same project from before, and we add a bid that goes over 3 buckets.
	// Bucket size is 50k CTs, and current price is 16 USD/CT
	// We need to buy 50k at 16 , 50k at 17, and 13.5k at 18 = 1893k USD

	// Amount needed to spend 1893k USD through several buckets with DOT at 69 USD/DOT
	let dot_amount = 274_347_826_086_956_u128;
	let expected_ct_amount = 113_500 * CT_UNIT;

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount(
			&TestRuntime,
			block_hash,
			project_id_3,
			AcceptedFundingAsset::DOT,
			dot_amount,
		)
		.unwrap();
		assert_close_enough!(ct_amount, expected_ct_amount, Perquintill::from_float(0.9999));
	});
}

#[test]
fn all_project_participations_by_did() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let did_user = generate_did_from_account(420);
	let project_metadata = default_project_metadata(ISSUER_1);
	let cid = project_metadata.clone().policy_ipfs_cid.unwrap();
	let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);

	let evaluations = vec![
		UserToUSDBalance::new(EVALUATOR_1, 500_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_2, 250_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_3, 320_000 * USD_UNIT),
	];
	let bids = vec![
		BidParams::new(BIDDER_1, 400_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 50_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];
	let community_contributions = vec![
		ContributionParams::new(BUYER_1, 50_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BUYER_2, 130_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BUYER_3, 30_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BUYER_4, 210_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BUYER_5, 10_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];
	let remainder_contributions = vec![
		ContributionParams::new(EVALUATOR_2, 20_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BUYER_2, 5_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
		ContributionParams::new(BIDDER_1, 30_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT),
	];

	let evaluations_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);
	let bids_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata.clone(),
		None,
		true,
	);
	let community_contributions_plmc = inst.calculate_contributed_plmc_spent(
		community_contributions.clone(),
		project_metadata.minimum_price,
		true,
	);
	let remainder_contributions_plmc = inst.calculate_contributed_plmc_spent(
		remainder_contributions.clone(),
		project_metadata.minimum_price,
		true,
	);
	let all_plmc = inst.generic_map_operation(
		vec![evaluations_plmc, bids_plmc, community_contributions_plmc, remainder_contributions_plmc],
		MergeOperation::Add,
	);
	inst.mint_plmc_to(all_plmc);

	let bids_usdt = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata.clone(),
		None,
	);
	let community_contributions_usdt = inst.calculate_contributed_funding_asset_spent(
		community_contributions.clone(),
		project_metadata.minimum_price,
	);
	let remainder_contributions_usdt = inst.calculate_contributed_funding_asset_spent(
		remainder_contributions.clone(),
		project_metadata.minimum_price,
	);
	let all_usdt = inst.generic_map_operation(
		vec![bids_usdt, community_contributions_usdt, remainder_contributions_usdt],
		MergeOperation::Add,
	);
	inst.mint_funding_asset_to(all_usdt);

	inst.evaluate_for_users(project_id, evaluations[..1].to_vec()).unwrap();
	for evaluation in evaluations[1..].to_vec() {
		let jwt = get_mock_jwt_with_cid(evaluation.account, InvestorType::Retail, did_user.clone(), cid.clone());
		inst.execute(|| {
			PolimecFunding::evaluate(RuntimeOrigin::signed(evaluation.account), jwt, project_id, evaluation.usd_amount)
				.unwrap();
		});
	}

	assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::AuctionRound);

	inst.bid_for_users(project_id, bids[..1].to_vec()).unwrap();
	for bid in bids[1..].to_vec() {
		let jwt = get_mock_jwt_with_cid(bid.bidder, InvestorType::Institutional, did_user.clone(), cid.clone());
		inst.execute(|| {
			PolimecFunding::bid(
				RuntimeOrigin::signed(bid.bidder),
				jwt,
				project_id,
				bid.amount,
				bid.multiplier,
				bid.asset,
			)
			.unwrap();
		});
	}

	let ProjectStatus::CommunityRound(remainder_start) = inst.go_to_next_state(project_id) else {
		panic!("Expected CommunityRound")
	};
	inst.contribute_for_users(project_id, community_contributions).unwrap();

	inst.jump_to_block(remainder_start);

	for contribution in remainder_contributions {
		let jwt =
			get_mock_jwt_with_cid(contribution.contributor, InvestorType::Professional, did_user.clone(), cid.clone());
		inst.execute(|| {
			PolimecFunding::contribute(
				RuntimeOrigin::signed(contribution.contributor),
				jwt,
				project_id,
				contribution.amount,
				contribution.multiplier,
				contribution.asset,
			)
			.unwrap();
		});
	}

	assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let items =
			TestRuntime::all_project_participations_by_did(&TestRuntime, block_hash, project_id, did_user).unwrap();
		dbg!(items);
	});
}

#[test]
fn usd_target_percent_reached() {
	let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let (inst, project_id_1) =
		create_finished_project_with_usd_raised(inst, 2_000_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_2) = create_finished_project_with_usd_raised(inst, 945_000 * USD_UNIT, 1_000_000 * USD_UNIT);
	let (inst, project_id_3) = create_finished_project_with_usd_raised(inst, 517_000 * USD_UNIT, 100_000 * USD_UNIT);

	let (mut inst, project_id_4) = create_finished_project_with_usd_raised(inst, 50_000 * USD_UNIT, 100_000 * USD_UNIT);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let percent_200: FixedU128 =
			TestRuntime::usd_target_percent_reached(&TestRuntime, block_hash, project_id_1).unwrap();
		assert_close_enough!(
			percent_200.into_inner(),
			FixedU128::from_float(2.0f64).into_inner(),
			Perquintill::from_float(0.999)
		);

		let percent_94_5: FixedU128 =
			TestRuntime::usd_target_percent_reached(&TestRuntime, block_hash, project_id_2).unwrap();
		assert_close_enough!(
			percent_94_5.into_inner(),
			FixedU128::from_float(0.945f64).into_inner(),
			Perquintill::from_float(0.999)
		);

		let percent_517: FixedU128 =
			TestRuntime::usd_target_percent_reached(&TestRuntime, block_hash, project_id_3).unwrap();
		assert_close_enough!(
			percent_517.into_inner(),
			FixedU128::from_float(5.17f64).into_inner(),
			Perquintill::from_float(0.999)
		);

		let percent_50: FixedU128 =
			TestRuntime::usd_target_percent_reached(&TestRuntime, block_hash, project_id_4).unwrap();
		assert_close_enough!(
			percent_50.into_inner(),
			FixedU128::from_float(0.5f64).into_inner(),
			Perquintill::from_float(0.999)
		);
	});
}

#[test]
fn projects_by_did() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let did_user = generate_did_from_account(420);

	let project_id_1 = inst.create_settled_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		Some(did_user.clone()),
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);

	let _project_id_2 = inst.create_settled_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		None,
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);

	let project_id_3 = inst.create_settled_project(
		default_project_metadata(ISSUER_2),
		ISSUER_2,
		Some(did_user.clone()),
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);

	let _project_id_4 = inst.create_settled_project(
		default_project_metadata(ISSUER_3),
		ISSUER_3,
		None,
		default_evaluations(),
		default_bids(),
		default_community_contributions(),
		default_remainder_contributions(),
		true,
	);

	inst.execute(|| {
		let projects = ProjectsDetails::<TestRuntime>::iter().collect_vec();
		dbg!(projects);
		let block_hash = System::block_hash(System::block_number());
		let project_ids = TestRuntime::projects_by_did(&TestRuntime, block_hash, did_user).unwrap();
		assert_eq!(project_ids, vec![project_id_1, project_id_3]);
	});
}
