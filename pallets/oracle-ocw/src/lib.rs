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
		AssetName, AssetRequest, BitFinexFetcher, BitStampFetcher, CoinbaseFetcher, KrakenFetcher, MexcFetcher, OpenCloseVolume, XTFetcher,
	},
};
use core::ops::Rem;
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
pub use pallet::*;
use sp_runtime::{
	traits::{Convert, Saturating, Zero},
	FixedU128, RuntimeAppPublic,
};
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

mod mock;
mod tests;

mod traits;

pub mod types;

pub mod crypto;

const LOG_TARGET: &str = "ocw::oracle";
// Change values in Fetcher urls when changing this value
pub(crate) const NUMBER_OF_CANDLES: usize = 15;

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
		traits::{IdentifyAccount, Zero},
	};

	const LOCK_TIMEOUT_EXPIRATION: u64 = 30_000; // 30 seconds

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type PublicOf<T> = <T as SigningTypes>::Public;
	pub type SignatureOf<T> = <T as SigningTypes>::Signature;
	pub type GenericPublicOf<T> = <<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::GenericPublic;
	pub type RuntimeAppPublicOf<T> =
		<<T as Config>::AppCrypto as AppCrypto<PublicOf<T>, SignatureOf<T>>>::RuntimeAppPublic;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		CreateSignedTransaction<OracleCall<Self>> + frame_system::Config + orml_oracle::Config<()>
	{
		/// The overarching event type of the runtime.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The cryptographic interface for the offchain worker to sign transactions.
		type AppCrypto: AppCrypto<Self::Public, Self::Signature>;
		/// List of members that are allowed to send transactions.
		type Members: frame_support::traits::Contains<AccountIdOf<Self>>;
		/// Interval between price fetches in block numbers.
		/// The interval is started at block `n % FetchInterval == 0`
		type FetchInterval: Get<BlockNumberFor<Self>>;
		/// Window size for fetching prices. Ocw will try to successfully fetch the prices once in this window.
		/// The window is started at block `n % FetchWindow == 0`
		/// If the fetch is not successful, the ocw will try again on the next block in the window.
		/// If the ocw is not successful in the window, it will try again in the next window.
		/// Example: FetchInterval = 10, FetchWindow = 5 => Ocw will try to fetch prices once
		/// for the next windows: [0, 5), [10, 15), [20, 25), ...
		type FetchWindow: Get<BlockNumberFor<Self>>;
		/// Convert AssetName and FixedU128 to OracleKey and OracleValue
		type ConvertAssetPricePair: Convert<(AssetName, FixedU128), (Self::OracleKey, Self::OracleValue)>;
	}

	#[pallet::event]
	pub enum Event<T: Config> {}

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
			log::trace!(target: LOG_TARGET, "Running offchain worker for block: {:?}", block_number);
			let local_keys = RuntimeAppPublicOf::<T>::all();
			log::trace!(target: LOG_TARGET, "Num of Local keys: {:?}", local_keys.len());
			// Check if Signing key is in the permissioned set of keys.
			let maybe_key: Option<RuntimeAppPublicOf<T>> = local_keys.into_iter().find_map(|key| {
				log::trace!(target: LOG_TARGET, "runtime key: {:?}", key.to_raw_vec());
				let generic_public = GenericPublicOf::<T>::from(key);
				let public: T::Public = generic_public.into();
				let account = public.clone().into_account();
				if <T as pallet::Config>::Members::contains(&account) {
					if let Ok(generic_public) = TryInto::<GenericPublicOf<T>>::try_into(public) {
						return Some(generic_public.into());
					}
				}
				None
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
							(AssetName::PLMC, Zero::zero()),
						]),
					};
					let assets = last_send_for_assets
						.iter()
						.filter_map(|(asset_name, last_send)| {
							let window = T::FetchWindow::get();
							let remainder = block_number.rem(T::FetchInterval::get());
							if remainder >= BlockNumberFor::<T>::zero() &&
								remainder < window && last_send < &block_number.saturating_sub(window)
							{
								return Some(*asset_name);
							}
							None
						})
						.collect::<Vec<AssetName>>();

					if assets.is_empty() {
						return;
					}

					log::trace!(target: LOG_TARGET, "Transaction grace period reached for assets {:?} in block {:?}", assets, block_number);

					let prices = Self::fetch_prices(assets);
					if prices.is_empty() {
						return;
					}

					for (asset_name, price) in prices.clone() {
						log::trace!(target: LOG_TARGET, "Fetched price for {:?}: {}", asset_name, price);
					}
					let result = Self::send_signed_transaction(prices.clone());
					if result.is_ok() {
						for (asset_name, _) in prices {
							last_send_for_assets.insert(asset_name, block_number);
						}
						val.set(&last_send_for_assets);
					}
				};
			}

			// Todo:
			// - Fetch price information for Polimec
		}
	}

	impl<T: Config> Pallet<T> {
		fn fetch_prices(assets: Vec<AssetName>) -> BTreeMap<AssetName, FixedU128> {
			let fetchers = vec![
				KrakenFetcher::get_moving_average,
				BitFinexFetcher::get_moving_average,
				BitStampFetcher::get_moving_average,
				CoinbaseFetcher::get_moving_average,
				XTFetcher::get_moving_average,
				MexcFetcher::get_moving_average,
			];

			let mut aggr_prices: BTreeMap<AssetName, Vec<(FixedU128, FixedU128)>> = BTreeMap::new();
			for fetcher in fetchers.into_iter() {
				let fetcher_prices = fetcher(assets.clone(), 5000);
				for (asset_name, volume_price_sum, tot_vol) in fetcher_prices {
					aggr_prices
						.entry(asset_name)
						.and_modify(|e| e.push((volume_price_sum, tot_vol)))
						.or_insert(vec![(volume_price_sum, tot_vol)]);
				}
			}

			Self::combine_prices(aggr_prices)
		}

		fn combine_prices(prices: BTreeMap<AssetName, Vec<(FixedU128, FixedU128)>>) -> BTreeMap<AssetName, FixedU128> {
			prices
				.into_iter()
				.filter_map(|(key, price_list)| {
					if price_list.is_empty() {
						return None;
					}
					let combined_prices =
						price_list.into_iter().fold((FixedU128::zero(), FixedU128::zero()), |acc, (price, volume)| {
							(acc.0 + price, acc.1 + volume)
						});
					if combined_prices.1.is_zero() {
						return None;
					}
					Some((key, combined_prices.0.div(combined_prices.1)))
				})
				.collect::<BTreeMap<AssetName, FixedU128>>()
		}

		fn send_signed_transaction(prices: BTreeMap<AssetName, FixedU128>) -> Result<(), ()> {
			let signer = Signer::<T, T::AppCrypto>::any_account();
			let prices = prices
				.into_iter()
				.map(|(asset_name, price)| T::ConvertAssetPricePair::convert((asset_name, price)))
				.collect::<Vec<(T::OracleKey, T::OracleValue)>>();

			let call = OracleCall::<T, ()>::feed_values { values: BoundedVec::<_, _>::truncate_from(prices) };
			let result = signer.send_signed_transaction(|_account| call.clone());
			match result {
				Some((_, Ok(_))) => {
					log::trace!(target: LOG_TARGET, "offchain tx sent successfully");
					return Ok(());
				},
				_ => {
					log::trace!(target: LOG_TARGET, "failure: offchain_signed_tx");
					return Err(());
				},
			}
		}
	}
}
