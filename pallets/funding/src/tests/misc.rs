use super::*;

// check that functions created to facilitate testing return the expected results
mod helper_functions {
	use super::*;

	#[test]
	fn calculate_evaluation_plmc_spent() {
		const EVALUATOR_1: AccountIdOf<TestRuntime> = 1u32;
		const USD_AMOUNT_1: u128 = 150_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 17_857_1_428_571_428_u128;

		const EVALUATOR_2: AccountIdOf<TestRuntime> = 2u32;
		const USD_AMOUNT_2: u128 = 50_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 5_952_3_809_523_809_u128;

		const EVALUATOR_3: AccountIdOf<TestRuntime> = 3u32;
		const USD_AMOUNT_3: u128 = 75_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 8_928_5_714_285_714_u128;

		const EVALUATOR_4: AccountIdOf<TestRuntime> = 4u32;
		const USD_AMOUNT_4: u128 = 100_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 11_9_047_619_047_u128;

		const EVALUATOR_5: AccountIdOf<TestRuntime> = 5u32;
		const USD_AMOUNT_5: u128 = 123_7_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 14_7_261_904_761_u128;

		const PLMC_PRICE: f64 = 8.4f64;

		assert_eq!(
			<TestRuntime as Config>::PriceProvider::get_price(PLMC_FOREIGN_ID).unwrap(),
			PriceOf::<TestRuntime>::from_float(PLMC_PRICE)
		);

		let evaluations = vec![
			UserToUSDBalance::new(EVALUATOR_1, USD_AMOUNT_1),
			UserToUSDBalance::new(EVALUATOR_2, USD_AMOUNT_2),
			UserToUSDBalance::new(EVALUATOR_3, USD_AMOUNT_3),
			UserToUSDBalance::new(EVALUATOR_4, USD_AMOUNT_4),
			UserToUSDBalance::new(EVALUATOR_5, USD_AMOUNT_5),
		];

		let expected_plmc_spent = vec![
			UserToPLMCBalance::new(EVALUATOR_1, EXPECTED_PLMC_AMOUNT_1),
			UserToPLMCBalance::new(EVALUATOR_2, EXPECTED_PLMC_AMOUNT_2),
			UserToPLMCBalance::new(EVALUATOR_3, EXPECTED_PLMC_AMOUNT_3),
			UserToPLMCBalance::new(EVALUATOR_4, EXPECTED_PLMC_AMOUNT_4),
			UserToPLMCBalance::new(EVALUATOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = MockInstantiator::calculate_evaluation_plmc_spent(evaluations);
		assert_eq!(result, expected_plmc_spent);
	}

	#[test]
	fn calculate_auction_plmc_returned() {
		const CT_AMOUNT_1: u128 = 5000 * ASSET_UNIT;
		const CT_AMOUNT_2: u128 = 40_000 * ASSET_UNIT;
		const CT_AMOUNT_3: u128 = 10_000 * ASSET_UNIT;
		const CT_AMOUNT_4: u128 = 6000 * ASSET_UNIT;
		const CT_AMOUNT_5: u128 = 2000 * ASSET_UNIT;

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

		const ORIGINAL_PLMC_CHARGED_BIDDER_1: u128 = 18_452_3_809_523_790;
		const ORIGINAL_PLMC_CHARGED_BIDDER_2: u128 = 47_619_0_476_190_470;
		const ORIGINAL_PLMC_CHARGED_BIDDER_3: u128 = 86_90_4_761_904_760;
		const ORIGINAL_PLMC_CHARGED_BIDDER_4: u128 = 30_95_2_380_952_380;

		const FINAL_PLMC_CHARGED_BIDDER_1: u128 = 12_236_4_594_692_840;
		const FINAL_PLMC_CHARGED_BIDDER_2: u128 = 38_095_2_380_952_380;
		const FINAL_PLMC_CHARGED_BIDDER_3: u128 = 75_40_8_942_202_840;
		const FINAL_PLMC_CHARGED_BIDDER_4: u128 = 2_513_6_314_067_610;

		let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_metadata = ProjectMetadata {
			token_information: default_token_information(),
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 100_000 * ASSET_UNIT,
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

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			ISSUER_1,
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
			MockInstantiator::calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
		returned_plmc_mappings.sort_by(|b1, b2| b1.account.cmp(&b2.account));

		let returned_plmc_balances = returned_plmc_mappings.into_iter().map(|map| map.plmc_amount).collect_vec();

		for (expected, calculated) in zip(expected_returns, returned_plmc_balances) {
			assert_close_enough!(expected, calculated, Perquintill::from_float(0.99));
		}
	}

	#[test]
	fn calculate_contributed_plmc_spent() {
		const PLMC_PRICE: f64 = 8.4f64;
		const CT_PRICE: f64 = 16.32f64;

		const CONTRIBUTOR_1: AccountIdOf<TestRuntime> = 1u32;
		const TOKEN_AMOUNT_1: u128 = 120_0_000_000_000_u128;
		const MULTIPLIER_1: u8 = 1u8;
		const _TICKET_SIZE_USD_1: u128 = 1_958_4_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_1: u128 = 233_1_428_571_428_u128;

		const CONTRIBUTOR_2: AccountIdOf<TestRuntime> = 2u32;
		const TOKEN_AMOUNT_2: u128 = 5023_0_000_000_000_u128;
		const MULTIPLIER_2: u8 = 2u8;
		const _TICKET_SIZE_USD_2: u128 = 81_975_3_600_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_2: u128 = 4_879_4_857_142_857_u128;

		const CONTRIBUTOR_3: AccountIdOf<TestRuntime> = 3u32;
		const TOKEN_AMOUNT_3: u128 = 20_000_0_000_000_000_u128;
		const MULTIPLIER_3: u8 = 17u8;
		const _TICKET_SIZE_USD_3: u128 = 326_400_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_3: u128 = 2_285_7_142_857_142_u128;

		const CONTRIBUTOR_4: AccountIdOf<TestRuntime> = 4u32;
		const TOKEN_AMOUNT_4: u128 = 1_000_000_0_000_000_000_u128;
		const MULTIPLIER_4: u8 = 25u8;
		const _TICKET_SIZE_4: u128 = 16_320_000_0_000_000_000_u128;
		const EXPECTED_PLMC_AMOUNT_4: u128 = 77_714_2_857_142_857_u128;

		const CONTRIBUTOR_5: AccountIdOf<TestRuntime> = 5u32;
		const TOKEN_AMOUNT_5: u128 = 0_1_233_000_000_u128;
		const MULTIPLIER_5: u8 = 10u8;
		const _TICKET_SIZE_5: u128 = 2_0_122_562_000_u128;
		const EXPECTED_PLMC_AMOUNT_5: u128 = 0_0_239_554_285_u128;

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
			UserToPLMCBalance::new(CONTRIBUTOR_1, EXPECTED_PLMC_AMOUNT_1),
			UserToPLMCBalance::new(CONTRIBUTOR_2, EXPECTED_PLMC_AMOUNT_2),
			UserToPLMCBalance::new(CONTRIBUTOR_3, EXPECTED_PLMC_AMOUNT_3),
			UserToPLMCBalance::new(CONTRIBUTOR_4, EXPECTED_PLMC_AMOUNT_4),
			UserToPLMCBalance::new(CONTRIBUTOR_5, EXPECTED_PLMC_AMOUNT_5),
		];

		let result = MockInstantiator::calculate_contributed_plmc_spent(
			contributions,
			PriceOf::<TestRuntime>::from_float(CT_PRICE),
		);
		assert_eq!(result, expected_plmc_spent);
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

// test the parallel instantiation of projects
mod async_tests {
	use super::*;

	#[test]
	fn prototype_2() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let project_params = vec![
			TestProjectParams {
				expected_state: ProjectStatus::Application,
				metadata: default_project_metadata(ISSUER_1),
				issuer: ISSUER_1,
				evaluations: vec![],
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::EvaluationRound,
				metadata: default_project_metadata(ISSUER_2),
				issuer: ISSUER_2,
				evaluations: vec![],
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::AuctionOpening,
				metadata: default_project_metadata(ISSUER_3),
				issuer: ISSUER_3,
				evaluations: default_evaluations(),
				bids: vec![],
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::CommunityRound,
				metadata: default_project_metadata(ISSUER_4),
				issuer: ISSUER_4,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: vec![],
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::RemainderRound,
				metadata: default_project_metadata(ISSUER_5),
				issuer: ISSUER_5,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: default_community_buys(),
				remainder_contributions: vec![],
			},
			TestProjectParams {
				expected_state: ProjectStatus::FundingSuccessful,
				metadata: default_project_metadata(ISSUER_6),
				issuer: ISSUER_6,
				evaluations: default_evaluations(),
				bids: default_bids(),
				community_contributions: default_community_buys(),
				remainder_contributions: default_remainder_buys(),
			},
		];

		let (project_ids, mut inst) = create_multiple_projects_at(inst, project_params);

		dbg!(inst.get_project_details(project_ids[0]).status);
		dbg!(inst.get_project_details(project_ids[1]).status);
		dbg!(inst.get_project_details(project_ids[2]).status);
		dbg!(inst.get_project_details(project_ids[3]).status);
		dbg!(inst.get_project_details(project_ids[4]).status);
		dbg!(inst.get_project_details(project_ids[5]).status);

		assert_eq!(inst.get_project_details(project_ids[0]).status, ProjectStatus::Application);
		assert_eq!(inst.get_project_details(project_ids[1]).status, ProjectStatus::EvaluationRound);
		assert_eq!(inst.get_project_details(project_ids[2]).status, ProjectStatus::AuctionOpening);
		assert_eq!(inst.get_project_details(project_ids[3]).status, ProjectStatus::CommunityRound);
		assert_eq!(inst.get_project_details(project_ids[4]).status, ProjectStatus::RemainderRound);
		assert_eq!(inst.get_project_details(project_ids[5]).status, ProjectStatus::FundingSuccessful);
	}

	#[test]
	fn genesis_parallel_instantiaton() {
		let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// only used to generate some values, and not for chain interactions
		let funding_percent = 93u64;
		let project_metadata = default_project_metadata(ISSUER_1.into());
		let min_price = project_metadata.minimum_price;
		let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
			(project_metadata.minimum_price.checked_mul_int(project_metadata.total_allocation_size).unwrap());
		let evaluations = default_evaluations();
		let bids = MockInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(50u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_bidders(),
			default_bidder_multipliers(),
		);
		let community_contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(30u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_community_contributors(),
			default_community_contributor_multipliers(),
		);
		let remainder_contributions = MockInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(20u8) * twenty_percent_funding_usd,
			min_price,
			default_weights(),
			default_remainder_contributors(),
			default_remainder_contributor_multipliers(),
		);
		let ed = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		mock::RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: vec![
					(<TestRuntime as Config>::PalletId::get().into_account_truncating(), ed),
					(<TestRuntime as Config>::ContributionTreasury::get(), ed),
				],
			},
			foreign_assets: ForeignAssetsConfig {
				assets: vec![(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				)],
				metadata: vec![],
				accounts: vec![],
			},
			polimec_funding: PolimecFundingConfig {
				starting_projects: vec![
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::FundingSuccessful,
						metadata: default_project_metadata(ISSUER_1.into()),
						issuer: ISSUER_1.into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: community_contributions.clone(),
						remainder_contributions: remainder_contributions.clone(),
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::RemainderRound,
						metadata: default_project_metadata(ISSUER_2.into()),
						issuer: (ISSUER_2).into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: community_contributions.clone(),
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::CommunityRound,
						metadata: default_project_metadata(ISSUER_3.into()),
						issuer: (ISSUER_3).into(),
						evaluations: evaluations.clone(),
						bids: bids.clone(),
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::AuctionOpening,
						metadata: default_project_metadata(ISSUER_4.into()),
						issuer: ISSUER_4.into(),
						evaluations: evaluations.clone(),
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::EvaluationRound,
						metadata: default_project_metadata(ISSUER_5.into()),
						issuer: ISSUER_5.into(),
						evaluations: vec![],
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
					TestProjectParams::<TestRuntime> {
						expected_state: ProjectStatus::Application,
						metadata: default_project_metadata(ISSUER_6.into()),
						issuer: ISSUER_6.into(),
						evaluations: vec![],
						bids: vec![],
						community_contributions: vec![],
						remainder_contributions: vec![],
					},
				],
				phantom: PhantomData,
			},

			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let ext = sp_io::TestExternalities::new(t);
		let mut inst = MockInstantiator::new(Some(RefCell::new(ext)));

		dbg!(inst.get_project_details(0).status);
		dbg!(inst.get_project_details(1).status);
		dbg!(inst.get_project_details(2).status);
		dbg!(inst.get_project_details(3).status);
		dbg!(inst.get_project_details(4).status);
		dbg!(inst.get_project_details(5).status);

		assert_eq!(inst.get_project_details(5).status, ProjectStatus::Application);
		assert_eq!(inst.get_project_details(4).status, ProjectStatus::EvaluationRound);
		assert_eq!(inst.get_project_details(3).status, ProjectStatus::AuctionOpening);
		assert_eq!(inst.get_project_details(2).status, ProjectStatus::CommunityRound);
		assert_eq!(inst.get_project_details(1).status, ProjectStatus::RemainderRound);
		assert_eq!(inst.get_project_details(0).status, ProjectStatus::FundingSuccessful);
	}

	#[test]
	fn starting_auction_round_with_bids() {
		let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();

		// only used to generate some values, and not for chain interactions
		let mut project_metadata = default_project_metadata(ISSUER_1.into());
		let evaluations = default_evaluations();
		let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
		let min_bid = project_metadata.bidding_ticket_sizes.institutional.usd_minimum_per_participation.unwrap();
		let auction_allocation_percentage = project_metadata.auction_round_allocation_percentage;
		let auction_ct_required = min_bid.saturating_mul(max_bids_per_project as u128);
		let total_allocation_required = auction_allocation_percentage.saturating_reciprocal_mul(auction_ct_required);
		project_metadata.total_allocation_size = total_allocation_required;

		let max_bids = (0u32..max_bids_per_project)
			.map(|i| {
				instantiator::BidParams::<TestRuntime>::new(
					(i + 69).into(),
					project_metadata.bidding_ticket_sizes.institutional.usd_minimum_per_participation.unwrap(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		let ed = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		mock::RuntimeGenesisConfig {
			balances: BalancesConfig {
				balances: vec![
					(<TestRuntime as Config>::PalletId::get().into_account_truncating(), ed),
					(<TestRuntime as Config>::ContributionTreasury::get(), ed),
				],
			},
			foreign_assets: ForeignAssetsConfig {
				assets: vec![(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					false,
					10,
				)],
				metadata: vec![],
				accounts: vec![],
			},
			polimec_funding: PolimecFundingConfig {
				starting_projects: vec![TestProjectParams::<TestRuntime> {
					expected_state: ProjectStatus::AuctionOpening,
					metadata: default_project_metadata(ISSUER_1.into()),
					issuer: ISSUER_1.into(),
					evaluations: evaluations.clone(),
					bids: max_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				}],
				phantom: PhantomData,
			},

			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let ext = sp_io::TestExternalities::new(t);
		let mut inst = MockInstantiator::new(Some(RefCell::new(ext)));

		assert_eq!(inst.get_project_details(0).status, ProjectStatus::AuctionOpening);
		let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
		let total_bids_count = inst.execute(|| Bids::<TestRuntime>::iter_values().collect_vec().len());
		assert_eq!(total_bids_count, max_bids_per_project as usize);
	}
}

// Bug hunting
mod bug_hunting {
	use super::*;

	#[test]
	// Check that a failed do_function in on_initialize doesn't change the storage
	fn transactional_on_initialize() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let max_projects_per_update_block: u32 = <TestRuntime as Config>::MaxProjectsToUpdatePerBlock::get();
		// This bug will more likely happen with a limit of 1
		assert_eq!(max_projects_per_update_block, 1u32);
		let max_insertion_attempts: u32 = <TestRuntime as Config>::MaxProjectsToUpdateInsertionAttempts::get();

		let project_id = inst.create_evaluating_project(default_project_metadata(ISSUER_1), ISSUER_1);
		let plmc_balances = MockInstantiator::calculate_evaluation_plmc_spent(default_evaluations());
		let ed = plmc_balances.accounts().existential_deposits();
		inst.mint_plmc_to(plmc_balances);
		inst.mint_plmc_to(ed);
		inst.evaluate_for_users(project_id, default_evaluations()).unwrap();
		let update_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
		inst.execute(|| frame_system::Pallet::<TestRuntime>::set_block_number(update_block - 1));
		let now = inst.current_block();

		let auction_initialize_period_start_block = now + 2u64;
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block + <TestRuntime as Config>::AuctionInitializePeriodDuration::get();
		let automatic_auction_start = auction_initialize_period_end_block + 1u64;
		for i in 0..max_insertion_attempts {
			let key: BlockNumberFor<TestRuntime> = automatic_auction_start + i as u64;
			let val: (ProjectId, UpdateType) = (69u32, UpdateType::EvaluationEnd);
			inst.execute(|| crate::ProjectsToUpdate::<TestRuntime>::insert(key, val));
		}

		let old_project_details = inst.get_project_details(project_id);
		inst.advance_time(1).unwrap();

		let new_project_details = inst.get_project_details(project_id);
		assert_eq!(old_project_details, new_project_details);
	}
}
