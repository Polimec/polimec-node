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
use parity_scale_codec::alloc::string::ToString;
use polimec_common::credentials::{Did, InvestorType, SampleClaims, UntrustedToken};

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified investor type
#[cfg(feature = "std")]
pub fn get_test_jwt<AccountId: core::fmt::Display>(
	account_id: AccountId,
	investor_type: InvestorType,
) -> UntrustedToken {
	// TODO: Accept the DID as a parameter.
	let did = "did:polimec:0x1234";
	// TODO: Accept the CID as a parameter.

	let cid = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";

	let url = format!(
		"http://jws-producer.polimec.workers.dev/mock/{}/{}/{}/{}",
		account_id,
		investor_type.as_str(),
		did,
		cid
	);
	println!("URL: {}", url);
	// TODO: This should be a POST with everything in the body.
	let jwt = reqwest::blocking::get(url)
		.expect("Failed to perform the HTTP GET")
		.text()
		.expect("Failed to get the response body (jwt) from the specified endpoint");
	let res = UntrustedToken::new(&jwt).expect("Failed to parse the JWT");
	res
}

fn create_jwt<AccountId: frame_support::Serialize>(
	account_id: AccountId,
	investor_type: InvestorType,
	did: BoundedVec<u8, ConstU32<57>>,
	ipfs_cid: Option<BoundedVec<u8, ConstU32<96>>>,
) -> UntrustedToken {
	use chrono::{TimeZone, Utc};
	use jwt_compact::{alg::SigningKey, Claims};

	// Create a signing key from raw bytes.
	let key = SigningKey::from_slice(
		[
			80, 168, 164, 18, 76, 133, 92, 116, 50, 20, 155, 28, 33, 89, 151, 207, 199, 247, 113, 185, 127, 156, 2,
			132, 65, 58, 76, 156, 143, 109, 29, 251,
		]
		.as_ref(),
	)
	.unwrap();

	let header: Header = Header::empty();

	// Handle optional IPFS CID
	let ipfs_cid = ipfs_cid.unwrap_or_else(|| BoundedVec::with_bounded_capacity(96));
	let custom_claims =
		SampleClaims { subject: account_id, investor_type, issuer: "verifier".to_string(), did, ipfs_cid };

	let mut claims = Claims::new(custom_claims);
	claims.expiration = Some(Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap());

	let token_string = Ed25519.token(&header, &claims, &key).unwrap();
	UntrustedToken::new(&token_string).expect("Failed to parse the JWT")
}

// The `Serialize` trait is needed to serialize the `account_id` into a  `SampleClaims` struct.
pub fn get_mock_jwt<AccountId: frame_support::Serialize>(
	account_id: AccountId,
	investor_type: InvestorType,
	did: BoundedVec<u8, ConstU32<57>>,
) -> UntrustedToken {
	create_jwt(account_id, investor_type, did, None)
}

// The `Serialize` trait is needed to serialize the `account_id` into a  `SampleClaims` struct.
pub fn get_mock_jwt_with_cid<AccountId: frame_support::Serialize>(
	account_id: AccountId,
	investor_type: InvestorType,
	did: BoundedVec<u8, ConstU32<57>>,
	ipfs_cid: BoundedVec<u8, ConstU32<96>>,
) -> UntrustedToken {
	create_jwt(account_id, investor_type, did, Some(ipfs_cid))
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

#[cfg(feature = "std")]
pub fn do_request(url: &str) -> String {
	reqwest::blocking::Client::builder()
		.user_agent("polimec")
		.build()
		.expect("Failed to build Client")
		.get(url)
		.send()
		.expect("Failed to perform the HTTP GET")
		.text()
		.expect("Failed to get the response body from the specified endpoint")
}

#[cfg(test)]
mod tests {
	use crate::{generate_did_from_account, get_mock_jwt, get_mock_jwt_with_cid};
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

	#[test]
	fn test_get_test_jwt_with_cid() {
		let verifying_key = VerifyingKey::from_slice(
			[
				32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253, 195, 18, 202,
				111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
			]
			.as_ref(),
		)
		.unwrap();
		let cid: &str = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";
		let bounded_cid = frame_support::BoundedVec::try_from(cid.as_bytes().to_vec()).unwrap();
		let token = get_mock_jwt_with_cid(
			"0x1234",
			InvestorType::Institutional,
			generate_did_from_account(40u64),
			bounded_cid.clone(),
		);
		let res = Ed25519.validator::<SampleClaims<String>>(&verifying_key).validate(&token);
		assert!(res.is_ok());
		let validated_token = res.unwrap();
		let claims = validated_token.claims();
		assert_eq!(claims.custom.ipfs_cid, bounded_cid);
		let cid_from_token = std::str::from_utf8(&claims.custom.ipfs_cid).unwrap();
		assert_eq!(cid_from_token, cid);
	}
}
