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

//! Offchain Worker for Oracle price feed
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;
use crate::{
	traits::FetchPrice,
	types::{AssetName, AssetRequest, KrakenFetcher}
};
use sp_runtime::{traits::{Zero, Saturating}, FixedU128, RuntimeAppPublic};
use std::collections::HashMap;
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
mod mock;
mod tests;

mod traits;

mod types;

mod crypto;

#[frame_support::pallet]
pub mod pallet {

use crate::types::{BitFinexFetcher, BitStampFetcher};

	use super::*;
	use frame_support::{pallet_prelude::*, traits::Contains};
	use frame_system::pallet_prelude::*;
	use frame_system::offchain::SigningTypes;
	use sp_runtime::traits::IdentifyAccount;


	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type PublicOf<T> = <T as SigningTypes>::Public;
	// pub type SignatureOf<T> = <T as SigningTypes>::Signature;
	// pub type GenericPublicOf<T> = <<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::GenericPublic;
	// pub type RuntimeAppPublicOf<T> = <<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::RuntimeAppPublic;


	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		
		/// The overarching event type of the runtime.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// An Identifying key for the offchain worker. Used to determine if the offchain
		/// worker is authorized to submit price feeding transactions.
		type AuthorityId: Member
		+ Parameter
		+ RuntimeAppPublic
		+ Ord
		+ MaybeSerializeDeserialize
		+ MaxEncodedLen
		+ Into<Self::Public>;
		
		type AppCrypto: AppCrypto<Self::Public, Self::Signature>;

		type Members: frame_support::traits::Contains<Self::AccountId>;
		
	}

	#[pallet::storage]
	pub type DummyValue<T> = StorageValue<_, u32>;
	
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The value was changed to the new value.
		Changed {
			/// The new value.
			value: u32,
		}
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The value was too large.
		TooLarge,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// successfully imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			let local_keys = T::AuthorityId::all();

			let mut found_key: bool = false;
			for key in local_keys.iter() {
				let account: AccountIdOf<T> = <PublicOf<T> as IdentifyAccount>::into_account(key.clone().into());
				if T::Members::contains(&account) && !found_key {
					found_key = true;
					
					let prices = Self::fetch_prices();
					println!("Prices: {:?}", prices);
					
				} 
			}

			// Todo: 
			// - Introduce backoff system to prevent spamming
			// - Fetch price information for each asset
			// - Fetch price information from different sources
			// - Send signed price value to chain
			
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		fn fetch_prices() -> HashMap<AssetName, FixedU128> {
			let assets: Vec<AssetName> = vec![AssetName::USDT, AssetName::USDC, AssetName::DOT];
			let kraken_prices = KrakenFetcher::get_moving_average(assets.clone(), 5000);
			let bitfinex_prices = BitFinexFetcher::get_moving_average(assets.clone(), 5000);
			let bitstamp_prices = BitStampFetcher::get_moving_average(assets.clone(), 5000);
			
			let mut prices: HashMap<AssetName, Vec<FixedU128>> = HashMap::new();
			for (asset_name, price) in kraken_prices.into_iter().chain(bitfinex_prices.into_iter()).chain(bitstamp_prices.into_iter()) {
				prices.entry(asset_name).and_modify(|e| e.push(price)).or_insert(vec![price]);
			}

			Self::combine_prices(prices)

		}

		fn combine_prices(prices: HashMap<AssetName, Vec<FixedU128>>) -> HashMap<AssetName, FixedU128>{

			prices.into_iter().filter_map(|(key, value)| {
				let mut value = value;
				value.sort();
				match value.len() {
					0 => None,
					1 => Some((key, value[0])),
					2 => Some((key, value[0].saturating_add(value[1]) / FixedU128::from_u32(2u32))),
					_ => {
						let value = value[1..value.len()].iter().fold(FixedU128::from_u32(0), |acc, x| acc.saturating_add(*x)) / FixedU128::from_u32((value.len() - 1) as u32 );
						Some((key, value))
					}
				}
			}).collect::<HashMap<AssetName, FixedU128>>()
		}
	}
}
