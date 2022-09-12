use crate::{mock::*, Error, Proposal as ProposalStruct};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

#[test]
fn mock_works() {
	new_test_ext().execute_with(|| {
		// Only the `root` account can call the `register` function
		assert_ok!(Proposal::test(Origin::signed(1)));
	})
}

#[test]
fn execute_proposal_works() {
	new_test_ext().execute_with(|| {
		let proposal = ProposalStruct { call: todo!(), metadata: todo!() };
		assert_ok!(Proposal::add_proposal(proposal, 0xFFFFFFF));
	})
}
