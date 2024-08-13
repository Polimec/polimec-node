// Copyright (C) Parity Technologies (UK) Ltd.

// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// This library includes code from Substrate, which is licensed
// under both the GNU General Public License version 3 (GPLv3) and the
// Apache License 2.0. You may choose to redistribute and/or modify this
// code under either the terms of the GPLv3 or the Apache 2.0 License,
// whichever suits your needs.

//! The tests for cancelation functionality.

use super::*;

#[test]
fn cancel_referendum_should_work() {
	new_test_ext().execute_with(|| {
		let r = Democracy::inject_referendum(2, set_balance_proposal(2), VoteThreshold::SuperMajorityApprove, 0);
		assert_ok!(Democracy::vote(RuntimeOrigin::signed(1), r, aye(1)));
		assert_ok!(Democracy::cancel_referendum(RuntimeOrigin::root(), r));
		assert_eq!(Democracy::lowest_unbaked(), 0);

		next_block();

		next_block();

		assert_eq!(Democracy::lowest_unbaked(), 1);
		assert_eq!(Democracy::lowest_unbaked(), Democracy::referendum_count());
		assert_eq!(Balances::free_balance(42), 0);
	});
}

#[test]
fn emergency_cancel_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		let r = Democracy::inject_referendum(2, set_balance_proposal(2), VoteThreshold::SuperMajorityApprove, 2);
		assert!(Democracy::referendum_status(r).is_ok());

		assert_noop!(Democracy::emergency_cancel(RuntimeOrigin::signed(3), r), BadOrigin);
		assert_ok!(Democracy::emergency_cancel(RuntimeOrigin::signed(4), r));
		assert!(Democracy::referendum_info(r).is_none());

		// some time later...

		let r = Democracy::inject_referendum(2, set_balance_proposal(2), VoteThreshold::SuperMajorityApprove, 2);
		assert!(Democracy::referendum_status(r).is_ok());
		assert_noop!(Democracy::emergency_cancel(RuntimeOrigin::signed(4), r), Error::<Test>::AlreadyCanceled,);
	});
}
