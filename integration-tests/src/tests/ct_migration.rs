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
use frame_support::traits::{fungible::Mutate, fungibles::Inspect};
use itertools::Itertools;
use pallet_funding::{assert_close_enough, types::*, ProjectId, WeightInfo};
use polimec_common::migration_types::{MigrationStatus, Migrations, ParticipationType};
use polimec_runtime::{Funding, RuntimeOrigin};
use polkadot_service::chain_spec::get_account_id_from_seed;
use sp_runtime::Perquintill;
use std::collections::HashMap;
use tests::defaults::*;
use xcm_executor::traits::WeightBounds;

fn alice() -> AccountId {
	get_account_id_from_seed::<sr25519::Public>(ALICE)
}

fn mock_hrmp_establishment(project_id: u32) {
	let ct_issued = PolimecNet::execute_with(|| {
		<PolimecRuntime as pallet_funding::Config>::ContributionTokenCurrency::total_issuance(project_id)
	});
	PenNet::execute_with(|| {
		let polimec_sovereign_account =
			<Penpal<PolkadotNet>>::sovereign_account_id_of((Parent, xcm::prelude::Parachain(polimec::PARA_ID)).into());
		PenpalBalances::set_balance(&polimec_sovereign_account, ct_issued + polimec_runtime::EXISTENTIAL_DEPOSIT);
	});

	PolimecNet::execute_with(|| {
		const SENDER: u32 = 6969;
		assert_ok!(Funding::do_start_pallet_migration(&ISSUER.into(), project_id, ParaId::from(SENDER)));
		assert_ok!(Funding::do_handle_channel_open_request(SENDER, 50_000, 8));
		assert_ok!(Funding::do_handle_channel_accepted(SENDER));
	});

	// Required for passing migration ready check.
	PenNet::execute_with(|| {});
}

fn assert_migration_is_ready(project_id: u32) {
	PolimecNet::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		let Some(MigrationType::Pallet(receiver_pallet_info)) = project_details.migration_type else {
			panic!("Migration type is not ParachainReceiverPallet");
		};
		assert!(receiver_pallet_info.migration_readiness_check.unwrap().is_ready())
	});
}

fn get_migrations_for_participants(
	project_id: ProjectId,
	participants: Vec<AccountId>,
) -> HashMap<AccountId, (MigrationStatus, Migrations)> {
	let mut user_migrations = HashMap::new();
	PolimecNet::execute_with(|| {
		for participant in participants {
			let (status, migrations) =
				pallet_funding::UserMigrations::<PolimecRuntime>::get((project_id, participant.clone())).unwrap();
			user_migrations.insert(participant, (status, Migrations::from(migrations.into())));
		}
	});
	user_migrations
}

fn send_migrations(project_id: ProjectId, accounts: Vec<AccountId>) {
	for user in accounts.into_iter() {
		PolimecNet::execute_with(|| {
			assert_ok!(Funding::send_pallet_migration_for(
				PolimecOrigin::signed(user.clone()),
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
	PolimecNet::execute_with(|| {
		for user in accounts.iter() {
			let (current_status, _) = user_migrations.get(user).unwrap();
			assert_eq!(current_status, &MigrationStatus::Confirmed);
		}

		PolimecFunding::do_mark_project_ct_migration_as_finished(project_id).unwrap();
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert_eq!(project_details.status, pallet_funding::ProjectStatus::CTMigrationFinished)
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
	PolimecNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER.into()),
			ISSUER.into(),
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);
		assert_eq!(
			inst.go_to_next_state(project_id),
			pallet_funding::ProjectStatus::SettlementStarted(FundingOutcome::Success)
		);
		let mut participants: Vec<AccountId> =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,))
				.map(|eval| eval.evaluator)
				.chain(pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id,)).map(|bid| bid.bidder))
				.chain(
					pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id,))
						.map(|contribution| contribution.contributor),
				)
				.collect();
		participants.sort();
		participants.dedup();

		inst.settle_project(project_id, true);
		(project_id, participants)
	})
}

#[test]
fn full_pallet_migration_test() {
	polimec::set_prices();
	let (project_id, participants) = create_settled_project();
	let project_status =
		PolimecNet::execute_with(|| pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap().status);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	send_migrations(project_id, participants.clone());

	migrations_are_executed(project_id, participants.clone());

	migrations_are_confirmed(project_id, participants.clone());

	vest_migrations(project_id, participants.clone());

	migrations_are_vested(project_id, participants.clone());
}

/// Creates a project with all participations settled except for one.
fn create_project_with_unsettled_participation(participation_type: ParticipationType) -> (ProjectId, Vec<AccountId>) {
	let mut inst = IntegrationInstantiator::new(None);
	PolimecNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			default_project_metadata(ISSUER.into()),
			ISSUER.into(),
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		);

		assert_eq!(
			inst.go_to_next_state(project_id),
			pallet_funding::ProjectStatus::SettlementStarted(FundingOutcome::Success)
		);
		let evaluations_to_settle =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();
		let bids_to_settle = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();
		let contributions_to_settle =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();

		let mut participants: Vec<AccountId> = evaluations_to_settle
			.iter()
			.map(|eval| eval.evaluator.clone())
			.chain(bids_to_settle.iter().map(|bid| bid.bidder.clone()))
			.chain(contributions_to_settle.iter().map(|contribution| contribution.contributor.clone()))
			.collect();
		participants.sort();
		participants.dedup();

		let start = if participation_type == ParticipationType::Evaluation { 1 } else { 0 };
		for evaluation in evaluations_to_settle[start..].iter() {
			PolimecFunding::settle_evaluation(
				RuntimeOrigin::signed(alice()),
				project_id,
				evaluation.evaluator.clone(),
				evaluation.id,
			)
			.unwrap()
		}

		let start = if participation_type == ParticipationType::Bid { 1 } else { 0 };
		for bid in bids_to_settle[start..].iter() {
			PolimecFunding::settle_bid(RuntimeOrigin::signed(alice()), project_id, bid.bidder.clone(), bid.id).unwrap()
		}

		let start = if participation_type == ParticipationType::Contribution { 1 } else { 0 };
		for contribution in contributions_to_settle[start..].iter() {
			PolimecFunding::settle_contribution(
				RuntimeOrigin::signed(alice()),
				project_id,
				contribution.contributor.clone(),
				contribution.id,
			)
			.unwrap()
		}

		let evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();
		let bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();
		let contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id,)).collect_vec();

		if participation_type == ParticipationType::Evaluation {
			assert_eq!(evaluations.len(), 1);
			assert_eq!(bids.len(), 0);
			assert_eq!(contributions.len(), 0);
		} else if participation_type == ParticipationType::Bid {
			assert_eq!(evaluations.len(), 0);
			assert_eq!(bids.len(), 1);
			assert_eq!(contributions.len(), 0);
		} else {
			assert_eq!(evaluations.len(), 0);
			assert_eq!(bids.len(), 0);
			assert_eq!(contributions.len(), 1);
		}

		(project_id, participants)
	})
}

#[test]
fn cannot_start_pallet_migration_with_unsettled_participations() {
	polimec::set_prices();

	let tup_1 = create_project_with_unsettled_participation(ParticipationType::Evaluation);
	let tup_2 = create_project_with_unsettled_participation(ParticipationType::Bid);
	let tup_3 = create_project_with_unsettled_participation(ParticipationType::Contribution);

	let tups = vec![tup_1, tup_2, tup_3];

	for (project_id, participants) in tups.into_iter() {
		PolimecNet::execute_with(|| {
			assert_noop!(
				PolimecFunding::do_start_pallet_migration(&ISSUER.into(), project_id, ParaId::from(6969u32)),
				pallet_funding::Error::<PolimecRuntime>::SettlementNotComplete
			);
		});
	}
}

#[test]
fn hrmp_functions_weight_is_under_assumed_maximum() {
	type WeightInfo = <PolimecRuntime as pallet_funding::Config>::WeightInfo;
	type XcmWeigher = <polimec_runtime::xcm_config::XcmConfig as xcm_executor::Config>::Weigher;

	let open_channel_message = xcm::v4::Instruction::<PolimecCall>::HrmpNewChannelOpenRequest {
		sender: 6969,
		max_message_size: 102_300,
		max_capacity: 1000,
	};
	let channel_accepted_message = xcm::v4::Instruction::<PolimecCall>::HrmpChannelAccepted { recipient: 6969u32 };

	let open_channel_message_real_weight = WeightInfo::do_handle_channel_open_request();
	let open_channel_message_deducted_weight = XcmWeigher::instr_weight(&open_channel_message).unwrap();

	let channel_accepted_message_real_weight = WeightInfo::do_handle_channel_accepted();
	let channel_accepted_message_deducted_weight = XcmWeigher::instr_weight(&channel_accepted_message).unwrap();

	assert!(open_channel_message_deducted_weight.all_gte(open_channel_message_real_weight));
	assert!(channel_accepted_message_deducted_weight.all_gte(channel_accepted_message_real_weight));
}
