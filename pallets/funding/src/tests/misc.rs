use super::*;

// check that functions created to facilitate testing return the expected results
mod helper_functions {
	use super::*;
	use polimec_common::USD_DECIMALS;

	#[test]
	fn test_usd_price_decimal_aware() {
		let submitted_price = FixedU128::from_float(1.85);
		let asset_decimals = 4;
		let expected_price = FixedU128::from_float(185.0);
		type PriceProvider = <TestRuntime as Config>::PriceProvider;
		assert_eq!(
			PriceProvider::calculate_decimals_aware_price(submitted_price, USD_DECIMALS, asset_decimals).unwrap(),
			expected_price
		);

		let submitted_price = FixedU128::from_float(1.0);
		let asset_decimals = 12;
		let expected_price = FixedU128::from_float(0.000001);

		assert_eq!(
			PriceProvider::calculate_decimals_aware_price(submitted_price, USD_DECIMALS, asset_decimals).unwrap(),
			expected_price
		);
	}

	#[test]
	fn test_convert_from_decimal_aware_back_to_normal() {
		// Test with an asset with less decimals than USD
		let original_price = FixedU128::from_float(1.85);
		let asset_decimals = 4;
		let decimal_aware = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
			original_price,
			USD_DECIMALS,
			asset_decimals,
		)
		.unwrap();
		let converted_back = <TestRuntime as Config>::PriceProvider::convert_back_to_normal_price(
			decimal_aware,
			USD_DECIMALS,
			asset_decimals,
		)
		.unwrap();
		assert_eq!(converted_back, original_price);

		// Test with an asset with more decimals than USD
		let original_price = FixedU128::from_float(1.85);
		let asset_decimals = 12;
		let decimal_aware = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
			original_price,
			USD_DECIMALS,
			asset_decimals,
		)
		.unwrap();
		let converted_back = <TestRuntime as Config>::PriceProvider::convert_back_to_normal_price(
			decimal_aware,
			USD_DECIMALS,
			asset_decimals,
		)
		.unwrap();
		assert_eq!(converted_back, original_price);
	}

	#[test]
	fn calculate_evaluation_plmc_spent() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1;
		const USD_AMOUNT_1: Balance = 150_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_1: f64 = 17_857.1428571428f64;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2;
		const USD_AMOUNT_2: Balance = 50_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_2: f64 = 5_952.3809523809f64;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3;
		const USD_AMOUNT_3: Balance = 75_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_3: f64 = 8_928.5714285714f64;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4;
		const USD_AMOUNT_4: Balance = 100 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_4: f64 = 11.9047619047f64;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5;

		// 123.7 USD
		const USD_AMOUNT_5: Balance = 1237 * USD_UNIT / 10;
		const EXPECTED_PLMC_AMOUNT_5: f64 = 14.7261904761f64;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(Location::here()).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let evaluations = vec![
			EvaluationParams::<TestRuntime>::from((EVALUATOR_1, USD_AMOUNT_1)),
			EvaluationParams::<TestRuntime>::from((EVALUATOR_2, USD_AMOUNT_2)),
			EvaluationParams::<TestRuntime>::from((EVALUATOR_3, USD_AMOUNT_3)),
			EvaluationParams::<TestRuntime>::from((EVALUATOR_4, USD_AMOUNT_4)),
			EvaluationParams::<TestRuntime>::from((EVALUATOR_5, USD_AMOUNT_5)),
		];

		let expected_plmc_spent = vec![
			(EVALUATOR_1, EXPECTED_PLMC_AMOUNT_1),
			(EVALUATOR_2, EXPECTED_PLMC_AMOUNT_2),
			(EVALUATOR_3, EXPECTED_PLMC_AMOUNT_3),
			(EVALUATOR_4, EXPECTED_PLMC_AMOUNT_4),
			(EVALUATOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let calculated_plmc_spent = inst
			.calculate_evaluation_plmc_spent(evaluations)
			.into_iter()
			.sorted_by(|a, b| a.account.cmp(&b.account))
			.map(|map| map.plmc_amount)
			.collect_vec();
		let expected_plmc_spent = expected_plmc_spent
			.into_iter()
			.sorted_by(|a, b| a.0.cmp(&b.0))
			.map(|map| {
				let f64_amount = map.1;
				let fixed_amount = FixedU128::from_float(f64_amount);
				fixed_amount.checked_mul_int(PLMC).unwrap()
			})
			.collect_vec();
		for (expected, calculated) in zip(expected_plmc_spent, calculated_plmc_spent) {
			assert_close_enough!(expected, calculated, Perquintill::from_float(0.999));
		}
	}
}

// logic of small functions that extrinsics use to process data or interact with storage
mod inner_functions {
	use super::*;

	#[test]
	fn calculate_vesting_duration() {
		let default_multiplier = MultiplierOf::<TestRuntime>::default();
		let default_multiplier_duration = default_multiplier.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(default_multiplier_duration, 1u64);

		let multiplier_1 = MultiplierOf::<TestRuntime>::try_from(1u8).unwrap();
		let multiplier_1_duration = multiplier_1.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_1_duration, 1u64);

		let multiplier_2 = MultiplierOf::<TestRuntime>::try_from(2u8).unwrap();
		let multiplier_2_duration = multiplier_2.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_2_duration, FixedU128::from_rational(2167, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_3 = MultiplierOf::<TestRuntime>::try_from(3u8).unwrap();
		let multiplier_3_duration = multiplier_3.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_3_duration, FixedU128::from_rational(4334, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_19 = MultiplierOf::<TestRuntime>::try_from(19u8).unwrap();
		let multiplier_19_duration = multiplier_19.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_19_duration, FixedU128::from_rational(39006, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_20 = MultiplierOf::<TestRuntime>::try_from(20u8).unwrap();
		let multiplier_20_duration = multiplier_20.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_20_duration, FixedU128::from_rational(41173, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_24 = MultiplierOf::<TestRuntime>::try_from(24u8).unwrap();
		let multiplier_24_duration = multiplier_24.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_24_duration, FixedU128::from_rational(49841, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_25 = MultiplierOf::<TestRuntime>::try_from(25u8).unwrap();
		let multiplier_25_duration = multiplier_25.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_25_duration, FixedU128::from_rational(52008, 1000).saturating_mul_int((DAYS * 7) as u64));
	}

	#[test]
	pub fn calculate_usd_sold_from_bucket() {
		let project_metadata = default_project_metadata(ISSUER_1);

		let mut bucket = Pallet::<TestRuntime>::create_bucket_from_metadata(&project_metadata).unwrap();
		bucket.update(10_000 * CT_UNIT);

		// We bought 10k CTs at a price of 10USD, meaning we should get 100k USD
		let usd_sold = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		assert_eq!(usd_sold, 100_000 * USD_UNIT);

		// This bucket has 2 buckets sold out on top of the first one
		let mut bucket = Pallet::<TestRuntime>::create_bucket_from_metadata(&project_metadata).unwrap();

		let usd_raised_first_bucket = bucket.current_price.saturating_mul_int(400_000 * CT_UNIT);
		bucket.update(project_metadata.total_allocation_size);

		let usd_raised_second_bucket = bucket.current_price.saturating_mul_int(50_000 * CT_UNIT);
		bucket.update(50_000 * CT_UNIT);

		let usd_raised_third_bucket = bucket.current_price.saturating_mul_int(50_000 * CT_UNIT);
		bucket.update(50_000 * CT_UNIT);

		let total_expected_rasied = usd_raised_first_bucket + usd_raised_second_bucket + usd_raised_third_bucket;

		// We bought 10k CTs at a price of 10USD, meaning we should get 100k USD
		let usd_sold = bucket.calculate_usd_raised(project_metadata.total_allocation_size);
		assert_eq!(usd_sold, total_expected_rasied);
	}
}

#[test]
fn project_state_transition_event() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_metadata = default_project_metadata(ISSUER_1);
	let project_id = inst.create_settled_project(
		project_metadata.clone(),
		ISSUER_1,
		None,
		inst.generate_successful_evaluations(project_metadata.clone(), 10),
		inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 90, 30),
		true,
	);

	let events = inst.execute(System::events);
	let transition_events = events
		.into_iter()
		.filter_map(|event| {
			if let RuntimeEvent::PolimecFunding(e @ crate::Event::ProjectPhaseTransition { .. }) = event.event {
				Some(e)
			} else {
				None
			}
		})
		.collect_vec();

	let mut desired_transitions = vec![
		ProjectStatus::EvaluationRound,
		ProjectStatus::AuctionRound,
		ProjectStatus::FundingSuccessful,
		ProjectStatus::SettlementStarted(FundingOutcome::Success),
		ProjectStatus::SettlementFinished(FundingOutcome::Success),
	]
	.into_iter();

	transition_events.into_iter().for_each(|event| {
		assert_eq!(event, Event::ProjectPhaseTransition { project_id, phase: desired_transitions.next().unwrap() });
	});
}
