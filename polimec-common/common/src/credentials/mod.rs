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
use scale_info::{prelude::string::String, TypeInfo};
use serde::{de::Error, ser::SerializeStruct, Serializer};
use sp_runtime::{traits::BadOrigin, DeserializeOwned};

pub use jwt_compact::{
	alg::{Ed25519, VerifyingKey},
	Claims as StandardClaims, *,
};
use serde::Deserializer;

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, TypeInfo, Deserialize, Serialize, MaxEncodedLen, RuntimeDebug)]
#[serde(rename_all = "lowercase")]
pub enum InvestorType {
	Retail,
	Professional,
	Institutional,
}

impl InvestorType {
	#[must_use]
	pub const fn as_str(&self) -> &'static str {
		match self {
			Self::Retail => "retail",
			Self::Professional => "professional",
			Self::Institutional => "institutional",
		}
	}
}

parameter_types! {
	pub const Retail: InvestorType = InvestorType::Retail;
	pub const Professional: InvestorType = InvestorType::Professional;
	pub const Institutional: InvestorType = InvestorType::Institutional;
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, TypeInfo, Deserialize)]
pub struct PolimecPayload<AccountId> {
	#[serde(rename = "sub")]
	pub subject: AccountId,
	#[serde(rename = "iss")]
	pub issuer: String,
	#[serde(rename = "aud", deserialize_with = "from_bounded_cid")]
	pub ipfs_cid: Cid,
	pub investor_type: InvestorType,
	#[serde(deserialize_with = "from_bounded_did")]
	pub did: Did,
}

pub type Did = BoundedVec<u8, ConstU32<64>>;
pub type Cid = BoundedVec<u8, ConstU32<96>>;

pub struct EnsureInvestor<T>(core::marker::PhantomData<T>);
impl<T> EnsureOriginWithCredentials<T::RuntimeOrigin> for EnsureInvestor<T>
where
	T: frame_system::Config + pallet_timestamp::Config,
{
	type Claims = PolimecPayload<T::AccountId>;
	type Success = (T::AccountId, Did, InvestorType, Cid);

	fn try_origin(
		origin: T::RuntimeOrigin,
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<Self::Success, T::RuntimeOrigin> {
		let Some(who) = origin.clone().into_signer() else { return Err(origin) };
		let Ok(token) = Self::verify_token(token, verifying_key) else { return Err(origin) };
		let claims = token.claims();
		// Get current timestamp from pallet_timestamp (milliseconds)
		let Ok(now) = Now::<T>::get().try_into() else { return Err(origin) };
		let Some(date_time) = claims.expiration else { return Err(origin) };

		let timestamp: u64 = date_time.timestamp_millis().try_into().map_err(|_| origin.clone())?;

		if claims.custom.subject == who && timestamp >= now {
			return Ok((who, claims.custom.did.clone(), claims.custom.investor_type, claims.custom.ipfs_cid.clone()));
		}

		Err(origin)
	}
}

#[allow(clippy::module_name_repetitions)]
pub trait EnsureOriginWithCredentials<OuterOrigin>
where
	OuterOrigin: OriginTrait,
{
	type Success;
	type Claims: Clone + Encode + Decode + Eq + PartialEq + TypeInfo + DeserializeOwned;

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

	fn verify_token(
		token: &jwt_compact::UntrustedToken,
		verifying_key: [u8; 32],
	) -> Result<jwt_compact::Token<Self::Claims>, ValidationError> {
		let signing_key =
			<<Ed25519 as Algorithm>::VerifyingKey>::from_slice(&verifying_key).expect("The Key is always valid");
		Ed25519.validator::<Self::Claims>(&signing_key).validate(token)
	}
}

pub fn from_bounded_did<'de, D>(deserializer: D) -> Result<Did, D::Error>
where
	D: Deserializer<'de>,
{
	String::deserialize(deserializer)
		.map(|string| string.into_bytes())
		.and_then(|vec| BoundedVec::try_from(vec).map_err(|_| Error::custom("DID exceeds length limit")))
}

pub fn from_bounded_cid<'de, D>(deserializer: D) -> Result<Cid, D::Error>
where
	D: Deserializer<'de>,
{
	String::deserialize(deserializer)
		.map(|string| string.into_bytes())
		.and_then(|vec| BoundedVec::try_from(vec).map_err(|_| Error::custom("CID exceeds length limit")))
}

// Key corrected serialization implementation
impl<AccountId> Serialize for PolimecPayload<AccountId>
where
	AccountId: Serialize,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		// Define how many fields we are serializing.
		let mut state = serializer.serialize_struct("PolimecPayload", 5)?;

		// Serialize each field.
		// Fields like `subject`, `issuer`, and `investor_type` can be serialized directly.
		state.serialize_field("sub", &self.subject)?;
		state.serialize_field("iss", &self.issuer)?;
		state.serialize_field("investor_type", &self.investor_type)?;
		// Serialize the `ipfs_cid` and `did` fields as strings.
		state.serialize_field("aud", core::str::from_utf8(&self.ipfs_cid).map_err(serde::ser::Error::custom)?)?;
		state.serialize_field("did", core::str::from_utf8(&self.did).map_err(serde::ser::Error::custom)?)?;
		state.end()
	}
}
