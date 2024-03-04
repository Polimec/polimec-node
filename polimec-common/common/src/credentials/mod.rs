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

use frame_support::{pallet_prelude::*, parameter_types, traits::OriginTrait, Deserialize, RuntimeDebug, Serialize};
use pallet_timestamp::Now;
use parity_scale_codec::{Decode, Encode};
use scale_info::{prelude::string::String, TypeInfo};
use serde::{de::Error, ser::SerializeStruct, Serializer};
use sp_runtime::{traits::BadOrigin, DeserializeOwned};

pub use jwt_compact::{
	alg::{Ed25519, VerifyingKey},
	Claims as StandardClaims, *,
};
use serde::Deserializer;

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

#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, Deserialize)]
pub struct SampleClaims<AccountId> {
	#[serde(rename = "sub")]
	pub subject: AccountId,
	#[serde(rename = "iss")]
	pub issuer: String,
	pub investor_type: InvestorType,
	#[serde(deserialize_with = "from_bounded_vec")]
	pub did: DID,
}

pub type DID = BoundedVec<u8, ConstU32<57>>;

pub struct EnsureInvestor<T>(sp_std::marker::PhantomData<T>);
impl<'de, T> EnsureOriginWithCredentials<T::RuntimeOrigin> for EnsureInvestor<T>
where
	T: frame_system::Config + pallet_timestamp::Config,
{
	type Claims = SampleClaims<T::AccountId>;
	type Success = (T::AccountId, DID, InvestorType);

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

		if claims.custom.subject == who && (date_time.timestamp() as u64) >= now {
			return Ok((who, claims.custom.did.clone(), claims.custom.investor_type.clone()));
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

pub fn from_bounded_vec<'de, D>(deserializer: D) -> Result<BoundedVec<u8, ConstU32<57>>, D::Error>
where
	D: Deserializer<'de>,
{
	String::deserialize(deserializer)
		.map(|string| string.as_bytes().to_vec())
		.and_then(|vec| vec.try_into().map_err(|_| Error::custom("failed to deserialize")))
}

impl<AccountId> Serialize for SampleClaims<AccountId>
where
	AccountId: Serialize, // Ensure AccountId can be serialized
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// Define how many fields we are serializing.
		let mut state = serializer.serialize_struct("SampleClaims", 4)?;

		// Serialize each field.
		// Fields like `subject`, `issuer`, and `investor_type` can be serialized directly.
		state.serialize_field("sub", &self.subject)?;
		state.serialize_field("iss", &self.issuer)?;
		state.serialize_field("investor_type", &self.investor_type)?;

		// For the `did` field, you'd use your custom logic to convert it to a string or another format suitable for serialization.
		// Assuming `did` is a `BoundedVec<u8, ConstU32<57>>` and you're encoding it as a UTF-8 string.
		let did_bytes: scale_info::prelude::vec::Vec<u8> = self.did.clone().into(); // Convert BoundedVec to Vec<u8>
		let did_string = String::from_utf8_lossy(&did_bytes); // Convert Vec<u8> to String
		state.serialize_field("did", &did_string)?;

		// End the serialization
		state.end()
	}
}
