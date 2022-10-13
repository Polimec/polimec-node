use crate::{mock::*, Error, Project};
use frame_support::assert_ok;

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

const ALICE: AccountId = 1;
const BOB: AccountId = 2;

mod create {
	use frame_support::assert_noop;

	use crate::{ParticipantsSize, TicketSize};

	use super::*;

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project, 1));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::FundingModule(crate::Event::Created { project: 1, issuer: ALICE })
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
				FundingModule::create(Origin::signed(ALICE), project, 1),
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
				FundingModule::create(Origin::signed(ALICE), project, 1),
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
				FundingModule::create(Origin::signed(ALICE), project, 1),
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
				FundingModule::create(Origin::signed(ALICE), project, 1),
				Error::<Test>::TicketSizeError
			);
		})
	}
}

mod evaluation {

	use frame_support::assert_noop;

	use crate::{EvaluationStatus, ParticipantsSize, TicketSize};

	use super::*;
	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			let project = Project {
				minimum_price: 1,
				ticket_size: TicketSize { minimum: Some(1), maximum: None },
				participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
				..Default::default()
			};

			assert_ok!(FundingModule::create(Origin::signed(ALICE), project, 1));
			let project = FundingModule::evaluations(ALICE, 1);
			assert!(project.evaluation_status == EvaluationStatus::NotYetStarted);
			assert_noop!(
				FundingModule::bond(Origin::signed(BOB), ALICE, 1, 128),
				Error::<Test>::EvaluationNotStarted
			);
			assert_ok!(FundingModule::start_evaluation(Origin::signed(ALICE), 1));
			let project = FundingModule::evaluations(ALICE, 1);
			assert!(project.evaluation_status == EvaluationStatus::Started);
			assert_ok!(FundingModule::bond(Origin::signed(BOB), ALICE, 1, 128));
		})
	}
}
