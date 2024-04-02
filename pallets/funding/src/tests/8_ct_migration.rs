use super::*;
use frame_support::assert_err;

#[test]
fn para_id_for_project_can_be_set_by_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = inst.create_finished_project(
		default_project_metadata(inst.get_new_nonce(), ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
		default_community_buys(),
		default_remainder_buys(),
	);

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
	inst.execute(|| {
		assert_ok!(crate::Pallet::<TestRuntime>::do_set_para_id_for_project(
			&ISSUER_1,
			project_id,
			ParaId::from(2006u32),
		));
	});
	let project_details = inst.get_project_details(project_id);
	assert_eq!(project_details.parachain_id, Some(ParaId::from(2006u32)));
}

#[test]
fn para_id_for_project_cannot_be_set_by_anyone_but_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = inst.create_finished_project(
		default_project_metadata(inst.get_new_nonce(), ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
		default_community_buys(),
		default_remainder_buys(),
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();

	inst.execute(|| {
		assert_err!(
			crate::Pallet::<TestRuntime>::do_set_para_id_for_project(&EVALUATOR_1, project_id, ParaId::from(2006u32),),
			Error::<TestRuntime>::NotAllowed
		);
		assert_err!(
			crate::Pallet::<TestRuntime>::do_set_para_id_for_project(&BIDDER_1, project_id, ParaId::from(2006u32),),
			Error::<TestRuntime>::NotAllowed
		);
		assert_err!(
			crate::Pallet::<TestRuntime>::do_set_para_id_for_project(&BUYER_1, project_id, ParaId::from(2006u32),),
			Error::<TestRuntime>::NotAllowed
		);
	});
	let project_details = inst.get_project_details(project_id);
	assert_eq!(project_details.parachain_id, None);
}
