use crate::*;
use pallet_funding::{BidInfoOf, ContributionInfoOf, EvaluationInfoOf, RewardOrSlash};
use polimec_parachain_runtime::PolimecFunding;
use sp_runtime::{FixedPointNumber, Perquintill};
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

		inst.advance_time(<PolimecRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
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

#[test]
fn migration_is_sent() {
	let mut inst = IntegrationInstantiator::new(None);
	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(issuer(), 0),
			issuer(),
			vec![
				UserToUSDBalance::new(eval_1(), 50_000 * PLMC),
				UserToUSDBalance::new(eval_2(), 25_000 * PLMC),
				UserToUSDBalance::new(eval_3(), 32_000 * PLMC),
			],
			IntegrationInstantiator::generate_bids_from_total_usd(
				Perquintill::from_percent(40) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![eval_1(), bidder_2(), bidder_3(), bidder_4(), bidder_5()],
			),
			IntegrationInstantiator::generate_contributions_from_total_usd(
				Perquintill::from_percent(50) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![eval_1(), buyer_2(), buyer_3(), buyer_4(), buyer_5()],
			),
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

		inst.advance_time(<PolimecRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		dbg!(project_details.evaluation_round_info.evaluators_outcome);
		let evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		dbg!(evaluations);
	});

	// Migration is ready
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	// Migrate one user's contribution tokens. He evaluated, bid, and contributed
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::do_migrate_one_participant(eval_1(), project_id, eval_1()));
		println!("Polimec events:");
		dbg!(Polimec::events());
		let mut user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));

		assert!(user_evaluations.all(|evaluation| evaluation.ct_migration_sent));
		assert!(user_bids.all(|bid| bid.ct_migration_sent));
		assert!(user_contributions.all(|contribution| contribution.ct_migration_sent));
	});

	Penpal::execute_with(|| {
		println!("Penpal events:");
		dbg!(Penpal::events());
	});
}

#[test]
fn migration_is_executed_on_project() {
	let mut inst = IntegrationInstantiator::new(None);
	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(issuer(), 0),
			issuer(),
			vec![
				UserToUSDBalance::new(eval_1(), 50_000 * PLMC),
				UserToUSDBalance::new(eval_2(), 25_000 * PLMC),
				UserToUSDBalance::new(eval_3(), 32_000 * PLMC),
			],
			IntegrationInstantiator::generate_bids_from_total_usd(
				Perquintill::from_percent(40) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![eval_1(), bidder_2(), bidder_3(), bidder_4(), bidder_5()],
			),
			IntegrationInstantiator::generate_contributions_from_total_usd(
				Perquintill::from_percent(50) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![eval_1(), buyer_2(), buyer_3(), buyer_4(), buyer_5()],
			),
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

		inst.advance_time(<PolimecRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		dbg!(project_details.evaluation_round_info.evaluators_outcome);
		let evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		dbg!(evaluations);
	});

	// Migration is ready
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	// Migrate is sent
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::do_migrate_one_participant(eval_1(), project_id, eval_1()));
		println!("Polimec events:");
		dbg!(Polimec::events());
		let mut user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));

		assert!(user_evaluations.all(|evaluation| evaluation.ct_migration_sent));
		assert!(user_bids.all(|bid| bid.ct_migration_sent));
		assert!(user_contributions.all(|contribution| contribution.ct_migration_sent));
	});

	Penpal::execute_with(|| {
		dbg!(Penpal::events());
	});
}
