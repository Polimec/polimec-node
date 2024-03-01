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
use crate::instantiator::*;
use frame_benchmarking::v2::*;
#[cfg(test)]
use frame_support::assert_ok;
use frame_support::{
	dispatch::RawOrigin,
	traits::{
		fungible::{InspectHold, MutateHold},
		fungibles::metadata::MetadataDeposit,
		OriginTrait,
	},
	Parameter,
};
#[allow(unused_imports)]
use pallet::Pallet as PalletFunding;
use parity_scale_codec::{Decode, Encode};
use polimec_common::{credentials::InvestorType, ReleaseSchedule};
use polimec_common_test_utils::get_mock_jwt;
use scale_info::prelude::format;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{BlakeTwo256, Get, Member, TrailingZeroInput};

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
type BenchInstantiator<T> = Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;

pub fn usdt_id() -> u32 {
	AcceptedFundingAsset::USDT.to_assethub_id()
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
			1u8,
			AcceptedFundingAsset::USDT,
		),
		BidParams::new(
			account::<AccountIdOf<T>>("bidder_2", 0, 0),
			(5_000 * ASSET_UNIT).into(),
			7u8,
			AcceptedFundingAsset::USDT,
		),
	]
}

pub fn full_bids<T>() -> Vec<BidParams<T>>
where
	T: Config,
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
{
	let default_project = default_project::<T>(0, account::<AccountIdOf<T>>("issuer", 0, 0));
	let total_ct_for_bids = default_project.total_allocation_size.0;
	let total_usd_for_bids = default_project.minimum_price.checked_mul_int(total_ct_for_bids).unwrap();
	BenchInstantiator::<T>::generate_bids_from_total_usd(
		total_usd_for_bids,
		default_project.minimum_price,
		default_weights(),
		default_bidders::<T>(),
		default_bidder_multipliers(),
	)
}

pub fn default_community_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
{
	vec![
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_1", 0, 0),
			(10_000 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_2", 0, 0),
			(6_000 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_3", 0, 0),
			(30_000 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
	]
}

pub fn default_remainder_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
{
	vec![
		ContributionParams::new(
			account::<AccountIdOf<T>>("contributor_1", 0, 0),
			(10 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("bidder_1", 0, 0),
			(60 * ASSET_UNIT).into(),
			1u8,
			AcceptedFundingAsset::USDT,
		),
		ContributionParams::new(
			account::<AccountIdOf<T>>("evaluator_1", 0, 0),
			(30 * ASSET_UNIT).into(),
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

pub fn default_bidder_multipliers() -> Vec<u8> {
	vec![20u8, 3u8, 15u8, 13u8, 9u8]
}
pub fn default_community_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 5u8, 3u8, 1u8, 2u8]
}
pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 10u8, 3u8, 2u8, 4u8]
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

#[cfg(feature = "std")]
pub fn populate_with_projects<T>(amount: u32, inst: BenchInstantiator<T>) -> BenchInstantiator<T>
where
	T: Config
		+ frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent>
		+ pallet_balances::Config<Balance = BalanceOf<T>>,
	<T as Config>::RuntimeEvent: TryInto<Event<T>> + Parameter + Member,
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
	<T as frame_system::Config>::AccountId:
		Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
	<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
{
	let states = vec![
		ProjectStatus::Application,
		ProjectStatus::EvaluationRound,
		ProjectStatus::AuctionRound(AuctionPhase::English),
		ProjectStatus::CommunityRound,
		ProjectStatus::RemainderRound,
		ProjectStatus::FundingSuccessful,
	]
	.into_iter()
	.cycle()
	.take(amount as usize);

	let instantiation_details = states
		.map(|state| {
			let nonce = inst.get_new_nonce();
			let issuer_name: String = format!("issuer_{}", nonce);

			let issuer = string_account::<AccountIdOf<T>>(issuer_name, 0, 0);
			TestProjectParams::<T> {
				expected_state: state,
				metadata: default_project::<T>(inst.get_new_nonce(), issuer.clone()),
				issuer: issuer.clone(),
				evaluations: default_evaluations::<T>(),
				bids: default_bids::<T>(),
				community_contributions: default_community_contributions::<T>(),
				remainder_contributions: default_remainder_contributions::<T>(),
			}
		})
		.collect::<Vec<TestProjectParams<T>>>();

	async_features::create_multiple_projects_at(inst, instantiation_details).1
}

// IMPORTANT: make sure your project starts at (block 1 + `total_vecs_in_storage` - `fully_filled_vecs_from_insertion`) to always have room to insert new vecs
pub fn fill_projects_to_update<T: Config>(
	fully_filled_vecs_from_insertion: u32,
	mut expected_insertion_block: BlockNumberFor<T>,
	maybe_total_vecs_in_storage: Option<u32>,
) {
	// fill the `ProjectsToUpdate` vectors from @ expected_insertion_block to @ expected_insertion_block+x, to benchmark all the failed insertion attempts
	for _ in 0..fully_filled_vecs_from_insertion {
		while ProjectsToUpdate::<T>::try_append(expected_insertion_block, (&69u32, UpdateType::EvaluationEnd)).is_ok() {
			continue;
		}
		expected_insertion_block += 1u32.into();
	}

	// sometimes we don't expect to remove anything from storage
	if let Some(total_vecs_in_storage) = maybe_total_vecs_in_storage {
		// fill `ProjectsToUpdate` with `y` different BlockNumber->Vec items to benchmark deletion of our project from the map
		// We keep in mind that we already filled `x` amount of vecs to max capacity
		let remaining_vecs = total_vecs_in_storage.saturating_sub(fully_filled_vecs_from_insertion);
		if remaining_vecs > 0 {
			let items_per_vec = T::MaxProjectsToUpdatePerBlock::get();
			let mut block_number: BlockNumberFor<T> = Zero::zero();
			for _ in 0..remaining_vecs {
				// To iterate over all expected items when looking to remove, we need to insert everything _before_ our already stored project's block_number
				let mut vec: Vec<(ProjectId, UpdateType)> = ProjectsToUpdate::<T>::get(block_number).to_vec();
				let items_to_fill = items_per_vec - vec.len() as u32;
				for _ in 0..items_to_fill {
					vec.push((69u32, UpdateType::EvaluationEnd));
				}
				let bounded_vec: BoundedVec<(ProjectId, UpdateType), T::MaxProjectsToUpdatePerBlock> =
					vec.try_into().unwrap();
				ProjectsToUpdate::<T>::insert(block_number, bounded_vec);
				block_number += 1u32.into();
			}
		}
	}
}

// returns how much PLMC was minted and held to the user
pub fn make_ct_deposit_for<T: Config>(user: AccountIdOf<T>, project_id: ProjectId) {
	let ct_deposit = T::ContributionTokenCurrency::deposit_required(project_id);
	// Reserve plmc deposit to create a contribution token account for this project
	if T::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), &user) < ct_deposit {
		T::NativeCurrency::hold(&HoldReason::FutureDeposit(project_id).into(), &user, ct_deposit).unwrap();
	}
}

pub fn run_blocks_to_execute_next_transition<T: Config>(
	project_id: ProjectId,
	maybe_update_type: Option<UpdateType>,
	inst: &mut BenchInstantiator<T>,
) {
	let (update_block, stored_update_type) = inst.get_update_pair(project_id);
	if let Some(expected_update_type) = maybe_update_type {
		assert_eq!(stored_update_type, expected_update_type);
	}
	frame_system::Pallet::<T>::set_block_number(update_block - 1u32.into());
	inst.advance_time(One::one()).unwrap();
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

	//
	// Extrinsics
	//
	#[benchmark]
	fn create() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let ed = BenchInstantiator::<T>::get_ed();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);
		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

		let metadata_deposit = T::ContributionTokenCurrency::calc_metadata_deposit(
			project_metadata.token_information.name.as_slice(),
			project_metadata.token_information.symbol.as_slice(),
		);
		inst.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), ed * 2u64.into() + metadata_deposit)]);
		let jwt = get_mock_jwt(issuer.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		create(RawOrigin::Signed(issuer.clone()), jwt, project_metadata.clone());

		// * validity checks *
		// Storage
		let projects_metadata = ProjectsMetadata::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_metadata = &projects_metadata.iter().last().unwrap().1;
		let project_id = projects_metadata.iter().last().unwrap().0;
		assert_eq!(stored_metadata, &project_metadata);

		let project_details = ProjectsDetails::<T>::iter().sorted_by(|a, b| a.0.cmp(&b.0)).collect::<Vec<_>>();
		let stored_details = &project_details.iter().last().unwrap().1;
		assert_eq!(&stored_details.issuer, &issuer);

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::ProjectCreated { project_id, issuer }.into());
	}

	#[benchmark]
	fn edit_metadata() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone());
		let original_metadata_hash = project_metadata.offchain_information_hash.unwrap();
		let edited_metadata_hash: H256 = hashed(EDITED_METADATA);
		let jwt = get_mock_jwt(issuer.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		edit_metadata(RawOrigin::Signed(issuer), jwt, project_id, edited_metadata_hash.into());

		// * validity checks *
		// Storage
		let stored_metadata = ProjectsMetadata::<T>::get(project_id).unwrap();
		assert_eq!(stored_metadata.offchain_information_hash, Some(edited_metadata_hash.into()));
		assert!(original_metadata_hash != edited_metadata_hash.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::MetadataEdited { project_id }.into());
	}

	#[benchmark]
	fn start_evaluation(
		// insertion attempts in add_to_update_store.
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_new_project(project_metadata, issuer.clone());

		// start_evaluation fn will try to add an automatic transition 1 block after the last evaluation block
		let mut block_number: BlockNumberFor<T> = inst.current_block() + T::EvaluationDuration::get() + One::one();
		// fill the `ProjectsToUpdate` vectors from @ block_number to @ block_number+x, to benchmark all the failed insertion attempts
		for _ in 0..x {
			while ProjectsToUpdate::<T>::try_append(block_number, (&69u32, UpdateType::EvaluationEnd)).is_ok() {
				continue;
			}
			block_number += 1u32.into();
		}
		let jwt = get_mock_jwt(issuer.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		start_evaluation(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::EvaluationRound);
		let starting_evaluation_info = EvaluationRoundInfoOf::<T> {
			total_bonded_usd: Zero::zero(),
			total_bonded_plmc: Zero::zero(),
			evaluators_outcome: EvaluatorsOutcome::Unchanged,
		};
		assert_eq!(stored_details.evaluation_round_info, starting_evaluation_info);
		let evaluation_transition_points = stored_details.phase_transition_points.evaluation;
		match evaluation_transition_points {
			BlockNumberPair { start: Some(_), end: Some(_) } => {},
			_ => assert!(false, "Evaluation transition points are not set"),
		}

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::EvaluationStarted { project_id }.into())
	}

	#[benchmark]
	fn start_auction_manually(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Total amount of storage items iterated through in `ProjectsToUpdate` when trying to remove our project in `remove_from_update_store`.
		// Upper bound is assumed to be enough
		y: Linear<1, 10_000>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion
		let u32_remaining_vecs: u32 = y.saturating_sub(x).into();
		let time_advance: u32 = 1 + u32_remaining_vecs + 1;
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());

		let evaluations = default_evaluations();
		let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(evaluations.clone());
		let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().ct_account_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(ct_account_deposits);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");

		run_blocks_to_execute_next_transition(project_id, Some(UpdateType::EvaluationEnd), &mut inst);
		inst.advance_time(1u32.into()).unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		let current_block = inst.current_block();
		// `do_english_auction` fn will try to add an automatic transition 1 block after the last english round block
		let insertion_block_number: BlockNumberFor<T> = current_block + T::EnglishAuctionDuration::get() + One::one();

		fill_projects_to_update::<T>(x, insertion_block_number, Some(y));

		let jwt = get_mock_jwt(issuer.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		start_auction(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::AuctionRound(AuctionPhase::English));

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::EnglishAuctionStarted { project_id, when: current_block.into() }.into(),
		);
	}

	// possible branches:
	// - pays ct account deposit
	//      - we know this happens only if param x = 0, but we cannot fit it in the linear regression
	// - is over max evals per user, and needs to unbond the lowest evaluation
	// 		- this case, we know they paid already for ct account deposit

	fn evaluation_setup<T>(x: u32) -> (BenchInstantiator<T>, ProjectId, UserToUSDBalance<T>, BalanceOf<T>, BalanceOf<T>)
	where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let test_evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
		whitelist_account!(test_evaluator);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let test_project_id = inst.create_evaluating_project(project_metadata, issuer);

		let existing_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (100 * US_DOLLAR).into());
		let extrinsic_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (1_000 * US_DOLLAR).into());
		let existing_evaluations = vec![existing_evaluation; x as usize];

		let plmc_for_existing_evaluations =
			BenchInstantiator::<T>::calculate_evaluation_plmc_spent(existing_evaluations.clone());
		let plmc_for_extrinsic_evaluation =
			BenchInstantiator::<T>::calculate_evaluation_plmc_spent(vec![extrinsic_evaluation.clone()]);
		let existential_plmc: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_evaluation.accounts().existential_deposits();
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_evaluation.accounts().ct_account_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(ct_account_deposits);
		inst.mint_plmc_to(plmc_for_existing_evaluations.clone());
		inst.mint_plmc_to(plmc_for_extrinsic_evaluation.clone());

		inst.advance_time(One::one()).unwrap();

		// do "x" evaluations for this user
		inst.bond_for_users(test_project_id, existing_evaluations).expect("All evaluations are accepted");

		let extrinsic_plmc_bonded = plmc_for_extrinsic_evaluation[0].plmc_amount;
		let mut total_expected_plmc_bonded = BenchInstantiator::<T>::sum_balance_mappings(vec![
			plmc_for_existing_evaluations.clone(),
			plmc_for_extrinsic_evaluation.clone(),
		]);

		// if we are going to unbond evaluations due to being over the limit per user, then deduct them from the total expected plmc bond
		if x >= <T as Config>::MaxEvaluationsPerUser::get() {
			total_expected_plmc_bonded -= plmc_for_existing_evaluations[0].plmc_amount *
				(x as u128 - <T as Config>::MaxEvaluationsPerUser::get() as u128 + 1u128).into();
		}

		(inst, test_project_id, extrinsic_evaluation, extrinsic_plmc_bonded, total_expected_plmc_bonded)
	}
	fn evaluation_verification<T>(
		mut inst: BenchInstantiator<T>,
		project_id: ProjectId,
		evaluation: UserToUSDBalance<T>,
		extrinsic_plmc_bonded: BalanceOf<T>,
		total_expected_plmc_bonded: BalanceOf<T>,
	) where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// * validity checks *
		// Storage
		let stored_evaluation = Evaluations::<T>::iter_prefix_values((project_id, evaluation.account.clone()))
			.sorted_by(|a, b| a.id.cmp(&b.id))
			.last()
			.unwrap();

		match stored_evaluation {
			EvaluationInfo {
				project_id,
				evaluator,
				original_plmc_bond,
				current_plmc_bond,
				rewarded_or_slashed,
				..
			} if project_id == project_id &&
				evaluator == evaluation.account.clone() &&
				original_plmc_bond == extrinsic_plmc_bonded &&
				current_plmc_bond == extrinsic_plmc_bonded &&
				rewarded_or_slashed.is_none() => {},
			_ => assert!(false, "Evaluation is not stored correctly"),
		}

		// Balances
		let bonded_plmc = inst.get_reserved_plmc_balances_for(
			vec![evaluation.account.clone()],
			HoldReason::Evaluation(project_id).into(),
		)[0]
		.plmc_amount;
		assert_eq!(bonded_plmc, total_expected_plmc_bonded);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::FundsBonded { project_id, amount: extrinsic_plmc_bonded, bonder: evaluation.account.clone() }
				.into(),
		);
	}

	// - We know how many iterations it does in storage
	// - We know that it requires a ct deposit
	// - We know that it does not require to unbond the lowest evaluation
	#[benchmark]
	pub fn first_evaluation() {
		// How many other evaluations the user did for that same project
		let x = 0;
		let (inst, project_id, extrinsic_evaluation, extrinsic_plmc_bonded, total_expected_plmc_bonded) =
			evaluation_setup::<T>(x);

		let jwt = get_mock_jwt(extrinsic_evaluation.account.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		evaluate(
			RawOrigin::Signed(extrinsic_evaluation.account.clone()),
			jwt,
			project_id,
			extrinsic_evaluation.usd_amount,
		);

		evaluation_verification::<T>(
			inst,
			project_id,
			extrinsic_evaluation,
			extrinsic_plmc_bonded,
			total_expected_plmc_bonded,
		);
	}

	// - We know that it does not require a ct deposit
	// - We know that it does not require to unbond the lowest evaluation.
	// - We don't know how many iterations it does in storage (i.e "x")
	#[benchmark]
	fn second_to_limit_evaluation(
		// How many other evaluations the user did for that same project
		x: Linear<1, { T::MaxEvaluationsPerUser::get() - 1 }>,
	) {
		let (inst, project_id, extrinsic_evaluation, extrinsic_plmc_bonded, total_expected_plmc_bonded) =
			evaluation_setup::<T>(x);

		let jwt = get_mock_jwt(extrinsic_evaluation.account.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		evaluate(
			RawOrigin::Signed(extrinsic_evaluation.account.clone()),
			jwt,
			project_id,
			extrinsic_evaluation.usd_amount,
		);

		evaluation_verification::<T>(
			inst,
			project_id,
			extrinsic_evaluation,
			extrinsic_plmc_bonded,
			total_expected_plmc_bonded,
		);
	}

	// - We know how many iterations it does in storage
	// - We know that it does not require a ct deposit
	// - We know that it requires to unbond the lowest evaluation
	#[benchmark]
	fn evaluation_over_limit() {
		// How many other evaluations the user did for that same project
		let x = <T as Config>::MaxEvaluationsPerUser::get();
		let (inst, project_id, extrinsic_evaluation, extrinsic_plmc_bonded, total_expected_plmc_bonded) =
			evaluation_setup::<T>(x);

		let jwt = get_mock_jwt(extrinsic_evaluation.account.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		evaluate(
			RawOrigin::Signed(extrinsic_evaluation.account.clone()),
			jwt,
			project_id,
			extrinsic_evaluation.usd_amount,
		);

		evaluation_verification::<T>(
			inst,
			project_id,
			extrinsic_evaluation,
			extrinsic_plmc_bonded,
			total_expected_plmc_bonded,
		);
	}

	fn bid_setup<T>(
		existing_bids_count: u32,
		do_perform_bid_calls: u32,
	) -> (
		BenchInstantiator<T>,
		ProjectId,
		ProjectMetadataOf<T>,
		BidParams<T>,
		Option<BidParams<T>>,
		Vec<(BidParams<T>, PriceOf<T>)>,
		Vec<(BidParams<T>, PriceOf<T>)>,
		BalanceOf<T>,
		BalanceOf<T>,
		BalanceOf<T>,
		BalanceOf<T>,
	)
	where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, default_evaluations::<T>());

		let existing_bid =
			BidParams::new(bidder.clone(), (100u128 * ASSET_UNIT).into(), 5u8, AcceptedFundingAsset::USDT);

		let existing_bids = vec![existing_bid; existing_bids_count as usize];
		let existing_bids_post_bucketing = BenchInstantiator::<T>::get_actual_price_charged_for_bucketed_bids(
			&existing_bids,
			project_metadata.clone(),
			None,
		);
		let plmc_for_existing_bids =
			BenchInstantiator::<T>::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&existing_bids,
				project_metadata.clone(),
				None,
			);

		let existential_deposits: Vec<UserToPLMCBalance<T>> = vec![bidder.clone()].existential_deposits();
		let ct_account_deposits = vec![bidder.clone()].ct_account_deposits();

		let usdt_for_existing_bids: Vec<UserToForeignAssets<T>> =
			BenchInstantiator::<T>::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&existing_bids,
				project_metadata.clone(),
				None,
			);
		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_escrow_usdt_locked =
			inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()]);

		inst.mint_plmc_to(plmc_for_existing_bids.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_plmc_to(ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_existing_bids.clone());

		// do "x" contributions for this user
		inst.bid_for_users(project_id, existing_bids.clone()).unwrap();

		// to call do_perform_bid several times, we need the bucket to reach its limit. You can only bid over 10 buckets
		// in a single bid, since the increase delta is 10% of the total allocation, and you cannot bid more than the allocation.
		let mut ct_amount = (1000u128 * ASSET_UNIT).into();
		let mut maybe_filler_bid = None;
		let new_bidder = account::<AccountIdOf<T>>("new_bidder", 0, 0);

		let mut usdt_for_filler_bidder = vec![UserToForeignAssets::<T>::new(
			new_bidder.clone(),
			Zero::zero(),
			AcceptedFundingAsset::USDT.to_assethub_id(),
		)];
		if do_perform_bid_calls > 0 {
			let current_bucket = Buckets::<T>::get(project_id).unwrap();
			// first lets bring the bucket to almost its limit with another bidder:
			assert!(new_bidder.clone() != bidder.clone());
			let bid_params = BidParams::new(new_bidder, current_bucket.amount_left, 1u8, AcceptedFundingAsset::USDT);
			maybe_filler_bid = Some(bid_params.clone());
			let plmc_for_new_bidder = BenchInstantiator::<T>::calculate_auction_plmc_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
			);
			let plmc_ed = plmc_for_new_bidder.accounts().existential_deposits();
			let plmc_ct_deposit = plmc_for_new_bidder.accounts().ct_account_deposits();
			let usdt_for_new_bidder = BenchInstantiator::<T>::calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
			);

			inst.mint_plmc_to(plmc_for_new_bidder);
			inst.mint_plmc_to(plmc_ed);
			inst.mint_plmc_to(plmc_ct_deposit);
			inst.mint_foreign_asset_to(usdt_for_new_bidder.clone());

			inst.bid_for_users(project_id, vec![bid_params]).unwrap();

			ct_amount = Percent::from_percent(10) *
				project_metadata.total_allocation_size.0 *
				(do_perform_bid_calls as u128).into();
			usdt_for_filler_bidder = usdt_for_new_bidder;
		}
		let extrinsic_bid = BidParams::new(bidder.clone(), ct_amount, 1u8, AcceptedFundingAsset::USDT);
		let original_extrinsic_bid = extrinsic_bid.clone();
		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		// we need to call this after bidding `x` amount of times, to get the latest bucket from storage
		let extrinsic_bids_post_bucketing = BenchInstantiator::<T>::get_actual_price_charged_for_bucketed_bids(
			&vec![extrinsic_bid.clone()],
			project_metadata.clone(),
			Some(current_bucket),
		);
		assert_eq!(extrinsic_bids_post_bucketing.len(), (do_perform_bid_calls as usize).max(1usize));

		let plmc_for_extrinsic_bids: Vec<UserToPLMCBalance<T>> =
			BenchInstantiator::<T>::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);
		let usdt_for_extrinsic_bids: Vec<UserToForeignAssets<T>> =
			BenchInstantiator::<T>::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid],
				project_metadata.clone(),
				Some(current_bucket),
			);
		inst.mint_plmc_to(plmc_for_extrinsic_bids.clone());
		inst.mint_foreign_asset_to(usdt_for_extrinsic_bids.clone());

		let total_free_plmc = existential_deposits[0].plmc_amount;
		let total_plmc_participation_bonded = BenchInstantiator::<T>::sum_balance_mappings(vec![
			plmc_for_extrinsic_bids.clone(),
			plmc_for_existing_bids.clone(),
		]);
		let total_free_usdt = Zero::zero();
		let total_escrow_usdt_locked = BenchInstantiator::<T>::sum_foreign_mappings(vec![
			prev_total_escrow_usdt_locked.clone(),
			usdt_for_extrinsic_bids.clone(),
			usdt_for_existing_bids.clone(),
			usdt_for_filler_bidder.clone(),
		]);

		(
			inst,
			project_id,
			project_metadata,
			original_extrinsic_bid,
			maybe_filler_bid,
			extrinsic_bids_post_bucketing,
			existing_bids_post_bucketing,
			total_free_plmc,
			total_plmc_participation_bonded,
			total_free_usdt,
			total_escrow_usdt_locked,
		)
	}

	fn bid_verification<T>(
		mut inst: BenchInstantiator<T>,
		project_id: ProjectId,
		project_metadata: ProjectMetadataOf<T>,
		maybe_filler_bid: Option<BidParams<T>>,
		extrinsic_bids_post_bucketing: Vec<(BidParams<T>, PriceOf<T>)>,
		existing_bids_post_bucketing: Vec<(BidParams<T>, PriceOf<T>)>,
		total_free_plmc: BalanceOf<T>,
		total_plmc_bonded: BalanceOf<T>,
		total_free_usdt: BalanceOf<T>,
		total_usdt_locked: BalanceOf<T>,
	) -> ()
	where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// * validity checks *

		let bidder = extrinsic_bids_post_bucketing[0].0.bidder.clone();
		// Storage
		for (bid_params, price) in extrinsic_bids_post_bucketing.clone() {
			let bid_filter = BidInfoFilter::<T> {
				id: None,
				project_id: Some(project_id),
				bidder: Some(bidder.clone()),
				status: Some(BidStatus::YetUnknown),
				original_ct_amount: Some(bid_params.amount),
				original_ct_usd_price: Some(price),
				final_ct_amount: Some(bid_params.amount),
				final_ct_usd_price: None,
				funding_asset: Some(AcceptedFundingAsset::USDT),
				funding_asset_amount_locked: None,
				multiplier: Some(bid_params.multiplier),
				plmc_bond: None,
				plmc_vesting_info: Some(None),
				when: None,
				funds_released: Some(false),
				ct_minted: Some(false),
			};
			Bids::<T>::iter_prefix_values((project_id, bidder.clone()))
				.find(|stored_bid| bid_filter.matches_bid(stored_bid))
				.expect("bid not found");
		}

		// Bucket Storage Check
		let bucket_delta_amount = Percent::from_percent(10) * project_metadata.total_allocation_size.0;
		let ten_percent_in_price: <T as Config>::Price = PriceOf::<T>::checked_from_rational(1, 10).unwrap();

		let mut starting_bucket = Bucket::new(
			project_metadata.total_allocation_size.0,
			project_metadata.minimum_price,
			ten_percent_in_price,
			bucket_delta_amount,
		);

		for (bid_params, _price_) in existing_bids_post_bucketing.clone() {
			starting_bucket.update(bid_params.amount);
		}
		if let Some(bid_params) = maybe_filler_bid {
			starting_bucket.update(bid_params.amount);
		}
		for (bid_params, _price_) in extrinsic_bids_post_bucketing.clone() {
			starting_bucket.update(bid_params.amount);
		}

		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		assert_eq!(current_bucket, starting_bucket);

		// Balances
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![bidder.clone()], HoldReason::Participation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, total_plmc_bonded);

		let free_plmc = inst.get_free_plmc_balances_for(vec![bidder.clone()])[0].plmc_amount;
		assert_eq!(free_plmc, total_free_plmc);

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let locked_usdt =
			inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()])[0].asset_amount;
		assert_eq!(locked_usdt, total_usdt_locked);

		let free_usdt = inst.get_free_foreign_asset_balances_for(usdt_id(), vec![bidder])[0].asset_amount;
		assert_eq!(free_usdt, total_free_usdt);

		// Events
		for (bid_params, _price_) in extrinsic_bids_post_bucketing {
			find_event! {
				T,
				Event::<T>::Bid {
					project_id,
					amount,
					multiplier, ..
				},
				project_id == project_id,
				amount == bid_params.amount,
				multiplier == bid_params.multiplier
			}
			.expect("Event has to be emitted");
		}
	}

	#[benchmark]
	fn bid_no_ct_deposit(
		// amount of already made bids by the same user
		x: Linear<0, { T::MaxBidsPerUser::get() - 1 }>,
		// amount of times where `perform_bid` is called (i.e how many buckets)
		y: Linear<0, 10>,
	) {
		let (
			inst,
			project_id,
			project_metadata,
			original_extrinsic_bid,
			maybe_filler_bid,
			extrinsic_bids_post_bucketing,
			existing_bids_post_bucketing,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
		) = bid_setup::<T>(x, y);

		let _new_plmc_minted = make_ct_deposit_for::<T>(original_extrinsic_bid.bidder.clone(), project_id);

		let jwt = get_mock_jwt(original_extrinsic_bid.bidder.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		bid(
			RawOrigin::Signed(original_extrinsic_bid.bidder.clone()),
			jwt,
			project_id,
			original_extrinsic_bid.amount,
			original_extrinsic_bid.multiplier,
			original_extrinsic_bid.asset,
		);

		bid_verification::<T>(
			inst,
			project_id,
			project_metadata,
			maybe_filler_bid,
			extrinsic_bids_post_bucketing,
			existing_bids_post_bucketing,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
		);
	}

	#[benchmark]
	fn bid_with_ct_deposit(
		// amount of times where `perform_bid` is called (i.e how many buckets)
		y: Linear<0, 10>,
	) {
		// if x were > 0, then the ct deposit would already be paid
		let x = 0;
		let (
			inst,
			project_id,
			project_metadata,
			original_extrinsic_bid,
			maybe_filler_bid,
			extrinsic_bids_post_bucketing,
			existing_bids_post_bucketing,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
		) = bid_setup::<T>(x, y);

		let jwt = get_mock_jwt(original_extrinsic_bid.bidder.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		bid(
			RawOrigin::Signed(original_extrinsic_bid.bidder.clone()),
			jwt,
			project_id,
			original_extrinsic_bid.amount,
			original_extrinsic_bid.multiplier,
			original_extrinsic_bid.asset,
		);

		bid_verification::<T>(
			inst,
			project_id,
			project_metadata,
			maybe_filler_bid,
			extrinsic_bids_post_bucketing,
			existing_bids_post_bucketing,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
		);
	}

	fn contribution_setup<T>(
		x: u32,
		ends_round: Option<(u32, u32)>,
	) -> (
		BenchInstantiator<T>,
		ProjectId,
		ProjectMetadataOf<T>,
		ContributionParams<T>,
		BalanceOf<T>,
		BalanceOf<T>,
		BalanceOf<T>,
		BalanceOf<T>,
		BalanceOf<T>,
	)
	where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion
		let mut time_advance: u32 = 1;
		if let Some((y, z)) = ends_round {
			let u32_remaining_vecs: u32 = z.saturating_sub(y).into();
			time_advance += u32_remaining_vecs + 1;
		}
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributor = account::<AccountIdOf<T>>("contributor", 0, 0);
		whitelist_account!(contributor);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			default_evaluations::<T>(),
			full_bids::<T>(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let existing_amount: BalanceOf<T> = (50 * ASSET_UNIT).into();
		let extrinsic_amount: BalanceOf<T> = if ends_round.is_some() {
			project_metadata.total_allocation_size.0 -
				existing_amount * (x.min(<T as Config>::MaxContributionsPerUser::get() - 1) as u128).into()
		} else {
			(100 * ASSET_UNIT).into()
		};
		let existing_contribution =
			ContributionParams::new(contributor.clone(), existing_amount, 1u8, AcceptedFundingAsset::USDT);
		let extrinsic_contribution =
			ContributionParams::new(contributor.clone(), extrinsic_amount, 1u8, AcceptedFundingAsset::USDT);
		let existing_contributions = vec![existing_contribution; x as usize];

		let mut total_ct_sold: BalanceOf<T> = existing_amount * (x as u128).into() + extrinsic_amount;

		let plmc_for_existing_contributions =
			BenchInstantiator::<T>::calculate_contributed_plmc_spent(existing_contributions.clone(), price);
		let plmc_for_extrinsic_contribution =
			BenchInstantiator::<T>::calculate_contributed_plmc_spent(vec![extrinsic_contribution.clone()], price);
		let usdt_for_existing_contributions =
			BenchInstantiator::<T>::calculate_contributed_funding_asset_spent(existing_contributions.clone(), price);
		let usdt_for_extrinsic_contribution = BenchInstantiator::<T>::calculate_contributed_funding_asset_spent(
			vec![extrinsic_contribution.clone()],
			price,
		);

		let existential_deposits: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_contribution.accounts().existential_deposits();
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_contribution.accounts().ct_account_deposits();

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_usdt_locked = inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()]);

		inst.mint_plmc_to(plmc_for_existing_contributions.clone());
		inst.mint_plmc_to(plmc_for_extrinsic_contribution.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_plmc_to(ct_account_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_existing_contributions.clone());
		inst.mint_foreign_asset_to(usdt_for_extrinsic_contribution.clone());

		// do "x" contributions for this user
		inst.contribute_for_users(project_id, existing_contributions).expect("All contributions are accepted");

		let mut total_plmc_bonded = BenchInstantiator::<T>::sum_balance_mappings(vec![
			plmc_for_existing_contributions.clone(),
			plmc_for_extrinsic_contribution.clone(),
		]);
		let mut total_usdt_locked = BenchInstantiator::<T>::sum_foreign_mappings(vec![
			prev_total_usdt_locked,
			usdt_for_existing_contributions.clone(),
			usdt_for_extrinsic_contribution.clone(),
		]);

		let over_limit_count = x.saturating_sub(<T as Config>::MaxContributionsPerUser::get() - 1);

		let mut total_free_plmc = existential_deposits[0].plmc_amount;
		let mut total_free_usdt = Zero::zero();

		if x > 0 {
			let plmc_returned = plmc_for_existing_contributions[0].plmc_amount * (over_limit_count as u128).into();
			total_plmc_bonded -= plmc_returned;

			let usdt_returned = usdt_for_existing_contributions[0].asset_amount * (over_limit_count as u128).into();
			total_usdt_locked -= usdt_returned;
			total_ct_sold -= existing_amount * (over_limit_count as u128).into();
			total_free_plmc += plmc_returned;
			total_free_usdt += usdt_returned;
		}

		if let Some((fully_filled_vecs_from_insertion, total_vecs_in_storage)) = ends_round {
			// if all CTs are sold, next round is scheduled for next block (either remainder or success)
			let expected_insertion_block = inst.current_block() + One::one();
			fill_projects_to_update::<T>(
				fully_filled_vecs_from_insertion,
				expected_insertion_block,
				Some(total_vecs_in_storage),
			);
		}

		(
			inst,
			project_id,
			project_metadata,
			extrinsic_contribution,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
			total_ct_sold,
		)
	}

	fn contribution_verification<T>(
		mut inst: BenchInstantiator<T>,
		project_id: ProjectId,
		project_metadata: ProjectMetadataOf<T>,
		extrinsic_contribution: ContributionParams<T>,
		total_free_plmc: BalanceOf<T>,
		total_plmc_bonded: BalanceOf<T>,
		total_free_usdt: BalanceOf<T>,
		total_usdt_locked: BalanceOf<T>,
		total_ct_sold: BalanceOf<T>,
	) where
		T: Config,
		<T as Config>::Balance: From<u128>,
		<T as Config>::Price: From<u128>,
		T::Hash: From<H256>,
		<T as frame_system::Config>::RuntimeEvent: From<pallet::Event<T>>,
	{
		// * validity checks *
		// Storage
		let contributor = extrinsic_contribution.contributor.clone();
		let stored_contribution = Contributions::<T>::iter_prefix_values((project_id, contributor.clone()))
			.sorted_by(|a, b| a.id.cmp(&b.id))
			.last()
			.unwrap();

		match stored_contribution {
			ContributionInfoOf::<T> { project_id, contributor, ct_amount, .. }
				if project_id == project_id &&
					contributor == contributor &&
					ct_amount == extrinsic_contribution.amount => {},
			_ => {
				assert!(false, "Contribution is not stored correctly")
			},
		}

		let stored_project_details = ProjectsDetails::<T>::get(project_id).unwrap();

		assert_eq!(
			stored_project_details.remaining_contribution_tokens.1,
			project_metadata.total_allocation_size.1.saturating_sub(total_ct_sold)
		);

		// Balances
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![contributor.clone()], HoldReason::Participation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, total_plmc_bonded);

		let free_plmc = inst.get_free_plmc_balances_for(vec![contributor.clone()])[0].plmc_amount;
		assert_eq!(free_plmc, total_free_plmc);

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let locked_usdt =
			inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()])[0].asset_amount;
		assert_eq!(locked_usdt, total_usdt_locked);

		let free_usdt = inst.get_free_foreign_asset_balances_for(usdt_id(), vec![contributor.clone()])[0].asset_amount;
		assert_eq!(free_usdt, total_free_usdt);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::Contribution {
				project_id,
				contributor,
				amount: extrinsic_contribution.amount,
				multiplier: extrinsic_contribution.multiplier,
			}
			.into(),
		);
	}

	#[benchmark]
	fn contribution(
		// How many other contributions the user did for that same project
		x: Linear<0, { T::MaxContributionsPerUser::get() - 1 }>,
	) {
		let ends_round = None;

		let (
			inst,
			project_id,
			project_metadata,
			extrinsic_contribution,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
			total_ct_sold,
		) = contribution_setup::<T>(x, ends_round);

		#[extrinsic_call]
		community_contribute(
			RawOrigin::Signed(extrinsic_contribution.contributor.clone()),
			project_id,
			extrinsic_contribution.amount,
			extrinsic_contribution.multiplier,
			extrinsic_contribution.asset,
		);

		contribution_verification::<T>(
			inst,
			project_id,
			project_metadata,
			extrinsic_contribution,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
			total_ct_sold,
		);
	}

	#[benchmark]
	fn contribution_ends_round(
		// How many other contributions the user did for that same project
		x: Linear<0, { T::MaxContributionsPerUser::get() - 1 }>,
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		y: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Total amount of storage items iterated through in `ProjectsToUpdate` when trying to remove our project in `remove_from_update_store`.
		// Upper bound is assumed to be enough
		z: Linear<1, 10_000>,
	) {
		let ends_round = Some((y, z));

		let (
			inst,
			project_id,
			project_metadata,
			extrinsic_contribution,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
			total_ct_sold,
		) = contribution_setup::<T>(x, ends_round);

		#[extrinsic_call]
		community_contribute(
			RawOrigin::Signed(extrinsic_contribution.contributor.clone()),
			project_id,
			extrinsic_contribution.amount,
			extrinsic_contribution.multiplier,
			extrinsic_contribution.asset,
		);

		contribution_verification::<T>(
			inst,
			project_id,
			project_metadata,
			extrinsic_contribution,
			total_free_plmc,
			total_plmc_bonded,
			total_free_usdt,
			total_usdt_locked,
			total_ct_sold,
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingSuccessful);
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
		// Balance
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![evaluator.clone()], HoldReason::Evaluation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BondReleased {
				project_id,
				amount: evaluation_to_unbond.current_plmc_bond,
				bonder: evaluator.clone(),
				releaser: evaluator,
			}
			.into(),
		);
	}

	#[benchmark]
	fn evaluation_reward_payout_for_with_ct_account_creation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations: Vec<UserToUSDBalance<T>> = vec![
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_bench", 0, 0), (50_000 * US_DOLLAR).into()),
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_bench", 0, 0), (25_000 * US_DOLLAR).into()),
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_3", 0, 0), (32_000 * US_DOLLAR).into()),
		];
		let evaluator: AccountIdOf<T> = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

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
		let stored_evaluation =
			Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_unbond.id)).unwrap();
		assert!(stored_evaluation.rewarded_or_slashed.is_some());

		// Balances
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let reward_info = match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(reward_info) => reward_info,
			_ => panic!("EvaluatorsOutcome should be Rewarded"),
		};
		let total_reward =
			BenchInstantiator::<T>::calculate_total_reward_for_evaluation(stored_evaluation.clone(), reward_info);
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![evaluator.clone()])[0];
		assert_eq!(ct_amount, total_reward);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationRewarded {
				project_id,
				evaluator: evaluator.clone(),
				id: stored_evaluation.id,
				amount: total_reward,
				caller: evaluator,
			}
			.into(),
		);
	}

	#[benchmark]
	fn evaluation_reward_payout_for_no_ct_account_creation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations: Vec<UserToUSDBalance<T>> = vec![
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_1", 0, 0), (50_000 * US_DOLLAR).into()),
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_1", 0, 0), (25_000 * US_DOLLAR).into()),
			UserToUSDBalance::new(account::<AccountIdOf<T>>("evaluator_3", 0, 0), (32_000 * US_DOLLAR).into()),
		];
		let evaluator: AccountIdOf<T> = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_id = inst.create_finished_project(
			default_project::<T>(inst.get_new_nonce(), issuer.clone()),
			issuer,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let mut evaluations_to_unbond =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())));

		let pre_evaluation = evaluations_to_unbond.next().unwrap();
		let bench_evaluation = evaluations_to_unbond.next().unwrap();

		Pallet::<T>::evaluation_reward_payout_for(
			RawOrigin::Signed(evaluator.clone()).into(),
			project_id,
			evaluator.clone(),
			pre_evaluation.id,
		)
		.unwrap();

		#[extrinsic_call]
		evaluation_reward_payout_for(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			bench_evaluation.id,
		);

		// * validity checks *
		// Storage
		let stored_evaluation = Evaluations::<T>::get((project_id, evaluator.clone(), bench_evaluation.id)).unwrap();
		assert!(stored_evaluation.rewarded_or_slashed.is_some());

		// Balances
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let reward_info = match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(reward_info) => reward_info,
			_ => panic!("EvaluatorsOutcome should be Rewarded"),
		};

		let pre_reward =
			BenchInstantiator::<T>::calculate_total_reward_for_evaluation(pre_evaluation.clone(), reward_info.clone());
		let bench_reward =
			BenchInstantiator::<T>::calculate_total_reward_for_evaluation(bench_evaluation.clone(), reward_info);
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![evaluator.clone()])[0];
		assert_eq!(ct_amount, pre_reward + bench_reward);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationRewarded {
				project_id,
				evaluator: evaluator.clone(),
				id: stored_evaluation.id,
				amount: bench_reward,
				caller: evaluator,
			}
			.into(),
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
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
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
		let stored_evaluation =
			Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_unbond.id)).unwrap();
		assert!(stored_evaluation.rewarded_or_slashed.is_some());
		let slashed_amount = T::EvaluatorSlash::get() * evaluation_to_unbond.original_plmc_bond;
		let current_plmc_bond = evaluation_to_unbond.current_plmc_bond.saturating_sub(slashed_amount);
		assert_eq!(stored_evaluation.current_plmc_bond, current_plmc_bond);

		// Balance
		let treasury_account = T::TreasuryAccount::get();
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![evaluator.clone()], HoldReason::Evaluation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(bonded_plmc, stored_evaluation.current_plmc_bond);
		let free_treasury_plmc = inst.get_free_plmc_balances_for(vec![treasury_account])[0].plmc_amount;
		assert_eq!(free_treasury_plmc, slashed_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationSlashed {
				project_id,
				evaluator: evaluator.clone(),
				id: stored_evaluation.id,
				amount: slashed_amount,
				caller: evaluator,
			}
			.into(),
		);
	}

	#[benchmark]
	fn bid_ct_mint_for_with_ct_account_creation() {
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

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
			Event::ContributionTokenMinted { releaser: bidder.clone(), project_id, claimer: bidder, amount: ct_amount }
				.into(),
		);
	}

	#[benchmark]
	fn bid_ct_mint_for_no_ct_account_creation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bids: Vec<BidParams<T>> = vec![
			BidParams::new(
				account::<AccountIdOf<T>>("bidder_1", 0, 0),
				(40_000 * ASSET_UNIT).into(),
				1u8,
				AcceptedFundingAsset::USDT,
			),
			BidParams::new(
				account::<AccountIdOf<T>>("bidder_1", 0, 0),
				(5_000 * ASSET_UNIT).into(),
				7u8,
				AcceptedFundingAsset::USDT,
			),
		];
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let mut bids_to_mint_ct = inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())));

		let pre_bid_to_mint_ct = bids_to_mint_ct.next().unwrap();
		let bench_bid_to_mint_ct = bids_to_mint_ct.next().unwrap();

		Pallet::<T>::bid_ct_mint_for(
			RawOrigin::Signed(bidder.clone()).into(),
			project_id,
			bidder.clone(),
			pre_bid_to_mint_ct.id,
		)
		.unwrap();

		#[extrinsic_call]
		bid_ct_mint_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bench_bid_to_mint_ct.id);

		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::get((project_id, bidder.clone(), bench_bid_to_mint_ct.id)).unwrap();
		assert!(stored_bid.ct_minted);

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![bidder.clone()])[0];
		assert_eq!(ct_amount, pre_bid_to_mint_ct.final_ct_amount + stored_bid.final_ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionTokenMinted {
				releaser: bidder.clone(),
				project_id,
				claimer: bidder,
				amount: bench_bid_to_mint_ct.final_ct_amount,
			}
			.into(),
		);
	}

	#[benchmark]
	fn contribution_ct_mint_for_with_ct_account_creation() {
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

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
		let stored_contribution =
			Contributions::<T>::get((project_id, contributor.clone(), contribution_to_mint_ct.id)).unwrap();
		assert!(stored_contribution.ct_minted);

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![contributor.clone()])[0];
		assert_eq!(stored_contribution.ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionTokenMinted {
				releaser: contributor.clone(),
				project_id,
				claimer: contributor,
				amount: ct_amount,
			}
			.into(),
		);
	}

	#[benchmark]
	fn contribution_ct_mint_for_no_ct_account_creation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributions: Vec<ContributionParams<T>> = vec![
			ContributionParams::new(
				account::<AccountIdOf<T>>("contributor_1", 0, 0),
				(10_000 * ASSET_UNIT).into(),
				1u8,
				AcceptedFundingAsset::USDT,
			),
			ContributionParams::new(
				account::<AccountIdOf<T>>("contributor_1", 0, 0),
				(6_000 * ASSET_UNIT).into(),
				1u8,
				AcceptedFundingAsset::USDT,
			),
			ContributionParams::new(
				account::<AccountIdOf<T>>("contributor_3", 0, 0),
				(30_000 * ASSET_UNIT).into(),
				1u8,
				AcceptedFundingAsset::USDT,
			),
		];
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let mut contributions_to_mint_ct =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())));

		let pre_contribution_to_mint_ct = contributions_to_mint_ct.next().unwrap();
		let bench_contribution_to_mint_ct = contributions_to_mint_ct.next().unwrap();

		Pallet::<T>::contribution_ct_mint_for(
			RawOrigin::Signed(contributor.clone()).into(),
			project_id,
			contributor.clone(),
			pre_contribution_to_mint_ct.id,
		)
		.unwrap();

		#[extrinsic_call]
		contribution_ct_mint_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			bench_contribution_to_mint_ct.id,
		);

		// * validity checks *
		// Storage
		let stored_contribution =
			Contributions::<T>::get((project_id, contributor.clone(), bench_contribution_to_mint_ct.id)).unwrap();
		assert!(stored_contribution.ct_minted);

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![contributor.clone()])[0];
		assert_eq!(ct_amount, pre_contribution_to_mint_ct.ct_amount + bench_contribution_to_mint_ct.ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionTokenMinted {
				releaser: contributor.clone(),
				project_id,
				claimer: contributor,
				amount: bench_contribution_to_mint_ct.ct_amount,
			}
			.into(),
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

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
		let total_vested =
			T::Vesting::total_scheduled_amount(&bidder, HoldReason::Participation(project_id).into()).unwrap();
		assert_eq!(vest_info.total_amount, total_vested);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidPlmcVestingScheduled {
				project_id,
				bidder: bidder.clone(),
				id: stored_bid.id,
				amount: vest_info.total_amount,
				caller: bidder,
			}
			.into(),
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

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
		let stored_contribution =
			Contributions::<T>::get((project_id, contributor.clone(), contribution_to_vest.id)).unwrap();
		assert!(stored_contribution.plmc_vesting_info.is_some());
		let vest_info = stored_contribution.plmc_vesting_info.unwrap();
		let total_vested =
			T::Vesting::total_scheduled_amount(&contributor, HoldReason::Participation(project_id).into()).unwrap();
		assert_eq!(vest_info.total_amount, total_vested);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionPlmcVestingScheduled {
				project_id,
				contributor: contributor.clone(),
				id: stored_contribution.id,
				amount: vest_info.total_amount,
				caller: contributor,
			}
			.into(),
		);
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let bid_to_payout =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		payout_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder.clone(), bid_to_payout.id);

		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::get((project_id, bidder.clone(), bid_to_payout.id)).unwrap();
		assert!(stored_bid.funds_released);

		// Balances
		let asset = stored_bid.funding_asset.to_assethub_id();
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![project_details.issuer])[0].asset_amount;
		assert_eq!(free_assets, stored_bid.funding_asset_amount_locked);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidFundingPaidOut { project_id, bidder, id: stored_bid.id, amount: free_assets, caller: issuer }
				.into(),
		);
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

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Success(CleanerState::Initialized(PhantomData))
		);

		let contribution_to_payout =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		payout_contribution_funds_for(
			RawOrigin::Signed(issuer.clone()),
			project_id,
			contributor.clone(),
			contribution_to_payout.id,
		);

		// * validity checks *
		// Storage
		let stored_contribution =
			Contributions::<T>::get((project_id, contributor.clone(), contribution_to_payout.id)).unwrap();
		assert!(stored_contribution.funds_released);

		// Balances
		let asset = stored_contribution.funding_asset.to_assethub_id();
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![project_details.issuer])[0].asset_amount;
		assert_eq!(free_assets, stored_contribution.funding_asset_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionFundingPaidOut {
				project_id,
				contributor,
				id: stored_contribution.id,
				amount: free_assets,
				caller: issuer,
			}
			.into(),
		);
	}

	#[benchmark]
	fn decide_project_outcome(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Total amount of storage items iterated through in `ProjectsToUpdate` when trying to remove our project in `remove_from_update_store`.
		// Upper bound is assumed to be enough
		y: Linear<1, 10_000>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion
		let u32_remaining_vecs: u32 = y.saturating_sub(x).into();
		let time_advance: u32 = 1 + u32_remaining_vecs + 1;
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let evaluations = default_evaluations::<T>();
		let bids = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(40) * target_funding_amount,
			1u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.1);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(40) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();

		let current_block = inst.current_block();
		let insertion_block_number: BlockNumberFor<T> = current_block + One::one();

		fill_projects_to_update::<T>(x, insertion_block_number, Some(y));

		#[extrinsic_call]
		decide_project_outcome(RawOrigin::Signed(issuer), project_id, FundingOutcomeDecision::AcceptFunding);

		// * validity checks *
		// Storage
		let project_status = inst.get_update_pair(project_id).1;

		assert_eq!(project_status, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding));

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ProjectOutcomeDecided { project_id, decision: FundingOutcomeDecision::AcceptFunding }.into(),
		);
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let bid_to_payout =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
		let asset = bid_to_payout.funding_asset.to_assethub_id();
		let free_assets_before = inst.get_free_foreign_asset_balances_for(asset, vec![bidder.clone()])[0].asset_amount;
		#[extrinsic_call]
		release_bid_funds_for(RawOrigin::Signed(issuer.clone()), project_id, bidder.clone(), bid_to_payout.id);

		// * validity checks *
		// Storage
		let stored_bid = Bids::<T>::get((project_id, bidder.clone(), bid_to_payout.id)).unwrap();
		assert!(stored_bid.funds_released);

		// Balances
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![bidder.clone()])[0].asset_amount;
		assert_eq!(free_assets, stored_bid.funding_asset_amount_locked + free_assets_before);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidFundingReleased {
				project_id,
				bidder,
				id: stored_bid.id,
				amount: stored_bid.funding_asset_amount_locked,
				caller: issuer,
			}
			.into(),
		);
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			1u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_id =
			inst.create_finished_project(project_metadata, issuer, evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).cleanup,
			Cleaner::Failure(CleanerState::Initialized(PhantomData))
		);

		let contribution_to_payout =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		let asset = contribution_to_payout.funding_asset.to_assethub_id();
		let free_assets_before =
			inst.get_free_foreign_asset_balances_for(asset, vec![contributor.clone()])[0].asset_amount;
		#[extrinsic_call]
		release_contribution_funds_for(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			contribution_to_payout.id,
		);

		// * validity checks *
		// Storage
		let stored_contribution =
			Contributions::<T>::get((project_id, contributor.clone(), contribution_to_payout.id)).unwrap();
		assert!(stored_contribution.funds_released);

		// Balances
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![contributor.clone()])[0].asset_amount;
		assert_eq!(free_assets, stored_contribution.funding_asset_amount + free_assets_before);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionFundingReleased {
				project_id,
				contributor: contributor.clone(),
				id: stored_contribution.id,
				amount: stored_contribution.funding_asset_amount,
				caller: contributor,
			}
			.into(),
		);
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			10u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer, evaluations, bids, contributions, vec![]);

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
		bid_unbond_for(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), stored_bid.id);

		// * validity checks *
		// Storage
		assert!(!Bids::<T>::contains_key((project_id, bidder.clone(), stored_bid.id)));
		// Balances
		let reserved_plmc = inst
			.get_reserved_plmc_balances_for(vec![bidder.clone()], HoldReason::Participation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(reserved_plmc, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BondReleased { project_id, amount: stored_bid.plmc_bond, bonder: bidder.clone(), releaser: bidder }
				.into(),
		);
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
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size.0);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			1u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions: Vec<ContributionParams<T>> = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
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

		// * validity checks *
		// Storage
		assert!(!Contributions::<T>::contains_key((project_id, contributor.clone(), stored_contribution.id)));
		// Balances
		let reserved_plmc = inst
			.get_reserved_plmc_balances_for(vec![contributor.clone()], HoldReason::Participation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(reserved_plmc, 0.into());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BondReleased {
				project_id,
				amount: stored_contribution.plmc_bond,
				bonder: contributor,
				releaser: issuer,
			}
			.into(),
		);
	}

	//
	// on_initialize
	//

	//do_evaluation_end
	#[benchmark]
	fn end_evaluation_success(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
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
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().ct_account_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(ct_account_deposits);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");

		let evaluation_end_block =
			inst.get_project_details(project_id).phase_transition_points.evaluation.end().unwrap();
		// move block manually without calling any hooks, to avoid triggering the transition outside the benchmarking context
		frame_system::Pallet::<T>::set_block_number(evaluation_end_block + One::one());

		let insertion_block_number =
			inst.current_block() + One::one() + <T as Config>::AuctionInitializePeriodDuration::get();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		// Instead of advancing in time for the automatic `do_evaluation_end` call in on_initialize, we call it directly to benchmark it
		#[block]
		{
			Pallet::<T>::do_evaluation_end(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::AuctionInitializePeriod);
	}

	#[benchmark]
	fn end_evaluation_failure() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());
		let project_details = inst.get_project_details(project_id);

		let evaluation_usd_target =
			<T as Config>::EvaluationSuccessThreshold::get() * project_details.fundraising_target;
		// we only fund 50% of the minimum threshold for the evaluation round, since we want it to fail
		let evaluations = vec![
			UserToUSDBalance::new(
				account::<AccountIdOf<T>>("evaluator_1", 0, 0),
				(Percent::from_percent(5) * evaluation_usd_target).into(),
			),
			UserToUSDBalance::new(
				account::<AccountIdOf<T>>("evaluator_2", 0, 0),
				(Percent::from_percent(20) * evaluation_usd_target).into(),
			),
			UserToUSDBalance::new(
				account::<AccountIdOf<T>>("evaluator_3", 0, 0),
				(Percent::from_percent(25) * evaluation_usd_target).into(),
			),
		];
		let plmc_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(evaluations.clone());
		let existential_plmc: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().existential_deposits();
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().ct_account_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(ct_account_deposits);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");

		let evaluation_end_block =
			inst.get_project_details(project_id).phase_transition_points.evaluation.end().unwrap();
		// move block manually without calling any hooks, to avoid triggering the transition outside the benchmarking context
		frame_system::Pallet::<T>::set_block_number(evaluation_end_block + One::one());

		// Instead of advancing in time for the automatic `do_evaluation_end` call in on_initialize, we call it directly to benchmark it
		#[block]
		{
			Pallet::<T>::do_evaluation_end(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::EvaluationFailed);
	}

	//do_english_auction
	#[benchmark]
	fn start_auction_automatically(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// No `y` param because we don't need to remove the automatic transition from storage
	) {
		// * setup *
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
		let ct_account_deposits: Vec<UserToPLMCBalance<T>> = plmc_for_evaluating.accounts().ct_account_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(ct_account_deposits);
		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.bond_for_users(project_id, evaluations).expect("All evaluations are accepted");

		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		let current_block = inst.current_block();
		let automatic_transition_block =
			current_block + <T as Config>::AuctionInitializePeriodDuration::get() + One::one();
		let insertion_block_number: BlockNumberFor<T> =
			automatic_transition_block + T::EnglishAuctionDuration::get() + One::one();
		let block_number = insertion_block_number;

		fill_projects_to_update::<T>(x, block_number, None);

		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the extrinsic
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(automatic_transition_block);

		let jwt = get_mock_jwt(issuer.clone(), InvestorType::Institutional);
		#[extrinsic_call]
		start_auction(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::AuctionRound(AuctionPhase::English));

		// Events
		let current_block = inst.current_block();
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::EnglishAuctionStarted { project_id, when: current_block.into() }.into(),
		);
	}

	// do_candle_auction
	#[benchmark]
	fn start_candle_phase(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_auctioning_project(project_metadata, issuer.clone(), default_evaluations());

		let english_end_block =
			inst.get_project_details(project_id).phase_transition_points.english_auction.end().unwrap();
		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the extrinsic
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(english_end_block + One::one());

		let insertion_block_number = inst.current_block() + T::CandleAuctionDuration::get() + One::one();

		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_candle_auction(project_id).unwrap();
		}
		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::AuctionRound(AuctionPhase::Candle));

		// Events
		let current_block = inst.current_block();
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::CandleAuctionStarted { project_id, when: current_block.into() }.into(),
		);
	}

	// do_community_funding
	// Should be complex due to calling `calculate_weighted_average_price`
	#[benchmark]
	fn start_community_funding_success(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Accepted Bids
		y: Linear<1, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
		// Failed Bids
		z: Linear<0, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);
		let bounded_name = BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
		let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
		let metadata_hash = hashed(format!("{}-{}", METADATA, 69));
		// default has 50k allocated for bidding, so we cannot test the cap of bidding (100k bids) with it, since the ticket size is 1.
		let project_metadata = ProjectMetadata {
			token_information: CurrencyMetadata {
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: BalanceOf::<T>::try_from(8_000_000_0_000_000_000u128)
				.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
			total_allocation_size: (
				BalanceOf::<T>::try_from((10 * (y + z) + 1) as u128 * ASSET_UNIT)
					.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				BalanceOf::<T>::try_from(50_000u128 * ASSET_UNIT)
					.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
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
			funding_destination_account: issuer.clone(),
			offchain_information_hash: Some(metadata_hash.into()),
		};
		let project_id =
			inst.create_auctioning_project(project_metadata.clone(), issuer.clone(), default_evaluations());

		let accepted_bids = (0..y)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(10u128 * ASSET_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let rejected_bids = (0..z)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(10u128 * ASSET_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let all_bids = accepted_bids.iter().chain(rejected_bids.iter()).cloned().collect_vec();

		let plmc_needed_for_bids =
			BenchInstantiator::<T>::calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
		let plmc_ed = all_bids.accounts().existential_deposits();
		let plmc_ct_account_deposit = all_bids.accounts().ct_account_deposits();
		let funding_asset_needed_for_bids =
			BenchInstantiator::<T>::calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);

		inst.mint_plmc_to(plmc_needed_for_bids);
		inst.mint_plmc_to(plmc_ed);
		inst.mint_plmc_to(plmc_ct_account_deposit);
		inst.mint_foreign_asset_to(funding_asset_needed_for_bids);

		inst.bid_for_users(project_id, accepted_bids).unwrap();

		let now = inst.current_block();
		frame_system::Pallet::<T>::set_block_number(now + <T as Config>::EnglishAuctionDuration::get());
		// automatic transition to candle
		inst.advance_time(1u32.into()).unwrap();

		// testing always produced this random ending
		let random_ending: BlockNumberFor<T> = 9176u32.into();
		frame_system::Pallet::<T>::set_block_number(random_ending + 2u32.into());

		inst.bid_for_users(project_id, rejected_bids).unwrap();

		let auction_candle_end_block =
			inst.get_project_details(project_id).phase_transition_points.candle_auction.end().unwrap();
		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the fn
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(auction_candle_end_block + One::one());
		let now = inst.current_block();

		let community_end_block = now + T::CommunityFundingDuration::get();

		let insertion_block_number = community_end_block + One::one();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_community_funding(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::CommunityRound);

		let accepted_bids_count =
			Bids::<T>::iter_prefix_values((project_id,)).filter(|b| matches!(b.status, BidStatus::Accepted)).count();
		let rejected_bids_count =
			Bids::<T>::iter_prefix_values((project_id,)).filter(|b| matches!(b.status, BidStatus::Rejected(_))).count();
		assert_eq!(rejected_bids_count, z as usize);
		assert_eq!(accepted_bids_count, y as usize);

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::CommunityFundingStarted { project_id }.into());
	}

	#[benchmark]
	fn start_community_funding_failure(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_auctioning_project(project_metadata, issuer.clone(), default_evaluations());

		// no bids are made, so the project fails
		run_blocks_to_execute_next_transition(project_id, None, &mut inst);

		let auction_candle_end_block =
			inst.get_project_details(project_id).phase_transition_points.candle_auction.end().unwrap();
		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the fn
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(auction_candle_end_block + One::one());
		let now = inst.current_block();

		let community_end_block = now + T::CommunityFundingDuration::get();

		let insertion_block_number = community_end_block + One::one();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_community_funding(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::FundingFailed);

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::AuctionFailed { project_id }.into());
	}

	// do_remainder_funding
	#[benchmark]
	fn start_remainder_funding(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_community_contributing_project(
			project_metadata,
			issuer.clone(),
			default_evaluations(),
			default_bids(),
		);

		let community_end_block = inst.get_project_details(project_id).phase_transition_points.community.end().unwrap();

		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the fn
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(community_end_block + One::one());

		let now = inst.current_block();
		let remainder_end_block = now + T::RemainderFundingDuration::get();
		let insertion_block_number = remainder_end_block + 1u32.into();

		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_remainder_funding(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::RemainderRound);

		// Events
		frame_system::Pallet::<T>::assert_last_event(Event::<T>::RemainderFundingStarted { project_id }.into());
	}

	// do_end_funding
	#[benchmark]
	fn end_funding_automatically_rejected_evaluators_slashed(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);

		let automatically_rejected_threshold = Percent::from_percent(33);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
		);

		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::RemainderRound);
		let last_funding_block = project_details.phase_transition_points.remainder.end().unwrap();

		frame_system::Pallet::<T>::set_block_number(last_funding_block + 1u32.into());

		let insertion_block_number = inst.current_block() + 1u32.into();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_end_funding(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingFailed);
	}
	#[benchmark]
	fn end_funding_awaiting_decision_evaluators_slashed(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);

		let automatically_rejected_threshold = Percent::from_percent(75);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
		);

		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::RemainderRound);
		let last_funding_block = project_details.phase_transition_points.remainder.end().unwrap();

		frame_system::Pallet::<T>::set_block_number(last_funding_block + 1u32.into());

		let insertion_block_number = inst.current_block() + T::ManualAcceptanceDuration::get().into() + 1u32.into();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_end_funding(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::AwaitingProjectDecision);
		assert_eq!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed)
	}
	#[benchmark]
	fn end_funding_awaiting_decision_evaluators_unchanged(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);

		let automatically_rejected_threshold = Percent::from_percent(89);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
		);

		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::RemainderRound);
		let last_funding_block = project_details.phase_transition_points.remainder.end().unwrap();

		frame_system::Pallet::<T>::set_block_number(last_funding_block + 1u32.into());

		let insertion_block_number = inst.current_block() + T::ManualAcceptanceDuration::get().into() + 1u32.into();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_end_funding(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::AwaitingProjectDecision);
		assert_eq!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Unchanged)
	}
	#[benchmark]
	fn end_funding_automatically_accepted_evaluators_rewarded(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// How many evaluations have been made. Used when calculating evaluator rewards
		y: Linear<1, { <T as Config>::MaxEvaluationsPerProject::get() }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);

		let automatically_rejected_threshold = Percent::from_percent(91);

		let mut evaluations = (0..y.saturating_sub(1))
			.map(|i| {
				UserToUSDBalance::<T>::new(account::<AccountIdOf<T>>("evaluator", 0, i), (10u128 * ASSET_UNIT).into())
			})
			.collect_vec();

		let evaluation_target_usd = <T as Config>::EvaluationSuccessThreshold::get() * target_funding_amount;
		evaluations.push(UserToUSDBalance::<T>::new(
			account::<AccountIdOf<T>>("evaluator_success", 0, 69420),
			evaluation_target_usd,
		));

		let plmc_needed_for_evaluating = BenchInstantiator::<T>::calculate_evaluation_plmc_spent(evaluations.clone());
		let plmc_ed = evaluations.accounts().existential_deposits();
		let plmc_ct_account_deposit = evaluations.accounts().ct_account_deposits();

		inst.mint_plmc_to(plmc_needed_for_evaluating);
		inst.mint_plmc_to(plmc_ed);
		inst.mint_plmc_to(plmc_ct_account_deposit);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer.clone(),
			evaluations,
			bids,
			contributions,
		);

		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::RemainderRound);
		let last_funding_block = project_details.phase_transition_points.remainder.end().unwrap();

		frame_system::Pallet::<T>::set_block_number(last_funding_block + 1u32.into());

		let insertion_block_number = inst.current_block() + T::SuccessToSettlementTime::get().into();
		fill_projects_to_update::<T>(x, insertion_block_number, None);

		#[block]
		{
			Pallet::<T>::do_end_funding(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);
	}

	// do_project_decision
	#[benchmark]
	fn project_decision_accept_funding() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);
		let manual_outcome_threshold = Percent::from_percent(50);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
			vec![],
		);

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		#[block]
		{
			Pallet::<T>::do_project_decision(project_id, FundingOutcomeDecision::AcceptFunding).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingSuccessful);
	}

	#[benchmark]
	fn project_decision_reject_funding() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);
		let manual_outcome_threshold = Percent::from_percent(50);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
			vec![],
		);

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AwaitingProjectDecision);

		#[block]
		{
			Pallet::<T>::do_project_decision(project_id, FundingOutcomeDecision::RejectFunding).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingFailed);
	}

	// do_start_settlement
	#[benchmark]
	fn start_settlement_funding_success() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		// let issuer_mint = UserToPLMCBalance::<T>::new(issuer.clone(), (100 * ASSET_UNIT).into());
		// inst.mint_plmc_to(vec![issuer_mint]);

		#[block]
		{
			Pallet::<T>::do_start_settlement(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Success(CleanerState::Initialized(PhantomData)));
	}

	#[benchmark]
	fn start_settlement_funding_failure() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project::<T>(inst.get_new_nonce(), issuer.clone());
		let target_funding_amount: BalanceOf<T> = project_metadata
			.minimum_price
			.saturating_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1);

		let bids: Vec<BidParams<T>> = BenchInstantiator::generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			1u128.into(),
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = BenchInstantiator::generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			bids,
			contributions,
			vec![],
		);

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::FundingFailed);

		#[block]
		{
			Pallet::<T>::do_start_settlement(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.cleanup, Cleaner::Failure(CleanerState::Initialized(PhantomData)));
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::mock::{new_test_ext, TestRuntime};

		#[test]
		fn bench_create() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_create());
			});
		}

		#[test]
		fn bench_edit_metadata() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_edit_metadata());
			});
		}

		#[test]
		fn bench_start_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_evaluation());
			});
		}

		#[test]
		fn bench_first_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_first_evaluation());
			});
		}

		#[test]
		fn bench_second_to_limit_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_second_to_limit_evaluation());
			});
		}

		#[test]
		fn bench_evaluation_over_limit() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation_over_limit());
			});
		}

		#[test]
		fn bench_start_auction_manually() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_auction_manually());
			});
		}

		#[test]
		fn bench_start_auction_automatically() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_auction_automatically());
			});
		}

		#[test]
		fn bench_bid_with_ct_deposit() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid_with_ct_deposit());
			});
		}

		#[test]
		fn bench_bid_no_ct_deposit() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid_no_ct_deposit());
			});
		}

		// #[test]
		// fn bench_first_contribution_no_ct_deposit() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_first_contribution_no_ct_deposit());
		// 	});
		// }

		// #[test]
		// fn bench_first_contribution_with_ct_deposit() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_first_contribution_with_ct_deposit());
		// 	});
		// }

		// #[test]
		// fn bench_first_contribution_ends_round_no_ct_deposit() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_first_contribution_ends_round_no_ct_deposit());
		// 	});
		// }

		// #[test]
		// fn bench_first_contribution_ends_round_with_ct_deposit() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_first_contribution_ends_round_with_ct_deposit());
		// 	});
		// }

		// #[test]
		// fn bench_second_to_limit_contribution() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_second_to_limit_contribution());
		// 	});
		// }

		// #[test]
		// fn bench_second_to_limit_contribution_ends_round() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_second_to_limit_contribution_ends_round());
		// 	});
		// }

		// #[test]
		// fn bench_contribution_over_limit() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_contribution_over_limit());
		// 	});
		// }

		#[test]
		fn bench_evaluation_unbond_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation_unbond_for());
			});
		}

		#[test]
		fn bench_evaluation_reward_payout_for_with_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation_reward_payout_for_with_ct_account_creation());
			});
		}

		#[test]
		fn bench_evaluation_reward_payout_for_no_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation_reward_payout_for_no_ct_account_creation());
			});
		}

		#[test]
		fn bench_evaluation_slash_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation_slash_for());
			});
		}

		#[test]
		fn bench_bid_ct_mint_for_with_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid_ct_mint_for_with_ct_account_creation());
			});
		}

		#[test]
		fn bench_bid_ct_mint_for_no_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid_ct_mint_for_no_ct_account_creation());
			});
		}

		#[test]
		fn bench_contribution_ct_mint_for_with_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_contribution_ct_mint_for_with_ct_account_creation());
			});
		}

		#[test]
		fn bench_contribution_ct_mint_for_no_ct_account_creation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_contribution_ct_mint_for_no_ct_account_creation());
			});
		}

		#[test]
		fn bench_start_bid_vesting_schedule_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_bid_vesting_schedule_for());
			});
		}

		#[test]
		fn bench_start_contribution_vesting_schedule_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_contribution_vesting_schedule_for());
			});
		}

		#[test]
		fn bench_payout_bid_funds_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_payout_bid_funds_for());
			});
		}

		#[test]
		fn bench_payout_contribution_funds_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_payout_contribution_funds_for());
			});
		}

		#[test]
		fn bench_decide_project_outcome() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_decide_project_outcome());
			});
		}

		#[test]
		fn bench_release_bid_funds_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_release_bid_funds_for());
			});
		}

		#[test]
		fn bench_release_contribution_funds_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_release_contribution_funds_for());
			});
		}

		#[test]
		fn bench_bid_unbond_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid_unbond_for());
			});
		}

		#[test]
		fn bench_contribution_unbond_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_contribution_unbond_for());
			});
		}

		// on_initialize benches
		#[test]
		fn bench_end_evaluation_success() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_evaluation_success());
			});
		}

		#[test]
		fn bench_end_evaluation_failure() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_evaluation_failure());
			});
		}

		#[test]
		fn bench_start_candle_phase() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_candle_phase());
			});
		}

		#[test]
		fn bench_start_community_funding_success() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_community_funding_success());
			});
		}

		#[test]
		fn bench_start_community_funding_failure() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_community_funding_success());
			});
		}

		#[test]
		fn bench_start_remainder_funding() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_remainder_funding());
			});
		}

		#[test]
		fn bench_start_settlement_funding_success() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_settlement_funding_success());
			});
		}

		#[test]
		fn bench_start_settlement_funding_failure() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_settlement_funding_failure());
			});
		}

		#[test]
		fn bench_project_decision_accept_funding() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_project_decision_accept_funding());
			});
		}

		#[test]
		fn bench_project_decision_reject_funding() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_project_decision_reject_funding());
			});
		}

		#[test]
		fn bench_end_funding_automatically_rejected_evaluators_slashed() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_automatically_rejected_evaluators_slashed());
			});
		}

		// #[test]
		// fn bench_end_funding_automatically_accepted_evaluators_rewarded() {
		// 	new_test_ext().execute_with(|| {
		// 		assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_automatically_accepted_evaluators_rewarded());
		// 	});
		// }

		#[test]
		fn bench_end_funding_awaiting_decision_evaluators_unchanged() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_awaiting_decision_evaluators_unchanged());
			});
		}

		#[test]
		fn bench_end_funding_awaiting_decision_evaluators_slashed() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_awaiting_decision_evaluators_slashed());
			});
		}
	}
}
