#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use orml_traits::{currency::MultiCurrencyExtended, MultiCurrency};

	type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

	// TODO: Use SCALE compact representation since this type will probably be u64/128
	type AmountOf<T> = <T as orml_tokens::Config>::Amount;
	type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_tokens::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;

		// TODO: Add Weight type

		// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn currency_metadata)]
	pub(super) type CurrencyMetadata<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, (T::AccountId, bool), OptionQuery>;

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
			ensure_root(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			Self::do_register_currency(issuer, &currency_id)
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
			currency_id: CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				currency_id != T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			Self::do_mint(issuer, &currency_id, amount)
		}

		/// Lock currency trading
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn lock_trading(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::change_trading_status(&who, &currency_id, false)
		}

		/// Unlock currency trading
		// TODO: Add proper weight
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn unlock_trading(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::change_trading_status(&who, &currency_id, true)
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

	impl<T: Config> Pallet<T> {
		pub fn do_register_currency(
			issuer: T::AccountId,
			currency_id: &CurrencyIdOf<T>,
		) -> Result<(), DispatchError> {
			if !<CurrencyMetadata<T>>::contains_key(currency_id) {
				// do not enable trading by default
				<CurrencyMetadata<T>>::insert(currency_id, (&issuer, false));
				Self::deposit_event(Event::<T>::RegisteredCurrency(*currency_id, issuer));
				Ok(())
			} else {
				Err(Error::<T>::CurrencyAlreadyExists.into())
			}
		}

		/// NOTE: the amount is in PICO! In the polkadot apps all currencies are
		/// displayed in Units, whereas here it displays PICO because it doesn't
		/// know that this is a currency. We maybe also want to have CurrencyOf as a
		/// type here because we never take currency away.
		pub fn do_mint(
			who: T::AccountId,
			currency_id: &CurrencyIdOf<T>,
			amount: AmountOf<T>,
		) -> Result<(), DispatchError> {
			ensure!(
				currency_id != &T::GetNativeCurrencyId::get(),
				Error::<T>::NativeCurrencyCannotBeChanged
			);
			if let Some((issuer, _)) = CurrencyMetadata::<T>::get(&currency_id) {
				ensure!(issuer == who, Error::<T>::Unauthorized);
				orml_tokens::Pallet::<T>::update_balance(*currency_id, &who, amount)?;
				Self::deposit_event(Event::<T>::MintedCurrency(*currency_id, who, amount));
				Ok(())
			} else {
				Err(Error::<T>::CurrencyNotFound.into())
			}
		}

		pub fn change_trading_status(
			who: &T::AccountId,
			currency_id: &CurrencyIdOf<T>,
			trading_status: bool,
		) -> Result<(), DispatchError> {
			CurrencyMetadata::<T>::mutate(
				currency_id,
				|currency_metadata| -> Result<(), DispatchError> {
					match currency_metadata {
						Some((issuer, trading_enabled)) => {
							ensure!(issuer == who, Error::<T>::Unauthorized);
							*trading_enabled = trading_status;
							if trading_status {
								Self::deposit_event(Event::<T>::UnlockedTrading(*currency_id));
							} else {
								Self::deposit_event(Event::<T>::LockedTrading(*currency_id));
							}
						},
						None => return Err(Error::<T>::CurrencyNotFound.into()),
					}
					Ok(())
				},
			)
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

			// Check whether transfer is unlocked
			match CurrencyMetadata::<T>::get(&currency_id) {
				Some((_, true)) => {
					<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
						*currency_id,
						&from,
						&to,
						*amount,
					)?;
					Self::deposit_event(Event::<T>::Transferred(*currency_id, from, to, *amount));
					Ok(())
				},
				Some((_, false)) => Err(Error::<T>::TransferLocked.into()),
				None => Err(Error::<T>::CurrencyNotFound.into()),
			}
		}
	}
}
