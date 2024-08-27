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
	pallet_prelude::Weight,
	parameter_types,
	sp_runtime::traits::AccountIdConversion,
	traits::{
		fungible::{Balanced, Credit},
		Imbalance, OnUnbalanced,
	},
	weights::{
		constants::ExtrinsicBaseWeight, FeePolynomial, WeightToFeeCoefficient, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
};
use parachains_common::{AccountId, SLOT_DURATION};
use scale_info::prelude::vec;
use smallvec::smallvec;
use sp_arithmetic::Perbill;

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
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
	pub const MaxAuthorities: u32 = 75;
}

ord_parameter_types! {
	pub const BlockchainOperationTreasury: AccountId =
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
		let staking_pot = BlockchainOperationTreasury::get().into();
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
