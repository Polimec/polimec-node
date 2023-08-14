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
use frame_support::{traits::OriginTrait, Parameter, log::Level,};
use pallet::Pallet as PalletFunding;
use scale_info::prelude::format;
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Member};
use frame_benchmarking::log::log;
const METADATA: &str = r#"
{
    "whitepaper":"ipfs_url",
    "team_description":"ipfs_url",
    "tokenomics":"ipfs_url",
    "roadmap":"ipfs_url",
    "usage_of_founds":"ipfs_url"
}
"#;

const EDIT_METADATA: &str = r#"
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
const PLMC_UNIT: u128 = 1_0_000_000_000u128;

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

pub fn default_evaluations<T: Config>() -> Vec<UserToUSDBalance<T>> {
	vec![
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_1", 0, 0),
			(50_000 * US_DOLLAR).try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		),
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_2", 0, 0),
			(25_000 * US_DOLLAR).try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		),
		UserToUSDBalance::new(
			account::<AccountIdOf<T>>("evaluator_3", 0, 0),
			(32_000 * US_DOLLAR).try_into().unwrap_or_else(|_| panic!("Failed to create BalanceOf")),
		),
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
	use frame_support::dispatch::RawOrigin;

	impl_benchmark_test_suite!(PalletFunding, crate::mock::new_test_ext(), crate::mock::TestRuntime);

	type BenchInstantiator<T> = Instantiator<
		T,
		<T as Config>::AllPalletsWithoutSystem,
		<T as Config>::RuntimeEvent,
	>;

	// #[benchmark]
	// fn bid() {
	// 	let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
	// 	frame_system::Pallet::<T>::remark_with_event(RawOrigin::Signed(issuer.clone()).into(), vec![1u8,2u8,3u8,4u8]);
	// 	let debug_events = frame_system::Pallet::<T>::events();
	// 	log!(
	// 		Level::Error,
	// 		"frame system default events {:?}",
	// 		debug_events
	// 	);
	// 	let mut inst = BenchInstantiator::<T>::new(None);
	//
	// 	let bidder = account::<AccountIdOf<T>>("bidder", 0, 0);
	// 	whitelist_account!(bidder);
	//
	// 	let project_id = inst.create_auctioning_project(
	// 		default_project::<T>(inst.get_new_nonce(), issuer.clone()),
	// 		issuer,
	// 		default_evaluations::<T>(),
	// 	);
	//
	// 	let bid_params = BidParams::new(
	// 		bidder.clone(),
	// 		(50000u128 * ASSET_UNIT).into(),
	// 		18_u128.into(),
	// 		1u8,
	// 		AcceptedFundingAsset::USDT,
	// 	);
	// 	let necessary_plmc = BenchInstantiator::<T>::calculate_auction_plmc_spent(vec![bid_params.clone()]);
	// 	let existential_deposits: Vec<UserToPLMCBalance<T>> = necessary_plmc.accounts().existential_deposits();
	// 	let necessary_usdt = BenchInstantiator::<T>::calculate_auction_funding_asset_spent(vec![bid_params.clone()]);
	//
	// 	inst.mint_plmc_to(necessary_plmc);
	// 	inst.mint_plmc_to(existential_deposits);
	// 	inst.mint_statemint_asset_to(necessary_usdt);
	//
	// 	#[extrinsic_call]
	// 	bid(
	// 		RawOrigin::Signed(bidder.clone()),
	// 		project_id,
	// 		bid_params.amount,
	// 		bid_params.price,
	// 		bid_params.multiplier,
	// 		bid_params.asset,
	// 	);
	//
	// 	let debug_events = frame_system::Pallet::<T>::events();
	// 	log!(
	// 		Level::Error,
	// 		"frame system default events {:?}",
	// 		debug_events
	// 	);
	//
	// }

	#[benchmark]
	fn test(){
		let issuer = account::<AccountIdOf<T>>("issuer", 0, 0);
		frame_system::Pallet::<T>::remark_with_event(RawOrigin::Signed(issuer.clone()).into(), vec![1u8,2u8,3u8,4u8]);
		let mut inst = BenchInstantiator::<T>::new(None);
		inst.advance_time(5u32.into()).unwrap();

		let debug_events = frame_system::Pallet::<T>::events();
		log!(
			Level::Error,
			"frame system default events {:?}",
			debug_events
		);

		#[block]
		{

		}

		let debug_events = frame_system::Pallet::<T>::events();
		log!(
			Level::Error,
			"frame system default events {:?}",
			debug_events
		);
	}
}
