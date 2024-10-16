use super::*;
use crate::runtime_api::{ExtrinsicHelpers, Leaderboards, ProjectInformation, UserInformation};
use frame_support::traits::fungibles::{metadata::Inspect, Mutate};
use sp_runtime::bounded_vec;

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

	let dot_amount: u128 = 1350_0_000_000_000;
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
		let ct_amount = TestRuntime::funding_asset_to_ct_amount_classic(
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
		inst.generate_bids_that_take_price_to(project_metadata_2.clone(), decimal_aware_price, 420, |acc| acc + 1);
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
		let ct_amount = TestRuntime::funding_asset_to_ct_amount_classic(
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
		420,
		|acc| acc + 1,
		AcceptedFundingAsset::USDT,
	);
	let necessary_plmc =
		inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata_3.clone(), None);
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

	let dot_amount: u128 = 217_0_000_000_000;
	let expected_ct_amount: u128 = 935_812_500_000_000_000;

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount_classic(
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
	let dot_amount = 27_434_7_826_086_956u128;
	let expected_ct_amount = 113_500 * CT_UNIT;

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let ct_amount = TestRuntime::funding_asset_to_ct_amount_classic(
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
fn get_next_vesting_schedule_merge_candidates() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let evaluations = vec![
		UserToUSDBalance::new(EVALUATOR_1, 500_000 * USD_UNIT),
		UserToUSDBalance::new(EVALUATOR_2, 250_000 * USD_UNIT),
		UserToUSDBalance::new(BIDDER_1, 320_000 * USD_UNIT),
	];
	let bids = vec![
		BidParams::new(BIDDER_1, 50_000 * CT_UNIT, ParticipationMode::Classic(10u8), AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_1, 400_000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 50_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
	];
	let remaining_contributions = vec![
		ContributionParams::new(BIDDER_1, 1_000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDT),
		ContributionParams::new(
			BIDDER_1,
			15_000 * CT_UNIT,
			ParticipationMode::Classic(10u8),
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(BIDDER_1, 100 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
	];

	let project_id = inst.create_finished_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		None,
		evaluations.clone(),
		bids.clone(),
		default_community_contributions(),
		remaining_contributions.clone(),
	);
	assert_eq!(ProjectStatus::SettlementStarted(FundingOutcome::Success), inst.go_to_next_state(project_id));
	inst.execute(|| {
		PolimecFunding::settle_evaluation(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 2).unwrap();
		PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 0).unwrap();
		PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 1).unwrap();
		PolimecFunding::settle_contribution(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 5).unwrap();
		PolimecFunding::settle_contribution(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 6).unwrap();
		PolimecFunding::settle_contribution(RuntimeOrigin::signed(BIDDER_1), project_id, BIDDER_1, 7).unwrap();
	});

	let hold_reason: mock::RuntimeHoldReason = HoldReason::Participation.into();
	let bidder_1_schedules =
		inst.execute(|| pallet_linear_release::Vesting::<TestRuntime>::get(BIDDER_1, hold_reason).unwrap().to_vec());
	// Evaluations didn't get a vesting schedule
	assert_eq!(bidder_1_schedules.len(), 4);

	inst.execute(|| {
		let block_hash = System::block_hash(System::block_number());
		let (idx_1, idx_2) = TestRuntime::get_next_vesting_schedule_merge_candidates(
			&TestRuntime,
			block_hash,
			BIDDER_1,
			HoldReason::Participation.into(),
			// within 100 blocks
			100u128,
		)
		.unwrap()
		.unwrap();
		assert_eq!((idx_1, idx_2), (1, 2));

		// Merging the two schedules deletes them and creates a new one at the end of the vec.
		LinearRelease::merge_schedules(RuntimeOrigin::signed(BIDDER_1), idx_1, idx_2, hold_reason).unwrap();

		let (idx_1, idx_2) = TestRuntime::get_next_vesting_schedule_merge_candidates(
			&TestRuntime,
			block_hash,
			BIDDER_1,
			HoldReason::Participation.into(),
			// within 100 blocks
			100u128,
		)
		.unwrap()
		.unwrap();
		assert_eq!((idx_1, idx_2), (0, 1));
	});
}

#[test]
fn calculate_otm_fee() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let mut project_metadata = default_project_metadata(ISSUER_1);
	project_metadata.participation_currencies = bounded_vec![AcceptedFundingAsset::DOT];

	let dot_id = AcceptedFundingAsset::DOT.id();
	let dot_decimals = inst.execute(|| ForeignAssets::decimals(dot_id));
	let dot_unit = 10u128.pow(dot_decimals as u32);
	let dot_ticket = 10_000 * dot_unit;
	let dot_ed = inst.get_funding_asset_ed(dot_id);

	let block_hash = inst.execute(|| System::block_hash(System::block_number()));
	let calculated_fee = inst.execute(|| {
		TestRuntime::calculate_otm_fee(&TestRuntime, block_hash, AcceptedFundingAsset::DOT, dot_ticket)
			.unwrap()
			.unwrap()
	});

	let project_id = inst.create_auctioning_project(project_metadata, ISSUER_1, None, default_evaluations());

	let ct_amount = inst
		.execute(|| {
			TestRuntime::funding_asset_to_ct_amount_classic(
				&TestRuntime,
				block_hash,
				project_id,
				AcceptedFundingAsset::DOT,
				dot_ticket,
			)
		})
		.unwrap();

	inst.execute(|| ForeignAssets::set_balance(dot_id, &BIDDER_1, dot_ticket + calculated_fee + dot_ed));

	let jwt = get_mock_jwt_with_cid(
		BIDDER_1,
		InvestorType::Professional,
		generate_did_from_account(BIDDER_1),
		default_project_metadata(ISSUER_1).policy_ipfs_cid.unwrap(),
	);

	inst.execute(|| {
		PolimecFunding::bid(
			RuntimeOrigin::signed(BIDDER_1),
			jwt,
			project_id,
			ct_amount,
			ParticipationMode::OTM,
			AcceptedFundingAsset::DOT,
		)
		.unwrap()
	});

	let balance = inst.get_free_funding_asset_balance_for(dot_id, BIDDER_1);
	inst.execute(|| {
		assert_close_enough!(balance, dot_ed, Perquintill::from_float(0.9999));
	});
}

#[test]
fn get_funding_asset_min_max_amounts() {
	ConstPriceProvider::set_price(AcceptedFundingAsset::USDT.id(), PriceOf::<TestRuntime>::from_float(1.0f64));
	ConstPriceProvider::set_price(AcceptedFundingAsset::USDC.id(), PriceOf::<TestRuntime>::from_float(1.0f64));
	ConstPriceProvider::set_price(AcceptedFundingAsset::DOT.id(), PriceOf::<TestRuntime>::from_float(10.0f64));
	ConstPriceProvider::set_price(PLMC_FOREIGN_ID, PriceOf::<TestRuntime>::from_float(0.5f64));
	const DOT_UNIT: u128 = 10u128.pow(10u32);

	// We test the following cases:
	// Bidder Professional where max is the ct max because it's lower than the ticket max. DOT
	// Bidder Institutional where max is the ticket max (first bid). USDT
	// Contributor Retail where the max is the ticket max (first contribution). DOT
	// Contributor Institutional where max is the ct max because there is no ticket max. USDT
	// Contributor Professional where max is the ticket max (4500 USD already contributed). USDC

	let mut project_metadata = default_project_metadata(ISSUER_1);
	let min_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
		PriceOf::<TestRuntime>::from_float(1.0f64),
		USD_DECIMALS,
		CT_DECIMALS,
	)
	.unwrap();
	project_metadata.minimum_price = min_price;
	project_metadata.total_allocation_size = 5_000_000 * CT_UNIT;
	project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
		professional: TicketSize {
			usd_minimum_per_participation: 5_000 * USD_UNIT,
			usd_maximum_per_did: Some(10_000_000 * USD_UNIT),
		},
		institutional: TicketSize {
			usd_minimum_per_participation: 10_000 * USD_UNIT,
			usd_maximum_per_did: Some(1_000_000 * USD_UNIT),
		},
		phantom: Default::default(),
	};
	project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
		retail: TicketSize {
			usd_minimum_per_participation: 50 * USD_UNIT,
			usd_maximum_per_did: Some(10_000 * USD_UNIT),
		},
		professional: TicketSize {
			usd_minimum_per_participation: 100 * USD_UNIT,
			usd_maximum_per_did: Some(100_000 * USD_UNIT),
		},
		institutional: TicketSize { usd_minimum_per_participation: 5000 * USD_UNIT, usd_maximum_per_did: None },
		phantom: Default::default(),
	};
	project_metadata.participation_currencies =
		bounded_vec![AcceptedFundingAsset::DOT, AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC];

	const BIDDING_USD_MAX: u128 = 2_500_000;
	const CONTRIBUTING_USD_MAX: u128 = 5_000_000;

	const BIDDER_PROFESSIONAL_DOT_MIN: u128 = 500 * DOT_UNIT;
	const BIDDER_PROFESSIONAL_DOT_MAX: u128 = (BIDDING_USD_MAX / 10) * DOT_UNIT;

	const BIDDER_INSTITUTIONAL_USDT_MIN: u128 = 10_000 * USD_UNIT;
	const BIDDER_INSTITUTIONAL_USDT_MAX: u128 = 1_000_000 * USD_UNIT;

	const CONTRIBUTOR_RETAIL_DOT_MIN: u128 = 5 * DOT_UNIT;
	const CONTRIBUTOR_RETAIL_DOT_MAX: u128 = 1_000 * DOT_UNIT;

	const CONTRIBUTOR_INSTITUTIONAL_USDT_MIN: u128 = 5000 * USD_UNIT;
	const CONTRIBUTOR_INSTITUTIONAL_USDT_MAX: u128 = CONTRIBUTING_USD_MAX * USD_UNIT;

	const CONTRIBUTOR_PROFESSIONAL_USDC_MIN: u128 = 100 * USD_UNIT;
	const CONTRIBUTOR_PROFESSIONAL_USDC_MAX: u128 = (100_000 - 6000) * USD_UNIT;

	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	let evaluations =
		inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
	let project_id = inst.create_auctioning_project(project_metadata, ISSUER_1, None, evaluations);

	let block_hash = inst.execute(|| System::block_hash(System::block_number()));

	let (min, max) = inst
		.execute(|| {
			TestRuntime::get_funding_asset_min_max_amounts(
				&TestRuntime,
				block_hash,
				project_id,
				generate_did_from_account(BIDDER_1),
				AcceptedFundingAsset::DOT,
				InvestorType::Professional,
			)
		})
		.unwrap()
		.unwrap();
	assert_eq!(min, BIDDER_PROFESSIONAL_DOT_MIN);
	assert_eq!(max, BIDDER_PROFESSIONAL_DOT_MAX);

	let (min, max) = inst
		.execute(|| {
			TestRuntime::get_funding_asset_min_max_amounts(
				&TestRuntime,
				block_hash,
				project_id,
				generate_did_from_account(BIDDER_1),
				AcceptedFundingAsset::USDT,
				InvestorType::Institutional,
			)
		})
		.unwrap()
		.unwrap();
	assert_eq!(min, BIDDER_INSTITUTIONAL_USDT_MIN);
	assert_eq!(max, BIDDER_INSTITUTIONAL_USDT_MAX);

	assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));

	let (min, max) = inst
		.execute(|| {
			TestRuntime::get_funding_asset_min_max_amounts(
				&TestRuntime,
				block_hash,
				project_id,
				generate_did_from_account(BUYER_1),
				AcceptedFundingAsset::DOT,
				InvestorType::Retail,
			)
		})
		.unwrap()
		.unwrap();
	assert_eq!(min, CONTRIBUTOR_RETAIL_DOT_MIN);
	assert_eq!(max, CONTRIBUTOR_RETAIL_DOT_MAX);

	let (min, max) = inst
		.execute(|| {
			TestRuntime::get_funding_asset_min_max_amounts(
				&TestRuntime,
				block_hash,
				project_id,
				generate_did_from_account(BUYER_1),
				AcceptedFundingAsset::USDT,
				InvestorType::Institutional,
			)
		})
		.unwrap()
		.unwrap();
	assert_eq!(min, CONTRIBUTOR_INSTITUTIONAL_USDT_MIN);
	assert_eq!(max, CONTRIBUTOR_INSTITUTIONAL_USDT_MAX);

	// This test requires the buyer to have contributed 4500 USD before calling the API
	let required_ct = inst
		.execute(|| {
			TestRuntime::funding_asset_to_ct_amount_classic(
				&TestRuntime,
				block_hash,
				project_id,
				AcceptedFundingAsset::USDC,
				6000 * USD_UNIT,
			)
		})
		.unwrap();
	let contribution =
		ContributionParams::new(BUYER_1, required_ct, ParticipationMode::OTM, AcceptedFundingAsset::USDC);
	let usdc_to_mint = inst.calculate_contributed_funding_asset_spent(vec![contribution.clone()], min_price);
	inst.mint_funding_asset_ed_if_required(usdc_to_mint.to_account_asset_map());
	inst.mint_funding_asset_to(usdc_to_mint);
	inst.contribute_for_users(project_id, vec![contribution]).unwrap();

	let (min, max) = inst
		.execute(|| {
			TestRuntime::get_funding_asset_min_max_amounts(
				&TestRuntime,
				block_hash,
				project_id,
				generate_did_from_account(BUYER_1),
				AcceptedFundingAsset::USDC,
				InvestorType::Professional,
			)
		})
		.unwrap()
		.unwrap();
	assert_eq!(min, CONTRIBUTOR_PROFESSIONAL_USDC_MIN);
	assert_eq!(max, CONTRIBUTOR_PROFESSIONAL_USDC_MAX);
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
		BidParams::new(BIDDER_1, 400_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
		BidParams::new(BIDDER_2, 50_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
	];
	let community_contributions = vec![
		ContributionParams::new(BUYER_1, 50_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
		ContributionParams::new(
			BUYER_2,
			130_000 * CT_UNIT,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(BUYER_3, 30_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
		ContributionParams::new(
			BUYER_4,
			210_000 * CT_UNIT,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(BUYER_5, 10_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
	];
	let remainder_contributions = vec![
		ContributionParams::new(
			EVALUATOR_2,
			20_000 * CT_UNIT,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(BUYER_2, 5_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT),
		ContributionParams::new(
			BIDDER_1,
			30_000 * CT_UNIT,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		),
	];

	let evaluations_plmc = inst.calculate_evaluation_plmc_spent(evaluations.clone());
	let bids_plmc =
		inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
	let community_contributions_plmc =
		inst.calculate_contributed_plmc_spent(community_contributions.clone(), project_metadata.minimum_price);
	let remainder_contributions_plmc =
		inst.calculate_contributed_plmc_spent(remainder_contributions.clone(), project_metadata.minimum_price);
	let all_plmc = inst.generic_map_operation(
		vec![evaluations_plmc, bids_plmc, community_contributions_plmc, remainder_contributions_plmc],
		MergeOperation::Add,
	);
	inst.mint_plmc_ed_if_required(all_plmc.accounts());
	inst.mint_plmc_to(all_plmc);

	let bids_usdt = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata.clone(),
		None,
	);
	let community_contributions_usdt =
		inst.calculate_contributed_funding_asset_spent(community_contributions.clone(), project_metadata.minimum_price);
	let remainder_contributions_usdt =
		inst.calculate_contributed_funding_asset_spent(remainder_contributions.clone(), project_metadata.minimum_price);
	let all_usdt = inst.generic_map_operation(
		vec![bids_usdt, community_contributions_usdt, remainder_contributions_usdt],
		MergeOperation::Add,
	);
	inst.mint_funding_asset_ed_if_required(all_usdt.to_account_asset_map());
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
			PolimecFunding::bid(RuntimeOrigin::signed(bid.bidder), jwt, project_id, bid.amount, bid.mode, bid.asset)
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
				contribution.mode,
				contribution.asset,
			)
			.unwrap();
		});
	}

	assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
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
		let block_hash = System::block_hash(System::block_number());
		let project_ids = TestRuntime::projects_by_did(&TestRuntime, block_hash, did_user).unwrap();
		assert_eq!(project_ids, vec![project_id_1, project_id_3]);
	});
}
