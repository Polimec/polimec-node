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
use pallet_funding::{assert_close_enough, ProjectId};
use polimec_common::migration_types::{MigrationStatus, Migrations};
use politest_runtime::PolimecFunding;
use sp_runtime::Perquintill;
use std::collections::HashMap;
use tests::defaults::*;

fn mock_hrmp_establishment(project_id: u32) {
	PolitestNet::execute_with(|| {
		assert_ok!(PolimecFunding::do_set_para_id_for_project(&ISSUER.into(), project_id, ParaId::from(6969u32),));

		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));
	});

	// Required for passing migration ready check.
	PenNet::execute_with(|| {});
}

fn assert_migration_is_ready(project_id: u32) {
	PolitestNet::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolitestRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});
}

fn get_migrations_for_participants(
	project_id: ProjectId,
	participants: Vec<AccountId>,
) -> HashMap<AccountId, (MigrationStatus, Migrations)> {
	let mut user_migrations = HashMap::new();
	PolitestNet::execute_with(|| {
		for participant in participants {
			let (status, migrations) =
				pallet_funding::UserMigrations::<PolitestRuntime>::get(project_id, participant.clone()).unwrap();
			user_migrations.insert(participant, (status, Migrations::from(migrations.into())));
		}
	});
	user_migrations
}

fn send_migrations(project_id: ProjectId, accounts: Vec<AccountId>) {
	for user in accounts.into_iter() {
		PolitestNet::execute_with(|| {
			assert_ok!(PolimecFunding::migrate_one_participant(
				PolitestOrigin::signed(user.clone()),
				project_id,
				user.clone()
			));
		});
	}
}

fn migrations_are_executed(project_id: ProjectId, accounts: Vec<AccountId>) {
	let user_migrations = get_migrations_for_participants(project_id, accounts.clone());
	for account in accounts.into_iter() {
		let user_info = PenNet::account_data_of(account.clone());
		PenNet::execute_with(|| {
			let (_, migrations) = user_migrations.get(&account).unwrap();

			assert_close_enough!(user_info.free, migrations.total_ct_amount(), Perquintill::from_float(0.99));

			let vest_scheduled_cts = migrations
				.clone()
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
		});
	}
}

fn migrations_are_confirmed(project_id: u32, accounts: Vec<AccountId>) {
	let user_migrations = get_migrations_for_participants(project_id, accounts.clone());
	PolitestNet::execute_with(|| {
		for user in accounts.iter() {
			let (current_status, _) = user_migrations.get(user).unwrap();
			assert_eq!(current_status, &MigrationStatus::Confirmed);
		}
	});
}

fn vest_migrations(project_id: u32, accounts: Vec<AccountId>) {
	let user_migrations = get_migrations_for_participants(project_id, accounts.clone());
	let biggest_time =
		user_migrations.iter().map(|(_, (_, migrations))| migrations.biggest_vesting_time()).max().unwrap();

	PenNet::execute_with(|| {
		PenpalSystem::set_block_number(biggest_time as u32 + 1u32);
	});
	for account in accounts {
		let user_info = PenNet::account_data_of(account.clone());
		PenNet::execute_with(|| {
			if user_info.frozen > 0 {
				assert_ok!(pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(account)));
			}
		});
	}
}

fn migrations_are_vested(project_id: u32, accounts: Vec<AccountId>) {
	let user_migrations = get_migrations_for_participants(project_id, accounts.clone());
	user_migrations.iter().for_each(|(user, (_, migrations))| {
		let user_info = PenNet::account_data_of(user.clone());
		assert_eq!(user_info.frozen, 0);
		assert_eq!(user_info.free, migrations.clone().total_ct_amount());
	});
}

fn create_settled_project() -> (ProjectId, Vec<AccountId>) {
	let mut inst = IntegrationInstantiator::new(None);
	PolitestNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			default_project_metadata(0, ISSUER.into()),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		inst.advance_time(<PolitestRuntime as pallet_funding::Config>::SuccessToSettlementTime::get()).unwrap();
		let mut participants: Vec<AccountId> =
			pallet_funding::Evaluations::<PolitestRuntime>::iter_prefix_values((project_id,))
				.map(|eval| eval.evaluator)
				.chain(pallet_funding::Bids::<PolitestRuntime>::iter_prefix_values((project_id,)).map(|bid| bid.bidder))
				.chain(
					pallet_funding::Contributions::<PolitestRuntime>::iter_prefix_values((project_id,))
						.map(|contribution| contribution.contributor),
				)
				.collect();
		participants.sort();
		participants.dedup();

		inst.settle_project(project_id).unwrap();
		(project_id, participants)
	})
}

#[test]
fn full_migration_test() {
	let (project_id, participants) = create_settled_project();

	dbg!(get_migrations_for_participants(project_id, participants.clone()));

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	// Migrate is sent
	send_migrations(project_id, participants.clone());

	migrations_are_executed(project_id, participants.clone());

	migrations_are_confirmed(project_id, participants.clone());

	vest_migrations(project_id, participants.clone());

	migrations_are_vested(project_id, participants.clone());
}
