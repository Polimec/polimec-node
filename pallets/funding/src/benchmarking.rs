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
use crate::{
	instantiator::*,
	traits::{ProvideAssetPrice, SetPrices},
};
use frame_benchmarking::v2::*;
#[cfg(test)]
use frame_support::assert_ok;
use frame_support::{
	dispatch::RawOrigin,
	traits::{
		fungibles::{metadata::MetadataDeposit, Inspect},
		OriginTrait,
	},
	Parameter,
};
use itertools::Itertools;
#[allow(unused_imports)]
use pallet::Pallet as PalletFunding;
use parity_scale_codec::{Decode, Encode};
use polimec_common::{credentials::InvestorType, USD_DECIMALS, USD_UNIT};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{Get, Member, TrailingZeroInput, Zero};
use xcm::v3::MaxPalletNameLen;

const IPFS_CID: &str = "QmbvsJBhQtu9uAGVp7x4H77JkwAQxV7TA6xTfdeALuDiYB";
const CT_DECIMALS: u8 = 17;
const CT_UNIT: u128 = 10u128.pow(CT_DECIMALS as u32);
type BenchInstantiator<T> = Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;

pub fn usdt_id() -> u32 {
	AcceptedFundingAsset::USDT.to_assethub_id()
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
		mainnet_token_max_supply: BalanceOf::<T>::try_from(1_000_000 * CT_UNIT)
			.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		total_allocation_size: BalanceOf::<T>::try_from(1_000_000 * CT_UNIT)
			.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(10u128.into(), USD_DECIMALS, CT_DECIMALS)
			.unwrap(),

		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(
				BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				None,
			),
			institutional: TicketSize::new(
				BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				None,
			),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(
				BalanceOf::<T>::try_from(USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				None,
			),
			professional: TicketSize::new(
				BalanceOf::<T>::try_from(USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				None,
			),
			institutional: TicketSize::new(
				BalanceOf::<T>::try_from(USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
				None,
			),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: issuer,
		policy_ipfs_cid: Some(metadata_hash.into()),
	}
}

pub fn default_evaluations<T: Config>() -> Vec<UserToUSDBalance<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
{
	let threshold = <T as Config>::EvaluationSuccessThreshold::get();
	let default_project_metadata: ProjectMetadataOf<T> =
		default_project_metadata::<T>(account::<AccountIdOf<T>>("issuer", 0, 0));
	let funding_target =
		default_project_metadata.minimum_price.saturating_mul_int(default_project_metadata.total_allocation_size);
	let evaluation_target = threshold * funding_target;

	vec![
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_1", 0, 0),
			Percent::from_percent(35) * evaluation_target,
		),
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_2", 0, 0),
			Percent::from_percent(35) * evaluation_target,
		),
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_3", 0, 0),
			Percent::from_percent(35) * evaluation_target,
		),
	]
}

pub fn default_bids<T: Config>() -> Vec<BidParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
{
	let default_project_metadata = default_project_metadata::<T>(account::<AccountIdOf<T>>("issuer", 0, 0));
	let auction_funding_target = default_project_metadata.minimum_price.saturating_mul_int(
		default_project_metadata.auction_round_allocation_percentage * default_project_metadata.total_allocation_size,
	);
	let inst = BenchInstantiator::<T>::new(None);

	inst.generate_bids_from_total_usd(
		Percent::from_percent(95) * auction_funding_target,
		default_project_metadata.minimum_price,
		default_weights(),
		default_bidders::<T>(),
		default_bidder_multipliers(),
	)
}

pub fn full_bids<T>() -> Vec<BidParams<T>>
where
	T: Config,
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
{
	let inst = BenchInstantiator::<T>::new(None);
	let default_project = default_project_metadata::<T>(account::<AccountIdOf<T>>("issuer", 0, 0));
	let total_ct_for_bids = default_project.auction_round_allocation_percentage * default_project.total_allocation_size;
	let total_usd_for_bids = default_project.minimum_price.checked_mul_int(total_ct_for_bids).unwrap();
	inst.generate_bids_from_total_usd(
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
	T::Hash: From<H256>,
{
	let inst = BenchInstantiator::<T>::new(None);
	let default_project_metadata = default_project_metadata::<T>(account::<AccountIdOf<T>>("issuer", 0, 0));

	let funding_target =
		default_project_metadata.minimum_price.saturating_mul_int(default_project_metadata.total_allocation_size);
	let auction_funding_target = default_project_metadata.minimum_price.saturating_mul_int(
		default_project_metadata.auction_round_allocation_percentage * default_project_metadata.total_allocation_size,
	);

	let contributing_funding_target = funding_target - auction_funding_target;

	inst.generate_contributions_from_total_usd(
		Percent::from_percent(85) * contributing_funding_target,
		default_project_metadata.minimum_price,
		default_weights(),
		default_community_contributors::<T>(),
		default_community_contributor_multipliers(),
	)
}

pub fn default_remainder_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128>,
	T::Hash: From<H256>,
{
	let inst = BenchInstantiator::<T>::new(None);
	let default_project_metadata = default_project_metadata::<T>(account::<AccountIdOf<T>>("issuer", 0, 0));

	let funding_target =
		default_project_metadata.minimum_price.saturating_mul_int(default_project_metadata.total_allocation_size);
	let auction_funding_target = default_project_metadata.minimum_price.saturating_mul_int(
		default_project_metadata.auction_round_allocation_percentage * default_project_metadata.total_allocation_size,
	);

	let contributing_funding_target = funding_target - auction_funding_target;

	inst.generate_contributions_from_total_usd(
		Percent::from_percent(15) * contributing_funding_target,
		10u128.into(),
		default_weights(),
		default_remainder_contributors::<T>(),
		default_remainder_contributor_multipliers(),
	)
}

pub fn default_weights() -> Vec<u8> {
	vec![20u8, 15u8, 10u8, 25u8, 30u8]
}

pub fn default_evaluators<T: Config>() -> Vec<AccountIdOf<T>> {
	vec![
		account::<AccountIdOf<T>>("evaluator_1", 0, 0),
		account::<AccountIdOf<T>>("evaluator_2", 0, 0),
		account::<AccountIdOf<T>>("evaluator_3", 0, 0),
		account::<AccountIdOf<T>>("evaluator_4", 0, 0),
		account::<AccountIdOf<T>>("evaluator_5", 0, 0),
	]
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

pub fn default_community_contributors<T: Config>() -> Vec<AccountIdOf<T>> {
	vec![
		account::<AccountIdOf<T>>("contributor_1", 0, 0),
		account::<AccountIdOf<T>>("contributor_2", 0, 0),
		account::<AccountIdOf<T>>("contributor_3", 0, 0),
		account::<AccountIdOf<T>>("contributor_4", 0, 0),
		account::<AccountIdOf<T>>("contributor_5", 0, 0),
	]
}
pub fn default_remainder_contributors<T: Config>() -> Vec<AccountIdOf<T>> {
	vec![
		account::<AccountIdOf<T>>("bidder_1", 0, 0),
		account::<AccountIdOf<T>>("bidder_2", 0, 0),
		account::<AccountIdOf<T>>("evaluator_1", 0, 0),
		account::<AccountIdOf<T>>("evaluator_2", 0, 0),
		account::<AccountIdOf<T>>("contributor_6", 0, 0),
	]
}

pub fn default_bidder_multipliers() -> Vec<u8> {
	vec![10u8, 3u8, 1u8, 7u8, 4u8]
}
pub fn default_community_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 1u8, 1u8, 1u8, 1u8]
}
pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 11u8, 1u8, 1u8, 1u8]
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

// IMPORTANT: make sure your project starts at (block 1 + `total_vecs_in_storage` - `fully_filled_vecs_from_insertion`) to always have room to insert new vecs
pub fn fill_projects_to_update<T: Config>(
	fully_filled_vecs_from_insertion: u32,
	mut expected_insertion_block: BlockNumberFor<T>,
) {
	// fill the `ProjectsToUpdate` vectors from @ expected_insertion_block to @ expected_insertion_block+x, to benchmark all the failed insertion attempts
	for _ in 0..fully_filled_vecs_from_insertion {
		ProjectsToUpdate::<T>::insert(expected_insertion_block, (&69u32, UpdateType::EvaluationEnd));
		expected_insertion_block += 1u32.into();
	}
}

pub fn run_blocks_to_execute_next_transition<T: Config>(
	project_id: ProjectId,
	update_type: UpdateType,
	inst: &mut BenchInstantiator<T>,
) {
	let update_block = inst.get_update_block(project_id, &update_type).unwrap();
	frame_system::Pallet::<T>::set_block_number(update_block - 1u32.into());
	inst.advance_time(One::one()).unwrap();
}

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent> + pallet_balances::Config<Balance = BalanceOf<T>>,
	<T as Config>::RuntimeEvent: TryInto<Event<T>> + Parameter + Member,
	<T as Config>::Price: From<u128>,
	<T as Config>::Balance: From<u128> + Into<u128>,
	T::Hash: From<H256>,
	<T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
	<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
)]
mod benchmarks {
	use super::*;

	impl_benchmark_test_suite!(PalletFunding, crate::mock::new_test_ext(), crate::mock::TestRuntime);

	//
	// Extrinsics
	//
	#[benchmark]
	fn create_project() {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

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
			ed * 2u64.into() + metadata_deposit + ct_treasury_account_deposit,
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
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone());
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

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let issuer_funding = account::<AccountIdOf<T>>("issuer_funding", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone());

		let project_metadata = ProjectMetadataOf::<T> {
			token_information: CurrencyMetadata {
				name: BoundedVec::try_from("Contribution Token TEST v2".as_bytes().to_vec()).unwrap(),
				symbol: BoundedVec::try_from("CTESTv2".as_bytes().to_vec()).unwrap(),
				decimals: CT_DECIMALS - 2,
			},
			mainnet_token_max_supply: BalanceOf::<T>::try_from(200_000 * CT_UNIT)
				.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
			total_allocation_size: BalanceOf::<T>::try_from(200_000 * CT_UNIT)
				.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
			auction_round_allocation_percentage: Percent::from_percent(30u8),
			minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(
				11u128.into(),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(
					BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					Some(
						BalanceOf::<T>::try_from(10_000 * USD_UNIT)
							.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					),
				),
				institutional: TicketSize::new(
					BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					Some(
						BalanceOf::<T>::try_from(10_000 * USD_UNIT)
							.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					),
				),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(
					BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					Some(
						BalanceOf::<T>::try_from(10_000 * USD_UNIT)
							.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					),
				),
				professional: TicketSize::new(
					BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					Some(
						BalanceOf::<T>::try_from(10_000 * USD_UNIT)
							.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					),
				),
				institutional: TicketSize::new(
					BalanceOf::<T>::try_from(5000 * USD_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					Some(
						BalanceOf::<T>::try_from(10_000 * USD_UNIT)
							.unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
					),
				),
				phantom: Default::default(),
			},
			participation_currencies: vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC].try_into().unwrap(),
			funding_destination_account: issuer_funding.clone().clone(),
			policy_ipfs_cid: Some(BoundedVec::try_from(IPFS_CID.as_bytes().to_vec()).unwrap()),
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
	fn start_evaluation(
		// insertion attempts in add_to_update_store.
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_new_project(project_metadata.clone(), issuer.clone());

		// start_evaluation fn will try to add an automatic transition 1 block after the last evaluation block
		let block_number: BlockNumberFor<T> = inst.current_block() + T::EvaluationDuration::get() + One::one();
		// fill the `ProjectsToUpdate` vectors from @ block_number to @ block_number+x, to benchmark all the failed insertion attempts
		fill_projects_to_update::<T>(x, block_number);
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
		frame_system::Pallet::<T>::assert_last_event(
			Event::ProjectPhaseTransition { project_id, phase: ProjectPhases::Evaluation }.into(),
		)
	}

	#[benchmark]
	fn start_auction_manually(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion
		let time_advance: u32 = x + 2;
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer.clone());

		let evaluations = default_evaluations();
		let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.evaluate_for_users(project_id, evaluations).expect("All evaluations are accepted");

		run_blocks_to_execute_next_transition(project_id, UpdateType::EvaluationEnd, &mut inst);
		inst.advance_time(1u32.into()).unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

		let current_block = inst.current_block();
		// `do_auction_opening` fn will try to add an automatic transition 1 block after the last opening round block
		let insertion_block_number: BlockNumberFor<T> = current_block + T::AuctionOpeningDuration::get();

		fill_projects_to_update::<T>(x, insertion_block_number);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);
		#[extrinsic_call]
		start_auction(RawOrigin::Signed(issuer), jwt, project_id);

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::AuctionOpening);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::AuctionOpening }.into(),
		);
	}

	// - We don't know how many iterations it does in storage (i.e "x")
	#[benchmark]
	fn evaluation(
		// How many other evaluations the user did for that same project
		x: Linear<0, { T::MaxEvaluationsPerUser::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let test_evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
		whitelist_account!(test_evaluator);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer);

		let existing_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (200 * USD_UNIT).into());
		let extrinsic_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (1_000 * USD_UNIT).into());
		let existing_evaluations = vec![existing_evaluation; x as usize];

		let plmc_for_existing_evaluations = inst.calculate_evaluation_plmc_spent(existing_evaluations.clone(), false);
		let plmc_for_extrinsic_evaluation =
			inst.calculate_evaluation_plmc_spent(vec![extrinsic_evaluation.clone()], false);
		let existential_plmc: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_evaluation.accounts().existential_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(plmc_for_existing_evaluations.clone());
		inst.mint_plmc_to(plmc_for_extrinsic_evaluation.clone());

		inst.advance_time(One::one()).unwrap();

		// do "x" evaluations for this user
		inst.evaluate_for_users(project_id, existing_evaluations).expect("All evaluations are accepted");

		let extrinsic_plmc_bonded = plmc_for_extrinsic_evaluation[0].plmc_amount;
		let total_expected_plmc_bonded = inst
			.sum_balance_mappings(vec![plmc_for_existing_evaluations.clone(), plmc_for_extrinsic_evaluation.clone()]);

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
		let bonded_plmc = inst.get_reserved_plmc_balances_for(
			vec![extrinsic_evaluation.account.clone()],
			HoldReason::Evaluation(project_id).into(),
		)[0]
		.plmc_amount;
		assert_eq!(bonded_plmc, total_expected_plmc_bonded);

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
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let mut project_metadata = default_project_metadata::<T>(issuer.clone());
		project_metadata.mainnet_token_max_supply =
			(100_000 * CT_UNIT).try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.total_allocation_size =
			(100_000 * CT_UNIT).try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.minimum_price = PriceProviderOf::<T>::calculate_decimals_aware_price(
			PriceOf::<T>::checked_from_rational(100, 1).unwrap(),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap();

		let evaluations = inst.generate_successful_evaluations(
			project_metadata.clone(),
			default_evaluators::<T>(),
			default_weights(),
		);

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, evaluations);

		let existing_bid = BidParams::new(bidder.clone(), (50 * CT_UNIT).into(), 5u8, AcceptedFundingAsset::USDT);

		let existing_bids = vec![existing_bid; existing_bids_count as usize];
		let existing_bids_post_bucketing =
			inst.get_actual_price_charged_for_bucketed_bids(&existing_bids, project_metadata.clone(), None);
		let plmc_for_existing_bids = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&existing_bids,
			project_metadata.clone(),
			None,
			false,
		);

		let existential_deposits: Vec<UserToPLMCBalance<T>> = vec![bidder.clone()].existential_deposits();

		let usdt_for_existing_bids: Vec<UserToForeignAssets<T>> = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&existing_bids,
				project_metadata.clone(),
				None,
			);
		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_escrow_usdt_locked =
			inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()]);

		inst.mint_plmc_to(plmc_for_existing_bids.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_existing_bids.clone());

		// do "x" contributions for this user
		inst.bid_for_users(project_id, existing_bids.clone()).unwrap();

		// to call do_perform_bid several times, we need the bucket to reach its limit. You can only bid over 10 buckets
		// in a single bid, since the increase delta is 10% of the total allocation, and you cannot bid more than the allocation.
		let mut ct_amount = (50 * CT_UNIT).into();
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
			let plmc_for_new_bidder = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
				false,
			);
			let plmc_ed = plmc_for_new_bidder.accounts().existential_deposits();
			let usdt_for_new_bidder = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
			);

			inst.mint_plmc_to(plmc_for_new_bidder);
			inst.mint_plmc_to(plmc_ed);
			inst.mint_foreign_asset_to(usdt_for_new_bidder.clone());

			inst.bid_for_users(project_id, vec![bid_params]).unwrap();

			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let bucket_size = Percent::from_percent(10) * auction_allocation;
			ct_amount = bucket_size * (do_perform_bid_calls as u128).into();
			usdt_for_filler_bidder = usdt_for_new_bidder;
		}
		let extrinsic_bid = BidParams::new(bidder.clone(), ct_amount, 1u8, AcceptedFundingAsset::USDT);
		let original_extrinsic_bid = extrinsic_bid.clone();
		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		// we need to call this after bidding `x` amount of times, to get the latest bucket from storage
		let extrinsic_bids_post_bucketing = inst.get_actual_price_charged_for_bucketed_bids(
			&vec![extrinsic_bid.clone()],
			project_metadata.clone(),
			Some(current_bucket),
		);
		assert_eq!(extrinsic_bids_post_bucketing.len(), (do_perform_bid_calls as usize).max(1usize));

		let plmc_for_extrinsic_bids: Vec<UserToPLMCBalance<T>> = inst
			.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
				false,
			);
		let usdt_for_extrinsic_bids: Vec<UserToForeignAssets<T>> = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid],
				project_metadata.clone(),
				Some(current_bucket),
			);
		inst.mint_plmc_to(plmc_for_extrinsic_bids.clone());
		inst.mint_foreign_asset_to(usdt_for_extrinsic_bids.clone());

		let total_free_plmc = existential_deposits[0].plmc_amount;
		let total_plmc_participation_bonded =
			inst.sum_balance_mappings(vec![plmc_for_extrinsic_bids.clone(), plmc_for_existing_bids.clone()]);
		let total_free_usdt = Zero::zero();
		let total_escrow_usdt_locked = inst.sum_foreign_mappings(vec![
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
				when: None,
			};
			Bids::<T>::iter_prefix_values((project_id, bidder.clone()))
				.find(|stored_bid| bid_filter.matches_bid(stored_bid))
				.expect("bid not found");
		}

		// Bucket Storage Check
		let bucket_delta_amount = Percent::from_percent(10) *
			project_metadata.auction_round_allocation_percentage *
			project_metadata.total_allocation_size;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).unwrap() * project_metadata.minimum_price;

		let mut starting_bucket = Bucket::new(
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
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
					ct_amount,
					multiplier, ..
				},
				project_id == project_id,
				ct_amount == bid_params.amount,
				multiplier == bid_params.multiplier
			}
			.expect("Event has to be emitted");
		}
	}

	#[benchmark]
	fn bid(
		// amount of already made bids by the same user. Leave y::max (10) to make the extrinsic pass
		x: Linear<0, { T::MaxBidsPerUser::get() - 10 }>,
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

		let jwt = get_mock_jwt_with_cid(
			original_extrinsic_bid.bidder.clone(),
			InvestorType::Institutional,
			generate_did_from_account(original_extrinsic_bid.bidder.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);
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
		ends_round: Option<u32>,
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
		<T as Config>::SetPrices::set_prices();

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion
		let mut time_advance: u32 = 1;
		if let Some(y) = ends_round {
			time_advance += y + 1;
		}
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributor = account::<AccountIdOf<T>>("contributor", 0, 0);
		whitelist_account!(contributor);

		let project_metadata = default_project_metadata::<T>(issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			default_evaluations::<T>(),
			full_bids::<T>(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let existing_amount: BalanceOf<T> = (50 * CT_UNIT).into();
		let extrinsic_amount: BalanceOf<T> = if ends_round.is_some() {
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size -
				existing_amount * (x.min(<T as Config>::MaxContributionsPerUser::get() - 1) as u128).into()
		} else {
			(100 * CT_UNIT).into()
		};
		let existing_contribution =
			ContributionParams::new(contributor.clone(), existing_amount, 1u8, AcceptedFundingAsset::USDT);
		let extrinsic_contribution =
			ContributionParams::new(contributor.clone(), extrinsic_amount, 1u8, AcceptedFundingAsset::USDT);
		let existing_contributions = vec![existing_contribution; x as usize];

		let mut total_ct_sold: BalanceOf<T> = existing_amount * (x as u128).into() + extrinsic_amount;

		let plmc_for_existing_contributions =
			inst.calculate_contributed_plmc_spent(existing_contributions.clone(), price, false);
		let plmc_for_extrinsic_contribution =
			inst.calculate_contributed_plmc_spent(vec![extrinsic_contribution.clone()], price, false);
		let usdt_for_existing_contributions =
			inst.calculate_contributed_funding_asset_spent(existing_contributions.clone(), price);
		let usdt_for_extrinsic_contribution =
			inst.calculate_contributed_funding_asset_spent(vec![extrinsic_contribution.clone()], price);

		let existential_deposits: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_contribution.accounts().existential_deposits();

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_usdt_locked = inst.get_free_foreign_asset_balances_for(usdt_id(), vec![escrow_account.clone()]);

		inst.mint_plmc_to(plmc_for_existing_contributions.clone());
		inst.mint_plmc_to(plmc_for_extrinsic_contribution.clone());
		inst.mint_plmc_to(existential_deposits.clone());
		inst.mint_foreign_asset_to(usdt_for_existing_contributions.clone());
		inst.mint_foreign_asset_to(usdt_for_extrinsic_contribution.clone());

		// do "x" contributions for this user
		inst.contribute_for_users(project_id, existing_contributions).expect("All contributions are accepted");

		let mut total_plmc_bonded = inst.sum_balance_mappings(vec![
			plmc_for_existing_contributions.clone(),
			plmc_for_extrinsic_contribution.clone(),
		]);
		let mut total_usdt_locked = inst.sum_foreign_mappings(vec![
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

		if let Some(fully_filled_vecs_from_insertion) = ends_round {
			// if all CTs are sold, next round is scheduled for next block (either remainder or success)
			let expected_insertion_block = inst.current_block() + One::one();
			fill_projects_to_update::<T>(fully_filled_vecs_from_insertion, expected_insertion_block);
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

		let bid_ct_sold = crate::Bids::<T>::iter_prefix_values((project_id,))
			.map(|bid_in_project: BidInfoOf<T>| bid_in_project.final_ct_amount)
			.fold(Zero::zero(), |acc, x| acc + x);

		assert_eq!(
			stored_project_details.remaining_contribution_tokens,
			project_metadata.total_allocation_size.saturating_sub(total_ct_sold).saturating_sub(bid_ct_sold)
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
				id: stored_contribution.id,
				ct_amount: extrinsic_contribution.amount,
				funding_asset: stored_contribution.funding_asset,
				funding_amount: stored_contribution.funding_asset_amount,
				plmc_bond: stored_contribution.plmc_bond,
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

		let jwt = get_mock_jwt_with_cid(
			extrinsic_contribution.contributor.clone(),
			InvestorType::Retail,
			generate_did_from_account(extrinsic_contribution.contributor.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		community_contribute(
			RawOrigin::Signed(extrinsic_contribution.contributor.clone()),
			jwt,
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
	) {
		let ends_round = Some(y);

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

		let jwt = get_mock_jwt_with_cid(
			extrinsic_contribution.contributor.clone(),
			InvestorType::Retail,
			generate_did_from_account(extrinsic_contribution.contributor.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		community_contribute(
			RawOrigin::Signed(extrinsic_contribution.contributor.clone()),
			jwt,
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
	fn decide_project_outcome(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the extrinsic pass
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We need to leave enough block numbers to fill `ProjectsToUpdate` before our project insertion

		let time_advance: u32 = x + 2;
		frame_system::Pallet::<T>::set_block_number(time_advance.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let evaluations = default_evaluations::<T>();
		let bids = inst.generate_bids_from_total_usd(
			Percent::from_percent(30) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);

		let contributions = inst.generate_contributions_from_total_usd(
			Percent::from_percent(40) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			evaluations,
			bids,
			contributions,
			vec![],
		);

		inst.advance_time(One::one()).unwrap();

		let current_block = inst.current_block();
		let insertion_block_number: BlockNumberFor<T> = current_block + One::one();

		fill_projects_to_update::<T>(x, insertion_block_number);
		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		decide_project_outcome(RawOrigin::Signed(issuer), jwt, project_id, FundingOutcomeDecision::AcceptFunding);

		// * validity checks *
		// Storage
		let maybe_transition =
			inst.get_update_block(project_id, &UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding));
		assert!(maybe_transition.is_some());

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ProjectOutcomeDecided { project_id, decision: FundingOutcomeDecision::AcceptFunding }.into(),
		);
	}

	#[benchmark]
	fn settle_successful_evaluation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations: Vec<UserToUSDBalance<T>> = default_evaluations::<T>();
		let evaluator: AccountIdOf<T> = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		run_blocks_to_execute_next_transition(project_id, UpdateType::StartSettlement, &mut inst);

		let evaluation_to_settle =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());

		#[extrinsic_call]
		settle_successful_evaluation(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			evaluation_to_settle.id,
		);

		// * validity checks *
		// Evaluation should be removed
		assert!(Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_settle.id)).is_none());

		// Balances
		let project_details = ProjectsDetails::<T>::get(project_id).unwrap();
		let reward_info = match project_details.evaluation_round_info.evaluators_outcome {
			EvaluatorsOutcome::Rewarded(reward_info) => reward_info,
			_ => panic!("EvaluatorsOutcome should be Rewarded"),
		};
		let reward = Pallet::<T>::calculate_evaluator_reward(&evaluation_to_settle, &reward_info);

		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![evaluator.clone()])[0];
		assert_eq!(ct_amount, reward);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationSettled {
				project_id,
				account: evaluator.clone(),
				id: evaluation_to_settle.id,
				ct_amount: reward,
				slashed_plmc_amount: 0.into(),
			}
			.into(),
		);
	}

	#[benchmark]
	fn settle_failed_evaluation() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();
		let evaluator = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids = inst.generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer, evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)
		);

		let evaluation_to_settle =
			inst.execute(|| Evaluations::<T>::iter_prefix_values((project_id, evaluator.clone())).next().unwrap());

		let treasury_account = T::BlockchainOperationTreasury::get();
		let prev_free_treasury_plmc = inst.get_free_plmc_balances_for(vec![treasury_account])[0].plmc_amount;

		#[extrinsic_call]
		settle_failed_evaluation(
			RawOrigin::Signed(evaluator.clone()),
			project_id,
			evaluator.clone(),
			evaluation_to_settle.id,
		);

		// * validity checks *
		// Storage
		// Evaluation should be removed
		assert!(Evaluations::<T>::get((project_id, evaluator.clone(), evaluation_to_settle.id)).is_none());
		let slashed_amount = T::EvaluatorSlash::get() * evaluation_to_settle.original_plmc_bond;

		let reserved_plmc = inst
			.get_reserved_plmc_balances_for(vec![evaluator.clone()], HoldReason::Evaluation(project_id).into())[0]
			.plmc_amount;
		assert_eq!(reserved_plmc, 0.into());

		let treasury_account = T::BlockchainOperationTreasury::get();
		let post_free_treasury_plmc = inst.get_free_plmc_balances_for(vec![treasury_account])[0].plmc_amount;
		assert_eq!(post_free_treasury_plmc, prev_free_treasury_plmc + slashed_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::EvaluationSettled {
				project_id,
				account: evaluator.clone(),
				id: evaluation_to_settle.id,
				ct_amount: 0.into(),
				slashed_plmc_amount: slashed_amount,
			}
			.into(),
		);
	}

	#[benchmark]
	fn settle_successful_bid() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bids = default_bids::<T>();
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			bids,
			default_community_contributions::<T>(),
			vec![],
		);

		run_blocks_to_execute_next_transition(project_id, UpdateType::StartSettlement, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful)
		);

		let bid_to_settle =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		#[extrinsic_call]
		settle_successful_bid(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bid_to_settle.id);

		// * validity checks *
		// Storage
		assert!(Bids::<T>::get((project_id, bidder.clone(), bid_to_settle.id)).is_none());

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![bidder.clone()])[0];
		assert_eq!(bid_to_settle.final_ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidSettled { project_id, account: bidder.clone(), id: bid_to_settle.id, ct_amount }.into(),
		);
	}

	#[benchmark]
	fn settle_failed_bid() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let bidder = bids[0].bidder.clone();
		whitelist_account!(bidder);
		let contributions = inst.generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
			default_community_contributor_multipliers(),
		);

		let project_id =
			inst.create_finished_project(project_metadata, issuer.clone(), evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)
		);

		let bid_to_settle =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());
		let asset = bid_to_settle.funding_asset.to_assethub_id();
		let free_assets_before = inst.get_free_foreign_asset_balances_for(asset, vec![bidder.clone()])[0].asset_amount;
		#[extrinsic_call]
		settle_failed_bid(RawOrigin::Signed(issuer.clone()), project_id, bidder.clone(), bid_to_settle.id);

		// * validity checks *
		// Storage
		assert!(Bids::<T>::get((project_id, bidder.clone(), bid_to_settle.id)).is_none());

		// Balances
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![bidder.clone()])[0].asset_amount;
		assert_eq!(free_assets, bid_to_settle.funding_asset_amount_locked + free_assets_before);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidSettled { project_id, account: bidder.clone(), id: bid_to_settle.id, ct_amount: 0.into() }.into(),
		);
	}

	#[benchmark]
	fn settle_successful_contribution() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributions = default_community_contributions::<T>();
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			default_evaluations::<T>(),
			default_bids::<T>(),
			contributions,
			vec![],
		);

		run_blocks_to_execute_next_transition(project_id, UpdateType::StartSettlement, &mut inst);

		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful)
		);

		let contribution_to_settle =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		settle_successful_contribution(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			contribution_to_settle.id,
		);

		// * validity checks *
		// Storage
		assert!(Contributions::<T>::get((project_id, contributor.clone(), contribution_to_settle.id)).is_none());

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![contributor.clone()])[0];
		assert_eq!(contribution_to_settle.ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionSettled {
				project_id,
				account: contributor.clone(),
				id: contribution_to_settle.id,
				ct_amount,
			}
			.into(),
		);
	}

	#[benchmark]
	fn settle_failed_contribution() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let evaluations = default_evaluations::<T>();

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions: Vec<ContributionParams<T>> = inst.generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
			default_community_contributor_multipliers(),
		);
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_id =
			inst.create_finished_project(project_metadata, issuer, evaluations, bids, contributions, vec![]);

		inst.advance_time(One::one()).unwrap();
		assert_eq!(
			inst.get_project_details(project_id).status,
			ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed)
		);

		let contribution_to_settle =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		let asset = contribution_to_settle.funding_asset.to_assethub_id();
		let free_assets_before =
			inst.get_free_foreign_asset_balances_for(asset, vec![contributor.clone()])[0].asset_amount;
		#[extrinsic_call]
		settle_failed_contribution(
			RawOrigin::Signed(contributor.clone()),
			project_id,
			contributor.clone(),
			contribution_to_settle.id,
		);

		// * validity checks *
		// Storage
		assert!(Contributions::<T>::get((project_id, contributor.clone(), contribution_to_settle.id)).is_none());

		// Balances
		let free_assets = inst.get_free_foreign_asset_balances_for(asset, vec![contributor.clone()])[0].asset_amount;
		assert_eq!(free_assets, contribution_to_settle.funding_asset_amount + free_assets_before);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::ContributionSettled {
				project_id,
				account: contributor.clone(),
				id: contribution_to_settle.id,
				ct_amount: 0.into(),
			}
			.into(),
		);
	}

	//do_evaluation_end
	#[benchmark]
	fn end_evaluation_success(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());

		let evaluations = default_evaluations();
		let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.evaluate_for_users(project_id, evaluations).expect("All evaluations are accepted");

		let evaluation_end_block =
			inst.get_project_details(project_id).phase_transition_points.evaluation.end().unwrap();
		// move block manually without calling any hooks, to avoid triggering the transition outside the benchmarking context
		frame_system::Pallet::<T>::set_block_number(evaluation_end_block + One::one());

		let insertion_block_number =
			inst.current_block() + One::one() + <T as Config>::AuctionInitializePeriodDuration::get();
		fill_projects_to_update::<T>(x, insertion_block_number);

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
	fn end_evaluation_failure(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata, issuer.clone());
		let project_details = inst.get_project_details(project_id);

		let evaluation_usd_target =
			<T as Config>::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
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
		let plmc_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

		inst.mint_plmc_to(plmc_for_evaluating);

		inst.advance_time(One::one()).unwrap();
		inst.evaluate_for_users(project_id, evaluations).expect("All evaluations are accepted");

		let evaluation_end_block =
			inst.get_project_details(project_id).phase_transition_points.evaluation.end().unwrap();
		// move block manually without calling any hooks, to avoid triggering the transition outside the benchmarking context
		frame_system::Pallet::<T>::set_block_number(evaluation_end_block + One::one());

		fill_projects_to_update::<T>(x, evaluation_end_block + 2u32.into());

		// Instead of advancing in time for the automatic `do_evaluation_end` call in on_initialize, we call it directly to benchmark it
		#[block]
		{
			Pallet::<T>::do_evaluation_end(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::FundingFailed);
	}

	// do_auction_closing_auction
	#[benchmark]
	fn start_auction_closing_phase(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_auctioning_project(project_metadata, issuer.clone(), default_evaluations());

		let opening_end_block =
			inst.get_project_details(project_id).phase_transition_points.auction_opening.end().unwrap();
		// we don't use advance time to avoid triggering on_initialize. This benchmark should only measure the extrinsic
		// weight and not the whole on_initialize call weight
		frame_system::Pallet::<T>::set_block_number(opening_end_block + One::one());

		let insertion_block_number = inst.current_block() + T::AuctionClosingDuration::get() + One::one();

		fill_projects_to_update::<T>(x, insertion_block_number);

		#[block]
		{
			Pallet::<T>::do_start_auction_closing(project_id).unwrap();
		}
		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::AuctionClosing);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::AuctionClosing }.into(),
		);
	}

	#[benchmark]
	fn end_auction_closing(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Accepted Bids
		y: Linear<1, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
		// Failed Bids
		z: Linear<1, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let mut project_metadata = default_project_metadata::<T>(issuer.clone());
		project_metadata.mainnet_token_max_supply =
			BalanceOf::<T>::try_from(10_000_000 * CT_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.total_allocation_size =
			BalanceOf::<T>::try_from(10_000_000 * CT_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.auction_round_allocation_percentage = Percent::from_percent(100u8);

		let project_id = inst.create_auctioning_project(
			project_metadata.clone(),
			issuer.clone(),
			inst.generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators::<T>(),
				default_weights(),
			),
		);

		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let min_bid_amount = 500u128;
		let smaller_than_wap_accepted_bid = vec![BidParams::<T>::new(
			account::<AccountIdOf<T>>("bidder", 0, 0),
			auction_allocation,
			1u8,
			AcceptedFundingAsset::USDT,
		)];
		let higher_than_wap_accepted_bids = (1..y)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(min_bid_amount * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let accepted_bids =
			vec![smaller_than_wap_accepted_bid, higher_than_wap_accepted_bids].into_iter().flatten().collect_vec();
		let rejected_bids = (0..z)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(500 * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let all_bids = vec![accepted_bids.clone(), rejected_bids.clone()].into_iter().flatten().collect_vec();

		let plmc_needed_for_bids = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&all_bids,
			project_metadata.clone(),
			None,
			false,
		);
		let plmc_ed = all_bids.accounts().existential_deposits();
		let funding_asset_needed_for_bids = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);

		inst.mint_plmc_to(plmc_needed_for_bids);
		inst.mint_plmc_to(plmc_ed);
		inst.mint_foreign_asset_to(funding_asset_needed_for_bids);

		inst.bid_for_users(project_id, accepted_bids).unwrap();

		let transition_block = inst.get_update_block(project_id, &UpdateType::AuctionClosingStart).unwrap();
		inst.jump_to_block(transition_block);
		let auction_closing_end_block =
			inst.get_project_details(project_id).phase_transition_points.auction_closing.end().unwrap();
		// Go to the last block of closing auction, to make bids fail
		frame_system::Pallet::<T>::set_block_number(auction_closing_end_block);
		inst.bid_for_users(project_id, rejected_bids).unwrap();

		let now = inst.current_block();
		let transition_block = now + One::one();
		frame_system::Pallet::<T>::set_block_number(transition_block);

		fill_projects_to_update::<T>(x, transition_block);

		#[block]
		{
			Pallet::<T>::do_end_auction_closing(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::CalculatingWAP);
		assert!(
			stored_details.phase_transition_points.random_closing_ending.unwrap() <
				stored_details.phase_transition_points.auction_closing.end().unwrap()
		);
		let accepted_bids_count = Bids::<T>::iter_prefix_values((project_id,))
			.filter(|b| matches!(b.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.count();
		let rejected_bids_count =
			Bids::<T>::iter_prefix_values((project_id,)).filter(|b| matches!(b.status, BidStatus::Rejected(_))).count();
		assert_eq!(rejected_bids_count, 0);
		assert_eq!(accepted_bids_count, y as usize);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::CalculatingWAP }.into(),
		);
	}

	// do_community_funding
	// Should be complex due to calling `calculate_weighted_average_price`
	#[benchmark]
	fn start_community_funding(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
		// Accepted Bids
		y: Linear<1, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
		// Rejected Bids
		z: Linear<1, { <T as Config>::MaxBidsPerProject::get() / 2 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);
		let mut project_metadata = default_project_metadata::<T>(issuer.clone());

		project_metadata.mainnet_token_max_supply =
			BalanceOf::<T>::try_from(10_000_000 * CT_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.total_allocation_size =
			BalanceOf::<T>::try_from(10_000_000 * CT_UNIT).unwrap_or_else(|_| panic!("Failed to create BalanceOf"));
		project_metadata.auction_round_allocation_percentage = Percent::from_percent(100u8);

		let project_id = inst.create_auctioning_project(
			project_metadata.clone(),
			issuer.clone(),
			inst.generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators::<T>(),
				default_weights(),
			),
		);

		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let min_bid_amount = 500u128;
		let smaller_than_wap_accepted_bid = vec![BidParams::<T>::new(
			account::<AccountIdOf<T>>("bidder", 0, 0),
			auction_allocation,
			1u8,
			AcceptedFundingAsset::USDT,
		)];
		let higher_than_wap_accepted_bids = (1..y)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(min_bid_amount * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let accepted_bids =
			vec![smaller_than_wap_accepted_bid, higher_than_wap_accepted_bids].into_iter().flatten().collect_vec();

		let rejected_bids = (0..z)
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(500 * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let all_bids = vec![accepted_bids.clone(), rejected_bids.clone()].into_iter().flatten().collect_vec();

		let plmc_needed_for_bids = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&all_bids,
			project_metadata.clone(),
			None,
			false,
		);
		let plmc_ed = all_bids.accounts().existential_deposits();
		let funding_asset_needed_for_bids = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);

		inst.mint_plmc_to(plmc_needed_for_bids);
		inst.mint_plmc_to(plmc_ed);
		inst.mint_foreign_asset_to(funding_asset_needed_for_bids);

		inst.bid_for_users(project_id, accepted_bids).unwrap();

		let transition_block = inst.get_update_block(project_id, &UpdateType::AuctionClosingStart).unwrap();
		inst.jump_to_block(transition_block);
		let auction_closing_end_block =
			inst.get_project_details(project_id).phase_transition_points.auction_closing.end().unwrap();
		// Go to the last block of closing auction, to make bids fail
		frame_system::Pallet::<T>::set_block_number(auction_closing_end_block);
		inst.bid_for_users(project_id, rejected_bids).unwrap();

		let transition_block = inst.get_update_block(project_id, &UpdateType::AuctionClosingEnd).unwrap();
		inst.jump_to_block(transition_block);
		let transition_block = inst.get_update_block(project_id, &UpdateType::CommunityFundingStart).unwrap();
		// Block is at automatic transition, but it's not run with on_initialize, we do it manually
		frame_system::Pallet::<T>::set_block_number(transition_block);

		let now = inst.current_block();
		let community_end_block = now + T::CommunityFundingDuration::get() - One::one();

		let insertion_block_number = community_end_block + One::one();
		fill_projects_to_update::<T>(x, insertion_block_number);

		#[block]
		{
			Pallet::<T>::do_start_community_funding(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::CommunityRound);
		let accepted_bids_count = Bids::<T>::iter_prefix_values((project_id,)).count();
		assert_eq!(accepted_bids_count, y as usize);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::CommunityFunding }.into(),
		);
	}

	// do_remainder_funding
	#[benchmark]
	fn start_remainder_funding(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// real benchmark starts at block 0, and we can't call `events()` at block 0
		inst.advance_time(1u32.into()).unwrap();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
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

		fill_projects_to_update::<T>(x, insertion_block_number);

		#[block]
		{
			Pallet::<T>::do_start_remainder_funding(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert_eq!(stored_details.status, ProjectStatus::RemainderRound);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition { project_id, phase: ProjectPhases::RemainderFunding }.into(),
		);
	}

	// do_end_funding
	#[benchmark]
	fn end_funding_automatically_rejected_evaluators_slashed(
		// Insertion attempts in add_to_update_store. Total amount of storage items iterated through in `ProjectsToUpdate`. Leave one free to make the fn succeed
		x: Linear<1, { <T as Config>::MaxProjectsToUpdateInsertionAttempts::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let automatically_rejected_threshold = Percent::from_percent(33);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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
		fill_projects_to_update::<T>(x, insertion_block_number);

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
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let automatically_rejected_threshold = Percent::from_percent(75);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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

		let insertion_block_number = inst.current_block() + T::ManualAcceptanceDuration::get() + 1u32.into();
		fill_projects_to_update::<T>(x, insertion_block_number);

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
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let automatically_rejected_threshold = Percent::from_percent(89);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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

		let insertion_block_number = inst.current_block() + T::ManualAcceptanceDuration::get() + 1u32.into();
		fill_projects_to_update::<T>(x, insertion_block_number);

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
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let automatically_rejected_threshold = Percent::from_percent(91);

		let mut evaluations = (0..y.saturating_sub(1))
			.map(|i| {
				UserToUSDBalance::<T>::new(account::<AccountIdOf<T>>("evaluator", 0, i), (100u128 * USD_UNIT).into())
			})
			.collect_vec();

		let evaluation_target_usd = <T as Config>::EvaluationSuccessThreshold::get() * target_funding_amount;
		evaluations.push(UserToUSDBalance::<T>::new(
			account::<AccountIdOf<T>>("evaluator_success", 0, 69420),
			evaluation_target_usd,
		));

		let plmc_needed_for_evaluating = inst.calculate_evaluation_plmc_spent(evaluations.clone(), true);

		inst.mint_plmc_to(plmc_needed_for_evaluating);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			(automatically_rejected_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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

		let insertion_block_number = inst.current_block() + T::SuccessToSettlementTime::get();
		fill_projects_to_update::<T>(x, insertion_block_number);

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
	fn project_decision() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);
		let manual_outcome_threshold = Percent::from_percent(50);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);
		let contributions = inst.generate_contributions_from_total_usd(
			(manual_outcome_threshold * target_funding_amount) / 2.into(),
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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

	// do_start_settlement
	#[benchmark]
	fn start_settlement_funding_success() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata,
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		#[block]
		{
			Pallet::<T>::do_start_settlement(project_id).unwrap();
		}

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::FundingSuccessful));
	}

	#[benchmark]
	fn start_settlement_funding_failure() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let target_funding_amount: BalanceOf<T> =
			project_metadata.minimum_price.saturating_mul_int(project_metadata.total_allocation_size);

		let bids: Vec<BidParams<T>> = inst.generate_bids_from_total_usd(
			Percent::from_percent(15) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_bidders::<T>(),
			default_bidder_multipliers(),
		);

		let contributions = inst.generate_contributions_from_total_usd(
			Percent::from_percent(10) * target_funding_amount,
			project_metadata.minimum_price,
			default_weights(),
			default_community_contributors::<T>(),
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
		assert_eq!(project_details.status, ProjectStatus::SettlementStarted(FundingOutcome::FundingFailed));
	}

	#[benchmark]
	fn start_pallet_migration() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		start_pallet_migration(RawOrigin::Signed(issuer), jwt, project_id, ParaId::from(6969));

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
		assert_eq!(
			project_details.migration_type,
			Some(MigrationType::Pallet(PalletMigrationInfo {
				parachain_id: ParaId::from(6969),
				hrmp_channel_status: HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Closed,
					polimec_to_project: ChannelStatus::Closed
				},
				migration_readiness_check: None,
			}))
		)
	}

	#[benchmark]
	fn start_offchain_migration() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

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
		assert_eq!(UnmigratedCounter::<T>::get(project_id), 13);
	}

	#[benchmark]
	fn confirm_offchain_migration(
		// Amount of migrations to confirm for a single user
		x: Linear<1, { MaxParticipationsPerUser::<T>::get() }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let participant = account::<AccountIdOf<T>>("test_participant", 0, 0);

		let max_evaluations = (x / 3).min(<T as Config>::MaxEvaluationsPerUser::get());
		let max_bids = ((x - max_evaluations) / 2).min(<T as Config>::MaxBidsPerUser::get());
		let max_contributions = x - max_evaluations - max_bids;

		let participant_evaluations = (0..max_evaluations)
			.map(|_| UserToUSDBalance::new(participant.clone(), (100 * USD_UNIT).into()))
			.collect_vec();
		let participant_bids = (0..max_bids)
			.map(|_| BidParams::new(participant.clone(), (500 * CT_UNIT).into(), 1u8, AcceptedFundingAsset::USDT))
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_offchain_migration(RawOrigin::Signed(issuer.clone()).into(), jwt.clone(), project_id)
			.unwrap();

		let participant_migrations_len = UserMigrations::<T>::get((project_id, participant.clone())).unwrap().1.len();
		assert_eq!(participant_migrations_len as u32, x);

		#[extrinsic_call]
		confirm_offchain_migration(RawOrigin::Signed(issuer), project_id, participant);

		// * validity checks *
		let project_details = inst.get_project_details(project_id);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
		assert_eq!(UnmigratedCounter::<T>::get(project_id), 13);
	}

	#[benchmark]
	fn mark_project_ct_migration_as_finished() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

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

	#[benchmark]
	fn start_pallet_migration_readiness_check() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		// Mock hrmp establishment
		let mut project_details = inst.get_project_details(project_id);
		project_details.migration_type = Some(MigrationType::Pallet(PalletMigrationInfo {
			parachain_id: ParaId::from(6969),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Open,
				polimec_to_project: ChannelStatus::Open,
			},
			migration_readiness_check: Some(PalletMigrationReadinessCheck {
				holding_check: (0, CheckOutcome::Failed),
				pallet_check: (1, CheckOutcome::Failed),
			}),
		}));
		ProjectsDetails::<T>::insert(project_id, project_details);

		#[extrinsic_call]
		start_pallet_migration_readiness_check(RawOrigin::Signed(issuer.clone()), jwt, project_id);

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationReadinessCheckStarted { project_id, caller: issuer.clone() }.into(),
		);
	}

	#[benchmark]
	fn pallet_migration_readiness_response_holding() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		// Mock hrmp establishment
		let mut project_details = inst.get_project_details(project_id);
		project_details.migration_type = Some(MigrationType::Pallet(PalletMigrationInfo {
			parachain_id: ParaId::from(6969),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Open,
				polimec_to_project: ChannelStatus::Open,
			},
			migration_readiness_check: None,
		}));
		ProjectsDetails::<T>::insert(project_id, project_details);

		// Create query id's
		crate::Pallet::<T>::do_start_pallet_migration_readiness_check(
			&T::PalletId::get().into_account_truncating(),
			project_id,
		)
		.unwrap();

		let ct_issuance: u128 = <T as crate::Config>::ContributionTokenCurrency::total_issuance(project_id).into();
		let xcm_response = Response::Assets(
			vec![MultiAsset { id: Concrete(MultiLocation::new(1, X1(Parachain(6969)))), fun: Fungible(ct_issuance) }]
				.into(),
		);

		#[block]
		{
			// We call the inner function directly to avoid having to hardcode a benchmark pallet_xcm origin as a config type
			crate::Pallet::<T>::do_pallet_migration_readiness_response(
				MultiLocation::new(1, X1(Parachain(6969))),
				0,
				xcm_response.clone(),
			)
			.unwrap();
		}

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationCheckResponseAccepted { project_id, query_id: 0, response: xcm_response }.into(),
		);
	}

	#[benchmark]
	fn pallet_migration_readiness_response_pallet_info() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		// Mock hrmp establishment
		let mut project_details = inst.get_project_details(project_id);
		project_details.migration_type = Some(MigrationType::Pallet(PalletMigrationInfo {
			parachain_id: ParaId::from(6969),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Open,
				polimec_to_project: ChannelStatus::Open,
			},
			migration_readiness_check: None,
		}));
		ProjectsDetails::<T>::insert(project_id, project_details);

		// Create query id's
		crate::Pallet::<T>::do_start_pallet_migration_readiness_check(
			&T::PalletId::get().into_account_truncating(),
			project_id,
		)
		.unwrap();

		let module_name: BoundedVec<u8, MaxPalletNameLen> =
			BoundedVec::try_from("polimec_receiver".as_bytes().to_vec()).unwrap();
		let pallet_info = xcm::latest::PalletInfo {
			// index is used for future `Transact` calls to the pallet for migrating a user
			index: 69,
			// Doesn't matter
			name: module_name.clone(),
			// Main check that the receiver pallet is there
			module_name,
			// These might be useful in the future, but not for now
			major: 0,
			minor: 0,
			patch: 0,
		};
		let xcm_response = Response::PalletsInfo(vec![pallet_info].try_into().unwrap());

		#[block]
		{
			// We call the inner function directly to avoid having to hardcode a benchmark pallet_xcm origin as a config type
			crate::Pallet::<T>::do_pallet_migration_readiness_response(
				MultiLocation::new(1, X1(Parachain(6969))),
				1,
				xcm_response.clone(),
			)
			.unwrap();
		}

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationCheckResponseAccepted { project_id, query_id: 1, response: xcm_response }.into(),
		);
	}

	#[benchmark]
	fn send_pallet_migration_for(
		// Amount of migrations to confirm for a single user
		x: Linear<1, { MaxParticipationsPerUser::<T>::get() }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let participant = account::<AccountIdOf<T>>("test_participant", 0, 0);

		let max_evaluations = (x / 3).min(<T as Config>::MaxEvaluationsPerUser::get());
		let max_bids = ((x - max_evaluations) / 2).min(<T as Config>::MaxBidsPerUser::get());
		let max_contributions = x - max_evaluations - max_bids;

		let participant_evaluations = (0..max_evaluations)
			.map(|_| UserToUSDBalance::new(participant.clone(), (100 * USD_UNIT).into()))
			.collect_vec();
		let participant_bids = (0..max_bids)
			.map(|_| BidParams::new(participant.clone(), (500 * CT_UNIT).into(), 1u8, AcceptedFundingAsset::USDT))
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		// Mock hrmp establishment
		let mut project_details = inst.get_project_details(project_id);
		project_details.migration_type = Some(MigrationType::Pallet(PalletMigrationInfo {
			parachain_id: ParaId::from(6969),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Open,
				polimec_to_project: ChannelStatus::Open,
			},
			migration_readiness_check: Some(PalletMigrationReadinessCheck {
				holding_check: (0, CheckOutcome::Passed(None)),
				pallet_check: (1, CheckOutcome::Passed(Some(42))),
			}),
		}));
		ProjectsDetails::<T>::insert(project_id, project_details);

		#[extrinsic_call]
		send_pallet_migration_for(RawOrigin::Signed(issuer), project_id, participant.clone());

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationStatusUpdated { project_id, account: participant, status: MigrationStatus::Sent(0) }
				.into(),
		);
	}

	#[benchmark]
	fn confirm_pallet_migrations(
		// Amount of migrations to confirm for a single user
		x: Linear<1, { MaxParticipationsPerUser::<T>::get() }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let participant = account::<AccountIdOf<T>>("test_participant", 0, 0);

		let max_evaluations = (x / 3).min(<T as Config>::MaxEvaluationsPerUser::get());
		let max_bids = ((x - max_evaluations) / 2).min(<T as Config>::MaxBidsPerUser::get());
		let max_contributions = x - max_evaluations - max_bids;

		let participant_evaluations = (0..max_evaluations)
			.map(|_| UserToUSDBalance::new(participant.clone(), (100 * USD_UNIT).into()))
			.collect_vec();
		let participant_bids = (0..max_bids)
			.map(|_| BidParams::new(participant.clone(), (500 * CT_UNIT).into(), 1u8, AcceptedFundingAsset::USDT))
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					1u8,
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer.clone(),
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
		);

		let settlement_block = inst.get_update_block(project_id, &UpdateType::StartSettlement).unwrap();
		inst.jump_to_block(settlement_block);

		inst.settle_project(project_id).unwrap();

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		crate::Pallet::<T>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		// Mock hrmp establishment
		let mut project_details = inst.get_project_details(project_id);
		project_details.migration_type = Some(MigrationType::Pallet(PalletMigrationInfo {
			parachain_id: ParaId::from(6969),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Open,
				polimec_to_project: ChannelStatus::Open,
			},
			migration_readiness_check: Some(PalletMigrationReadinessCheck {
				holding_check: (0, CheckOutcome::Passed(None)),
				pallet_check: (1, CheckOutcome::Passed(Some(42))),
			}),
		}));
		ProjectsDetails::<T>::insert(project_id, project_details);

		crate::Pallet::<T>::send_pallet_migration_for(
			RawOrigin::Signed(issuer).into(),
			project_id,
			participant.clone(),
		)
		.unwrap();

		let project_location = MultiLocation::new(1, X1(Parachain(6969)));
		let xcm_response = Response::DispatchResult(MaybeErrorCode::Success);

		#[block]
		{
			crate::Pallet::<T>::do_confirm_pallet_migrations(project_location, 0, xcm_response).unwrap();
		}

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationStatusUpdated { project_id, account: participant, status: MigrationStatus::Confirmed }
				.into(),
		);
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::mock::{new_test_ext, TestRuntime};

		#[test]
		fn bench_create_project() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_create_project());
			});
		}

		#[test]
		fn bench_remove_project() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_remove_project());
			});
		}

		#[test]
		fn bench_edit_project() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_edit_project());
			});
		}

		#[test]
		fn bench_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_evaluation());
			});
		}

		#[test]
		fn bench_start_auction_manually() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_auction_manually());
			});
		}

		#[test]
		fn bench_bid() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_bid());
			});
		}

		#[test]
		fn bench_contribution() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_contribution());
			});
		}

		#[test]
		fn bench_contribution_ends_round() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_contribution_ends_round());
			});
		}

		#[test]
		fn bench_decide_project_outcome() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_decide_project_outcome());
			});
		}

		#[test]
		fn bench_settle_successful_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_successful_evaluation());
			});
		}

		#[test]
		fn bench_settle_failed_evaluation() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_failed_evaluation());
			});
		}

		#[test]
		fn bench_settle_successful_bid() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_successful_bid());
			});
		}

		#[test]
		fn bench_settle_failed_bid() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_failed_bid());
			});
		}

		#[test]
		fn bench_settle_successful_contribution() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_successful_contribution());
			});
		}

		#[test]
		fn bench_settle_failed_contribution() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_settle_failed_contribution());
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
		fn bench_start_auction_closing_phase() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_auction_closing_phase());
			});
		}

		#[test]
		fn bench_end_auction_closing() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_auction_closing());
			});
		}

		#[test]
		fn bench_start_community_funding() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_community_funding());
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
		fn bench_project_decision() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_project_decision());
			});
		}

		#[test]
		fn bench_end_funding_automatically_rejected_evaluators_slashed() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_automatically_rejected_evaluators_slashed());
			});
		}

		#[test]
		fn bench_end_funding_automatically_accepted_evaluators_rewarded() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_end_funding_automatically_accepted_evaluators_rewarded());
			});
		}

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

		#[test]
		fn bench_start_pallet_migration() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_pallet_migration());
			});
		}

		#[test]
		fn bench_start_offchain_migration() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_offchain_migration());
			});
		}

		#[test]
		fn bench_confirm_offchain_migration() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_confirm_offchain_migration());
			});
		}

		#[test]
		fn bench_mark_project_ct_migration_as_finished() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_mark_project_ct_migration_as_finished());
			});
		}

		#[test]
		fn bench_start_pallet_migration_readiness_check() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_start_pallet_migration_readiness_check());
			});
		}

		#[test]
		fn bench_pallet_migration_readiness_response_holding() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_pallet_migration_readiness_response_holding());
			});
		}

		#[test]
		fn bench_pallet_migration_readiness_response_pallet_info() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_pallet_migration_readiness_response_pallet_info());
			});
		}

		#[test]
		fn bench_send_pallet_migration_for() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_send_pallet_migration_for());
			});
		}

		#[test]
		fn bench_confirm_pallet_migrations() {
			new_test_ext().execute_with(|| {
				assert_ok!(PalletFunding::<TestRuntime>::test_confirm_pallet_migrations());
			});
		}
	}
}
