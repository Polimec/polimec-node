use super::*;
use crate::instantiator::async_features::create_multiple_projects_at;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn remainder_round_works() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let _ = inst.create_finished_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
				default_remainder_buys(),
			);
		}

		#[test]
		fn remainder_round_ends_on_all_ct_sold_exact() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_id = inst.create_remainder_contributing_project(
				default_project_metadata(ISSUER_1),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
			);
			const BOB: AccountId = 808;

			let remaining_ct = inst.get_project_details(project_id).remaining_contribution_tokens;
			let ct_price = inst.get_project_details(project_id).weighted_average_price.expect("CT Price should exist");

			let contributions = vec![ContributionParams::new(BOB, remaining_ct, 1u8, AcceptedFundingAsset::USDT)];
			let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(contributions.clone(), ct_price);
			let plmc_existential_deposits = contributions.accounts().existential_deposits();
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
			assert_eq!(
				inst.get_project_details(project_id).remaining_contribution_tokens,
				0,
				"There are still remaining CTs"
			);

			// Check project is in FundingEnded state
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);

			inst.do_free_plmc_assertions(plmc_existential_deposits);
			inst.do_free_foreign_asset_assertions(vec![UserToForeignAssets::<TestRuntime>::new(
				BOB,
				0_u128,
				AcceptedFundingAsset::USDT.to_assethub_id(),
			)]);
			inst.do_reserved_plmc_assertions(
				vec![plmc_fundings[0].clone()],
				HoldReason::Participation(project_id).into(),
			);
			inst.do_contribution_transferred_foreign_asset_assertions(foreign_asset_fundings, project_id);
		}

		#[test]
		fn round_has_total_ct_allocation_minus_auction_sold() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let bids = default_bids();

			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				evaluations.clone(),
				bids.clone(),
				vec![],
			);
			let project_details = inst.get_project_details(project_id);
			let bid_ct_sold: BalanceOf<TestRuntime> = inst.execute(|| {
				Bids::<TestRuntime>::iter_prefix_values((project_id,))
					.fold(Zero::zero(), |acc, bid| acc + bid.final_ct_amount)
			});
			assert_eq!(
				project_details.remaining_contribution_tokens,
				project_metadata.total_allocation_size - bid_ct_sold
			);

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
	}
}

#[cfg(test)]
mod remaining_contribute_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn remainder_contributor_was_evaluator() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = default_evaluations();
			let community_contributions = default_community_buys();
			let evaluator_contributor = 69;
			let evaluation_amount = 420 * US_DOLLAR;
			let remainder_contribution =
				ContributionParams::new(evaluator_contributor, 600 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			evaluations.push(UserToUSDBalance::new(evaluator_contributor, evaluation_amount));
			let bids = default_bids();

			let project_id = inst.create_remainder_contributing_project(
				project_metadata,
				issuer,
				evaluations,
				bids,
				community_contributions,
			);
			let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let already_bonded_plmc = MockInstantiator::calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(
				evaluator_contributor,
				evaluation_amount,
			)])[0]
				.plmc_amount;
			let plmc_available_for_contribution =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_buy =
				MockInstantiator::calculate_contributed_plmc_spent(vec![remainder_contribution.clone()], ct_price)[0]
					.plmc_amount;
			let necessary_usdt_for_buy = MockInstantiator::calculate_contributed_funding_asset_spent(
				vec![remainder_contribution.clone()],
				ct_price,
			);

			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_contributor,
				necessary_plmc_for_buy - plmc_available_for_contribution,
			)]);
			inst.mint_foreign_asset_to(necessary_usdt_for_buy);

			inst.contribute_for_users(project_id, vec![remainder_contribution]).unwrap();
		}

		#[test]
		fn contribute_with_multiple_currencies() {
			let mut project_metadata_all = default_project_metadata(ISSUER_1);
			project_metadata_all.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.try_into()
					.unwrap();

			let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
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
					expected_state: ProjectStatus::RemainderRound,
					metadata: project_metadata_all.clone(),
					issuer: ISSUER_1,
					evaluations: evaluations.clone(),
					bids: usdt_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::RemainderRound,
					metadata: project_metadata_usdt,
					issuer: ISSUER_2,
					evaluations: evaluations.clone(),
					bids: usdt_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::RemainderRound,
					metadata: project_metadata_usdc,
					issuer: ISSUER_3,
					evaluations: evaluations.clone(),
					bids: usdc_bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams {
					expected_state: ProjectStatus::RemainderRound,
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

			let usdt_contribution =
				ContributionParams::new(BUYER_1, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let usdc_contribution =
				ContributionParams::new(BUYER_2, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::USDC);
			let dot_contribution =
				ContributionParams::new(BUYER_3, 10_000 * ASSET_UNIT, 1u8, AcceptedFundingAsset::DOT);

			let wap = inst.get_project_details(project_id_all).weighted_average_price.unwrap();

			let plmc_fundings = MockInstantiator::calculate_contributed_plmc_spent(
				vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
				wap,
			);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();

			let plmc_all_mints = MockInstantiator::generic_map_operation(
				vec![plmc_fundings, plmc_existential_deposits],
				MergeOperation::Add,
			);
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
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.contribute_for_users(project_id_usdt, vec![dot_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);

			assert_err!(
				inst.contribute_for_users(project_id_usdc, vec![usdt_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_ok!(inst.contribute_for_users(project_id_usdc, vec![usdc_contribution.clone()]));
			assert_err!(
				inst.contribute_for_users(project_id_usdc, vec![dot_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);

			assert_err!(
				inst.contribute_for_users(project_id_dot, vec![usdt_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_err!(
				inst.contribute_for_users(project_id_dot, vec![usdc_contribution.clone()]),
				Error::<TestRuntime>::ParticipationFailed(ParticipationError::FundingAssetNotAccepted)
			);
			assert_ok!(inst.contribute_for_users(project_id_dot, vec![dot_contribution.clone()]));
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
				policy_ipfs_cid: Some(ipfs_hash()),
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
			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				evaluations,
				bids,
				vec![],
			);
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			// Professional bids: 0x multiplier should fail
			let jwt = get_mock_jwt(BUYER_1, InvestorType::Professional, generate_did_from_account(BUYER_1));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt,
						project_id,
						1000 * ASSET_UNIT,
						Multiplier::force_new(0),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
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
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::remaining_contribute(
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
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_1),
							jwt,
							project_id,
							1000 * ASSET_UNIT,
							Multiplier::force_new(multiplier),
							AcceptedFundingAsset::USDT
						),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
					);
				});
			}

			// Institutional bids: 0x multiplier should fail
			let jwt = get_mock_jwt(BUYER_2, InvestorType::Institutional, generate_did_from_account(BUYER_2));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_2),
						jwt,
						project_id,
						1000 * ASSET_UNIT,
						Multiplier::force_new(0),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
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
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::remaining_contribute(
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
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_2),
							jwt,
							project_id,
							1000 * ASSET_UNIT,
							Multiplier::force_new(multiplier),
							AcceptedFundingAsset::USDT
						),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
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
				inst.create_remainder_contributing_project(
					default_project_metadata(issuer),
					issuer,
					default_evaluations(),
					default_bids(),
					vec![],
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
					Pallet::<TestRuntime>::remaining_contribute(
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
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_1),
							get_mock_jwt(BUYER_1, InvestorType::Retail, generate_did_from_account(BUYER_1)),
							project_id,
							1000 * ASSET_UNIT,
							Multiplier::force_new(0),
							AcceptedFundingAsset::USDT
						),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
					);
				});

				// Multipliers that should work
				for multiplier in 1..=max_allowed_multiplier {
					log::debug!("success? - multiplier: {}", multiplier);
					assert_ok!(contribute(&mut inst, project_id, multiplier));
				}

				// Multipliers that should NOT work
				for multiplier in max_allowed_multiplier + 1..=50 {
					log::debug!("error? - multiplier: {}", multiplier);
					assert_err!(
						contribute(&mut inst, project_id, multiplier),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
					);
				}
			}
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn issuer_cannot_contribute_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				default_community_buys(),
			);
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_remaining_contribute(
					&(&ISSUER_1 + 1),
					project_id,
					500 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional
				)),
				Error::<TestRuntime>::IssuerError(IssuerErrorReason::ParticipationToOwnProject)
			);
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
					professional: TicketSize::new(Some(8000 * US_DOLLAR), None),
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
				policy_ipfs_cid: Some(ipfs_hash()),
			};

			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				vec![],
			);

			inst.mint_plmc_to(vec![
				(BUYER_4, 50_000 * ASSET_UNIT).into(),
				(BUYER_5, 50_000 * ASSET_UNIT).into(),
				(BUYER_6, 50_000 * ASSET_UNIT).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_4, 50_000 * US_DOLLAR).into(),
				(BUYER_5, 50_000 * US_DOLLAR).into(),
				(BUYER_6, 50_000 * US_DOLLAR).into(),
			]);

			// contribution below 1 CT (10 USD) should fail for retail
			let jwt = get_mock_jwt(BUYER_4, InvestorType::Retail, generate_did_from_account(BUYER_4));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_4),
						jwt,
						project_id,
						ASSET_UNIT / 2,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
			// contribution below 10_000 CT (100k USD) should fail for professionals
			let jwt = get_mock_jwt(BUYER_5, InvestorType::Professional, generate_did_from_account(BUYER_5));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_5),
						jwt,
						project_id,
						9_999,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});

			// contribution below 20_000 CT (200k USD) should fail for institutionals
			let jwt = get_mock_jwt(BUYER_6, InvestorType::Institutional, generate_did_from_account(BUYER_6));
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_6),
						jwt,
						project_id,
						19_999,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
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
					retail: TicketSize::new(None, Some(300_000 * US_DOLLAR)),
					professional: TicketSize::new(None, Some(20_000 * US_DOLLAR)),
					institutional: TicketSize::new(None, Some(50_000 * US_DOLLAR)),
					phantom: Default::default(),
				},
				participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
				funding_destination_account: ISSUER_1,
				policy_ipfs_cid: Some(ipfs_hash()),
			};

			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				vec![],
			);

			inst.mint_plmc_to(vec![
				(BUYER_4, 500_000 * ASSET_UNIT).into(),
				(BUYER_5, 500_000 * ASSET_UNIT).into(),
				(BUYER_6, 500_000 * ASSET_UNIT).into(),
				(BUYER_7, 500_000 * ASSET_UNIT).into(),
				(BUYER_8, 500_000 * ASSET_UNIT).into(),
				(BUYER_9, 500_000 * ASSET_UNIT).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_4, 500_000 * US_DOLLAR).into(),
				(BUYER_5, 500_000 * US_DOLLAR).into(),
				(BUYER_6, 500_000 * US_DOLLAR).into(),
				(BUYER_7, 500_000 * US_DOLLAR).into(),
				(BUYER_8, 500_000 * US_DOLLAR).into(),
				(BUYER_9, 500_000 * US_DOLLAR).into(),
			]);

			// total contributions with same DID above 30k CT (300k USD) should fail for retail
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_4,
					project_id,
					28_000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_4),
					InvestorType::Retail
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_5,
						project_id,
						2001 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 1, on a different account
						generate_did_from_account(BUYER_4),
						InvestorType::Retail
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_5,
					project_id,
					2000 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_4),
					InvestorType::Retail
				));
			});

			// total contributions with same DID above 2k CT (20k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_6,
					project_id,
					1800 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_6),
					InvestorType::Professional
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_7,
						project_id,
						201 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 1, on a different account
						generate_did_from_account(BUYER_6),
						InvestorType::Professional
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_7,
					project_id,
					200 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_6),
					InvestorType::Professional
				));
			});

			// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_8,
					project_id,
					4690 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_8),
					InvestorType::Institutional
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_9,
						project_id,
						311 * ASSET_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 3, on a different account
						generate_did_from_account(BUYER_8),
						InvestorType::Institutional
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 5k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_9,
					project_id,
					310 * ASSET_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 3, on a different account
					generate_did_from_account(BUYER_8),
					InvestorType::Institutional
				));
			});
		}
	}
}
