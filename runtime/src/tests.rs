use super::{issuer_council, AccountId, Call, CurrencyId, VoteMajority};
use issuer_council::Call as CouncilCall;
use issuer_council::MajorityCount;
use sp_runtime::{Perbill, Permill};

#[test]
fn is_majority_should_work() {
	let alice: AccountId = Default::default();
	let missing_token_id: CurrencyId = Default::default();

	let admit_call = Call::IssuerCouncil(CouncilCall::admit_new_member(
		alice.clone(),
		alice.clone(),
		99_999,
		missing_token_id,
		Permill::from_percent(10),
	));
	assert_eq!(
		<VoteMajority as MajorityCount<Call>>::is_majority(
			admit_call.clone(),
			Perbill::from_percent(10),
			Perbill::from_percent(10)
		),
		false
	);
	assert!(<VoteMajority as MajorityCount<Call>>::is_majority(
		admit_call,
		Perbill::from_percent(11),
		Perbill::from_percent(10)
	));

	let expel_call = Call::IssuerCouncil(CouncilCall::expel_member(alice));
	assert_eq!(
		<VoteMajority as MajorityCount<Call>>::is_majority(
			expel_call.clone(),
			Perbill::from_percent(10),
			Perbill::from_percent(10)
		),
		false
	);
	assert!(<VoteMajority as MajorityCount<Call>>::is_majority(
		expel_call,
		Perbill::from_percent(75),
		Perbill::from_percent(25)
	));
}
