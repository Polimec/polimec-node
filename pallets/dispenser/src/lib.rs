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

// If you feel like getting in touch with us, you can do so at info@polimec.org

#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

pub use frame_support::traits::{
	tokens::{currency::VestingSchedule, Balance},
	Currency, ExistenceRequirement,
};
pub use polimec_common::credentials::{Did, EnsureOriginWithCredentials, InvestorType, UntrustedToken};
pub use sp_runtime::traits::Convert;

pub mod extensions;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type BalanceOf<T> = <CurrencyOf<T> as Currency<AccountIdOf<T>>>::Balance;
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<AccountIdOf<T>>>::Currency;
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, CheckedDiv},
		Saturating,
	};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The Origin that has admin access to change the dispense amount.
		type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Block to balance converter.
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		/// The amount of dispensed tokens that are free, so they could be used to pay for
		/// future transaction fees.
		#[pallet::constant]
		type FreeDispenseAmount: Get<BalanceOf<Self>>;

		/// The amount of tokens that are initially dispensed from the dispenser.
		#[pallet::constant]
		type InitialDispenseAmount: Get<BalanceOf<Self>>;

		/// The Origin that can dispense funds from the dispenser. The Origin must contain a valid JWT token.
		type InvestorOrigin: EnsureOriginWithCredentials<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = (AccountIdOf<Self>, Did, InvestorType),
		>;

		/// The period of time that the dispensed funds are locked. Used to calculate the
		/// starting block of the vesting schedule.
		#[pallet::constant]
		type LockPeriod: Get<BlockNumberFor<Self>>;

		/// The dispenser's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The loose coupling to a vesting schedule implementation.
		type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;

		/// The period of time that the dispensed funds are in a vesting schedule. The schedule
		/// starts after the lock period.
		#[pallet::constant]
		type VestPeriod: Get<BlockNumberFor<Self>>;

		/// The Ed25519 Verifier Public Key to verify the signature of the credentials.
		#[pallet::constant]
		type VerifierPublicKey: Get<[u8; 32]>;

		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: crate::weights::WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type DispenseAmount<T> = StorageValue<_, BalanceOf<T>, ValueQuery, <T as Config>::InitialDispenseAmount>;

	#[pallet::storage]
	pub type Dispensed<T> = StorageMap<_, Blake2_128Concat, Did, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Dispensed { dispensed_to_did: Did, dispensed_to: T::AccountId, amount: BalanceOf<T> },
		DispenseAmountChanged(BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The dispenser has already dispensed to the DID.
		DispensedAlreadyToDid,
		/// The dispenser account does not have any funds to distribute.
		DispenserDepleted,
		/// The dispense amount is too low. It must be greater than the free dispense amount.
		DispenseAmountTooLow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::feeless_if( | origin: &OriginFor<T>, jwt: &UntrustedToken | -> bool {
            if let Ok((_, did, _)) = T::InvestorOrigin::ensure_origin(origin.clone(), jwt, T::VerifierPublicKey::get()) {
                return Dispensed::<T>::get(did).is_none()
            } else {
                return false
            }
        })]
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::dispense())]
		pub fn dispense(origin: OriginFor<T>, jwt: UntrustedToken) -> DispatchResultWithPostInfo {
			let (who, did, _investor_type) =
				T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
			ensure!(Dispensed::<T>::get(&did).is_none(), Error::<T>::DispensedAlreadyToDid);

			let amount = DispenseAmount::<T>::get();
			ensure!(CurrencyOf::<T>::free_balance(&Self::dispense_account()) >= amount, Error::<T>::DispenserDepleted);

			let current_block = <frame_system::Pallet<T>>::block_number();
			let length_as_balance = T::BlockNumberToBalance::convert(T::VestPeriod::get());
			let locked_amount = amount.saturating_sub(T::FreeDispenseAmount::get());
			let per_block = locked_amount
				.checked_div(&length_as_balance.max(sp_runtime::traits::One::one()))
				.ok_or(DispatchError::Arithmetic(sp_runtime::ArithmeticError::Underflow))?;

			T::VestingSchedule::can_add_vesting_schedule(
				&who,
				locked_amount,
				per_block,
				current_block + T::LockPeriod::get(),
			)?;

			<CurrencyOf<T>>::transfer(&Self::dispense_account(), &who, amount, ExistenceRequirement::AllowDeath)?;
			T::VestingSchedule::add_vesting_schedule(
				&who,
				locked_amount,
				per_block,
				current_block + T::LockPeriod::get(),
			)?;

			Dispensed::<T>::insert(did.clone(), ());
			Self::deposit_event(Event::Dispensed { dispensed_to_did: did, dispensed_to: who, amount });

			Ok(Pays::No.into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::set_dispense_amount())]
		pub fn set_dispense_amount(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
			T::AdminOrigin::ensure_origin(origin)?;
			ensure!(amount > T::FreeDispenseAmount::get(), Error::<T>::DispenseAmountTooLow);
			DispenseAmount::<T>::put(amount);
			Self::deposit_event(Event::DispenseAmountChanged(amount));
			Ok(Pays::No.into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn dispense_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
}
