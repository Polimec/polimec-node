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
use tests::defaults::*;
use macros::generate_accounts;
use frame_support::{assert_ok, dispatch::GetDispatchInfo};
use polimec_common::credentials::InvestorType;
use polimec_common_test_utils::{get_fake_jwt, get_test_jwt};
use sp_runtime::{AccountId32, DispatchError, traits::SignedExtension, generic::Era};

#[test]
fn test_jwt_for_create() {
	let project = default_project_metadata(0, ISSUER.into());
	PolitestNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolitestBalances::force_set_balance(PolitestOrigin::root(), issuer.into(), 10_000 * PLMC));
		let retail_jwt = get_test_jwt(PolitestAccountId::from(ISSUER), InvestorType::Retail);
		assert_noop!(
			PolitestFundingPallet::create_project(PolitestOrigin::signed(ISSUER.into()), retail_jwt, project.clone()),
			pallet_funding::Error::<PolitestRuntime>::NotAllowed
		);
		let inst_jwt = get_test_jwt(PolitestAccountId::from(ISSUER), InvestorType::Institutional);
		assert_ok!(PolitestFundingPallet::create_project(
			PolitestOrigin::signed(ISSUER.into()),
			inst_jwt,
			project.clone()
		));
	});
}

#[test]
fn test_jwt_verification() {
	let project = default_project_metadata(0, ISSUER.into());
	PolitestNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolitestBalances::force_set_balance(PolitestOrigin::root(), issuer.into(), 1000 * PLMC));
		// This JWT tokens is signed with a private key that is not the one set in the Pallet Funding configuration in the real runtime.
		let inst_jwt = get_fake_jwt(PolitestAccountId::from(ISSUER), InvestorType::Institutional);
		assert_noop!(
			PolitestFundingPallet::create_project(PolitestOrigin::signed(ISSUER.into()), inst_jwt, project.clone()),
			DispatchError::BadOrigin
		);
	});
}

generate_accounts!(CLAIMER);

#[test]
fn faucet_pre_dispatch_passed_for_new_account() {
	PolitestNet::execute_with(|| {
		let who = PolitestAccountId::from(CLAIMER);
		let jwt = get_test_jwt(who.clone(), InvestorType::Retail);
		let call = PolitestCall::Claims(pallet_faucet::Call::claim {jwt: jwt.clone()});
		let extra: politest_runtime::SignedExtra = (
			frame_system::CheckNonZeroSender::<PolitestRuntime>::new(),
			frame_system::CheckSpecVersion::<PolitestRuntime>::new(),
			frame_system::CheckTxVersion::<PolitestRuntime>::new(),
			frame_system::CheckGenesis::<PolitestRuntime>::new(),
			frame_system::CheckEra::<PolitestRuntime>::from(Era::mortal(0u64, 0u64)),
			pallet_faucet::extensions::CheckNonce::<PolitestRuntime>::from(0u32),
			frame_system::CheckWeight::<PolitestRuntime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<PolitestRuntime>::from(0u64.into()).into(),
		);
		assert_ok!(extra.validate(&who, &call, &call.get_dispatch_info(), 0));
		assert_ok!(extra.pre_dispatch(&who, &call, &call.get_dispatch_info(), 0));
	});
}
