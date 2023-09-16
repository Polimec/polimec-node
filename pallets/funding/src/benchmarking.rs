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
use frame_benchmarking::v2::*;
use frame_support::{dispatch::RawOrigin, traits::OriginTrait, Parameter};
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
		total_allocation_size: BalanceOf::<T>::try_from(1_000_000_0_000_000_000u128)
			.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
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
			(50_000 * ASSET_UNIT).into(),
			18_u128.into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		BidParams::new(
			account::<AccountIdOf<T>>("bidder_2", 0, 0),
			(40_000 * ASSET_UNIT).into(),
			15_u128.into(),
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
	#[benchmark]
	fn create() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

		#[extrinsic_call]
		create(RawOrigin::Signed(issuer.clone()), project_metadata.clone());

		// * validity checks *
		// pallet-funding storage
		let projects_metadata = ProjectsMetadata::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_metadata = projects_metadata.iter().last().unwrap().1.clone();
		let project_id = projects_metadata.iter().last().unwrap().0;
		assert_eq!(stored_metadata, project_metadata);

		let project_details = ProjectsDetails::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_details = project_details.iter().last().unwrap().1.clone();
		assert_eq!(stored_details.issuer, issuer.clone());

		// Balances

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::ProjectCreated { project_id, issuer }.into());

		// Misc
	}

	#[benchmark]
	fn edit_metadata() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_new_project(project_metadata, issuer.clone());
		let edited_metadata: H256 = hashed(EDITED_METADATA);

		#[extrinsic_call]
		edit_metadata(RawOrigin::Signed(issuer.clone()), project_id, edited_metadata.into())

		// validity checks
	}

	#[benchmark]
	fn start_evaluation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_new_project(project_metadata, issuer.clone());

		#[extrinsic_call]
		start_evaluation(RawOrigin::Signed(issuer.clone()), project_id)

		// validity checks
	}

	#[benchmark]
	fn start_auction() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());

		let evaluations = default_evaluations();
		let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(evaluations.clone());
		let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");

		inst.advance_time(<T as Config>::EvaluationDuration::get() + One::one()).unwrap();

		#[extrinsic_call]
		start_auction(RawOrigin::Signed(issuer.clone()), project_id)

		// validity checks
	}

	#[benchmark]
	fn bond_evaluation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
		whitelist_account!(evaluator);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());

		let evaluation = UserToUSDBalance::new(evaluator.clone(), (50_000 * US_DOLLAR).into());

		let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(vec![evaluation.clone()]);
		let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();

		#[extrinsic_call]
		bond_evaluation(RawOrigin::Signed(evaluator.clone()), project_id, evaluation.usd_amount)

		// validity checks
	}

	#[benchmark]
	fn bid() {
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let project_id = inst.create_auctioning_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			default_evaluations::<T>(),
		);

		let bid_params = BidParams::new(
			bidder.clone(),
			(50000u128 * ASSET_UNIT).into(),
			18_u128.into(),
			1u8,
			AcceptedFundingAsset::USDT,
		);
		let necessary_plmc = BenchInstantiator::<T>::calculate_auction_plmc_spent(vec![bid_params.clone()]);
		let existential_deposits: Vec<UserToPLMCBalance<T>> = necessary_plmc.accounts().existential_deposits();
		let necessary_usdt = BenchInstantiator::<T>::calculate_auction_funding_asset_spent(vec![bid_params.clone()]);

		inst.mint_plmc_to(necessary_plmc);
		inst.mint_plmc_to(existential_deposits);
		inst.mint_statemint_asset_to(necessary_usdt);

		#[extrinsic_call]
		bid(
			RawOrigin::Signed(bidder.clone()),
			project_id,
			bid_params.amount,
			bid_params.price,
			bid_params.multiplier,
			bid_params.asset,
		);

		// validity checks
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

		let project_id = inst.create_community_contributing_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
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

		inst.mint_plmc_to(necessary_plmc);
		inst.mint_plmc_to(existential_deposits);
		inst.mint_statemint_asset_to(necessary_usdt);

		#[extrinsic_call]
		contribute(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contribution_params.amount,
			contribution_params.multiplier,
			contribution_params.asset,
		)

		// validity checks
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
		)

		// validity checks
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

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
		)

		// validity checks
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
		)

		// validity checks
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
		bid_ct_mint_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bid_to_mint_ct.id)

		// validity checks
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
		)

		// validity checks
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

		let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		start_bid_vesting_schedule_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), stored_bid.id)

		// validity checks
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

		let stored_contribution =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		start_contribution_vesting_schedule_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			stored_contribution.id,
		)

		// validity checks
	}

	#[benchmark]
	fn payout_bid_funds_for() {
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
			issuer.clone(),
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

		let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		payout_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder.clone(), stored_bid.id)

		// validity checks
	}

	#[benchmark]
	fn payout_contribution_funds_for() {
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
			issuer.clone(),
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

		let stored_contribution =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		payout_contribution_funds_for(
			RawOrigin::Signed(issuer.clone()),
			project_id,
			contributor.clone(),
			stored_contribution.id,
		)

		// validity checks
	}

	#[benchmark]
	fn decide_project_outcome() {
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(20) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(20) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();

		#[extrinsic_call]
		decide_project_outcome(RawOrigin::Signed(issuer.clone()), project_id, FundingOutcomeDecision::AcceptFunding)

		// validity checks
	}

	#[benchmark]
	fn release_bid_funds_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		release_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder, stored_bid.id)

		// validity checks
	}

	#[benchmark]
	fn bid_unbond_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_bid = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		inst.execute(|| {
			PalletFunding::<T>::release_bid_funds_for(
				<T as frame_system::Config>::RuntimeOrigin::signed(bidder.clone().into()),
				project_id,
				bidder.clone(),
				stored_bid.id,
			)
			.expect("Funds are released")
		});

		#[extrinsic_call]
		bid_unbond_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), stored_bid.id)

		// validity checks
	}

	#[benchmark]
	fn release_contribution_funds_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_contribution =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		release_contribution_funds_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			stored_contribution.id,
		)

		// validity checks
	}

	#[benchmark]
	fn contribution_unbond_for() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
		);
		let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			BenchInstantiator::calculate_price_from_test_bids(bids.clone()),
			default_weights(),
			default_contributors::<T>(),
		);
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let stored_contribution =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		inst.execute(|| {
			PalletFunding::<T>::release_contribution_funds_for(
				<T as frame_system::Config>::RuntimeOrigin::signed(contributor.clone().into()),
				project_id,
				contributor.clone(),
				stored_contribution.id,
			)
			.expect("Funds are released")
		});

		#[extrinsic_call]
		contribution_unbond_for(
			RawOrigin::Signed(issuer.clone()),
			project_id,
			contributor.clone(),
			stored_contribution.id,
		);

		// validity checks
	}

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
