use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

#[test]
fn must_be_root() {
	new_test_ext().execute_with(|| {
		// Only the `root` account can call the `register` function
		assert_ok!(Proposal::test(Origin::signed(1)));
	})
}
