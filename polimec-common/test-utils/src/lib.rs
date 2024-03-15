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

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{sp_runtime::app_crypto::sp_core::bytes::to_hex, traits::ConstU32, BoundedVec, Parameter};
use jwt_compact::{alg::Ed25519, AlgorithmExt, Header};
use polimec_common::credentials::{Did, InvestorType, SampleClaims, UntrustedToken};

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified investor type
#[cfg(feature = "std")]
pub fn get_test_jwt<AccountId: core::fmt::Display>(
	account_id: AccountId,
	investor_type: InvestorType,
) -> UntrustedToken {
	let jwt = reqwest::blocking::get(format!(
		"http://jws-producer.polimec.workers.dev/mock/{}/{}",
		account_id,
		investor_type.as_str()
	))
	.expect("Failed to perform the HTTP GET")
	.text()
	.expect("Failed to get the response body (jwt) from the specified endpoint");
	let res = UntrustedToken::new(&jwt).expect("Failed to parse the JWT");
	res
}

// The `Serialize` trait is needed to serialize the `account_id` into a  `SampleClaims` struct.
pub fn get_mock_jwt<AccountId: frame_support::Serialize>(
	account_id: AccountId,
	investor_type: InvestorType,
	did: BoundedVec<u8, ConstU32<57>>,
) -> UntrustedToken {
	use chrono::{TimeZone, Utc};
	use jwt_compact::{alg::SigningKey, Claims};

	#[allow(unused)]
	// Needed to convert the "issuer" field to a string.
	use parity_scale_codec::alloc::string::ToString;

	// Create a signing key from raw bytes.
	let key = SigningKey::from_slice(
		[
			80, 168, 164, 18, 76, 133, 92, 116, 50, 20, 155, 28, 33, 89, 151, 207, 199, 247, 113, 185, 127, 156, 2,
			132, 65, 58, 76, 156, 143, 109, 29, 251,
		]
		.as_ref(),
	)
	.unwrap();
	// We don't need any custom fields in the header, so we use the empty.
	let header: Header = Header::empty();

	// Create the custom part of the `Claims` struct.
	let custom_claims: SampleClaims<AccountId> =
		SampleClaims { subject: account_id, investor_type, issuer: "verifier".to_string(), did };

	// Wrap the SampleClaims` struct in the `Claims` struct.
	let mut claims = Claims::new(custom_claims);
	// Set the expiration date to 2030-01-01.
	// We need to unwrap the `Utc::with_ymd_and_hms` because it returns a `LocalResult<DateTime<Utc>>` but we ned a `DateTime<Utc>.
	claims.expiration = Some(Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap());

	// Create a JWT using the Ed25519 algorithm.
	let token_string = Ed25519.token(&header, &claims, &key).unwrap();

	// Create an `UntrustedToken` from the signed JWT string.
	UntrustedToken::new(&token_string).expect("Failed to parse the JWT")
}

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified
/// investor type and a random signing key. This is useful for testing the signature
/// verification logic.
#[cfg(feature = "std")]
pub fn get_fake_jwt<AccountId: core::fmt::Display>(
	account_id: AccountId,
	investor_type: InvestorType,
) -> UntrustedToken {
	let jwt = reqwest::blocking::get(format!(
		"http://jws-producer.polimec.workers.dev/fake/{}/{}",
		account_id,
		investor_type.as_str()
	))
	.expect("Failed to perform the HTTP GET")
	.text()
	.expect("Failed to get the response body (jwt) from the specified endpoint");
	let res = UntrustedToken::new(&jwt).expect("Failed to parse the JWT");
	res
}

pub fn generate_did_from_account(account_id: impl Parameter) -> Did {
	let mut hex_account = to_hex(&account_id.encode(), true);
	if hex_account.len() > 57 {
		#[allow(unused_imports)]
		use parity_scale_codec::alloc::string::ToString;
		hex_account = hex_account[0..57].to_string();
	}
	hex_account.into_bytes().try_into().unwrap()
}

#[cfg(test)]
mod tests {
	use crate::{generate_did_from_account, get_mock_jwt};
	use jwt_compact::{
		alg::{Ed25519, VerifyingKey},
		AlgorithmExt,
	};
	use polimec_common::credentials::{InvestorType, SampleClaims};

	#[test]
	fn test_get_test_jwt() {
		let verifying_key = VerifyingKey::from_slice(
			[
				32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253, 195, 18, 202,
				111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
			]
			.as_ref(),
		)
		.unwrap();
		let token = get_mock_jwt("0x1234", InvestorType::Institutional, generate_did_from_account(40u64));
		let res = Ed25519.validator::<SampleClaims<String>>(&verifying_key).validate(&token);
		assert!(res.is_ok());
	}
}
