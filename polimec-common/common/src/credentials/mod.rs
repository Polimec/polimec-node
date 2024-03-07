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

use frame_support::{pallet_prelude::*, parameter_types, traits::OriginTrait, Deserialize, Serialize};
use pallet_timestamp::Now;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{traits::BadOrigin, DeserializeOwned, RuntimeDebug};

pub use jwt_compact::{
	alg::{Ed25519, VerifyingKey},
	Claims as StandardClaims, *,
};

#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InvestorType {
	Retail,
	Professional,
	Institutional,
}

impl InvestorType {
	pub fn as_str(&self) -> &'static str {
		match self {
			InvestorType::Retail => "retail",
			InvestorType::Professional => "professional",
			InvestorType::Institutional => "institutional",
		}
	}
}

parameter_types! {
	pub const Retail: InvestorType = InvestorType::Retail;
	pub const Professional: InvestorType = InvestorType::Professional;
	pub const Institutional: InvestorType = InvestorType::Institutional;
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, Deserialize, Serialize)]
pub struct SampleClaims<AccountId> {
	#[serde(rename = "sub")]
	pub subject: AccountId,
	#[serde(rename = "iss")]
	pub issuer: scale_info::prelude::string::String,
	pub investor_type: InvestorType,
}

pub struct EnsureInvestor<T, I, Type>(sp_std::marker::PhantomData<(T, I, Type)>);
impl<'de, T, I, Type> EnsureOriginWithCredentials<T::RuntimeOrigin> for EnsureInvestor<T, I, Type>
where
	T: frame_system::Config + pallet_timestamp::Config,
	I: 'static,
	Type: Get<InvestorType>,
{
	type Claims = SampleClaims<T::AccountId>;
	type Success = T::AccountId;

	fn try_origin(
		origin: T::RuntimeOrigin,
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<Self::Success, T::RuntimeOrigin> {
		let Some(who) = origin.clone().into_signer() else { return Err(origin) };
		let Ok(token) = Self::verify_token(token, verifying_key) else { return Err(origin) };
		let Ok(claims) = Self::extract_claims(&token) else { return Err(origin) };
		let Ok(now) = Now::<T>::get().try_into() else { return Err(origin) };
		let Some(date_time) = claims.expiration else { return Err(origin) };

		if claims.custom.investor_type == Type::get() &&
			claims.custom.subject == who &&
			(date_time.timestamp() as u64) >= now
		{
			return Ok(who);
		}

		Err(origin)
	}
}

pub trait EnsureOriginWithCredentials<OuterOrigin>
where
	OuterOrigin: OriginTrait,
{
	type Success;
	type Claims: Clone + Encode + Decode + Eq + PartialEq + Ord + PartialOrd + TypeInfo + DeserializeOwned;

	fn try_origin(
		origin: OuterOrigin,
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<Self::Success, OuterOrigin>;

	fn ensure_origin(
		origin: OuterOrigin,
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<Self::Success, BadOrigin> {
		Self::try_origin(origin, token, verifying_key).map_err(|_| BadOrigin)
	}

	fn extract_claims(token: &jwt_compact::Token<Self::Claims>) -> Result<&StandardClaims<Self::Claims>, ()> {
		Ok(&token.claims())
	}

	fn verify_token(
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<jwt_compact::Token<Self::Claims>, ValidationError> {
		let signing_key =
			<<Ed25519 as Algorithm>::VerifyingKey>::from_slice(&verifying_key).expect("The Key is always valid");
		Ed25519.validator::<Self::Claims>(&signing_key).validate(&token)
	}
}
