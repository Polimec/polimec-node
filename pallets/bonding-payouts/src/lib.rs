#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::DispatchResultWithPostInfo,
	weights::{PostDispatchInfo, Weight},
	BoundedVec,
};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, MaxEncodedLen};
use orml_traits::{LockIdentifier, MultiCurrency, MultiLockableCurrency};
use scale_info::TypeInfo;
use sp_runtime::{traits::Zero, Permill, RuntimeDebug};
use sp_std::convert::From;

pub const PAYOUTS_ID: LockIdentifier = *b"payouts ";
pub const MAX_UNLOCKING_CHUNKS: usize = 32;

type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be
/// unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct UnlockChunk<T: Config + orml_tokens::Config> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	pub value: BalanceOf<T>,
	/// Block number at which point it'll be unlocked.
	#[codec(compact)]
	pub block: T::BlockNumber,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct StakingLedger<T: Config + orml_tokens::Config> {
	/// The stash account whose balance is actually locked and at stake.
	pub stash: T::AccountId,
	/// The currency which is staked
	pub currency_id: CurrencyIdOf<T>,
	/// The total amount of the stash's balance that we are currently accounting
	/// for. It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: BalanceOf<T>,
	/// The total amount of the stash's balance that will be at stake in any
	/// forthcoming rounds.
	#[codec(compact)]
	pub active: BalanceOf<T>,
	/// Any balance that is becoming free, which may eventually be transferred
	/// out of the stash.
	pub unlocking: BoundedVec<UnlockChunk<T>, T::MaxUnlockingChunks>,
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::{OptionQuery, ValueQuery, *},
		sp_runtime::traits::{Saturating, StaticLookup},
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{arithmetic::Zero, LockIdentifier, MultiCurrency, MultiLockableCurrency};
	use sp_runtime::Permill;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_multi_mint::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxUnlockingChunks: Get<u32>;

		/// Number of blocks that staked funds must remain bonded for.
		type BondingDuration: Get<Self::BlockNumber>;

		/// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;

		type MaxProposals: Get<pallet_proposal::ProposalIndex>;

		/// Address for payouts.
		type PayoutPoolAddress: Get<<Self as frame_system::Config>::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	/// Map from all locked "stash" accounts to the controller account.
	pub type Bonded<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		crate::CurrencyIdOf<T>,
		T::AccountId,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	/// Map from all (unlocked) "controller" accounts to the info regarding the staking.
	pub type Ledger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		crate::CurrencyIdOf<T>,
		// TODO: Use StakingLedger<T> instead of T::AccountId as map value.
		//crate::StakingLedger<T>
		T::AccountId,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn payouts)]
	pub(super) type PayoutPool<T: Config> =
		StorageMap<_, Blake2_256, crate::CurrencyIdOf<T>, crate::BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn scheduled_payouts)]
	pub(super) type ScheduledPayouts<T: Config> =
		StorageMap<_, Blake2_256, T::BlockNumber, crate::BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn payout_rate)]
	pub(super) type PayoutRate<T: Config> =
		StorageMap<_, Blake2_256, crate::CurrencyIdOf<T>, Permill, OptionQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Unreserve(T::AccountId, T::CurrencyId, T::Balance, T::BlockNumber),
		/// An account has bonded this amount. \[stash, amount\]
		Bonded(T::AccountId, T::CurrencyId, T::Balance),
		/// An account has unbonded this amount. \[stash, amount\]
		Unbonded(T::AccountId, T::CurrencyId, T::Balance),
		/// An account has called `withdraw_unbonded` and removed unbonding
		/// chunks worth `Balance` from the unlocking queue. \[stash, amount\]
		Withdrawn(T::AccountId, T::CurrencyId, T::Balance),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Not a controller account.
		NotController,
		/// Not a stash account.
		NotStash,
		/// Stash is already bonded.
		AlreadyBonded,
		/// Controller is already paired.
		AlreadyPaired,
		/// Slash record index out of bounds.
		InsufficientValue,
		/// Can not schedule more unlock chunks.
		NoMoreChunks,
		/// Can not rebond without unlocking chunks.
		NoUnlockChunk,
		/// Attempting to target a stash that still has funds.
		FundedTarget,
		/// Can not bond native currency.
		IsNativeCurrency,
		/// Payouts and thus bonding is disabled
		PayoutPoolEmpty,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn bond(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			currency_id: crate::CurrencyIdOf<T>,
			value: crate::BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);

			let available_pool_value = PayoutPool::<T>::get(currency_id);
			ensure!(!available_pool_value.is_zero(), Error::<T>::PayoutPoolEmpty);

			if <Bonded<T>>::contains_key(&stash, &currency_id) {
				return Err(Error::<T>::AlreadyBonded.into())
			}

			let controller = T::Lookup::lookup(controller)?;

			if <Ledger<T>>::contains_key(&controller, &currency_id) {
				return Err(Error::<T>::AlreadyPaired.into())
			}

			// TODO: Re-enable once we have minimum balance
			// reject a bond which is considered to be _dust_.
			// if value < T::Currency::minimum_balance() {
			//     Err(Error::<T>::InsufficientValue)?
			// }

			// You're auto-bonded forever, here.
			<Bonded<T>>::insert(&stash, &currency_id, &controller);

			frame_system::Pallet::<T>::inc_consumers(&stash);

			let stash_balance = <orml_tokens::Pallet<T> as orml_traits::MultiCurrency<
				T::AccountId,
			>>::free_balance(currency_id, &stash);
			let value = value.min(stash_balance);
			let value = value.min(available_pool_value);

			if !value.is_zero() {
				Self::deposit_event(Event::Bonded(stash.clone(), currency_id, value));
				let item = crate::StakingLedger {
					stash,
					currency_id,
					total: value,
					active: value,
					unlocking: BoundedVec::default(),
				};
				Self::update_ledger(&controller, &item);

				// Update available balance of payout pool
				<PayoutPool<T>>::insert(currency_id, available_pool_value - value);
				// Set scheduled payout
				// TODO:
				// <ScheduledPayouts<T>>::append(
				// 	Self::get_payout_block(<frame_system::Pallet<T>>::block_number()),
				// 	(controller, currency_id),
				// );
			}
			Ok(())
		}
	}
}

use frame_support::traits::Get;
use sp_runtime::{traits::Saturating, DispatchResult};

impl<T: Config> Pallet<T> {
	/// Update the ledger for a controller.
	///
	/// This will also update the stash lock.
	fn update_ledger(controller: &T::AccountId, ledger: &StakingLedger<T>) {
		let _ = <orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::set_lock(
			PAYOUTS_ID,
			ledger.currency_id,
			&ledger.stash,
			ledger.total,
		);
		// TODO:
		//<Ledger<T>>::insert(controller, ledger.currency_id, ledger);
	}

	/// Remove all associated data of a stash account from the staking system.
	///
	/// Assumes storage is upgraded before calling.
	///
	/// This is called:
	/// - after a `withdraw_unbond()` call that frees all of a stash's bonded balance.
	/// - through `reap_stash()` if the balance has fallen to zero (through slashing).
	fn kill_stash(stash: &T::AccountId, currency_id: &CurrencyIdOf<T>) -> DispatchResult {
		let controller = Bonded::<T>::get(stash, currency_id).ok_or(Error::<T>::NotStash)?;

		<Bonded<T>>::remove(stash, currency_id);
		<Ledger<T>>::remove(&controller, currency_id);

		frame_system::Pallet::<T>::dec_consumers(stash);

		Ok(())
	}

	/// Calculate the block for the automatic scheduled payout.
	fn get_payout_block(block: T::BlockNumber) -> T::BlockNumber {
		let duration = T::BondingDuration::get();
		let remaining_era_blocks = duration.saturating_sub(block % duration);

		// adding 1 yields the effect of checking current_block > unlock_block during
		// unbond whereas in check_scheduled_payouts we check whether current_block ==
		// payout_block
		block + remaining_era_blocks
	}

	/// Calculate the block in which you can `withdraw_unbondend`.
	///
	/// Unlocking shall not happen in this era, thus increment the payout block
	/// by the duration. Another possibility would be to only yield payout on
	/// active bonding, not total.
	fn calc_unlock_block(block: T::BlockNumber) -> T::BlockNumber {
		let duration = T::BondingDuration::get();
		// have to reduce by 1 due to checking current_block > unlock_block during
		// unbond
		Self::get_payout_block(block) + duration
	}

	fn do_withdraw_unbonded(
		controller: &T::AccountId,
		currency_id: &CurrencyIdOf<T>,
	) -> DispatchResultWithPostInfo {
		let mut ledger =
			Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
		let current_block = <frame_system::Pallet<T>>::block_number();
		// let (stash, old_total) = (ledger.stash.clone(), ledger.total);
		// ledger = ledger.consolidate_unlocked(current_block);

		// let mut post_info_weight = if ledger.unlocking.is_empty() && ledger.active.is_zero() {
		// 	// This account must have called `unbond()` with some value that caused the
		// 	// active portion to fall below existential deposit + will have no more
		// 	// unlocking chunks left. We can now safely remove all staking-related
		// 	// information.
		// 	Self::kill_stash(&stash, &currency_id)?;
		// 	// remove the lock.
		// 	<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
		// 		PAYOUTS_ID,
		// 		*currency_id,
		// 		&stash,
		// 	)?;
		// 	// This is worst case scenario, so we use the full weight and return None
		// 	0
		// } else {
		// 	// This was the consequence of a partial unbond. just update the ledger and move
		// 	// on.
		// 	Self::update_ledger(&controller, &ledger);

		// 	// This is only an update, so we use less overall weight.
		// 	<T as Config>::WeightInfo::withdraw_unbonded_update(ledger.unlocking.len() as u32)
		// };

		// // `old_total` should never be less than the new total because
		// // `consolidate_unlocked` strictly subtracts balance.
		// if ledger.total < old_total {
		// 	// Already checked that this won't overflow by entry condition.
		// 	let value = old_total - ledger.total;
		// 	Self::deposit_event(RawEvent::Withdrawn(stash, *currency_id, value));

		// 	// Update available balance of payout pool
		// 	let available_pool_value = PayoutPool::<T>::get(currency_id);

		// 	<PayoutPool<T>>::insert(currency_id, available_pool_value + value);
		// 	post_info_weight =
		// 		post_info_weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));
		// }

		// Ok(Some(post_info_weight).into())
		Ok((Some(0)).into())
	}

	// transfer payouts from pool to stash accounts for current block
	// count all db reads and writes to know the exact weight
	fn check_scheduled_payout() -> Weight {
		let now = frame_system::Pallet::<T>::block_number();
		let payouts = <ScheduledPayouts<T>>::get(now);
		let mut post_info_weight: Weight = 0;

		// for (controller, currency_id) in payouts.into_iter() {
		// 	if let Some(StakingLedger { stash, total, .. }) =
		// 		<Ledger<T>>::get(&controller, &currency_id)
		// 	{
		// 		// get payout_rate
		// 		if let Some(rate) = <PayoutRate<T>>::get(currency_id) {
		// 			// withdraw unbonded + update bonded balance
		// 			// upon failer, controller does not exist
		// 			if let Ok(withdraw) = Self::do_withdraw_unbonded(&controller, &currency_id) {
		// 				post_info_weight =
		// 					post_info_weight.saturating_add(withdraw.actual_weight.unwrap_or(0));
		// 				if let PostDispatchInfo { actual_weight: Some(weight), .. } = withdraw {
		// 					post_info_weight = post_info_weight.saturating_add(weight);
		// 				}
		// 			}

		// 			// set new scheduled payout for updated bonded balance
		// 			if let Some(StakingLedger { total: total_new, .. }) =
		// 				<Ledger<T>>::get(&controller, &currency_id)
		// 			{
		// 				if total_new > T::Balance::zero() {
		// 					<ScheduledPayouts<T>>::append(
		// 						Self::get_payout_block(now),
		// 						(controller, currency_id),
		// 					);
		// 					post_info_weight =
		// 						post_info_weight.saturating_add(T::DbWeight::get().writes(1));
		// 				}
		// 			}
		// 			post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));

		// 			// transfer payout from pool to stash
		// 			let pool_balance =
		// 				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(
		// 					currency_id,
		// 					&T::PayoutPoolAddress::get(),
		// 				);
		// 			post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));

		// 			let payout = pool_balance.min(rate * total);
		// 			if payout > T::Balance::zero() {
		// 				// transfer payout from pool to stash, e.g.
		// 				// no automatic bond of payout for compound payouts
		// 				// note: we don't use mutli_mint::do_transfer since this requires transfers
		// 				// to be unlocked
		// 				let _ = <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
		// 					currency_id,
		// 					&T::PayoutPoolAddress::get(),
		// 					&stash,
		// 					payout,
		// 				);
		// 				post_info_weight = post_info_weight.saturating_add(
		// 					<T as pallet_multi_mint::Config>::WeightInfo::transfer(),
		// 				);
		// 			}
		// 		} else {
		// 			post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(2));
		// 		}
		// 	} else {
		// 		post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));
		// 	}
		// }
		// remove current entry
		<ScheduledPayouts<T>>::remove(now);

		// return best estimation of used weight
		post_info_weight.saturating_add(T::DbWeight::get().writes(1))
	}
}

impl<T: Config> traits::PayoutPool<CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_amount(currency_id: &CurrencyIdOf<T>) -> BalanceOf<T> {
		PayoutPool::<T>::get(currency_id)
	}
	fn set_amount(currency_id: &CurrencyIdOf<T>, amount: &BalanceOf<T>) {
		PayoutPool::<T>::insert(currency_id, amount)
	}
	fn set_rate(currency_id: &CurrencyIdOf<T>, rate: &Permill) {
		PayoutRate::<T>::insert(currency_id, rate)
	}
}
