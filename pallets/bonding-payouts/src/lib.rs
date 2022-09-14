#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	pallet_prelude::DispatchResultWithPostInfo, weights::Weight, BoundedVec, RuntimeDebugNoBound,
};
use orml_traits::{arithmetic::Zero, LockIdentifier, MultiCurrency, MultiLockableCurrency};
use pallet_multi_stake::EraIndex;
use scale_info::TypeInfo;
use sp_runtime::{Permill, RuntimeDebug};
use sp_std::convert::From;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub const PAYOUTS_ID: LockIdentifier = *b"payouts ";
pub const MAX_UNLOCKING_CHUNKS: usize = 32;

pub(crate) type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UnlockChunk<Balance: HasCompact> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	value: Balance,
	/// Era number at which point it'll be unlocked.
	#[codec(compact)]
	era: EraIndex,
}

/// The ledger of a (bonded) stash.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct StakingLedger<T: Config + orml_tokens::Config> {
	/// The stash account whose balance is actually locked and at stake.
	pub stash: T::AccountId,
	/// The currency which is staked
	pub currency_id: CurrencyIdOf<T>,
	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: BalanceOf<T>,
	/// The total amount of the stash's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active: BalanceOf<T>,
	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming it doesn't get slashed first). It is assumed that this will be treated as a first
	/// in, first out queue where the new (higher value) eras get pushed on the back.
	pub unlocking: BoundedVec<UnlockChunk<BalanceOf<T>>, T::MaxUnlockingChunks>,
}

impl<T: Config + orml_tokens::Config> StakingLedger<T> {
	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	/// Remove entries from `unlocking` that are sufficiently old and reduce the
	/// total by the sum of their balances.
	fn consolidate_unlocked(self, current_era: EraIndex) -> Self {
		let mut total = self.total;
		let unlocking: BoundedVec<_, _> = self
			.unlocking
			.into_iter()
			.filter(|chunk| {
				if chunk.era > current_era {
					true
				} else {
					total = total.saturating_sub(chunk.value);
					false
				}
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect(
				"filtering items from a bounded vec always leaves length less than bounds. qed",
			);

		Self {
			stash: self.stash,
			total,
			active: self.active,
			unlocking,
			currency_id: self.currency_id,
		}
	}

	/// Re-bond funds that were scheduled for unlocking.
	fn rebond(mut self, value: BalanceOf<T>) -> Self {
		let mut unlocking_balance = BalanceOf::<T>::zero();

		while let Some(last) = self.unlocking.last_mut() {
			if unlocking_balance + last.value <= value {
				unlocking_balance += last.value;
				self.active += last.value;
				self.unlocking.pop();
			} else {
				let diff = value - unlocking_balance;

				unlocking_balance += diff;
				self.active += diff;
				last.value -= diff;
			}

			if unlocking_balance >= value {
				break
			}
		}

		self
	}
}

#[frame_support::pallet]
pub mod pallet {
	use crate::{
		BalanceOf, CurrencyIdOf, EraIndex, StakingLedger, UnlockChunk, MAX_UNLOCKING_CHUNKS,
		PAYOUTS_ID,
	};
	use frame_support::{
		pallet_prelude::{OptionQuery, ValueQuery, *},
		sp_runtime::traits::StaticLookup,
		traits::DefensiveSaturating,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{arithmetic::Zero, MultiCurrency, MultiLockableCurrency};
	use sp_runtime::{traits::CheckedSub, Permill};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_multi_mint::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxUnlockingChunks: Get<u32>;

		/// Number of blocks that staked funds must remain bonded for.
		type BondingDuration: Get<EraIndex>;

		/// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;

		type MaxProposals: Get<pallet_proposal::ProposalIndex>;

		/// Address for payouts.
		type PayoutPoolAddress: Get<<Self as frame_system::Config>::AccountId>;
	}

	/// The current era index.
	///
	/// This is the latest planned era, depending on how the Session pallet queues the validator
	/// set, it might be active or not.
	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	/// Map from all locked "stash" accounts to the controller account.
	pub type Bonded<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyIdOf<T>,
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
		CurrencyIdOf<T>,
		StakingLedger<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn payouts)]
	/// Map from currency to amount of balance which can still be bonded.

	pub(super) type PayoutPool<T: Config> =
		StorageMap<_, Blake2_256, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn scheduled_payouts)]
	/// Map from block number to controller account and potential balance of unbonded currency.
	pub(super) type ScheduledPayouts<T: Config> =
		StorageMap<_, Blake2_256, EraIndex, Vec<(T::AccountId, CurrencyIdOf<T>)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn payout_rate)]
	/// Map from currency to payout rate in percent
	/// Has to be submitted during pallet_issuer_council::apply_for_seat
	/// And cannot be changed
	pub(super) type PayoutRate<T: Config> =
		StorageMap<_, Blake2_256, CurrencyIdOf<T>, Permill, OptionQuery>;

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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			Self::check_scheduled_payout()
		}
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

			frame_system::Pallet::<T>::inc_consumers(&stash)?;

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
				<ScheduledPayouts<T>>::append(1_u32, (controller, currency_id));
			}
			Ok(())
		}

		/// Add some extra amount that have appeared in the stash `free_balance` into the balance up
		/// for staking.
		///
		/// Use this if there are additional funds in your stash account that you wish to bond.
		/// Unlike [`bond`] or [`unbond`] this function does not impose any limitation on the amount
		/// that can be added.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller and
		/// it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// Emits `Bonded`.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - O(1).
		/// - One DB entry.
		/// ------------
		/// DB Weight:
		/// - Read: Bonded, Ledger, [Origin Account], Locks
		/// - Write: [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn bond_extra(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			max_additional: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);

			let available_pool_value = PayoutPool::<T>::get(currency_id);
			ensure!(!available_pool_value.is_zero(), Error::<T>::PayoutPoolEmpty);

			let controller = Self::bonded(&stash, &currency_id).ok_or(Error::<T>::NotStash)?;
			let mut ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;

			let stash_balance =
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(
					currency_id,
					&stash,
				);

			if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
				let extra = extra.min(max_additional);
				let extra = extra.min(available_pool_value);
				ledger.total += extra;
				ledger.active += extra;
				Self::deposit_event(Event::Bonded(stash, currency_id, extra));
				Self::update_ledger(&controller, &ledger);

				// Update available balance of payout pool
				<PayoutPool<T>>::insert(currency_id, available_pool_value - extra);
			}
			Ok(())
		}

		/// Schedule a portion of the stash to be unlocked ready for transfer out after the bond
		/// period ends.
		///
		/// Once the unlock period is done, you can call `withdraw_unbonded` to actually move
		/// the funds out of management ready for transfer.
		///
		/// No more than a limited number of unlocking chunks (see `MAX_UNLOCKING_CHUNKS`)
		/// can co-exists at the same time. In that case, [`Call::withdraw_unbonded`] need
		/// to be called first to remove some of the chunks (if possible).
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		///
		/// Emits `Unbonded`.
		/// Calls `UnbondedVote::update_amount`.
		///
		/// See also [`Call::withdraw_unbonded`].
		///
		/// # <weight>
		/// - Time complexity: O(A * U), where A is number of active applications, U number of
		///   applicants user voted for
		/// - Independent of the arguments. Limited but potentially exploitable complexity.
		/// - Contains a limited number of reads.
		/// - Each call will cause a new entry to be inserted into a vector (`Ledger.unlocking`)
		///   kept in storage. The only way to clean the aforementioned storage item is also
		///   user-controlled via `withdraw_unbonded`.
		/// - One DB entry.
		/// ----------
		/// DB Weight:
		/// - Read: Block, Ledger, Locks, BalanceOf Stash, (UserVotes, Applicants)
		/// - Write: Locks, Ledger, BalanceOf Stash, (ApplicantVotes, UserVotes)
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let controller = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);
			let mut ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
			ensure!(ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS, Error::<T>::NoMoreChunks,);

			let value = value.min(ledger.active);
			let post_info_weight = if !value.is_zero() {
				ledger.active -= value;

				// TODO: Re-activate after adding MinimumExistentialDeposit
				// Avoid there being a dust balance left in the staking system.
				// if ledger.active < T::Currency::minimum_balance() {
				//     value += ledger.active;
				//     ledger.active = Zero::zero();
				// }

				let era = Self::calc_unlock_block(Self::current_era().unwrap_or(0));
				if let Some(mut chunk) =
					ledger.unlocking.last_mut().filter(|chunk| chunk.era == era)
				{
					// To keep the chunk count down, we only keep one chunk per era. Since
					// `unlocking` is a FiFo queue, if a chunk exists for `era` we know that it will
					// be the last one.
					chunk.value = chunk.value.defensive_saturating_add(value)
				} else {
					ledger
						.unlocking
						.try_push(UnlockChunk { value, era })
						.map_err(|_| Error::<T>::NoMoreChunks)?;
				};
				Self::update_ledger(&controller, &ledger);
				Self::deposit_event(Event::Unbonded(ledger.stash, currency_id, value));

				// Reduce voting weight
				// addition is safe due to above chec k

				// TODO:
				//Some(<T as Config>::WeightInfo::unbond())
				None
			} else {
				None
			};

			Ok(post_info_weight.into())
		}

		/// (Re-)set the controller of a stash.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
		///
		/// # <weight>
		/// - Independent of the arguments. Insignificant complexity.
		/// - Contains a limited number of reads.
		/// - Writes are limited to the `origin` account key.
		/// ----------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: Bonded, Ledger New Controller, Ledger Old Controller
		/// - Write: Bonded, Ledger New Controller, Ledger Old Controller
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn set_controller(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);
			let old_controller = Self::bonded(&stash, &currency_id).ok_or(Error::<T>::NotStash)?;
			let controller = T::Lookup::lookup(controller)?;
			if <Ledger<T>>::contains_key(&controller, &currency_id) {
				return Err(Error::<T>::AlreadyPaired.into())
			}
			if controller != old_controller {
				<Bonded<T>>::insert(&stash, &currency_id, &controller);
				if let Some(l) = <Ledger<T>>::take(&old_controller, &currency_id) {
					<Ledger<T>>::insert(&controller, &currency_id, l);
				}
			}
			Ok(())
		}
		/// Force a current staker to become completely unstaked, immediately.
		///
		/// The dispatch origin must be Root.
		///
		/// # <weight>
		/// O(1)
		/// Reads: Bonded, Slashing Spans, Account, Locks
		/// Writes: Bonded, Ledger, Account, Locks
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn force_unstake(
			origin: OriginFor<T>,
			stash: T::AccountId,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			// remove all staking-related information.
			Self::kill_stash(&stash, &currency_id)?;

			// remove the lock.
			let _ = <orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
				PAYOUTS_ID,
				currency_id,
				&stash,
			)?;

			Ok(())
		}

		/// Rebond a portion of the stash scheduled to be unlocked.
		///
		/// The dispatch origin must be signed by the controller, and it can be only called when
		/// [`EraElectionStatus`] is `Closed`.
		///
		/// # <weight>
		/// - Time complexity: O(L), where L is unlocking chunks
		/// - Bounded by `MAX_UNLOCKING_CHUNKS`.
		/// - Storage changes: Can't increase storage, only decrease it.
		/// ---------------
		/// - DB Weight:
		///     - Reads: Ledger, Locks, [Origin Account]
		///     - Writes: [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn rebond(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);
			let ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
			ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockChunk);

			let ledger = ledger.rebond(value);
			Self::update_ledger(&controller, &ledger);
			// TODO: Return DispatchResultWithPostInfo
			// TODO: Add Weights
			// Ok(Some(<T as Config>::WeightInfo::rebond(ledger.unlocking.len() as u32)).into())
			Ok(())
		}

		/// Remove all data structure concerning a staker/stash once its balance is zero.
		/// This is essentially equivalent to `withdraw_unbonded` except it can be called by anyone
		/// and the target `stash` must have no funds left.
		///
		/// This can be called from any origin.
		///
		/// - `stash`: The stash account to reap. Its balance must be zero.
		///
		/// # <weight>
		/// Complexity: O(1)
		/// DB Weight:
		/// - Reads: Stash Account, Bonded, Locks
		/// - Writes: Bonded, Ledger, Stash Account, Locks
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn reap_stash(
			_origin: OriginFor<T>,
			stash: T::AccountId,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure!(
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::total_balance(
					currency_id,
					&stash
				)
				.is_zero(),
				Error::<T>::FundedTarget
			);
			Self::kill_stash(&stash, &currency_id)?;
			<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
				PAYOUTS_ID,
				currency_id,
				&stash,
			)?;
			Ok(())
		}
	}
}

use frame_support::{traits::Get, weights::PostDispatchInfo};
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
		<Ledger<T>>::insert(controller, ledger.currency_id, ledger);
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
	fn get_payout_era(block: EraIndex) -> EraIndex {
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
	fn calc_unlock_block(era: EraIndex) -> EraIndex {
		let duration = T::BondingDuration::get();
		// have to reduce by 1 due to checking current_block > unlock_block during
		// unbond
		Self::get_payout_era(era) + duration
	}

	fn do_withdraw_unbonded(
		controller: &T::AccountId,
		currency_id: &CurrencyIdOf<T>,
	) -> DispatchResultWithPostInfo {
		let mut ledger =
			Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
		let current_block = Self::current_era().unwrap_or(0) + T::BondingDuration::get();
		let (stash, old_total) = (ledger.stash.clone(), ledger.total);
		ledger = ledger.consolidate_unlocked(current_block);

		let mut post_info_weight = if ledger.unlocking.is_empty() && ledger.active.is_zero() {
			// This account must have called `unbond()` with some value that caused the
			// active portion to fall below existential deposit + will have no more
			// unlocking chunks left. We can now safely remove all staking-related
			// information.
			Self::kill_stash(&stash, &currency_id)?;
			// remove the lock.
			<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
				PAYOUTS_ID,
				*currency_id,
				&stash,
			)?;
			// This is worst case scenario, so we use the full weight and return None
			0
		} else {
			// This was the consequence of a partial unbond. just update the ledger and move
			// on.
			Self::update_ledger(&controller, &ledger);

			// This is only an update, so we use less overall weight.
			// TODO: Add weight
			//<T as Config>::WeightInfo::withdraw_unbonded_update(ledger.unlocking.len() as u32)
			// TODO: Remove the 1 and return the correct weight
			1
		};

		// `old_total` should never be less than the new total because
		// `consolidate_unlocked` strictly subtracts balance.
		if ledger.total < old_total {
			// Already checked that this won't overflow by entry condition.
			let value = old_total - ledger.total;
			Self::deposit_event(Event::Withdrawn(stash, *currency_id, value));

			// Update available balance of payout pool
			let available_pool_value = PayoutPool::<T>::get(currency_id);

			<PayoutPool<T>>::insert(currency_id, available_pool_value + value);
			post_info_weight =
				post_info_weight.saturating_add(T::DbWeight::get().reads_writes(2, 1));
		}

		Ok(Some(post_info_weight).into())
		//Ok((Some(0)).into())
	}

	// transfer payouts from pool to stash accounts for current block
	// count all db reads and writes to know the exact weight
	fn check_scheduled_payout() -> Weight {
		let era = Self::current_era().unwrap_or(0) + T::BondingDuration::get();
		let payouts = <ScheduledPayouts<T>>::get(era);
		let mut post_info_weight: Weight = 0;

		for (controller, currency_id) in payouts.into_iter() {
			if let Some(StakingLedger { stash, total, .. }) =
				<Ledger<T>>::get(&controller, &currency_id)
			{
				// get payout_rate
				if let Some(rate) = <PayoutRate<T>>::get(currency_id) {
					// withdraw unbonded + update bonded balance
					// upon failer, controller does not exist
					if let Ok(withdraw) = Self::do_withdraw_unbonded(&controller, &currency_id) {
						post_info_weight =
							post_info_weight.saturating_add(withdraw.actual_weight.unwrap_or(0));
						if let PostDispatchInfo { actual_weight: Some(weight), .. } = withdraw {
							post_info_weight = post_info_weight.saturating_add(weight);
						}
					}

					// set new scheduled payout for updated bonded balance
					if let Some(StakingLedger { total: total_new, .. }) =
						<Ledger<T>>::get(&controller, &currency_id)
					{
						if total_new > T::Balance::zero() {
							<ScheduledPayouts<T>>::append(
								Self::get_payout_era(era),
								(controller, currency_id),
							);
							post_info_weight =
								post_info_weight.saturating_add(T::DbWeight::get().writes(1));
						}
					}
					post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));

					// transfer payout from pool to stash
					let pool_balance =
						<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(
							currency_id,
							&T::PayoutPoolAddress::get(),
						);
					post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));

					let payout = pool_balance.min(rate * total);
					if payout > T::Balance::zero() {
						// transfer payout from pool to stash, e.g.
						// no automatic bond of payout for compound payouts
						// note: we don't use mutli_mint::do_transfer since this requires transfers
						// to be unlocked
						let _ = <orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::transfer(
							currency_id,
							&T::PayoutPoolAddress::get(),
							&stash,
							payout,
						);
						// TODO: Add Weight in multi-mint pallet
						// post_info_weight = post_info_weight.saturating_add(
						// 	<T as pallet_multi_mint::Config>::WeightInfo::transfer(),
						// );
					}
				} else {
					post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(2));
				}
			} else {
				post_info_weight = post_info_weight.saturating_add(T::DbWeight::get().reads(1));
			}
		}
		// remove current entry
		<ScheduledPayouts<T>>::remove(era);

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
