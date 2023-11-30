use crate::*;
use pallet_funding::{
	assert_close_enough, traits::VestingDurationCalculation, AcceptedFundingAsset, BidStatus, MigrationStatus,
	Multiplier, MultiplierOf, ProjectIdOf, RewardOrSlash,
};
use polimec_parachain_runtime::PolimecFunding;
use polimec_traits::migration_types::MigrationInfo;
use sp_runtime::{FixedPointNumber, Perquintill};
use std::collections::HashMap;
use tests::defaults::*;

fn calculate_total_ct_amount<'a>(migrations: impl Iterator<Item = &'a MigrationInfo>) -> u128 {
	migrations.into_iter().map(|m| m.contribution_token_amount).sum()
}

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
) -> HashMap<AccountId, Vec<MigrationInfo>> {
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
					MigrationInfo {
						contribution_token_amount: amount,
						vesting_time: Multiplier::new(1u8)
							.unwrap()
							.calculate_vesting_duration::<PolimecRuntime>()
							.into(),
					}
				} else {
					panic!("should be rewarded")
				}
			});
			let bid_migrations = user_bids.map(|bid| {
				assert_eq!(bid.ct_migration_status, MigrationStatus::Sent(query_id));
				MigrationInfo {
					contribution_token_amount: bid.final_ct_amount,
					vesting_time: bid.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
				}
			});
			let contribution_ct_amount = user_contributions.map(|contribution| {
				assert_eq!(contribution.ct_migration_status, MigrationStatus::Sent(query_id));
				MigrationInfo {
					contribution_token_amount: contribution.ct_amount,
					vesting_time: contribution.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
				}
			});

			evaluation_migrations.chain(bid_migrations).chain(contribution_ct_amount).collect::<Vec<MigrationInfo>>()
		});
		output.insert(account.clone(), migrations);
	}
	output
}

fn migrations_are_executed(migrations: impl Iterator<Item = (AccountId, Vec<MigrationInfo>)>) {
	Penpal::execute_with(|| {
		for (account, migrations) in migrations {
			let amount = calculate_total_ct_amount(migrations.iter());

			assert_expected_events!(
				Penpal,
				vec![
					PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationsExecuted{migrations}) => {
						migrations: {
							let origins = migrations.origins();
							let infos = migrations.infos();
							let mut migrations_come_from_same_user = origins.into_iter().map(|origin| {
								let account: [u8; 32] = account.clone().into();
								origin.user == account
							});
							migrations_come_from_same_user.all(|x|x) &&
								infos.into_iter().map(|info| info.contribution_token_amount).sum::<u128>() == amount
						},
					},
				]
			);
		}
	});
}

fn migrations_are_confirmed_for(project_id: u32, accounts: Vec<AccountId>) {
	Polimec::execute_with(|| {
		for account in accounts {
			let mut user_evaluations =
				pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()));
			let mut user_bids =
				pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()));
			let mut user_contributions =
				pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, account.clone()));

			assert!(user_evaluations.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
			assert!(user_bids.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
			assert!(
				user_contributions.all(|contribution| contribution.ct_migration_status == MigrationStatus::Confirmed)
			);

			assert_expected_events!(
				Polimec,
				vec![
					PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed{project_id, ..}) => {
						project_id: project_id == project_id,
					},
				]
			);
		}
	});
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

	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

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
	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let pre_migration_balance = Penpal::account_data_of(EVAL_1.into());

	let migrations = send_migrations(project_id, vec![EVAL_1.into()]);
	let eval_1_migrations = migrations[&EVAL_1.into()].clone();

	migrations_are_confirmed_for(project_id, vec![EVAL_1.into()]);
	let eval_1_total_migrated_amount = calculate_total_ct_amount(eval_1_migrations.iter());

	// Balance is there for the user after vesting (Multiplier 1, so no vesting)
	let post_migration_balance = Penpal::account_data_of(EVAL_1.into());
	assert_eq!(post_migration_balance.free - pre_migration_balance.free, eval_1_total_migrated_amount);
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
	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

	mock_hrmp_establishment(project_id);

	assert_migration_is_ready(project_id);

	let pre_migration_balance = Penpal::account_data_of(BUYER_1.into());

	// Migrate is sent
	let migrations_sent = send_migrations(project_id, vec![BUYER_1.into()]);

	migrations_are_executed(migrations_sent.into_iter());

	let post_migration_balance = Penpal::account_data_of(BUYER_1.into());

	assert_close_enough!(
		post_migration_balance.free -
			pre_migration_balance.free -
			post_migration_balance.frozen -
			pre_migration_balance.frozen,
		10_250 * ASSET_UNIT,
		Perquintill::from_parts(10_000_000_000u64)
	);

	migrations_are_confirmed_for(project_id, vec![BUYER_1.into()]);

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
}
