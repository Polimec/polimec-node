use crate::*;
use pallet_funding::{
	assert_close_enough, traits::VestingDurationCalculation, AcceptedFundingAsset, BidStatus, MigrationStatus,
	Multiplier, MultiplierOf, ProjectIdOf, RewardOrSlash,
};
use polimec_parachain_runtime::PolimecFunding;
use polimec_traits::migration_types::{Migration, MigrationInfo, MigrationOrigin, Migrations, ParticipationType};
use sp_runtime::{FixedPointNumber, Perquintill};
use std::collections::HashMap;
use tests::defaults::*;

fn execute_cleaner(inst: &mut IntegrationInstantiator) {
	Polimec::execute_with(|| {
		inst.advance_time(<PolimecRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
	});
}
fn mock_hrmp_establishment(project_id: u32) {
	Polimec::execute_with(|| {
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

	Penpal::execute_with(|| {
		println!("penpal events:");
		dbg!(Penpal::events());
	});
}

fn assert_migration_is_ready(project_id: u32) {
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});
}

fn send_migrations(
	project_id: ProjectIdOf<PolimecRuntime>,
	accounts: Vec<AccountId>,
) -> HashMap<AccountId, Migrations> {
	let mut output = HashMap::new();
	for account in accounts {
		let migrations = Polimec::execute_with(|| {
			assert_ok!(PolimecFunding::migrate_one_participant(
				PolimecOrigin::signed(account.clone()),
				project_id,
				account.clone()
			));
			let (query_id, _migrations) =
				pallet_funding::UnconfirmedMigrations::<PolimecRuntime>::iter().next().unwrap();

			let user_evaluations =
				pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()));
			let user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()))
				.filter(|bid| matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)));
			let user_contributions =
				pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()));

			let evaluation_migrations = user_evaluations.map(|evaluation| {
				assert_eq!(evaluation.ct_migration_status, MigrationStatus::Sent(query_id));
				if let Some(RewardOrSlash::Reward(amount)) = evaluation.rewarded_or_slashed {
					Migration {
						info: MigrationInfo {
							contribution_token_amount: amount,
							vesting_time: Multiplier::new(1u8)
								.unwrap()
								.calculate_vesting_duration::<PolimecRuntime>()
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
				assert_eq!(bid.ct_migration_status, MigrationStatus::Sent(query_id));
				Migration {
					info: MigrationInfo {
						contribution_token_amount: bid.final_ct_amount,
						vesting_time: bid.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
					},
					origin: MigrationOrigin {
						user: account.clone().into(),
						id: bid.id,
						participation_type: ParticipationType::Bid,
					},
				}
			});
			let contribution_ct_amount = user_contributions.map(|contribution| {
				assert_eq!(contribution.ct_migration_status, MigrationStatus::Sent(query_id));
				Migration {
					info: MigrationInfo {
						contribution_token_amount: contribution.ct_amount,
						vesting_time: contribution.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
					},
					origin: MigrationOrigin {
						user: account.clone().into(),
						id: contribution.id,
						participation_type: ParticipationType::Contribution,
					},
				}
			});

			evaluation_migrations.chain(bid_migrations).chain(contribution_ct_amount).collect::<Migrations>()
		});
		output.insert(account.clone(), migrations);
	}
	output
}

fn migrations_are_executed(grouped_migrations: Vec<Migrations>) {
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin|origin.user == user ));
		Penpal::execute_with(|| {
			assert_expected_events!(
				Penpal,
				vec![
					PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationsExecuted{migrations}) => {
						migrations: {
							migrations.clone().sort_by_ct_amount() == migration_group.clone().sort_by_ct_amount()
						},
					},
				]
			);
		});

		let user_info = Penpal::account_data_of(user.into());
		assert_close_enough!(user_info.free, migration_group.total_ct_amount(), Perquintill::from_parts(10_000_000_000u64));

		let vest_scheduled_cts = migration_group.inner().iter().filter_map(|migration| {
			if migration.info.vesting_time > 1 {
				Some(migration.info.contribution_token_amount)
			} else {
				None
			}
		}).sum::<u128>();
		assert_close_enough!(user_info.frozen, vest_scheduled_cts, Perquintill::from_parts(1_000_000_000u64));
	}
}

fn migrations_are_confirmed(project_id: u32, grouped_migrations: Vec<Migrations>) {
	Polimec::execute_with(|| {
		for migration_group in grouped_migrations {
			assert_expected_events!(
				Polimec,
				vec![
					PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed{project_id, migration_origins}) => {
						project_id: project_id == project_id,
						migration_origins: {
							let mut a = migration_group.origins().clone();
							let mut b = migration_origins.to_vec().clone();
							a.sort();
							b.sort();
							a == b
						},
					},
				]
			);
			for migration_origin in migration_group.origins() {
				match migration_origin.participation_type {
					ParticipationType::Evaluation => {
						let evaluation = pallet_funding::Evaluations::<PolimecRuntime>::get((
							project_id,
							AccountId::from(migration_origin.user),
							migration_origin.id,
						)
						)
						.unwrap();
						assert_eq!(evaluation.ct_migration_status, MigrationStatus::Confirmed);
					},
					ParticipationType::Bid => {
						let bid = pallet_funding::Bids::<PolimecRuntime>::get((
							project_id,
							AccountId::from(migration_origin.user),
							migration_origin.id,
						))
						.unwrap();
						assert_eq!(bid.ct_migration_status, MigrationStatus::Confirmed);
					},
					ParticipationType::Contribution => {
						let contribution = pallet_funding::Contributions::<PolimecRuntime>::get((
							project_id,
							AccountId::from(migration_origin.user),
							migration_origin.id,
						))
						.unwrap();
						assert_eq!(contribution.ct_migration_status, MigrationStatus::Confirmed);
					},
				}
			}
		}
	});
}

fn vest_migrations(grouped_migrations: Vec<Migrations>) {
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin|origin.user == user ));

		Penpal::execute_with(|| {
			assert_ok!(pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(user.into())));
		});
	}
}
fn migrations_are_vested(grouped_migrations: Vec<Migrations>) {
	for migration_group in grouped_migrations {
		let user = migration_group.clone().inner()[0].origin.user;
		assert!(migration_group.origins().iter().all(|origin|origin.user == user ));
		let total_ct = migration_group.total_ct_amount();
		let user_info = Penpal::account_data_of(user.into());
	}
}

#[test]
fn migration_check() {
	let mut inst = IntegrationInstantiator::new(None);
	set_oracle_prices();
	let project_id = Polimec::execute_with(|| {
		let project_id = inst.create_finished_project(
			default_project(ISSUER.into(), 0),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			vec![],
		);

		inst.advance_time(<PolimecRuntime as pallet_funding::Config>::SuccessToSettlementTime::get() + 1u32).unwrap();
		project_id
	});

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);
}

#[test]
fn migration_is_sent() {
	let mut inst = IntegrationInstantiator::new(None);
	set_oracle_prices();

	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(ISSUER.into(), 0),
			ISSUER.into(),
			vec![
				UserToUSDBalance::new(EVAL_1.into(), 50_000 * PLMC),
				UserToUSDBalance::new(EVAL_2.into(), 25_000 * PLMC),
				UserToUSDBalance::new(EVAL_3.into(), 32_000 * PLMC),
			],
			IntegrationInstantiator::generate_bids_from_total_usd(
				Perquintill::from_percent(40) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![EVAL_1.into(), BIDDER_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into()],
			),
			IntegrationInstantiator::generate_contributions_from_total_usd(
				Perquintill::from_percent(50) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![EVAL_1.into(), BUYER_1.into(), BUYER_2.into(), BUYER_3.into(), BUYER_4.into()],
			),
			vec![],
		)
	});
	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	send_migrations(project_id, vec![EVAL_1.into()]);
}

#[test]
fn migration_is_executed_on_project_and_confirmed_on_polimec() {
	let mut inst = IntegrationInstantiator::new(None);
	set_oracle_prices();

	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(ISSUER.into(), 0),
			ISSUER.into(),
			vec![
				UserToUSDBalance::new(EVAL_1.into(), 50_000 * PLMC),
				UserToUSDBalance::new(EVAL_2.into(), 25_000 * PLMC),
				UserToUSDBalance::new(EVAL_3.into(), 32_000 * PLMC),
			],
			IntegrationInstantiator::generate_bids_from_total_usd(
				Perquintill::from_percent(40) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![EVAL_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(), BIDDER_5.into()],
			),
			IntegrationInstantiator::generate_contributions_from_total_usd(
				Perquintill::from_percent(50) *
					(sp_runtime::FixedU128::from_float(1.0).checked_mul_int(100_000 * ASSET_UNIT).unwrap()),
				sp_runtime::FixedU128::from_float(1.0),
				default_weights(),
				vec![EVAL_1.into(), BUYER_2.into(), BUYER_3.into(), BUYER_4.into(), BUYER_5.into()],
			),
			vec![],
		)
	});
	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let migrations_map = send_migrations(project_id, vec![EVAL_1.into()]);
	let grouped_migrations = migrations_map.values().cloned().collect::<Vec<_>>();

	migrations_are_executed(grouped_migrations.clone());
	migrations_are_confirmed(project_id, grouped_migrations.clone());
}

#[test]
fn vesting_over_several_blocks_on_project() {
	let mut inst = IntegrationInstantiator::new(None);
	set_oracle_prices();

	let mut bids = Vec::new();
	let mut contributions = Vec::new();
	let multiplier_for_vesting = MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap();

	bids.push(BidParams {
		bidder: BUYER_1.into(),
		amount: 2_000 * ASSET_UNIT,
		price: 12u128.into(),
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	bids.push(BidParams {
		bidder: BIDDER_1.into(),
		amount: 20_000 * ASSET_UNIT,
		price: 10u128.into(),
		multiplier: multiplier_for_vesting,
		asset: AcceptedFundingAsset::USDT,
	});
	bids.push(BidParams {
		bidder: BIDDER_2.into(),
		amount: 12_000 * ASSET_UNIT,
		price: 11u128.into(),
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});

	contributions.push(ContributionParams {
		contributor: BUYER_1.into(),
		amount: 10_250 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	contributions.push(ContributionParams {
		contributor: BUYER_2.into(),
		amount: 5000 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	contributions.push(ContributionParams {
		contributor: BUYER_3.into(),
		amount: 30000 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});

	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(ISSUER.into(), 0),
			ISSUER.into(),
			default_evaluations(),
			bids,
			contributions,
			vec![],
		)
	});
	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let pre_migration_balance = Penpal::account_data_of(BUYER_1.into());

	// Migrate is sent
	let user_migrations = send_migrations(project_id, vec![BUYER_1.into()]);
	let grouped_migrations = user_migrations.values().cloned().collect::<Vec<_>>();

	migrations_are_executed(grouped_migrations.clone());

	let post_migration_balance = Penpal::account_data_of(BUYER_1.into());

	assert_close_enough!(
		post_migration_balance.free -
			pre_migration_balance.free -
			post_migration_balance.frozen -
			pre_migration_balance.frozen,
		10_250 * ASSET_UNIT,
		Perquintill::from_parts(10_000_000_000u64)
	);

	migrations_are_confirmed(project_id, grouped_migrations.clone());

	Penpal::execute_with(|| {
		let unblock_time: u32 = multiplier_for_vesting.calculate_vesting_duration::<PolimecRuntime>();
		PenpalSystem::set_block_number(unblock_time + 1u32);
		assert_ok!(pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(BUYER_1.into())));
	});

	let post_vest_balance = Penpal::account_data_of(BUYER_1.into());
	assert_close_enough!(
		post_vest_balance.free - post_vest_balance.frozen,
		10_250 * ASSET_UNIT + 2000 * ASSET_UNIT,
		Perquintill::from_parts(10_000_000_000u64)
	);
}

#[test]
fn disallow_duplicated_migrations_on_receiver_pallet() {
	let mut inst = IntegrationInstantiator::new(None);
	set_oracle_prices();

	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(ISSUER.into(), 0),
			ISSUER.into(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			default_remainder_contributions(),
		)
	});

	let mut participants = vec![
		EVAL_1, EVAL_2, EVAL_3, EVAL_4, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5, BIDDER_6, BUYER_1, BUYER_2,
		BUYER_3, BUYER_4, BUYER_5, BUYER_6,
	]
	.into_iter()
	.map(|x| AccountId::from(x))
	.collect::<Vec<_>>();

	execute_cleaner(&mut inst);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let migrations_sent = send_migrations(project_id, participants);
}
