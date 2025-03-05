use crate::{constants::PricesBuilder, tests::defaults::*, *};
use frame_support::traits::fungible::Mutate;
use frame_system::{pallet_prelude::BlockNumberFor, Account};
use macros::generate_accounts;
use pallet_balances::AccountData;
use pallet_funding::*;
use pallet_vesting::VestingInfo;
use polimec_common::USD_UNIT;
use polimec_runtime::PLMC;
use sp_arithmetic::Perquintill;
use sp_runtime::{FixedU128, MultiAddress::Id};
use xcm::v5::Junction;

generate_accounts!(STASH, ALICE, BOB, CHARLIE, DAVE, ISSUER);

#[test]
fn evaluator_slash_reduces_vesting_schedules() {
	// Set PLMC price to 1 USD
	let prices = PricesBuilder::new().plmc(FixedU128::from_float(1.0)).build();
	polimec::set_prices(prices);

	let mut inst = IntegrationInstantiator::new(None);
	let alice: PolimecAccountId = ALICE.into();
	let bob: PolimecAccountId = BOB.into();

	PolimecNet::execute_with(|| {
		// Account that does the vested transfers (i.e. treasury)
		PolimecBalances::set_balance(&STASH.into(), 1_000_000 * PLMC);
		// For alice, we try to slash 4 schedules, each with a different duration and amount.
		// One schedule should dissapear to due being fully vested before the slash, the other ones should be modified
		PolimecBalances::set_balance(&alice.clone(), 0);
		// For bob, we try to slash 1 schedule, which makes it dissapear since the slash amount is higher than the amount remaining for vesting
		PolimecBalances::set_balance(&bob.clone(), BOB_STARTING_BALANCE);

		let slash_percent = <PolimecRuntime as pallet_funding::Config>::EvaluatorSlash::get();

		const BOB_STARTING_BALANCE: u128 = 100_000 * PLMC;
		const LOCK_1: u128 = 10_000 * PLMC;
		const LOCK_2: u128 = 13_000 * PLMC;
		const LOCK_3: u128 = 7_500 * PLMC;
		const LOCK_4: u128 = 20_000 * PLMC;
		const PER_BLOCK_1: u128 = 10 * PLMC;
		const PER_BLOCK_2: u128 = 650 * PLMC;
		const PER_BLOCK_3: u128 = 5 * PLMC;
		const PER_BLOCK_4: u128 = 200 * PLMC;
		const DURATION_1: u128 = LOCK_1 / PER_BLOCK_1;
		const DURATION_3: u128 = LOCK_3 / PER_BLOCK_3;
		const DURATION_4: u128 = LOCK_4 / PER_BLOCK_4;

		// Duration 1000 blocks
		let vesting_info_1 = VestingInfo::new(LOCK_1, PER_BLOCK_1, 5);
		// Duration 20 blocks
		let vesting_info_2 = VestingInfo::new(LOCK_2, PER_BLOCK_2, 5);
		// Duration 1500 blocks
		let vesting_info_3 = VestingInfo::new(LOCK_3, PER_BLOCK_3, 5);
		// Duration 100 blocks
		let vesting_info_4 = VestingInfo::new(LOCK_4, PER_BLOCK_4, 5);

		let total_alice_transferred = LOCK_1 + LOCK_2 + LOCK_3 + LOCK_4;
		assert_ok!(PolimecVesting::vested_transfer(
			PolimecOrigin::signed(STASH.into()),
			Id(alice.clone()),
			vesting_info_1
		));
		assert_ok!(PolimecVesting::vested_transfer(
			PolimecOrigin::signed(STASH.into()),
			Id(alice.clone()),
			vesting_info_2
		));
		assert_ok!(PolimecVesting::vested_transfer(
			PolimecOrigin::signed(STASH.into()),
			Id(alice.clone()),
			vesting_info_3
		));
		assert_ok!(PolimecVesting::vested_transfer(
			PolimecOrigin::signed(STASH.into()),
			Id(alice.clone()),
			vesting_info_4
		));

		let alice_evaluation = EvaluationParams::<PolimecRuntime>::new(
			alice.clone(),
			35_000 * USD_UNIT,
			Junction::AccountId32 { network: Some(NetworkId::Polkadot), id: [0u8; 32] },
		);
		let alice_plmc_evaluated = inst.calculate_evaluation_plmc_spent(vec![alice_evaluation.clone()])[0].plmc_amount;
		let alice_slashed = slash_percent * alice_plmc_evaluated;

		const BOB_EVALUATION: u128 = 60_000;
		// We want the amount to be slashed to be higher than the amount remaining for vesting, after unlocking some tokens
		let lock_5: u128 = ((slash_percent * BOB_EVALUATION) * PLMC) + PER_BLOCK_5 * 10;
		const PER_BLOCK_5: u128 = 100 * PLMC;
		let vesting_info_5 = VestingInfo::new(lock_5, PER_BLOCK_5, 5);

		assert_ok!(PolimecVesting::vested_transfer(
			PolimecOrigin::signed(STASH.into()),
			Id(bob.clone()),
			vesting_info_5
		));
		let bob_evaluation = EvaluationParams::<PolimecRuntime>::new(
			bob.clone(),
			BOB_EVALUATION * USD_UNIT,
			Junction::AccountId32 { network: Some(NetworkId::Polkadot), id: [0u8; 32] },
		);
		let bob_plmc_evaluated = inst.calculate_evaluation_plmc_spent(vec![bob_evaluation.clone()])[0].plmc_amount;
		let bob_slashed = slash_percent * bob_plmc_evaluated;

		// Set metadata so 50k USD succeeds the evaluation round
		let mut project_metadata = default_project_metadata(ISSUER.into());
		project_metadata.total_allocation_size = 5_000 * CT_UNIT;

		// Create a project where alice and bob evaluated making the round successful, but then the project fails funding at block 25.
		let project_id = inst.create_evaluating_project(project_metadata, ISSUER.into(), None);
		assert_ok!(inst.evaluate_for_users(project_id, vec![alice_evaluation.clone(), bob_evaluation.clone()]));
		assert_eq!(ProjectStatus::AuctionRound, inst.go_to_next_state(project_id));
		assert_eq!(ProjectStatus::FundingFailed, inst.go_to_next_state(project_id));
		assert_eq!(ProjectStatus::SettlementStarted(FundingOutcome::Failure), inst.go_to_next_state(project_id));

		const END_BLOCK: u32 = 18;
		assert_eq!(inst.current_block(), BlockNumberFor::<PolimecRuntime>::from(END_BLOCK));

		// All schedules start at block 5, and funding ended at block 18
		const TIME_PASSED: u128 = 13u128;

		let alice_account_data = Account::<PolimecRuntime>::get(&alice.clone()).data;
		assert_eq!(
			alice_account_data,
			AccountData {
				free: total_alice_transferred - alice_plmc_evaluated,
				reserved: alice_plmc_evaluated,
				frozen: total_alice_transferred,
				flags: Default::default(),
			}
		);
		assert_eq!(PolimecBalances::usable_balance(alice.clone()), 0);

		// vest schedule 2 was fully vested
		assert_ok!(PolimecVesting::vest(PolimecOrigin::signed(alice.clone())));
		let alice_account_data = Account::<PolimecRuntime>::get(&alice.clone()).data;
		let vested = (PER_BLOCK_1 + PER_BLOCK_2 + PER_BLOCK_3 + PER_BLOCK_4) * TIME_PASSED;

		let free = total_alice_transferred - alice_plmc_evaluated;
		let reserved = alice_plmc_evaluated;
		let frozen = total_alice_transferred - vested;

		// `untouchable` is the amount we need to substract from the free balance to get the usable balance.
		// When the reserved amount is higher than the frozen amount, it means that the frozen balance restriction is fully covered by the already reserved tokens
		// When the frozen amount is higher, it means we need to use some free tokens to cover this restriction.
		// This amount can never be below the existential deposit.
		let untouchable = frozen.saturating_sub(reserved).max(inst.get_ed());

		assert_eq!(alice_account_data, AccountData { free, reserved, frozen, flags: Default::default() });
		assert_eq!(PolimecBalances::usable_balance(alice.clone()), free - untouchable);

		assert_ok!(PolimecFunding::settle_evaluation(
			PolimecOrigin::signed(alice.clone()),
			project_id,
			alice.clone(),
			0
		));

		let alice_schedules = <pallet_vesting::Vesting<PolimecRuntime>>::get(alice.clone()).unwrap().to_vec();
		let new_lock_1 = LOCK_1 - (PER_BLOCK_1 * TIME_PASSED) - alice_slashed;
		let new_lock_3 = LOCK_3 - (PER_BLOCK_3 * TIME_PASSED) - alice_slashed;
		let new_lock_4 = LOCK_4 - (PER_BLOCK_4 * TIME_PASSED) - alice_slashed;
		const TIME_REMAINING_1: u128 = DURATION_1 - TIME_PASSED;
		const TIME_REMAINING_3: u128 = DURATION_3 - TIME_PASSED;
		const TIME_REMAINING_4: u128 = DURATION_4 - TIME_PASSED;
		let new_per_block_1 = new_lock_1 / TIME_REMAINING_1;
		let new_per_block_3 = new_lock_3 / TIME_REMAINING_3;
		let new_per_block_4 = new_lock_4 / TIME_REMAINING_4;

		let alice_account_data = Account::<PolimecRuntime>::get(&alice.clone()).data;
		let free = free + alice_plmc_evaluated - alice_slashed;
		let frozen = new_lock_1 + new_lock_3 + new_lock_4;
		let reserved = 0u128;
		let untouchable = frozen.saturating_sub(reserved).max(inst.get_ed());

		assert_eq!(
			alice_schedules,
			vec![
				VestingInfo::new(new_lock_1, new_per_block_1, END_BLOCK),
				VestingInfo::new(new_lock_3, new_per_block_3, END_BLOCK),
				VestingInfo::new(new_lock_4, new_per_block_4, END_BLOCK),
			]
		);
		assert_eq!(alice_account_data, AccountData { free, reserved, frozen, flags: Default::default() });
		assert_eq!(PolimecBalances::usable_balance(alice.clone()), free - untouchable);

		let bob_account_data = Account::<PolimecRuntime>::get(&bob.clone()).data;
		let free = BOB_STARTING_BALANCE + lock_5 - bob_plmc_evaluated;
		let reserved = bob_plmc_evaluated;
		let frozen = lock_5;
		assert_eq!(bob_account_data, AccountData { free, reserved, frozen, flags: Default::default() });
		let untouchable = frozen.saturating_sub(reserved).max(inst.get_ed());
		assert_eq!(PolimecBalances::usable_balance(bob.clone()), free - untouchable);

		// Schedule has some tokens vested, and the remaining locked amount is lower than the slash about to occur
		assert_ok!(PolimecVesting::vest(PolimecOrigin::signed(bob.clone())));
		let bob_account_data = Account::<PolimecRuntime>::get(&bob.clone()).data;
		let vested = PER_BLOCK_5 * TIME_PASSED;

		let free = BOB_STARTING_BALANCE + lock_5 - bob_plmc_evaluated;
		let reserved = bob_plmc_evaluated;
		let frozen = lock_5 - vested;
		let untouchable = frozen.saturating_sub(reserved).max(inst.get_ed());
		assert_eq!(bob_account_data, AccountData { free, reserved, frozen, flags: Default::default() });
		assert_eq!(PolimecBalances::usable_balance(bob.clone()), free - untouchable);

		// Here the slash amount is higher than the amount remaining for vesting, so the schedule should dissapear
		assert_ok!(PolimecFunding::settle_evaluation(PolimecOrigin::signed(bob.clone()), project_id, bob.clone(), 1));
		assert!(bob_slashed > lock_5 - vested && bob_slashed < lock_5);

		assert!(<pallet_vesting::Vesting<PolimecRuntime>>::get(bob.clone()).is_none());
		let bob_account_data = Account::<PolimecRuntime>::get(&bob.clone()).data;
		let free = free + bob_plmc_evaluated - bob_slashed;
		let frozen = 0u128;
		let reserved = 0u128;
		let untouchable = frozen.saturating_sub(reserved).max(inst.get_ed());

		assert_eq!(bob_account_data, AccountData { free, reserved, frozen, flags: Default::default() });
		assert_close_enough!(
			PolimecBalances::usable_balance(bob.clone()),
			free - untouchable,
			Perquintill::from_float(0.999)
		);
	});
}
