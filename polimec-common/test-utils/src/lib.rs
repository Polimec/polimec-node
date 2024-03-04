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

use jwt_compact::{alg::Ed25519, AlgorithmExt, Header};
use polimec_common::credentials::{InvestorType, SampleClaims, UntrustedToken};

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified investor type
#[cfg(not(feature = "runtime-benchmarks"))]
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
		SampleClaims { subject: account_id, investor_type, issuer: "verifier".to_string() };
	// Wrap the `SampleClaims` struct in the `Claims` struct.
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
#[cfg(not(feature = "runtime-benchmarks"))]
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
