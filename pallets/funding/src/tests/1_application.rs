use super::*;
use polimec_common::credentials::InvestorType;
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn application_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);

			inst.create_evaluating_project(project_metadata, issuer);
		}
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

			let created_project_1_id = inst.create_evaluating_project(project_1, ISSUER_1);
			let created_project_2_id = inst.create_evaluating_project(project_2, ISSUER_2);
			let created_project_3_id = inst.create_evaluating_project(project_3, ISSUER_3);

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
				inst.create_evaluating_project(project_metadata, issuer);
				inst.advance_time(1u64).unwrap();
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
			let jwt: UntrustedToken = get_mock_jwt(ISSUER_1, InvestorType::Institutional, did);
			let project_metadata: ProjectMetadataOf<TestRuntime> = default_project_metadata(issuer);

			let failing_bids = vec![(BIDDER_1, 1000 * ASSET_UNIT).into(), (BIDDER_2, 1000 * ASSET_UNIT).into()];

			inst.mint_plmc_to(default_plmc_balances());
			inst.mint_foreign_asset_to(default_usdt_balances());

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
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
				);
			});

			// A Project is "inactive" after the evaluation fails
			inst.start_evaluation(0, ISSUER_1).unwrap();
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
				);
			});
			inst.advance_time(<TestRuntime as Config>::EvaluationDuration::get() + 1).unwrap();
			assert_eq!(inst.get_project_details(0).status, ProjectStatus::EvaluationFailed);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// A Project is "inactive" after the auction fails
			inst.start_evaluation(1, ISSUER_1).unwrap();
			inst.evaluate_for_users(1, default_evaluations()).unwrap();
			inst.start_auction(1, ISSUER_1).unwrap();
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
				);
			});
			inst.start_community_funding(1).unwrap_err();
			inst.advance_time(1).unwrap();
			assert_eq!(inst.get_project_details(1).status, ProjectStatus::FundingFailed);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// A Project is "inactive" after the funding fails
			inst.start_evaluation(2, ISSUER_1).unwrap();
			inst.evaluate_for_users(2, default_evaluations()).unwrap();
			inst.start_auction(2, ISSUER_1).unwrap();
			inst.bid_for_users(2, failing_bids).unwrap();
			inst.start_community_funding(2).unwrap();
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
				);
			});
			inst.finish_funding(2).unwrap();
			assert_eq!(inst.get_project_details(2).status, ProjectStatus::FundingFailed);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// A project is "inactive" after the funding succeeds
			inst.start_evaluation(3, ISSUER_1).unwrap();
			inst.evaluate_for_users(3, default_evaluations()).unwrap();
			inst.start_auction(3, ISSUER_1).unwrap();
			inst.bid_for_users(3, default_bids()).unwrap();
			inst.start_community_funding(3).unwrap();
			inst.contribute_for_users(3, default_community_buys()).unwrap();
			inst.start_remainder_or_end_funding(3).unwrap();
			inst.contribute_for_users(3, default_remainder_buys()).unwrap();
			inst.finish_funding(3).unwrap();
			assert_eq!(inst.get_project_details(3).status, ProjectStatus::FundingSuccessful);
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

			// funding target of 1000 USD at 1quadrillion supply
			const QUADRILLION_SUPPLY: u128 = 1_000_000_000_000_000 * ASSET_UNIT;
			const LOW_PRICE: f64 = 0.000_000_000_001f64;

			project_metadata.mainnet_token_max_supply = QUADRILLION_SUPPLY;
			project_metadata.total_allocation_size = QUADRILLION_SUPPLY;
			project_metadata.minimum_price = FixedU128::from_float(LOW_PRICE);

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
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
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Retail, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_metadata.clone()
					),
					Error::<TestRuntime>::NotAllowed
				);
			});

			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Professional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_metadata
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		#[test]
		fn did_cannot_have_2_active_projects() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let ed = MockInstantiator::get_ed();
			let issuer_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed * 2).into();
			// Create a first project
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.mint_plmc_to(vec![issuer_mint.clone()]);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::create_project(
					RuntimeOrigin::signed(ISSUER_1),
					jwt.clone(),
					project_metadata.clone()
				));
			});

			// different account, same did
			let jwt = get_mock_jwt(ISSUER_2, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
				);
			});
		}

		#[test]
		fn not_enough_plmc_for_escrow_ed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let ed = MockInstantiator::get_ed();
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(ISSUER_1, ed)]);
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata,),
					Error::<TestRuntime>::NotEnoughFundsForEscrowCreation
				);
			});
		}

		// Invalid metadata tests:
		#[test]
		fn mainnet_supply_less_than_allocation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000_001 * ASSET_UNIT;
			project_metadata.mainnet_token_max_supply = 100_000_000 * ASSET_UNIT;
			inst.mint_plmc_to(default_plmc_balances());
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_create_project(
						&ISSUER_1,
						project_metadata,
						generate_did_from_account(ISSUER_1),
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::AllocationSizeError)
				);
			});
		}

		#[test]
		fn invalid_ticket_sizes() {
			let mut correct_project = default_project_metadata(ISSUER_1);

			// min in bidding below 5k
			let mut wrong_project_1 = correct_project.clone();
			wrong_project_1.bidding_ticket_sizes.professional = TicketSize::new(Some(4999 * US_DOLLAR), None);

			let mut wrong_project_2 = correct_project.clone();
			wrong_project_2.bidding_ticket_sizes.institutional = TicketSize::new(Some(4999 * US_DOLLAR), None);

			let mut wrong_project_3 = correct_project.clone();
			wrong_project_3.bidding_ticket_sizes.professional = TicketSize::new(Some(3000 * US_DOLLAR), None);
			wrong_project_3.bidding_ticket_sizes.institutional = TicketSize::new(Some(0 * US_DOLLAR), None);

			let mut wrong_project_4 = correct_project.clone();
			wrong_project_4.bidding_ticket_sizes.professional = TicketSize::new(None, None);
			wrong_project_4.bidding_ticket_sizes.institutional = TicketSize::new(None, None);

			// min higher than max
			let mut wrong_project_5 = correct_project.clone();
			wrong_project_5.bidding_ticket_sizes.professional =
				TicketSize::new(Some(5000 * US_DOLLAR), Some(4990 * US_DOLLAR));

			let mut wrong_project_6 = correct_project.clone();
			wrong_project_6.bidding_ticket_sizes.institutional =
				TicketSize::new(Some(6000 * US_DOLLAR), Some(5500 * US_DOLLAR));

			let wrong_projects = vec![
				wrong_project_1.clone(),
				wrong_project_2,
				wrong_project_3.clone(),
				wrong_project_4,
				wrong_project_5,
				wrong_project_6,
			];

			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			inst.mint_plmc_to(default_plmc_balances());

			for project in wrong_projects {
				let project_err = inst.execute(|| {
					Pallet::<TestRuntime>::do_create_project(&ISSUER_1, project, generate_did_from_account(ISSUER_1))
						.unwrap_err()
				});
				assert_eq!(project_err, Error::<TestRuntime>::BadMetadata(MetadataError::TicketSizeError).into());
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
				assert_eq!(
					project_err,
					Error::<TestRuntime>::BadMetadata(MetadataError::ParticipationCurrenciesError).into()
				);
			}
		}

		#[test]
		fn price_zero() {
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = 0_u128.into();

			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			inst.mint_plmc_to(default_plmc_balances());
			let project_err = inst.execute(|| {
				Pallet::<TestRuntime>::do_create_project(&ISSUER_1, project_metadata, generate_did_from_account(ISSUER_1))
					.unwrap_err()
			});
			assert_eq!(project_err, Error::<TestRuntime>::BadMetadata(MetadataError::PriceTooLow).into());
		}

		#[test]
		fn allocation_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 0;

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::BadMetadata(MetadataError::AllocationSizeError)
				);
			});
		}

		#[test]
		fn auction_round_percentage_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(0);

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::BadMetadata(MetadataError::AuctionRoundPercentageError)
				);
			});
		}

		#[test]
		fn target_funding_less_than_1000_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(1.0);
			project_metadata.total_allocation_size = 999u128;

			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_metadata.clone()
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::FundingTargetTooLow)
				);
			});

			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(0.00000000001);
			project_metadata.total_allocation_size = 99999999999999u128;
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::create_project(RuntimeOrigin::signed(ISSUER_1), jwt, project_metadata),
					Error::<TestRuntime>::BadMetadata(MetadataError::FundingTargetTooLow)
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
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(15.0);
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				project_metadata.clone()
			)));
			let next_project_id = inst.execute(|| NextProjectId::<TestRuntime>::get());
			assert_eq!(project_id, next_project_id - 1);
			let projects_details = inst.execute(|| ProjectsDetails::<TestRuntime>::iter_keys().collect_vec());
			let project_metadatas = inst.execute(|| ProjectsMetadata::<TestRuntime>::iter_keys().collect_vec());
			assert_eq!(projects_details, vec![project_id]);
			assert_eq!(project_metadatas, vec![project_id]);
		}

		#[test]
		fn multiple_fields_edited() {
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
					retail: TicketSize::new(None, None),
					professional: TicketSize::new(None, None),
					institutional: TicketSize::new(None, None),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
				funding_destination_account: ISSUER_1,
				offchain_information_hash: None,
			};
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			let mut new_metadata_1 = project_metadata.clone();
			new_metadata_1.minimum_price = PriceOf::<TestRuntime>::from_float(15.0);
			let new_metadata_2 = ProjectMetadataOf::<TestRuntime> {
				token_information: CurrencyMetadata {
					name: BoundedVec::try_from("Changed Name".as_bytes().to_vec()).unwrap(),
					symbol: BoundedVec::try_from("CN".as_bytes().to_vec()).unwrap(),
					decimals: 12,
				},
				mainnet_token_max_supply: 100_000_000 * ASSET_UNIT,
				total_allocation_size: 50_000_000 * ASSET_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(30u8),
				minimum_price: PriceOf::<TestRuntime>::from_float(20.0),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(Some(10_000 * US_DOLLAR), Some(20_000 * US_DOLLAR)),
					institutional: TicketSize::new(Some(20_000 * US_DOLLAR), Some(30_000 * US_DOLLAR)),
					phantom: Default::default(),
				},
				contributing_ticket_sizes: ContributingTicketSizes {
					retail: TicketSize::new(Some(1_000 * US_DOLLAR), Some(2_000 * US_DOLLAR)),
					professional: TicketSize::new(Some(2_000 * US_DOLLAR), Some(3_000 * US_DOLLAR)),
					institutional: TicketSize::new(Some(3_000 * US_DOLLAR), Some(4_000 * US_DOLLAR)),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC]
					.try_into()
					.unwrap(),

				funding_destination_account: ISSUER_2,
				offchain_information_hash: Some(hashed(METADATA)),
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
		fn adding_offchain_hash() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.offchain_information_hash = None;
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			let mut new_metadata = project_metadata.clone();
			new_metadata.offchain_information_hash = Some(hashed(METADATA));
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
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			let mut new_metadata = project_metadata.clone();
			new_metadata.total_allocation_size = 100_000 * ASSET_UNIT;
			new_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(1f64);
			assert_ok!(inst.execute(|| crate::Pallet::<TestRuntime>::edit_project(
				RuntimeOrigin::signed(ISSUER_1),
				jwt.clone(),
				project_id,
				new_metadata.clone()
			)));
			// Project details reflect changes
			assert_eq!(inst.get_project_details(project_id).fundraising_target, 100_000 * US_DOLLAR);
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
			let ed = MockInstantiator::get_ed();
			let issuer_1_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed).into();
			let issuer_2_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_2, ed).into();

			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			inst.mint_plmc_to(vec![issuer_1_mint.clone(), issuer_2_mint.clone()]);

			let jwt_1 = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let jwt_2 = get_mock_jwt(ISSUER_2, InvestorType::Institutional, generate_did_from_account(ISSUER_2));

			let project_id_1 = inst.create_new_project(project_metadata_1.clone(), ISSUER_1);
			let project_id_2 = inst.create_new_project(project_metadata_2.clone(), ISSUER_2);

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt_2,
						project_id_1,
						project_metadata_2
					),
					Error::<TestRuntime>::NotAllowed
				);
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt_1,
						project_id_2,
						project_metadata_1
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		#[test]
		fn evaluation_already_started() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			inst.start_evaluation(project_id, ISSUER_1).unwrap();
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id,
						project_metadata.clone()
					),
					Error::<TestRuntime>::Frozen
				);
			});
		}

		#[test]
		fn non_institutional_credential() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Retail, generate_did_from_account(ISSUER_1));

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_id,
						project_metadata.clone()
					),
					Error::<TestRuntime>::NotAllowed
				);
			});

			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Professional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		// Same tests for create_project extrinsic
		#[test]
		fn mainnet_supply_less_than_allocation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.mainnet_token_max_supply = 100_000_000 * ASSET_UNIT;
			project_metadata.total_allocation_size = 100_000_001 * ASSET_UNIT;
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1)),
						project_id,
						project_metadata,
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::AllocationSizeError)
				);
			});
		}

		#[test]
		fn invalid_ticket_sizes() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			// min in bidding below 5k
			let mut wrong_project_1 = project_metadata.clone();
			wrong_project_1.bidding_ticket_sizes.professional = TicketSize::new(Some(4999 * US_DOLLAR), None);

			let mut wrong_project_2 = project_metadata.clone();
			wrong_project_2.bidding_ticket_sizes.institutional = TicketSize::new(Some(4999 * US_DOLLAR), None);

			let mut wrong_project_3 = project_metadata.clone();
			wrong_project_3.bidding_ticket_sizes.professional = TicketSize::new(Some(3000 * US_DOLLAR), None);
			wrong_project_3.bidding_ticket_sizes.institutional = TicketSize::new(Some(0 * US_DOLLAR), None);

			let mut wrong_project_4 = project_metadata.clone();
			wrong_project_4.bidding_ticket_sizes.professional = TicketSize::new(None, None);
			wrong_project_4.bidding_ticket_sizes.institutional = TicketSize::new(None, None);

			// min higher than max
			let mut wrong_project_5 = project_metadata.clone();
			wrong_project_5.bidding_ticket_sizes.professional =
				TicketSize::new(Some(5000 * US_DOLLAR), Some(4990 * US_DOLLAR));

			let mut wrong_project_6 = project_metadata.clone();
			wrong_project_6.bidding_ticket_sizes.institutional =
				TicketSize::new(Some(6000 * US_DOLLAR), Some(5500 * US_DOLLAR));

			let wrong_projects = vec![
				wrong_project_1.clone(),
				wrong_project_2,
				wrong_project_3.clone(),
				wrong_project_4,
				wrong_project_5,
				wrong_project_6,
			];

			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			for project in wrong_projects {
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::edit_project(
							RuntimeOrigin::signed(ISSUER_1),
							jwt.clone(),
							project_id,
							project,
						),
						Error::<TestRuntime>::BadMetadata(MetadataError::TicketSizeError)
					);
				});
			}
		}

		#[test]
		fn duplicated_participation_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDT].try_into().unwrap();
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1)),
						project_id,
						project_metadata,
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::ParticipationCurrenciesError)
				);
			});
		}

		#[test]
		fn price_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(0.0);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1)),
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::PriceTooLow)
				);
			});
		}

		#[test]
		fn allocation_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.total_allocation_size = 0;

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1)),
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::AllocationSizeError)
				);
			});
		}

		#[test]
		fn auction_round_percentage_zero() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.auction_round_allocation_percentage = Percent::from_percent(0);

			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1)),
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::AuctionRoundPercentageError)
				);
			});
		}

		#[test]
		fn target_funding_less_than_1000_usd() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(1.0);
			project_metadata.total_allocation_size = 999u128;

			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id,
						project_metadata.clone()
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::FundingTargetTooLow)
				);
			});

			project_metadata.minimum_price = PriceOf::<TestRuntime>::from_float(0.00000000001);
			project_metadata.total_allocation_size = 99999999999999u128;
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::edit_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt,
						project_id,
						project_metadata
					),
					Error::<TestRuntime>::BadMetadata(MetadataError::FundingTargetTooLow)
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
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
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
			let ed = MockInstantiator::get_ed();
			let issuer_mint: UserToPLMCBalance<TestRuntime> = (ISSUER_1, ed * 2).into();
			// Create a first project
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
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
					Error::<TestRuntime>::IssuerHasActiveProjectAlready
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
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Professional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		#[test]
		fn different_account() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_2, InvestorType::Institutional, generate_did_from_account(ISSUER_2));

			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);

			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_2),
						jwt.clone(),
						project_id
					),
					Error::<TestRuntime>::NotAllowed
				);
			});
		}

		#[test]
		fn evaluation_already_started() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			inst.mint_plmc_to(default_plmc_balances());
			let jwt = get_mock_jwt(ISSUER_1, InvestorType::Institutional, generate_did_from_account(ISSUER_1));
			let project_id = inst.create_new_project(project_metadata.clone(), ISSUER_1);
			inst.start_evaluation(project_id, ISSUER_1).unwrap();
			inst.execute(|| {
				assert_noop!(
					crate::Pallet::<TestRuntime>::remove_project(
						RuntimeOrigin::signed(ISSUER_1),
						jwt.clone(),
						project_id,
					),
					Error::<TestRuntime>::Frozen
				);
			});
		}
	}
}