use crate::*;
use polimec_parachain_runtime::PolimecFunding;
use tests::defaults::*;

#[test]
fn migration_check() {
	let mut inst = IntegrationInstantiator::new(None);
	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(issuer(), 0),
			issuer(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			vec![],
		)
	});
	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

	// Mock HRMP establishment
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::do_set_para_id_for_project(&issuer(), project_id, ParaId::from(6969u32)));

		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));

		inst.advance_time(10u32.into()).unwrap();
	});

	Penpal::execute_with(|| {
		println!("penpal events:");
		dbg!(Penpal::events());
	});

	Polimec::execute_with(|| {
		println!("Polimec events:");
		dbg!(Polimec::events());

		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});
}
