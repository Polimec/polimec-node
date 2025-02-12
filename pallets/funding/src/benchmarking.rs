// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! Benchmarking setup for Funding pallet

use super::*;
use crate::{instantiator::*, traits::SetPrices};
use polimec_common::assets::AcceptedFundingAsset;

use frame_benchmarking::v2::*;
use frame_support::{
	assert_ok,
	dispatch::RawOrigin,
	traits::{
		fungibles::{metadata::MetadataDeposit, Inspect},
		OriginTrait,
	},
	Parameter,
};
use itertools::Itertools;
use parity_scale_codec::{Decode, Encode};
use polimec_common::{
	assets::AcceptedFundingAsset::{DOT, USDC, USDT, WETH},
	credentials::InvestorType,
	ProvideAssetPrice, USD_DECIMALS, USD_UNIT,
};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{Get, Member, TrailingZeroInput, Zero};

const IPFS_CID: &str = "QmbvsJBhQtu9uAGVp7x4H77JkwAQxV7TA6xTfdeALuDiYB";
const CT_DECIMALS: u8 = 17;
const CT_UNIT: u128 = 10u128.pow(CT_DECIMALS as u32);
type BenchInstantiator<T> = Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;

pub fn usdt_id() -> Location {
	AcceptedFundingAsset::USDT.id()
}

pub fn default_project_metadata<T: Config>(issuer: AccountIdOf<T>) -> ProjectMetadataOf<T>
where
	T::Price: From<u128>,
	T::Hash: From<H256>,
{
	let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
	let metadata_hash = BoundedVec::try_from(IPFS_CID.as_bytes().to_vec()).unwrap();
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
		mainnet_token_max_supply: 1_000_000u128 * CT_UNIT,
		total_allocation_size: 500_000u128 * CT_UNIT,
		minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(10u128.into(), USD_DECIMALS, CT_DECIMALS)
			.unwrap(),

		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(10u128 * USD_UNIT, None),
			institutional: TicketSize::new(10u128 * USD_UNIT, None),
			retail: TicketSize::new(10u128 * USD_UNIT, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![USDT, USDC, DOT, WETH].try_into().unwrap(),
		funding_destination_account: issuer,
		policy_ipfs_cid: Some(metadata_hash.into()),
		participants_account_type: ParticipantsAccountType::Polkadot,
	}
}

/// Grab an account, seeded by a name and index.
pub fn string_account<AccountId: Decode>(
	name: scale_info::prelude::string::String,
	index: u32,
	seed: u32,
) -> AccountId {
	let entropy = (name, index, seed).using_encoded(blake2_256);
	Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
		.expect("infinite length input; no invalid inputs for type; qed")
}

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent> + pallet_balances::Config<Balance = Balance> + sp_std::fmt::Debug,
	<T as Config>::RuntimeEvent: TryInto<Event<T>> + Parameter + Member,
	<T as Config>::Price: From<u128>,
	T::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
	<T as pallet_balances::Config>::Balance: Into<Balance>,
)]
mod benchmarks {
	use super::*;

	// This is actually used in the benchmarking setup, check one line below.
	#[allow(unused_imports)]
	use pallet::Pallet as PalletFunding;
	use polimec_common::credentials::InvestorType::Institutional;

	impl_benchmark_test_suite!(PalletFunding, crate::mock::new_test_ext(), crate::mock::TestRuntime);

	#[benchmark]
	fn create_project() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let ed = inst.get_ed();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);
		let project_metadata = default_project_metadata::<T>(issuer.clone());

		let metadata_deposit = T::ContributionTokenCurrency::calc_metadata_deposit(
			project_metadata.token_information.name.as_slice(),
			project_metadata.token_information.symbol.as_slice(),
		);
		let ct_treasury_account_deposit = T::ContributionTokenCurrency::deposit_required(0);
		inst.mint_plmc_to(vec![UserToPLMCBalance::new(
			issuer.clone(),
			ed * 2u128 + metadata_deposit + ct_treasury_account_deposit,
		)]);
		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		create_project(RawOrigin::Signed(issuer.clone()), jwt, project_metadata.clone());

		// * validity checks *
		// Storage
		let projects_metadata = ProjectsMetadata::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_metadata = &projects_metadata.iter().last().unwrap().1;
		let project_id = projects_metadata.iter().last().unwrap().0;

		assert_eq!(stored_metadata, &project_metadata);

		let project_details = ProjectsDetails::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_details = &project_details.iter().last().unwrap().1;
		assert_eq!(&stored_details.issuer_account, &issuer);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectCreated { project_id, issuer, metadata: stored_metadata.clone() }.into(),
		);
	}

	#[benchmark]
	fn remove_project() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone(), None);
		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		remove_project(RawOrigin::Signed(issuer.clone()), jwt, project_id);

		// * validity checks *
		// Storage
		assert!(ProjectsMetadata::<T>::get(project_id).is_none());
		assert!(ProjectsDetails::<T>::get(project_id).is_none());

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::ProjectRemoved { project_id, issuer }.into());
	}

	#[benchmark]
	fn edit_project() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let issuer_funding = account::<AccountIdOf<T>>("issuer_funding", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone(), None);

		let project_metadata = ProjectMetadataOf::<T> {
			token_information: CurrencyMetadata {
				name: BoundedVec::try_from("Contribution Token TEST v2".as_bytes().to_vec()).unwrap(),
				symbol: BoundedVec::try_from("CTESTv2".as_bytes().to_vec()).unwrap(),
				decimals: CT_DECIMALS - 2,
			},
			mainnet_token_max_supply: 200_000u128 * CT_UNIT,
			total_allocation_size: 100_000u128 * CT_UNIT,
			minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(
				11u128.into(),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				institutional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				retail: TicketSize::new(100 * USD_UNIT, Some(10_000 * USD_UNIT)),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC].try_into().unwrap(),
			funding_destination_account: issuer_funding.clone().clone(),
			policy_ipfs_cid: Some(BoundedVec::try_from(IPFS_CID.as_bytes().to_vec()).unwrap()),
			participants_account_type: ParticipantsAccountType::Ethereum,
		};

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		edit_project(RawOrigin::Signed(issuer), jwt, project_id, project_metadata.clone());

		// * validity checks *
		// Storage
		let stored_metadata = ProjectsMetadata::<T>::get(project_id).unwrap();

		assert_eq!(stored_metadata, project_metadata);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MetadataEdited { project_id, metadata: project_metadata }.into(),
		);
	}

	#[benchmark]
	fn start_evaluation() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.jump_to_block(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone(), None);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);
		#[extrinsic_call]
		start_evaluation(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::EvaluationRound);
		let starting_evaluation_info = EvaluationRoundInfo {
			total_bonded_usd: Balance::zero(),
			total_bonded_plmc: Balance::zero(),
			evaluators_outcome: None,
		};
		assert_eq!(stored_details.evaluation_round_info, starting_evaluation_info);
		let evaluation_transition_points = stored_details.round_duration;
		let evaluation_start: BlockNumberFor<T> = 1u32.into();
		let evaluation_end = <T as Config>::EvaluationRoundDuration::get();

		assert_eq!(evaluation_transition_points.start(), Some(evaluation_start));
		assert_eq!(evaluation_transition_points.end(), Some(evaluation_end));

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ProjectPhaseTransition { project_id, phase: ProjectStatus::EvaluationRound }.into(),
		)
	}

	#[benchmark]
	fn evaluate() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let test_evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
		whitelist_account!(test_evaluator);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

		let extrinsic_evaluation = EvaluationParams::from((test_evaluator.clone(), (1_000 * USD_UNIT).into()));

		let plmc_for_extrinsic_evaluation = inst.calculate_evaluation_plmc_spent(vec![extrinsic_evaluation.clone()]);
		let existential_plmc: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_evaluation.accounts().existential_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(plmc_for_extrinsic_evaluation.clone());

		let extrinsic_plmc_bonded = plmc_for_extrinsic_evaluation[0].plmc_amount;

		let jwt = get_mock_jwt_with_cid(
			extrinsic_evaluation.account.clone(),
			InvestorType::Institutional,
			generate_did_from_account(extrinsic_evaluation.account.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		evaluate(
			RawOrigin::Signed(extrinsic_evaluation.account.clone()),
			jwt,
			project_id,
			extrinsic_evaluation.usd_amount,
		);

		// * validity checks *
		// Storage
		let stored_evaluation =
			Evaluations::<T>::iter_prefix_values((project_id, extrinsic_evaluation.account.clone()))
				.sorted_by(|a, b| a.id.cmp(&b.id))
				.last()
				.unwrap();

		let correct = match stored_evaluation {
			EvaluationInfo { project_id, evaluator, original_plmc_bond, current_plmc_bond, .. }
				if project_id == project_id &&
					evaluator == extrinsic_evaluation.account.clone() &&
					original_plmc_bond == extrinsic_plmc_bonded &&
					current_plmc_bond == extrinsic_plmc_bonded =>
				true,
			_ => false,
		};
		assert!(correct, "Evaluation is not stored correctly");

		// Balances
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![extrinsic_evaluation.account.clone()], HoldReason::Evaluation.into())[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, extrinsic_plmc_bonded);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::Evaluation {
				project_id,
				evaluator: extrinsic_evaluation.account.clone(),
				id: stored_evaluation.id,
				plmc_amount: extrinsic_plmc_bonded,
			}
			.into(),
		);
	}

	// There are 2 logic branches in end_evaluation
	// 1. If the evaluation round is successful
	// 2. If the evaluation round failed
	// 2- only differs by having one additional storage write, so we choose to only benchmark and use this one.
	#[benchmark]
	fn end_evaluation_failure() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone(), None);
		let project_details = inst.get_project_details(project_id);

		let evaluation_usd_target =
			<T as Config>::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
		// we only fund 50% of the minimum threshold for the evaluation round, since we want it to fail
		let evaluations = vec![
			EvaluationParams::from((
				account::<AccountIdOf<T>>("evaluator_1", 0, 0),
				(Percent::from_percent(5) * evaluation_usd_target).into(),
			)),
			EvaluationParams::from((
				account::<AccountIdOf<T>>("evaluator_2", 0, 0),
				(Percent::from_percent(20) * evaluation_usd_target).into(),
			)),
			EvaluationParams::from((
				account::<AccountIdOf<T>>("evaluator_3", 0, 0),
				(Percent::from_percent(25) * evaluation_usd_target).into(),
			)),
		];
		let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone());

		inst.mint_plmc_ed_if_required(plmc_for_evaluating.accounts());
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one());
		inst.evaluate_for_users(project_id, evaluations).expect("All evaluations are accepted");

		let evaluation_end_block = inst.get_project_details(project_id).round_duration.end().unwrap();
		// move block manually without calling any hooks, to avoid triggering the transition outside the benchmarking context
		frame_system::Pallet::<T>::set_block_number(evaluation_end_block);

		// Instead of advancing in time for the automatic `do_evaluation_end` call in on_initialize, we call it directly to benchmark it
		#[block]
		{
			Pallet::<T>::do_end_evaluation(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingFailed);
	}

	#[benchmark]
	fn bid(
		// Amount of buckets this bid is split into
		x: Linear<1, 10>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

		let first_bucket_bid = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 100, 1);
		let first_bucket_bid_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&first_bucket_bid,
			project_metadata.clone(),
			None,
		);
		let first_bucket_bid_funding_asset = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&first_bucket_bid,
				project_metadata.clone(),
				None,
			);
		inst.mint_plmc_ed_if_required(first_bucket_bid_plmc.accounts());
		inst.mint_plmc_to(first_bucket_bid_plmc.clone());
		inst.mint_funding_asset_ed_if_required(first_bucket_bid_funding_asset.to_account_asset_map());
		inst.mint_funding_asset_to(first_bucket_bid_funding_asset.clone());
		inst.bid_for_users(project_id, first_bucket_bid).unwrap();

		let current_bucket = Buckets::<T>::get(project_id).unwrap();

		let ct_amount = (Percent::from_percent(10) * project_metadata.total_allocation_size) * x as u128;

		let extrinsic_bid = BidParams::from((
			bidder.clone(),
			Institutional,
			ct_amount,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		));

		let extrinsic_bid_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&vec![extrinsic_bid.clone()],
			project_metadata.clone(),
			Some(current_bucket),
		);
		let extrinsic_bid_funding_asset = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);

		inst.mint_plmc_ed_if_required(extrinsic_bid_plmc.accounts());
		inst.mint_plmc_to(extrinsic_bid_plmc.clone());
		inst.mint_funding_asset_ed_if_required(extrinsic_bid_funding_asset.to_account_asset_map());
		inst.mint_funding_asset_to(extrinsic_bid_funding_asset.clone());

		let extrinsic_bids_post_bucketing = inst.get_actual_price_charged_for_bucketed_bids(
			&vec![extrinsic_bid.clone()],
			project_metadata.clone(),
			Some(current_bucket),
		);

		let jwt = get_mock_jwt_with_cid(
			bidder.clone(),
			InvestorType::Institutional,
			generate_did_from_account(bidder.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		bid(
			RawOrigin::Signed(bidder.clone()),
			jwt,
			project_id,
			extrinsic_bid.amount,
			extrinsic_bid.mode,
			extrinsic_bid.asset,
		);

		// * validity checks *
		let bids_count = Bids::<T>::iter_prefix_values(project_id).collect_vec().len();
		assert_eq!(bids_count, extrinsic_bids_post_bucketing.len() + 1);

		// Storage
		for (bid_params, price) in extrinsic_bids_post_bucketing.clone() {
			Bids::<T>::iter_prefix_values(project_id)
				.find(|stored_bid| {
					stored_bid.bidder == bidder.clone() &&
						stored_bid.original_ct_amount == bid_params.amount &&
						stored_bid.original_ct_usd_price == price &&
						stored_bid.funding_asset == AcceptedFundingAsset::USDT &&
						stored_bid.mode == bid_params.mode
				})
				.expect("bid not found");
		}
	}

	// We benchmark the worst case, which is a new cutoff being calculated.
	// This doesn't happen when the first bid we read is partially accepted instead of rejected.
	#[benchmark]
	fn process_next_oversubscribed_bid() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let first_bucket_bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 100, 10);
		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

		inst.mint_necessary_tokens_for_bids(project_id, first_bucket_bids.clone());
		inst.bid_for_users(project_id, first_bucket_bids.clone()).unwrap();

		let oversubscribing_bid_amount = first_bucket_bids[9].amount;
		let oversubscribing_bid = BidParams::from((
			bidder.clone(),
			Institutional,
			oversubscribing_bid_amount,
			ParticipationMode::Classic(1u8),
			AcceptedFundingAsset::USDT,
		));
		inst.mint_necessary_tokens_for_bids(project_id, vec![oversubscribing_bid.clone()]);
		inst.bid_for_users(project_id, vec![oversubscribing_bid.clone()]).unwrap();

		Pallet::<T>::do_process_next_oversubscribed_bid(project_id).unwrap();
		let pre_cutoff = OutbidBidsCutoffs::<T>::get(project_id).unwrap();

		inst.mint_necessary_tokens_for_bids(project_id, vec![oversubscribing_bid.clone()]);
		inst.bid_for_users(project_id, vec![oversubscribing_bid.clone()]).unwrap();

		#[extrinsic_call]
		process_next_oversubscribed_bid(RawOrigin::Signed(bidder), project_id);

		// * validity checks *
		let oversubscribed_amount = CTAmountOversubscribed::<T>::get(project_id);
		assert!(oversubscribed_amount.is_zero());

		let rejected_bid = Bids::<T>::get(project_id, 9).unwrap();
		assert_eq!(rejected_bid.status, BidStatus::Rejected);

		let rejected_bid = Bids::<T>::get(project_id, 8).unwrap();
		assert_eq!(rejected_bid.status, BidStatus::Rejected);

		let post_cutoff = OutbidBidsCutoffs::<T>::get(project_id).unwrap();
		assert_ne!(pre_cutoff, post_cutoff);
	}

	// end_funding has 2 logic paths:
	// 1 - Funding successful (most expensive, not by much)
	// 2 - Funding failed
	// They only differ in that 1- has to calculate the evaluator rewards.
	#[benchmark]
	fn end_funding_project_successful() {
		// log something

		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let weth_price = PriceProviderOf::<T>::get_price(WETH.id()).unwrap();
		log::info!("weth_price: {:?}", weth_price);

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer.clone(), None, evaluations);

		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 95, 10);
		let accounts = bids.accounts();
		let necessary_plmc = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);
		let necessary_funding_asset = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
			&bids,
			project_metadata.clone(),
			None,
		);

		inst.mint_plmc_ed_if_required(accounts.clone());
		inst.mint_funding_asset_ed_if_required(necessary_funding_asset.clone().to_account_asset_map());
		inst.mint_plmc_to(necessary_plmc.clone());
		inst.mint_funding_asset_to(necessary_funding_asset.clone());
		inst.bid_for_users(project_id, bids).unwrap();

		let end_block = inst.get_project_details(project_id).round_duration.end().unwrap();
		inst.jump_to_block(end_block);

		#[block]
		{
			Pallet::<T>::do_end_funding(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);
		assert!(matches!(
			project_details.evaluation_round_info.evaluators_outcome,
			Some(EvaluatorsOutcome::Rewarded(_))
		));
	}

	// Success case is the most expensive, so we always charge for that.
	#[benchmark]
	fn start_settlement() {
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let anyone = account::<AccountIdOf<T>>("anyone", 0, 0);
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(anyone);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 95, 30);
		let project_id = inst.create_finished_project(project_metadata, issuer, None, evaluations, bids);

		#[extrinsic_call]
		start_settlement(RawOrigin::Signed(anyone), project_id);

		// * validity checks *
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::Success));
		assert!(<T as Config>::ContributionTokenCurrency::asset_exists(project_id));
	}

	// We have 3 logic paths:
	// 1 - Evaluation rewarded
	// 2 - Evaluation slashed
	// 3 - Evaluation failed (evaluation round unsuccessful)
	// Path 1 is the most expensive but not by far, so we only benchmark and charge for this weight
	#[benchmark]
	fn settle_rewarded_evaluation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let evaluator: AccountIdOf<T> = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 95, 30);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			None,
			evaluations,
			bids,
		);

		let evaluation_to_settle =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());

		assert_ok!(<Pallet<T>>::do_start_settlement(project_id));

		#[extrinsic_call]
		settle_evaluation(RawOrigin::Signed(evaluator.clone()), project_id, evaluator.clone(), evaluation_to_settle.id);

		// * validity checks *
		// Evaluation should be removed
		assert!(Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_settle.id)).is_none());

		// Balances
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let reward_info = match project_details.evaluation_round_info.evaluators_outcome {
			Some(EvaluatorsOutcome::Rewarded(reward_info)) => reward_info,
			_ => panic!("EvaluatorsOutcome should be Rewarded"),
		};
		let reward = Pallet::<T>::calculate_evaluator_reward(&evaluation_to_settle, &reward_info);
		inst.assert_ct_balance(project_id, evaluator.clone(), reward);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationSettled {
				project_id,
				account: evaluator.clone(),
				id: evaluation_to_settle.id,
				ct_rewarded: reward,
				plmc_released: evaluation_to_settle.original_plmc_bond,
			}
			.into(),
		);
	}

	// We have 3 logic paths
	// 1 - Accepted bid with no refunds
	// 2 - Accepted bid with refund (i.e. partially accepted bid )
	// 3 - Rejected bid (i.e. bid not accepted, everything refunded, no CT/migration)
	// Path 2 is the most expensive but not by far, so we only benchmark and charge for this weight
	#[benchmark]
	fn settle_accepted_bid_with_refund() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let increase = project_metadata.minimum_price * PriceOf::<T>::saturating_from_rational(5, 10);
		let target_price = project_metadata.minimum_price + increase;

		let mut new_bucket = Pallet::<T>::create_bucket_from_metadata(&project_metadata).unwrap();
		new_bucket.current_price = target_price;
		new_bucket.amount_left = new_bucket.delta_amount;

		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_bucket(project_metadata.clone(), new_bucket, USDT);

		let project_id =
			inst.create_finished_project(project_metadata.clone(), issuer, None, evaluations, bids.clone());

		assert_ok!(<Pallet<T>>::do_start_settlement(project_id));

		let bid_to_settle = inst.execute(|| {
			let mut bids_iter = Bids::<T>::iter_prefix_values(project_id);
			bids_iter.find(|b| matches!(b.status, BidStatus::PartiallyAccepted(_))).unwrap()
		});

		let bidder = bid_to_settle.bidder.clone();
		whitelist_account!(bidder);

		let BidStatus::PartiallyAccepted(expected_ct_amount) = bid_to_settle.status else {
			unreachable!();
		};

		#[extrinsic_call]
		settle_bid(RawOrigin::Signed(bidder.clone()), project_id, bid_to_settle.id);

		// * validity checks *
		// Storage
		assert!(Bids::<T>::get(project_id, bid_to_settle.id).is_none());

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![bidder.clone()])[0];
		assert_eq!(expected_ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidSettled {
				project_id,
				account: bidder.clone(),
				id: bid_to_settle.id,
				status: bid_to_settle.status,
				final_ct_amount: expected_ct_amount,
			}
			.into(),
		);
	}

	#[benchmark]
	fn mark_project_as_settled() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let anyone = account::<AccountIdOf<T>>("anyone", 0, 0);
		whitelist_account!(anyone);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 95, 30);

		let project_id =
			inst.create_settled_project(project_metadata.clone(), issuer.clone(), None, evaluations, bids, false);

		#[extrinsic_call]
		mark_project_as_settled(RawOrigin::Signed(anyone), project_id);

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::SettlementFinished(FundingOutcome::Success));
	}

	#[benchmark]
	fn start_offchain_migration() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 34, 30);
		let project_id =
			inst.create_settled_project(project_metadata.clone(), issuer.clone(), None, evaluations, bids, true);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		start_offchain_migration(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
		assert_eq!(UnmigratedCounter::<T>::get(project_id), 40);
	}

	#[benchmark]
	fn confirm_offchain_migration(
		// Amount of migrations to confirm for a single user
		x: Linear<1, 100>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let participant = account::<AccountIdOf<T>>("test_participant", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());

		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);

		let mut bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 34, 50);
		let max_bids = x;
		let participant_bids = (0..max_bids)
			.map(|_| {
				BidParams::from((
					participant.clone(),
					Institutional,
					(50 * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				))
			})
			.collect_vec();
		bids.extend(participant_bids);

		let project_id =
			inst.create_settled_project(project_metadata.clone(), issuer.clone(), None, evaluations, bids, true);

		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementFinished(FundingOutcome::Success)
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_offchain_migration(RawOrigin::Signed(issuer.clone()).into(), jwt.clone(), project_id)
			.unwrap();

		let participant_migrations = UserMigrations::<T>::get((project_id, participant.clone()));
		let participant_migrations_len = participant_migrations.unwrap().1.len();
		assert_eq!(participant_migrations_len as u32, x);

		#[extrinsic_call]
		confirm_offchain_migration(RawOrigin::Signed(issuer), project_id, participant);

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);

		assert_eq!(UnmigratedCounter::<T>::get(project_id), 10 + 50);
	}

	#[benchmark]
	fn mark_project_ct_migration_as_finished() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 34, 30);
		let project_id =
			inst.create_settled_project(project_metadata.clone(), issuer.clone(), None, evaluations, bids, true);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_offchain_migration(RawOrigin::Signed(issuer.clone()).into(), jwt.clone(), project_id)
			.unwrap();

		let participants = UserMigrations::<T>::iter_key_prefix((project_id,)).collect_vec();
		for participant in participants {
			<crate::Pallet<T>>::confirm_offchain_migration(
				RawOrigin::Signed(issuer.clone().clone()).into(),
				project_id,
				participant,
			)
			.unwrap()
		}

		#[extrinsic_call]
		mark_project_ct_migration_as_finished(RawOrigin::Signed(issuer), project_id);

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationFinished);
		assert_eq!(UnmigratedCounter::<T>::get(project_id), 0);
		assert_eq!(
			UserMigrations::<T>::iter_prefix_values((project_id,))
				.map(|item| item.0)
				.all(|status| status == MigrationStatus::Confirmed),
			true
		);
	}
}
