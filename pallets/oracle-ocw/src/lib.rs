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
use crate::{
	traits::FetchPrice,
	types::{
		AssetName, AssetRequest, BitFinexFetcher, BitStampFetcher, CoinbaseFetcher, KrakenFetcher, OpenCloseVolume,
	},
};
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
pub use pallet::*;
use sp_runtime::{
	traits::{Convert, Saturating, Zero},
	FixedU128, RuntimeAppPublic,
};
use std::collections::HashMap;
mod mock;
mod tests;

mod traits;

mod types;

mod crypto;

const LOG_TARGET: &str = "ocw::oracle";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::*, traits::Contains};
	use frame_system::{offchain::SigningTypes, pallet_prelude::*};
	use orml_oracle::Call as OracleCall;
	use sp_runtime::{
		offchain::{
			storage::{StorageRetrievalError, StorageValueRef},
			storage_lock::{StorageLock, Time},
			Duration,
		},
		traits::IdentifyAccount,
	};
	use std::collections::BTreeMap;

	const LOCK_TIMEOUT_EXPIRATION: u64 = 30_000; // 30 seconds

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type PublicOf<T> = <T as SigningTypes>::Public;
	// pub type SignatureOf<T> = <T as SigningTypes>::Signature;
	// pub type GenericPublicOf<T> = <<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::GenericPublic;
	// pub type RuntimeAppPublicOf<T> = <<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::RuntimeAppPublic;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<OracleCall<Self>> + frame_system::Config + orml_oracle::Config<()>
	{
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

		type GracePeriod: Get<BlockNumberFor<Self>>;

		type ConvertAssetPricePair: Convert<(AssetName, FixedU128), (Self::OracleKey, Self::OracleValue)>;
	}

	#[pallet::storage]
	pub type DummyValue<T> = StorageValue<_, u32>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {}

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
			let maybe_key = local_keys.iter().find_map(|key| {
				let account: AccountIdOf<T> = <PublicOf<T> as IdentifyAccount>::into_account(key.clone().into());
				if <T as pallet::Config>::Members::contains(&account) {
					Some(key)
				} else {
					None
				}
			});

			if let Some(_authority_key) = maybe_key {
				let mut lock = StorageLock::<Time>::with_deadline(
					b"oracle_ocw::lock",
					Duration::from_millis(LOCK_TIMEOUT_EXPIRATION),
				);

				// We try to acquire the lock here. If failed, we know another ocw
				// is executing at the moment and exit this ocw.
				if let Ok(_guard) = lock.try_lock() {
					let val = StorageValueRef::persistent(b"oracle_ocw::last_send");
					let last_send_for_assets_result: Result<
						Option<BTreeMap<AssetName, BlockNumberFor<T>>>,
						StorageRetrievalError,
					> = val.get();
					let mut last_send_for_assets = match last_send_for_assets_result {
						Ok(Some(v)) => v,
						_ => BTreeMap::from([
							(AssetName::USDT, Zero::zero()),
							(AssetName::USDC, Zero::zero()),
							(AssetName::DOT, Zero::zero()),
						]),
					};
					let assets = last_send_for_assets
						.iter()
						.filter_map(|(asset_name, last_send)| {
							if block_number >= last_send.saturating_add(T::GracePeriod::get()) {
								return Some(*asset_name)
							}
							None
						})
						.collect::<Vec<AssetName>>();

					if assets.is_empty() {
						return
					}

					log::trace!(target: LOG_TARGET, "Transaction grace period reached for assets {:?} in block {:?}", assets.clone(), block_number);

					let prices = Self::fetch_prices(assets);
					let result = Self::send_signed_transaction(prices.clone());
					if result.is_ok() {
						for (asset_name, _) in prices {
							last_send_for_assets.insert(asset_name, block_number);
						}
						let _ = val.set(&last_send_for_assets);
					}
				};
			}

			// Todo:
			// - Fetch price information for each asset
			// - Fetch price information from different sources
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		fn fetch_prices(assets: Vec<AssetName>) -> HashMap<AssetName, FixedU128> {
			let fetchers = vec![
				KrakenFetcher::get_moving_average,
				BitFinexFetcher::get_moving_average,
				BitStampFetcher::get_moving_average,
				CoinbaseFetcher::get_moving_average,
			];

			let mut aggr_prices: HashMap<AssetName, Vec<FixedU128>> = HashMap::new();
			for fetcher in fetchers.into_iter() {
				let fetcher_prices = fetcher(assets.clone(), 5000);
				for (asset_name, price) in fetcher_prices {
					aggr_prices.entry(asset_name).and_modify(|e| e.push(price)).or_insert(vec![price]);
				}
			}

			Self::combine_prices(aggr_prices)
		}

		fn combine_prices(prices: HashMap<AssetName, Vec<FixedU128>>) -> HashMap<AssetName, FixedU128> {
			prices
				.into_iter()
				.filter_map(|(key, mut price_list)| {
					if price_list.is_empty() {
						return None
					}
					price_list.sort();
					let combined_prices = match price_list.len() {
						1 => price_list[0],
						2 => price_list[0].saturating_add(price_list[1]) / FixedU128::from_u32(2u32),
						len => {
							// Remove the highest and lowest price
							price_list[1..len - 1].iter().fold(FixedU128::from_u32(0), |acc, x| acc.saturating_add(*x)) /
								FixedU128::from_u32((len - 2) as u32)
						},
					};
					Some((key, combined_prices))
				})
				.collect::<HashMap<AssetName, FixedU128>>()
		}

		fn send_signed_transaction(prices: HashMap<AssetName, FixedU128>) -> Result<(), ()> {
			let signer = Signer::<T, T::AppCrypto>::any_account();
			let prices = prices
				.into_iter()
				.map(|(asset_name, price)| T::ConvertAssetPricePair::convert((asset_name, price)))
				.collect::<Vec<(T::OracleKey, T::OracleValue)>>();

			let call = OracleCall::<T, ()>::feed_values { values: BoundedVec::<_, _>::truncate_from(prices) };
			let result = signer.send_signed_transaction(|_account| call.clone());
			match result {
				Some((account, Ok(_))) => {
					log::trace!(target: LOG_TARGET, "offchain tx sent with: {:?}", account.id);
					return Ok(())
				},
				_ => {
					log::trace!(target: LOG_TARGET, "failure: offchain_signed_tx");
					return Err(())
				},
			}
		}
	}
}
