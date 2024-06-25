use super::*;
use frame_support::assert_err;

#[test]
fn para_id_for_project_can_be_set_by_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = inst.create_finished_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
		default_community_buys(),
		default_remainder_buys(),
	);

	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();
	inst.execute(|| {
		assert_ok!(crate::Pallet::<TestRuntime>::do_configure_receiver_pallet_migration(
			&ISSUER_1,
			project_id,
			ParaId::from(2006u32).into(),
		));
	});
	let project_details = inst.get_project_details(project_id);

	assert_eq!(
		project_details.migration_type,
		MigrationType::ParachainReceiverPallet(ParachainReceiverPalletInfo {
			parachain_id: ParaId::from(2006u32),
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Closed,
				polimec_to_project: ChannelStatus::Closed
			},
			migration_readiness_check: None,
		})
	);
}

#[test]
fn migration_config_cannot_be_set_by_anyone_but_issuer() {
	let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let project_id = inst.create_finished_project(
		default_project_metadata(ISSUER_1),
		ISSUER_1,
		default_evaluations(),
		default_bids(),
		default_community_buys(),
		default_remainder_buys(),
	);
	inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get() + 20u64).unwrap();

	inst.execute(|| {
		assert_err!(
			crate::Pallet::<TestRuntime>::do_configure_receiver_pallet_migration(
				&EVALUATOR_1,
				project_id,
				ParaId::from(2006u32),
			),
			Error::<TestRuntime>::NotIssuer
		);
		assert_err!(
			crate::Pallet::<TestRuntime>::do_configure_receiver_pallet_migration(
				&BIDDER_1,
				project_id,
				ParaId::from(2006u32),
			),
			Error::<TestRuntime>::NotIssuer
		);
		assert_err!(
			crate::Pallet::<TestRuntime>::do_configure_receiver_pallet_migration(
				&BUYER_1,
				project_id,
				ParaId::from(2006u32),
			),
			Error::<TestRuntime>::NotIssuer
		);
	});
	let project_details = inst.get_project_details(project_id);
	assert_eq!(project_details.migration_type, MigrationType::Offchain);
}
