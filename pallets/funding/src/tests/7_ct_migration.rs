use super::*;
use frame_support::{assert_err, traits::fungibles::Inspect};
use sp_runtime::bounded_vec;
use xcm::v4::MaxPalletNameLen;

mod pallet_migration {
	use super::*;

	#[test]
	fn start_pallet_migration() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);

		inst.execute(|| {
			assert_err!(
				crate::Pallet::<TestRuntime>::do_start_pallet_migration(
					&EVALUATOR_1,
					project_id,
					ParaId::from(2006u32),
				),
				Error::<TestRuntime>::NotIssuer
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::do_start_pallet_migration(&BIDDER_1, project_id, ParaId::from(2006u32),),
				Error::<TestRuntime>::NotIssuer
			);
			assert_err!(
				crate::Pallet::<TestRuntime>::do_start_pallet_migration(&BUYER_1, project_id, ParaId::from(2006u32),),
				Error::<TestRuntime>::NotIssuer
			);
			assert_ok!(crate::Pallet::<TestRuntime>::do_start_pallet_migration(
				&ISSUER_1,
				project_id,
				ParaId::from(2006u32),
			));
		});

		let project_details = inst.get_project_details(project_id);
		assert_eq!(
			project_details.migration_type,
			Some(MigrationType::Pallet(PalletMigrationInfo {
				parachain_id: 2006.into(),
				hrmp_channel_status: HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Closed,
					polimec_to_project: ChannelStatus::Closed
				},
				migration_readiness_check: None,
			}))
		);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
		assert_eq!(inst.execute(|| UnmigratedCounter::<TestRuntime>::get(project_id)), 10);
	}

	fn create_pallet_migration_project(mut inst: MockInstantiator) -> (ProjectId, MockInstantiator) {
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);
		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_start_pallet_migration(
				&ISSUER_1,
				project_id,
				ParaId::from(6969u32)
			));
		});
		(project_id, inst)
	}

	fn fake_hrmp_establishment() {
		// Notification sent by the relay when the project starts a project->polimec channel
		const SENDER: u32 = 6969;

		// This makes Polimec send an acceptance + open channel (polimec->project) message back to the relay
		assert_ok!(PolimecFunding::do_handle_channel_open_request(SENDER, 50_000, 8));

		// Finally the relay notifies the channel Polimec->project has been accepted by the project
		// We set the hrmp flags as "Open" and start the receiver pallet check
		assert_ok!(PolimecFunding::do_handle_channel_accepted(SENDER));
	}

	#[test]
	fn automatic_hrmp_establishment() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, mut inst) = create_pallet_migration_project(inst);

		inst.execute(fake_hrmp_establishment);

		let project_details = inst.get_project_details(project_id);
		assert_eq!(
			project_details.migration_type,
			Some(MigrationType::Pallet(PalletMigrationInfo {
				parachain_id: 6969.into(),
				hrmp_channel_status: HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Open,
					polimec_to_project: ChannelStatus::Open
				},
				migration_readiness_check: Some(PalletMigrationReadinessCheck {
					holding_check: (0, CheckOutcome::AwaitingResponse),
					pallet_check: (1, CheckOutcome::AwaitingResponse)
				}),
			}))
		);
	}

	/// Check that the polimec sovereign account has the ct issuance on the project chain, and the receiver pallet is in
	/// the runtime.
	#[test]
	fn pallet_readiness_check() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, mut inst) = create_pallet_migration_project(inst);
		inst.execute(fake_hrmp_establishment);

		// At this point, we sent the pallet check xcm to the project chain, and we are awaiting a query response message.
		// query id 0 is the CT balance of the Polimec SA
		// query id 1 is the existence of the receiver pallet

		// We simulate the response from the project chain
		let ct_issuance =
			inst.execute(|| <TestRuntime as crate::Config>::ContributionTokenCurrency::total_issuance(project_id));
		let ct_assets: Assets = vec![Asset {
			id: AssetId(Location::new(1u8, [Parachain(6969u32)])),
			fun: Fungibility::Fungible(ct_issuance),
		}]
		.into();

		inst.execute(|| {
			assert_ok!(PolimecFunding::do_pallet_migration_readiness_response(
				Location::new(1u8, [Parachain(6969u32)]),
				0u64,
				Response::Assets(ct_assets),
			));
		});

		let module_name: BoundedVec<u8, MaxPalletNameLen> =
			BoundedVec::try_from("polimec_receiver".as_bytes().to_vec()).unwrap();
		let pallet_info = xcm::v4::PalletInfo::new(
			// index is used for future `Transact` calls to the pallet for migrating a user
			69,
			// Doesn't matter
			module_name.to_vec(),
			// Main check that the receiver pallet is there
			module_name.to_vec(),
			// These might be useful in the future, but not for now
			0,
			0,
			0,
		)
		.unwrap();
		inst.execute(|| {
			assert_ok!(PolimecFunding::do_pallet_migration_readiness_response(
				Location::new(1u8, [Parachain(6969u32)]),
				1u64,
				Response::PalletsInfo(bounded_vec![pallet_info]),
			));
		});

		let project_details = inst.get_project_details(project_id);
		if let MigrationType::Pallet(info) = project_details.migration_type.unwrap() {
			assert_eq!(info.migration_readiness_check.unwrap().holding_check.1, CheckOutcome::Passed(None));
			assert_eq!(info.migration_readiness_check.unwrap().pallet_check.1, CheckOutcome::Passed(Some(69)));
		} else {
			panic!("Migration type is not Pallet")
		}
	}
}

mod offchain_migration {
	use super::*;

	#[test]
	fn start_offchain_migration() {
		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		// Create migrations for 2 projects, to check the `remaining_participants` is unaffected by other projects
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);

		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);

		inst.execute(|| {
			assert_err!(
				crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, EVALUATOR_1,),
				Error::<TestRuntime>::NotIssuer
			);

			assert_ok!(crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, ISSUER_1,));
		});

		let project_details = inst.get_project_details(project_id);
		assert_eq!(inst.execute(|| UnmigratedCounter::<TestRuntime>::get(project_id)), 10);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
	}

	fn create_offchain_migration_project(mut inst: MockInstantiator) -> (ProjectId, MockInstantiator) {
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);
		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, ISSUER_1,));
		});
		(project_id, inst)
	}

	#[test]
	fn confirm_offchain_migration() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, mut inst) = create_offchain_migration_project(inst);

		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, BIDDER_1))).unwrap();
		assert_eq!(bidder_1_migrations.0, MigrationStatus::NotStarted);

		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_confirm_offchain_migration(project_id, ISSUER_1, BIDDER_1));
		});

		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, BIDDER_1))).unwrap();
		assert_eq!(bidder_1_migrations.0, MigrationStatus::Confirmed);
	}

	#[test]
	fn mark_project_migration_as_finished() {
		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
		let (project_id, mut inst) = create_offchain_migration_project(inst);

		let participants = inst.execute(|| UserMigrations::<TestRuntime>::iter_key_prefix((project_id,)).collect_vec());
		for participant in participants {
			inst.execute(|| {
				assert_ok!(crate::Pallet::<TestRuntime>::do_confirm_offchain_migration(
					project_id,
					ISSUER_1,
					participant
				));
			});
		}

		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_mark_project_ct_migration_as_finished(project_id));
		});
	}
}
