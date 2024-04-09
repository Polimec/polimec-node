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

// If you feel like getting in touch with us, you can do so at info@polimec.org

use super::*;
use crate as pallet_faucet;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use polimec_common::credentials::InvestorType;
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};
use sp_runtime::DispatchError;

mod admin {
	use super::*;
	#[test]
	fn initial_amount_is_correctly_set() {
		ExtBuilder::default().build().execute_with(|| {
			// Test that the initial claiming amount is set to the InitialClaimAmount constant.
			assert_eq!(ClaimAmount::<Test>::get(), <Test as pallet_faucet::Config>::InitialClaimAmount::get());
		});
	}

	/// Test that only the Admin can change the claiming amount.
	#[test]
	fn only_admin_can_change_claiming_amount() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(Faucet::set_claiming_amount(RuntimeOrigin::signed(1), 100), DispatchError::BadOrigin);
			assert_ok!(Faucet::set_claiming_amount(RuntimeOrigin::signed(Admin::get()), 1000));
			assert_eq!(ClaimAmount::<Test>::get(), 1000);
		});
	}
}

mod claim {
	use super::*;
	#[test]
	fn user_can_claim_for_free() {
		ExtBuilder::default().build().execute_with(|| {
			// User has no balance.
			assert_eq!(Balances::free_balance(1), 0);
			// User can claim tokens for free.
			let jwt = get_mock_jwt(1, InvestorType::Retail, generate_did_from_account(1));
			assert_ok!(Faucet::claim(RuntimeOrigin::signed(1), jwt));

			// Tokens are claimed and locked.
			assert_eq!(Balances::free_balance(1), <Test as pallet_faucet::Config>::InitialClaimAmount::get());
			assert_eq!(Balances::usable_balance(1), 0);
			assert_eq!(Vesting::vesting_balance(&1), Some(<Test as pallet_faucet::Config>::InitialClaimAmount::get()));
		});
	}

	#[test]
	fn user_cannot_claim_twice() {
		ExtBuilder::default().build().execute_with(|| {
			let jwt = get_mock_jwt(1, InvestorType::Retail, generate_did_from_account(1));
			assert_ok!(Faucet::claim(RuntimeOrigin::signed(1), jwt.clone()));
			assert_noop!(Faucet::claim(RuntimeOrigin::signed(1), jwt), Error::<Test>::DidAlreadyClaimed);
		});
	}

	#[test]
	fn x_users_claim_until_faucet_is_empty() {
		let x = 10;
		ExtBuilder::default().claiming_account(x).build().execute_with(|| {
			assert_eq!(
				Balances::free_balance(Faucet::claiming_account()),
				x * <Test as pallet_faucet::Config>::InitialClaimAmount::get()
			);
			for i in 1..=x {
				let jwt = get_mock_jwt(i, InvestorType::Retail, generate_did_from_account(i));
				assert_ok!(Faucet::claim(RuntimeOrigin::signed(i), jwt));
				assert_eq!(Balances::free_balance(i), <Test as pallet_faucet::Config>::InitialClaimAmount::get());
				assert_eq!(Balances::usable_balance(i), 0);
				assert_eq!(
					Vesting::vesting_balance(&i),
					Some(<Test as pallet_faucet::Config>::InitialClaimAmount::get())
				);
			}
			// Faucet is empty.
			assert_noop!(
				Faucet::claim(
					RuntimeOrigin::signed(x + 1),
					get_mock_jwt(x + 1, InvestorType::Retail, generate_did_from_account(x + 1))
				),
				Error::<Test>::FaucetDepleted
			);
		});
	}
}
