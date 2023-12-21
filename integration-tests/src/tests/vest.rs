use crate::{polimec::ED, *};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::fungible::InspectHold;
use pallet_funding::LockType;
use pallet_vesting::VestingInfo;
use polimec_parachain_runtime::{Balances, LinearRelease, ParachainStaking, RuntimeOrigin, Vesting};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;

#[test]
fn vested_can_stake() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		let coll_1 = get_account_id_from_seed::<sr25519::Public>("COLL_1");
		let new_account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");

		// Initially the NEW_ACCOUNT has no PLMC
		assert_eq!(Balances::balance(&new_account), 0 * PLMC);

		// Stake 60 PLMC from "new_account" to "COLL_1", it should fail since the account has no PLMC
		assert_noop!(
			ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), coll_1.clone(), 60 * PLMC, 0, 0),
			pallet_parachain_staking::Error::<polimec_parachain_runtime::Runtime>::InsufficientBalance
		);

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

		// Stake 60 PLMC from "new_account" to "COLL_1", it should go through since the account has 60 + ED free PLMC
		assert_ok!(ParachainStaking::delegate(RuntimeOrigin::signed(new_account.clone()), coll_1, 60 * PLMC, 0, 0));
		// "New Account" has stil 60 + ED free PLMC, using fungible::Inspect. Locked (overlapped) in both staking and vesting
		assert_eq!(Balances::balance(&new_account), 60 * PLMC + ED);

		// Check that the staking state is correct
		ParachainStaking::delegator_state(&new_account).map(|state| {
			assert_eq!(state.total, 60 * PLMC);
			assert_eq!(state.delegations.0.len(), 1);
		});
	})
}
