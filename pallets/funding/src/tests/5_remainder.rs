use super::*;
use crate::instantiator::async_features::create_multiple_projects_at;
use frame_support::traits::fungibles::metadata::Inspect;
use sp_runtime::bounded_vec;
use std::collections::HashSet;

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
			let plmc_fundings = inst.calculate_contributed_plmc_spent(contributions.clone(), ct_price);
			let plmc_existential_deposits = contributions.accounts().existential_deposits();
			let foreign_asset_fundings =
				inst.calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);

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

			let plmc_contribution_funding = inst.calculate_contributed_plmc_spent(
				contributions.clone(),
				project_details.weighted_average_price.unwrap(),
			);
			let plmc_existential_deposits = plmc_contribution_funding.accounts().existential_deposits();
			inst.mint_plmc_to(plmc_contribution_funding.clone());
			inst.mint_plmc_to(plmc_existential_deposits.clone());

			let foreign_asset_contribution_funding = inst.calculate_contributed_funding_asset_spent(
				contributions.clone(),
				project_details.weighted_average_price.unwrap(),
			);
			inst.mint_foreign_asset_to(foreign_asset_contribution_funding.clone());

			inst.contribute_for_users(project_id, contributions).unwrap();

			assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);
		}

		#[test]
		fn different_decimals_ct_works_as_expected() {
			// Setup some base values to compare different decimals
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let ed = inst.get_ed();
			let default_project_metadata = default_project_metadata(ISSUER_1);
			let original_decimal_aware_price = default_project_metadata.minimum_price;
			let original_price = <TestRuntime as Config>::PriceProvider::convert_back_to_normal_price(
				original_decimal_aware_price,
				USD_DECIMALS,
				default_project_metadata.token_information.decimals,
			)
			.unwrap();
			let usable_plmc_price = inst.execute(|| {
				<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(
					PLMC_FOREIGN_ID,
					USD_DECIMALS,
					PLMC_DECIMALS,
				)
				.unwrap()
			});
			let usdt_price = inst.execute(|| {
				<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(
					AcceptedFundingAsset::USDT.to_assethub_id(),
					USD_DECIMALS,
					ForeignAssets::decimals(AcceptedFundingAsset::USDT.to_assethub_id()),
				)
				.unwrap()
			});
			let usdc_price = inst.execute(|| {
				<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(
					AcceptedFundingAsset::USDC.to_assethub_id(),
					USD_DECIMALS,
					ForeignAssets::decimals(AcceptedFundingAsset::USDC.to_assethub_id()),
				)
				.unwrap()
			});
			let dot_price = inst.execute(|| {
				<TestRuntime as Config>::PriceProvider::get_decimals_aware_price(
					AcceptedFundingAsset::DOT.to_assethub_id(),
					USD_DECIMALS,
					ForeignAssets::decimals(AcceptedFundingAsset::DOT.to_assethub_id()),
				)
				.unwrap()
			});

			let mut funding_assets_cycle =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.into_iter()
					.cycle();

			let mut total_fundings_ct = Vec::new();
			let mut total_fundings_usd = Vec::new();
			let mut total_fundings_plmc = Vec::new();

			let mut decimal_test = |decimals: u8| {
				let funding_asset = funding_assets_cycle.next().unwrap();
				let funding_asset_usd_price = match funding_asset {
					AcceptedFundingAsset::USDT => usdt_price,
					AcceptedFundingAsset::USDC => usdc_price,
					AcceptedFundingAsset::DOT => dot_price,
				};

				let mut project_metadata = default_project_metadata.clone();
				project_metadata.token_information.decimals = decimals;
				project_metadata.minimum_price =
					<TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
						original_price,
						USD_DECIMALS,
						decimals,
					)
					.unwrap();

				project_metadata.total_allocation_size = 1_000_000 * 10u128.pow(decimals as u32);
				project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
				project_metadata.participation_currencies = bounded_vec!(funding_asset);

				let issuer: AccountIdOf<TestRuntime> = (10_000 + inst.get_new_nonce()).try_into().unwrap();
				let evaluations = inst.generate_successful_evaluations(
					project_metadata.clone(),
					default_evaluators(),
					default_weights(),
				);
				let project_id = inst.create_remainder_contributing_project(
					project_metadata.clone(),
					issuer,
					evaluations,
					vec![],
					vec![],
				);

				let total_funding_ct = project_metadata.total_allocation_size;
				let total_funding_usd = project_metadata.minimum_price.saturating_mul_int(total_funding_ct);
				let total_funding_plmc = usable_plmc_price.reciprocal().unwrap().saturating_mul_int(total_funding_usd);
				let total_funding_funding_asset =
					funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(total_funding_usd);

				total_fundings_ct.push(total_funding_ct);
				total_fundings_usd.push(total_funding_usd);
				total_fundings_plmc.push(total_funding_plmc);

				// Every project should want to raise 10MM USD
				assert_eq!(total_funding_usd, 10_000_000 * USD_UNIT);

				// Every project should produce the same PLMC bond when having the full funding at multiplier 1.
				assert_close_enough!(total_funding_plmc, 1_190_476 * PLMC, Perquintill::from_float(0.999));

				// Every project should have a different amount of CTs to raise, depending on their decimals
				assert_eq!(total_funding_ct, 1_000_000 * 10u128.pow(decimals as u32));

				// Buying all the remaining tokens. This is a fixed USD value, but the extrinsic amount depends on CT decimals.
				inst.mint_plmc_to(vec![UserToPLMCBalance::new(BUYER_1, total_funding_plmc + ed)]);
				inst.mint_foreign_asset_to(vec![UserToForeignAssets::new(
					BUYER_1,
					total_funding_funding_asset,
					funding_asset.to_assethub_id(),
				)]);

				assert_ok!(inst.execute(|| PolimecFunding::remaining_contribute(
					RuntimeOrigin::signed(BUYER_1),
					get_mock_jwt_with_cid(
						BUYER_1,
						InvestorType::Retail,
						generate_did_from_account(BUYER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					total_funding_ct,
					1u8.try_into().unwrap(),
					funding_asset,
				)));

				// the remaining tokens should be zero
				assert_eq!(inst.get_project_details(project_id).remaining_contribution_tokens, 0);

				// We can successfully finish the project
				inst.finish_funding(project_id).unwrap();
			};

			for decimals in 6..=18 {
				decimal_test(decimals);
			}

			// Since we use the same original price and allocation size and adjust for decimals,
			// the USD and PLMC amounts should be the same
			assert!(total_fundings_usd.iter().all(|x| *x == total_fundings_usd[0]));
			assert!(total_fundings_plmc.iter().all(|x| *x == total_fundings_plmc[0]));

			// CT amounts however should be different from each other
			let mut hash_set_1 = HashSet::new();
			for amount in total_fundings_ct {
				assert!(!hash_set_1.contains(&amount));
				hash_set_1.insert(amount);
			}
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
			let evaluation_amount = 420 * USD_UNIT;
			let remainder_contribution =
				ContributionParams::new(evaluator_contributor, 600 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT);
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
			let already_bonded_plmc = inst
				.calculate_evaluation_plmc_spent(vec![UserToUSDBalance::new(evaluator_contributor, evaluation_amount)])[0]
				.plmc_amount;
			let plmc_available_for_contribution =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_buy =
				inst.calculate_contributed_plmc_spent(vec![remainder_contribution.clone()], ct_price)[0].plmc_amount;
			let necessary_usdt_for_buy =
				inst.calculate_contributed_funding_asset_spent(vec![remainder_contribution.clone()], ct_price);

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

			let usdt_contribution = ContributionParams::new(BUYER_1, 10_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT);
			let usdc_contribution = ContributionParams::new(BUYER_2, 10_000 * CT_UNIT, 1u8, AcceptedFundingAsset::USDC);
			let dot_contribution = ContributionParams::new(BUYER_3, 10_000 * CT_UNIT, 1u8, AcceptedFundingAsset::DOT);

			let wap = inst.get_project_details(project_id_all).weighted_average_price.unwrap();

			let plmc_fundings = inst.calculate_contributed_plmc_spent(
				vec![usdt_contribution.clone(), usdc_contribution.clone(), dot_contribution.clone()],
				wap,
			);
			let plmc_existential_deposits = plmc_fundings.accounts().existential_deposits();

			let plmc_all_mints =
				inst.generic_map_operation(vec![plmc_fundings, plmc_existential_deposits], MergeOperation::Add);
			inst.mint_plmc_to(plmc_all_mints.clone());
			inst.mint_plmc_to(plmc_all_mints.clone());
			inst.mint_plmc_to(plmc_all_mints.clone());

			let usdt_fundings = inst.calculate_contributed_funding_asset_spent(
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
				mainnet_token_max_supply: 100_000 * CT_UNIT,
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
			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
			let bids = inst.generate_bids_from_total_ct_percent(
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
			let jwt = get_mock_jwt_with_cid(
				BUYER_1,
				InvestorType::Professional,
				generate_did_from_account(BUYER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt,
						project_id,
						1000 * CT_UNIT,
						Multiplier::force_new(0),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			});
			// Professional bids: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				let jwt = get_mock_jwt_with_cid(
					BUYER_1,
					InvestorType::Professional,
					generate_did_from_account(BUYER_1),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);
				let bidder_plmc = inst.calculate_contributed_plmc_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let bidder_usdt = inst.calculate_contributed_funding_asset_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let ed = inst.get_ed();
				inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::remaining_contribute(
					RuntimeOrigin::signed(BUYER_1),
					jwt,
					project_id,
					1000 * CT_UNIT,
					Multiplier::force_new(multiplier),
					AcceptedFundingAsset::USDT
				)));
			}
			// Professional bids: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				let jwt = get_mock_jwt_with_cid(
					BUYER_1,
					InvestorType::Professional,
					generate_did_from_account(BUYER_1),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);
				let bidder_plmc = inst.calculate_contributed_plmc_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let bidder_usdt = inst.calculate_contributed_funding_asset_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let ed = inst.get_ed();
				inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_1),
							jwt,
							project_id,
							1000 * CT_UNIT,
							Multiplier::force_new(multiplier),
							AcceptedFundingAsset::USDT
						),
						Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
					);
				});
			}

			// Institutional bids: 0x multiplier should fail
			let jwt = get_mock_jwt_with_cid(
				BUYER_2,
				InvestorType::Institutional,
				generate_did_from_account(BUYER_2),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_2),
						jwt,
						project_id,
						1000 * CT_UNIT,
						Multiplier::force_new(0),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::ForbiddenMultiplier)
				);
			});
			// Institutional bids: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				let jwt = get_mock_jwt_with_cid(
					BUYER_2,
					InvestorType::Institutional,
					generate_did_from_account(BUYER_2),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);
				let bidder_plmc = inst.calculate_contributed_plmc_spent(
					vec![(BUYER_2, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let bidder_usdt = inst.calculate_contributed_funding_asset_spent(
					vec![(BUYER_2, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let ed = inst.get_ed();
				inst.mint_plmc_to(vec![(BUYER_2, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				assert_ok!(inst.execute(|| Pallet::<TestRuntime>::remaining_contribute(
					RuntimeOrigin::signed(BUYER_2),
					jwt,
					project_id,
					1000 * CT_UNIT,
					multiplier.try_into().unwrap(),
					AcceptedFundingAsset::USDT
				)));
			}
			// Institutional bids: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				let jwt = get_mock_jwt_with_cid(
					BUYER_2,
					InvestorType::Institutional,
					generate_did_from_account(BUYER_2),
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				);
				let bidder_plmc = inst.calculate_contributed_plmc_spent(
					vec![(BUYER_2, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let bidder_usdt = inst.calculate_contributed_funding_asset_spent(
					vec![(BUYER_2, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let ed = inst.get_ed();
				inst.mint_plmc_to(vec![(BUYER_2, ed).into()]);
				inst.mint_plmc_to(bidder_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_2),
							jwt,
							project_id,
							1000 * CT_UNIT,
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
				let project_policy = inst.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
				let jwt = get_mock_jwt_with_cid(
					BUYER_1,
					InvestorType::Retail,
					generate_did_from_account(BUYER_1),
					project_policy,
				);
				let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
				let contributor_plmc = inst.calculate_contributed_plmc_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let bidder_usdt = inst.calculate_contributed_funding_asset_spent(
					vec![(BUYER_1, 1_000 * CT_UNIT, Multiplier::force_new(multiplier)).into()],
					wap,
				);
				let ed = inst.get_ed();
				inst.mint_plmc_to(vec![(BUYER_1, ed).into()]);
				inst.mint_plmc_to(contributor_plmc);
				inst.mint_foreign_asset_to(bidder_usdt);
				inst.execute(|| {
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_1),
						jwt,
						project_id,
						1000 * CT_UNIT,
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
				let project_policy = inst.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
				inst.execute(|| {
					assert_noop!(
						Pallet::<TestRuntime>::remaining_contribute(
							RuntimeOrigin::signed(BUYER_1),
							get_mock_jwt_with_cid(
								BUYER_1,
								InvestorType::Retail,
								generate_did_from_account(BUYER_1),
								project_policy
							),
							project_id,
							1000 * CT_UNIT,
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
					500 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(ISSUER_1),
					InvestorType::Institutional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				)),
				Error::<TestRuntime>::IssuerError(IssuerErrorReason::ParticipationToOwnProject)
			);
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = ProjectMetadata {
				token_information: default_token_information(),
				mainnet_token_max_supply: 8_000_000 * CT_UNIT,
				total_allocation_size: 1_000_000 * CT_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(50u8),
				minimum_price: PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
					PriceOf::<TestRuntime>::from_float(10.0),
					USD_DECIMALS,
					CT_DECIMALS,
				)
				.unwrap(),
				bidding_ticket_sizes: BiddingTicketSizes {
					professional: TicketSize::new(8000 * USD_UNIT, None),
					institutional: TicketSize::new(20_000 * USD_UNIT, None),
					phantom: Default::default(),
				},
				contributing_ticket_sizes: ContributingTicketSizes {
					retail: TicketSize::new(10 * USD_UNIT, None),
					professional: TicketSize::new(100_000 * USD_UNIT, None),
					institutional: TicketSize::new(200_000 * USD_UNIT, None),
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
				(BUYER_4, 50_000 * PLMC).into(),
				(BUYER_5, 50_000 * PLMC).into(),
				(BUYER_6, 50_000 * PLMC).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_4, 50_000 * USDT_UNIT).into(),
				(BUYER_5, 50_000 * USDT_UNIT).into(),
				(BUYER_6, 50_000 * USDT_UNIT).into(),
			]);

			// contribution below 1 CT (10 USD) should fail for retail
			let jwt = get_mock_jwt_with_cid(
				BUYER_4,
				InvestorType::Retail,
				generate_did_from_account(BUYER_4),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::remaining_contribute(
						RuntimeOrigin::signed(BUYER_4),
						jwt,
						project_id,
						CT_UNIT / 2,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooLow)
				);
			});
			// contribution below 10_000 CT (100k USD) should fail for professionals
			let jwt = get_mock_jwt_with_cid(
				BUYER_5,
				InvestorType::Professional,
				generate_did_from_account(BUYER_5),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
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
			let jwt = get_mock_jwt_with_cid(
				BUYER_6,
				InvestorType::Institutional,
				generate_did_from_account(BUYER_6),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
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
				mainnet_token_max_supply: 8_000_000 * CT_UNIT,
				total_allocation_size: 1_000_000 * CT_UNIT,
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
					retail: TicketSize::new(USD_UNIT, Some(300_000 * USD_UNIT)),
					professional: TicketSize::new(USD_UNIT, Some(20_000 * USD_UNIT)),
					institutional: TicketSize::new(USD_UNIT, Some(50_000 * USD_UNIT)),
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
				(BUYER_4, 500_000 * PLMC).into(),
				(BUYER_5, 500_000 * PLMC).into(),
				(BUYER_6, 500_000 * PLMC).into(),
				(BUYER_7, 500_000 * PLMC).into(),
				(BUYER_8, 500_000 * PLMC).into(),
				(BUYER_9, 500_000 * PLMC).into(),
			]);

			inst.mint_foreign_asset_to(vec![
				(BUYER_4, 500_000 * USDT_UNIT).into(),
				(BUYER_5, 500_000 * USDT_UNIT).into(),
				(BUYER_6, 500_000 * USDT_UNIT).into(),
				(BUYER_7, 500_000 * USDT_UNIT).into(),
				(BUYER_8, 500_000 * USDT_UNIT).into(),
				(BUYER_9, 500_000 * USDT_UNIT).into(),
			]);

			// total contributions with same DID above 30k CT (300k USD) should fail for retail
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_4,
					project_id,
					28_000 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_4),
					InvestorType::Retail,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_5,
						project_id,
						2001 * CT_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 1, on a different account
						generate_did_from_account(BUYER_4),
						InvestorType::Retail,
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_5,
					project_id,
					2000 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_4),
					InvestorType::Retail,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});

			// total contributions with same DID above 2k CT (20k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_6,
					project_id,
					1800 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_6),
					InvestorType::Professional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_7,
						project_id,
						201 * CT_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 1, on a different account
						generate_did_from_account(BUYER_6),
						InvestorType::Professional,
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 2k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_7,
					project_id,
					200 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 1, on a different account
					generate_did_from_account(BUYER_6),
					InvestorType::Professional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});

			// total contributions with same DID above 5k CT (50 USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_8,
					project_id,
					4690 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					generate_did_from_account(BUYER_8),
					InvestorType::Institutional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_remaining_contribute(
						&BUYER_9,
						project_id,
						311 * CT_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT,
						// note we use the same did as bidder 3, on a different account
						generate_did_from_account(BUYER_8),
						InvestorType::Institutional,
						project_metadata.clone().policy_ipfs_cid.unwrap(),
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::TooHigh)
				);
			});
			// bidding 5k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_remaining_contribute(
					&BUYER_9,
					project_id,
					310 * CT_UNIT,
					1u8.try_into().unwrap(),
					AcceptedFundingAsset::USDT,
					// note we use the same did as bidder 3, on a different account
					generate_did_from_account(BUYER_8),
					InvestorType::Institutional,
					project_metadata.clone().policy_ipfs_cid.unwrap(),
				));
			});
		}

		#[test]
		fn cannot_use_evaluation_bond_on_another_project_contribution() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			let mut evaluations_1 = default_evaluations();
			let evaluations_2 = default_evaluations();

			let evaluator_contributor = 69;
			let evaluation_amount = 420 * USD_UNIT;
			let evaluator_contribution =
				ContributionParams::new(evaluator_contributor, 600 * CT_UNIT, 1u8, AcceptedFundingAsset::USDT);
			evaluations_1.push((evaluator_contributor, evaluation_amount).into());

			let _project_id_1 = inst.create_remainder_contributing_project(
				project_metadata_1.clone(),
				ISSUER_1,
				evaluations_1,
				default_bids(),
				vec![],
			);
			let project_id_2 = inst.create_remainder_contributing_project(
				project_metadata_2.clone(),
				ISSUER_2,
				evaluations_2,
				default_bids(),
				vec![],
			);

			let wap = inst.get_project_details(project_id_2).weighted_average_price.unwrap();

			// Necessary Mints
			let already_bonded_plmc =
				inst.calculate_evaluation_plmc_spent(vec![(evaluator_contributor, evaluation_amount).into()])[0]
					.plmc_amount;
			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_contribution =
				inst.calculate_contributed_plmc_spent(vec![evaluator_contribution.clone()], wap)[0].plmc_amount;
			let necessary_usdt_for_contribution =
				inst.calculate_contributed_funding_asset_spent(vec![evaluator_contribution.clone()], wap);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_contributor,
				necessary_plmc_for_contribution - usable_evaluation_plmc,
			)]);
			inst.mint_foreign_asset_to(necessary_usdt_for_contribution);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::remaining_contribute(
						RuntimeOrigin::signed(evaluator_contributor),
						get_mock_jwt_with_cid(
							evaluator_contributor,
							InvestorType::Retail,
							generate_did_from_account(evaluator_contributor),
							project_metadata_2.clone().policy_ipfs_cid.unwrap(),
						),
						project_id_2,
						evaluator_contribution.amount,
						evaluator_contribution.multiplier,
						evaluator_contribution.asset
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::NotEnoughFunds)
				);
			});
		}

		#[test]
		fn wrong_policy_on_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_remainder_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				default_evaluations(),
				default_bids(),
				vec![],
			);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::remaining_contribute(
						RuntimeOrigin::signed(BUYER_1),
						get_mock_jwt_with_cid(
							BUYER_1,
							InvestorType::Retail,
							generate_did_from_account(BUYER_1),
							"wrong_cid".as_bytes().to_vec().try_into().unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						1u8.try_into().unwrap(),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::ParticipationFailed(ParticipationError::PolicyMismatch)
				);
			});
		}
	}
}
