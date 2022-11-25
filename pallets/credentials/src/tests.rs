use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, error::BadOrigin, BoundedVec};
use polimec_traits::{Big4, Country, Credential, MemberRole, PolimecMembers};

pub fn last_event() -> RuntimeEvent {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;
const CHARLIE: AccountId = 3;
const DAVE: AccountId = 4;
const EVE: AccountId = 5;

fn new_test_credential(role: MemberRole) -> Credential {
	Credential {
		issuer: Big4::KPMG,
		role,
		domicile: BoundedVec::default(),
		country: Country::Switzerland,
		date_of_birth: 10,
	}
}

#[test]
fn add_during_genesis_works() {
	new_test_ext().execute_with(|| {
		assert!(Credentials::members(MemberRole::Issuer, ALICE).is_some());
		assert!(Credentials::is_in(&MemberRole::Issuer, &ALICE));

		assert!(Credentials::members(MemberRole::Retail, BOB).is_some());
		assert!(Credentials::is_in(&MemberRole::Retail, &BOB));

		assert!(Credentials::members(MemberRole::Professional, CHARLIE).is_some());
		assert!(Credentials::is_in(&MemberRole::Professional, &CHARLIE));

		assert!(Credentials::members(MemberRole::Institutional, DAVE).is_some());
		assert!(Credentials::is_in(&MemberRole::Institutional, &DAVE));
	})
}

#[test]
fn add_member_works() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_ok!(Credentials::add_member(RuntimeOrigin::root(), cred, BOB));
		assert_eq!(last_event(), RuntimeEvent::Credentials(crate::Event::MemberAdded));
	})
}

#[test]
fn only_root_can_add_member() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_noop!(Credentials::add_member(RuntimeOrigin::signed(ALICE), cred, BOB), BadOrigin);
	})
}

#[test]
fn cant_add_already_member() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_noop!(
			Credentials::add_member(RuntimeOrigin::root(), cred, ALICE),
			Error::<Test>::AlreadyMember
		);
	})
}

#[test]
fn remove_member_works() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_ok!(Credentials::remove_member(RuntimeOrigin::root(), cred, ALICE));
		assert_eq!(last_event(), RuntimeEvent::Credentials(crate::Event::MemberRemoved));
	})
}

#[test]
fn only_root_can_remove_member() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_noop!(
			Credentials::remove_member(RuntimeOrigin::signed(ALICE), cred, ALICE),
			BadOrigin
		);
	})
}

#[test]
fn cant_remove_not_a_member() {
	new_test_ext().execute_with(|| {
		let cred = new_test_credential(MemberRole::Issuer);
		assert_noop!(
			Credentials::remove_member(RuntimeOrigin::root(), cred, EVE),
			Error::<Test>::NotMember
		);
	})
}

#[test]
fn get_members_of_works() {
	new_test_ext().execute_with(|| {
		let issuers = Credentials::get_members_of(&MemberRole::Issuer);
		assert!(issuers == vec![1]);
		let cred = new_test_credential(MemberRole::Issuer);
		let _ = Credentials::add_member(RuntimeOrigin::root(), cred.clone(), BOB);
		let issuers = Credentials::get_members_of(&MemberRole::Issuer);
		assert!(issuers == vec![1, 2]);
		let _ = Credentials::remove_member(RuntimeOrigin::root(), cred, ALICE);
		let issuers = Credentials::get_members_of(&MemberRole::Issuer);
		assert!(issuers == vec![2]);
	})
}

#[test]
fn get_roles_of_works() {
	new_test_ext().execute_with(|| {
		let roles = Credentials::get_roles_of(&EVE);
		let expected_roles = vec![MemberRole::Professional, MemberRole::Institutional];
		assert!(roles.len() == 2);
		assert!(roles == expected_roles);
	})
}

#[test]
fn get_roles_of_not_user() {
	new_test_ext().execute_with(|| {
		let roles = Credentials::get_roles_of(&6);
		let expected_roles: Vec<MemberRole> = vec![];
		assert!(roles.is_empty());
		assert!(roles == expected_roles);
	})
}
