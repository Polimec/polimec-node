// use super::*;
// use frame_support::{assert_err, traits::fungibles::Inspect};
// use sp_runtime::bounded_vec;
// use xcm::latest::MaxPalletNameLen;
//
// mod pallet_migration {
// 	use super::*;
//
// 	#[test]
// 	fn start_pallet_migration() {
// 		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		let project_id = inst.create_finished_project(
// 			default_project_metadata(ISSUER_1),
// 			ISSUER_1,
// 			None,
// 			default_evaluations(),
// 			default_bids(),
// 			default_community_buys(),
// 			default_remainder_buys(),
// 		);
// 		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 		inst.settle_project(project_id).unwrap();
//
// 		inst.execute(|| {
// 			assert_err!(
// 				crate::Pallet::<TestRuntime>::do_start_pallet_migration(
// 					&EVALUATOR_1,
// 					project_id,
// 					ParaId::from(2006u32),
// 				),
// 				Error::<TestRuntime>::NotIssuer
// 			);
// 			assert_err!(
// 				crate::Pallet::<TestRuntime>::do_start_pallet_migration(&BIDDER_1, project_id, ParaId::from(2006u32),),
// 				Error::<TestRuntime>::NotIssuer
// 			);
// 			assert_err!(
// 				crate::Pallet::<TestRuntime>::do_start_pallet_migration(&BUYER_1, project_id, ParaId::from(2006u32),),
// 				Error::<TestRuntime>::NotIssuer
// 			);
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_start_pallet_migration(
// 				&ISSUER_1,
// 				project_id,
// 				ParaId::from(2006u32).into(),
// 			));
// 		});
//
// 		let project_details = inst.get_project_details(project_id);
// 		assert_eq!(
// 			project_details.migration_type,
// 			Some(MigrationType::Pallet(PalletMigrationInfo {
// 				parachain_id: 2006.into(),
// 				hrmp_channel_status: HRMPChannelStatus {
// 					project_to_polimec: ChannelStatus::Closed,
// 					polimec_to_project: ChannelStatus::Closed
// 				},
// 				migration_readiness_check: None,
// 			}))
// 		);
// 		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
// 		assert_eq!(inst.execute(|| UnmigratedCounter::<TestRuntime>::get(project_id)), 10);
// 	}
//
// 	fn create_pallet_migration_project(mut inst: MockInstantiator) -> (ProjectId, MockInstantiator) {
// 		let project_id = inst.create_finished_project(
// 			default_project_metadata(ISSUER_1),
// 			ISSUER_1,
// 			None,
// 			default_evaluations(),
// 			default_bids(),
// 			default_community_buys(),
// 			default_remainder_buys(),
// 		);
// 		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 		inst.settle_project(project_id).unwrap();
// 		inst.execute(|| {
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_start_pallet_migration(
// 				&ISSUER_1,
// 				project_id,
// 				ParaId::from(6969u32)
// 			));
// 		});
// 		(project_id, inst)
// 	}
//
// 	fn fake_hrmp_establishment() {
// 		// Notification sent by the relay when the project starts a project->polimec channel
// 		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
// 			sender: 6969,
// 			max_message_size: 102_300,
// 			max_capacity: 1000,
// 		};
// 		// This makes Polimec send an acceptance + open channel (polimec->project) message back to the relay
// 		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));
//
// 		// Finally the relay notifies the channel polimec->project has been accepted by the project
// 		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
//
// 		// We set the hrmp flags as "Open" and start the receiver pallet check
// 		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));
// 	}
//
// 	#[test]
// 	fn automatic_hrmp_establishment() {
// 		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		let (project_id, mut inst) = create_pallet_migration_project(inst);
//
// 		inst.execute(|| fake_hrmp_establishment());
//
// 		let project_details = inst.get_project_details(project_id);
// 		assert_eq!(
// 			project_details.migration_type,
// 			Some(MigrationType::Pallet(PalletMigrationInfo {
// 				parachain_id: 6969.into(),
// 				hrmp_channel_status: HRMPChannelStatus {
// 					project_to_polimec: ChannelStatus::Open,
// 					polimec_to_project: ChannelStatus::Open
// 				},
// 				migration_readiness_check: Some(PalletMigrationReadinessCheck {
// 					holding_check: (0, CheckOutcome::AwaitingResponse),
// 					pallet_check: (1, CheckOutcome::AwaitingResponse)
// 				}),
// 			}))
// 		);
// 	}
//
// 	/// Check that the polimec sovereign account has the ct issuance on the project chain, and the receiver pallet is in
// 	/// the runtime.
// 	#[test]
// 	fn pallet_readiness_check() {
// 		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		let (project_id, mut inst) = create_pallet_migration_project(inst);
// 		inst.execute(|| fake_hrmp_establishment());
//
// 		// At this point, we sent the pallet check xcm to the project chain, and we are awaiting a query response message.
// 		// query id 0 is the CT balance of the Polimec SA
// 		// query id 1 is the existence of the receiver pallet
//
// 		// We simulate the response from the project chain
// 		let ct_issuance =
// 			inst.execute(|| <TestRuntime as crate::Config>::ContributionTokenCurrency::total_issuance(project_id));
// 		let ct_multiassets: MultiAssets = vec![MultiAsset {
// 			id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(6969)) }),
// 			fun: Fungibility::Fungible(ct_issuance),
// 		}]
// 		.into();
//
// 		inst.execute(|| {
// 			assert_ok!(PolimecFunding::do_pallet_migration_readiness_response(
// 				MultiLocation::new(1u8, X1(Parachain(6969u32))),
// 				0u64,
// 				Response::Assets(ct_multiassets),
// 			));
// 		});
//
// 		let module_name: BoundedVec<u8, MaxPalletNameLen> =
// 			BoundedVec::try_from("polimec_receiver".as_bytes().to_vec()).unwrap();
// 		let pallet_info = xcm::latest::PalletInfo {
// 			// index is used for future `Transact` calls to the pallet for migrating a user
// 			index: 69,
// 			// Doesn't matter
// 			name: module_name.clone(),
// 			// Main check that the receiver pallet is there
// 			module_name,
// 			// These might be useful in the future, but not for now
// 			major: 0,
// 			minor: 0,
// 			patch: 0,
// 		};
// 		inst.execute(|| {
// 			assert_ok!(PolimecFunding::do_pallet_migration_readiness_response(
// 				MultiLocation::new(1u8, X1(Parachain(6969u32))),
// 				1u64,
// 				Response::PalletsInfo(bounded_vec![pallet_info]),
// 			));
// 		});
//
// 		let project_details = inst.get_project_details(project_id);
// 		if let MigrationType::Pallet(info) = project_details.migration_type.unwrap() {
// 			assert_eq!(info.migration_readiness_check.unwrap().holding_check.1, CheckOutcome::Passed(None));
// 			assert_eq!(info.migration_readiness_check.unwrap().pallet_check.1, CheckOutcome::Passed(Some(69)));
// 		} else {
// 			panic!("Migration type is not Pallet")
// 		}
// 	}
// }
//
// mod offchain_migration {
// 	use super::*;
//
// 	#[test]
// 	fn start_offchain_migration() {
// 		let mut inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		// Create migrations for 2 projects, to check the `remaining_participants` is unaffected by other projects
// 		let project_id = inst.create_finished_project(
// 			default_project_metadata(ISSUER_1),
// 			ISSUER_1,
// 			None,
// 			default_evaluations(),
// 			default_bids(),
// 			default_community_buys(),
// 			default_remainder_buys(),
// 		);
// 		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 		inst.settle_project(project_id).unwrap();
//
// 		let project_id = inst.create_finished_project(
// 			default_project_metadata(ISSUER_1),
// 			ISSUER_1,
// 			None,
// 			default_evaluations(),
// 			default_bids(),
// 			default_community_buys(),
// 			default_remainder_buys(),
// 		);
// 		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 		inst.settle_project(project_id).unwrap();
//
// 		inst.execute(|| {
// 			assert_err!(
// 				crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, EVALUATOR_1,),
// 				Error::<TestRuntime>::NotIssuer
// 			);
//
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, ISSUER_1,));
// 		});
//
// 		let project_details = inst.get_project_details(project_id);
// 		assert_eq!(inst.execute(|| UnmigratedCounter::<TestRuntime>::get(project_id)), 10);
// 		assert_eq!(project_details.status, ProjectStatus::CTMigrationStarted);
// 	}
//
// 	fn create_offchain_migration_project(mut inst: MockInstantiator) -> (ProjectId, MockInstantiator) {
// 		let project_id = inst.create_finished_project(
// 			default_project_metadata(ISSUER_1),
// 			ISSUER_1,
// 			None,
// 			default_evaluations(),
// 			default_bids(),
// 			default_community_buys(),
// 			default_remainder_buys(),
// 		);
// 		inst.advance_time(<TestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();
// 		inst.settle_project(project_id).unwrap();
// 		inst.execute(|| {
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_start_offchain_migration(project_id, ISSUER_1,));
// 		});
// 		(project_id, inst)
// 	}
//
// 	#[test]
// 	fn confirm_offchain_migration() {
// 		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		let (project_id, mut inst) = create_offchain_migration_project(inst);
//
// 		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, BIDDER_1))).unwrap();
// 		assert_eq!(bidder_1_migrations.0, MigrationStatus::NotStarted);
//
// 		inst.execute(|| {
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_confirm_offchain_migration(project_id, ISSUER_1, BIDDER_1));
// 		});
//
// 		let bidder_1_migrations = inst.execute(|| UserMigrations::<TestRuntime>::get((project_id, BIDDER_1))).unwrap();
// 		assert_eq!(bidder_1_migrations.0, MigrationStatus::Confirmed);
// 	}
//
// 	#[test]
// 	fn mark_project_migration_as_finished() {
// 		let inst = MockInstantiator::new(Some(RefCell::new(new_test_ext())));
// 		let (project_id, mut inst) = create_offchain_migration_project(inst);
//
// 		let participants = inst.execute(|| UserMigrations::<TestRuntime>::iter_key_prefix((project_id,)).collect_vec());
// 		for participant in participants {
// 			inst.execute(|| {
// 				assert_ok!(crate::Pallet::<TestRuntime>::do_confirm_offchain_migration(
// 					project_id,
// 					ISSUER_1,
// 					participant
// 				));
// 			});
// 		}
//
// 		inst.execute(|| {
// 			assert_ok!(crate::Pallet::<TestRuntime>::do_mark_project_ct_migration_as_finished(project_id));
// 		});
// 	}
//
// 	// Can't start if project is not settled
// }
