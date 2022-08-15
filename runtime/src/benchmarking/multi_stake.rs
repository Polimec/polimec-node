use sp_std::prelude::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_support::storage::{StorageDoubleMap, StorageMap};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;

use super::utils::{create_funded_user, lookup_of_account, SEED};
use crate::benchmarking::utils::fill_proposals;
use crate::benchmarking::utils::MAX_PROPOSALS;
use crate::{
	AccountId, Balance, BondingDuration, CurrencyId, IssuerCouncil, MultiStake, Origin, PreCurrencyMint, Runtime,
	System,
};
use issuer_council::{BondingConfig, CouncilProposal, CurrencyConfig, PayoutsEnabled};
use multi_stake::{Bonded, Ledger, UnlockChunk, MAX_UNLOCKING_CHUNKS};

/// Create a stash and controller pair.
pub fn create_stash_controller(
	n: u32,
	currency_id: CurrencyId,
	amount: Balance,
) -> Result<(AccountId, AccountId, AccountId), &'static str> {
	let stash: AccountId = create_funded_user("stash", currency_id, amount, n);
	let controller: AccountId = account("conroller", n, SEED);
	let controller_lookup = lookup_of_account(controller.clone());
	Ok((stash, controller, controller_lookup))
}

const CURRENCY_ID: CurrencyId = [0_u8, 0, 0, 0, 0, 0, 0, 0xFF];
const AMOUNT: Balance = 100_000_000_000;
const AMOUNT_EXTRA: Balance = 100_000_000;

runtime_benchmarks! {
	{ Runtime, multi_stake }

	_ {
	}

	bond {
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
	}: _(RawOrigin::Signed(stash.clone()), controller_lookup, CURRENCY_ID, AMOUNT)
	verify {
		assert!(Bonded::<Runtime>::contains_key(stash, CURRENCY_ID));
		assert!(Ledger::<Runtime>::contains_key(controller, CURRENCY_ID));
	}

	bond_extra {
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT + AMOUNT_EXTRA)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created before")?;
		let original_bonded: Balance = ledger.active;
	}: _(RawOrigin::Signed(stash.clone()), CURRENCY_ID, AMOUNT_EXTRA)
	verify {
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created after")?;
		let new_bonded: Balance = ledger.active;
		assert!(original_bonded < new_bonded);
	}

	unbond {
		let p in 1..MAX_PROPOSALS => ();
		let proposals = fill_proposals(p as usize);

		// bond currency
		PreCurrencyMint::register(Origin::root(), account("issuer", 0, SEED), CURRENCY_ID)?;
		let (stash, controller, controller_lookup) = create_stash_controller(1, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;

		// vote on all proposals
		CurrencyConfig::<Runtime>::insert(CURRENCY_ID, BondingConfig {payout: PayoutsEnabled::No, vote: true});
		for CouncilProposal::<Runtime> { proposal_hash, .. } in proposals.iter() {
			IssuerCouncil::vote(RawOrigin::Signed(stash.clone()).into(), *proposal_hash, CURRENCY_ID, true)?;
		}

		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created before")?;
		let original_bonded: Balance = ledger.active;
	}: _(RawOrigin::Signed(controller.clone()), CURRENCY_ID, AMOUNT)
	verify {
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created after")?;
		let new_bonded: Balance = ledger.active;
		assert!(original_bonded > new_bonded);
	}

	set_controller {
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;
		let new_controller = create_funded_user("new_controller", CURRENCY_ID, 0, 1);
		let new_controller_lookup = lookup_of_account(new_controller.clone());
	}: _(RawOrigin::Signed(stash), new_controller_lookup, CURRENCY_ID)
	verify {
		assert!(Ledger::<Runtime>::contains_key(&new_controller, CURRENCY_ID));
	}

	force_unstake {
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;
	}: _(RawOrigin::Root, stash, CURRENCY_ID)
	verify {
		assert!(!Ledger::<Runtime>::contains_key(&controller, CURRENCY_ID));
	}

	// Withdraw only updates the ledger
	withdraw_unbonded_update {
		let l in 1 .. MAX_UNLOCKING_CHUNKS as u32;
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;

		// worst case scenario: max number of unlocks for same block
		for _ in 0 .. l {
			MultiStake::unbond(RawOrigin::Signed(controller.clone()).into(), CURRENCY_ID, 100)?;
		}

		// increase block to enable withdrawal
		let block = System::block_number();
		System::set_block_number(BondingDuration::get() * 3 + block);
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created before")?;
		let original_total: Balance = ledger.total;
	}: withdraw_unbonded(RawOrigin::Signed(controller.clone()), CURRENCY_ID)
	verify {
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created after")?;
		let new_total: Balance = ledger.total;
		assert!(original_total > new_total);
	}

	// Worst case scenario, everything is removed after the bonding duration
	withdraw_unbonded_kill {
		let l in 1 .. MAX_UNLOCKING_CHUNKS as u32;
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;

		// worst case scenario: max number of unlocks for same block
		for _ in 1 .. l {
			MultiStake::unbond(RawOrigin::Signed(controller.clone()).into(), CURRENCY_ID, AMOUNT / MAX_UNLOCKING_CHUNKS as u128)?;
		}
		// unbond all remaining balance
		MultiStake::unbond(RawOrigin::Signed(controller.clone()).into(), CURRENCY_ID, AMOUNT)?;

		// increase block to enable withdrawal
		let block = System::block_number();
		System::set_block_number(BondingDuration::get() * 2 + block);
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created before")?;
		let original_total: Balance = ledger.total;
	}: withdraw_unbonded(RawOrigin::Signed(controller.clone()), CURRENCY_ID)
	verify {
		assert!(!Bonded::<Runtime>::contains_key(stash, CURRENCY_ID));
		assert!(!Ledger::<Runtime>::contains_key(controller, CURRENCY_ID));
	}


	rebond {
		let l in 1 .. MAX_UNLOCKING_CHUNKS as u32;
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, AMOUNT)?;
		MultiStake::bond(RawOrigin::Signed(stash).into(), controller_lookup, CURRENCY_ID, AMOUNT)?;
		let mut staking_ledger = Ledger::<Runtime>::get(controller.clone(), CURRENCY_ID).unwrap();
		let unlock_chunk = UnlockChunk {
			value: 1u32.into(),
			block: System::block_number(),
		};
		for _ in 0 .. l {
			staking_ledger.unlocking.push(unlock_chunk.clone())
		}
		Ledger::<Runtime>::insert(controller.clone(), CURRENCY_ID, staking_ledger.clone());
		let original_bonded: Balance = staking_ledger.active;
	}: _(RawOrigin::Signed(controller.clone()), CURRENCY_ID, (l + 100).into())
	verify {
		let ledger = Ledger::<Runtime>::get(&controller, CURRENCY_ID).ok_or("ledger not created after")?;
		let new_bonded: Balance = ledger.active;
		assert!(original_bonded < new_bonded);
	}

	reap_stash {
		let (stash, controller, controller_lookup) = create_stash_controller(0, CURRENCY_ID, 0)?;
		MultiStake::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, CURRENCY_ID, 0)?;
	}: _(RawOrigin::Signed(controller), stash.clone(), CURRENCY_ID)
	verify {
		assert!(!Bonded::<Runtime>::contains_key(&stash, CURRENCY_ID));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;

	fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}

	#[test]
	fn bond() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_bond());
		});
	}
	#[test]
	fn bond_extra() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_bond_extra());
		});
	}
	#[test]
	fn unbond() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_unbond());
		});
	}
	#[test]
	fn set_controller() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_controller());
		});
	}
	#[test]
	fn force_unstake() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_force_unstake());
		});
	}
	#[test]
	fn withdraw_unbonded_update() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_withdraw_unbonded_update());
		});
	}
	#[test]
	fn withdraw_unbonded_kill() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_withdraw_unbonded_kill());
		});
	}
	#[test]
	fn rebond() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_rebond());
		});
	}
	#[test]
	fn reap_stash() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_reap_stash());
		});
	}
}
