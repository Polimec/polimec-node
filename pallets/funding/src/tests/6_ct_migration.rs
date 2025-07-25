use super::*;

mod offchain_migration {
	use super::*;

	#[test]
	fn start_offchain_migration() {
		let mut inst = MockInstantiator::default();
		let evaluations = inst.generate_successful_evaluations(default_project_metadata(ISSUER_1), 5);
		let bids = inst.generate_bids_from_total_ct_percent(default_project_metadata(ISSUER_1), 90, 10);
		// Create migrations for 2 projects, to check the `remaining_participants` is unaffected by other projects
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER_1),
			ISSUER_1,
			None,
			evaluations.clone(),
			bids.clone(),
		);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);

		let project_id =
			inst.create_finished_project(default_project_metadata(ISSUER_1), ISSUER_1, None, evaluations, bids);
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
		assert_eq!(inst.execute(|| UnmigratedCounter::<TestRuntime>::get(project_id)), 15);
		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
	}

	fn create_offchain_migration_project(mut inst: MockInstantiator) -> (ProjectId, MockInstantiator) {
		let project_metadata = default_project_metadata(ISSUER_1);
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 90, 10);
		let project_id = inst.create_finished_project(project_metadata, ISSUER_1, None, evaluations, bids);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));
		inst.settle_project(project_id, true);
		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, ISSUER_1,));
		});
		(project_id, inst)
	}

	#[test]
	fn confirm_offchain_migration() {
		let inst = MockInstantiator::default();
		let (project_id, mut inst) = create_offchain_migration_project(inst);
		let bidder_1 = inst.account_from_u32(0, "BIDDER");

		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, bidder_1))).unwrap();
		assert_eq!(bidder_1_migrations.0, MigrationStatus::NotStarted);

		inst.execute(|| {
			assert_ok!(crate::Pallet::<TestRuntime>::do_confirm_offchain_migration(project_id, ISSUER_1, bidder_1));
		});

		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, bidder_1))).unwrap();
		assert_eq!(bidder_1_migrations.0, MigrationStatus::Confirmed);
	}

	#[test]
	fn mark_project_migration_as_finished() {
		let inst = MockInstantiator::default();
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
