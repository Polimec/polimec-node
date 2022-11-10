use crate::{mock::*, Error};
use frame_support::assert_ok;
use polimec_traits::{Credential, MemberRole, PolimecMembers};

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 4;

#[test]
fn genesis_works() {
	new_test_ext().execute_with(|| {
		assert!(Credentials::members(MemberRole::Issuer).len() == 1);
		assert!(Credentials::is_in(&ALICE, &MemberRole::Issuer));

		assert!(Credentials::members(MemberRole::Retail).len() == 1);
		assert!(Credentials::is_in(&BOB, &MemberRole::Retail));

		assert!(Credentials::members(MemberRole::Professional).len() == 1);
		assert!(Credentials::is_in(&CHARLIE, &MemberRole::Professional));

		assert!(Credentials::members(MemberRole::Institutional).len() == 1);
		assert!(Credentials::is_in(&DAVE, &MemberRole::Institutional));
	})
}

#[test]
fn add_works() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_ok!(Credentials::add_member(Origin::signed(ALICE), BOB, cred));
		assert_eq!(last_event(), Event::Credentials(crate::Event::MemberAdded));
	})
}
