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
		CurrencyMetadata<BoundedVec<u8, T::StringLimit>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RegisteredCurrency(T::CurrencyId, T::AccountId),
		MintedCurrency(T::CurrencyId, T::AccountId, T::Amount),
		BurnedCurrency(T::CurrencyId, T::AccountId, T::Amount),
		Transferred(T::CurrencyId, T::AccountId, T::AccountId, T::Balance),
		ChangedTrading(T::CurrencyId, TradingStatus),
		ChangedTransfer(T::CurrencyId, TransferStatus),
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
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn register(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			currency_metadata: CurrencyMetadata<BoundedVec<u8, T::StringLimit>>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let issuer = ensure_signed(origin.clone())?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			ensure!(!Currencies::<T>::contains_key(currency_id), Error::<T>::CurrencyAlreadyExists);

			// 	Do not enable trading by default
			// The issuer is also the owner by default
			let currency_info = CurrencyInfo {
				current_owner: issuer.clone(),
				issuer: issuer.clone(),
				transfers_enabled: TransferStatus::Enabled,
				trading_enabled: TradingStatus::Disabled,
			};
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
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn mint(
			origin: OriginFor<T>,
			target: T::AccountId,
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let issuer = ensure_signed(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			ensure!(amount > 0_u8.into(), Error::<T>::AmountTooLow);

			Self::do_mint(issuer, target, &currency_id, amount)
		}

		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn burn(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> DispatchResult {
			// TODO: Ensure that the user is credentialized
			let who = ensure_signed(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			ensure!(amount > 0_u8.into(), Error::<T>::AmountTooLow);

			Self::do_burn(who, &currency_id, amount)
		}

		/// Lock currency trading
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn lock_trading(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(who == currency.issuer, Error::<T>::Unauthorized);

			Self::set_trading_status(&currency_id, TradingStatus::Disabled)
		}

		/// Unlock currency trading
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn unlock_trading(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(who == currency.issuer, Error::<T>::Unauthorized);

			Self::set_trading_status(&currency_id, TradingStatus::Enabled)
		}

		/// Lock currency trading
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn lock_transfer(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(who == currency.issuer, Error::<T>::Unauthorized);

			Self::set_transfer_status(&currency_id, TransferStatus::Disabled)
		}

		/// Unlock currency trading
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn unlock_transfer(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(who == currency.issuer, Error::<T>::Unauthorized);

			Self::set_transfer_status(&currency_id, TransferStatus::Enabled)
		}

		/// Transfer some amount from one account to another.
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn transfer(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;
			// Check if the amount is more than 0 before accessing the storage.
			ensure!(amount > 0_u8.into(), Error::<T>::AmountTooLow);

			// Check if the from and the to are diffrent accounts.
			ensure!(from != to, Error::<T>::TransferToThemself);

			Self::do_transfer(from, to, &currency_id, &amount)
		}

		// Destroy a registered currency.
		// TODO: Add proper weight
		#[pallet::weight(Weight::from_ref_time(10_000) + T::DbWeight::get().reads_writes(1,1))]
		pub fn destroy(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
			ensure!(who == currency.issuer, Error::<T>::Unauthorized);
			Self::do_destroy(&currency_id)
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
		let currency_info =
			Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
		ensure!(currency_info.issuer == who, Error::<T>::Unauthorized);

		orml_tokens::Pallet::<T>::update_balance(*currency_id, &target, amount)?;

		Self::deposit_event(Event::<T>::MintedCurrency(*currency_id, target, amount));
		Ok(())
	}

	pub fn set_trading_status(
		currency_id: &CurrencyIdOf<T>,
		trading_status: TradingStatus,
	) -> Result<(), DispatchError> {
		Currencies::<T>::try_mutate(currency_id, |maybe_currency| -> Result<(), DispatchError> {
			maybe_currency.as_mut().ok_or(Error::<T>::CurrencyNotFound)?.trading_enabled =
				trading_status.clone();
			Self::deposit_event(Event::<T>::ChangedTrading(*currency_id, trading_status));
			Ok(())
		})
	}

	pub fn set_transfer_status(
		currency_id: &CurrencyIdOf<T>,
		transfer_status: TransferStatus,
	) -> Result<(), DispatchError> {
		Currencies::<T>::try_mutate(currency_id, |maybe_currency| -> Result<(), DispatchError> {
			maybe_currency.as_mut().ok_or(Error::<T>::CurrencyNotFound)?.transfers_enabled =
				transfer_status.clone();
			Self::deposit_event(Event::<T>::ChangedTransfer(*currency_id, transfer_status));
			Ok(())
		})
	}

	pub fn do_transfer(
		from: T::AccountId,
		to: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount: &BalanceOf<T>,
	) -> Result<(), DispatchError> {
		let currency_info =
			Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;

		// Check whether transfer is unlocked
		match currency_info.transfers_enabled {
			TransferStatus::Enabled => {
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
					*currency_id,
					&from,
					&to,
					*amount,
				)?;
				Self::deposit_event(Event::<T>::Transferred(*currency_id, from, to, *amount));
				Ok(())
			},
			TransferStatus::Disabled => Err(Error::<T>::TransferLocked.into()),
		}
	}

	pub fn do_destroy(_currency_id: &CurrencyIdOf<T>) -> Result<(), DispatchError> {
		todo!("https://github.com/paritytech/substrate/blob/master/frame/assets/src/functions.rs#L668")
	}

	pub fn do_burn(
		who: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount: AmountOf<T>,
	) -> Result<(), DispatchError> {
		Currencies::<T>::get(currency_id).ok_or(Error::<T>::CurrencyNotFound)?;
		orml_tokens::Pallet::<T>::update_balance(*currency_id, &who, -amount)?;

		Self::deposit_event(Event::<T>::BurnedCurrency(*currency_id, who, amount));
		Ok(())
	}
}
