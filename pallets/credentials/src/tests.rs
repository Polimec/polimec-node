use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use polimec_traits::{Credential, MemberRole, PolimecMembers};

pub fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 4;
const EVE: AccountId = 5;

#[test]
fn add_during_genesis_works() {
	new_test_ext().execute_with(|| {
		assert!(Credentials::members(ALICE, MemberRole::Issuer).is_some());
		assert!(Credentials::is_in(&ALICE, &MemberRole::Issuer));

		assert!(Credentials::members(BOB, MemberRole::Retail).is_some());
		assert!(Credentials::is_in(&BOB, &MemberRole::Retail));

		assert!(Credentials::members(CHARLIE, MemberRole::Professional).is_some());
		assert!(Credentials::is_in(&CHARLIE, &MemberRole::Professional));

		assert!(Credentials::members(DAVE, MemberRole::Institutional).is_some());
		assert!(Credentials::is_in(&DAVE, &MemberRole::Institutional));
	})
}

#[test]
fn add_member_works() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_ok!(Credentials::add_member(RuntimeOrigin::root(), BOB, cred));
		assert_eq!(last_event(), RuntimeEvent::Credentials(crate::Event::MemberAdded));
	})
}

#[test]
fn only_root_can_add_member() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_noop!(Credentials::add_member(RuntimeOrigin::signed(ALICE), BOB, cred), BadOrigin);
	})
}

#[test]
fn cant_add_already_member() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_noop!(
			Credentials::add_member(RuntimeOrigin::root(), ALICE, cred),
			Error::<Test>::AlreadyMember
		);
	})
}

#[test]
fn remove_member_works() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_ok!(Credentials::remove_member(RuntimeOrigin::root(), ALICE, cred));
		assert_eq!(last_event(), RuntimeEvent::Credentials(crate::Event::MemberRemoved));
	})
}

#[test]
fn only_root_can_remove_member() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_noop!(
			Credentials::remove_member(RuntimeOrigin::signed(ALICE), ALICE, cred),
			BadOrigin
		);
	})
}

#[test]
fn cant_remove_not_a_member() {
	new_test_ext().execute_with(|| {
		let cred = Credential { role: MemberRole::Issuer, ..Default::default() };
		assert_noop!(
			Credentials::remove_member(RuntimeOrigin::root(), EVE, cred),
			Error::<Test>::NotMember
		);
	})
}