// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::*;
use pallet_funding::{
	assert_close_enough, traits::VestingDurationCalculation, BidStatus, EvaluatorsOutcome, MigrationStatus, Multiplier,
	ProjectId, ProjectsToUpdate, RewardOrSlash,
};
use polimec_common::migration_types::{Migration, MigrationInfo, MigrationOrigin, Migrations, ParticipationType};
use politest_runtime::PolimecFunding;
use sp_runtime::{traits::Convert, FixedPointNumber, Perquintill};
use std::collections::HashMap;
use tests::defaults::*;

fn execute_cleaner(inst: &mut IntegrationInstantiator) {
	PolitestNet::execute_with(|| {
		dbg!(<PolitestRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 25u32);
		inst.advance_time(<PolitestRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 25u32).unwrap();
	});
}
fn mock_hrmp_establishment(project_id: u32) {
	PolitestNet::execute_with(|| {
		assert_ok!(PolimecFunding::do_set_para_id_for_project(&ISSUER.into(), project_id, ParaId::from(6969u32)));

		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));
	});

	PenNet::execute_with(|| {
		println!("penpal events:");
		dbg!(PenNet::events());
	});
}

fn assert_migration_is_ready(project_id: u32) {
	PolitestNet::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolitestRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});
}

fn send_migrations(project_id: ProjectId, accounts: Vec<AccountId>) -> HashMap<AccountId, Migrations> {
	let mut output = HashMap::new();
	PolitestNet::execute_with(|| {
		for account in accounts {
			assert_ok!(PolimecFunding::migrate_one_participant(
				PolitestOrigin::signed(account.clone()),
				project_id,
				account.clone()
			));

			let user_evaluations =
				pallet_funding::Evaluations::<PolitestRuntime>::iter_prefix_values((project_id, account.clone()));
			let user_bids = pallet_funding::Bids::<PolitestRuntime>::iter_prefix_values((project_id, account.clone()))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));
			let user_contributions =
				pallet_funding::Contributions::<PolitestRuntime>::iter_prefix_values((project_id, account.clone()));

			let evaluation_migrations = user_evaluations.map(|evaluation| {
				let evaluator_bytes = <PolitestRuntime as pallet_funding::Config>::AccountId32Conversion::convert(
					evaluation.evaluator.clone(),
				);
				assert!(
					matches!(evaluation.ct_migration_status, MigrationStatus::Sent(_)),
					"{:?}'s evaluation was not sent {:?}",
					names()[&evaluator_bytes],
					evaluation
				);
				if let Some(RewardOrSlash::Reward(amount)) = evaluation.rewarded_or_slashed {
					Migration {
						info: MigrationInfo {
							contribution_token_amount: amount,
							vesting_time: Multiplier::new(1u8)
								.unwrap()
								.calculate_vesting_duration::<PolitestRuntime>()
								.into(),
						},
						origin: MigrationOrigin {
							user: account.clone().into(),
							id: evaluation.id,
							participation_type: ParticipationType::Evaluation,
						},
					}
				} else {
					panic!("should be rewarded")
				}
			});
			let bid_migrations = user_bids.map(|bid| {
				assert!(matches!(bid.ct_migration_status, MigrationStatus::Sent(_)));
				Migration {
					info: MigrationInfo {
						contribution_token_amount: bid.final_ct_amount,
						vesting_time: bid.multiplier.calculate_vesting_duration::<PolitestRuntime>().into(),
					},
					origin: MigrationOrigin {
						user: account.clone().into(),
						id: bid.id,
						participation_type: ParticipationType::Bid,
					},
				}
			});
			let contribution_migrations = user_contributions.map(|contribution| {
				assert!(matches!(contribution.ct_migration_status, MigrationStatus::Sent(_)));
				Migration {
					info: MigrationInfo {
						contribution_token_amount: contribution.ct_amount,
						vesting_time: contribution.multiplier.calculate_vesting_duration::<PolitestRuntime>().into(),
					},
					origin: MigrationOrigin {
						user: account.clone().into(),
						id: contribution.id,
						participation_type: ParticipationType::Contribution,
					},
				}
			});

			let migrations =
				evaluation_migrations.chain(bid_migrations).chain(contribution_migrations).collect::<Migrations>();

			if migrations.clone().inner().is_empty() {
				panic!("no migrations for account: {:?}", account)
			}
			output.insert(account.clone(), migrations);
		}
	});
	output
}

fn migrations_are_executed(grouped_migrations: Vec<Migrations>) {
	let all_migrations =
		grouped_migrations.iter().flat_map(|migrations| migrations.clone().inner()).collect::<Vec<_>>();
	PenNet::execute_with(|| {
		assert_expected_events!(
			PenNet,
			vec![
				PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationExecuted{migration}) => {
					migration: all_migrations.contains(&migration),
				},
			]
		);
	});

	// since current way to migrate is by bundling each user's migrations into one, we can assume that all migrations in a group are from the same user
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin| origin.user == user));

		let user_info = PenNet::account_data_of(user.into());
		assert_close_enough!(user_info.free, migration_group.total_ct_amount(), Perquintill::from_float(0.99));

		let vest_scheduled_cts = migration_group
			.inner()
			.iter()
			.filter_map(|migration| {
				if migration.info.vesting_time > 1 {
					Some(migration.info.contribution_token_amount)
				} else {
					None
				}
			})
			.sum::<u128>();

		assert_close_enough!(user_info.frozen, vest_scheduled_cts, Perquintill::from_float(0.99));
	}
}

fn migrations_are_confirmed(project_id: u32, grouped_migrations: Vec<Migrations>) {
	let ordered_grouped_origins = grouped_migrations
		.clone()
		.into_iter()
		.map(|group| {
			let mut origins = group.origins();
			origins.sort();
			origins
		})
		.collect::<Vec<_>>();
	PolitestNet::execute_with(|| {
		assert_expected_events!(
			PolitestNet,
			vec![
				PolitestEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed{project_id, migration_origins}) => {
					project_id: project_id == project_id,
					migration_origins: {
						let mut migration_origins = migration_origins.to_vec();
						migration_origins.sort();
						ordered_grouped_origins.contains(&migration_origins)
					},
				},
			]
		);
		let all_migration_origins =
			grouped_migrations.iter().flat_map(|migrations| migrations.clone().origins()).collect::<Vec<_>>();
		for migration_origin in all_migration_origins {
			match migration_origin.participation_type {
				ParticipationType::Evaluation => {
					let evaluation = pallet_funding::Evaluations::<PolitestRuntime>::get((
						project_id,
						AccountId::from(migration_origin.user),
						migration_origin.id,
					))
					.unwrap();
					assert_eq!(evaluation.ct_migration_status, MigrationStatus::Confirmed);
				},
				ParticipationType::Bid => {
					let bid = pallet_funding::Bids::<PolitestRuntime>::get((
						project_id,
						AccountId::from(migration_origin.user),
						migration_origin.id,
					))
					.unwrap();
					assert_eq!(bid.ct_migration_status, MigrationStatus::Confirmed);
				},
				ParticipationType::Contribution => {
					let contribution = pallet_funding::Contributions::<PolitestRuntime>::get((
						project_id,
						AccountId::from(migration_origin.user),
						migration_origin.id,
					))
					.unwrap();
					assert_eq!(contribution.ct_migration_status, MigrationStatus::Confirmed);
				},
			}
		}
	});
}

fn vest_migrations(grouped_migrations: Vec<Migrations>) {
	let biggest_time = grouped_migrations.iter().map(|migrations| migrations.biggest_vesting_time()).max().unwrap();
	PenNet::execute_with(|| {
		PenpalSystem::set_block_number(biggest_time as u32 + 1u32);
	});
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin| origin.user == user));
		// check if any vesting_time is bigger than 1, which means the balance was actually frozen
		let has_frozen_balance = migration_group.inner().iter().any(|migration| migration.info.vesting_time > 1);
		if has_frozen_balance {
			PenNet::execute_with(|| {
				assert_ok!(pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(user.into())));
			});
		}
	}
}

fn migrations_are_vested(grouped_migrations: Vec<Migrations>) {
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin| origin.user == user));
		let user_info = PenNet::account_data_of(user.into());
		assert_eq!(user_info.frozen, 0);
		assert_eq!(user_info.free, migration_group.total_ct_amount());
	}
}

#[test]
fn migration_check() {
	let mut inst = IntegrationInstantiator::new(None);
	let project_id = PolitestNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			vec![],
		);

		inst.advance_time(<PolitestRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
		project_id
	});

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);
}

#[test]
fn migration_is_sent() {
	let mut inst = IntegrationInstantiator::new(None);
	let participants =
		vec![EVAL_1, EVAL_2, EVAL_3, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BUYER_1, BUYER_2, BUYER_3, BUYER_4]
			.into_iter()
			.map(|x| AccountId::from(x))
			.collect::<Vec<_>>();
	let project_id = PolitestNet::execute_with(|| {
		inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		)
	});

	PolitestNet::execute_with(|| {
		dbg!(PolimecSystem::block_number());
		dbg!(ProjectsToUpdate::<PolitestRuntime>::iter().collect::<Vec<_>>());
	});

	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	send_migrations(project_id, participants);
}

#[test]
fn migration_is_executed_on_project_and_confirmed_on_polimec() {
	let mut inst = IntegrationInstantiator::new(None);
	let participants =
		vec![EVAL_1, EVAL_2, EVAL_3, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5, BUYER_2, BUYER_3, BUYER_4, BUYER_5]
			.into_iter()
			.map(|x| AccountId::from(x))
			.collect::<Vec<_>>();
	let project_id = PolitestNet::execute_with(|| {
		inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		)
	});
	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let migrations_map = send_migrations(project_id, participants);
	let grouped_migrations = migrations_map.values().cloned().collect::<Vec<_>>();

	migrations_are_executed(grouped_migrations.clone());

	migrations_are_confirmed(project_id, grouped_migrations.clone());
}

#[test]
fn vesting_over_several_blocks_on_project() {
	let mut inst = IntegrationInstantiator::new(None);
	let participants = vec![EVAL_1, EVAL_2, EVAL_3, BIDDER_1, BIDDER_2, BUYER_1, BUYER_2]
		.into_iter()
		.map(|x| AccountId::from(x))
		.collect::<Vec<_>>();

	let bids = default_bids();
	let community_contributions = default_community_contributions();
	let remainder_contributions = default_remainder_contributions();

	let project_id = PolitestNet::execute_with(|| {
		inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			bids,
			community_contributions,
			remainder_contributions,
		)
	});
	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	// Migrate is sent
	let user_migrations = send_migrations(project_id, participants);
	let grouped_migrations = user_migrations.values().cloned().collect::<Vec<_>>();

	migrations_are_executed(grouped_migrations.clone());

	migrations_are_confirmed(project_id, grouped_migrations.clone());

	vest_migrations(grouped_migrations.clone());

	migrations_are_vested(grouped_migrations.clone());
}

#[test]
fn disallow_duplicated_migrations_on_receiver_pallet() {
	let mut inst = IntegrationInstantiator::new(None);

	let project_id = PolitestNet::execute_with(|| {
		inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		)
	});

	let project_details = PolitestNet::execute_with(|| inst.get_project_details(project_id));
	if let EvaluatorsOutcome::Rewarded(info) = project_details.evaluation_round_info.evaluators_outcome {
		println!("rewarded: {:?}", info);
	} else {
		panic!("should be rewarded")
	}

	let participants = vec![
		EVAL_1, EVAL_2, EVAL_3, EVAL_4, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5, BIDDER_6, BUYER_1, BUYER_2,
		BUYER_3, BUYER_4, BUYER_5, BUYER_6,
	]
	.into_iter()
	.map(|x| AccountId::from(x))
	.collect::<Vec<_>>();

	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	// Migrate is sent
	let user_migrations = send_migrations(project_id, participants);
	let grouped_migrations = user_migrations.values().cloned().collect::<Vec<_>>();

	migrations_are_executed(grouped_migrations.clone());

	migrations_are_confirmed(project_id, grouped_migrations.clone());

	vest_migrations(grouped_migrations.clone());

	migrations_are_vested(grouped_migrations.clone());

	// just any number that lets us execute our xcm's
	for migrations in grouped_migrations.clone() {
		for (_, xcm) in PolitestFundingPallet::construct_migration_xcm_messages(migrations) {
			let _call: <PolitestRuntime as pallet_funding::Config>::RuntimeCall =
				pallet_funding::Call::confirm_migrations { query_id: Default::default(), response: Default::default() }
					.into();

			let max_weight = Weight::from_parts(700_000_000, 10_000);
			let mut instructions = xcm.into_inner();
			instructions.push(ReportTransactStatus(QueryResponseInfo {
				destination: ParentThen(X1(Parachain(PolitestNet::para_id().into()))).into(),
				query_id: 69,
				max_weight,
			}));
			let xcm = Xcm(instructions);
			let project_multilocation = MultiLocation { parents: 1, interior: X1(Parachain(PenNet::para_id().into())) };

			PolitestNet::execute_with(|| {
				PolitestXcmPallet::send_xcm(Here, project_multilocation, xcm).unwrap();
			});
		}
	}

	// each duplicated migration was skipped (in this case we duplicated all of them)
	let all_migrations =
		grouped_migrations.iter().flat_map(|migrations| migrations.clone().inner()).collect::<Vec<_>>();
	PenNet::execute_with(|| {
		assert_expected_events!(
			PenNet,
			vec![
				PenpalEvent::PolimecReceiver(polimec_receiver::Event::DuplicatedMigrationSkipped{migration}) => {
					migration: all_migrations.contains(&migration),
				},
			]
		);
	});

	migrations_are_vested(grouped_migrations.clone());
}

#[ignore]
#[test]
fn failing_bid_doesnt_get_migrated() {
	todo!();
}
