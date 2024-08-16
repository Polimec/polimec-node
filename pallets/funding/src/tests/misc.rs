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
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1u32;
		const USD_AMOUNT_1: BalanceOf<TestRuntime> = 150_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_1: f64 = 17_857.1428571428f64;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2u32;
		const USD_AMOUNT_2: BalanceOf<TestRuntime> = 50_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_2: f64 = 5_952.3809523809f64;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3u32;
		const USD_AMOUNT_3: BalanceOf<TestRuntime> = 75_000 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_3: f64 = 8_928.5714285714f64;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4u32;
		const USD_AMOUNT_4: BalanceOf<TestRuntime> = 100 * USD_UNIT;
		const EXPECTED_PLMC_AMOUNT_4: f64 = 11.9047619047f64;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5u32;

		// 123.7 USD
		const USD_AMOUNT_5: BalanceOf<TestRuntime> = 1237 * USD_UNIT / 10;
		const EXPECTED_PLMC_AMOUNT_5: f64 = 14.7261904761f64;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let evaluations = vec![
			UserToUSDBalance::<TestRuntime>::new(EVALUATOR_1, USD_AMOUNT_1),
			UserToUSDBalance::<TestRuntime>::new(EVALUATOR_2, USD_AMOUNT_2),
			UserToUSDBalance::<TestRuntime>::new(EVALUATOR_3, USD_AMOUNT_3),
			UserToUSDBalance::<TestRuntime>::new(EVALUATOR_4, USD_AMOUNT_4),
			UserToUSDBalance::<TestRuntime>::new(EVALUATOR_5, USD_AMOUNT_5),
		];

		let expected_plmc_spent = vec![
			(EVALUATOR_1, EXPECTED_PLMC_AMOUNT_1),
			(EVALUATOR_2, EXPECTED_PLMC_AMOUNT_2),
			(EVALUATOR_3, EXPECTED_PLMC_AMOUNT_3),
			(EVALUATOR_4, EXPECTED_PLMC_AMOUNT_4),
			(EVALUATOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let calculated_plmc_spent = inst
			.calculate_evaluation_plmc_spent(evaluations, false)
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

	#[test]
	fn calculate_auction_plmc_returned() {
		const CT_AMOUNT_1: u128 = 5000 * CT_UNIT;
		const CT_AMOUNT_2: u128 = 40_000 * CT_UNIT;
		const CT_AMOUNT_3: u128 = 10_000 * CT_UNIT;
		const CT_AMOUNT_4: u128 = 6000 * CT_UNIT;
		const CT_AMOUNT_5: u128 = 2000 * CT_UNIT;

		let bid_1 = BidParams::new(BIDDER_1, CT_AMOUNT_1, 1u8, AcceptedFundingAsset::USDT);
		let bid_2 = BidParams::new(BIDDER_2, CT_AMOUNT_2, 1u8, AcceptedFundingAsset::USDT);
		let bid_3 = BidParams::new(BIDDER_1, CT_AMOUNT_3, 1u8, AcceptedFundingAsset::USDT);
		let bid_4 = BidParams::new(BIDDER_3, CT_AMOUNT_4, 1u8, AcceptedFundingAsset::USDT);
		let bid_5 = BidParams::new(BIDDER_4, CT_AMOUNT_5, 1u8, AcceptedFundingAsset::USDT);

		// post bucketing, the bids look like this:
		// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_1, 5k) - (BIDDER_1, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
		// | -------------------- 1USD ----------------------|---- 1.1 USD ---|---- 1.2 USD ----|----------- 1.3 USD -------------|
		// post wap ~ 1.0557252:
		// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

		const ORIGINAL_PLMC_CHARGED_BIDDER_1: f64 = 18_452.3809523790;
		const ORIGINAL_PLMC_CHARGED_BIDDER_2: f64 = 47_619.0476190470;
		const ORIGINAL_PLMC_CHARGED_BIDDER_3: f64 = 86_90.4761904760;
		const ORIGINAL_PLMC_CHARGED_BIDDER_4: f64 = 30_95.2380952380;

		const FINAL_PLMC_CHARGED_BIDDER_1: f64 = 12_236.4594692840;
		const FINAL_PLMC_CHARGED_BIDDER_2: f64 = 38_095.2380952380;
		const FINAL_PLMC_CHARGED_BIDDER_3: f64 = 75_40.8942202840;
		const FINAL_PLMC_CHARGED_BIDDER_4: f64 = 2_513.6314067610;

		let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * CT_UNIT,
			total_allocation_size: 100_000 * CT_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(10.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(5000 * USD_UNIT, None),
				institutional: TicketSize::new(5000 * USD_UNIT, None),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(USD_UNIT, None),
				professional: TicketSize::new(USD_UNIT, None),
				institutional: TicketSize::new(USD_UNIT, None),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
			funding_destination_account: ISSUER_1,
			policy_ipfs_cid: Some(ipfs_hash()),
		};

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER_1,
			None,
			default_evaluations(),
			bids.clone(),
		);

		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let expected_returns = vec![
			ORIGINAL_PLMC_CHARGED_BIDDER_1 - FINAL_PLMC_CHARGED_BIDDER_1,
			ORIGINAL_PLMC_CHARGED_BIDDER_2 - FINAL_PLMC_CHARGED_BIDDER_2,
			ORIGINAL_PLMC_CHARGED_BIDDER_3 - FINAL_PLMC_CHARGED_BIDDER_3,
			ORIGINAL_PLMC_CHARGED_BIDDER_4 - FINAL_PLMC_CHARGED_BIDDER_4,
		];

		let mut returned_plmc_mappings =
			inst.calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
		returned_plmc_mappings.sort_by(|b1, b2| b1.account.cmp(&b2.account));

		let returned_plmc_balances = returned_plmc_mappings.into_iter().map(|map| map.plmc_amount).collect_vec();

		for (expected_return, returned_balance) in zip(expected_returns, returned_plmc_balances) {
			let expected_value = FixedU128::from_float(expected_return).checked_mul_int(PLMC).unwrap();

			assert_close_enough!(expected_value, returned_balance, Perquintill::from_float(0.99));
		}
	}

	#[test]
	fn calculate_contributed_plmc_spent() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		const PLMC_PRICE: f64 = 8.4f64;
		const CT_PRICE: f64 = 16.32f64;

		const CONTRIBUTOR_1: AccountIdOf<TestRuntime> = 1u32;
		const TOKEN_AMOUNT_1: u128 = 120 * CT_UNIT;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 1_958_4_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: f64 = 233.1_428_571_428f64;

		const CONTRIBUTOR_2: AccountIdOf<TestRuntime> = 2u32;
		const TOKEN_AMOUNT_2: u128 = 5023 * CT_UNIT;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 81_975_3_600_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: f64 = 4_879.4_857_142_857f64;

		const CONTRIBUTOR_3: AccountIdOf<TestRuntime> = 3u32;
		const TOKEN_AMOUNT_3: u128 = 20_000 * CT_UNIT;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 326_400_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: f64 = 2_285.7_142_857_142f64;

		const CONTRIBUTOR_4: AccountIdOf<TestRuntime> = 4u32;
		const TOKEN_AMOUNT_4: u128 = 1_000_000 * CT_UNIT;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_4: u128 = 16_320_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: f64 = 77_714.2_857_142_857f64;

		const CONTRIBUTOR_5: AccountIdOf<TestRuntime> = 5u32;
		// 0.1233 CTs
		const TOKEN_AMOUNT_5: u128 = 1_233 * CT_UNIT / 10_000;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_5: u128 = 2_0_122_562_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: f64 = 0.0_239_554_285f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let contributions = vec![
			ContributionParams::new(CONTRIBUTOR_1, TOKEN_AMOUNT_1, MULTIPLIER_1, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_2, TOKEN_AMOUNT_2, MULTIPLIER_2, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_3, TOKEN_AMOUNT_3, MULTIPLIER_3, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_4, TOKEN_AMOUNT_4, MULTIPLIER_4, AcceptedFundingAsset::USDT),
			ContributionParams::new(CONTRIBUTOR_5, TOKEN_AMOUNT_5, MULTIPLIER_5, AcceptedFundingAsset::USDT),
		];

		let expected_plmc_spent = vec![
			(CONTRIBUTOR_1, EXPECTED_PLMC_AMOUNT_1),
			(CONTRIBUTOR_2, EXPECTED_PLMC_AMOUNT_2),
			(CONTRIBUTOR_3, EXPECTED_PLMC_AMOUNT_3),
			(CONTRIBUTOR_4, EXPECTED_PLMC_AMOUNT_4),
			(CONTRIBUTOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let calculated_plmc_spent = inst
			.calculate_contributed_plmc_spent(
				contributions,
				PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
					PriceOf::<TestRuntime>::from_float(CT_PRICE),
					USD_DECIMALS,
					CT_DECIMALS,
				)
				.unwrap(),
				false,
			)
			.into_iter()
			.sorted_by(|a, b| a.account.cmp(&b.account))
			.map(|map| map.plmc_amount)
			.collect_vec();
		let expected_plmc_spent = expected_plmc_spent
			.into_iter()
			.sorted_by(|a, b| a.0.cmp(&b.0))
			.map(|map| {
				let fixed_amount = FixedU128::from_float(map.1);
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

		let multiplier_1 = MultiplierOf::<TestRuntime>::new(1u8).unwrap();
		let multiplier_1_duration = multiplier_1.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_1_duration, 1u64);

		let multiplier_2 = MultiplierOf::<TestRuntime>::new(2u8).unwrap();
		let multiplier_2_duration = multiplier_2.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_2_duration, FixedU128::from_rational(2167, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_3 = MultiplierOf::<TestRuntime>::new(3u8).unwrap();
		let multiplier_3_duration = multiplier_3.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_3_duration, FixedU128::from_rational(4334, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_19 = MultiplierOf::<TestRuntime>::new(19u8).unwrap();
		let multiplier_19_duration = multiplier_19.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_19_duration, FixedU128::from_rational(39006, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_20 = MultiplierOf::<TestRuntime>::new(20u8).unwrap();
		let multiplier_20_duration = multiplier_20.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_20_duration, FixedU128::from_rational(41173, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_24 = MultiplierOf::<TestRuntime>::new(24u8).unwrap();
		let multiplier_24_duration = multiplier_24.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_24_duration, FixedU128::from_rational(49841, 1000).saturating_mul_int((DAYS * 7) as u64));

		let multiplier_25 = MultiplierOf::<TestRuntime>::new(25u8).unwrap();
		let multiplier_25_duration = multiplier_25.calculate_vesting_duration::<TestRuntime>();
		assert_eq!(multiplier_25_duration, FixedU128::from_rational(52008, 1000).saturating_mul_int((DAYS * 7) as u64));
	}
}
