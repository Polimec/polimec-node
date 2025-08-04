use crate::{AccountIdOf, AssetId, BalanceOf, Config, Error, Pallet, PriceProviderOf, ReleaseType, Releases};
use frame_support::traits::{
	fungible,
	fungible::{Inspect, Mutate, MutateHold},
	fungibles,
	fungibles::Mutate as FungiblesMutate,
	tokens::{Fortitude, Precision, Preservation},
};
use frame_system::pallet_prelude::BlockNumberFor;
use polimec_common::ProvideAssetPrice;
use sp_runtime::{
	traits::{AccountIdConversion, Get},
	DispatchError, FixedPointNumber,
};

impl<T: Config> Pallet<T> {
	/// Calculate the USD fee in `fee_asset` for bonding `bond_amount` of the native token.
	/// e.g. if the fee is 1%, native token PLMC, fee_asset USDT, bond_amount 1000 PLMC, PLMC price 0.5USD, USDT price 1USD,
	/// Then the calculated fee would be 1% * 1000 * 0.5 = 5USD, which is 5 USDT at a price of 1USD.
	pub fn calculate_fee(bond_amount: BalanceOf<T>, fee_asset: AssetId) -> Result<BalanceOf<T>, DispatchError> {
		let bonding_token_price =
			<PriceProviderOf<T>>::get_decimals_aware_price(&T::BondingTokenId::get(), T::BondingTokenDecimals::get())
				.ok_or(Error::<T>::PriceNotAvailable)?;

		log::info!("Bonding token price: {:?}", bonding_token_price.clone());
		let fee_asset_decimals =
			<T::FeeToken as fungibles::metadata::Inspect<AccountIdOf<T>>>::decimals(fee_asset.clone());
		let fee_token_price = <PriceProviderOf<T>>::get_decimals_aware_price(&fee_asset, fee_asset_decimals)
			.ok_or(Error::<T>::PriceNotAvailable)?;

		log::info!("Fee asset price: {:?}", fee_token_price.clone());

		let bonded_in_usd = bonding_token_price.saturating_mul_int(bond_amount);
		log::info!("Bonded in USD: {:?}", bonded_in_usd.clone());
		let fee_in_usd = T::FeePercentage::get() * bonded_in_usd;
		let fee_in_fee_asset =
			fee_token_price.reciprocal().ok_or(Error::<T>::PriceNotAvailable)?.saturating_mul_int(fee_in_usd);

		Ok(fee_in_fee_asset)
	}

	pub fn get_bonding_account(derivation_path: u32) -> AccountIdOf<T> {
		// We need to add 1 since 0u32 means no derivation from root.
		T::RootId::get().into_sub_account_truncating(derivation_path.saturating_add(1u32))
	}

	/// Put some tokens on hold from the treasury into a sub-account, on behalf of a user.
	/// User pays a fee for this functionality, which can be later refunded.
	pub fn bond_on_behalf_of(
		derivation_path: u32,
		account: T::AccountId,
		bond_amount: BalanceOf<T>,
		fee_asset: AssetId,
		hold_reason: T::RuntimeHoldReason,
	) -> Result<(), DispatchError> {
		let treasury = T::Treasury::get();
		let bonding_account: AccountIdOf<T> = Self::get_bonding_account(derivation_path);
		let existential_deposit = <T::BondingToken as fungible::Inspect<T::AccountId>>::minimum_balance();

		let fee_in_fee_asset = Self::calculate_fee(bond_amount, fee_asset.clone())?;
		log::info!("Paid fee of {:?} {:?}", fee_in_fee_asset.clone(), fee_asset.clone());

		// Pay the fee from the user to the bonding account. It awaits either a full transfer to the T::FeeRecipient, or a refund to each user
		T::FeeToken::transfer(fee_asset, &account, &bonding_account, fee_in_fee_asset, Preservation::Preserve)?;

		// Ensure the sub-account has an ED by the treasury. This will be refunded after all the tokens are unlocked
		if T::BondingToken::balance(&bonding_account) < existential_deposit {
			T::BondingToken::transfer(&treasury, &bonding_account, existential_deposit, Preservation::Preserve)?;
		}
		// Bond the PLMC on behalf of the user
		T::BondingToken::transfer_and_hold(
			&hold_reason.into(),
			&treasury,
			&bonding_account,
			bond_amount,
			Precision::Exact,
			Preservation::Preserve,
			Fortitude::Polite,
		)?;

		Ok(())
	}

	/// Set the block for which we can release the bonds of a sub-account, and transfer it back to the treasury.
	pub fn set_release_type(
		derivation_path: u32,
		hold_reason: T::RuntimeHoldReason,
		release_type: ReleaseType<BlockNumberFor<T>>,
	) {
		Releases::<T>::insert(derivation_path, hold_reason, release_type);
	}

	/// Refund the fee paid by a user to lock up some treasury tokens. It is this function's caller responsibility to
	/// ensure that the fee should be refunded, and is not refunded twice
	pub fn refund_fee(
		derivation_path: u32,
		account: &T::AccountId,
		bond_amount: BalanceOf<T>,
		fee_asset: AssetId,
	) -> Result<(), DispatchError> {
		let bonding_account = Self::get_bonding_account(derivation_path);
		let fee_in_fee_asset = Self::calculate_fee(bond_amount, fee_asset.clone())?;

		// We know this fee token account is existing thanks to the provider reference of the ED of the native asset, so we can fully move all the funds.
		// FYI same cannot be said of the `account`. We assume they only hold the fee token so their fee asset balance must not go below the min_balance.
		T::FeeToken::transfer(fee_asset, &bonding_account, account, fee_in_fee_asset, Preservation::Expendable)?;

		Ok(())
	}
}
