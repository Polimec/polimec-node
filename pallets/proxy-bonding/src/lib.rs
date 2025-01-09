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

//! # Proxy Bonding Pallet
//!
//! A FRAME pallet that facilitates token bonding operations with fee management capabilities. This pallet allows users to bond tokens from a configurable account (we call Treasury) while paying fees in various assets.
//! This pallet is intended to be used as an alternative to a direct bonding mechanism. In this way, the user does not need to own or hold the tokens, but can still participate in various activities by paying a fee.
//!
//! ## Overview
//!
//! The Bonding Pallet provides functionality to:
//! - Bond treasury tokens on behalf of users
//! - Pay a bonding fee in different assets (e.g., DOT)
//! - Set the bond release to either immediate refund or time-locked release
//!
//! ## Features
//!
//! ### Token Bonding
//! - Bond tokens from a treasury account into sub-accounts
//! - Support for existential deposit management
//! - Hold-based bonding mechanism using runtime-defined hold reasons
//!
//! ### Fee Management
//! - Accept fees in configurable assets (e.g., DOT)
//! - Calculate fees based on bond amount and current token prices
//! - Support both fee refunds and fee transfers to recipients
//! - Percentage-based fee calculation in USD terms
//!
//! ### Release Mechanisms
//! Two types of release mechanisms are supported:
//! - Immediate refund: Bonds can be immediately returned to treasury, and fees await refunding to users.
//! - Time-locked release: Bonds are locked until a specific block number, and fees can be sent to the configured fee recipient.
//!
//! ## Configuration
//! [`Config`](crate::pallet::Config)
//!
//! ### Example Configuration (Similar on how it's configured on the Polimec Runtime)
//!
//! ```rust
//! parameter_types! {
//! 	// Fee is defined as 1.5% of the USD Amount. Since fee is applied to the PLMC amount, and that is always 5 times
//! 	// less than the usd_amount (multiplier of 5), we multiply the 1.5 by 5 to get 7.5%
//! 	pub FeePercentage: Perbill = Perbill::from_rational(75u32, 1000u32);
//! 	pub FeeRecipient: AccountId =  AccountId::from(hex_literal::hex!("3ea952b5fa77f4c67698e79fe2d023a764a41aae409a83991b7a7bdd9b74ab56"));
//! 	pub RootId: PalletId = PalletId(*b"treasury");
//! }
//!
//! impl pallet_proxy_bonding::Config for Runtime {
//! 	type BondingToken = Balances; // The Balances pallet is used for the bonding token
//! 	type BondingTokenDecimals = ConstU8<10>; // The PLMC token has 10 decimals
//! 	type BondingTokenId = ConstU32<X>; // TODO: Replace with a proper number and explanation.
//! 	type FeePercentage = FeePercentage; // The fee kept by the treasury
//! 	type FeeRecipient = FeeRecipient; // THe account that receives the fee
//! 	type FeeToken = ForeignAssets; // The Asset pallet is used for the fee token
//! 	type Id = PalletId; // The ID type used for the ... account
//! 	type PriceProvider = OraclePriceProvider<AssetId, Price, Oracle>; // The Oracle pallet is used for the price provider
//! 	type RootId = TreasuryId; // The treasury account ID
//! 	type Treasury = TreasuryAccount; // The treasury account
//! 	type UsdDecimals = ConstU8<X>; // TODO: Replace with a proper number and explanation.
//! 	type RuntimeEvent = RuntimeEvent;
//! 	type RuntimeHoldReason = RuntimeHoldReason;
//! }
//! ```
//!
//!
//! ## Extrinsics
//! [`transfer_bonds_back_to_treasury`](crate::pallet::Pallet::transfer_bonds_back_to_treasury)
//! [`transfer_fees_to_recipient`](crate::pallet::Pallet::transfer_fees_to_recipient)
//!
//! ## Public Functions
//! [`calculate_fee`](crate::pallet::Pallet::calculate_fee)
//! [`get_bonding_account`](crate::pallet::Pallet::get_bonding_account)
//! [`bond_on_behalf_of`](crate::pallet::Pallet::bond_on_behalf_of)
//! [`set_release_type`](crate::pallet::Pallet::set_release_type)
//! [`refund_fee`](crate::pallet::Pallet::refund_fee)
//!
//!
//! ### transfer_bonds_back_to_treasury
//! Transfer bonded tokens back to the treasury when release conditions are met.
//!
//! Parameters:
//! - `derivation_path`: The sub-account derivation path
//! - `hold_reason`: The reason for the hold
//! - `origin`: Signed origin
//!
//! ### transfer_fees_to_recipient
//! Transfer collected fees to the designated fee recipient.
//!
//! Parameters:
//! - `derivation_path`: The sub-account derivation path
//! - `hold_reason`: The reason for the hold
//! - `fee_asset`: The asset ID of the fee token
//! - `origin`: Signed origin
//!
//! ## Public Functions
//!
//! ### bond_on_behalf_of
//! Bonds tokens from the treasury into a sub-account on behalf of a user.
//!
//! Parameters:
//! - `derivation_path`: Sub-account derivation path
//! - `account`: Account ID of the user
//! - `bond_amount`: Amount of tokens to bond
//! - `fee_asset`: Asset ID of the fee token
//! - `hold_reason`: Reason for the hold
//!
//! ### calculate_fee
//! Calculates the fee amount in the specified fee asset based on the bond amount.
//!
//! ### refund_fee
//! Refunds the fee to the specified account.
//!
//! ## Events
//!
//! - `BondsTransferredBackToTreasury`: Emitted when bonds are transferred back to treasury
//! - `FeesTransferredToFeeRecipient`: Emitted when fees are transferred to the fee recipient
//!
//! ## Errors
//!
//! - `ReleaseTypeNotSet`: Release type not configured for the given derivation path/hold reason
//! - `TooEarlyToUnlock`: Attempted to unlock tokens before the configured release block
//! - `FeeToRecipientDisallowed`: Fee transfer to recipient not allowed for refunded release type
//! - `FeeRefundDisallowed`: Fee refund not allowed for locked release type
//! - `PriceNotAvailable`: Price information unavailable for fee calculation
//!
//! ## Example integration
//!
//! The Proxy Bonding Pallet work seamlessly with the Funding Pallet to handle OTM (One-Token-Model) participation modes in project funding. Here's how the integration works:
//!
//! ### Contribution Flow
//! 1. When a user contributes to a project using OTM mode:
//! - The Funding Pallet calls `bond_on_behalf_of` with:
//! - Project ID as the derivation path
//! - User's account
//! - PLMC bond amount
//! - Funding asset ID
//! - Participation hold reason
//!
//! 2. During project settlement phase:
//! - For successful projects:
//! - An OTM release type is set with a time-lock based on the multiplier
//! - Bonds remain locked until the vesting duration completes
//! - For failed projects:
//! - Release type is set to `Refunded`
//! - Allows immediate return of bonds to treasury
//! - Enables fee refunds to participants
//!
//! ### Key Interactions
//! ```rust
//! // In Funding Pallet
//! pub fn bond_plmc_with_mode(
//! 	who: &T::AccountId,
//! 	project_id: ProjectId,
//! 	amount: Balance,
//! 	mode: ParticipationMode,
//! 	asset: AcceptedFundingAsset,
//! ) -> DispatchResult {
//! 	match mode {
//! 		ParticipationMode::OTM => pallet_proxy_bonding::Pallet::<T>::bond_on_behalf_of(
//! 			project_id,
//! 			who.clone(),
//! 			amount,
//! 			asset.id(),
//! 			HoldReason::Participation.into(),
//! 		),
//! 		ParticipationMode::Classic(_) => // ... other handling
//! 	}
//! }
//! ```
//!
//! ### Settlement Process
//! The settlement process determines the release conditions for bonded tokens:
//! - Success: Tokens remain locked with a time-based release schedule
//! - Failure: Tokens are marked for immediate return to treasury with fee refunds
//!
//! ## License
//!
//! License: GPL-3.0

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
	use sp_runtime::{Perbill, TypeId};

	pub type AssetId = u32;
	pub type BalanceOf<T> = <<T as Config>::BondingToken as fungible::Inspect<AccountIdOf<T>>>::Balance;
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type HoldReasonOf<T> = <<T as Config>::BondingToken as fungible::InspectHold<AccountIdOf<T>>>::Reason;
	pub type PriceProviderOf<T> = <T as Config>::PriceProvider;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching hold reason generated by `construct_runtime`. This is used for the bonding.
		type RuntimeHoldReason: IsType<HoldReasonOf<Self>> + Parameter + MaxEncodedLen;

		/// The pallet giving access to the bonding token
		type BondingToken: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::MutateHold<Self::AccountId>;

		/// The number of decimals one unit of the bonding token has. Used to calculate decimal aware prices.
		#[pallet::constant]
		type BondingTokenDecimals: Get<u8>;

		/// The number of decimals one unit of USD has. Used to calculate decimal aware prices. USD is not a real asset, but a reference point.
		#[pallet::constant]
		type UsdDecimals: Get<u8>;

		/// The id of the bonding token. Used to get the price of the bonding token.
		#[pallet::constant]
		type BondingTokenId: Get<AssetId>;

		/// The pallet giving access to fee-paying assets, like USDT
		type FeeToken: fungibles::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>
			+ fungibles::Mutate<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>
			+ fungibles::metadata::Inspect<Self::AccountId, Balance = BalanceOf<Self>, AssetId = AssetId>;

		/// The percentage of the bonded amount in USD that will be taken as a fee in the fee asset.
		#[pallet::constant]
		type FeePercentage: Get<Perbill>;

		/// Method to get the price of an asset like USDT or PLMC. Likely to come from an oracle
		type PriceProvider: ProvideAssetPrice<AssetId = u32>;

		/// The account holding the tokens to be bonded. Normally the treasury
		#[pallet::constant]
		type Treasury: Get<Self::AccountId>;

		/// The account receiving the fees
		#[pallet::constant]
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
		/// Tried to unlock the native tokens and send them back to the treasury, but the release is configured for a later block.
		TooEarlyToUnlock,
		/// The release type for the given derivation path / hold reason is set to `Refunded`, which disallows sending fees to the recipient
		FeeToRecipientDisallowed,
		/// The release type for the given derivation path / hold reason is set to `Locked`, which disallows refunding fees
		FeeRefundDisallowed,
		/// The price of a fee asset or the native token could not be retrieved
		PriceNotAvailable,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer bonded tokens back to the treasury if conditions are met.
		///
		/// # Description
		/// This extrinsic allows transferring bonded tokens back to the treasury account when either:
		/// - The release block number has been reached for time-locked bonds
		/// - Or immediately if the release type is set to `Refunded`
		/// 
		/// The function will release all tokens held under the specified hold reason and transfer them,
		/// including the existential deposit, back to the treasury account.
		/// If sub-account has all the tokens unbonded, it will transfer everything including ED back to the treasury
		///
		/// # Parameters
		/// * `origin` - The origin of the call. Must be signed. Can be anyone.
		/// * `derivation_path` - The derivation path used to calculate the bonding sub-account
		/// * `hold_reason` - The reason for which the tokens were held
		///
		/// # Errors
		/// * [`Error::ReleaseTypeNotSet`] - If no release type is configured for the given derivation path and hold reason
		/// * [`Error::TooEarlyToUnlock`] - If the current block is before the configured release block for locked bonds
		///
		/// # Events
		/// * [`Event::BondsTransferredBackToTreasury`] - When tokens are successfully transferred back to treasury
		///
		/// ```
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())]
		pub fn transfer_bonds_back_to_treasury(
			origin: OriginFor<T>,
			derivation_path: u32,
			hold_reason: T::RuntimeHoldReason,
		) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			let treasury = T::Treasury::get();
			let bonding_account = Self::get_bonding_account(derivation_path);
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

		/// Transfer collected fees to the designated fee recipient.
		///
		/// # Description
		/// This extrinsic transfers all collected fees in the specified fee asset from the bonding 
		/// sub-account to the configured fee recipient. This operation is only allowed when the 
		/// release type is set to `Locked`, indicating that the bonds are being held legitimately
		/// rather than awaiting refund.
		///
		/// # Parameters
		/// * `origin` - The origin of the call. Must be signed. Can be anyone.
		/// * `derivation_path` - The derivation path used to calculate the bonding sub-account
		/// * `hold_reason` - The reason for which the tokens were held
		/// * `fee_asset` - The asset ID of the fee token to transfer
		///
		/// # Errors
		/// * [`Error::ReleaseTypeNotSet`] - If no release type is configured for the given derivation path and hold reason
		/// * [`Error::FeeToRecipientDisallowed`] - If the release type is set to `Refunded`, which means fees should be refunded instead
		///
		/// # Events
		/// * [`Event::FeesTransferredToFeeRecipient`] - When fees are successfully transferred to the recipient
		///
		/// ```
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
			let bonding_account = Self::get_bonding_account(derivation_path);
			let release_type = Releases::<T>::get(derivation_path, hold_reason).ok_or(Error::<T>::ReleaseTypeNotSet)?;
			ensure!(release_type != ReleaseType::Refunded, Error::<T>::FeeToRecipientDisallowed);

			let fees_balance = T::FeeToken::balance(fee_asset, &bonding_account);
			T::FeeToken::transfer(fee_asset, &bonding_account, &fee_recipient, fees_balance, Preservation::Expendable)?;

			Self::deposit_event(Event::FeesTransferredToFeeRecipient { fee_asset, fee_amount: fees_balance });

			Ok(())
		}
	}
}
