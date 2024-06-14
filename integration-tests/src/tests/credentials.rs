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
use frame_support::{assert_err, assert_ok, dispatch::GetDispatchInfo, traits::tokens::currency::VestingSchedule};
use macros::generate_accounts;
use polimec_common::credentials::{Did, InvestorType};
use polimec_common_test_utils::{get_fake_jwt, get_mock_jwt_with_cid, get_test_jwt};
use polimec_runtime::PLMC;
use sp_runtime::{
	generic::Era,
	traits::SignedExtension,
	transaction_validity::{InvalidTransaction::Payment, TransactionValidityError},
	AccountId32, DispatchError,
};
use tests::defaults::*;

#[test]
fn test_jwt_for_create() {
	let project = default_project_metadata(ISSUER.into());
	PolitestNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolitestBalances::force_set_balance(PolitestOrigin::root(), issuer.into(), 10_000 * PLMC));
		let retail_jwt = get_test_jwt(PolitestAccountId::from(ISSUER), InvestorType::Retail);
		assert_noop!(
			PolitestFundingPallet::create_project(PolitestOrigin::signed(ISSUER.into()), retail_jwt, project.clone()),
			pallet_funding::Error::<PolitestRuntime>::WrongInvestorType
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
	let project = default_project_metadata(ISSUER.into());
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

generate_accounts!(EMPTY_ACCOUNT);

#[test]
fn dispenser_signed_extensions_pass_for_new_account() {
	PolitestNet::execute_with(|| {
		let who = PolitestAccountId::from(EMPTY_ACCOUNT);
		assert_eq!(PolimecBalances::free_balance(who.clone()), 0);

		let jwt = get_test_jwt(who.clone(), InvestorType::Retail);
		let free_call = PolitestCall::Dispenser(pallet_dispenser::Call::dispense { jwt: jwt.clone() });
		let paid_call = PolitestCall::System(frame_system::Call::remark { remark: vec![69, 69] });
		let extra: politest_runtime::SignedExtra = (
			frame_system::CheckNonZeroSender::<PolitestRuntime>::new(),
			frame_system::CheckSpecVersion::<PolitestRuntime>::new(),
			frame_system::CheckTxVersion::<PolitestRuntime>::new(),
			frame_system::CheckGenesis::<PolitestRuntime>::new(),
			frame_system::CheckEra::<PolitestRuntime>::from(Era::mortal(0u64, 0u64)),
			pallet_dispenser::extensions::CheckNonce::<PolitestRuntime>::from(0u32),
			frame_system::CheckWeight::<PolitestRuntime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<PolitestRuntime>::from(0u64.into()).into(),
		);
		assert_err!(
			extra.validate(&who, &paid_call, &paid_call.get_dispatch_info(), 0),
			TransactionValidityError::Invalid(Payment)
		);
		assert_err!(
			extra.clone().pre_dispatch(&who, &paid_call, &paid_call.get_dispatch_info(), 0),
			TransactionValidityError::Invalid(Payment)
		);

		assert_ok!(extra.validate(&who, &free_call, &free_call.get_dispatch_info(), 0));
		assert_ok!(extra.pre_dispatch(&who, &free_call, &free_call.get_dispatch_info(), 0));
	});
}

#[test]
fn dispenser_works_with_runtime_values() {
	PolitestNet::execute_with(|| {
		let who = PolitestAccountId::from(EMPTY_ACCOUNT);
		let did = "kilt:did:tz:tz1K7fCz9QJtXv3J8Ud3Zvz7eQ6";
		let bytes_did = did.as_bytes().to_vec();
		let bounded_did: Did = bytes_did.try_into().unwrap();
		let jwt = get_mock_jwt_with_cid(
			who.clone(),
			InvestorType::Retail,
			bounded_did,
			politest_runtime::DispenserWhitelistedPolicy::get(),
		);
		PolitestBalances::force_set_balance(
			PolitestOrigin::root(),
			PolitestDispenser::dispense_account().into(),
			1000 * PLMC,
		)
		.unwrap();
		assert_ok!(PolitestDispenser::dispense(PolitestOrigin::signed(who.clone()), jwt));
		assert_eq!(PolitestBalances::free_balance(&who), 700 * PLMC);
		assert_eq!(
			PolitestBalances::usable_balance(who.clone()),
			<PolitestRuntime as pallet_dispenser::Config>::FreeDispenseAmount::get()
		);
		assert_eq!(
			PolitestVesting::vesting_balance(&who),
			Some(
				<PolitestRuntime as pallet_dispenser::Config>::InitialDispenseAmount::get() -
					<PolitestRuntime as pallet_dispenser::Config>::FreeDispenseAmount::get()
			)
		);
	})
}
