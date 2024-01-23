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
use polimec_base_runtime::{Balances, ParachainStaking, RuntimeOrigin, Vesting};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128, Perquintill};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;
use frame_support::traits::LockableCurrency;
use frame_support::traits::ReservableCurrency;
use pallet_parachain_staking::HoldReason;
use frame_support::traits::tokens::Precision;
use frame_support::traits::Imbalance;
generate_accounts!(PEPE, CARLOS,);

#[test]
fn locks_holds_work_together() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);
		let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");

		// Initially the NEW_ACCOUNT has no PLMC
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);


		// Create a vesting schedule for 60 PLMC + ED over 60 blocks (~1 PLMC per block) to NEW_ACCOUNT
		let vesting_schedule = VestingInfo::new(
			60 * PLMC + ED,
			PLMC, // Vesting over 60 blocks
			1,
		);
		// The actual vested transfer
		assert_ok!(Vesting::vested_transfer(
			RuntimeOrigin::signed(alice.clone()),
			sp_runtime::MultiAddress::Id(new_account.clone()),
			vesting_schedule
		));

		// Alice now has 360 PLMC left
		assert_eq!(Balances::balance(&alice), 360 * PLMC - ED);

		// "New Account" has 60 free PLMC, using fungible::Inspect
		assert_eq!(Balances::balance(&new_account), 60 * PLMC + ED);

		// "New Account" only has ED free PLMC, using fungible::Inspect, since staking applies a `Hold` (which includes frozen balance)
		assert_eq!(Balances::balance(&new_account), 60*PLMC + ED);

        assert_ok!(Balances::hold(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 60 * PLMC));


		Balances::set_lock(*b"plmc/gov", &new_account, 60 * PLMC + ED, WithdrawReasons::all());

        assert_ok!(Balances::release(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 60 * PLMC, Precision::Exact));

        assert_noop!(Balances::reserve(&new_account, 60 * PLMC), pallet_balances::Error::<polimec_base_runtime::Runtime>::LiquidityRestrictions);

        assert_ok!(Balances::hold(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 60 * PLMC));


        
        let slashed = Balances::slash(&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(pallet_parachain_staking::HoldReason::StakingCollator), &new_account, 60 * PLMC);
        assert_eq!(slashed.0.peek(), 60 * PLMC);

        println!("{:?}", Balances::locks(&new_account));
        println!("{:?}", Balances::reserved_balance(&new_account));
        println!("{:?}", Balances::free_balance(&new_account));

	})
}


