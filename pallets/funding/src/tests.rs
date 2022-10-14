use crate::{mock::*, Error, Project};
use frame_support::assert_ok;

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;

mod creation_phase {
	use super::*;
	use crate::{ParticipantsSize, TicketSize};
	use frame_support::assert_noop;

	#[test]
	fn create_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::FundingModule(crate::Event::Created { project_id: 0, issuer: ALICE })
			);
		})
	}

	#[test]
	fn price_too_low() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::PriceTooLow
			);
		})
	}

	#[test]
	fn participants_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::ParticipantsSizeError
			);
		})
	}

	#[test]
	fn ticket_size_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: Some(1), maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}

	#[test]
	#[ignore = "ATM only the first error will be thrown"]
	fn multiple_field_error() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 0,
				ticket_size: TicketSize { minimum: None, maximum: None },
				participants_size: ParticipantsSize { minimum: None, maximum: None },
				..Default::default()
			};

			assert_noop!(
				FundingModule::create(Origin::signed(ALICE), project),
				Error::<Test>::TicketSizeError
			);
		})
	}
}

mod evaluation_phase {
	use super::*;
	use crate::{EvaluationStatus, ParticipantsSize, TicketSize};
	use frame_support::assert_noop;

	#[test]
	fn start_evaluation_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			let project = FundingModule::evaluations(ALICE, 0);
			assert!(project.evaluation_status == EvaluationStatus::NotYetStarted);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			let project = FundingModule::evaluations(ALICE, 0);
			assert!(project.evaluation_status == EvaluationStatus::Started);
		})
	}

	#[test]
	fn bond_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), ALICE, 0, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));
			assert_ok!(FundingModule::bond(Origin::signed(BOB), ALICE, 0, 128));
		})
	}

	#[test]
	fn cannot_bond() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};
			assert_ok!(FundingModule::create(Origin::signed(ALICE), project));
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 0));

			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), ALICE, 0, 1024),
				Error::<Test>::InsufficientBalance
			);
		})
	}
}
