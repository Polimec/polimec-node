use super::*;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[test]
	fn application_round_completed() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

		let issuer = ISSUER_1;
		let project_metadata = default_project_metadata(issuer);

		inst.create_evaluating_project(project_metadata, issuer, None);
	}
}

#[cfg(test)]
mod create_project_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn project_id_autoincrement_works() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_1 = default_project_metadata(ISSUER_1);
			let project_2 = default_project_metadata(ISSUER_2);
			let project_3 = default_project_metadata(ISSUER_3);

			let created_project_1_id = inst.create_evaluating_project(project_1, ISSUER_1, None);
			let created_project_2_id = inst.create_evaluating_project(project_2, ISSUER_2, None);
			let created_project_3_id = inst.create_evaluating_project(project_3, ISSUER_3, None);

			assert_eq!(created_project_1_id, 0);
			assert_eq!(created_project_2_id, 1);
			assert_eq!(created_project_3_id, 2);
		}

		#[test]
		fn multiple_creations_different_issuers() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut issuer = ISSUER_1;
			for _ in 0..512 {
				let project_metadata = default_project_metadata(issuer);
				inst.create_evaluating_project(project_metadata, issuer, None);
				inst.advance_time(1u64);
				issuer += 1;
			}
		}

		#[test]
		fn multiple_funding_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let default_project_metadata = default_project_metadata(ISSUER_1);

			let mut one_currency_1 = default_project_metadata.clone();
			one_currency_1.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

			let mut one_currency_2 = default_project_metadata.clone();
			one_currency_2.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

			let mut one_currency_3 = default_project_metadata.clone();
			one_currency_3.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

			let mut two_currencies_1 = default_project_metadata.clone();
			two_currencies_1.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC].try_into().unwrap();

			let mut two_currencies_2 = default_project_metadata.clone();
			two_currencies_2.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::DOT].try_into().unwrap();

			let mut two_currencies_3 = default_project_metadata.clone();
			two_currencies_3.participation_currencies =
				vec![AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT].try_into().unwrap();

			let mut three_currencies = default_project_metadata.clone();
			three_currencies.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.try_into()
					.unwrap();

			let projects = vec![
				one_currency_1.clone(),
				one_currency_2.clone(),
				one_currency_3,
				two_currencies_1,
				two_currencies_2,
				two_currencies_3,
				three_currencies,
			];

			let mut issuer = ISSUER_1;
			for project in projects {
				issuer += 1;
				let issuer_mint = (issuer, 1000 * PLMC).into();
				inst.mint_plmc_to(vec![issuer_mint]);
				assert_ok!(inst.execute(|| {
					Pallet::<TestRuntime>::do_create_project(&issuer, project, generate_did_from_account(issuer))
				}));
			}
		}

		#[test]
		fn issuer_can_create_second_project_after_first_is_inactive() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer: AccountId = ISSUER_1;
			let did: Did = BoundedVec::new();
			let project_metadata: ProjectMetadataOf<TestRuntime> = default_project_metadata(issuer);
			let jwt: UntrustedToken = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				did,
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			let failing_bids =
				vec![(BIDDER_1, Professional, 1000 * CT_UNIT).into(), (BIDDER_2, Retail, 1000 * CT_UNIT).into()];
			let successful_evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
			let successful_bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 90, 10);

			let accounts = vec![
				vec![ISSUER_1],
				successful_evaluations.accounts(),
				successful_bids.accounts(),
				failing_bids.accounts(),
			]
			.concat()
			.into_iter()
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect_vec();

			inst.mint_plmc_to(
				accounts.iter().map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: 1_000_000 * PLMC }).collect(),
			);
			inst.mint_funding_asset_to(
				accounts
					.iter()
					.map(|acc| UserToFundingAsset {
						account: *acc,
						asset_amount: 1_000_000 * USD_UNIT,
						asset_id: USDT.id(),
					})
					.collect(),
			);
			inst.mint_funding_asset_to(
				accounts
					.iter()
					.map(|acc| UserToFundingAsset {
						account: *acc,
						asset_amount: 1_000_000 * USD_UNIT,
						asset_id: USDC.id(),
					})
					.collect(),
			);
			inst.mint_funding_asset_to(
				accounts
					.iter()
					.map(|acc| UserToFundingAsset {
						account: *acc,
						asset_amount: 1_000_000__000_000_000_0,
						asset_id: DOT.id(),
					})
					.collect(),
			);
			inst.mint_funding_asset_to(
				accounts
					.iter()
					.map(|acc| UserToFundingAsset {
						account: *acc,
						asset_amount: 1_000_000__000_000_000_000_000_000,
						asset_id: ETH.id(),
					})
					.collect(),
			);

			// Cannot create 2 projects consecutively
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::HasActiveProject
				);
			});

			// A Project is "inactive" after the evaluation fails
			assert_eq!(inst.go_to_next_state(0), ProjectStatus::EvaluationRound);

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::HasActiveProject
				);
			});
			assert_eq!(inst.go_to_next_state(0), ProjectStatus::FundingFailed);

			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// A Project is "inactive" after the funding fails
			assert_eq!(inst.go_to_next_state(1), ProjectStatus::EvaluationRound);

			inst.evaluate_for_users(1, successful_evaluations.clone()).unwrap();

			assert_eq!(inst.go_to_next_state(1), ProjectStatus::AuctionRound);

			inst.bid_for_users(1, failing_bids).unwrap();

			assert_eq!(inst.go_to_next_state(1), ProjectStatus::FundingFailed);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// A project is "inactive" after the funding succeeds
			assert_eq!(inst.go_to_next_state(2), ProjectStatus::EvaluationRound);
			inst.evaluate_for_users(2, successful_evaluations).unwrap();

			assert_eq!(inst.go_to_next_state(2), ProjectStatus::AuctionRound);

			inst.bid_for_users(2, successful_bids).unwrap();

			assert_eq!(inst.go_to_next_state(2), ProjectStatus::FundingSuccessful);

			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::create_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_metadata.clone()
			)));
		}

		#[test]
		fn shitcoin_tokenomics() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);

			// funding target of 1000 USD at 100 trillion supply
			const QUADRILLION_SUPPLY: u128 = 100_000_000_000_000 * CT_UNIT;
			// at the lowest possible price, which makes a funding target of 1 bn USD
			const LOW_PRICE: f64 = 0.00001f64;

			project_metadata.mainnet_token_max_supply = QUADRILLION_SUPPLY;
			project_metadata.total_allocation_size = QUADRILLION_SUPPLY;
			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				FixedU128::from_float(LOW_PRICE),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_ok!(crate::Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt,
					project_metadata
				));
			});
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn non_institutional_credential_fails() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Retail,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_metadata.clone()
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});

			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Professional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_metadata
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});
		}

		#[test]
		fn did_cannot_have_2_active_projects() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let ed = inst.get_ed();
			let issuer_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed * 2).into();
			// Create a first project
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.mint_plmc_to(vec![issuer_mint.clone()]);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// different account, same did
			let jwt = get_mock_jwt_with_cid(
				ISSUER_2,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::HasActiveProject
				);
			});
		}

		#[test]
		fn not_enough_plmc_for_escrow_ed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let ed = inst.get_ed();
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(ISSUER_1, ed)]);
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata,),
					Error::<TestRuntime>::IssuerNotEnoughFunds
				);
			});
		}

		#[test]
		fn mainnet_supply_less_than_allocation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000_001 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = 100_000_000 * CT_UNIT;
			inst.mint_plmc_to(default_plmc_balances());
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_create_project(
						&ISSUER_1,
						project_metadata,
						generate_did_from_account(ISSUER_1),
					),
					Error::<TestRuntime>::AllocationSizeError
				);
			});
		}

		#[test]
		fn invalid_ticket_sizes() {
			let correct_project = default_project_metadata(ISSUER_1);

			// min in bidding below 10 USD
			let mut wrong_project_1 = correct_project.clone();
			wrong_project_1.bidding_ticket_sizes.professional = TicketSize::new(9 * USD_UNIT, None);

			let mut wrong_project_2 = correct_project.clone();
			wrong_project_2.bidding_ticket_sizes.institutional = TicketSize::new(9 * USD_UNIT, None);

			let mut wrong_project_3 = correct_project.clone();
			wrong_project_3.bidding_ticket_sizes.professional = TicketSize::new(9 * USD_UNIT, None);
			wrong_project_3.bidding_ticket_sizes.institutional = TicketSize::new(9 * USD_UNIT, None);
			wrong_project_3.bidding_ticket_sizes.retail = TicketSize::new(9 * USD_UNIT, None);

			let mut wrong_project_4 = correct_project.clone();
			wrong_project_4.bidding_ticket_sizes.professional = TicketSize::new(0, None);
			wrong_project_4.bidding_ticket_sizes.institutional = TicketSize::new(0, None);
			wrong_project_4.bidding_ticket_sizes.retail = TicketSize::new(0, None);

			// min higher than max
			let mut wrong_project_8 = correct_project.clone();
			wrong_project_8.bidding_ticket_sizes.professional = TicketSize::new(100 * USD_UNIT, Some(50 * USD_UNIT));
			wrong_project_8.bidding_ticket_sizes.retail = TicketSize::new(100 * USD_UNIT, Some(50 * USD_UNIT));

			let mut wrong_project_9 = correct_project.clone();
			wrong_project_9.bidding_ticket_sizes.institutional =
				TicketSize::new(6000 * USD_UNIT, Some(5500 * USD_UNIT));

			let wrong_projects = vec![
				wrong_project_1.clone(),
				wrong_project_2,
				wrong_project_3.clone(),
				wrong_project_4,
				wrong_project_8,
				wrong_project_9,
			];

			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			inst.mint_plmc_to(default_plmc_balances());

			for project in wrong_projects {
				let project_err = inst.execute(|| {
					Pallet::<TestRuntime>::do_create_project(&ISSUER_1, project, generate_did_from_account(ISSUER_1))
						.unwrap_err()
				});
				assert_eq!(project_err, Error::<TestRuntime>::TicketSizeError.into());
			}
		}

		#[test]
		fn duplicated_participation_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut issuer = ISSUER_1;
			let default_project_metadata = default_project_metadata(ISSUER_1);

			let mut wrong_project_1 = default_project_metadata.clone();
			wrong_project_1.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT].try_into().unwrap();

			let mut wrong_project_2 = default_project_metadata.clone();
			wrong_project_2.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT]
					.try_into()
					.unwrap();

			let mut wrong_project_3 = default_project_metadata.clone();
			wrong_project_3.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::USDT]
					.try_into()
					.unwrap();

			let mut wrong_project_4 = default_project_metadata.clone();
			wrong_project_4.participation_currencies =
				vec![AcceptedFundingAsset::DOT, AcceptedFundingAsset::DOT, AcceptedFundingAsset::USDC]
					.try_into()
					.unwrap();

			let wrong_projects = vec![wrong_project_1, wrong_project_2, wrong_project_3, wrong_project_4];
			for project in wrong_projects {
				issuer += 1;
				let issuer_mint = (issuer, 1000 * PLMC).into();
				inst.mint_plmc_to(vec![issuer_mint]);
				let project_err = inst.execute(|| {
					Pallet::<TestRuntime>::do_create_project(&issuer, project, generate_did_from_account(issuer))
						.unwrap_err()
				});
				assert_eq!(project_err, Error::<TestRuntime>::ParticipationCurrenciesError.into());
			}
		}

		#[test]
		fn price_zero() {
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = 0_u128.into();

			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			inst.mint_plmc_to(default_plmc_balances());
			let project_err = inst.execute(|| {
				Pallet::<TestRuntime>::do_create_project(
					&ISSUER_1,
					project_metadata,
					generate_did_from_account(ISSUER_1),
				)
				.unwrap_err()
			});
			assert_eq!(project_err, Error::<TestRuntime>::PriceTooLow.into());
		}

		#[test]
		fn allocation_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 0;

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::AllocationSizeError
				);
			});
		}

		#[test]
		fn target_funding_less_than_1000_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = <PriceProviderOf<TestRuntime>>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.total_allocation_size = 999 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = 999 * CT_UNIT;

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::FundingTargetTooLow
				);
			});

			project_metadata.minimum_price = <PriceProviderOf<TestRuntime>>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(0.0001),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.total_allocation_size = 9999999u128 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = 9999999u128 * CT_UNIT;
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::FundingTargetTooLow
				);
			});
		}

		#[test]
		fn target_funding_more_than_1bn_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = <PriceProviderOf<TestRuntime>>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.total_allocation_size = 1_000_000_001 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = 1_000_000_001 * CT_UNIT;

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::FundingTargetTooHigh
				);
			});

			project_metadata.minimum_price = <PriceProviderOf<TestRuntime>>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(0.0001),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.total_allocation_size = 10_000_000_000_001 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = 10_000_000_000_001 * CT_UNIT;
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::FundingTargetTooHigh
				);
			});
		}

		#[test]
		fn unaccepted_decimal_ranges() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut fail_with_decimals = |decimals: u8| {
				let mut project_metadata = default_project_metadata(ISSUER_1);
				project_metadata.token_information.decimals = decimals;
				project_metadata.total_allocation_size = 100_000 * 10u128.pow(decimals.into());
				project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
				project_metadata.minimum_price =
					<TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
						PriceOf::<TestRuntime>::from_float(10_000.0f64),
						USD_DECIMALS,
						project_metadata.token_information.decimals,
					)
					.unwrap();

				let jwt = get_mock_jwt_with_cid(
					ISSUER_1,
					InvestorType::Institutional,
					generate_did_from_account(ISSUER_1),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::create_project(
							RuntimeOrigin::signed(ISSUER_1),
							jwt.clone(),
							project_metadata.clone()
						),
						Error::<TestRuntime>::BadDecimals
					);
				});
			};

			// less than 6 should fail
			for i in 0..=5 {
				fail_with_decimals(i);
			}

			// more than 18 should fail
			for i in 19..=24 {
				fail_with_decimals(i);
			}

			let mut issuer = ISSUER_2;
			let mut succeed_with_decimals = |decimals: u8| {
				let mut project_metadata = default_project_metadata(issuer);
				project_metadata.token_information.decimals = decimals;
				project_metadata.total_allocation_size = 100_000 * 10u128.pow(decimals.into());
				project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
				project_metadata.minimum_price =
					<TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
						PriceOf::<TestRuntime>::from_float(1.0),
						USD_DECIMALS,
						project_metadata.token_information.decimals,
					)
					.unwrap();
				let jwt = get_mock_jwt_with_cid(
					issuer,
					InvestorType::Institutional,
					generate_did_from_account(issuer),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);

				inst.mint_plmc_to(vec![(issuer, 1000 * PLMC).into()]);
				inst.execute(|| {
					assert_ok!(Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(issuer),
						jwt.clone(),
						project_metadata.clone()
					));
				});
				issuer += 1;
			};
			// 5 to 20 succeeds
			for i in 6..=18 {
				succeed_with_decimals(i);
			}
		}

		#[test]
		fn unaccepted_prices() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut issuer = ISSUER_1;
			let mut assert_price = |price: f64, fail: bool| {
				inst.mint_plmc_to(vec![(issuer, 1000 * PLMC).into()]);
				let mut project_metadata = default_project_metadata(issuer);

				// Need this helper function because the price provider does not allow prices below 10^-6
				let calculate_decimals_aware_price = |price: f64, decimals: u8| {
					let price = PriceOf::<TestRuntime>::from_float(price);
					let usd_unit = 10u128.checked_pow(USD_DECIMALS.into()).unwrap();
					let usd_price_with_decimals = price.checked_mul_int(usd_unit * 1_000_000).unwrap();
					let asset_unit = 10u128.checked_pow(decimals.into()).unwrap();

					let divisor = FixedU128::from_float(1_000_000f64);

					FixedU128::checked_from_rational(usd_price_with_decimals, asset_unit).unwrap().div(divisor)
				};

				project_metadata.minimum_price =
					calculate_decimals_aware_price(price, project_metadata.token_information.decimals);
				project_metadata.total_allocation_size =
					project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(100_000 * USD_UNIT);
				project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;

				let jwt = get_mock_jwt_with_cid(
					issuer,
					InvestorType::Institutional,
					generate_did_from_account(issuer),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);

				let run_extrinsic = || {
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(issuer),
						jwt.clone(),
						project_metadata.clone(),
					)
				};
				inst.execute(|| {
					if fail {
						assert_noop!(run_extrinsic(), Error::<TestRuntime>::BadTokenomics,);
					} else {
						assert_ok!(run_extrinsic());
					}
				});
				issuer += 1;
			};

			let low_prices = vec![0.0000001, 0.000001];
			let high_prices = vec![10_000f64, 100_000f64];
			let right_prices = vec![0.00001, 0.001, 0.01, 0.1, 1.0, 10.0, 100f64, 1_000f64];

			for price in low_prices {
				assert_price(price, true);
			}
			for price in high_prices {
				assert_price(price, true);
			}
			for price in right_prices {
				assert_price(price, false);
			}
		}

		#[test]
		fn allocation_smaller_than_decimals() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 2_000_000;
			project_metadata.token_information.decimals = 8;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(100_000.0f64),
				USD_DECIMALS,
				project_metadata.token_information.decimals,
			)
			.unwrap();

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::AllocationSizeError
				);
			});
		}
	}
}

#[cfg(test)]
mod edit_project_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn project_id_stays_the_same() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);

			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(15.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				project_metadata.clone()
			)));
			let next_project_id = inst.execute(NextProjectId::<TestRuntime>::get);
			assert_eq!(project_id, next_project_id - 1);
			let projects_details = inst.execute(|| ProjectsDetails::<TestRuntime>::iter_keys().collect_vec());
			let project_metadatas = inst.execute(|| ProjectsMetadata::<TestRuntime>::iter_keys().collect_vec());
			assert_eq!(projects_details, vec![project_id]);
			assert_eq!(project_metadatas, vec![project_id]);
		}

		#[test]
		fn multiple_fields_edited() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			let mut new_metadata_1 = project_metadata.clone();
			let new_policy_hash = ipfs_hash();
			new_metadata_1.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(15.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			let new_metadata_2 = ProjectMetadataOf::<TestRuntime> {
				token_information: CurrencyMetadata {
					name: BoundedVec::try_from("Changed Name".as_bytes().to_vec()).unwrap(),
					symbol: BoundedVec::try_from("CN".as_bytes().to_vec()).unwrap(),
					decimals: 12,
				},
				mainnet_token_max_supply: 100_000_000 * CT_UNIT,
				total_allocation_size: 5_000_000 * CT_UNIT,
				minimum_price: PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
					PriceOf::<TestRuntime>::from_float(20.0),
					USD_DECIMALS,
					CT_DECIMALS,
				)
				.unwrap(),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(10_000 * USD_UNIT, Some(20_000 * USD_UNIT)),
					institutional: TicketSize::new(20_000 * USD_UNIT, Some(30_000 * USD_UNIT)),
					retail: TicketSize::new(100 * USD_UNIT, None),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC]
					.try_into()
					.unwrap(),

				funding_destination_account: ISSUER_2,
				policy_ipfs_cid: Some(new_policy_hash),
				participants_account_type: ParticipantsAccountType::Polkadot,
			};

			// No fields changed
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				project_metadata.clone()
			)));
			inst.execute(|| {
				find_event!(TestRuntime, Event::<TestRuntime>::MetadataEdited{project_id, ref metadata}, project_id == 0, metadata == &project_metadata);
			});

			// Just one field changed
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				new_metadata_1.clone()
			)));
			assert_eq!(inst.get_project_metadata(project_id), new_metadata_1);
			inst.execute(|| {
				find_event!(TestRuntime, Event::<TestRuntime>::MetadataEdited{project_id, ref metadata}, project_id == 0, metadata == &new_metadata_1);
			});

			// All fields changed
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				new_metadata_2.clone()
			)));
			assert_eq!(inst.get_project_metadata(project_id), new_metadata_2);
			inst.execute(|| {
				find_event!(TestRuntime, Event::<TestRuntime>::MetadataEdited{project_id, ref metadata}, project_id == 0, metadata == &new_metadata_2);
			});
		}

		#[test]
		fn adding_project_policy() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.policy_ipfs_cid = None;
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			let mut new_metadata = project_metadata.clone();
			let new_policy_hash = ipfs_hash();
			new_metadata.policy_ipfs_cid = Some(new_policy_hash);
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				new_metadata.clone()
			)));
			assert_eq!(inst.get_project_metadata(project_id), new_metadata);
		}

		#[test]
		fn storage_changes() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			let mut new_metadata = project_metadata.clone();

			let new_price = PriceOf::<TestRuntime>::from_float(1f64);
			new_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				new_price,
				USD_DECIMALS,
				new_metadata.token_information.decimals,
			)
			.unwrap();
			new_metadata.total_allocation_size = 100_000 * CT_UNIT;
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				new_metadata.clone()
			)));
			// Project details reflect changes
			assert_eq!(inst.get_project_details(project_id).fundraising_target_usd, 100_000 * USD_UNIT);
			// Bucket reflects changes
			let new_bucket = Pallet::<TestRuntime>::create_bucket_from_metadata(&new_metadata).unwrap();
			let stored_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());
			assert_eq!(stored_bucket, new_bucket);
			// Event emitted
			inst.execute(|| {
				find_event!(TestRuntime, Event::<TestRuntime>::MetadataEdited{project_id, ref metadata}, project_id == 0, metadata == &new_metadata);
			});
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn called_by_different_issuer() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let issuer_1_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed).into();
			let issuer_2_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_2, ed).into();

			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			inst.mint_plmc_to(vec![issuer_1_mint.clone(), issuer_2_mint.clone()]);

			let jwt_1 = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata_1.clone().policy_ipfs_cid.unwrap(),
			);
			let jwt_2 = get_mock_jwt_with_cid(
				ISSUER_2,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_2),
				project_metadata_2.clone().policy_ipfs_cid.unwrap(),
			);

			let project_id_1 = inst.create_new_project(project_metadata_1.clone(), ISSUER_1, None);
			let project_id_2 = inst.create_new_project(project_metadata_2.clone(), ISSUER_2, None);

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt_2,
						project_id_1,
						project_metadata_2
					),
					Error::<TestRuntime>::NotIssuer
				);
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt_1,
						project_id_2,
						project_metadata_1
					),
					Error::<TestRuntime>::NotIssuer
				);
			});
		}

		#[test]
		fn evaluation_already_started() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::EvaluationRound);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id,
						project_metadata.clone()
					),
					Error::<TestRuntime>::ProjectIsFrozen
				);
			});
		}

		#[test]
		fn non_institutional_credential() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Retail,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_id,
						project_metadata.clone()
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});

			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Professional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});
		}
	}
}

#[cfg(test)]
mod remove_project_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn normal_remove() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::remove_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id
			)));
			inst.execute(|| {
				assert!(ProjectsDetails::<TestRuntime>::get(project_id).is_none());
				assert!(ProjectsMetadata::<TestRuntime>::get(project_id).is_none());
				assert!(Buckets::<TestRuntime>::get(project_id).is_none());
				assert!(DidWithActiveProjects::<TestRuntime>::get(generate_did_from_account(ISSUER_1)).is_none());
			});
		}

		#[test]
		fn can_create_after_remove() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let ed = inst.get_ed();
			let issuer_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed * 2).into();
			// Create a first project
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.mint_plmc_to(vec![issuer_mint.clone()]);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// Same account same did
			inst.mint_plmc_to(vec![issuer_mint.clone()]);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::HasActiveProject
				);
			});

			// Remove the first project
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::remove_project(RuntimeOrigin::signed(ISSUER_1), jwt.clone(), 0));
			});

			// Create a second project
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn non_issuer_credential() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Professional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id
					),
					Error::<TestRuntime>::WrongInvestorType
				);
			});
		}

		#[test]
		fn different_account() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_2,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_2),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt.clone(),
						project_id
					),
					Error::<TestRuntime>::NotIssuer
				);
			});
		}

		#[test]
		fn evaluation_already_started() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt_with_cid(
				ISSUER_1,
				InvestorType::Institutional,
				generate_did_from_account(ISSUER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1, None);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::EvaluationRound);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id,
					),
					Error::<TestRuntime>::ProjectIsFrozen
				);
			});
		}
	}
}
