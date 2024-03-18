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

use crate::*;
use frame_support::assert_ok;
use polimec_common::credentials::InvestorType;
use polimec_common_test_utils::{get_fake_jwt, get_test_jwt};
use polimec_parachain_runtime::PolimecFunding;
use sp_runtime::{AccountId32, DispatchError};
use tests::defaults::*;

#[test]
fn test_jwt_for_create() {
	let project = default_project_metadata(0, ISSUER.into());
	PoliNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolimecBalances::force_set_balance(PolimecOrigin::root(), issuer.into(), 10_000 * PLMC));
		let retail_jwt = get_test_jwt(PolimecAccountId::from(ISSUER), InvestorType::Retail);
		assert_noop!(
			PolimecFunding::create(PolimecOrigin::signed(ISSUER.into()), retail_jwt, project.clone()),
			pallet_funding::Error::<PolimecRuntime>::NotAllowed
		);
		let inst_jwt = get_test_jwt(PolimecAccountId::from(ISSUER), InvestorType::Institutional);
		assert_ok!(PolimecFunding::create(PolimecOrigin::signed(ISSUER.into()), inst_jwt, project.clone()));
	});
}

#[test]
fn test_jwt_verification() {
	let project = default_project_metadata(0, ISSUER.into());
	PoliNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolimecBalances::force_set_balance(PolimecOrigin::root(), issuer.into(), 1000 * PLMC));
		// This JWT tokens is signed with a private key that is not the one set in the Pallet Funding configuration in the real runtime.
		let inst_jwt = get_fake_jwt(PolimecAccountId::from(ISSUER), InvestorType::Institutional);
		assert_noop!(
			PolimecFunding::create(PolimecOrigin::signed(ISSUER.into()), inst_jwt, project.clone()),
			DispatchError::BadOrigin
		);
	});
}
