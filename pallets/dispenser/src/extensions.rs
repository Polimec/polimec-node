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

extern crate alloc;

use crate::{Call, Config};
use alloc::vec;
use frame_support::{dispatch::DispatchInfo, pallet_prelude::*, traits::IsSubType};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::{
	AsSystemOriginSigner, DispatchInfoOf, Dispatchable, Implication, One, TransactionExtension, ValidateResult, Zero,
};
/// Custom CheckNonce signed extension for Polimec Blockchain. Based on the CheckNonce signed extension from the FRAME.
/// Removing the providers and sufficients checks for the `dispense` extrinsic, so a new account
/// can get tokens. This is a temporary solution until
/// https://github.com/paritytech/polkadot-sdk/issues/3991 is solved.
/// Nonce check and increment to give replay protection for transactions.
///
/// # Transaction Validity
///
/// This extension affects `requires` and `provides` tags of validity, but DOES NOT
/// set the `priority` field. Make sure that AT LEAST one of the signed extension sets
/// some kind of priority upon validating transactions.
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo, DecodeWithMemTracking)]
#[scale_info(skip_type_params(T))]
pub struct CheckNonce<T: Config>(#[codec(compact)] pub T::Nonce);

impl<T: Config> CheckNonce<T> {
	/// utility constructor. Used only in client/factory code.
	pub fn from(nonce: T::Nonce) -> Self {
		Self(nonce)
	}
}

impl<T: Config> core::fmt::Debug for CheckNonce<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "CheckNonce({})", self.0)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut core::fmt::Formatter) -> core::fmt::Result {
		Ok(())
	}
}

/// Operation to perform from `validate` to `prepare` in [`CheckNonce`] transaction extension.
#[derive(RuntimeDebugNoBound)]
pub enum Val<T: Config> {
	/// Account and its nonce to check for.
	CheckNonce((T::AccountId, T::Nonce)),
	/// Weight to refund.
	Refund(Weight),
}

/// Operation to perform from `prepare` to `post_dispatch_details` in [`CheckNonce`] transaction
/// extension.
#[derive(RuntimeDebugNoBound)]
pub enum Pre {
	/// The transaction extension weight should not be refunded.
	NonceChecked,
	/// The transaction extension weight should be refunded.
	Refund(Weight),
}

impl<T: Config> TransactionExtension<T::RuntimeCall> for CheckNonce<T>
where
	<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo> + IsSubType<Call<T>>,
	<T::RuntimeCall as Dispatchable>::RuntimeOrigin: AsSystemOriginSigner<T::AccountId> + Clone,
{
	type Implicit = ();
	type Pre = Pre;
	type Val = Val<T>;

	const IDENTIFIER: &'static str = "CheckNonce";

	fn implicit(&self) -> Result<Self::Implicit, TransactionValidityError> {
		Ok(())
	}

	fn weight(&self, _call: &T::RuntimeCall) -> Weight {
		<T::ExtensionsWeightInfo as frame_system::ExtensionsWeightInfo>::check_nonce()
	}

	fn validate(
		&self,
		origin: <T as frame_system::Config>::RuntimeOrigin,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Implication,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, T::RuntimeCall> {
		let Some(who) = origin.as_system_origin_signer() else {
			return Ok((Default::default(), Val::Refund(self.weight(call)), origin))
		};
		let account = frame_system::Account::<T>::get(who);
		if account.providers.is_zero() && account.sufficients.is_zero() {
			match call.is_sub_type() {
				Some(call) if matches!(call, &Call::<T>::dispense { .. }) => {},
				_ => return Err(InvalidTransaction::Payment.into()),
			}
		}
		if self.0 < account.nonce {
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Stale));
		}

		let provides = vec![Encode::encode(&(who, self.0))];
		let requires = if account.nonce < self.0 { vec![Encode::encode(&(who, self.0 - One::one()))] } else { vec![] };

		let valid_transaction =
			ValidTransaction { priority: 0, requires, provides, longevity: TransactionLongevity::MAX, propagate: true };

		Ok((valid_transaction, Val::CheckNonce((who.clone(), account.nonce)), origin))
	}

	fn prepare(
		self,
		val: Self::Val,
		_origin: &T::RuntimeOrigin,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		_len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		let (who, mut _nonce) = match val {
			Val::CheckNonce((who, nonce)) => (who, nonce),
			Val::Refund(weight) => return Ok(Pre::Refund(weight)),
		};
		let mut account = frame_system::Account::<T>::get(who.clone());
		if account.providers.is_zero() && account.sufficients.is_zero() {
			match call.is_sub_type() {
				Some(call) if matches!(call, &Call::<T>::dispense { .. }) => {},
				_ => return Err(InvalidTransaction::Payment.into()),
			}
		}
		if self.0 != account.nonce {
			return Err(
				if self.0 < account.nonce { InvalidTransaction::Stale } else { InvalidTransaction::Future }.into()
			)
		}
		account.nonce += T::Nonce::one();
		frame_system::Account::<T>::insert(who, account);
		Ok(Pre::NonceChecked)
	}
}
