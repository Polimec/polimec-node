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
use ParticipationMode::{Classic, OTM};

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
use polimec_common::{credentials::InvestorType, ProvideAssetPrice, ReleaseSchedule, USD_DECIMALS, USD_UNIT};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{Get, Member, TrailingZeroInput, Zero};
use xcm::v4::MaxPalletNameLen;

const IPFS_CID: &str = "QmbvsJBhQtu9uAGVp7x4H77JkwAQxV7TA6xTfdeALuDiYB";
const CT_DECIMALS: u8 = 17;
const CT_UNIT: u128 = 10u128.pow(CT_DECIMALS as u32);
type BenchInstantiator<T> = Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;

pub fn usdt_id() -> u32 {
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
		total_allocation_size: 1_000_000u128 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(10u128.into(), USD_DECIMALS, CT_DECIMALS)
			.unwrap(),

		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(5000u128 * USD_UNIT, None),
			institutional: TicketSize::new(5000u128 * USD_UNIT, None),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(USD_UNIT, None),
			professional: TicketSize::new(USD_UNIT, None),
			institutional: TicketSize::new(USD_UNIT, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: issuer,
		policy_ipfs_cid: Some(metadata_hash.into()),
		participants_account_type: ParticipantsAccountType::Polkadot,
	}
}

pub fn default_evaluations<T: Config>() -> Vec<UserToUSDBalance<T>>
where
	<T as Config>::Price: From<u128>,
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
		default_bidder_modes(),
	)
}

pub fn full_bids<T>() -> Vec<BidParams<T>>
where
	T: Config,
	<T as Config>::Price: From<u128>,
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
		default_bidder_modes(),
	)
}

pub fn default_community_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
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
		default_community_contributor_modes(),
	)
}

pub fn default_remainder_contributions<T: Config>() -> Vec<ContributionParams<T>>
where
	<T as Config>::Price: From<u128>,
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
		default_remainder_contributor_modes(),
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

pub fn default_bidder_modes() -> Vec<ParticipationMode> {
	vec![Classic(10u8), Classic(3u8), OTM, OTM, Classic(4u8)]
}
pub fn default_community_contributor_modes() -> Vec<ParticipationMode> {
	vec![Classic(2u8), Classic(1u8), Classic(3u8), OTM, OTM]
}
pub fn default_remainder_contributor_modes() -> Vec<ParticipationMode> {
	vec![Classic(1u8), OTM, Classic(1u8), OTM, Classic(1u8)]
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
			total_allocation_size: 200_000u128 * CT_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(30u8),
			minimum_price: PriceProviderOf::<T>::calculate_decimals_aware_price(
				11u128.into(),
				USD_DECIMALS,
				CT_DECIMALS,
			)
			.unwrap(),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				institutional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				phantom: Default::default(),
			},
			contributing_ticket_sizes: ContributingTicketSizes {
				retail: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				professional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
				institutional: TicketSize::new(5000 * USD_UNIT, Some(10_000 * USD_UNIT)),
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
	fn evaluate(
		// How many other evaluations the user did for that same project
		x: Linear<0, { T::MaxEvaluationsPerUser::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let test_evaluator = account::<AccountIdOf<T>>("evaluator", 0, 0);
		whitelist_account!(test_evaluator);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_evaluating_project(project_metadata.clone(), issuer, None);

		let existing_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (200 * USD_UNIT).into());
		let extrinsic_evaluation = UserToUSDBalance::new(test_evaluator.clone(), (1_000 * USD_UNIT).into());
		let existing_evaluations = vec![existing_evaluation; x as usize];

		let plmc_for_existing_evaluations = inst.calculate_evaluation_plmc_spent(existing_evaluations.clone());
		let plmc_for_extrinsic_evaluation = inst.calculate_evaluation_plmc_spent(vec![extrinsic_evaluation.clone()]);
		let existential_plmc: Vec<UserToPLMCBalance<T>> =
			plmc_for_extrinsic_evaluation.accounts().existential_deposits();

		inst.mint_plmc_to(existential_plmc);
		inst.mint_plmc_to(plmc_for_existing_evaluations.clone());
		inst.mint_plmc_to(plmc_for_extrinsic_evaluation.clone());

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
		let bonded_plmc = inst
			.get_reserved_plmc_balances_for(vec![extrinsic_evaluation.account.clone()], HoldReason::Evaluation.into())[0]
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
		// amount of already made bids by the same user. Leave 10 bids available to make the extrinsic pass in case y = max (10)
		x: Linear<0, { T::MaxBidsPerUser::get() - 10 }>,
		// amount of times when `perform_bid` is called (i.e. into how many buckets the bid is spread)
		y: Linear<0, 10>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
		whitelist_account!(bidder);

		let mut project_metadata = default_project_metadata::<T>(issuer.clone());
		project_metadata.mainnet_token_max_supply = 100_000 * CT_UNIT;
		project_metadata.total_allocation_size = 100_000 * CT_UNIT;
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

		let project_id = inst.create_auctioning_project(project_metadata.clone(), issuer, None, evaluations);

		let existing_bid = BidParams::new(
			bidder.clone(),
			(50 * CT_UNIT).into(),
			ParticipationMode::Classic(5u8),
			AcceptedFundingAsset::USDT,
		);

		let existing_bids = vec![existing_bid; x as usize];
		let existing_bids_post_bucketing =
			inst.get_actual_price_charged_for_bucketed_bids(&existing_bids, project_metadata.clone(), None);
		let plmc_for_existing_bids = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&existing_bids,
			project_metadata.clone(),
			None,
		);

		let usdt_for_existing_bids: Vec<UserToFundingAsset<T>> = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&existing_bids,
				project_metadata.clone(),
				None,
			);
		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_escrow_usdt_locked =
			inst.get_free_funding_asset_balances_for(vec![(escrow_account.clone(), usdt_id())]);

		inst.mint_plmc_ed_if_required(plmc_for_existing_bids.accounts());
		inst.mint_plmc_to(plmc_for_existing_bids.clone());
		inst.mint_funding_asset_ed_if_required(usdt_for_existing_bids.to_account_asset_map());
		inst.mint_funding_asset_to(usdt_for_existing_bids.clone());

		// do "x" contributions for this user
		inst.bid_for_users(project_id, existing_bids.clone()).unwrap();

		// to call do_perform_bid several times, we need the bucket to reach its limit. You can only bid over 10 buckets
		// in a single bid, since the increase delta is 10% of the total allocation, and you cannot bid more than the allocation.
		let mut ct_amount = (50 * CT_UNIT).into();
		let mut maybe_filler_bid = None;
		let new_bidder = account::<AccountIdOf<T>>("new_bidder", 0, 0);

		let mut usdt_for_filler_bidder =
			vec![UserToFundingAsset::<T>::new(new_bidder.clone(), Zero::zero(), AcceptedFundingAsset::USDT.id())];
		if y > 0 {
			let current_bucket = Buckets::<T>::get(project_id).unwrap();
			// first lets bring the bucket to almost its limit with another bidder:
			assert!(new_bidder.clone() != bidder.clone());
			let bid_params = BidParams::new(
				new_bidder.clone(),
				current_bucket.amount_left,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			maybe_filler_bid = Some(bid_params.clone());
			let plmc_for_new_bidder = inst.calculate_auction_plmc_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
			);
			let usdt_for_new_bidder = inst.calculate_auction_funding_asset_charged_with_given_price(
				&vec![bid_params.clone()],
				current_bucket.current_price,
			);

			inst.mint_plmc_ed_if_required(vec![(new_bidder.clone())]);
			inst.mint_plmc_to(plmc_for_new_bidder);

			inst.mint_funding_asset_ed_if_required(vec![(new_bidder, AcceptedFundingAsset::USDT.id())]);
			inst.mint_funding_asset_to(usdt_for_new_bidder.clone());

			inst.bid_for_users(project_id, vec![bid_params]).unwrap();

			let auction_allocation =
				project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
			let bucket_size = Percent::from_percent(10) * auction_allocation;
			ct_amount = bucket_size * (y as u128);
			usdt_for_filler_bidder = usdt_for_new_bidder;
		}
		let extrinsic_bid =
			BidParams::new(bidder.clone(), ct_amount, ParticipationMode::Classic(1u8), AcceptedFundingAsset::USDT);
		let original_extrinsic_bid = extrinsic_bid.clone();
		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		// we need to call this after bidding `x` amount of times, to get the latest bucket from storage
		let extrinsic_bids_post_bucketing = inst.get_actual_price_charged_for_bucketed_bids(
			&vec![extrinsic_bid.clone()],
			project_metadata.clone(),
			Some(current_bucket),
		);

		assert_eq!(extrinsic_bids_post_bucketing.len(), (y as usize).max(1usize));

		let plmc_for_extrinsic_bids: Vec<UserToPLMCBalance<T>> = inst
			.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid.clone()],
				project_metadata.clone(),
				Some(current_bucket),
			);
		let usdt_for_extrinsic_bids: Vec<UserToFundingAsset<T>> = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&vec![extrinsic_bid],
				project_metadata.clone(),
				Some(current_bucket),
			);
		inst.mint_plmc_ed_if_required(plmc_for_extrinsic_bids.accounts());
		inst.mint_plmc_to(plmc_for_extrinsic_bids.clone());
		inst.mint_funding_asset_ed_if_required(usdt_for_extrinsic_bids.to_account_asset_map());
		inst.mint_funding_asset_to(usdt_for_extrinsic_bids.clone());

		let total_free_plmc = inst.get_ed();
		let total_plmc_participation_bonded =
			inst.sum_balance_mappings(vec![plmc_for_extrinsic_bids.clone(), plmc_for_existing_bids.clone()]);
		let total_free_usdt = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
		let total_escrow_usdt_locked = inst.sum_funding_asset_mappings(vec![
			prev_total_escrow_usdt_locked.clone(),
			usdt_for_extrinsic_bids.clone(),
			usdt_for_existing_bids.clone(),
			usdt_for_filler_bidder.clone(),
		])[0]
			.1;

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
			original_extrinsic_bid.mode,
			original_extrinsic_bid.asset,
		);

		// * validity checks *

		// Storage
		for (bid_params, price) in extrinsic_bids_post_bucketing.clone() {
			let bid_filter = BidInfoFilter::<T> {
				id: None,
				project_id: Some(project_id),
				bidder: Some(bidder.clone()),
				status: Some(BidStatus::YetUnknown),
				original_ct_amount: Some(bid_params.amount),
				original_ct_usd_price: Some(price),
				funding_asset: Some(AcceptedFundingAsset::USDT),
				funding_asset_amount_locked: None,
				mode: Some(bid_params.mode),
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

		let mut expected_bucket = Bucket::new(
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			project_metadata.minimum_price,
			ten_percent_in_price,
			bucket_delta_amount,
		);

		for (bid_params, _price_) in existing_bids_post_bucketing.clone() {
			expected_bucket.update(bid_params.amount);
		}
		if let Some(bid_params) = maybe_filler_bid {
			expected_bucket.update(bid_params.amount);
		}
		for (bid_params, _price_) in extrinsic_bids_post_bucketing.clone() {
			expected_bucket.update(bid_params.amount);
		}

		let current_bucket = Buckets::<T>::get(project_id).unwrap();
		assert_eq!(current_bucket, expected_bucket);

		// Balances
		let bonded_plmc =
			inst.get_reserved_plmc_balances_for(vec![bidder.clone()], HoldReason::Participation.into())[0].plmc_amount;
		assert_eq!(bonded_plmc, total_plmc_participation_bonded);

		let free_plmc = inst.get_free_plmc_balances_for(vec![bidder.clone()])[0].plmc_amount;
		assert_eq!(free_plmc, total_free_plmc);

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let locked_usdt = inst.get_free_funding_asset_balance_for(usdt_id(), escrow_account.clone());
		assert_eq!(locked_usdt, total_escrow_usdt_locked);

		let free_usdt = inst.get_free_funding_asset_balance_for(usdt_id(), bidder);
		assert_eq!(free_usdt, total_free_usdt);

		// Events
		for (bid_params, _price_) in extrinsic_bids_post_bucketing {
			let maybe_event = find_event! {
				T,
				Event::<T>::Bid {
					project_id,
					ct_amount,
					mode, ..
				},
				project_id == project_id,
				ct_amount == bid_params.amount,
				mode == bid_params.mode
			};
			assert!(maybe_event.is_some(), "Event not found");
		}
	}

	#[benchmark]
	fn end_auction(
		// Accepted Bids
		x: Linear<10, { 25 }>,
		// Failed Bids
		y: Linear<0, { 8 }>,
	) {
		// * setup *
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();
		// We can't see events at block 0
		inst.jump_to_block(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(issuer);

		let mut project_metadata = default_project_metadata::<T>(issuer.clone());
		project_metadata.mainnet_token_max_supply = 10_000_000 * CT_UNIT;
		project_metadata.total_allocation_size = 10_000_000 * CT_UNIT;
		project_metadata.auction_round_allocation_percentage = Percent::from_percent(100u8);

		let project_id = inst.create_auctioning_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			inst.generate_successful_evaluations(
				project_metadata.clone(),
				default_evaluators::<T>(),
				default_weights(),
			),
		);
		let expected_remainder_round_block = inst.remainder_round_block() - One::one();

		let mut all_bids = Vec::new();

		let auction_allocation =
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;
		let min_bid_amount = 500u128;

		// These bids will always be rejected, and will be made after the first bucket bid
		let rejected_bids = (0..y.saturating_sub(1))
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(min_bid_amount * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		all_bids.extend(rejected_bids.clone());

		let already_accepted_bids_count = if y > 0 {
			// This one needs to fill the remaining with the bucket, so that all "accepted" bids will take the CT from a rejected one
			let last_rejected_bid = BidParams::<T>::new(
				account::<AccountIdOf<T>>("bidder", 0, 420),
				auction_allocation - (min_bid_amount * CT_UNIT * (y as u128 - 1u128)),
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			all_bids.push(last_rejected_bid.clone());

			// We first need to invalidate all rejected bids.
			// We do it by placing a bid of  the whole auction allocation, i.e. 10 new bids
			let allocation_bid = BidParams::<T>::new(
				account::<AccountIdOf<T>>("bidder", 0, y),
				auction_allocation,
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT,
			);
			all_bids.push(allocation_bid);

			10
		} else {
			0
		};

		let accepted_bids = (0..x.saturating_sub(already_accepted_bids_count))
			.map(|i| {
				BidParams::<T>::new(
					account::<AccountIdOf<T>>("bidder", 0, i),
					(min_bid_amount * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		all_bids.extend(accepted_bids.clone());

		let plmc_needed_for_bids = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
			&all_bids,
			project_metadata.clone(),
			None,
		);
		let funding_asset_needed_for_bids = inst
			.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
				&all_bids,
				project_metadata.clone(),
				None,
			);
		inst.mint_plmc_ed_if_required(plmc_needed_for_bids.accounts());
		inst.mint_plmc_to(plmc_needed_for_bids);
		inst.mint_funding_asset_ed_if_required(funding_asset_needed_for_bids.to_account_asset_map());
		inst.mint_funding_asset_to(funding_asset_needed_for_bids);

		inst.bid_for_users(project_id, all_bids).unwrap();

		let auction_end = inst.get_project_details(project_id).round_duration.end().unwrap();
		inst.jump_to_block(auction_end);

		#[block]
		{
			Pallet::<T>::do_end_auction(project_id).unwrap();
		}

		// * validity checks *
		// Storage
		let stored_details = ProjectsDetails::<T>::get(project_id).unwrap();
		assert!(matches!(stored_details.status, ProjectStatus::CommunityRound(..)));

		let accepted_bids_count = Bids::<T>::iter_prefix_values((project_id,))
			.filter(|b| matches!(b.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
			.count();
		let rejected_bids_count =
			Bids::<T>::iter_prefix_values((project_id,)).filter(|b| matches!(b.status, BidStatus::Rejected)).count();
		assert_eq!(accepted_bids_count, x as usize);
		assert_eq!(rejected_bids_count, y as usize);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::ProjectPhaseTransition {
				project_id,
				phase: ProjectStatus::CommunityRound(expected_remainder_round_block),
			}
			.into(),
		);
	}

	// We check if the user has a winning bid regardless if its the community or remainder round, so both rounds should have
	// the same weight with `x` being equal.
	#[benchmark]
	fn contribute(
		// How many other contributions the user did for that same project
		x: Linear<0, { T::MaxContributionsPerUser::get() - 1 }>,
	) {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.jump_to_block(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributor = account::<AccountIdOf<T>>("contributor", 0, 0);
		whitelist_account!(contributor);

		let project_metadata = default_project_metadata::<T>(issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer,
			None,
			default_evaluations::<T>(),
			full_bids::<T>(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let contributions = vec![
			ContributionParams::new(
				contributor.clone(),
				(50 * CT_UNIT).into(),
				ParticipationMode::Classic(1u8),
				AcceptedFundingAsset::USDT
			);
			x as usize + 1
		];

		let plmc = inst.calculate_contributed_plmc_spent(contributions.clone(), price);
		let usdt = inst.calculate_contributed_funding_asset_spent(contributions.clone(), price);

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let prev_total_usdt_locked =
			inst.get_free_funding_asset_balances_for(vec![(escrow_account.clone(), usdt_id())]);

		inst.mint_plmc_ed_if_required(plmc.accounts());
		inst.mint_plmc_to(plmc.clone());
		inst.mint_funding_asset_ed_if_required(usdt.to_account_asset_map());
		inst.mint_funding_asset_to(usdt.clone());

		// do "x" contributions for this user
		inst.contribute_for_users(project_id, contributions[1..].to_vec()).expect("All contributions are accepted");

		let total_plmc_bonded = inst.sum_balance_mappings(vec![plmc.clone()]);
		let total_usdt_locked = inst.sum_funding_asset_mappings(vec![prev_total_usdt_locked, usdt.clone()])[0].1;

		let total_free_plmc = inst.get_ed();
		let total_free_usdt = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());

		let jwt = get_mock_jwt_with_cid(
			contributor.clone(),
			InvestorType::Retail,
			generate_did_from_account(contributor.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		#[extrinsic_call]
		contribute(
			RawOrigin::Signed(contributor.clone()),
			jwt,
			project_id,
			contributions[0].amount,
			contributions[0].mode,
			contributions[0].asset,
		);

		// * validity checks *
		// Storage
		let stored_contributions =
			Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).collect_vec();
		assert_eq!(stored_contributions.len(), x as usize + 1);

		// Balances
		let bonded_plmc = inst.get_reserved_plmc_balance_for(contributor.clone(), HoldReason::Participation.into());
		assert_eq!(bonded_plmc, total_plmc_bonded);

		let free_plmc = inst.get_free_plmc_balance_for(contributor.clone());
		assert_eq!(free_plmc, total_free_plmc);

		let escrow_account = Pallet::<T>::fund_account_id(project_id);
		let locked_usdt = inst.get_free_funding_asset_balance_for(usdt_id(), escrow_account.clone());
		assert_eq!(locked_usdt, total_usdt_locked);

		let free_usdt = inst.get_free_funding_asset_balance_for(usdt_id(), contributor.clone());
		assert_eq!(free_usdt, total_free_usdt);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::Contribution {
				project_id,
				contributor,
				id: x,
				ct_amount: contributions[0].amount,
				funding_asset: AcceptedFundingAsset::USDT,
				funding_amount: usdt[0].asset_amount,
				plmc_bond: plmc[0].plmc_amount,
				mode: contributions[0].mode,
			}
			.into(),
		);
	}

	// end_funding has 2 logic paths:
	// 1 - Funding successful (most expensive, not by much)
	// 2 - Funding failed
	// They only differ in that 1- has to calculate the evaluator rewards.
	#[benchmark]
	fn end_funding_project_successful() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.jump_to_block(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());

		let project_id = inst.create_remainder_contributing_project(
			project_metadata,
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
		);

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
		inst.jump_to_block(1u32.into());

		let anyone = account::<AccountIdOf<T>>("anyone", 0, 0);
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		whitelist_account!(anyone);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
		);

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
		let evaluations: Vec<UserToUSDBalance<T>> = default_evaluations::<T>();
		let evaluator: AccountIdOf<T> = evaluations[0].account.clone();
		whitelist_account!(evaluator);

		let project_id = inst.create_finished_project(
			default_project_metadata::<T>(issuer.clone()),
			issuer,
			None,
			evaluations,
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
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
	// 1 - Accepted bid with no refunds (i.e. final price <= WAP, no partial acceptance)
	// 2 - Accepted bid with refund (i.e. final price > WAP or partial acceptance)
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
		let mut bidder_accounts = default_bidders::<T>().into_iter();

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		// let target_wap = project_metadata.minimum_price + project_metadata.minimum_price * <PriceOf<T>>::saturating_from_rational(1, 10);
		let mut target_bucket = <Pallet<T>>::create_bucket_from_metadata(&project_metadata.clone()).unwrap();
		target_bucket.update(target_bucket.amount_left);
		target_bucket.update(target_bucket.amount_left);

		let bids = inst.generate_bids_from_bucket(
			project_metadata.clone(),
			target_bucket,
			bidder_accounts.next().unwrap(),
			|_| bidder_accounts.next().unwrap(),
			AcceptedFundingAsset::USDT,
		);

		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer,
			None,
			default_evaluations::<T>(),
			bids.clone(),
			default_community_contributions::<T>(),
			vec![],
		);

		let wap = inst.get_project_details(project_id).weighted_average_price.unwrap();

		let bidder = bids.last().unwrap().bidder.clone();
		whitelist_account!(bidder);

		assert_ok!(<Pallet<T>>::do_start_settlement(project_id));

		let bid_to_settle =
			inst.execute(|| Bids::<T>::iter_prefix_values((project_id, bidder.clone())).next().unwrap());

		// Make sure a refund has to happen
		assert!(bid_to_settle.original_ct_usd_price > wap);

		#[extrinsic_call]
		settle_bid(RawOrigin::Signed(bidder.clone()), project_id, bidder.clone(), bid_to_settle.id);

		// * validity checks *
		// Storage
		assert!(Bids::<T>::get((project_id, bidder.clone(), bid_to_settle.id)).is_none());

		// Balances
		let ct_amount = inst.get_ct_asset_balances_for(project_id, vec![bidder.clone()])[0];
		assert_eq!(bid_to_settle.original_ct_amount, ct_amount);

		// Events
		frame_system::Pallet::<T>::assert_last_event(
			Event::BidSettled {
				project_id,
				account: bidder.clone(),
				id: bid_to_settle.id,
				final_ct_amount: bid_to_settle.original_ct_amount,
				final_ct_usd_price: wap,
			}
			.into(),
		);
	}

	// We have 2 logic paths
	// 1 - Project was successful, USDT is transferred to issuer, CT minted, PLMC locked for vesting
	// 2 - Project failed, USDT is refunded to contributor, CT is not minted, PLMC is released
	// Path 1 is the most expensive but not by far, so we only benchmark and charge for this weight
	#[benchmark]
	fn settle_contribution_project_successful() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		// We can't see events at block 0
		inst.advance_time(1u32.into());

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let contributions = default_community_contributions::<T>();
		let contributor = contributions[0].contributor.clone();
		whitelist_account!(contributor);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_finished_project(
			project_metadata.clone(),
			issuer,
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			contributions,
			vec![],
		);

		assert_ok!(<Pallet<T>>::do_start_settlement(project_id));

		let contribution_to_settle =
			inst.execute(|| Contributions::<T>::iter_prefix_values((project_id, contributor.clone())).next().unwrap());

		#[extrinsic_call]
		settle_contribution(
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
		inst.assert_plmc_held_balance(
			contributor.clone(),
			contribution_to_settle.plmc_bond,
			HoldReason::Participation.into(),
		);
		assert_eq!(
			VestingOf::<T>::total_scheduled_amount(&contributor, HoldReason::Participation.into()),
			Some(contribution_to_settle.plmc_bond)
		);
		let funding_account = project_metadata.funding_destination_account;
		inst.assert_funding_asset_free_balance(
			funding_account,
			AcceptedFundingAsset::USDT.id(),
			contribution_to_settle.funding_asset_amount,
		);

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
	fn mark_project_as_settled() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		let anyone = account::<AccountIdOf<T>>("anyone", 0, 0);
		whitelist_account!(anyone);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			false,
		);

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
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

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
			.map(|_| {
				BidParams::new(
					participant.clone(),
					(500 * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					ParticipationMode::Classic(1),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_offchain_migration(RawOrigin::Signed(issuer.clone()).into(), jwt.clone(), project_id)
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
	fn start_pallet_migration() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

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
	fn start_pallet_migration_readiness_check() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

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
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_pallet_migration(
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
		<Pallet<T>>::do_start_pallet_migration_readiness_check(
			&T::PalletId::get().into_account_truncating(),
			project_id,
		)
		.unwrap();

		let ct_issuance: u128 = <T as crate::Config>::ContributionTokenCurrency::total_issuance(project_id).into();
		let xcm_response = Response::Assets(
			vec![Asset { id: AssetId(Location::new(1, [Parachain(6969)])), fun: Fungible(ct_issuance) }].into(),
		);

		#[block]
		{
			// We call the inner function directly to avoid having to hardcode a benchmark pallet_xcm origin as a config type
			crate::Pallet::<T>::do_pallet_migration_readiness_response(
				Location::new(1, [Parachain(6969)]),
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
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_pallet_migration(
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
		let pallet_info = PalletInfo::new(
			// index is used for future `Transact` calls to the pallet for migrating a user
			69,
			// Doesn't matter
			module_name.to_vec(),
			// Main check that the receiver pallet is there
			module_name.to_vec(),
			// These might be useful in the future, but not for now
			0,
			0,
			0,
		)
		.unwrap();
		let xcm_response = Response::PalletsInfo(vec![pallet_info].try_into().unwrap());

		#[block]
		{
			// We call the inner function directly to avoid having to hardcode a benchmark pallet_xcm origin as a config type
			crate::Pallet::<T>::do_pallet_migration_readiness_response(
				Location::new(1, [Parachain(6969)]),
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
			.map(|_| {
				BidParams::new(
					participant.clone(),
					(500 * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					ParticipationMode::Classic(1),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_pallet_migration(
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
			.map(|_| {
				BidParams::new(
					participant.clone(),
					(500 * CT_UNIT).into(),
					ParticipationMode::Classic(1u8),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();
		let participant_contributions = (0..max_contributions)
			.map(|_| {
				ContributionParams::<T>::new(
					participant.clone(),
					(10 * CT_UNIT).into(),
					ParticipationMode::Classic(1),
					AcceptedFundingAsset::USDT,
				)
			})
			.collect_vec();

		let mut evaluations = default_evaluations::<T>();
		evaluations.extend(participant_evaluations);

		let mut bids = default_bids::<T>();
		bids.extend(participant_bids);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			evaluations,
			bids,
			default_community_contributions::<T>(),
			participant_contributions,
			true,
		);

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

		<Pallet<T>>::send_pallet_migration_for(RawOrigin::Signed(issuer).into(), project_id, participant.clone())
			.unwrap();

		let project_location = Location::new(1, [Parachain(6969)]);
		let xcm_response = Response::DispatchResult(MaybeErrorCode::Success);

		#[block]
		{
			<Pallet<T>>::do_confirm_pallet_migrations(project_location, 0, xcm_response).unwrap();
		}

		// * validity checks *
		frame_system::Pallet::<T>::assert_last_event(
			Event::<T>::MigrationStatusUpdated { project_id, account: participant, status: MigrationStatus::Confirmed }
				.into(),
		);
	}

	#[benchmark]
	fn do_handle_channel_open_request() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		#[block]
		{
			<Pallet<T>>::do_handle_channel_open_request(6969u32, 50_000, 8).unwrap();
		}
	}

	#[benchmark]
	fn do_handle_channel_accepted() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

		let jwt = get_mock_jwt_with_cid(
			issuer.clone(),
			InvestorType::Institutional,
			generate_did_from_account(issuer.clone()),
			project_metadata.clone().policy_ipfs_cid.unwrap(),
		);

		<Pallet<T>>::start_pallet_migration(
			RawOrigin::Signed(issuer.clone()).into(),
			jwt.clone(),
			project_id,
			6969u32.into(),
		)
		.unwrap();

		<Pallet<T>>::do_handle_channel_open_request(6969u32, 102_400u32, 1000).unwrap();

		#[block]
		{
			<Pallet<T>>::do_handle_channel_accepted(6969u32).unwrap();
		}
	}

	#[benchmark]
	fn mark_project_ct_migration_as_finished() {
		// setup
		let mut inst = BenchInstantiator::<T>::new(None);
		<T as Config>::SetPrices::set_prices();

		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);

		let project_metadata = default_project_metadata::<T>(issuer.clone());
		let project_id = inst.create_settled_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations::<T>(),
			default_bids::<T>(),
			default_community_contributions::<T>(),
			vec![],
			true,
		);

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
