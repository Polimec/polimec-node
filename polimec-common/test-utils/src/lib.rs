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

use polimec_common::credentials::{InvestorType, UntrustedToken};

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified investor type
pub fn get_test_jwt<AccountId: sp_std::fmt::Display>(
	account_id: AccountId,
	investor_type: InvestorType,
) -> UntrustedToken {
	let jwt = reqwest::blocking::get(format!(
		"https://jws-producer.polimec.workers.dev/mock/{}/{}",
		account_id,
		investor_type.as_str()
	))
	.expect("Failed to perform the HTTP GET")
	.text()
	.expect("Failed to get the response body (jwt) from the specified endpoint");
	let res = UntrustedToken::new(&jwt).expect("Failed to parse the JWT");
	res
}

/// Fetches a JWT from a dummy Polimec JWT producer that will return a JWT with the specified
/// investor type and a random signing key. This is useful for testing the signature
/// verification logic.
pub fn get_fake_jwt<AccountId: sp_std::fmt::Display>(
	account_id: AccountId,
	investor_type: InvestorType,
) -> UntrustedToken {
	let jwt = reqwest::blocking::get(format!(
		"https://jws-producer.polimec.workers.dev/fake/{}/{}",
		account_id,
		investor_type.as_str()
	))
	.expect("Failed to perform the HTTP GET")
	.text()
	.expect("Failed to get the response body (jwt) from the specified endpoint");
	let res = UntrustedToken::new(&jwt).expect("Failed to parse the JWT");
	res
}
