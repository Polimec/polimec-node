use crate::{polimec_base::ED, *};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::fungible::{InspectHold, Mutate, MutateHold};
use frame_support::traits::WithdrawReasons;
use frame_support::traits::fungible::BalancedHold;
use macros::generate_accounts;
use pallet_funding::assert_close_enough;
use pallet_vesting::VestingInfo;
use polimec_base_runtime::{Balances, ParachainStaking, RuntimeOrigin, Vesting, Council, TechnicalCommittee, GrowthTreasury, Elections, Democracy, Preimage};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128, Perquintill};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;
use frame_support::traits::LockableCurrency;
use frame_support::traits::ReservableCurrency;
use pallet_parachain_staking::HoldReason;
use frame_support::traits::tokens::Precision;
use frame_support::traits::Imbalance;
use frame_support::traits::StorePreimage;
generate_accounts!(PEPE, CARLOS,);

#[test]
fn locks_holds_work_together() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);
		let new_account = create_vested_account();

		// Alice now has 360 PLMC left
		assert_eq!(Balances::balance(&alice), 220 * PLMC - ED);

		// "New Account" has 60 free PLMC, using fungible::Inspect
		assert_eq!(Balances::balance(&new_account), 200 * PLMC + ED);

		// "New Account" only has ED free PLMC, using fungible::Inspect, since staking applies a `Hold` (which includes frozen balance)
		assert_eq!(Balances::balance(&new_account), 200 * PLMC + ED);

        assert_ok!(Balances::hold(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 200 * PLMC));


		Balances::set_lock(*b"plmc/gov", &new_account, 200 * PLMC + ED, WithdrawReasons::all());

        assert_ok!(Balances::release(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 200 * PLMC, Precision::Exact));

        assert_noop!(Balances::reserve(&new_account, 200 * PLMC), pallet_balances::Error::<polimec_base_runtime::Runtime>::LiquidityRestrictions);

        assert_ok!(Balances::hold(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 200 * PLMC));


        
        let slashed = Balances::slash(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 200 * PLMC);
        assert_eq!(slashed.0.peek(), 200 * PLMC);

	})
}

fn assert_same_members(expected: Vec<AccountId>, actual: &Vec<AccountId>) {
	assert_eq!(expected.len(), actual.len());
	for member in expected {
		assert!(actual.contains(&member));
	}
}

fn create_vested_account() -> AccountId {
	let alice = PolimecBase::account_id_of(ALICE);
	let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");

	// Initially the NEW_ACCOUNT has no PLMC
	assert_eq!(Balances::balance(&new_account), 0 * PLMC);


	// Create a vesting schedule for 200 PLMC + ED over 60 blocks (~1 PLMC per block) to NEW_ACCOUNT
	let vesting_schedule = VestingInfo::new(
		200 * PLMC + ED,
		PLMC, // Vesting over 60 blocks
		1,
	);
	// The actual vested transfer
	assert_ok!(Vesting::vested_transfer(
		RuntimeOrigin::signed(alice.clone()),
		sp_runtime::MultiAddress::Id(new_account.clone()),
		vesting_schedule
	));
	new_account
}

#[test]
fn council_and_technical_committee_members_set_correctly() {
	let alice = PolimecBase::account_id_of(ALICE);
	let bob = PolimecBase::account_id_of(BOB);
	let charlie = PolimecBase::account_id_of(CHARLIE);
	let dave = PolimecBase::account_id_of(DAVE);
	let eve = PolimecBase::account_id_of(EVE);
	let accounts = vec![alice, bob, charlie, dave, eve];
	Polimec::execute_with(|| {
		assert_same_members(Council::members(), &accounts);
		assert_same_members(TechnicalCommittee::members(), &accounts);
	});
}

#[test]
fn democracy_works() {
	let alice = PolimecBase::account_id_of(ALICE);
	let bob = PolimecBase::account_id_of(BOB);
	Polimec::execute_with(|| {
		// Create a proposal to set the the balance of `account` to 1000 PLMC
		let account = create_vested_account();
		let bounded_call = Preimage::bound(<PolimecBase as xcm_emulator::Parachain>::RuntimeCall::Balances(
				pallet_balances::Call::force_set_balance { who: account.clone().into(), new_free: 1000u128 * PLMC }
			)).unwrap();
		assert_ok!(Democracy::propose(
			RuntimeOrigin::signed(account.clone()),
			bounded_call,
			100 * PLMC,
		));

		// Democracy::
	});

}

