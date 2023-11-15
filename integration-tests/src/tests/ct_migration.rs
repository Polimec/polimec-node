use crate::*;
use pallet_funding::{AcceptedFundingAsset, MigrationStatus, MultiplierOf, RewardOrSlash};
use polimec_parachain_runtime::PolimecFunding;
use sp_runtime::{FixedPointNumber, Perquintill};
use pallet_funding::traits::VestingDurationCalculation;
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
		assert_ok!(PolimecFunding::migrate_one_participant(PolimecOrigin::signed(eval_1()), project_id, eval_1()));
		let query_id = pallet_funding::UnconfirmedMigrations::<PolimecRuntime>::iter_keys().next().unwrap();
		let mut user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));

		assert!(user_evaluations.all(|evaluation| evaluation.ct_migration_status == MigrationStatus::Sent(query_id)));
		assert!(user_bids.all(|bid| bid.ct_migration_status == MigrationStatus::Sent(query_id)));
		assert!(user_contributions.all(|contribution| contribution.ct_migration_status == MigrationStatus::Sent(query_id)));
	});
}

#[test]
fn migration_is_executed_on_project_and_confirmed_on_polimec() {
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
	});

	// Migration is ready
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	let pre_migration_balance = Penpal::account_data_of(eval_1());

	// Migrate is sent
	let migrated_ct_amount = Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::migrate_one_participant(PolimecOrigin::signed(eval_1()), project_id, eval_1()));
		let (query_id, _migrations) = pallet_funding::UnconfirmedMigrations::<PolimecRuntime>::iter().next().unwrap();

		let user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));

		let evaluation_ct_amount = user_evaluations.map(|evaluation| {
			assert_eq!(evaluation.ct_migration_status, MigrationStatus::Sent(query_id));
			if let Some(RewardOrSlash::Reward(amount)) = evaluation.rewarded_or_slashed {
				amount
			} else {
				panic!("should be rewarded")
			}
		}).sum::<u128>();
		let bid_ct_amount = user_bids.map(|bid| {
			assert_eq!(bid.ct_migration_status, MigrationStatus::Sent(query_id));
			bid.final_ct_amount
		}).sum::<u128>();
		let contribution_ct_amount = user_contributions.map(|contribution| {
			assert_eq!(contribution.ct_migration_status, MigrationStatus::Sent(query_id));
			contribution.ct_amount
		}).sum::<u128>();

		evaluation_ct_amount + bid_ct_amount + contribution_ct_amount

	});


	Penpal::execute_with(|| {
		assert_expected_events!(
			Penpal,
			vec![
				PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationsExecutedForUser{user, ..}) => {
					user: *user == eval_1(),
				},
			]
		);

		dbg!(Penpal::events());
	});

	// Balance is there for the user after vesting (Multiplier 1, so no vesting)
	let post_migration_balance = Penpal::account_data_of(eval_1());
	dbg!(migrated_ct_amount);
	dbg!(pre_migration_balance.clone());
	dbg!(post_migration_balance.clone());
	assert_eq!(post_migration_balance.free - pre_migration_balance.free, migrated_ct_amount);

	Polimec::execute_with(|| {
		let mut user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));
		let mut user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, eval_1()));

		assert!(user_evaluations.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
		assert!(user_bids.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
		assert!(user_contributions.all(|contribution| contribution.ct_migration_status == MigrationStatus::Confirmed));

		assert_expected_events!(
			Polimec,
			vec![
				PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed{project_id, ..}) => {
					project_id: project_id == project_id,
				},
			]
		);
	});
}

#[test]
fn vesting_over_several_blocks_on_project() {
	let mut inst = IntegrationInstantiator::new(None);
	let mut bids = Vec::new();
	let mut contributions = Vec::new();
	let multiplier_for_vesting = MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap();

	bids.push(BidParams {
		bidder: buyer_1(),
		amount: 2_000 * ASSET_UNIT,
		price: 12u128.into(),
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	bids.push(BidParams {
		bidder: bidder_1(),
		amount: 20_000 * ASSET_UNIT,
		price: 10u128.into(),
		multiplier: multiplier_for_vesting,
		asset: AcceptedFundingAsset::USDT,
	});
	bids.push(BidParams {
		bidder: bidder_2(),
		amount: 12_000 * ASSET_UNIT,
		price: 11u128.into(),
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(10u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});

	contributions.push(ContributionParams {
		contributor: buyer_1(),
		amount: 10_250 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	contributions.push(ContributionParams {
		contributor: buyer_2(),
		amount: 5000 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});
	contributions.push(ContributionParams {
		contributor: buyer_3(),
		amount: 30000 * ASSET_UNIT,
		multiplier: MultiplierOf::<PolimecRuntime>::try_from(1u8).unwrap(),
		asset: AcceptedFundingAsset::USDT,
	});

	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(issuer(), 0),
			issuer(),
			default_evaluations(),
			bids,
			contributions,
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

	// Migration is ready
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	let pre_migration_balance = Penpal::account_data_of(buyer_1());

	// Migrate is sent
	let migrated_ct_amount = Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::migrate_one_participant(PolimecOrigin::signed(buyer_1()), project_id, buyer_1()));
		let (query_id, _migrations) = pallet_funding::UnconfirmedMigrations::<PolimecRuntime>::iter().next().unwrap();

		let user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));
		let user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));
		let user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));

		let evaluation_ct_amount = user_evaluations.map(|evaluation| {
			assert_eq!(evaluation.ct_migration_status, MigrationStatus::Sent(query_id));
			if let Some(RewardOrSlash::Reward(amount)) = evaluation.rewarded_or_slashed {
				amount
			} else {
				panic!("should be rewarded")
			}
		}).sum::<u128>();
		let bid_ct_amount = user_bids.map(|bid| {
			assert_eq!(bid.ct_migration_status, MigrationStatus::Sent(query_id));
			bid.final_ct_amount
		}).sum::<u128>();
		let contribution_ct_amount = user_contributions.map(|contribution| {
			assert_eq!(contribution.ct_migration_status, MigrationStatus::Sent(query_id));
			contribution.ct_amount
		}).sum::<u128>();

		evaluation_ct_amount + bid_ct_amount + contribution_ct_amount

	});


	Penpal::execute_with(|| {
		assert_expected_events!(
			Penpal,
			vec![
				PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationsExecutedForUser{user, ..}) => {
					user: *user == buyer_1(),
				},
			]
		);

		dbg!(Penpal::events());
	});

	// Balance is there for the user after vesting (Multiplier 1, so no vesting)
	let post_migration_balance = Penpal::account_data_of(buyer_1());
	dbg!(migrated_ct_amount);
	dbg!(pre_migration_balance.clone());
	dbg!(post_migration_balance.clone());

	Polimec::execute_with(|| {
		let mut user_evaluations =
			pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));
		let mut user_bids = pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));
		let mut user_contributions =
			pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id, buyer_1()));

		assert!(user_evaluations.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
		assert!(user_bids.all(|bid| bid.ct_migration_status == MigrationStatus::Confirmed));
		assert!(user_contributions.all(|contribution| contribution.ct_migration_status == MigrationStatus::Confirmed));

		assert_expected_events!(
			Polimec,
			vec![
				PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed{project_id, ..}) => {
					project_id: project_id == project_id,
				},
			]
		);
	});

	Penpal::execute_with(|| {
		let unblock_time: u32 = multiplier_for_vesting.calculate_vesting_duration::<PolimecRuntime>();
		PenpalSystem::set_block_number(unblock_time + 1u32);
		assert_ok!(pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(buyer_1())));
	});

	let post_vest_balance = Penpal::account_data_of(buyer_1());
	dbg!(post_vest_balance);
}

