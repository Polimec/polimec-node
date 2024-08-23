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

use crate::{Call, Config};
use frame_support::{
	dispatch::{CheckIfFeeless, DispatchInfo},
	pallet_prelude::*,
	traits::{IsSubType, OriginTrait},
};
use parity_scale_codec::{Decode, Encode};
use scale_info::{StaticTypeInfo, TypeInfo};
use sp_runtime::traits::{DispatchInfoOf, Dispatchable, One, PostDispatchInfoOf, SignedExtension, Zero};
use sp_std::vec;
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
#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CheckNonce<T: Config>(#[codec(compact)] pub T::Nonce);

impl<T: Config> CheckNonce<T> {
	/// utility constructor. Used only in client/factory code.
	pub fn from(nonce: T::Nonce) -> Self {
		Self(nonce)
	}
}

impl<T: Config> sp_std::fmt::Debug for CheckNonce<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "CheckNonce({})", self.0)
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: Config> SignedExtension for CheckNonce<T>
where
	<T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo> + IsSubType<Call<T>>,
{
	type AccountId = T::AccountId;
	type AdditionalSigned = ();
	type Call = <T as frame_system::Config>::RuntimeCall;
	type Pre = ();

	const IDENTIFIER: &'static str = "CheckNonce";

	fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> Result<(), TransactionValidityError> {
		let mut account = frame_system::Account::<T>::get(who);
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
		Ok(())
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		let account = frame_system::Account::<T>::get(who);
		if account.providers.is_zero() && account.sufficients.is_zero() {
			match call.is_sub_type() {
				Some(call) if matches!(call, &Call::<T>::dispense { .. }) => {},
				_ => return Err(InvalidTransaction::Payment.into()),
			}
		}
		if self.0 < account.nonce {
			return InvalidTransaction::Stale.into()
		}

		let provides = vec![Encode::encode(&(who, self.0))];
		let requires = if account.nonce < self.0 { vec![Encode::encode(&(who, self.0 - One::one()))] } else { vec![] };

		Ok(ValidTransaction { priority: 0, requires, provides, longevity: TransactionLongevity::MAX, propagate: true })
	}
}

/// A [`SignedExtension`] that skips the wrapped extension if the dispatchable is feeless.
/// This is an adjusted version of the `CheckIfFeeless` signed extension from FRAME.
/// The FRAME implementation does currently not implement the 'validate' function, which opens
/// up the possibility of DoS attacks. This implementation is a temporary solution until fixed
/// (https://github.com/paritytech/polkadot-sdk/pull/3993) in FRAME.
#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct SkipCheckIfFeeless<T, S>(pub S, sp_std::marker::PhantomData<T>);

// Make this extension "invisible" from the outside (ie metadata type information)
impl<T, S: StaticTypeInfo> TypeInfo for SkipCheckIfFeeless<T, S> {
	type Identity = S;

	fn type_info() -> scale_info::Type {
		S::type_info()
	}
}

impl<T, S: Encode> sp_std::fmt::Debug for SkipCheckIfFeeless<T, S> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "SkipCheckIfFeeless<{:?}>", self.0.encode())
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T, S> From<S> for SkipCheckIfFeeless<T, S> {
	fn from(s: S) -> Self {
		Self(s, sp_std::marker::PhantomData)
	}
}

impl<T: Config + Send + Sync, S: SignedExtension<AccountId = T::AccountId>> SignedExtension for SkipCheckIfFeeless<T, S>
where
	S::Call: CheckIfFeeless<Origin = frame_system::pallet_prelude::OriginFor<T>>,
{
	type AccountId = T::AccountId;
	type AdditionalSigned = S::AdditionalSigned;
	type Call = S::Call;
	type Pre = Option<<S as SignedExtension>::Pre>;

	// From the outside this extension should be "invisible", because it just extends the wrapped
	// extension with an extra check in `pre_dispatch` and `post_dispatch`. Thus, we should forward
	// the identifier of the wrapped extension to let wallets see this extension as it would only be
	// the wrapped extension itself.
	const IDENTIFIER: &'static str = S::IDENTIFIER;

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		self.0.additional_signed()
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		if call.is_feeless(&<T as frame_system::Config>::RuntimeOrigin::signed(who.clone())) {
			Ok(None)
		} else {
			Ok(Some(self.0.pre_dispatch(who, call, info, len)?))
		}
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> TransactionValidity {
		if call.is_feeless(&<T as frame_system::Config>::RuntimeOrigin::signed(who.clone())) {
			Ok(ValidTransaction::default())
		} else {
			self.0.validate(who, call, info, len)
		}
	}

	fn post_dispatch(
		pre: Option<Self::Pre>,
		info: &DispatchInfoOf<Self::Call>,
		post_info: &PostDispatchInfoOf<Self::Call>,
		len: usize,
		result: &DispatchResult,
	) -> Result<(), TransactionValidityError> {
		if let Some(Some(pre)) = pre {
			S::post_dispatch(Some(pre), info, post_info, len, result)?;
		}
		Ok(())
	}
}
