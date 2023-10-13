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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::instantiator::*;
use polimec_traits::ReleaseSchedule;
use frame_benchmarking::v2::*;
use frame_support::{dispatch::RawOrigin, traits::{OriginTrait, fungibles::Inspect}, Parameter};
#[allow(unused_imports)]
use pallet::Pallet as PalletFunding;
use scale_info::prelude::format;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Get, Member};
use sp_std::marker::PhantomData;
const METADATA: &str = r#"
{
    "whitepaper":"ipfs_url",
    "team_description":"ipfs_url",
    "tokenomics":"ipfs_url",
    "roadmap":"ipfs_url",
    "usage_of_founds":"ipfs_url"
}
"#;
const EDITED_METADATA: &str = r#"
{
    "whitepaper":"new_ipfs_url",
    "team_description":"new_ipfs_url",
    "tokenomics":"new_ipfs_url",
    "roadmap":"new_ipfs_url",
    "usage_of_founds":"new_ipfs_url"
}
"#;

const ASSET_DECIMALS: u8 = 10;
const US_DOLLAR: u128 = 1_0_000_000_000u128;
const ASSET_UNIT: u128 = 1_0_000_000_000u128;

pub fn usdt_id() -> u32 {
	AcceptedFundingAsset::USDT.to_statemint_id()
}
pub fn hashed(data: impl AsRef<[u8]>) -> H256 {
	<BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
}

pub fn default_project<T: Config>(nonce: u64, issuer: AccountIdOf<T>) -> ProjectMetadataOf<T>
where
	T::Price: From<u128>,
	T::Hash: From<H256>,
{
	let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
	let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: ASSET_DECIMALS },
		mainnet_token_max_supply: BalanceOf::<T>::try_from(8_000_000_0_000_000_000u128)
			.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		total_allocation_size: (
			BalanceOf::<T>::try_from(50_000_0_000_000_000u128).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
			BalanceOf::<T>::try_from(50_000_0_000_000_000u128).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		),
		minimum_price: 1u128.into(),
		ticket_size: TicketSize {
			minimum: Some(1u128.try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf"))),
			maximum: None,
		},
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		funding_thresholds: Default::default(),
		conversion_rate: 0,
		participation_currencies: AcceptedFundingAsset::USDT,
		funding_destination_account: issuer,
		offchain_information_hash: Some(metadata_hash.into()),
	}
}

pub fn default_evaluations<T: Config>() -> Vec<UserToUSDBalance<T>>
where
	<T as Config>::Balance: From<u128>,
{
	vec![
		UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_1", 0, 0), (50_000 * US_DOLLAR).into()),
		UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_2", 0, 0), (25_000 * US_DOLLAR).into()),
		UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_3", 0, 0), (32_000 * US_DOLLAR).into()),
	]
}

pub fn default_bids<T: Config>() -> Vec<BidParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
{
	vec![
		BidParams::new(
			account::<AccountIdOf<T>>("bidder_1", 0, 0),
			(40_000 * ASSET_UNIT).into(),
			1_u128.into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		BidParams::new(
			account::<AccountIdOf<T>>("bidder_2", 0, 0),
			(5_000 * ASSET_UNIT).into(),
			1_u128.into(),
			7u8,
			AcceptedFundingAsset::USDT,
		),
	]
}

pub fn default_community_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
{
	vec![
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_1", 0, 0),
			(100 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_2", 0, 0),
			(200 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_3", 0, 0),
			(2000 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
	]
}

pub fn default_weights() -> Vec<u8> {
	vec![20u8, 15u8, 10u8, 25u8, 30u8]
}

pub fn default_bidders<T: Config>() -> Vec<AccountIdOf<T>> {
	vec![
		account::<AccountIdOf<T>>("bidder_1", 0, 0),
		account::<AccountIdOf<T>>("bidder_2", 0, 0),
		account::<AccountIdOf<T>>("bidder_3", 0, 0),
		account::<AccountIdOf<T>>("bidder_4", 0, 0),
		account::<AccountIdOf<T>>("bidder_5", 0, 0),
	]
}

pub fn default_contributors<T: Config>() -> Vec<AccountIdOf<T>> {
	vec![
		account::<AccountIdOf<T>>("contributor_1", 0, 0),
		account::<AccountIdOf<T>>("contributor_2", 0, 0),
		account::<AccountIdOf<T>>("contributor_3", 0, 0),
		account::<AccountIdOf<T>>("contributor_4", 0, 0),
		account::<AccountIdOf<T>>("contributor_5", 0, 0),
	]
}

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent> + pallet_balances::Config<Balance = BalanceOf<T>>,
	<T as Config>::RuntimeEvent: TryInto<Event<T>> + Parameter + Member,
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
	<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
)]
mod benchmarks {
	use super::*;
	use itertools::Itertools;

	impl_benchmark_test_suite!(PalletFunding, crate::mock::new_test_ext(), crate::mock::TestRuntime);

	type BenchInstantiator<T> = Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;
	// #[benchmark]
	// fn create() {
	// 	// * setup *
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();

	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	whitelist_account!(issuer);

	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

	// 	#[extrinsic_call]
	// 	create(RawOrigin::Signed(issuer.clone()), project_metadata.clone());

	// 	// * validity checks *
	// 	// Storage
	// 	let projects_metadata = ProjectsMetadata::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
	// 	let stored_metadata = projects_metadata.iter().last().unwrap().1.clone();
	// 	let project_id = projects_metadata.iter().last().unwrap().0;
	// 	assert_eq!(stored_metadata, project_metadata);

	// 	let project_details = ProjectsDetails::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
	// 	let stored_details = project_details.iter().last().unwrap().1.clone();
	// 	assert_eq!(stored_details.issuer, issuer.clone());

	// 	// Events
	// 	frame_system::Pallet::<T>::assert_last_event(Event::<T>::ProjectCreated { project_id, issuer }.into());
	// }
	//
	// #[benchmark]
	// fn edit_metadata() {
	// 	// * setup *
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	whitelist_account!(issuer);
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone());
	// 	let original_metadata_hash = project_metadata.clone().offchain_information_hash.unwrap();
	// 	let edited_metadata_hash: H256 = hashed(EDITED_METADATA);
	//
	// 	#[extrinsic_call]
	// 	edit_metadata(RawOrigin::Signed(issuer.clone()), project_id, edited_metadata_hash.into());
	//
	// 	// * validity checks *
	// 	// Storage
	// 	let stored_metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
	// 	assert_eq!(stored_metadata.offchain_information_hash, Some(edited_metadata_hash.into()));
	// 	assert!(original_metadata_hash != edited_metadata_hash.into());
	//
	// 	// Events
	// 	frame_system::Pallet::<T>::assert_last_event(Event::<T>::MetadataEdited { project_id }.into());
	// }

	// #[benchmark]
	// fn start_evaluation() {
	// 	// * setup *
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	whitelist_account!(issuer);
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let project_id = inst.create_new_project(project_metadata, issuer.clone());
	//
	// 	#[extrinsic_call]
	// 	start_evaluation(RawOrigin::Signed(issuer.clone()), project_id);
	//
	// 	// * validity checks *
	// 	// Storage
	// 	let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
	// 	assert_eq!(stored_details.status, ProjectStatus::EvaluationRound);
	// 	let starting_evaluation_info = EvaluationRoundInfoOf::<T> {
	// 		total_bonded_usd: Zero::zero(),
	// 		total_bonded_plmc: Zero::zero(),
	// 		evaluators_outcome: EvaluatorsOutcome::Unchanged,
	// 	};
	// 	assert_eq!(stored_details.evaluation_round_info, starting_evaluation_info);
	// 	let evaluation_transition_points = stored_details.phase_transition_points.evaluation;
	// 	match evaluation_transition_points {
	// 		BlockNumberPair {start: Some(_), end: Some(_)} => {},
	// 		_ => assert!(false, "Evaluation transition points are not set"),
	// 	}
	// }

	// #[benchmark]
	// fn bond_evaluation() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
	// 	whitelist_account!(evaluator);
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());
	//
	// 	let evaluation = UserToUSDBalance::new(evaluator.clone(), (50_000 * US_DOLLAR).into());
	//
	// 	let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
	// 	let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();
	//
	// 	inst.mint_plmc_to(existential_plmc);
	// 	inst.mint_plmc_to(plmc_for_evaluating.clone());
	//
	// 	inst.advance_time(One::one()).unwrap();
	//
	// 	#[extrinsic_call]
	// 	bond_evaluation(RawOrigin::Signed(evaluator.clone()), project_id, evaluation.clone().usd_amount);
	//
	// 	// * validity checks *
	// 	// Storage
	// 	let stored_evaluation = Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone()))
	// 		.sorted_by(|a, b| a.id.cmp(&b.id))
	// 		.last()
	// 		.unwrap();
	//
	// 	match stored_evaluation {
	// 		EvaluationInfo {
	// 			project_id,
	// 			evaluator,
	// 			original_plmc_bond,
	// 			current_plmc_bond,
	// 			rewarded_or_slashed,
	// 			..
	// 		} if project_id == project_id &&
	// 			evaluator == evaluator.clone() &&
	// 			original_plmc_bond == plmc_for_evaluating[0].plmc_amount &&
	// 			current_plmc_bond == plmc_for_evaluating[0].plmc_amount &&
	// 			rewarded_or_slashed == false => {},
	// 		_ => assert!(false, "Evaluation is not stored correctly"),
	// 	}
	//
	// 	// Balances
	// 	let bonded_plmc = inst
	// 		.get_reserved_plmc_balances_for(vec![evaluator.clone()], LockType::Evaluation(project_id))[0]
	// 		.plmc_amount;
	// 	assert_eq!(bonded_plmc, plmc_for_evaluating[0].plmc_amount);
	//
	// 	// Events
	// 	frame_system::Pallet::<T>::assert_last_event(
	// 		Event::<T>::FundsBonded {
	// 			project_id,
	// 			amount: plmc_for_evaluating[0].plmc_amount,
	// 			bonder: evaluator.clone(),
	// 		}
	// 		.into(),
	// 	);
	// }


	// #[benchmark]
	// fn start_auction() {
	// 	// * setup *
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	whitelist_account!(issuer);
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());
	//
	// 	let evaluations = default_evaluations();
	// 	let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(evaluations.clone());
	// 	let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();
	//
	// 	inst.mint_plmc_to(existential_plmc);
	// 	inst.mint_plmc_to(plmc_for_evaluating);
	//
	// 	inst.advance_time(One::one()).unwrap();
	// 	inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");
	//
	// 	inst.advance_time(<T as Config>::EvaluationDuration::get() + One::one()).unwrap();
	//
	// 	#[extrinsic_call]
	// 	start_auction(RawOrigin::Signed(issuer.clone()), project_id);
	//
	// 	// * validity checks *
	// 	// Storage
	// 	let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
	// 	assert_eq!(stored_details.status, ProjectStatus::AuctionRound(AuctionPhase::English));
	//
	// 	// Events
	// 	frame_system::Pallet::<T>::assert_last_event(Event::<T>::EnglishAuctionStarted { project_id, when: 8414u32.into() }.into());
	// }


	#[benchmark]
	fn bid() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);
		
		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		
		let project_id = inst.create_auctioning_project(
			project_metadata.clone(),
			issuer,
			default_evaluations::<T>(),
		);

		let bid_params = BidParams::new(
			bidder.clone(),
			(50000u128 * ASSET_UNIT).into(),
			1_u128.into(),
			1u8,
			AcceptedFundingAsset::USDT,
		);
		let necessary_plmc: Vec<UserToPLMCBalance<T>> = BenchInstantiator::<T>::calculate_auction_plmc_spent(vec![bid_params.clone()]);
		let existential_deposits: Vec<UserToPLMCBalance<T>> = necessary_plmc.accounts().existential_deposits();
		let necessary_usdt: Vec<UserToStatemintAsset<T>> = BenchInstantiator::<T>::calculate_auction_funding_asset_spent(vec![bid_params.clone()]);

		inst.mint_plmc_to(necessary_plmc.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_statemint_asset_to(necessary_usdt.clone());

		#[extrinsic_call]
		bid(
			RawOrigin::Signed(bidder.clone()),
			project_id,
			bid_params.amount,
			bid_params.price,
			bid_params.multiplier,
			bid_params.asset,
		);

		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::iter_prefix_values((project_id, bidder.clone()))
			.sorted_by(|a, b| a.id.cmp(&b.id))
			.last()
			.unwrap();
		let bid_filter = BidInfoFilter::<T> {
			id: None,
			project_id: Some(project_id),
			bidder: Some(bidder.clone()),
			status: Some(BidStatus::YetUnknown),
			original_ct_amount: Some(bid_params.amount),
			original_ct_usd_price: Some(bid_params.price),
			final_ct_amount: Some(bid_params.amount),
			final_ct_usd_price: Some(bid_params.price),
			funding_asset: Some(AcceptedFundingAsset::USDT),
			funding_asset_amount_locked: Some(necessary_usdt[0].asset_amount),
			multiplier: Some(bid_params.multiplier),
			plmc_bond: Some(necessary_plmc[0].plmc_amount),
			plmc_vesting_info: Some(None),
			when: None,
			funds_released: Some(false),
			ct_minted: Some(false)
		};
		assert!(bid_filter.matches_bid(&stored_bid));

		// Bucket Storage Check
		let bucket_delta_amount = Percent::from_percent(10) * project_metadata.total_allocation_size.0;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).unwrap();
		let bucket_delta_price: <T as Config>::Price =
			project_metadata.minimum_price.saturating_mul(ten_percent_in_price);

		let mut starting_bucket = Bucket::new(
			project_metadata.total_allocation_size.0,
			project_metadata.minimum_price,
			ten_percent_in_price,
			bucket_delta_amount,
		);

		starting_bucket.update(bid_params.amount);


		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		assert_eq!(current_bucket,
			starting_bucket
		);

		// Balances
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![bidder.clone()], LockType::Participation(project_id))[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, necessary_plmc[0].plmc_amount);

		let free_plmc = inst.get_free_plmc_balances_for(vec![bidder.clone()])[0].plmc_amount;
		assert_eq!(free_plmc, existential_deposits[0].plmc_amount);

		let free_usdt = inst.get_free_statemint_asset_balances_for(usdt_id(), vec![bidder.clone()])[0].asset_amount;
		assert_eq!(free_usdt, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::Bid { project_id, amount: bid_params.amount, price: bid_params.price, multiplier: bid_params.multiplier }.into()
		);
	}

	#[benchmark]
	fn contribute() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();
	
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributor = account::<AccountIdOf<T>>("contributor", 0, 0);
		whitelist_account!(contributor);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			default_evaluations::<T>(),
			default_bids::<T>(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let contribution_params =
			ContributionParams::new(contributor.clone(), (100 * ASSET_UNIT).into(), 1u8, AcceptedFundingAsset::USDT);
		let necessary_plmc =
			BenchInstantiator::<T>::calculate_contributed_plmc_spent(vec![contribution_params.clone()], price);
		let existential_deposits: Vec<UserToPLMCBalance<T>> = necessary_plmc.accounts().existential_deposits();
		let necessary_usdt =
			BenchInstantiator::<T>::calculate_contributed_funding_asset_spent(vec![contribution_params.clone()], price);

		inst.mint_plmc_to(necessary_plmc.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_statemint_asset_to(necessary_usdt.clone());

		let contribution_id = NextContributionId::<T>::get();

		#[extrinsic_call]
		contribute(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contribution_params.amount,
			contribution_params.multiplier,
			contribution_params.asset,
		);
	
		// * validity checks *
		// Storage
		let stored_contribution = Contributions::<T>::iter_prefix_values((project_id, contributor.clone()))
		.sorted_by(|a, b| a.id.cmp(&b.id))
		.last()
		.unwrap();

		let contribution = ContributionInfoOf::<T> {
			id: contribution_id,
			project_id: project_id,
			contributor: contributor.clone(),
			ct_amount: contribution_params.amount,
			usd_contribution_amount: necessary_usdt[0].asset_amount,
			multiplier: contribution_params.multiplier,
			funding_asset: contribution_params.asset,
			funding_asset_amount: necessary_usdt[0].asset_amount,
			plmc_bond: necessary_plmc[0].plmc_amount,
			plmc_vesting_info: None,
			funds_released: false,
			ct_minted: false,
		};
		assert_eq!(stored_contribution, contribution);

		assert_eq!(NextContributionId::<T>::get(), contribution_id.saturating_add(One::one()));

		let stored_project_details = ProjectsDetails::<T>::get(project_id).unwrap();

		assert_eq!(
			stored_project_details.remaining_contribution_tokens.1,
			project_metadata.total_allocation_size.1.saturating_sub(contribution_params.amount)
		);

		// Balances
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![contributor.clone()], LockType::Participation(project_id))[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, necessary_plmc[0].plmc_amount);

		let free_plmc = inst.get_free_plmc_balances_for(vec![contributor.clone()])[0].plmc_amount;
		assert_eq!(free_plmc, existential_deposits[0].plmc_amount);

		let free_usdt = inst.get_free_statemint_asset_balances_for(usdt_id(), vec![contributor.clone()])[0].asset_amount;
		assert_eq!(free_usdt, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::Contribution {
				project_id,
				contributor: contributor.clone(),
				amount: contribution_params.amount,
				multiplier: contribution_params.multiplier,
			}.into()
		);
	}

	#[benchmark]
	fn evaluation_unbond_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();
		let evaluator = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let evaluation_to_unbond =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());

		inst.execute(|| {
			PalletFunding::<T>::evaluation_reward_payout_for(
				<T as frame_system::Config>::RuntimeOrigin::signed(evaluator.clone().into()),
				project_id,
				evaluator.clone(),
				evaluation_to_unbond.id,
			)
			.expect("")
		});

		#[extrinsic_call]
		evaluation_unbond_for(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			evaluation_to_unbond.id,
		);

		// * validity checks *
		// Storage
		assert!(Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().is_none());

		// Balance
		let bonded_plmc = inst.get_reserved_plmc_balances_for(vec![evaluator.clone()], LockType::Evaluation(project_id))[0].plmc_amount;
		assert_eq!(bonded_plmc, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BondReleased {
				project_id,
				amount: evaluation_to_unbond.current_plmc_bond,
				bonder: evaluator.clone(),
				releaser: evaluator.clone(),
			}.into()
		);
	}

	#[benchmark]
	fn evaluation_slash_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();
		let evaluator = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let bids = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer, evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let evaluation_to_unbond =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());

		#[extrinsic_call]
		evaluation_slash_for(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			evaluation_to_unbond.id,
		);

		// * validity checks *
		// Storage
		let stored_evaluation = Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_unbond.id)).unwrap();
		assert!(stored_evaluation.rewarded_or_slashed);
		let slashed_amount = T::EvaluatorSlash::get() * evaluation_to_unbond.original_plmc_bond;
		let current_plmc_bond = evaluation_to_unbond.current_plmc_bond
			.saturating_sub(slashed_amount);
		assert_eq!(stored_evaluation.current_plmc_bond, current_plmc_bond);

		// Balance
		let treasury_account = T::TreasuryAccount::get();
		let bonded_plmc = inst.get_reserved_plmc_balances_for(vec![evaluator.clone()], LockType::Evaluation(project_id))[0].plmc_amount;
		assert_eq!(bonded_plmc, stored_evaluation.current_plmc_bond);
		let free_treasury_plmc = inst.get_free_plmc_balances_for(vec![treasury_account.clone()])[0].plmc_amount;
		assert_eq!(free_treasury_plmc, slashed_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationSlashed {
				project_id,
				evaluator: evaluator.clone(),
				id: stored_evaluation.id,
				amount: slashed_amount,
				caller: evaluator.clone(),
			}.into()
		);
	}

	#[benchmark]
	fn evaluation_reward_payout_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();
	
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();
		let evaluator = evaluations[0].account.clone();
		whitelist_account!(evaluator);
	
		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);
	
		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
	
		let evaluation_to_unbond =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());
	
		#[extrinsic_call]
		evaluation_reward_payout_for(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			evaluation_to_unbond.id,
		);
	
		// * validity checks *
		// Storage
		let stored_evaluation = Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_unbond.id)).unwrap();
		assert!(stored_evaluation.rewarded_or_slashed);

		// Balances
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let reward_info = match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(reward_info) => reward_info,
			_ => panic!("EvaluatorsOutcome should be Rewarded")
		};
		let total_reward = BenchInstantiator::<T>::calculate_total_reward_for_evaluation(stored_evaluation.clone(), reward_info);
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![evaluator.clone()])[0];
		assert_eq!(ct_amount, total_reward);
		
		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationRewarded {
				project_id,
				evaluator: evaluator.clone(),
				id: stored_evaluation.id,
				amount: total_reward,
				caller: evaluator.clone(),
			}.into()
		);
	}
	
	#[benchmark]
	fn bid_ct_mint_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bids = default_bids::<T>();
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
	
		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			bids,
			default_community_contributions::<T>(),
			vec![],
		);

		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let bid_to_mint_ct =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		bid_ct_mint_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bid_to_mint_ct.id);

		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::get((project_id, bidder.clone(), bid_to_mint_ct.id)).unwrap();
		assert!(stored_bid.ct_minted);

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![bidder.clone()])[0];
		assert_eq!(stored_bid.final_ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionTokenMinted {
				releaser: bidder.clone(),
				project_id: project_id,
				claimer: bidder.clone(),
				amount: ct_amount,
			}.into()
		);
	}

	#[benchmark]
	fn contribution_ct_mint_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();
	
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributions = default_community_contributions::<T>();
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);
	
		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			default_bids::<T>(),
			contributions,
			vec![],
		);
	
		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
	
		let contribution_to_mint_ct =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());
	
		#[extrinsic_call]
		contribution_ct_mint_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			contribution_to_mint_ct.id,
		);

		// * validity checks *
		// Storage
		let stored_contribution = Contributions::<T>::get((project_id, contributor.clone(), contribution_to_mint_ct.id)).unwrap();
		assert!(stored_contribution.ct_minted);

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![contributor.clone()])[0];
		assert_eq!(stored_contribution.ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionTokenMinted {
				releaser: contributor.clone(),
				project_id: project_id,
				claimer: contributor.clone(),
				amount: ct_amount,
			}.into()
		);
	}

	#[benchmark]
	fn start_bid_vesting_schedule_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bids = default_bids::<T>();
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);

		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			bids,
			default_community_contributions::<T>(),
			vec![],
		);

		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);
	
		let bid_to_vest = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
	
		#[extrinsic_call]
		start_bid_vesting_schedule_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bid_to_vest.id);
	
		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::get((project_id, bidder.clone(), bid_to_vest.id)).unwrap();
		assert!(stored_bid.plmc_vesting_info.is_some());
		let vest_info = stored_bid.plmc_vesting_info.unwrap();
		let total_vested = T::Vesting::total_scheduled_amount(&bidder, LockType::Participation(project_id)).unwrap();
		assert_eq!(vest_info.total_amount, total_vested);
		
		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidPlmcVestingScheduled {
				project_id,
				bidder: bidder.clone(),
				id: stored_bid.id,
				amount: vest_info.total_amount,
				caller: bidder.clone(),
			}.into()
		);
	}

	#[benchmark]
	fn start_contribution_vesting_schedule_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();
	
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributions = default_community_contributions::<T>();
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);
	
		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			default_bids::<T>(),
			contributions,
			vec![],
		);

		inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let contribution_to_vest =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		start_contribution_vesting_schedule_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			contribution_to_vest.id,
		);

		// * validity checks *
		// Storage
		let stored_contribution = Contributions::<T>::get((project_id, contributor.clone(), contribution_to_vest.id)).unwrap();
		assert!(stored_contribution.plmc_vesting_info.is_some());
		let vest_info = stored_contribution.plmc_vesting_info.unwrap();
		let total_vested = T::Vesting::total_scheduled_amount(&contributor, LockType::Participation(project_id)).unwrap();
		assert_eq!(vest_info.total_amount, total_vested);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionPlmcVestingScheduled {
				project_id,
				contributor: contributor.clone(),
				id: stored_contribution.id,
				amount: vest_info.total_amount,
				caller: contributor.clone(),
			}.into()
		);
	}
	//
	// #[benchmark]
	// fn payout_bid_funds_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let bids = default_bids::<T>();
	// 	let bidder = bids[0].bidder.clone();
	// 	whitelist_account!(bidder);
	//
	// 	let project_id = inst.create_finished_project(
	// 		default_project::<T>(inst.get_new_nonce(), issuer.clone()),
	// 		issuer.clone(),
	// 		default_evaluations::<T>(),
	// 		bids,
	// 		default_community_contributions::<T>(),
	// 		vec![],
	// 	);
	//
	// 	inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Success(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
	//
	// 	#[extrinsic_call]
	// 	payout_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder.clone(), stored_bid.id)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn payout_contribution_funds_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let contributions = default_community_contributions::<T>();
	// 	let contributor = contributions[0].contributor.clone();
	// 	whitelist_account!(contributor);
	//
	// 	let project_id = inst.create_finished_project(
	// 		default_project::<T>(inst.get_new_nonce(), issuer.clone()),
	// 		issuer.clone(),
	// 		default_evaluations::<T>(),
	// 		default_bids::<T>(),
	// 		contributions,
	// 		vec![],
	// 	);
	//
	// 	inst.advance_time(<T as Config>::SuccessToSettlementTime::get()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Success(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_contribution =
	// 		inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());
	//
	// 	#[extrinsic_call]
	// 	payout_contribution_funds_for(
	// 		RawOrigin::Signed(issuer.clone()),
	// 		project_id,
	// 		contributor.clone(),
	// 		stored_contribution.id,
	// 	)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn decide_project_outcome() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluations = default_evaluations::<T>();
	// 	let evaluator = evaluations[0].account.clone();
	// 	whitelist_account!(evaluator);
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);
	//
	// 	let bids = BenchInstantiator::generate_bids_from_total_usd(
	// 		Percent::from_percent(40) * target_funding_amount,
	// 		1u128.into(),
	// 		default_weights(),
	// 		default_bidders::<T>(),
	// 	);
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.1);
	// 	let contributions = BenchInstantiator::generate_contributions_from_total_usd(
	// 		Percent::from_percent(40) * target_funding_amount,
	// 		BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
	// 		default_weights(),
	// 		default_contributors::<T>(),
	// 	);
	//
	// 	let project_id =
	// 		inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);
	//
	// 	inst.advance_time(One::one()).unwrap();
	//
	// 	#[extrinsic_call]
	// 	decide_project_outcome(RawOrigin::Signed(issuer.clone()), project_id, FundingOutcomeDecision::AcceptFunding)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn release_bid_funds_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluations = default_evaluations::<T>();
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);
	//
	// 	let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
	// 		Percent::from_percent(15) * target_funding_amount,
	// 		10u128.into(),
	// 		default_weights(),
	// 		default_bidders::<T>(),
	// 	);
	// 	let bidder = bids[0].bidder.clone();
	// 	whitelist_account!(bidder);
	// 	let contributions = BenchInstantiator::generate_contributions_from_total_usd(
	// 		Percent::from_percent(10) * target_funding_amount,
	// 		BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
	// 		default_weights(),
	// 		default_contributors::<T>(),
	// 	);
	//
	// 	let project_id =
	// 		inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);
	//
	// 	inst.advance_time(One::one()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Failure(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
	//
	// 	#[extrinsic_call]
	// 	release_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder, stored_bid.id)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn bid_unbond_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluations = default_evaluations::<T>();
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);
	//
	// 	let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
	// 		Percent::from_percent(15) * target_funding_amount,
	// 		10u128.into(),
	// 		default_weights(),
	// 		default_bidders::<T>(),
	// 	);
	// 	let bidder = bids[0].bidder.clone();
	// 	whitelist_account!(bidder);
	// 	let contributions = BenchInstantiator::generate_contributions_from_total_usd(
	// 		Percent::from_percent(10) * target_funding_amount,
	// 		BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
	// 		default_weights(),
	// 		default_contributors::<T>(),
	// 	);
	//
	// 	let project_id =
	// 		inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);
	//
	// 	inst.advance_time(One::one()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Failure(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
	//
	// 	inst.execute(|| {
	// 		PalletFunding::<T>::release_bid_funds_for(
	// 			<T as frame_system::Config>::RuntimeOrigin::signed(bidder.clone().into()),
	// 			project_id,
	// 			bidder.clone(),
	// 			stored_bid.id,
	// 		)
	// 		.expect("Funds are released")
	// 	});
	//
	// 	#[extrinsic_call]
	// 	bid_unbond_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), stored_bid.id)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn release_contribution_funds_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluations = default_evaluations::<T>();
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);
	//
	// 	let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
	// 		Percent::from_percent(15) * target_funding_amount,
	// 		1u128.into(),
	// 		default_weights(),
	// 		default_bidders::<T>(),
	// 	);
	// 	let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
	// 		Percent::from_percent(10) * target_funding_amount,
	// 		BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
	// 		default_weights(),
	// 		default_contributors::<T>(),
	// 	);
	// 	let contributor = contributions[0].contributor.clone();
	// 	whitelist_account!(contributor);
	//
	// 	let project_id =
	// 		inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);
	//
	// 	inst.advance_time(One::one()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Failure(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_contribution =
	// 		inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());
	//
	// 	#[extrinsic_call]
	// 	release_contribution_funds_for(
	// 		RawOrigin::Signed(contributor.clone()),
	// 		project_id,
	// 		contributor.clone(),
	// 		stored_contribution.id,
	// 	)
	//
	// 	// validity checks
	// }
	//
	// #[benchmark]
	// fn contribution_unbond_for() {
	// 	// setup
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	// real benchmark starts at block 0, and we can't call `events()` at block 0
	// 	inst.advance_time(1u32.into()).unwrap();
	//
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	let evaluations = default_evaluations::<T>();
	//
	// 	let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
	// 	let target_funding_amount: BalanceOf<T> =
	// 		project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);
	//
	// 	let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
	// 		Percent::from_percent(15) * target_funding_amount,
	// 		1u128.into(),
	// 		default_weights(),
	// 		default_bidders::<T>(),
	// 	);
	// 	let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
	// 		Percent::from_percent(10) * target_funding_amount,
	// 		BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
	// 		default_weights(),
	// 		default_contributors::<T>(),
	// 	);
	// 	let contributor = contributions[0].contributor.clone();
	// 	whitelist_account!(contributor);
	//
	// 	let project_id =
	// 		inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);
	//
	// 	inst.advance_time(One::one()).unwrap();
	// 	assert_eq!(
	// 		inst.get_project_details(project_id).cleanup,
	// 		Cleaner::Failure(CleanerState::Initialized(PhantomData))
	// 	);
	//
	// 	let stored_contribution =
	// 		inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());
	//
	// 	inst.execute(|| {
	// 		PalletFunding::<T>::release_contribution_funds_for(
	// 			<T as frame_system::Config>::RuntimeOrigin::signed(contributor.clone().into()),
	// 			project_id,
	// 			contributor.clone(),
	// 			stored_contribution.id,
	// 		)
	// 		.expect("Funds are released")
	// 	});
	//
	// 	#[extrinsic_call]
	// 	contribution_unbond_for(
	// 		RawOrigin::Signed(issuer.clone()),
	// 		project_id,
	// 		contributor.clone(),
	// 		stored_contribution.id,
	// 	);
	//
	// 	// validity checks
	// }

	// #[benchmark]
	// fn test(){
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	// 	inst.advance_time(5u32.into()).unwrap();
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	frame_system::Pallet::<T>::remark_with_event(RawOrigin::Signed(issuer.clone()).into(), vec![1u8,2u8,3u8,4u8]);
	//
	// 	let debug_events = frame_system::Pallet::<T>::events();
	// 	if debug_events.len() == 0 {
	// 		panic!("events in store: {:?}", debug_events.len());
	// 	}
	//
	// 	#[block]
	// 	{
	//
	// 	}
	//
	// 	let debug_events = frame_system::Pallet::<T>::events();
	// 	log::info!(
	// 		"frame system default events {:?}",
	// 		debug_events
	// 	);
	// }

	// #[macro_export]
	// macro_rules! find_event {
	// 	($env: expr, $pattern:pat) => {
	// 		$env.execute(|| {
	// 			let events: Vec<frame_system::EventRecord<<T as Config>::RuntimeEvent, T::Hash>> = frame_system::Pallet::<T>::events();
	//
	// 			events.iter().find_map(|event_record| {
	// 				let runtime_event = event_record.event.clone();
	// 				if let Ok(eve) = runtime_event.try_into() {
	// 					if let $pattern = &eve {
	// 						return Some(Rc::new(eve))
	// 					} else {
	// 						return None
	// 					}
	// 				}
	// 				return None
	// 			})
	// 		})
	// 	};
	// }
}
