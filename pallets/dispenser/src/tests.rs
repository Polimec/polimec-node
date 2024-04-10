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
use crate as pallet_dispenser;
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
			// Test that the initial dispense amount is set to the InitialDispenseAmount constant.
			assert_eq!(DispenseAmount::<Test>::get(), <Test as pallet_dispenser::Config>::InitialDispenseAmount::get());
		});
	}

	/// Test that only the Admin can change the dispense amount.
	#[test]
	fn only_admin_can_change_dispense_amount() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(Dispenser::set_dispense_amount(RuntimeOrigin::signed(1), 100), DispatchError::BadOrigin);
			assert_ok!(Dispenser::set_dispense_amount(RuntimeOrigin::signed(Admin::get()), 1000));
			assert_eq!(DispenseAmount::<Test>::get(), 1000);
		});
	}

	#[test]
	fn amount_has_to_be_higher_then_free_amount() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(
				Dispenser::set_dispense_amount(
					RuntimeOrigin::signed(Admin::get()),
					<Test as pallet_dispenser::Config>::FreeDispenseAmount::get()
				),
				Error::<Test>::DispenseAmountTooLow
			);
		});
	}
}

mod dispense {
	use super::*;
	#[test]
	fn user_can_dispense_for_free() {
		ExtBuilder::default().build().execute_with(|| {
			// User has no balance.
			assert_eq!(Balances::free_balance(1), 0);
			// User can dispense tokens for free.
			let jwt = get_mock_jwt(1, InvestorType::Retail, generate_did_from_account(1));
			assert_ok!(Dispenser::dispense(RuntimeOrigin::signed(1), jwt));

			// Tokens are dispensed and locked.
			assert_eq!(Balances::free_balance(1), <Test as pallet_dispenser::Config>::InitialDispenseAmount::get());
			assert_eq!(Balances::usable_balance(1), <Test as pallet_dispenser::Config>::FreeDispenseAmount::get());
			assert_eq!(
				Vesting::vesting_balance(&1),
				Some(
					<Test as pallet_dispenser::Config>::InitialDispenseAmount::get() -
						<Test as pallet_dispenser::Config>::FreeDispenseAmount::get()
				)
			);
		});
	}

	#[test]
	fn user_cannot_dispense_twice() {
		ExtBuilder::default().build().execute_with(|| {
			let jwt = get_mock_jwt(1, InvestorType::Retail, generate_did_from_account(1));
			assert_ok!(Dispenser::dispense(RuntimeOrigin::signed(1), jwt.clone()));
			assert_noop!(Dispenser::dispense(RuntimeOrigin::signed(1), jwt), Error::<Test>::DispensedAlreadyToDid);
		});
	}

	#[test]
	fn correct_amount_received_after_dispense_amount_changed() {
		ExtBuilder::default().dispense_account(2).build().execute_with(|| {
			let jwt = get_mock_jwt(1, InvestorType::Retail, generate_did_from_account(1));
			assert_ok!(Dispenser::dispense(RuntimeOrigin::signed(1), jwt));
			assert_eq!(Balances::free_balance(1), <Test as pallet_dispenser::Config>::InitialDispenseAmount::get());
			assert_eq!(Balances::usable_balance(1), <Test as pallet_dispenser::Config>::FreeDispenseAmount::get());
			assert_eq!(
				Vesting::vesting_balance(&1),
				Some(
					<Test as pallet_dispenser::Config>::InitialDispenseAmount::get() -
						<Test as pallet_dispenser::Config>::FreeDispenseAmount::get()
				)
			);

			// Change the dispense amount.
			let new_amount: BalanceOf<Test> = 50u32.into();
			assert_ok!(Dispenser::set_dispense_amount(RuntimeOrigin::signed(Admin::get()), new_amount));
			let jwt = get_mock_jwt(2, InvestorType::Retail, generate_did_from_account(2));
			assert_ok!(Dispenser::dispense(RuntimeOrigin::signed(2), jwt));
			assert_eq!(Balances::free_balance(2), new_amount);
			assert_eq!(Balances::usable_balance(2), <Test as pallet_dispenser::Config>::FreeDispenseAmount::get());
			assert_eq!(
				Vesting::vesting_balance(&2),
				Some(new_amount - <Test as pallet_dispenser::Config>::FreeDispenseAmount::get())
			);
		});
	}

	#[test]
	fn x_users_dispense_until_dispenser_is_empty() {
		let x = 10;
		ExtBuilder::default().dispense_account(x).build().execute_with(|| {
			assert_eq!(
				Balances::free_balance(Dispenser::dispense_account()),
				x * <Test as pallet_dispenser::Config>::InitialDispenseAmount::get()
			);
			for i in 1..=x {
				let jwt = get_mock_jwt(i, InvestorType::Retail, generate_did_from_account(i));
				assert_ok!(Dispenser::dispense(RuntimeOrigin::signed(i), jwt));
				assert_eq!(Balances::free_balance(i), <Test as pallet_dispenser::Config>::InitialDispenseAmount::get());
				assert_eq!(Balances::usable_balance(i), <Test as pallet_dispenser::Config>::FreeDispenseAmount::get());
				assert_eq!(
					Vesting::vesting_balance(&i),
					Some(
						<Test as pallet_dispenser::Config>::InitialDispenseAmount::get() -
							<Test as pallet_dispenser::Config>::FreeDispenseAmount::get()
					)
				);
			}
			// Dispenser is empty.
			assert_noop!(
				Dispenser::dispense(
					RuntimeOrigin::signed(x + 1),
					get_mock_jwt(x + 1, InvestorType::Retail, generate_did_from_account(x + 1))
				),
				Error::<Test>::DispenserDepleted
			);
		});
	}
}
