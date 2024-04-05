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

pub use frame_support::{traits::{Currency, ExistenceRequirement, tokens::{Balance, currency::VestingSchedule}}};
pub use polimec_common::credentials::{Did, EnsureOriginWithCredentials, InvestorType, UntrustedToken };
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
type CurrencyOf<T> = <<T as Config>::VestingSchedule as VestingSchedule<
	AccountIdOf<T>,
>>::Currency;
#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_support::{pallet_prelude::{ValueQuery, *}, PalletId};
    use sp_runtime::traits::AccountIdConversion;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
        /// The Origin that has admin access to change the claiming amount.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;


        type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

        /// The faucet's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

        #[pallet::constant]
        type LockPeriod: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type VestPeriod: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type InitialClaimAmount: Get<BalanceOf<Self>>;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type InvestorOrigin: EnsureOriginWithCredentials<
            <Self as frame_system::Config>::RuntimeOrigin,
            Success = (AccountIdOf<Self>, Did, InvestorType),
        >;

        type VestingSchedule: VestingSchedule<Self::AccountId, Moment = BlockNumberFor<Self>>;
        /// The Ed25519 Verifier Public Key to verify the signature of the credentials.
		#[pallet::constant]
		type VerifierPublicKey: Get<[u8; 32]>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: crate::weights::WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type ClaimAmount<T> = StorageValue<_, BalanceOf<T>, ValueQuery, <T as Config>::InitialClaimAmount>;

    #[pallet::storage]
    pub type Claims<T> = StorageMap<_, Blake2_128Concat, Did, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Claimed{
            claimer_did: Did, 
            claimer: T::AccountId,
            amount: BalanceOf<T>,
        },
        ClaimAmountChanged(BalanceOf<T>),
	}

	
	#[pallet::error]
	pub enum Error<T> {
		DidAlreadyClaimed,
		FaucetDepleted
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}


	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::feeless_if( | origin: &OriginFor<T>, jwt: &UntrustedToken | -> bool { 
            if let Ok((_, did, _)) = T::InvestorOrigin::ensure_origin(origin.clone(), jwt, T::VerifierPublicKey::get()) {
                Claims::<T>::get(did).is_none()
            } else {
                false
            }
         })]
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn claim(origin: OriginFor<T>, jwt: UntrustedToken) -> DispatchResultWithPostInfo {
			
			let (who, did, _investor_type) = T::InvestorOrigin::ensure_origin(origin, &jwt, T::VerifierPublicKey::get())?;
            ensure!(Claims::<T>::get(&did).is_none(), Error::<T>::DidAlreadyClaimed);

            let amount = ClaimAmount::<T>::get();
            ensure!(CurrencyOf::<T>::free_balance(&Self::claiming_account()) >= amount, Error::<T>::FaucetDepleted);
            
            let current_block = <frame_system::Pallet<T>>::block_number();
            let length_as_balance = T::BlockNumberToBalance::convert(T::VestPeriod::get());
			let per_block = amount / length_as_balance.max(sp_runtime::traits::One::one());

            T::VestingSchedule::can_add_vesting_schedule(
                &who,
                amount,
                per_block,
                current_block + T::LockPeriod::get(),
            )?;

            <CurrencyOf<T>>::transfer(&Self::claiming_account(), &who, amount, ExistenceRequirement::AllowDeath)?;
            T::VestingSchedule::add_vesting_schedule(
                &who,
                amount,
                per_block,
                current_block + T::LockPeriod::get(),   
            )?;


            Claims::<T>::insert(did.clone(), ());
			Self::deposit_event(Event::Claimed { claimer_did: did, claimer: who, amount: amount });
			
            Ok(Pays::No.into())
		}

        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
        pub fn set_claiming_amount(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
            T::AdminOrigin::ensure_origin(origin)?;
            ClaimAmount::<T>::put(amount);
            Self::deposit_event(Event::ClaimAmountChanged(amount));
            Ok(Pays::No.into())
        }
	}

    impl<T: Config> Pallet<T> {
        pub fn claiming_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }
    }
}
