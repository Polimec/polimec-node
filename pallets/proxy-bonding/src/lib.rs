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

#![cfg_attr(not(feature = "std"), no_std)]
// Needed due to empty sections raising the warning
#![allow(unreachable_patterns)]
pub use pallet::*;

mod functions;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::{Weight, *},
		traits::{
			fungible,
			fungible::{Mutate, MutateHold},
			fungibles,
			fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
			tokens::{Precision, Preservation},
		},
	};
	use frame_system::pallet_prelude::*;
	use polimec_common::ProvideAssetPrice;
	use sp_runtime::{
		traits::{AccountIdConversion},
		Perbill, TypeId,
	};

	pub type AssetId = u32;
	pub type BalanceOf<T> = <<T as Config>::BondingToken as fungible::Inspect<AccountIdOf<T>>>::Balance;
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type HoldReasonOf<T> = <<T as Config>::BondingToken as fungible::InspectHold<AccountIdOf<T>>>::Reason;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type RuntimeHoldReason: IsType<HoldReasonOf<Self>> + Parameter + MaxEncodedLen;

		/// The pallet giving access to the bonding token
		type BondingToken: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::MutateHold<Self::AccountId>;

		type BondingTokenDecimals: Get<u8>;
		type UsdDecimals: Get<u8>;
		type BondingTokenId: Get<AssetId>;

		/// The pallet giving access to fee-paying assets, like USDT
		type FeeToken: fungibles::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>
			+ fungibles::Mutate<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>
			+ fungibles::metadata::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>;

		type FeePercentage: Get<Perbill>;

		/// Method to get the price of an asset like USDT or PLMC. Likely to come from an oracle
		type PriceProvider: ProvideAssetPrice<AssetId = u32>;

		/// The account holding the tokens to be bonded. Normally the treasury
		type Treasury: Get<Self::AccountId>;

		/// The account receiving the fees
		type FeeRecipient: Get<Self::AccountId>;

		/// The id type that can generate sub-accounts
		type Id: Encode + Decode + TypeId;

		/// The root id used to derive sub-accounts. These sub-accounts will be used to bond the tokens
		type RootId: Get<Self::Id>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum ReleaseType<BlockNumber> {
		/// The bonded tokens are immediately sent back to the treasury, and fees await refunding
		Refunded,
		/// The bonded tokens are locked until the block number, and the fees can be immediately sent to the [fee recipient](Config::FeeRecipient)
		Locked(BlockNumber),
	}

	/// Maps at which block can we release the bonds of a sub-account
	#[pallet::storage]
	pub type Releases<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Blake2_128Concat,
		T::RuntimeHoldReason,
		ReleaseType<BlockNumberFor<T>>,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		BondsTransferredBackToTreasury { bond_amount: BalanceOf<T> },
		FeesTransferredToFeeRecipient { fee_asset: AssetId, fee_amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The release type for the given derivation path / hold reason is not set
		ReleaseTypeNotSet,
		/// Tried to unlock the native tokens and send them back to the treasury, but the release type configured a later unlock block.
		TooEarlyToUnlock,
		/// The release type for the given derivation path / hold reason is set to refunded, which disallows sending fees to the recipient
		FeeToRecipientDisallowed,

	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// If sub-account has all the tokens unbonded, it will transfer everything including ED back to the treasury
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())]
		pub fn transfer_bonds_back_to_treasury(
			origin: OriginFor<T>,
			derivation_path: u32,
			hold_reason: T::RuntimeHoldReason,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			let treasury = T::Treasury::get();
			let bonding_account: AccountIdOf<T> = T::RootId::get().into_sub_account_truncating(derivation_path);
			let now = frame_system::Pallet::<T>::block_number();

			let release_block =
				match Releases::<T>::get(derivation_path, hold_reason.clone()).ok_or(Error::<T>::ReleaseTypeNotSet)? {
					ReleaseType::Locked(release_block) => release_block,
					ReleaseType::Refunded => now,
				};

			ensure!(release_block <= now, Error::<T>::TooEarlyToUnlock);

			let transfer_to_treasury_amount =
				T::BondingToken::release_all(&hold_reason.into(), &bonding_account, Precision::BestEffort)?;

			T::BondingToken::transfer(
				&bonding_account,
				&treasury,
				transfer_to_treasury_amount,
				Preservation::Expendable,
			)?;

			Self::deposit_event(Event::BondsTransferredBackToTreasury { bond_amount: transfer_to_treasury_amount });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::zero())]
		pub fn transfer_fees_to_recipient(
			origin: OriginFor<T>,
			derivation_path: u32,
			hold_reason: T::RuntimeHoldReason,
			fee_asset: AssetId,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;
			let fee_recipient = T::FeeRecipient::get();
			let bonding_account: AccountIdOf<T> = T::RootId::get().into_sub_account_truncating(derivation_path);
			let release_type = Releases::<T>::get(derivation_path, hold_reason).ok_or(Error::<T>::ReleaseTypeNotSet)?;
			ensure!(release_type != ReleaseType::Refunded, Error::<T>::FeeToRecipientDisallowed);

			let fees_balance = T::FeeToken::balance(fee_asset, &bonding_account);
			T::FeeToken::transfer(fee_asset, &bonding_account, &fee_recipient, fees_balance, Preservation::Expendable)?;

			Self::deposit_event(Event::FeesTransferredToFeeRecipient { fee_asset, fee_amount: fees_balance });

			Ok(())
		}
	}
}
