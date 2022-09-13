#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::DispatchResult;
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

use frame_support::traits::Get;
use orml_traits::MultiLockableCurrency;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{Saturating, StaticLookup},
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::{arithmetic::Zero, LockIdentifier, MultiCurrency, MultiLockableCurrency};
	use pallet_proposal::ProposalIndex;
	use scale_info::TypeInfo;

	pub const STAKING_ID: LockIdentifier = *b"staking ";
	pub const MAX_UNLOCKING_CHUNKS: usize = 32;

	pub(crate) type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

	pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

	/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct UnlockChunk<T: Config> {
		/// Amount of funds to be unlocked.
		pub value: BalanceOf<T>,
		/// Block number at which point it'll be unlocked.
		/// TODO: Check BlockNumber vs Era
		pub block: T::BlockNumber,
	}

	/// The ledger of a (bonded) stash.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen)]
	pub struct StakingLedger<T: Config> {
		/// The stash account whose balance is actually locked and at stake.
		pub stash: T::AccountId,
		/// The currency which is staked
		pub currency_id: CurrencyIdOf<T>,
		/// The total amount of the stash's balance that we are currently accounting
		/// for. It's just `active` plus all the `unlocking` balances.
		pub total: BalanceOf<T>,
		/// The total amount of the stash's balance that will be at stake in any
		/// forthcoming rounds.
		pub active: BalanceOf<T>,
		/// Any balance that is becoming free, which may eventually be transferred
		/// out of the stash.
		pub unlocking: BoundedVec<UnlockChunk<T>, T::MaxUnlockingChunks>,
	}

	impl<T: Config + orml_tokens::Config> StakingLedger<T> {
		/// Remove entries from `unlocking` that are sufficiently old and reduce the
		/// total by the sum of their balances.
		fn consolidate_unlocked(self, current_block: T::BlockNumber) -> Self {
			let mut total = self.total;
			let unlocking = self
				.unlocking
				.into_iter()
				.filter(|chunk| {
					if chunk.block > current_block {
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

	// TODO: Remove `#[pallet::without_storage_info]` and implement MaxEncodedLen for
	// `StakingLedger`
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_multi_mint::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Number of blocks that staked funds must remain bonded for.
		type BondingDuration: Get<Self::BlockNumber>;

		/// Weight information for extrinsics in this pallet.
		// TODO: Add Weight after benchmarks
		// type WeightInfo: WeightInfo;

		type BondedVote: traits::BondedVote<
			<Self as frame_system::Config>::AccountId,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		#[pallet::constant]
		type MaxProposals: Get<ProposalIndex>;

		/// The maximum number of `unlocking` chunks a [`StakingLedger`] can have. Effectively
		/// determines how many unique eras a staker may be unbonding in.
		#[pallet::constant]
		type MaxUnlockingChunks: Get<u32>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn bonded)]
	pub type Bonded<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		T::AccountId,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ledger)]
	pub type Ledger<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		// TODO: Use StakingLedger<T> instead of T::AccountId as map value.
		//crate::StakingLedger<T>
		T::AccountId,
	>;

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
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Take the origin account as a stash and lock up `value` of its balance. `controller` will
		/// be the account that controls it.
		///
		/// The dispatch origin for this call must be _Signed_ by the stash account.
		///
		/// Emits `Bonded`.
		///
		/// # <weight>
		/// - Independent of the arguments. Moderate complexity.
		/// - O(1).
		/// - Three extra DB entries.
		///
		/// NOTE: One of the storage writes (`Self::bonded`) is _never_ cleaned
		/// ------------------
		/// Weight: O(1)
		/// DB Weight:
		/// - Read: Bonded, Ledger, [Origin Account], Locks
		/// - Write: Bonded, [Origin Account], Locks, Ledger
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn bond(
			origin: OriginFor<T>,
			controller: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);

			if <Bonded<T>>::contains_key(&stash, &currency_id) {
				return Err(Error::<T>::AlreadyBonded.into())
			}

			let controller = T::Lookup::lookup(controller)?;

			// TODO: Add Ledger Logic
			// if <Ledger<T>>::contains_key(&controller, &currency_id) {
			// 	return Err(Error::<T>::AlreadyPaired.into())
			// }

			// TODO: Re-enable once we have minimum balance
			// reject a bond which is considered to be _dust_.
			// if value < T::Currency::minimum_balance() {
			//     Err(Error::<T>::InsufficientValue)?
			// }

			// You're auto-bonded forever, here.
			<Bonded<T>>::insert(&stash, &currency_id, &controller);

			frame_system::Pallet::<T>::inc_consumers(&stash)?;

			let stash_balance =
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(
					currency_id,
					&stash,
				);
			let value = value.min(stash_balance);
			Self::deposit_event(Event::Bonded(stash.clone(), currency_id, value));

			let item = StakingLedger::<T> {
				stash,
				currency_id,
				total: value,
				active: value,
				unlocking: BoundedVec::default(),
			};
			// TODO: Add Ledger Logic
			// Self::update_ledger(&controller, &item)?;

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
			controller: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);

			let controller = Self::bonded(&stash, &currency_id).ok_or(Error::<T>::NotStash)?;
			let mut ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
			let stash_balance =
				<orml_tokens::Pallet<T> as MultiCurrency<T::AccountId>>::free_balance(
					currency_id,
					&stash,
				);
			// TODO: Add Ledger Logic
			// if let Some(extra) = stash_balance.checked_sub(&ledger.total) {
			// 	let extra = extra.min(max_additional);
			// 	ledger.total += extra;
			// 	ledger.active += extra;
			// 	Self::deposit_event(Event::Bonded(stash, currency_id, extra));
			// 	Self::update_ledger(&controller, &ledger)?;
			// }

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
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);
			let mut ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
			// TODO: Add Ledger Logic
			// ensure!(ledger.unlocking.len() < MAX_UNLOCKING_CHUNKS, Error::<T>::NoMoreChunks,);

			// let value = value.min(ledger.active);
			// let post_info_weight = if !value.is_zero() {
			// 	ledger.active -= value;

			// 	// TODO: Re-activate after adding MinimumExistentialDeposit
			// 	// Avoid there being a dust balance left in the staking system.
			// 	// if ledger.active < T::Currency::minimum_balance() {
			// 	//     value += ledger.active;
			// 	//     ledger.active = Zero::zero();
			// 	// }
			// 	let block = Self::calc_unlock_block(<frame_system::Pallet<T>>::block_number());
			// 	ledger.unlocking.push(UnlockChunk { value, block });
			// 	Self::update_ledger(&controller, &ledger)?;
			// 	Self::deposit_event(Event::Unbonded(ledger.stash, currency_id, value));

			// 	// Reduce voting weight
			// 	// addition is safe due to above check
			// 	let votes = T::BondedVote::update_amount(
			// 		&controller,
			// 		&currency_id,
			// 		&ledger.active,
			// 		&(ledger.active + value),
			// 	);
			// 	Some(<T as Config>::WeightInfo::unbond(votes))
			// } else {
			// 	None
			// };

			// Ok(post_info_weight.into())
			Ok(())
		}

		/// Remove any unlocked chunks from the `unlocking` queue from our management.
		///
		/// This essentially frees up that balance to be used by the stash account to do
		/// whatever it wants.
		///
		/// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
		/// And, it can be only called when [`EraElectionStatus`] is `Closed`.
		///
		/// Emits `Withdrawn`.
		///
		/// See also [`Call::unbond`].
		///
		/// # <weight>
		/// - Could be dependent on the `origin` argument and how much `unlocking` chunks exist.
		///  It implies `consolidate_unlocked` which loops over `Ledger.unlocking`, which is
		///  indirectly user-controlled. See [`unbond`] for more detail.
		/// - Contains a limited number of reads, yet the size of which could be large based on
		///   `ledger`.
		/// - Writes are limited to the `origin` account key.
		/// ---------------
		/// Complexity O(S) where S is the number of slashing spans to remove
		/// Update:
		/// - Reads: Ledger, Locks, [Origin Account]
		/// - Writes: [Origin Account], Locks, Ledger
		/// Kill:
		/// - Reads: Ledger, Bonded, Slashing Spans, [Origin Account], Locks, BalanceOf stash
		/// - Writes: Bonded, Ledger, [Origin Account], Locks, BalanceOf stash.
		/// NOTE: Weight annotation is the kill scenario, we refund otherwise.
		/// # </weight>
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn withdraw_unbonded(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			ensure!(currency_id != T::GetNativeCurrencyId::get(), Error::<T>::IsNativeCurrency);
			let mut ledger =
				Self::ledger(&controller, &currency_id).ok_or(Error::<T>::NotController)?;
			// let (stash, old_total) = (ledger.stash.clone(), ledger.total);
			// let current_block = <frame_system::Pallet<T>>::block_number();
			// ledger = ledger.consolidate_unlocked(current_block);

			// let post_info_weight = if ledger.unlocking.is_empty() && ledger.active.is_zero() {
			// 	// This account must have called `unbond()` with some value that caused the active
			// 	// portion to fall below existential deposit + will have no more unlocking chunks
			// 	// left. We can now safely remove all staking-related information.
			// 	Self::kill_stash(&stash, &currency_id)?;
			// 	// remove the lock.
			// 	<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
			// 		STAKING_ID,
			// 		currency_id,
			// 		&stash,
			// 	)?;
			// 	// This is worst case scenario, so we use the full weight and return None
			// 	None
			// } else {
			// 	// This was the consequence of a partial unbond. just update the ledger and move on.
			// 	Self::update_ledger(&controller, &ledger)?;

			// 	// This is only an update, so we use less overall weight.
			// 	Some(<T as Config>::WeightInfo::withdraw_unbonded_update(
			// 		ledger.unlocking.len() as u32
			// 	))
			// };

			// // `old_total` should never be less than the new total because
			// // `consolidate_unlocked` strictly subtracts balance.
			// if ledger.total < old_total {
			// 	// Already checked that this won't overflow by entry condition.
			// 	let value = old_total - ledger.total;
			// 	Self::deposit_event(Event::Withdrawn(stash, currency_id, value));
			// }

			// Ok(post_info_weight.into())
			Ok(())
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
			<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::remove_lock(
				STAKING_ID,
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
			// TODO: Add Ledger logic
			// ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockChunk);

			// let ledger = ledger.rebond(value);
			// Self::update_ledger(&controller, &ledger)?;
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
				STAKING_ID,
				currency_id,
				&stash,
			)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Update the ledger for a controller.
	///
	/// This will also update the stash lock.
	fn update_ledger(controller: &T::AccountId, ledger: &StakingLedger<T>) -> DispatchResult {
		<orml_tokens::Pallet<T> as MultiLockableCurrency<T::AccountId>>::set_lock(
			STAKING_ID,
			ledger.currency_id,
			&ledger.stash,
			ledger.total,
		)?;
		// TODO: Add Ledger Logic
		// <Ledger<T>>::insert(controller, ledger.currency_id, ledger);

		Ok(())
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

	/// Calculate the block in which you can `withdraw_unbondend`.
	fn calc_unlock_block(block: T::BlockNumber) -> T::BlockNumber {
		let duration = T::BondingDuration::get();
		// last modulo handles case of block % duration == 0
		let remaining_era_blocks = (duration - block % duration) % duration;
		block + remaining_era_blocks + duration
	}
}

impl<T: Config> traits::BondedAmount<T::AccountId, CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	/// Get the amount of currently bonded currency
	/// # <weight>
	/// - Time complexity: O(1)
	/// ---------------
	/// - DB Weight:
	///     - Reads: Bonded, Ledger, [Origin Account]
	/// # </weight>
	fn get_active(stash: &T::AccountId, currency_id: &CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		// TODO: Add Ledger logic
		if let Some(controller) = Bonded::<T>::get(stash, currency_id) {
			// if let Some(StakingLedger { active, .. }) = Ledger::<T>::get(controller, currency_id)
			// { 	Some(active)
			// } else {
			// 	None
			// }
			None
		} else {
			None
		}
	}
}
