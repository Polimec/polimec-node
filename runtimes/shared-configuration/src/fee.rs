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

use crate::{currency::MILLI_PLMC, Balance, StakingPalletId};
use frame_support::{
	ord_parameter_types,
	pallet_prelude::{InvalidTransaction, PhantomData, TransactionValidityError},
	parameter_types,
	sp_runtime::traits::{AccountIdConversion, DispatchInfoOf, PostDispatchInfoOf},
	traits::{
		fungible::{Balanced, Credit, Debt, Inspect},
		tokens::Precision,
		Imbalance, OnUnbalanced,
	},
	weights::{constants::ExtrinsicBaseWeight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
};
use pallet_transaction_payment::OnChargeTransaction;
use parachains_common::{AccountId, SLOT_DURATION};
use smallvec::smallvec;
use sp_arithmetic::{
	traits::{Saturating, Zero},
	Perbill,
};
use sp_std::prelude::*;

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		// extrinsic base weight (smallest non-zero weight) is mapped to 1/10 CENT:
		let p = 10 * MILLI_PLMC;
		let q = Balance::from(ExtrinsicBaseWeight::get().ref_time());
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
	pub const MaxAuthorities: u32 = 75;
}

ord_parameter_types! {
	pub const PayMaster: AccountId =
		AccountIdConversion::<AccountId>::into_account_truncating(&StakingPalletId::get());
}

/// Logic for the author to get a portion of fees.
pub struct ToAuthor<R>(sp_std::marker::PhantomData<R>);
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
pub struct ToStakingPot<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for ToStakingPot<R>
where
	R: pallet_balances::Config + pallet_parachain_staking::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_nonzero_unbalanced(amount: Credit<<R as frame_system::Config>::AccountId, pallet_balances::Pallet<R>>) {
		let staking_pot = PayMaster::get().into();
		let _ = <pallet_balances::Pallet<R>>::resolve(&staking_pot, amount);
	}
}

pub struct DealWithFees<R>(sp_std::marker::PhantomData<R>);
impl<R> OnUnbalanced<Credit<R::AccountId, pallet_balances::Pallet<R>>> for DealWithFees<R>
where
	R: pallet_balances::Config + pallet_authorship::Config + pallet_parachain_staking::Config,
	<R as frame_system::Config>::AccountId: From<AccountId>,
	<R as frame_system::Config>::AccountId: Into<AccountId>,
{
	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = Credit<R::AccountId, pallet_balances::Pallet<R>>>) {
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

/// Implements transaction payment for a pallet implementing the [`fungible`]
/// trait (eg. pallet_balances) using an unbalance handler (implementing
/// [`OnUnbalanced`]).
///
/// The unbalance handler is given 2 unbalanceds in [`OnUnbalanced::on_unbalanceds`]: `fee` and
/// then `tip`.
pub struct FungibleAdapter<F, OU>(PhantomData<(F, OU)>);

impl<T, F, OU> OnChargeTransaction<T> for FungibleAdapter<F, OU>
where
	T: pallet_transaction_payment::Config,
	F: Balanced<T::AccountId>,
	OU: OnUnbalanced<Credit<T::AccountId, F>>,
{
	type Balance = <F as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
	type LiquidityInfo = Option<Credit<T::AccountId, F>>;

	fn withdraw_fee(
		who: &<T>::AccountId,
		_call: &<T>::RuntimeCall,
		_dispatch_info: &DispatchInfoOf<<T>::RuntimeCall>,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if fee.is_zero() {
			return Ok(None);
		}

		// TODO: This is a temporary workaround.
		// As soon the linked PR is merged, the fungible trait will be updated and this can be safely removed.
		// This will probably be included in v1.6 or 1.7.
		// src: https://github.com/paritytech/polkadot-sdk/pull/2823
		if F::can_withdraw(who, fee) != frame_support::traits::tokens::WithdrawConsequence::Success {
			return Err(InvalidTransaction::Payment.into());
		};

		match F::withdraw(
			who,
			fee,
			Precision::Exact,
			frame_support::traits::tokens::Preservation::Preserve,
			frame_support::traits::tokens::Fortitude::Polite,
		) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	fn correct_and_deposit_fee(
		who: &<T>::AccountId,
		_dispatch_info: &DispatchInfoOf<<T>::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<<T>::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(corrected_fee);
			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = F::deposit(who, refund_amount, Precision::BestEffort)
				.unwrap_or_else(|_| Debt::<T::AccountId, F>::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid: Credit<T::AccountId, F> = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);
			OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
		}

		Ok(())
	}
}
