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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::*,
		tokens::{Balance, Precision},
		Get, WithdrawReasons,
	},
};
use frame_system::{pallet_prelude::*, WeightInfo};

// use crate::types::LockType;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Bounded, Convert, One, Saturating, Zero},
	RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use traits::ReleaseSchedule;
use types::VestingInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod impls;
pub mod traits;
mod types;

pub type BalanceOf<T> = <T as Config>::Balance;
pub type ReasonOf<T> = <T as Config>::Reason;
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Actions to take against a user's `Vesting` storage entry.
#[derive(Clone, Copy)]
pub enum VestingAction {
	/// Do not actively remove any schedules.
	Passive,
	/// Remove the schedule specified by the index.
	Remove { index: usize },
	/// Remove the two schedules, specified by index, so they can be merged.
	Merge { index1: usize, index2: usize },
}

impl VestingAction {
	/// Whether or not the filter says the schedule index should be removed.
	fn should_remove(&self, index: usize) -> bool {
		match self {
			Self::Passive => false,
			Self::Remove { index: index1 } => *index1 == index,
			Self::Merge { index1, index2 } => *index1 == index || *index2 == index,
		}
	}

	/// Pick the schedules that this action dictates should continue vesting undisturbed.
	fn pick_schedules<T: Config>(
		&self,
		schedules: Vec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>>,
	) -> impl Iterator<Item = VestingInfo<BalanceOf<T>, BlockNumberFor<T>>> + '_ {
		schedules.into_iter().enumerate().filter_map(
			move |(index, schedule)| {
				if self.should_remove(index) {
					None
				} else {
					Some(schedule)
				}
			},
		)
	}
}

// Wrapper for `T::MAX_VESTING_SCHEDULES` to satisfy `trait Get`.
pub struct MaxVestingSchedulesGet<T>(PhantomData<T>);
impl<T: Config> Get<u32> for MaxVestingSchedulesGet<T> {
	fn get() -> u32 {
		T::MAX_VESTING_SCHEDULES
	}
}

/// Enable `dev_mode` for this pallet.
#[frame_support::pallet(dev_mode)]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type Balance: Balance + MaybeSerializeDeserialize;

		// TODO: Still I dont-like this. I want to be able to use the `LockType` from the pallet_balances, without coupling it.
		type Reason: Parameter + Copy + MaybeSerializeDeserialize;

		type Currency: InspectHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ MutateHold<AccountIdOf<Self>, Balance = BalanceOf<Self>, Reason = ReasonOf<Self>>
			+ BalancedHold<AccountIdOf<Self>, Balance = BalanceOf<Self>>
			+ Mutate<AccountIdOf<Self>, Balance = BalanceOf<Self>>;

		/// Convert the block number into a balance.
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		/// Reasons that determine under which conditions the balance may drop below
		/// the unvested amount.
		type UnvestedFundsAllowedWithdrawReasons: Get<WithdrawReasons>;

		/// The minimum amount transferred to call `vested_transfer`.
		#[pallet::constant]
		type MinVestedTransfer: Get<BalanceOf<Self>>;

		const MAX_VESTING_SCHEDULES: u32;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		#[pallet::constant_name(MaxVestingSchedules)]
		fn max_vesting_schedules() -> u32 {
			T::MAX_VESTING_SCHEDULES
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<(AccountIdOf<T>, T::BlockNumber, T::BlockNumber, BalanceOf<T>, ReasonOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { vesting: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			use sp_runtime::traits::Saturating;

			// Generate initial vesting configuration
			// * who - Account which we are generating vesting configuration for
			// * begin - Block when the account will start to vest
			// * length - Number of blocks from `begin` until fully vested
			// * liquid - Number of units which can be spent before vesting begins
			for &(ref who, begin, length, liquid, reason) in self.vesting.iter() {
				let balance = T::Currency::balance(who);
				assert!(!balance.is_zero(), "Currencies must be init'd before vesting");
				// Total genesis `balance` minus `liquid` equals funds locked for vesting
				let locked = balance.saturating_sub(liquid);
				let length_as_balance = T::BlockNumberToBalance::convert(length);
				let per_block = locked / length_as_balance.max(sp_runtime::traits::One::one());
				let vesting_info = VestingInfo::new(locked, per_block, begin);
				if !vesting_info.is_valid() {
					panic!("Invalid VestingInfo params at genesis")
				};

				Vesting::<T>::try_append(who, reason, vesting_info).expect("Too many vesting schedules at genesis.");

				T::Currency::hold(&reason, who, locked).map_err(|err| panic!("{:?}", err)).unwrap();
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			assert!(T::MAX_VESTING_SCHEDULES > 0, "`MaxVestingSchedules` must ge greater than 0");
		}
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The amount vested has been updated. This could indicate a change in funds available.
		/// The balance given is the amount which is left unvested (and thus locked).
		VestingUpdated {
			account: T::AccountId,
			unvested: BalanceOf<T>,
		},
		/// An \[account\] has become fully vested.
		VestingCompleted {
			account: T::AccountId,
		},
		// An \[account\] has reveived a vested transfer of \[amount\].
		VestingTransferred {
			to: T::AccountId,
			amount: BalanceOf<T>,
		},
	}

	/// Error for the vesting pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// The account given is not vesting.
		NotVesting,
		/// The account already has `MaxVestingSchedules` count of schedules and thus
		/// cannot add another one. Consider merging existing schedules in order to add another.
		AtMaxVestingSchedules,
		/// Amount being transferred is too low to create a vesting schedule.
		AmountLow,
		/// An index was out of bounds of the vesting schedules.
		ScheduleIndexOutOfBounds,
		/// Failed to create a new schedule because some parameter was invalid.
		InvalidScheduleParams,
	}

	/// Information regarding the vesting of a given account.
	#[pallet::storage]
	#[pallet::getter(fn vesting)]
	pub type Vesting<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		ReasonOf<T>,
		BoundedVec<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>, MaxVestingSchedulesGet<T>>,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Unlock any vested funds of the sender account, for the given `reason`.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(0)]
		pub fn vest(origin: OriginFor<T>, reason: ReasonOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_vest(who, reason)
		}

		/// Unlock any vested funds of a `target` account, for the given `reason`.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account whose vested funds should be unlocked. Must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(1)]
		pub fn vest_other(origin: OriginFor<T>, target: AccountIdOf<T>, reason: ReasonOf<T>) -> DispatchResult {
			ensure_signed(origin)?;
			Self::do_vest(target, reason)
		}

		/// Create a vested transfer.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account receiving the vested funds.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// NOTE: This will unlock all schedules through the current block.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(2)]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			target: AccountIdOf<T>,
			schedule: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
			reason: ReasonOf<T>,
		) -> DispatchResult {
			let transactor = ensure_signed(origin)?;
			Self::do_vested_transfer(transactor, target, schedule, reason)
		}

		/// Force a vested transfer.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `source`: The account whose funds should be transferred.
		/// - `target`: The account that should be transferred the vested funds.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// NOTE: This will unlock all schedules through the current block.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(3)]
		pub fn force_vested_transfer(
			origin: OriginFor<T>,
			source: AccountIdOf<T>,
			target: AccountIdOf<T>,
			schedule: VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
			reason: ReasonOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_vested_transfer(source, target, schedule, reason)
		}

		/// Merge two vesting schedules together, creating a new vesting schedule that unlocks over
		/// the highest possible start and end blocks. If both schedules have already started the
		/// current block will be used as the schedule start; with the caveat that if one schedule
		/// is finished by the current block, the other will be treated as the new merged schedule,
		/// unmodified.
		///
		/// NOTE: If `schedule1_index == schedule2_index` this is a no-op.
		/// NOTE: This will unlock all schedules through the current block prior to merging.
		/// NOTE: If both schedules have ended by the current block, no new schedule will be created
		/// and both will be removed.
		///
		/// Merged schedule attributes:
		/// - `starting_block`: `MAX(schedule1.starting_block, scheduled2.starting_block,
		///   current_block)`.
		/// - `ending_block`: `MAX(schedule1.ending_block, schedule2.ending_block)`.
		/// - `locked`: `schedule1.locked_at(current_block) + schedule2.locked_at(current_block)`.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `schedule1_index`: index of the first schedule to merge.
		/// - `schedule2_index`: index of the second schedule to merge.
		#[pallet::call_index(4)]
		pub fn merge_schedules(
			origin: OriginFor<T>,
			schedule1_index: u32,
			schedule2_index: u32,
			reason: ReasonOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if schedule1_index == schedule2_index {
				return Ok(())
			};
			let schedule1_index = schedule1_index as usize;
			let schedule2_index = schedule2_index as usize;
			// TODO: Add the different reasons check.

			let schedules = Self::vesting(&who, reason).ok_or(Error::<T>::NotVesting)?;
			let merge_action = VestingAction::Merge { index1: schedule1_index, index2: schedule2_index };

			let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), merge_action)?;

			Self::write_vesting_schedule(&who, schedules, reason)?;
			Self::write_release(&who, locked_now, reason)?;

			Ok(())
		}

		/// Unlock any vested funds of the sender account.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(5)]
		pub fn vest_all(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let reasons = <Vesting<T>>::iter_key_prefix(&who);
			for reason in reasons {
				Self::do_vest(who.clone(), reason)?;
			}
			Ok(())
		}

		/// Unlock any vested funds of a `target` account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account whose vested funds should be unlocked. Must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(6)]
		pub fn vest_all_other(origin: OriginFor<T>, target: AccountIdOf<T>) -> DispatchResult {
			ensure_signed(origin)?;
			let reasons = <Vesting<T>>::iter_key_prefix(&target);
			for reason in reasons {
				Self::do_vest(target.clone(), reason)?;
			}
			Ok(())
		}
	}
}
