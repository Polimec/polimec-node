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
use frame_support::{assert_ok, dispatch::GetDispatchInfo, traits::tokens::currency::VestingSchedule};
use macros::generate_accounts;
use pallet_funding::ParticipationMode::{Classic, OTM};
use polimec_common::credentials::{Did, InvestorType};
use polimec_common_test_utils::{get_fake_jwt, get_mock_jwt_with_cid, get_test_jwt};
use polimec_runtime::PLMC;
use sp_runtime::{generic::Era, traits::SignedExtension, AccountId32, DispatchError};
use tests::defaults::*;

#[test]
fn test_jwt_for_create() {
	let project = default_project_metadata(ISSUER.into());
	PolimecNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolimecBalances::force_set_balance(PolimecOrigin::root(), issuer.into(), 10_000 * PLMC));
		let retail_jwt = get_test_jwt(PolimecAccountId::from(ISSUER), InvestorType::Retail);
		assert_noop!(
			PolimecFunding::create_project(PolimecOrigin::signed(ISSUER.into()), retail_jwt, project.clone()),
			pallet_funding::Error::<PolimecRuntime>::WrongInvestorType
		);
		let inst_jwt = get_test_jwt(PolimecAccountId::from(ISSUER), InvestorType::Institutional);
		assert_ok!(PolimecFunding::create_project(PolimecOrigin::signed(ISSUER.into()), inst_jwt, project.clone()));
	});
}

#[test]
fn test_jwt_verification() {
	let project = default_project_metadata(ISSUER.into());
	PolimecNet::execute_with(|| {
		let issuer = AccountId32::from(ISSUER);
		assert_ok!(PolimecBalances::force_set_balance(PolimecOrigin::root(), issuer.into(), 1000 * PLMC));
		// This JWT tokens is signed with a private key that is not the one set in the Pallet Funding configuration in the real runtime.
		let inst_jwt = get_fake_jwt(PolimecAccountId::from(ISSUER), InvestorType::Institutional);
		assert_noop!(
			PolimecFunding::create_project(PolimecOrigin::signed(ISSUER.into()), inst_jwt, project.clone()),
			DispatchError::BadOrigin
		);
	});
}

generate_accounts!(EMPTY_ACCOUNT);

#[test]
fn dispenser_signed_extensions_pass_for_new_account() {
	PolimecNet::execute_with(|| {
		let who = PolimecAccountId::from(EMPTY_ACCOUNT);
		assert_eq!(PolimecBalances::free_balance(who.clone()), 0);

		let jwt = get_test_jwt(who.clone(), InvestorType::Retail);
		let free_call = PolimecCall::Dispenser(pallet_dispenser::Call::dispense { jwt: jwt.clone() });
		let paid_call = PolimecCall::System(frame_system::Call::remark { remark: vec![69, 69] });
		let extra: polimec_runtime::SignedExtra = (
			frame_system::CheckNonZeroSender::<PolimecRuntime>::new(),
			frame_system::CheckSpecVersion::<PolimecRuntime>::new(),
			frame_system::CheckTxVersion::<PolimecRuntime>::new(),
			frame_system::CheckGenesis::<PolimecRuntime>::new(),
			frame_system::CheckEra::<PolimecRuntime>::from(Era::mortal(0u64, 0u64)),
			pallet_dispenser::extensions::CheckNonce::<PolimecRuntime>::from(0u32),
			frame_system::CheckWeight::<PolimecRuntime>::new(),
			pallet_asset_tx_payment::ChargeAssetTxPayment::<PolimecRuntime>::from(0u64.into(), None).into(),
			frame_metadata_hash_extension::CheckMetadataHash::<PolimecRuntime>::new(true),
		);

		// `InitialPayment` struct from pallet_asset_tx_payment doesn't implement Debug and PartialEq to compare to a specific Error or use assert_ok!
		assert!(extra.validate(&who, &paid_call, &paid_call.get_dispatch_info(), 0).is_err());
		assert!(extra.clone().pre_dispatch(&who, &paid_call, &paid_call.get_dispatch_info(), 0).is_err());

		assert!(extra.validate(&who, &free_call, &free_call.get_dispatch_info(), 0).is_ok());
		assert!(extra.pre_dispatch(&who, &free_call, &free_call.get_dispatch_info(), 0).is_ok());
	});
}

#[test]
fn dispenser_works_with_runtime_values() {
	PolimecNet::execute_with(|| {
		let who = PolimecAccountId::from(EMPTY_ACCOUNT);
		let did = "kilt:did:tz:tz1K7fCz9QJtXv3J8Ud3Zvz7eQ6";
		let bytes_did = did.as_bytes().to_vec();
		let bounded_did: Did = bytes_did.try_into().unwrap();
		let jwt = get_mock_jwt_with_cid(
			who.clone(),
			InvestorType::Retail,
			bounded_did,
			polimec_runtime::DispenserWhitelistedPolicy::get(),
		);
		PolimecBalances::force_set_balance(
			PolimecOrigin::root(),
			PolimecDispenser::dispense_account().into(),
			1000 * PLMC,
		)
		.unwrap();
		assert_ok!(PolimecDispenser::dispense(PolimecOrigin::signed(who.clone()), jwt));
		assert_eq!(PolimecBalances::free_balance(&who), 700 * PLMC);
		assert_eq!(
			PolimecBalances::usable_balance(who.clone()),
			<PolimecRuntime as pallet_dispenser::Config>::FreeDispenseAmount::get()
		);
		assert_eq!(
			PolimecVesting::vesting_balance(&who),
			Some(
				<PolimecRuntime as pallet_dispenser::Config>::InitialDispenseAmount::get() -
					<PolimecRuntime as pallet_dispenser::Config>::FreeDispenseAmount::get()
			)
		);
	})
}
