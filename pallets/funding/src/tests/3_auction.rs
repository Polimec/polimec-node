use super::*;
use frame_support::traits::{fungible::InspectFreeze, fungibles::metadata::Inspect};
use sp_core::bounded_vec;
use std::collections::HashSet;

#[cfg(test)]
mod round_flow {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn auction_round_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let bids = default_bids();
			let _project_id =
				inst.create_community_contributing_project(project_metadata, ISSUER_1, None, evaluations, bids);
		}

		#[test]
		fn multiple_auction_projects_completed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project1 = default_project_metadata(ISSUER_1);
			let project2 = default_project_metadata(ISSUER_2);
			let project3 = default_project_metadata(ISSUER_3);
			let project4 = default_project_metadata(ISSUER_4);
			let evaluations = default_evaluations();
			let bids = default_bids();

			inst.create_community_contributing_project(project1, ISSUER_1, None, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project2, ISSUER_2, None, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project3, ISSUER_3, None, evaluations.clone(), bids.clone());
			inst.create_community_contributing_project(project4, ISSUER_4, None, evaluations, bids);
		}

		#[test]
		fn auction_gets_percentage_of_ct_total_allocation() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations = default_evaluations();
			let auction_percentage = project_metadata.auction_round_allocation_percentage;
			let total_allocation = project_metadata.total_allocation_size;

			let auction_allocation = auction_percentage * total_allocation;

			let bids = vec![(BIDDER_1, auction_allocation).into()];
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				evaluations.clone(),
				bids,
			);
			let mut bid_infos = Bids::<TestRuntime>::iter_prefix_values((project_id,));
			let bid_info = inst.execute(|| bid_infos.next().unwrap());
			assert!(inst.execute(|| bid_infos.next().is_none()));
			assert_eq!(bid_info.original_ct_amount, auction_allocation);

			let project_metadata = default_project_metadata(ISSUER_2);
			let bids = vec![(BIDDER_1, auction_allocation).into(), (BIDDER_1, 1000 * CT_UNIT).into()];
			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_2,
				None,
				evaluations.clone(),
				bids,
			);
			let project_details = inst.get_project_details(project_id);

			let bid_info_1 = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 1)).unwrap());
			let bid_info_2 = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 2)).unwrap());
			assert!(inst.execute(|| bid_infos.next().is_none()));
			assert_eq!(
				bid_info_1.status,
				BidStatus::PartiallyAccepted(auction_allocation - 1000 * CT_UNIT),
				"Should not be able to buy more than auction allocation"
			);
			assert_eq!(bid_info_2.status, BidStatus::Accepted, "Should outbid the previous bid");
			assert_eq!(project_details.remaining_contribution_tokens, total_allocation - auction_allocation);
		}

		#[test]
		fn no_bids_made() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let evaluations = default_evaluations();
			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));

			assert_eq!(
				inst.get_project_details(project_id).weighted_average_price,
				Some(project_metadata.minimum_price)
			);
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
				PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::USDT).unwrap()
			});
			let usdc_price = inst.execute(|| {
				PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::USDC).unwrap()
			});
			let dot_price = inst.execute(|| {
				PolimecFunding::get_decimals_aware_funding_asset_price(&AcceptedFundingAsset::DOT).unwrap()
			});

			let mut funding_assets_cycle =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.into_iter()
					.cycle();

			let mut min_bid_amounts_ct = Vec::new();
			let mut min_bid_amounts_usd = Vec::new();
			let mut auction_allocations_ct = Vec::new();
			let mut auction_allocations_usd = Vec::new();

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
				let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

				let auction_allocation_percentage = project_metadata.auction_round_allocation_percentage;
				let auction_allocation_ct = auction_allocation_percentage * project_metadata.total_allocation_size;
				auction_allocations_ct.push(auction_allocation_ct);
				let auction_allocation_usd = project_metadata.minimum_price.saturating_mul_int(auction_allocation_ct);
				auction_allocations_usd.push(auction_allocation_usd);

				let min_professional_bid_usd =
					project_metadata.bidding_ticket_sizes.professional.usd_minimum_per_participation;
				min_bid_amounts_usd.push(min_professional_bid_usd);
				let min_professional_bid_ct =
					project_metadata.minimum_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);
				let min_professional_bid_plmc =
					usable_plmc_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);
				min_bid_amounts_ct.push(min_professional_bid_ct);
				let min_professional_bid_funding_asset =
					funding_asset_usd_price.reciprocal().unwrap().saturating_mul_int(min_professional_bid_usd);

				// Every project should want to raise 5MM USD on the auction round regardless of CT decimals
				assert_eq!(auction_allocation_usd, 5_000_000 * USD_UNIT);

				// A minimum bid goes through. This is a fixed USD value, but the extrinsic amount depends on CT decimals.
				inst.mint_plmc_ed_if_required(vec![BIDDER_1]);
				inst.mint_funding_asset_ed_if_required(vec![(BIDDER_1, funding_asset.id())]);
				inst.mint_plmc_to(vec![UserToPLMCBalance::new(BIDDER_1, min_professional_bid_plmc + ed)]);
				inst.mint_funding_asset_to(vec![UserToFundingAsset::new(
					BIDDER_1,
					min_professional_bid_funding_asset,
					funding_asset.id(),
				)]);

				assert_ok!(inst.execute(|| PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_1),
					get_mock_jwt_with_cid(
						BIDDER_1,
						InvestorType::Professional,
						generate_did_from_account(BIDDER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					min_professional_bid_ct,
					ParticipationMode::Classic(1u8),
					funding_asset,
				)));

				// The bucket should have 50% of 1MM * 10^decimals CT minus what we just bid
				let bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());
				assert_eq!(bucket.amount_left, 500_000u128 * 10u128.pow(decimals as u32) - min_professional_bid_ct);
			};

			for decimals in 6..=18 {
				decimal_test(decimals);
			}

			// Since we use the same original price and allocation size and adjust for decimals,
			// the USD amounts should be the same
			assert!(min_bid_amounts_usd.iter().all(|x| *x == min_bid_amounts_usd[0]));
			assert!(auction_allocations_usd.iter().all(|x| *x == auction_allocations_usd[0]));

			// CT amounts however should be different from each other
			let mut hash_set_1 = HashSet::new();
			for amount in min_bid_amounts_ct {
				assert!(!hash_set_1.contains(&amount));
				hash_set_1.insert(amount);
			}
			let mut hash_set_2 = HashSet::new();
			for amount in auction_allocations_ct {
				assert!(!hash_set_2.contains(&amount));
				hash_set_2.insert(amount);
			}
		}

		#[test]
		fn all_bids_but_one_have_price_higher_than_wap() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let total_allocation = 10_000_000 * CT_UNIT;
			let min_bid_ct = 500 * CT_UNIT; // 5k USD at 10USD/CT
			let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
			let big_bid: BidParams<TestRuntime> = (BIDDER_1, total_allocation).into();
			let small_bids: Vec<BidParams<TestRuntime>> =
				(0..max_bids_per_project - 1).map(|i| (i as u64 + BIDDER_1, min_bid_ct).into()).collect();
			let all_bids = vec![vec![big_bid.clone()], small_bids.clone()].into_iter().flatten().collect_vec();

			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = total_allocation;
			project_metadata.total_allocation_size = total_allocation;
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(100);

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights()),
				all_bids,
			);

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let all_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());

			let higher_than_wap_bids = all_bids.iter().filter(|bid| bid.original_ct_usd_price > wap).collect_vec();
			assert_eq!(higher_than_wap_bids.len(), (max_bids_per_project - 1u32) as usize);
		}

		#[test]
		fn auction_oversubscription() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let bucket_size = Percent::from_percent(10) * auction_allocation;
			let bids = vec![
				(BIDDER_1, auction_allocation).into(),
				(BIDDER_2, bucket_size).into(),
				(BIDDER_3, bucket_size).into(),
				(BIDDER_4, bucket_size).into(),
				(BIDDER_5, bucket_size).into(),
				(BIDDER_6, bucket_size).into(),
			];

			let project_id = inst.create_community_contributing_project(
				project_metadata.clone(),
				ISSUER_1,
				None,
				default_evaluations(),
				bids,
			);

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
			assert!(wap > project_metadata.minimum_price);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn contribute_does_not_work() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			let did = generate_did_from_account(ISSUER_1);
			let investor_type = InvestorType::Retail;
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_contribute(DoContributeParams::<TestRuntime> {
						contributor: BIDDER_1,
						project_id,
						ct_amount: 100,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did,
						investor_type,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					}),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}

#[cfg(test)]
mod bid_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;
		use crate::AcceptedFundingAsset::USDT;
		use frame_support::pallet_prelude::DispatchResultWithPostInfo;

		#[test]
		fn evaluation_bond_counts_towards_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = default_evaluations();
			let evaluator_bidder = 69u64;
			let evaluation_amount = 420 * USD_UNIT;
			let evaluator_bid = BidParams::new(
				evaluator_bidder,
				600 * CT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let already_bonded_plmc =
				inst.calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0].plmc_amount;

			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;

			let necessary_plmc_for_bid = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			)[0]
			.plmc_amount;

			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_plmc_ed_if_required(vec![evaluator_bidder]);
			inst.mint_funding_asset_ed_if_required(vec![(evaluator_bidder, USDT.id())]);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_bidder,
				necessary_plmc_for_bid - usable_evaluation_plmc,
			)]);
			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			inst.bid_for_users(project_id, vec![evaluator_bid]).unwrap();

			let evaluation_items = inst.execute(|| {
				Evaluations::<TestRuntime>::iter_prefix_values((project_id, evaluator_bidder)).collect_vec()
			});
			assert_eq!(evaluation_items.len(), 1);
			assert_eq!(evaluation_items[0].current_plmc_bond, already_bonded_plmc - usable_evaluation_plmc);

			let bid_items =
				inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id, evaluator_bidder)).collect_vec());
			assert_eq!(bid_items.len(), 1);
			assert_eq!(bid_items[0].plmc_bond, necessary_plmc_for_bid);

			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, necessary_plmc_for_bid)],
				HoldReason::Participation.into(),
			);
			inst.do_reserved_plmc_assertions(
				vec![UserToPLMCBalance::new(evaluator_bidder, already_bonded_plmc - usable_evaluation_plmc)],
				HoldReason::Evaluation.into(),
			);
		}

		#[test]
		fn bid_with_multiple_currencies() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata_all = default_project_metadata(ISSUER_1);
			project_metadata_all.participation_currencies =
				vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT]
					.try_into()
					.unwrap();

			let mut project_metadata_usdt = default_project_metadata(ISSUER_2);
			project_metadata_usdt.participation_currencies = vec![AcceptedFundingAsset::USDT].try_into().unwrap();

			let mut project_metadata_usdc = default_project_metadata(ISSUER_3);
			project_metadata_usdc.participation_currencies = vec![AcceptedFundingAsset::USDC].try_into().unwrap();

			let mut project_metadata_dot = default_project_metadata(ISSUER_4);
			project_metadata_dot.participation_currencies = vec![AcceptedFundingAsset::DOT].try_into().unwrap();

			let evaluations = default_evaluations();

			let usdt_bid =
				BidParams::new(BIDDER_1, 10_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT);
			let usdc_bid =
				BidParams::new(BIDDER_1, 10_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDC);
			let dot_bid =
				BidParams::new(BIDDER_1, 10_000 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::DOT);

			let plmc_fundings = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);

			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_fundings.clone());
			inst.mint_plmc_to(plmc_fundings.clone());

			let usdt_fundings = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()],
				project_metadata_all.minimum_price,
			);
			inst.mint_funding_asset_to(usdt_fundings.clone());
			inst.mint_funding_asset_to(usdt_fundings.clone());
			inst.mint_funding_asset_to(usdt_fundings.clone());

			let project_id_all =
				inst.create_auctioning_project(project_metadata_all, ISSUER_1, None, evaluations.clone());
			assert_ok!(inst.bid_for_users(project_id_all, vec![usdt_bid.clone(), usdc_bid.clone(), dot_bid.clone()]));

			let project_id_usdt =
				inst.create_auctioning_project(project_metadata_usdt, ISSUER_2, None, evaluations.clone());
			assert_ok!(inst.bid_for_users(project_id_usdt, vec![usdt_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_err!(
				inst.bid_for_users(project_id_usdt, vec![dot_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);

			let project_id_usdc =
				inst.create_auctioning_project(project_metadata_usdc, ISSUER_3, None, evaluations.clone());
			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_ok!(inst.bid_for_users(project_id_usdc, vec![usdc_bid.clone()]));
			assert_err!(
				inst.bid_for_users(project_id_usdc, vec![dot_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);

			let project_id_dot =
				inst.create_auctioning_project(project_metadata_dot, ISSUER_4, None, evaluations.clone());
			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdt_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_err!(
				inst.bid_for_users(project_id_dot, vec![usdc_bid.clone()]),
				Error::<TestRuntime>::FundingAssetNotAccepted
			);
			assert_ok!(inst.bid_for_users(project_id_dot, vec![dot_bid.clone()]));
		}

		fn test_bid_setup(
			inst: &mut MockInstantiator,
			project_id: ProjectId,
			bidder: AccountIdOf<TestRuntime>,
			investor_type: InvestorType,
			u8_multiplier: u8,
		) -> DispatchResultWithPostInfo {
			let project_policy = inst.get_project_metadata(project_id).policy_ipfs_cid.unwrap();
			let jwt = get_mock_jwt_with_cid(bidder, investor_type, generate_did_from_account(BIDDER_1), project_policy);
			let amount = 1000 * CT_UNIT;
			let mode = ParticipationMode::Classic(u8_multiplier);

			if u8_multiplier > 0 {
				// We cannot use helper functions because some multipliers are invalid
				let necessary_plmc = vec![(bidder, 1_000_000 * PLMC).into()];
				let necessary_usdt = vec![(bidder, 1_000_000 * USDT_UNIT).into()];

				inst.mint_plmc_to(necessary_plmc.clone());
				inst.mint_funding_asset_to(necessary_usdt.clone());
			}
			inst.execute(|| {
				Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(bidder),
					jwt,
					project_id,
					amount,
					mode,
					AcceptedFundingAsset::USDT,
				)
			})
		}

		#[test]
		fn multiplier_limits() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);
			// Professional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, 0),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
			// Professional bids: 1 - 10x multiplier should work
			for multiplier in 1..=10u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier));
			}
			// Professional bids: >=11x multiplier should fail
			for multiplier in 11..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_1, InvestorType::Professional, multiplier),
					Error::<TestRuntime>::ForbiddenMultiplier
				);
			}

			// Institutional bids: 0x multiplier should fail
			assert_err!(
				test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, 0),
				Error::<TestRuntime>::ForbiddenMultiplier
			);
			// Institutional bids: 1 - 25x multiplier should work
			for multiplier in 1..=25u8 {
				assert_ok!(test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier));
			}
			// Institutional bids: >=26x multiplier should fail
			for multiplier in 26..=50u8 {
				assert_err!(
					test_bid_setup(&mut inst, project_id, BIDDER_2, InvestorType::Institutional, multiplier),
					Error::<TestRuntime>::ForbiddenMultiplier
				);
			}
		}

		#[test]
		fn bid_split_into_multiple_buckets() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				project_metadata.clone().token_information.decimals,
			)
			.unwrap();
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(50u8);

			let evaluations = default_evaluations();
			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

			// bid that fills 80% of the first bucket
			let bid_40_percent = inst.generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				40u8,
				vec![100],
				vec![BIDDER_1],
				vec![ParticipationMode::Classic(8u8)],
			);

			// Note: 5% of total CTs is one bucket, i.e 10% of the auction allocation
			// This bid fills last 20% of the first bucket,
			// and gets split into 3 more bids of 2 more full and one partially full buckets.
			// 10% + 5% + 5% + 3% = 23%
			let bid_23_percent = inst.generate_bids_from_total_ct_percent(
				project_metadata.clone(),
				23u8,
				vec![100],
				vec![BIDDER_2],
				vec![ParticipationMode::Classic(7u8)],
			);

			let all_bids = vec![bid_40_percent[0].clone(), bid_23_percent[0].clone()];

			inst.mint_plmc_ed_if_required(all_bids.accounts());
			inst.mint_funding_asset_ed_if_required(all_bids.to_account_asset_map());

			let necessary_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			let necessary_usdt = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
			inst.mint_plmc_to(necessary_plmc.clone());
			inst.mint_funding_asset_to(necessary_usdt.clone());

			inst.bid_for_users(project_id, bid_40_percent.clone()).unwrap();
			let stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
			assert_eq!(stored_bids.len(), 1);

			inst.bid_for_users(project_id, bid_23_percent.clone()).unwrap();
			let mut stored_bids = inst.execute(|| Bids::<TestRuntime>::iter_prefix_values((project_id,)).collect_vec());
			stored_bids.sort_by(|a, b| a.id.cmp(&b.id));
			// 40% + 10% + 5% + 5% + 3% = 5 total bids
			assert_eq!(stored_bids.len(), 5);

			let normalize_price = |decimal_aware_price| {
				PriceProviderOf::<TestRuntime>::convert_back_to_normal_price(
					decimal_aware_price,
					USD_DECIMALS,
					project_metadata.clone().token_information.decimals,
				)
				.unwrap()
			};
			assert_eq!(normalize_price(stored_bids[1].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.0));
			assert_eq!(
				stored_bids[1].original_ct_amount,
				Percent::from_percent(10) * project_metadata.total_allocation_size
			);
			assert_eq!(
				normalize_price(stored_bids[2].original_ct_usd_price),
				PriceOf::<TestRuntime>::from_rational(11, 10)
			);
			assert_eq!(
				stored_bids[2].original_ct_amount,
				Percent::from_percent(5) * project_metadata.total_allocation_size
			);

			assert_eq!(normalize_price(stored_bids[3].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.2));
			assert_eq!(
				stored_bids[3].original_ct_amount,
				Percent::from_percent(5) * project_metadata.total_allocation_size
			);

			assert_eq!(normalize_price(stored_bids[4].original_ct_usd_price), PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(
				stored_bids[4].original_ct_amount,
				Percent::from_percent(3) * project_metadata.total_allocation_size
			);
			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			assert_eq!(normalize_price(current_bucket.current_price), PriceOf::<TestRuntime>::from_float(1.3));
			assert_eq!(current_bucket.amount_left, Percent::from_percent(2) * project_metadata.total_allocation_size);
			assert_eq!(normalize_price(current_bucket.delta_price), PriceOf::<TestRuntime>::from_float(0.1));
		}

		#[test]
		fn can_bid_with_frozen_tokens_funding_failed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), issuer, None, default_evaluations());

			let bid =
				BidParams::new(BIDDER_4, 500 * CT_UNIT, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT);
			let plmc_required = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			let frozen_amount = plmc_required[0].plmc_amount;

			inst.mint_plmc_ed_if_required(plmc_required.accounts());
			inst.mint_plmc_to(plmc_required.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &BIDDER_4, plmc_required[0].plmc_amount).unwrap();
			});

			let usdt_required = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(vec![bid.clone()].to_account_asset_map());
			inst.mint_funding_asset_to(usdt_required);

			inst.execute(|| {
				assert_noop!(
					Balances::transfer_allow_death(RuntimeOrigin::signed(BIDDER_4), ISSUER_1, frozen_amount,),
					TokenError::Frozen
				);
			});

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_4),
					get_mock_jwt_with_cid(
						BIDDER_4,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_4),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					bid.amount,
					bid.mode,
					bid.asset
				));
			});

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));

			inst.execute(|| {
				PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_4), project_id, BIDDER_4, 0).unwrap();
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Evaluation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed() + frozen_amount);
			assert_eq!(bid_held_balance, Zero::zero());
			assert_eq!(frozen_balance, frozen_amount);
		}

		#[test]
		fn can_bid_with_frozen_tokens_funding_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), issuer, None, default_evaluations());

			let bid =
				BidParams::new(BIDDER_4, 500 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDT);
			let plmc_required = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			let frozen_amount = plmc_required[0].plmc_amount;

			inst.mint_plmc_ed_if_required(plmc_required.accounts());
			inst.mint_plmc_to(plmc_required.clone());

			inst.execute(|| {
				mock::Balances::set_freeze(&(), &BIDDER_4, plmc_required[0].plmc_amount).unwrap();
			});

			let usdt_required = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![bid.clone()],
				project_metadata.clone(),
				None,
			);
			inst.mint_funding_asset_ed_if_required(vec![bid.clone()].to_account_asset_map());
			inst.mint_funding_asset_to(usdt_required);

			inst.execute(|| {
				assert_noop!(
					Balances::transfer_allow_death(RuntimeOrigin::signed(BIDDER_4), ISSUER_1, frozen_amount,),
					TokenError::Frozen
				);
			});

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_4),
					get_mock_jwt_with_cid(
						BIDDER_4,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_4),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					bid.amount,
					bid.mode,
					bid.asset
				));
			});

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));
			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

			let contributions = inst.generate_contributions_from_total_ct_percent(
				project_metadata.clone(),
				90u8,
				default_weights(),
				default_community_contributors(),
				default_modes(),
			);
			let plmc_required = inst.calculate_contributed_plmc_spent(contributions.clone(), wap);
			inst.mint_plmc_ed_if_required(contributions.accounts());
			inst.mint_plmc_to(plmc_required.clone());

			let usdt_required = inst.calculate_contributed_funding_asset_spent(contributions.clone(), wap);
			inst.mint_funding_asset_ed_if_required(contributions.to_account_asset_map());
			inst.mint_funding_asset_to(usdt_required.clone());
			inst.contribute_for_users(project_id, contributions).unwrap();

			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			inst.execute(|| {
				PolimecFunding::settle_bid(RuntimeOrigin::signed(BIDDER_4), project_id, BIDDER_4, 0).unwrap();
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed());
			assert_eq!(bid_held_balance, frozen_amount);
			assert_eq!(frozen_balance, frozen_amount);

			let vest_duration =
				MultiplierOf::<TestRuntime>::try_from(5u8).unwrap().calculate_vesting_duration::<TestRuntime>();
			let now = inst.current_block();
			inst.jump_to_block(now + vest_duration + 1u64);
			inst.execute(|| {
				assert_ok!(mock::LinearRelease::vest(
					RuntimeOrigin::signed(BIDDER_4),
					HoldReason::Participation.into()
				));
			});

			let free_balance = inst.get_free_plmc_balance_for(BIDDER_4);
			let bid_held_balance = inst.get_reserved_plmc_balance_for(BIDDER_4, HoldReason::Participation.into());
			let frozen_balance = inst.execute(|| mock::Balances::balance_frozen(&(), &BIDDER_4));

			assert_eq!(free_balance, inst.get_ed() + frozen_amount);
			assert_eq!(bid_held_balance, Zero::zero());
			assert_eq!(frozen_balance, frozen_amount);
		}

		#[test]
		fn one_token_mode_bid_funding_success() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;

			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.mainnet_token_max_supply = 50_000 * CT_UNIT;
			project_metadata.total_allocation_size = 10_000 * CT_UNIT;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();

			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);
			let otm_multiplier: MultiplierOf<TestRuntime> =
				ParticipationMode::OTM.multiplier().try_into().ok().unwrap();
			let otm_duration = otm_multiplier.calculate_vesting_duration::<TestRuntime>();

			const USDT_ID: u32 = AcceptedFundingAsset::USDT.id();
			const USDT_PARTICIPATION: u128 = 5000 * USDT_UNIT;

			let otm_usdt_fee: u128 = (FeePercentage::get() / ParticipationMode::OTM.multiplier()) * USDT_PARTICIPATION;

			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let required_usdt = UserToFundingAsset::new(BIDDER_1, USDT_PARTICIPATION + otm_usdt_fee + usdt_ed, USDT_ID);
			inst.mint_funding_asset_to(vec![required_usdt.clone()]);

			let ct_participation = inst.execute(|| {
				<Pallet<TestRuntime>>::funding_asset_to_ct_amount_classic(
					project_id,
					AcceptedFundingAsset::USDT,
					USDT_PARTICIPATION,
				)
			});
			// USDT has the same decimals and price as our baseline USD
			let expected_plmc_bond =
				<Pallet<TestRuntime>>::calculate_plmc_bond(USDT_PARTICIPATION, otm_multiplier).unwrap();

			let otm_escrow_account =
				<TestRuntime as pallet_proxy_bonding::Config>::RootId::get().into_sub_account_truncating(project_id);
			let otm_treasury_account = <TestRuntime as pallet_proxy_bonding::Config>::Treasury::get();
			let otm_fee_recipient_account = <TestRuntime as pallet_proxy_bonding::Config>::FeeRecipient::get();
			let funding_project_escrow = PolimecFunding::fund_account_id(project_id);

			assert!(funding_project_escrow != otm_escrow_account);

			let pre_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let pre_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let pre_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let pre_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let pre_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_1),
					get_mock_jwt_with_cid(
						BIDDER_1,
						InvestorType::Professional,
						generate_did_from_account(BIDDER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					ct_participation,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT
				));
			});

			let post_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let post_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let post_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let post_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);

			assert_eq!(
				post_participation_treasury_free_plmc,
				pre_participation_treasury_free_plmc - expected_plmc_bond - inst.get_ed()
			);
			assert_eq!(
				post_participation_otm_escrow_held_plmc,
				pre_participation_otm_escrow_held_plmc + expected_plmc_bond
			);
			assert_close_enough!(
				post_participation_otm_escrow_usdt,
				pre_participation_otm_escrow_usdt + otm_usdt_fee,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_otm_fee_recipient_usdt,
				pre_participation_otm_fee_recipient_usdt,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_buyer_usdt,
				pre_participation_buyer_usdt - USDT_PARTICIPATION - otm_usdt_fee,
				Perquintill::from_float(0.999)
			);

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_fees_to_recipient(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into(),
					USDT_ID
				));
				assert_noop!(
					<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
						RuntimeOrigin::signed(BIDDER_1),
						project_id,
						HoldReason::Participation.into()
					),
					pallet_proxy_bonding::Error::<TestRuntime>::TooEarlyToUnlock
				);
			});
			let now = inst.current_block();
			inst.jump_to_block(otm_duration + now);
			inst.execute(|| {
				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into()
				));
			});

			let post_settlement_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_settlement_otm_escrow_held_plmc = inst.get_free_plmc_balance_for(otm_escrow_account);
			let post_settlement_otm_escrow_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let post_settlement_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let post_settlement_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);
			let issuer_funding_account = inst.get_free_funding_asset_balance_for(USDT_ID, issuer);

			assert_eq!(post_settlement_treasury_free_plmc, post_participation_treasury_free_plmc + expected_plmc_bond);
			assert_eq!(post_settlement_otm_escrow_held_plmc, inst.get_ed());
			assert_eq!(post_settlement_otm_escrow_usdt, Zero::zero());
			assert_close_enough!(post_settlement_otm_fee_recipient_usdt, otm_usdt_fee, Perquintill::from_float(0.999));
			assert_close_enough!(post_settlement_buyer_usdt, usdt_ed, Perquintill::from_float(0.999));
			assert_close_enough!(issuer_funding_account, USDT_PARTICIPATION, Perquintill::from_float(0.999));
		}

		#[test]
		fn one_token_mode_bid_funding_failed() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;

			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.mainnet_token_max_supply = 50_000 * CT_UNIT;
			project_metadata.total_allocation_size = 20_000 * CT_UNIT;
			project_metadata.minimum_price = <TestRuntime as Config>::PriceProvider::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();

			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), default_evaluators(), default_weights());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);
			let otm_multiplier: MultiplierOf<TestRuntime> =
				ParticipationMode::OTM.multiplier().try_into().ok().unwrap();

			const USDT_ID: u32 = AcceptedFundingAsset::USDT.id();
			const USDT_PARTICIPATION: u128 = 5000 * USDT_UNIT;

			let otm_usdt_fee: u128 = (FeePercentage::get() / ParticipationMode::OTM.multiplier()) * USDT_PARTICIPATION;
			let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
			let required_usdt = UserToFundingAsset::new(BIDDER_1, USDT_PARTICIPATION + otm_usdt_fee + usdt_ed, USDT_ID);
			inst.mint_funding_asset_to(vec![required_usdt.clone()]);

			let ct_participation = inst.execute(|| {
				<Pallet<TestRuntime>>::funding_asset_to_ct_amount_classic(
					project_id,
					AcceptedFundingAsset::USDT,
					USDT_PARTICIPATION,
				)
			});
			// USDT has the same decimals and price as our baseline USD
			let expected_plmc_bond =
				<Pallet<TestRuntime>>::calculate_plmc_bond(USDT_PARTICIPATION, otm_multiplier).unwrap();

			let otm_escrow_account =
				<TestRuntime as pallet_proxy_bonding::Config>::RootId::get().into_sub_account_truncating(project_id);
			let otm_treasury_account = <TestRuntime as pallet_proxy_bonding::Config>::Treasury::get();
			let otm_fee_recipient_account = <TestRuntime as pallet_proxy_bonding::Config>::FeeRecipient::get();
			let funding_project_escrow = PolimecFunding::fund_account_id(project_id);

			assert!(funding_project_escrow != otm_escrow_account);

			let pre_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let pre_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let pre_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let pre_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let pre_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);

			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(BIDDER_1),
					get_mock_jwt_with_cid(
						BIDDER_1,
						InvestorType::Institutional,
						generate_did_from_account(BIDDER_1),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					ct_participation,
					ParticipationMode::OTM,
					AcceptedFundingAsset::USDT
				));
			});

			let post_participation_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_participation_otm_escrow_held_plmc =
				inst.get_reserved_plmc_balance_for(otm_escrow_account, HoldReason::Participation.into());
			let post_participation_otm_escrow_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let post_participation_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let post_participation_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);

			assert_eq!(
				post_participation_treasury_free_plmc,
				pre_participation_treasury_free_plmc - expected_plmc_bond - inst.get_ed()
			);
			assert_eq!(
				post_participation_otm_escrow_held_plmc,
				pre_participation_otm_escrow_held_plmc + expected_plmc_bond
			);
			assert_close_enough!(
				post_participation_otm_escrow_usdt,
				pre_participation_otm_escrow_usdt + otm_usdt_fee,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_otm_fee_recipient_usdt,
				pre_participation_otm_fee_recipient_usdt,
				Perquintill::from_float(0.999)
			);
			assert_close_enough!(
				post_participation_buyer_usdt,
				pre_participation_buyer_usdt - USDT_PARTICIPATION - otm_usdt_fee,
				Perquintill::from_float(0.999)
			);

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingFailed);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Failure));
			inst.settle_project(project_id, true);

			inst.execute(|| {
				assert_noop!(
					<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_fees_to_recipient(
						RuntimeOrigin::signed(BIDDER_1),
						project_id,
						HoldReason::Participation.into(),
						USDT_ID
					),
					pallet_proxy_bonding::Error::<TestRuntime>::FeeToRecipientDisallowed
				);

				assert_ok!(<pallet_proxy_bonding::Pallet<TestRuntime>>::transfer_bonds_back_to_treasury(
					RuntimeOrigin::signed(BIDDER_1),
					project_id,
					HoldReason::Participation.into()
				));
			});

			let post_settlement_treasury_free_plmc = inst.get_free_plmc_balance_for(otm_treasury_account);
			let post_settlement_otm_escrow_held_plmc = inst.get_free_plmc_balance_for(otm_escrow_account);
			let post_settlement_otm_escrow_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, otm_escrow_account);
			let post_settlement_otm_fee_recipient_usdt =
				inst.get_free_funding_asset_balance_for(USDT_ID, otm_fee_recipient_account);
			let post_settlement_buyer_usdt = inst.get_free_funding_asset_balance_for(USDT_ID, BIDDER_1);
			let issuer_funding_account = inst.get_free_funding_asset_balance_for(USDT_ID, issuer);

			assert_eq!(post_settlement_treasury_free_plmc, post_participation_treasury_free_plmc + expected_plmc_bond);
			assert_eq!(post_settlement_otm_escrow_held_plmc, inst.get_ed());
			assert_eq!(post_settlement_otm_escrow_usdt, Zero::zero());
			assert_eq!(post_settlement_otm_fee_recipient_usdt, Zero::zero());
			assert_eq!(post_settlement_buyer_usdt, usdt_ed + USDT_PARTICIPATION + otm_usdt_fee);
			assert_eq!(issuer_funding_account, Zero::zero());
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_use_all_of_evaluation_bond_on_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let project_metadata = default_project_metadata(issuer);
			let mut evaluations = default_evaluations();
			let evaluator_bidder = 69;
			let evaluation_amount = 420 * USD_UNIT;
			let evaluator_bid = BidParams::new(
				evaluator_bidder,
				600 * CT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			evaluations.push((evaluator_bidder, evaluation_amount).into());

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata.minimum_price,
			);

			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			assert_err!(
				inst.bid_for_users(project_id, vec![evaluator_bid]),
				Error::<TestRuntime>::ParticipantNotEnoughFunds
			);
		}

		#[test]
		fn cannot_use_evaluation_bond_on_another_project_bid() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata_1 = default_project_metadata(ISSUER_1);
			let project_metadata_2 = default_project_metadata(ISSUER_2);

			let mut evaluations_1 = default_evaluations();
			let evaluations_2 = default_evaluations();

			let evaluator_bidder = 69;
			let evaluation_amount = 420 * USD_UNIT;
			let evaluator_bid = BidParams::new(
				evaluator_bidder,
				600 * CT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			evaluations_1.push((evaluator_bidder, evaluation_amount).into());

			let _project_id_1 =
				inst.create_auctioning_project(project_metadata_1.clone(), ISSUER_1, None, evaluations_1);
			let project_id_2 =
				inst.create_auctioning_project(project_metadata_2.clone(), ISSUER_2, None, evaluations_2);

			// Necessary Mints
			let already_bonded_plmc =
				inst.calculate_evaluation_plmc_spent(vec![(evaluator_bidder, evaluation_amount).into()])[0].plmc_amount;
			let usable_evaluation_plmc =
				already_bonded_plmc - <TestRuntime as Config>::EvaluatorSlash::get() * already_bonded_plmc;
			let necessary_plmc_for_bid = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata_2.minimum_price,
			)[0]
			.plmc_amount;
			let necessary_usdt_for_bid = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![evaluator_bid.clone()],
				project_metadata_2.minimum_price,
			);
			inst.mint_plmc_to(vec![UserToPLMCBalance::new(
				evaluator_bidder,
				necessary_plmc_for_bid - usable_evaluation_plmc,
			)]);
			inst.mint_funding_asset_to(necessary_usdt_for_bid);

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(evaluator_bidder),
						get_mock_jwt_with_cid(
							evaluator_bidder,
							InvestorType::Professional,
							generate_did_from_account(evaluator_bidder),
							project_metadata_2.clone().policy_ipfs_cid.unwrap()
						),
						project_id_2,
						evaluator_bid.amount,
						evaluator_bid.mode,
						evaluator_bid.asset
					),
					Error::<TestRuntime>::ParticipantNotEnoughFunds
				);
			});
		}

		#[test]
		fn cannot_bid_before_auction_round() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let _ = inst.create_evaluating_project(project_metadata.clone(), ISSUER_1, None);
			let did = generate_did_from_account(BIDDER_2);
			let investor_type = InvestorType::Institutional;

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_2,
						project_id: 0,
						ct_amount: 1,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did,
						investor_type,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					}),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}

		#[test]
		fn cannot_bid_more_than_project_limit_count() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = 1_000_000_000 * CT_UNIT;
			project_metadata.total_allocation_size = 100_000_000 * CT_UNIT;

			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), vec![EVALUATOR_1], vec![100u8]);
			let max_bids_per_project: u32 = <TestRuntime as Config>::MaxBidsPerProject::get();
			let bids = (0u32..max_bids_per_project - 1).map(|i| (i as u64 + 420, 5000 * CT_UNIT).into()).collect_vec();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

			let plmc_for_bidding =
				inst.calculate_auction_plmc_charged_with_given_price(&bids.clone(), project_metadata.minimum_price);
			let usdt_for_bidding = inst.calculate_auction_funding_asset_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);

			inst.mint_plmc_ed_if_required(bids.accounts());
			inst.mint_plmc_to(plmc_for_bidding.clone());

			inst.mint_funding_asset_ed_if_required(bids.to_account_asset_map());
			inst.mint_funding_asset_to(usdt_for_bidding.clone());

			inst.bid_for_users(project_id, bids.clone()).unwrap();

			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			let remaining_ct = current_bucket.amount_left;

			// This bid should be split in 2, but the second one should fail, making the whole extrinsic fail and roll back storage
			let failing_bid = BidParams::<TestRuntime>::new(
				BIDDER_1,
				remaining_ct + 5000 * CT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			let plmc_for_failing_bid = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![failing_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);

			let usdt_for_bidding = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![failing_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);

			inst.mint_plmc_to(plmc_for_failing_bid.clone());
			inst.mint_funding_asset_to(usdt_for_bidding.clone());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt_with_cid(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						failing_bid.amount,
						failing_bid.mode,
						failing_bid.asset
					),
					Error::<TestRuntime>::TooManyProjectParticipations
				);
			});

			// Now we test that after reaching the limit, just one bid is also not allowed
			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(failing_bid.bidder),
					get_mock_jwt_with_cid(
						failing_bid.bidder,
						InvestorType::Professional,
						generate_did_from_account(failing_bid.bidder),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					remaining_ct,
					failing_bid.mode,
					failing_bid.asset
				));
			});
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt_with_cid(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						failing_bid.mode,
						failing_bid.asset
					),
					Error::<TestRuntime>::TooManyProjectParticipations
				);
			});
		}

		#[test]
		fn cannot_bid_more_than_user_limit_count() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.mainnet_token_max_supply = 1_000_000_000 * CT_UNIT;
			project_metadata.total_allocation_size = 100_000_000 * CT_UNIT;

			let evaluations =
				inst.generate_successful_evaluations(project_metadata.clone(), vec![EVALUATOR_1], vec![100u8]);
			let max_bids_per_user: u32 = <TestRuntime as Config>::MaxBidsPerUser::get();
			let bids = (0u32..max_bids_per_user - 1u32).map(|_| (BIDDER_1, 5000 * CT_UNIT).into()).collect_vec();

			let project_id = inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations);

			let plmc_for_bidding =
				inst.calculate_auction_plmc_charged_with_given_price(&bids.clone(), project_metadata.minimum_price);
			let usdt_for_bidding = inst.calculate_auction_funding_asset_charged_with_given_price(
				&bids.clone(),
				project_metadata.minimum_price,
			);

			inst.mint_plmc_ed_if_required(bids.accounts());
			inst.mint_plmc_to(plmc_for_bidding.clone());

			inst.mint_funding_asset_ed_if_required(bids.to_account_asset_map());
			inst.mint_funding_asset_to(usdt_for_bidding.clone());

			inst.bid_for_users(project_id, bids.clone()).unwrap();

			let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id)).unwrap();
			let remaining_ct = current_bucket.amount_left;

			// This bid should be split in 2, but the second one should fail, making the whole extrinsic fail and roll back storage
			let failing_bid = BidParams::<TestRuntime>::new(
				BIDDER_1,
				remaining_ct + 5000 * CT_UNIT,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			let plmc_for_failing_bid = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![failing_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);
			let usdt_for_bidding = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![failing_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);
			inst.mint_plmc_to(plmc_for_failing_bid.clone());
			inst.mint_funding_asset_to(usdt_for_bidding.clone());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt_with_cid(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						failing_bid.amount,
						failing_bid.mode,
						failing_bid.asset
					),
					Error::<TestRuntime>::TooManyUserParticipations
				);
			});

			// Now we test that after reaching the limit, just one bid is also not allowed
			inst.execute(|| {
				assert_ok!(PolimecFunding::bid(
					RuntimeOrigin::signed(failing_bid.bidder),
					get_mock_jwt_with_cid(
						failing_bid.bidder,
						InvestorType::Professional,
						generate_did_from_account(failing_bid.bidder),
						project_metadata.clone().policy_ipfs_cid.unwrap()
					),
					project_id,
					remaining_ct,
					failing_bid.mode,
					failing_bid.asset
				));
			});
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(failing_bid.bidder),
						get_mock_jwt_with_cid(
							failing_bid.bidder,
							InvestorType::Professional,
							generate_did_from_account(failing_bid.bidder),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						failing_bid.mode,
						failing_bid.asset
					),
					Error::<TestRuntime>::TooManyUserParticipations
				);
			});
		}

		#[test]
		fn per_credential_type_ticket_size_minimums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * CT_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, None),
				institutional: TicketSize::new(20_000 * USD_UNIT, None),
				phantom: Default::default(),
			};

			let evaluations = default_evaluations();

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![(BIDDER_1, 50_000 * CT_UNIT).into(), (BIDDER_2, 50_000 * CT_UNIT).into()]);

			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 50_000 * USD_UNIT).into(),
				(BIDDER_2, 50_000 * USD_UNIT).into(),
			]);

			// bid below 800 CT (8k USD) should fail for professionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_1,
						project_id,
						ct_amount: 799 * CT_UNIT,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did: generate_did_from_account(BIDDER_1),
						investor_type: InvestorType::Professional,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					}),
					Error::<TestRuntime>::TooLow
				);
			});
			// bid below 2000 CT (20k USD) should fail for institutionals
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
						bidder: BIDDER_2,
						project_id,
						ct_amount: 1999 * CT_UNIT,
						mode: ParticipationMode::Classic(1u8),
						funding_asset: AcceptedFundingAsset::USDT,
						did: generate_did_from_account(BIDDER_1),
						investor_type: InvestorType::Institutional,
						whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
					}),
					Error::<TestRuntime>::TooLow
				);
			});
		}

		#[test]
		fn ticket_size_minimums_use_current_bucket_price() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * CT_UNIT;
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, None),
				institutional: TicketSize::new(20_000 * USD_UNIT, None),
				phantom: Default::default(),
			};
			project_metadata.minimum_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.0),
				USD_DECIMALS,
				project_metadata.clone().token_information.decimals,
			)
			.unwrap();

			let evaluations = default_evaluations();

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 200_000 * PLMC).into(),
				(BIDDER_2, 200_000 * PLMC).into(),
				(BIDDER_3, 200_000 * PLMC).into(),
			]);
			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 200_000 * USDT_UNIT).into(),
				(BIDDER_2, 200_000 * USDT_UNIT).into(),
				(BIDDER_3, 200_000 * USDT_UNIT).into(),
			]);

			// First bucket is covered by one bidder
			let big_bid: BidParams<TestRuntime> = (BIDDER_1, 50_000 * CT_UNIT).into();
			inst.bid_for_users(project_id, vec![big_bid.clone()]).unwrap();

			// A bid at the min price of 1 should require a min of 8k CT, but with a new price of 1.1, we can now bid with less
			let bucket_increase_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				PriceOf::<TestRuntime>::from_float(1.1),
				USD_DECIMALS,
				project_metadata.clone().token_information.decimals,
			)
			.unwrap();
			let smallest_ct_amount_at_8k_usd = bucket_increase_price
				.reciprocal()
				.unwrap()
				.checked_mul_int(8000 * USD_UNIT)
				// add 1 because result could be .99999 of what we expect
				.unwrap() + 1;
			assert!(smallest_ct_amount_at_8k_usd < 8000 * CT_UNIT);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: BIDDER_2,
					project_id,
					ct_amount: smallest_ct_amount_at_8k_usd,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(BIDDER_1),
					investor_type: InvestorType::Professional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
				}));
			});
			let smallest_ct_amount_at_20k_usd = bucket_increase_price
				.reciprocal()
				.unwrap()
				.checked_mul_int(20_000 * USD_UNIT)
				// add 1 because result could be .99999 of what we expect
				.unwrap() + 1;
			assert!(smallest_ct_amount_at_20k_usd < 20_000 * CT_UNIT);
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: BIDDER_3,
					project_id,
					ct_amount: smallest_ct_amount_at_20k_usd,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(BIDDER_1),
					investor_type: InvestorType::Institutional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
				}));
			});
		}

		#[test]
		fn per_credential_type_ticket_size_maximums() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.bidding_ticket_sizes = BiddingTicketSizes {
				professional: TicketSize::new(8_000 * USD_UNIT, Some(100_000 * USD_UNIT)),
				institutional: TicketSize::new(20_000 * USD_UNIT, Some(500_000 * USD_UNIT)),
				phantom: Default::default(),
			};
			project_metadata.contributing_ticket_sizes = ContributingTicketSizes {
				retail: TicketSize::new(USD_UNIT, Some(100_000 * USD_UNIT)),
				professional: TicketSize::new(USD_UNIT, Some(20_000 * USD_UNIT)),
				institutional: TicketSize::new(USD_UNIT, Some(50_000 * USD_UNIT)),
				phantom: Default::default(),
			};
			let evaluations = default_evaluations();

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, evaluations.clone());

			inst.mint_plmc_to(vec![
				(BIDDER_1, 500_000 * CT_UNIT).into(),
				(BIDDER_2, 500_000 * CT_UNIT).into(),
				(BIDDER_3, 500_000 * CT_UNIT).into(),
				(BIDDER_4, 500_000 * CT_UNIT).into(),
			]);

			inst.mint_funding_asset_to(vec![
				(BIDDER_1, 500_000 * USD_UNIT).into(),
				(BIDDER_2, 500_000 * USD_UNIT).into(),
				(BIDDER_3, 500_000 * USD_UNIT).into(),
				(BIDDER_4, 500_000 * USD_UNIT).into(),
			]);

			let bidder_1_jwt = get_mock_jwt_with_cid(
				BIDDER_1,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let bidder_2_jwt_same_did = get_mock_jwt_with_cid(
				BIDDER_2,
				InvestorType::Professional,
				generate_did_from_account(BIDDER_1),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total bids with same DID above 10k CT (100k USD) should fail for professionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_1),
					bidder_1_jwt,
					project_id,
					8000 * CT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_2),
						bidder_2_jwt_same_did.clone(),
						project_id,
						3000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::TooHigh
				);
			});
			// bidding 10k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_2),
					bidder_2_jwt_same_did,
					project_id,
					2000 * CT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});

			let bidder_3_jwt = get_mock_jwt_with_cid(
				BIDDER_3,
				InvestorType::Institutional,
				generate_did_from_account(BIDDER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			let bidder_4_jwt_same_did = get_mock_jwt_with_cid(
				BIDDER_4,
				InvestorType::Institutional,
				generate_did_from_account(BIDDER_3),
				project_metadata.clone().policy_ipfs_cid.unwrap(),
			);
			// total bids with same DID above 50k CT (500k USD) should fail for institutionals
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_3),
					bidder_3_jwt,
					project_id,
					40_000 * CT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
			inst.execute(|| {
				assert_noop!(
					Pallet::<TestRuntime>::bid(
						RuntimeOrigin::signed(BIDDER_4),
						bidder_4_jwt_same_did.clone(),
						project_id,
						11_000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT,
					),
					Error::<TestRuntime>::TooHigh
				);
			});
			// bidding 50k total works
			inst.execute(|| {
				assert_ok!(Pallet::<TestRuntime>::bid(
					RuntimeOrigin::signed(BIDDER_4),
					bidder_4_jwt_same_did,
					project_id,
					10_000 * CT_UNIT,
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				));
			});
		}

		#[test]
		fn issuer_cannot_bid_his_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());
			assert_err!(
				inst.execute(|| crate::Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: ISSUER_1,
					project_id,
					ct_amount: 5000 * CT_UNIT,
					mode: ParticipationMode::Classic(1u8),
					funding_asset: AcceptedFundingAsset::USDT,
					did: generate_did_from_account(ISSUER_1),
					investor_type: InvestorType::Professional,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
				})),
				Error::<TestRuntime>::ParticipationToOwnProject
			);
		}

		#[test]
		fn bid_with_asset_not_accepted() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());
			let bids = [BidParams::<TestRuntime>::new(
				BIDDER_1,
				10_000,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDC,
			)];

			let did = generate_did_from_account(bids[0].bidder);
			let investor_type = InvestorType::Institutional;

			let outcome = inst.execute(|| {
				Pallet::<TestRuntime>::do_bid(DoBidParams::<TestRuntime> {
					bidder: bids[0].bidder,
					project_id,
					ct_amount: bids[0].amount,
					mode: bids[0].mode,
					funding_asset: bids[0].asset,
					did,
					investor_type,
					whitelisted_policy: project_metadata.clone().policy_ipfs_cid.unwrap(),
				})
			});
			frame_support::assert_err!(outcome, Error::<TestRuntime>::FundingAssetNotAccepted);
		}

		#[test]
		fn wrong_policy_on_jwt() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(BIDDER_1),
						get_mock_jwt_with_cid(
							BIDDER_1,
							InvestorType::Professional,
							generate_did_from_account(BIDDER_1),
							"wrong_cid".as_bytes().to_vec().try_into().unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::PolicyMismatch
				);
			});
		}

		#[test]
		fn bid_after_end_block_before_transitioning_project() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());
			let end_block = inst.get_project_details(project_id).round_duration.end.unwrap();
			inst.jump_to_block(end_block + 1);
			assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionRound);
			inst.execute(|| {
				assert_noop!(
					PolimecFunding::bid(
						RuntimeOrigin::signed(BIDDER_1),
						get_mock_jwt_with_cid(
							BIDDER_1,
							InvestorType::Professional,
							generate_did_from_account(BIDDER_1),
							project_metadata.clone().policy_ipfs_cid.unwrap()
						),
						project_id,
						5000 * CT_UNIT,
						ParticipationMode::Classic(1u8),
						AcceptedFundingAsset::USDT
					),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}

#[cfg(test)]
mod end_auction_extrinsic {
	use super::*;

	#[cfg(test)]
	mod success {
		use super::*;

		#[test]
		fn wap_is_accurate() {
			// From the knowledge hub: https://hub.polimec.org/learn/calculation-example#auction-round-calculation-example
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			const ADAM: AccountIdOf<TestRuntime> = 60;
			const TOM: AccountIdOf<TestRuntime> = 61;
			const SOFIA: AccountIdOf<TestRuntime> = 62;
			const FRED: AccountIdOf<TestRuntime> = 63;
			const ANNA: AccountIdOf<TestRuntime> = 64;
			const DAMIAN: AccountIdOf<TestRuntime> = 65;

			let accounts = [ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];

			let bounded_name = bounded_name();
			let bounded_symbol = bounded_symbol();
			let metadata_hash = ipfs_hash();
			let normalized_price = PriceOf::<TestRuntime>::from_float(10.0);
			let decimal_aware_price = PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(
				normalized_price,
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			let project_metadata = ProjectMetadata {
				token_information: CurrencyMetadata {
					name: bounded_name,
					symbol: bounded_symbol,
					decimals: CT_DECIMALS,
				},
				mainnet_token_max_supply: 8_000_000 * CT_UNIT,
				total_allocation_size: 100_000 * CT_UNIT,
				auction_round_allocation_percentage: Percent::from_percent(50u8),
				minimum_price: decimal_aware_price,
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
				policy_ipfs_cid: Some(metadata_hash),
			};

			// overfund with plmc
			let plmc_fundings = accounts
				.iter()
				.map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: PLMC * 1_000_000 })
				.collect_vec();
			let usdt_fundings = accounts
				.iter()
				.map(|acc| UserToFundingAsset {
					account: *acc,
					asset_amount: USD_UNIT * 1_000_000,
					asset_id: AcceptedFundingAsset::USDT.id(),
				})
				.collect_vec();
			inst.mint_plmc_to(plmc_fundings);
			inst.mint_funding_asset_to(usdt_fundings);

			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());

			let bids = vec![
				(ADAM, 10_000 * CT_UNIT).into(),
				(TOM, 20_000 * CT_UNIT).into(),
				(SOFIA, 20_000 * CT_UNIT).into(),
				(FRED, 10_000 * CT_UNIT).into(),
				(ANNA, 5_000 * CT_UNIT).into(),
				(DAMIAN, 5_000 * CT_UNIT).into(),
			];

			inst.bid_for_users(project_id, bids).unwrap();

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));

			let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let normalized_wap =
				PriceProviderOf::<TestRuntime>::convert_back_to_normal_price(token_price, USD_DECIMALS, CT_DECIMALS)
					.unwrap();
			let desired_price = PriceOf::<TestRuntime>::from_float(11.1818f64);

			assert_close_enough!(
				normalized_wap.saturating_mul_int(CT_UNIT),
				desired_price.saturating_mul_int(CT_UNIT),
				Perquintill::from_float(0.9999)
			);
		}

		// Partial acceptance at price <= wap (refund due to less CT bought)
		// Full Acceptance at price > wap (refund due to final price lower than original price paid)
		// Rejection due to no more tokens left (full refund)
		#[test]
		fn bids_get_rejected_and_refunded() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let issuer = ISSUER_1;
			let mut project_metadata = default_project_metadata(issuer);
			project_metadata.total_allocation_size = 100_000 * CT_UNIT;
			project_metadata.mainnet_token_max_supply = project_metadata.total_allocation_size;
			project_metadata.auction_round_allocation_percentage = Percent::from_percent(50);
			project_metadata.minimum_price = ConstPriceProvider::calculate_decimals_aware_price(
				FixedU128::from_float(10.0f64),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap();
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT];

			let evaluations = default_evaluations();

			// We use multiplier > 1 so after settlement, only the refunds defined above are done. The rest will be done
			// through the linear release pallet
			let bid_1 =
				BidParams::new(BIDDER_1, 5000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDT);
			let bid_2 =
				BidParams::new(BIDDER_2, 40_000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDC);
			let bid_3 =
				BidParams::new(BIDDER_1, 10_000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::DOT);
			let bid_4 =
				BidParams::new(BIDDER_3, 6000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::USDT);
			let bid_5 =
				BidParams::new(BIDDER_4, 2000 * CT_UNIT, ParticipationMode::Classic(5u8), AcceptedFundingAsset::DOT);
			// post bucketing, the bids look like this:
			// (BIDDER_1, 5k) - (BIDDER_2, 40k) - (BIDDER_1, 5k) - (BIDDER_1, 5k) - (BIDDER_3 - 5k) - (BIDDER_3 - 1k) - (BIDDER_4 - 2k)
			// | -------------------- 10USD ----------------------|---- 11 USD ---|---- 12 USD ----|----------- 13 USD -------------|
			// post wap ~ 1.0557252:
			// (Accepted, 5k) - (Partially, 32k) - (Rejected, 5k) - (Accepted, 5k) - (Accepted - 5k) - (Accepted - 1k) - (Accepted - 2k)

			let bids = vec![bid_1, bid_2, bid_3, bid_4, bid_5];

			let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

			let plmc_amounts = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);
			let funding_asset_amounts = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&bids,
				project_metadata.clone(),
				None,
			);

			inst.mint_plmc_ed_if_required(bids.accounts());
			inst.mint_funding_asset_ed_if_required(bids.to_account_asset_map());

			let prev_plmc_balances = inst.get_free_plmc_balances_for(bids.accounts());
			let prev_funding_asset_balances = inst.get_free_funding_asset_balances_for(bids.to_account_asset_map());

			inst.mint_plmc_to(plmc_amounts.clone());
			inst.mint_funding_asset_to(funding_asset_amounts.clone());

			inst.bid_for_users(project_id, bids.clone()).unwrap();

			inst.do_free_plmc_assertions(vec![
				UserToPLMCBalance::new(BIDDER_1, inst.get_ed()),
				UserToPLMCBalance::new(BIDDER_2, inst.get_ed()),
			]);
			inst.do_reserved_plmc_assertions(plmc_amounts.clone(), HoldReason::Participation.into());

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));

			let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let returned_auction_plmc =
				inst.calculate_auction_plmc_returned_from_all_bids_made(&bids, project_metadata.clone(), wap);
			let returned_funding_assets =
				inst.calculate_auction_funding_asset_returned_from_all_bids_made(&bids, project_metadata, wap);

			let expected_free_plmc = inst
				.generic_map_operation(vec![returned_auction_plmc.clone(), prev_plmc_balances], MergeOperation::Add);
			let expected_free_funding_assets = inst.generic_map_operation(
				vec![returned_funding_assets.clone(), prev_funding_asset_balances],
				MergeOperation::Add,
			);
			let expected_reserved_plmc =
				inst.generic_map_operation(vec![plmc_amounts.clone(), returned_auction_plmc], MergeOperation::Subtract);
			let expected_final_funding_spent = inst.generic_map_operation(
				vec![funding_asset_amounts.clone(), returned_funding_assets],
				MergeOperation::Subtract,
			);
			let expected_issuer_funding = inst.sum_funding_asset_mappings(vec![expected_final_funding_spent]);

			// Assertions about rejected bid
			let rejected_bid = inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 2)).unwrap());
			assert_eq!(rejected_bid.status, BidStatus::Rejected);
			let bidder_plmc_pre_balance = inst.get_free_plmc_balance_for(rejected_bid.bidder);
			let bidder_funding_asset_pre_balance =
				inst.get_free_funding_asset_balance_for(rejected_bid.funding_asset.id(), rejected_bid.bidder);
			inst.execute(|| {
				PolimecFunding::settle_bid(
					RuntimeOrigin::signed(rejected_bid.bidder),
					project_id,
					rejected_bid.bidder,
					2,
				)
			})
			.unwrap();
			let bidder_plmc_post_balance = inst.get_free_plmc_balance_for(rejected_bid.bidder);
			let bidder_funding_asset_post_balance =
				inst.get_free_funding_asset_balance_for(rejected_bid.funding_asset.id(), rejected_bid.bidder);
			assert!(inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_1, 2))).is_none());
			assert_eq!(bidder_plmc_post_balance, bidder_plmc_pre_balance + rejected_bid.plmc_bond);
			assert_eq!(
				bidder_funding_asset_post_balance,
				bidder_funding_asset_pre_balance + rejected_bid.funding_asset_amount_locked
			);

			// Any refunds on bids that were accepted/partially accepted will be done at the settlement once funding finishes
			assert_eq!(
				inst.execute(|| Bids::<TestRuntime>::get((project_id, BIDDER_2, 1)).unwrap()).status,
				BidStatus::PartiallyAccepted(32_000 * CT_UNIT)
			);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
			assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

			inst.settle_project(project_id, true);

			inst.do_free_plmc_assertions(expected_free_plmc);
			inst.do_reserved_plmc_assertions(expected_reserved_plmc, HoldReason::Participation.into());
			inst.do_free_funding_asset_assertions(expected_free_funding_assets);

			for (asset, expected_amount) in expected_issuer_funding {
				let real_amount = inst.get_free_funding_asset_balance_for(asset, ISSUER_1);
				assert_eq!(real_amount, expected_amount);
			}
		}

		#[test]
		fn wap_from_different_funding_assets() {
			// From the knowledge hub: https://hub.polimec.org/learn/calculation-example#auction-round-calculation-example
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));

			const ADAM: AccountIdOf<TestRuntime> = 60;
			const TOM: AccountIdOf<TestRuntime> = 61;
			const SOFIA: AccountIdOf<TestRuntime> = 62;
			const FRED: AccountIdOf<TestRuntime> = 63;
			const ANNA: AccountIdOf<TestRuntime> = 64;
			const DAMIAN: AccountIdOf<TestRuntime> = 65;

			let accounts = [ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];
			let mut project_metadata = default_project_metadata(ISSUER_1);
			project_metadata.total_allocation_size = 100_000 * CT_UNIT;
			project_metadata.participation_currencies =
				bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT,];

			// overfund with plmc
			let plmc_fundings = accounts
				.iter()
				.map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: PLMC * 1_000_000 })
				.collect_vec();

			let fundings = [AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT];
			assert_eq!(fundings.len(), AcceptedFundingAsset::VARIANT_COUNT);
			let mut fundings = fundings.into_iter().cycle();

			let usdt_fundings = accounts
				.iter()
				.map(|acc| {
					let accepted_asset = fundings.next().unwrap();
					let asset_id = accepted_asset.id();
					let asset_decimals = inst.execute(|| <TestRuntime as Config>::FundingCurrency::decimals(asset_id));
					let asset_unit = 10u128.checked_pow(asset_decimals.into()).unwrap();
					UserToFundingAsset { account: *acc, asset_amount: asset_unit * 1_000_000, asset_id }
				})
				.collect_vec();
			inst.mint_plmc_to(plmc_fundings);
			inst.mint_funding_asset_to(usdt_fundings);

			let project_id = inst.create_auctioning_project(project_metadata, ISSUER_1, None, default_evaluations());

			let bids = vec![
				(ADAM, 10_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::USDT).into(),
				(TOM, 20_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::USDC).into(),
				(SOFIA, 20_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::DOT).into(),
				(FRED, 10_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::USDT).into(),
				(ANNA, 5_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::USDC).into(),
				(DAMIAN, 5_000 * CT_UNIT, ParticipationMode::Classic(1), AcceptedFundingAsset::DOT).into(),
			];

			inst.bid_for_users(project_id, bids).unwrap();

			assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));

			let token_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
			let normalized_wap =
				PriceProviderOf::<TestRuntime>::convert_back_to_normal_price(token_price, USD_DECIMALS, CT_DECIMALS)
					.unwrap();

			let desired_price = PriceOf::<TestRuntime>::from_float(11.1818f64);

			assert_close_enough!(
				normalized_wap.saturating_mul_int(USD_UNIT),
				desired_price.saturating_mul_int(USD_UNIT),
				Perquintill::from_float(0.99)
			);
		}
	}

	#[cfg(test)]
	mod failure {
		use super::*;

		#[test]
		fn cannot_be_called_early() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());

			let project_details = inst.get_project_details(project_id);
			let now = inst.current_block();
			assert!(now < project_details.round_duration.end().unwrap());

			inst.execute(|| {
				assert_noop!(
					PolimecFunding::end_auction(RuntimeOrigin::signed(420), project_id,),
					Error::<TestRuntime>::TooEarlyForRound
				);
			});
		}

		#[test]
		fn cannot_be_called_twice() {
			let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
			let project_metadata = default_project_metadata(ISSUER_1);
			let project_id =
				inst.create_auctioning_project(project_metadata.clone(), ISSUER_1, None, default_evaluations());

			let project_details = inst.get_project_details(project_id);

			inst.jump_to_block(project_details.round_duration.end().unwrap());

			inst.execute(|| {
				assert_ok!(PolimecFunding::end_auction(RuntimeOrigin::signed(420), project_id,));
				assert_noop!(
					PolimecFunding::end_auction(RuntimeOrigin::signed(420), project_id,),
					Error::<TestRuntime>::IncorrectRound
				);
			});
		}
	}
}
