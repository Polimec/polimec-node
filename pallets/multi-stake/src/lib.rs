#![cfg_attr(not(feature = "std"), no_std)]

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

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{pallet_prelude::*, sp_runtime::traits::Saturating};
	use frame_system::pallet_prelude::*;
	use orml_traits::{arithmetic::Zero, LockIdentifier, MultiCurrency, MultiLockableCurrency};
	use pallet_proposal::ProposalIndex;

	pub const STAKING_ID: LockIdentifier = *b"staking ";
	pub const MAX_UNLOCKING_CHUNKS: usize = 32;

	type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;

	type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

	/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be
	/// unlocked.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct UnlockChunk<T: Config + orml_tokens::Config> {
		/// Amount of funds to be unlocked.
		#[codec(compact)]
		pub value: BalanceOf<T>,
		/// Block number at which point it'll be unlocked.
		#[codec(compact)]
		pub block: T::BlockNumber,
	}

	/// The ledger of a (bonded) stash.
	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
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
		pub unlocking: Vec<UnlockChunk<T>>,
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
				.collect();

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
	#[pallet::without_storage_info]
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

		type MaxProposals: Get<ProposalIndex>;
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			Ok(())
		}
	}
}
