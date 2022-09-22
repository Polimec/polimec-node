#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;
pub use types::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use orml_traits::arithmetic::Zero;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_tokens::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;

		/// The maximum length of a name or symbol stored on-chain.
		#[pallet::constant]
		type StringLimit: Get<u32>;

		// TODO: Add Weight type

		// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;
	}

	// TODO: Since this storage is used also in other pallets. check best practices to
	// share the storage.
	// https://substrate.stackexchange.com/questions/3354/access-storage-map-from-another-pallet-without-trait-pallet-config
	#[pallet::storage]
	/// Details of currencies.
	#[pallet::getter(fn currencies)]
	pub(super) type Currencies<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, CurrencyInfo<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn currencies_metadata)]
	/// Metadata of a Currency.
	pub(super) type Metadata<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		CurrencyMetadata<BalanceOf<T>, BoundedVec<u8, T::StringLimit>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RegisteredCurrency(T::CurrencyId, T::AccountId),
		MintedCurrency(T::CurrencyId, T::AccountId, T::Amount),
		Transferred(T::CurrencyId, T::AccountId, T::AccountId, T::Balance),
		LockedTrading(T::CurrencyId),
		UnlockedTrading(T::CurrencyId),
		OwnershipChanged(T::CurrencyId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		CurrencyAlreadyExists,
		Unauthorized,
		CurrencyNotFound,
		NativeCurrencyCannotBeChanged,
		TransferLocked,
		TransferToThemself,
		AmountTooLow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn register(
			origin: OriginFor<T>,
			issuer: T::AccountId,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			ensure_root(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			ensure!(!Currencies::<T>::contains_key(currency_id), Error::<T>::CurrencyAlreadyExists);

			// TODO: Pass the `name` and the `symbol` as parameter to the `register`
			let bounded_name: BoundedVec<u8, T::StringLimit> =
				b"My Token".to_vec().try_into().expect("asset name is too long");
			let bounded_symbol: BoundedVec<u8, T::StringLimit> =
				b"TKN_____".to_vec().try_into().expect("asset symbol is too long");

			// TODO: 
			let currency_metadata = CurrencyMetadata {
				deposit: Zero::zero(),
				name: bounded_name,
				symbol: bounded_symbol,
				decimals: 12,
			};
			// 	Do not enable trading by default
			let currency_info = CurrencyInfo::new(issuer.clone(), false, TradingStatus::Disabled);
			<Currencies<T>>::insert(currency_id, currency_info);
			<Metadata<T>>::insert(currency_id, currency_metadata);
			Self::deposit_event(Event::<T>::RegisteredCurrency(currency_id, issuer));

			Ok(())
		}

		/// NOTE: the amount is in PICO! In the Polkadot apps all currencies are displayed in Units,
		/// whereas here it displays PICO because it doesn't know that this is a currency.
		/// We maybe also want to have CurrencyOf as a type here because we never take currency
		/// away.
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn mint(
			origin: OriginFor<T>,
			issuer: T::AccountId,
			target: T::AccountId,
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			Self::do_mint(issuer, target, &currency_id, amount)
		}

		/// Lock currency trading
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn lock_trading(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::set_trading_status(who, &currency_id, TradingStatus::Disabled)
		}

		/// Unlock currency trading
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn unlock_trading(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::set_trading_status(who, &currency_id, TradingStatus::Enabled)
		}

		/// Transfer some amount from one account to another.
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn transfer(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			Self::do_transfer(from, to, &currency_id, &amount)
		}
	}
}

use frame_support::{ensure, pallet_prelude::DispatchError, traits::Get, BoundedVec};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};

impl<T: Config> Pallet<T> {
	/// NOTE: the amount is in PICO! In the polkadot apps all currencies are
	/// displayed in Units, whereas here it displays PICO because it doesn't
	/// know that this is a currency. We maybe also want to have CurrencyOf as a
	/// type here because we never take currency away.
	pub fn do_mint(
		who: T::AccountId,
		target: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount: AmountOf<T>,
	) -> Result<(), DispatchError> {
		ensure!(
			currency_id != &T::GetNativeCurrencyId::get(),
			Error::<T>::NativeCurrencyCannotBeChanged
		);
		if let Some(currency_info) = Currencies::<T>::get(currency_id) {
			ensure!(currency_info.issuer == who, Error::<T>::Unauthorized);
			// should increase by `amount`, not set
			orml_tokens::Pallet::<T>::update_balance(*currency_id, &target, amount)?;
			Self::deposit_event(Event::<T>::MintedCurrency(*currency_id, target, amount));
			Ok(())
		} else {
			Err(Error::<T>::CurrencyNotFound.into())
		}
	}

	pub fn set_trading_status(
		who: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		trading_status: TradingStatus,
	) -> Result<(), DispatchError> {
		Currencies::<T>::mutate(currency_id, |currency| -> Result<(), DispatchError> {
			match currency {
				Some(currency_info) => {
					ensure!(currency_info.issuer == who, Error::<T>::Unauthorized);
					currency_info.trading_enabled = trading_status;
					match currency_info.trading_enabled {
						TradingStatus::Enabled =>
							Self::deposit_event(Event::<T>::UnlockedTrading(*currency_id)),
						TradingStatus::Disabled =>
							Self::deposit_event(Event::<T>::LockedTrading(*currency_id)),
					}
				},
				None => return Err(Error::<T>::CurrencyNotFound.into()),
			}
			Ok(())
		})
	}

	pub fn do_transfer(
		from: T::AccountId,
		to: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount: &BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// Check if the amount is more than 0 before accessing the storage.
		ensure!(*amount > 0_u8.into(), Error::<T>::AmountTooLow);

		// Check if the from and the to are diffrent accounts.
		ensure!(from != to, Error::<T>::TransferToThemself);

		let currency_info = Currencies::<T>::get(currency_id);

		// Check whether the currency exists
		ensure!(currency_info.is_some(), Error::<T>::CurrencyNotFound);

		// Check whether transfer is unlocked
		match currency_info.expect("Already checked").transfers_frozen {
			true => {
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
					*currency_id,
					&from,
					&to,
					*amount,
				)?;
				Self::deposit_event(Event::<T>::Transferred(*currency_id, from, to, *amount));
				Ok(())
			},
			false => Err(Error::<T>::TransferLocked.into()),
		}
	}
}
