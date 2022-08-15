#![cfg_attr(not(feature = "std"), no_std)]

/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in
/// runtime/src/lib.rs If you remove this file, you can remove those references

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod default_weights;

pub use default_weights::WeightInfo;
/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, traits::Get,
	unsigned::TransactionValidityError,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{currency::MultiCurrencyExtended, MultiCurrency};
use sp_runtime::{
	traits::{CheckedSub, DispatchInfoOf, PostDispatchInfoOf, Zero},
	transaction_validity::InvalidTransaction,
};
use sp_std::{
	convert::{TryFrom, TryInto},
	marker::PhantomData,
};
use system::ensure_root;
use transaction_payment::OnChargeTransaction;

type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

type AmountOf<T> = <T as orml_tokens::Config>::Amount;

type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

/// The pallet's configuration trait.
pub trait Config: frame_system::Config + orml_tokens::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
	// type MultiCurrency: MultiCurrencyExtended<<Self as
	// frame_system::Config>::AccountId>;
	type GetNativeCurrencyId: Get<CurrencyIdOf<Self>>;

	/// Weight information for extrinsics in this pallet.
	type WeightInfo: WeightInfo;
}

// This pallet's storage items.
decl_storage! {

	trait Store for Module<T: Config> as PreCurrencyModule {
		// currency_id -> (issuer_address, trading_enabled)?
		pub CurrencyMetadata get(fn issuer_of_currency) build(|config: &GenesisConfig<T>| {
			config.pre_currency.iter()
				.map(|(c_id, acc_id, enabled)| (*c_id, (acc_id.clone(), *enabled)))
				.collect()
		}):
			map hasher(opaque_blake2_256) CurrencyIdOf<T> => Option<(T::AccountId, bool)>;
	}
	add_extra_genesis {
		config(pre_currency): Vec<(CurrencyIdOf<T>, T::AccountId, bool)>;
	}
}

// The pallet's events
decl_event!(
	pub enum Event<T>
	where
		AccountId = <T as frame_system::Config>::AccountId,
		CurrencyId = CurrencyIdOf<T>,
		Amount = AmountOf<T>,
		Balance = BalanceOf<T>,
	{
		MintedCurrency(CurrencyId, AccountId, Amount),
		RegisteredCurrency(CurrencyId, AccountId),
		Transferred(CurrencyId, AccountId, AccountId, Balance),
		LockedTrading(CurrencyId),
		UnlockedTrading(CurrencyId),
		OwnershipChanged(CurrencyId, AccountId),
	}
);

// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Config> {
		CurrencyAlreadyExists,
		Unauthorized,
		CurrencyNotFound,
		NativeCurrencyCannotBeChanged,
		TransferLocked,
	}
}

// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;

		/// register a new currency
		#[weight = <T as Config>::WeightInfo::register()]
		pub fn register(origin, issuer: T::AccountId, currency_id: CurrencyIdOf<T>) -> dispatch::DispatchResult {
			ensure_root(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::NativeCurrencyCannotBeChanged);
			Self::do_register_currency(issuer, &currency_id)
		}

		/// NOTE: the amount is in PICO! In the polkadot apps all currencies are displayed in Units, whereas here it displays PICO because it doesn't know that this is a currency.
		/// We maybe also want to have CurrencyOf as a type here because we never take currency away.
		#[weight = <T as Config>::WeightInfo::mint()]
		fn mint(origin, issuer: T::AccountId, currency_id: CurrencyIdOf<T>, amount: AmountOf<T>) -> dispatch::DispatchResult {
			ensure_root(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::NativeCurrencyCannotBeChanged);
			Self::do_mint(issuer, &currency_id, amount)
		}

		/// Lock currency trading
		#[weight = <T as Config>::WeightInfo::lock_trading()]
		pub fn lock_trading(
			origin,
			currency_id: CurrencyIdOf<T>,
		) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_lock_trading(&who, &currency_id)
		}

		/// Unlock currency trading
		#[weight = <T as Config>::WeightInfo::unlock_trading()]
		pub fn unlock_trading(
			origin,
			currency_id: CurrencyIdOf<T>,
		) -> dispatch::DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_unlock_trading(&who, &currency_id)
		}

		/// Transfer some amount from one account to another.
		#[weight = <T as Config>::WeightInfo::transfer()]
		pub fn transfer(
			origin,
			currency_id: CurrencyIdOf<T>,
			to: T::AccountId,
			amount: BalanceOf<T>,
		) -> dispatch::DispatchResult {
			let from = ensure_signed(origin)?;
			Self::do_transfer(from, to, &currency_id, &amount)
		}
	}
}

pub trait GetFeeCurrencyID<C> {
	fn get_currency_id(&self) -> C;
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<T, C>(PhantomData<T>, PhantomData<C>)
where
	T: frame_system::Config + pallet_authorship::Config,
	C: MultiCurrencyExtended<T::AccountId>,
	T::Call: GetFeeCurrencyID<C::CurrencyId>;

impl<T, C> OnChargeTransaction<T> for ToAuthor<T, C>
where
	T: Config + transaction_payment::Config + pallet_authorship::Config,
	<T as transaction_payment::Config>::TransactionByteFee: Get<BalanceOf<T>>,
	C: MultiCurrencyExtended<T::AccountId, Balance = BalanceOf<T>>,
	C::Amount: TryFrom<BalanceOf<T>> + TryInto<BalanceOf<T>>,
	C::CurrencyId: Default,
	T::Call: GetFeeCurrencyID<C::CurrencyId>,
{
	/// (currency_id, paid_fee, tip included in fee)
	type LiquidityInfo = Option<(C::CurrencyId, Self::Balance, Self::Balance)>;
	type Balance = BalanceOf<T>;

	// FIXME: We have quite a lot storage calls here (Withdraw and deposit cost both
	// 2 storage calls). In substrate that is solved by a complex Imbalance system
	// that levarages the memory management for that.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::Call,
		_dispatch_info: &DispatchInfoOf<T::Call>,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		let currency_id = call.get_currency_id();
		if fee.is_zero() || C::withdraw(currency_id, who, fee).is_ok() {
			Ok(Some((currency_id, fee, tip)))
		} else {
			Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		}
	}

	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::Call>,
		_post_info: &PostDispatchInfoOf<T::Call>,
		fee: Self::Balance,
		_tip: Self::Balance,
		liquidity_info: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		// if there is no liquidity info, we don't need to do anything.
		let (currency_id, predicted_fee, _tip) = if let Some(li) = liquidity_info {
			li
		} else {
			return Ok(());
		};
		let author = <pallet_authorship::Module<T>>::author();
		// We can only refund here. If the predicted fee was less, the only that is
		// paid.
		let min_fee = predicted_fee.min(fee);

		// If the predicted_fee was to mutch, refund!
		if let Some(refund) = predicted_fee.checked_sub(&fee) {
			C::deposit(currency_id, who, refund).unwrap_or_else(|_| {
				debug::info!("Could not deposit refund for too high transaction fees!");
			})
		}
		C::deposit(currency_id, &author, min_fee)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))
	}
}

impl<T: Config> Module<T> {
	pub fn do_register_currency(
		issuer: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
	) -> Result<(), dispatch::DispatchError> {
		if !<CurrencyMetadata<T>>::contains_key(currency_id) {
			// do not enable trading by default
			<CurrencyMetadata<T>>::insert(currency_id, (&issuer, false));
			Self::deposit_event(RawEvent::RegisteredCurrency(*currency_id, issuer));
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
	) -> Result<(), dispatch::DispatchError> {
		ensure!(
			currency_id != &T::GetNativeCurrencyId::get(),
			Error::<T>::NativeCurrencyCannotBeChanged
		);
		if let Some((issuer, _)) = CurrencyMetadata::<T>::get(&currency_id) {
			ensure!(issuer == who, Error::<T>::Unauthorized);
			orml_tokens::Module::<T>::update_balance(*currency_id, &who, amount)?;
			Self::deposit_event(RawEvent::MintedCurrency(*currency_id, who, amount));
			Ok(())
		} else {
			Err(Error::<T>::CurrencyNotFound.into())
		}
	}

	pub fn do_lock_trading(who: &T::AccountId, currency_id: &CurrencyIdOf<T>) -> Result<(), dispatch::DispatchError> {
		if let Some((issuer, _)) = CurrencyMetadata::<T>::get(&currency_id) {
			ensure!(&issuer == who, Error::<T>::Unauthorized);
			<CurrencyMetadata<T>>::insert(currency_id, (&issuer, false));
			Self::deposit_event(RawEvent::LockedTrading(*currency_id));
			Ok(())
		} else {
			Err(Error::<T>::CurrencyNotFound.into())
		}
	}

	pub fn do_unlock_trading(who: &T::AccountId, currency_id: &CurrencyIdOf<T>) -> Result<(), dispatch::DispatchError> {
		if let Some((issuer, _)) = CurrencyMetadata::<T>::get(currency_id) {
			ensure!(&issuer == who, Error::<T>::Unauthorized);
			<CurrencyMetadata<T>>::insert(currency_id, (&issuer, true));
			Self::deposit_event(RawEvent::UnlockedTrading(*currency_id));
			Ok(())
		} else {
			Err(Error::<T>::CurrencyNotFound.into())
		}
	}

	pub fn do_transfer(
		from: T::AccountId,
		to: T::AccountId,
		currency_id: &CurrencyIdOf<T>,
		amount: &BalanceOf<T>,
	) -> Result<(), dispatch::DispatchError> {
		// check whether transfer is unlocked
		match CurrencyMetadata::<T>::get(&currency_id) {
			Some((_, true)) => {
				<orml_tokens::Module<T> as MultiCurrency<T::AccountId>>::transfer(*currency_id, &from, &to, *amount)?;
				Self::deposit_event(RawEvent::Transferred(*currency_id, from, to, *amount));
				Ok(())
			}
			Some((_, false)) => Err(Error::<T>::TransferLocked.into()),
			None => Err(Error::<T>::CurrencyNotFound.into()),
		}
	}
}
