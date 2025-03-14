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
extern crate alloc;
use crate::{currency::MILLI_PLMC, Balance, StakingPalletId};
use core::marker::PhantomData;
use frame_support::{
	ord_parameter_types,
	pallet_prelude::Weight,
	parameter_types,
	sp_runtime::traits::AccountIdConversion,
	traits::{
		fungible::{Balanced, Credit},
		fungibles,
		fungibles::Inspect,
		tokens::{
			ConversionToAssetBalance, Fortitude::Polite, Precision::Exact, Preservation::Protect, WithdrawConsequence,
		},
		Imbalance, OnUnbalanced,
	},
	weights::{
		constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
};
use pallet_asset_tx_payment::{HandleCredit, OnChargeAssetTransaction};
use pallet_transaction_payment::OnChargeTransaction;
use parachains_common::{impls::AccountIdOf, AccountId};
use scale_info::prelude::vec;
use smallvec::smallvec;
use sp_arithmetic::Perbill;
use sp_runtime::{
	traits::{DispatchInfoOf, Get, One, PostDispatchInfoOf, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidityError},
};

#[allow(clippy::module_name_repetitions)]
pub struct WeightToFee;
impl frame_support::weights::WeightToFee for WeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		let time_poly: FeePolynomial<Balance> = RefTimeToFee::polynomial().into();
		let proof_poly: FeePolynomial<Balance> = ProofSizeToFee::polynomial().into();

		// Take the maximum instead of the sum to charge by the more scarce resource.
		time_poly.eval(weight.ref_time()).max(proof_poly.eval(weight.proof_size()))
	}
}

/// Maps the reference time component of `Weight` to a fee.
#[allow(clippy::module_name_repetitions)]
pub struct RefTimeToFee;
impl WeightToFeePolynomial for RefTimeToFee {
	type Balance = Balance;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// In Kusama, extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		// The standard system parachain configuration is 1/10 of that, as in 1/100 CENT.
		let p = 10 * MILLI_PLMC;
		let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());

		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

/// Maps the proof size component of `Weight` to a fee.
#[allow(clippy::module_name_repetitions)]
pub struct ProofSizeToFee;
impl WeightToFeePolynomial for ProofSizeToFee {
	type Balance = Balance;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// Map 10kb proof to 1 CENT.
		let p = 10 * MILLI_PLMC;
		let q = 10_000;

		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

parameter_types! {
	pub const MaxAuthorities: u32 = 75;
}

ord_parameter_types! {
	pub const BlockchainOperationTreasury: AccountId =
		AccountIdConversion::<AccountId>::into_account_truncating(&StakingPalletId::get());
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for ToAuthor<R>
where
	R: pallet_balances::Config + pallet_authorship::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_nonzero_unbalanced(amount: Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>) {
		if let Some(author) = <pallet_authorship::Pallet<R>>::author() {
			let _ = <pallet_balances::Pallet<R>>::resolve(&author, amount);
		}
	}
}

/// Implementation of `OnUnbalanced` that deposits the fees into  the "Blockchain Operation Treasury" for later payout.
pub struct ToStakingPot<R>(PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for ToStakingPot<R>
where
	R: pallet_balances::Config + pallet_parachain_staking::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_nonzero_unbalanced(amount: Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>) {
		let staking_pot = BlockchainOperationTreasury::get().into();
		let _ = <pallet_balances::Pallet<R>>::resolve(&staking_pot, amount);
	}
}

pub struct DealWithFees<R>(PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_authorship::Config + pallet_parachain_staking::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_unbalanceds(mut fees_then_tips: impl Iterator<Item = Credit<R::AccountId, pallet_balances::Pallet<R>>>) {
		if let Some(fees) = fees_then_tips.next() {
			// for fees, 100% to treasury, 0% to author
			let mut split = fees.ration(100, 0);
			if let Some(tips) = fees_then_tips.next() {
				// for tips, if any, 100% to author
				tips.merge_into(&mut split.1);
			}
			<ToStakingPot<R> as OnUnbalanced<_>>::on_unbalanced(split.0);
			<ToAuthor<R> as OnUnbalanced<_>>::on_unbalanced(split.1);
		}
	}
}

type BalanceOf<T> = <<T as pallet_transaction_payment::Config>::OnChargeTransaction as OnChargeTransaction<T>>::Balance;
type AssetIdOf<T> = <<T as pallet_asset_tx_payment::Config>::Fungibles as Inspect<AccountIdOf<T>>>::AssetId;
type AssetBalanceOf<T> =
	<<T as pallet_asset_tx_payment::Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// Implements the asset transaction for a balance to asset converter (implementing
/// [`ConversionToAssetBalance`]) and 2 credit handlers (implementing [`HandleCredit`]).
///
/// First handler does the fee, second the tip.
pub struct TxFeeFungiblesAdapter<Converter, FeeCreditor, TipCreditor>(
	PhantomData<(Converter, FeeCreditor, TipCreditor)>,
);

/// Default implementation for a runtime instantiating this pallet, a balance to asset converter and
/// a credit handler.
impl<Runtime, Converter, FeeCreditor, TipCreditor> OnChargeAssetTransaction<Runtime>
	for TxFeeFungiblesAdapter<Converter, FeeCreditor, TipCreditor>
where
	Runtime: pallet_asset_tx_payment::Config,
	Runtime::Fungibles: Inspect<AccountIdOf<Runtime>, AssetId = xcm::v4::Location>,
	Converter: ConversionToAssetBalance<BalanceOf<Runtime>, AssetIdOf<Runtime>, AssetBalanceOf<Runtime>>,
	FeeCreditor: HandleCredit<Runtime::AccountId, Runtime::Fungibles>,
	TipCreditor: HandleCredit<Runtime::AccountId, Runtime::Fungibles>,
{
	// Note: We stick to `v3::MultiLocation`` because `v4::Location`` doesn't implement `Copy`.
	type AssetId = xcm::v3::MultiLocation;
	type Balance = BalanceOf<Runtime>;
	type LiquidityInfo = fungibles::Credit<Runtime::AccountId, Runtime::Fungibles>;

	/// Ensure payment of the transaction fees can be withdrawn.
	///
	/// Note: The `fee` already includes the `tip`.
	fn can_withdraw_fee(
		who: &Runtime::AccountId,
		_call: &Runtime::RuntimeCall,
		_info: &DispatchInfoOf<Runtime::RuntimeCall>,
		asset_id: Self::AssetId,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<(), TransactionValidityError> {
		let asset_id: xcm::v4::Location =
			asset_id.try_into().map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		let min_converted_fee = if fee.is_zero() { Zero::zero() } else { One::one() };
		let converted_fee = Converter::to_asset_balance(fee, asset_id.clone())
			.map_err(|_| TransactionValidityError::from(InvalidTransaction::Payment))?
			.max(min_converted_fee);

		// Ensure we can withdraw enough `asset_id` for the swap.
		match <Runtime::Fungibles as fungibles::Inspect<Runtime::AccountId>>::can_withdraw(
			asset_id.clone(),
			who,
			converted_fee,
		) {
			WithdrawConsequence::BalanceLow |
			WithdrawConsequence::UnknownAsset |
			WithdrawConsequence::Underflow |
			WithdrawConsequence::Overflow |
			WithdrawConsequence::Frozen => return Err(TransactionValidityError::from(InvalidTransaction::Payment)),
			WithdrawConsequence::Success | WithdrawConsequence::ReducedToZero(_) | WithdrawConsequence::WouldDie => {},
		};

		Ok(())
	}

	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &Runtime::AccountId,
		_call: &Runtime::RuntimeCall,
		_info: &DispatchInfoOf<Runtime::RuntimeCall>,
		asset_id: Self::AssetId,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// We don't know the precision of the underlying asset. Because the converted fee could be
		// less than one (e.g. 0.5) but gets rounded down by integer division we introduce a minimum
		// fee.
		let asset_id: xcm::v4::Location =
			asset_id.try_into().map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		let min_converted_fee = if fee.is_zero() { Zero::zero() } else { One::one() };
		let converted_fee = Converter::to_asset_balance(fee, asset_id.clone())
			.map_err(|_| TransactionValidityError::from(InvalidTransaction::Payment))?
			.max(min_converted_fee);
		let can_withdraw =
			<Runtime::Fungibles as Inspect<Runtime::AccountId>>::can_withdraw(asset_id.clone(), who, converted_fee);
		if can_withdraw != WithdrawConsequence::Success {
			return Err(InvalidTransaction::Payment.into())
		}
		<Runtime::Fungibles as fungibles::Balanced<Runtime::AccountId>>::withdraw(
			asset_id,
			who,
			converted_fee,
			Exact,
			Protect,
			Polite,
		)
		.map_err(|_| TransactionValidityError::from(InvalidTransaction::Payment))
	}

	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &Runtime::AccountId,
		_dispatch_info: &DispatchInfoOf<Runtime::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<Runtime::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		paid: Self::LiquidityInfo,
	) -> Result<(AssetBalanceOf<Runtime>, AssetBalanceOf<Runtime>), TransactionValidityError> {
		let min_converted_fee = if corrected_fee.is_zero() { Zero::zero() } else { One::one() };
		// Convert the corrected fee and tip into the asset used for payment.
		let converted_fee = Converter::to_asset_balance(corrected_fee, paid.asset())
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })?
			.max(min_converted_fee);
		let converted_tip = Converter::to_asset_balance(tip, paid.asset())
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })?;

		// Calculate how much refund we should return.
		let (final_fee, refund) = paid.split(converted_fee);
		// Split the tip from the fee
		let (final_tip, final_fee_minus_tip) = final_fee.split(converted_tip);

		let _ = <Runtime::Fungibles as fungibles::Balanced<Runtime::AccountId>>::resolve(who, refund);

		FeeCreditor::handle_credit(final_fee_minus_tip);
		TipCreditor::handle_credit(final_tip);

		Ok((converted_fee, converted_tip))
	}
}

pub struct CreditFungiblesToAccount<AccountId, Assets, Account>(PhantomData<(AccountId, Assets, Account)>);
impl<AccountId, Assets: frame_support::traits::fungibles::Balanced<AccountId>, Account: Get<AccountId>>
	HandleCredit<AccountId, Assets> for CreditFungiblesToAccount<AccountId, Assets, Account>
{
	fn handle_credit(credit: fungibles::Credit<AccountId, Assets>) {
		let payee: AccountId = Account::get();
		let _ = <Assets as fungibles::Balanced<AccountId>>::resolve(&payee, credit);
	}
}
