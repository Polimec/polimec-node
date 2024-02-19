use crate::{polimec_base::ED, *};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::{
	fungible::{BalancedHold, MutateFreeze, MutateHold, Unbalanced},
	OnInitialize, WithdrawReasons,
};
use macros::generate_accounts;
use sp_runtime::{traits::Hash, Digest};

use frame_support::{
	dispatch::GetDispatchInfo,
	traits::{
		fungible::InspectFreeze, tokens::Precision, Imbalance, LockableCurrency, ReservableCurrency, StorePreimage,
	},
};
use pallet_democracy::{AccountVote, Conviction, GetElectorate, ReferendumInfo, Vote};
use pallet_vesting::VestingInfo;
use polimec_base_runtime::{
	Balances, Council, Democracy, Elections, ParachainStaking, PayMaster, Preimage, RuntimeOrigin, TechnicalCommittee,
	Treasury, Vesting,
};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;
generate_accounts!(PEPE, CARLOS,);

/// Test that an account with vested tokens (a lock) can use those tokens for a hold.
/// The hold can also be released or slashed while the lock is still in place.
#[test]
fn vested_tokens_and_holds_work_together() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);
		let new_account = create_vested_account();

		assert_eq!(Balances::balance(&alice), 220 * PLMC - ED);
		assert_eq!(Balances::balance(&new_account), 200 * PLMC + ED);

		assert_ok!(Balances::hold(
			&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(
				pallet_parachain_staking::HoldReason::StakingCollator
			),
			&new_account,
			200 * PLMC
		));
		Balances::set_lock(*b"plmc/gov", &new_account, 200 * PLMC + ED, WithdrawReasons::all());
		assert_ok!(Balances::release(
			&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(
				pallet_parachain_staking::HoldReason::StakingCollator
			),
			&new_account,
			200 * PLMC,
			Precision::Exact
		));

		assert_eq!(Balances::reserved_balance(&new_account), 0);

		assert_ok!(Balances::hold(
			&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(
				pallet_parachain_staking::HoldReason::StakingCollator
			),
			&new_account,
			200 * PLMC
		));
		let slashed = Balances::slash(
			&polimec_base_runtime::RuntimeHoldReason::ParachainStaking(
				pallet_parachain_staking::HoldReason::StakingCollator,
			),
			&new_account,
			200 * PLMC,
		);
		assert_eq!(slashed.0.peek(), 200 * PLMC);

		assert_eq!(Balances::reserved_balance(&new_account), 0);
	})
}

/// Test that an account with vested tokens (a lock) cannot use those tokens for a reserve.
#[test]
fn vested_tokens_and_reserves_dont_work_together() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);
		let new_account = create_vested_account();

		assert_eq!(Balances::balance(&alice), 220 * PLMC - ED);
		assert_eq!(Balances::balance(&new_account), 200 * PLMC + ED);

		assert_noop!(
			Balances::reserve(&new_account, 200 * PLMC),
			pallet_balances::Error::<polimec_base_runtime::Runtime>::LiquidityRestrictions
		);
	});
}

/// Test that locks and freezes can be placed on balance that is already reserved.
#[test]
fn lock_and_freeze_after_reserve_does_work() {
	PolimecBase::execute_with(|| {
		let alice = PolimecBase::account_id_of(ALICE);

		assert_ok!(Balances::reserve(&alice, 400 * PLMC));
		assert_ok!(Balances::set_freeze(
			&polimec_base_runtime::RuntimeFreezeReason::Democracy(pallet_democracy::FreezeReason::Vote),
			&alice,
			400 * PLMC
		));
		Balances::set_lock(*b"py/trsry", &alice, 400 * PLMC, WithdrawReasons::all());
	});
}

/// Test that correct members are set with the default genesis config.
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

/// Test that basic democracy works correctly.
/// 1. Public proposal is created.
/// 2. Public votes on the proposal.
/// 3. Proposal is approved.
/// 4. Proposal is enacted.
#[test]
fn democracy_works() {
	let alice = PolimecBase::account_id_of(ALICE);
	// 1. Create a proposal to set the the balance of `account` to 1000 PLMC
	PolimecBase::execute_with(|| {
		let account = create_vested_account();
		let bounded_call = Preimage::bound(<PolimecBase as xcm_emulator::Parachain>::RuntimeCall::Balances(
			pallet_balances::Call::force_set_balance { who: account.clone().into(), new_free: 1000u128 * PLMC },
		))
		.unwrap();
		assert_ok!(Democracy::propose(RuntimeOrigin::signed(account.clone()), bounded_call, 100 * PLMC,));
	});

	run_gov_n_blocks(1);
	// 2. Proposal is turned into a referendum
	// Alice votes on the proposal with 100 PLMC
	PolimecBase::execute_with(|| {
		assert!(Democracy::referendum_count() == 1);
		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Democracy(pallet_democracy::FreezeReason::Vote),
				&alice
			),
			0
		);
		do_vote(alice.clone(), 0, true, 100 * PLMC);
		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Democracy(pallet_democracy::FreezeReason::Vote),
				&alice
			),
			100 * PLMC
		);
	});

	run_gov_n_blocks(2);
	// 3. Referendum is approved
	PolimecBase::execute_with(|| {
		assert_eq!(Democracy::referendum_info(0).unwrap(), ReferendumInfo::Finished { approved: true, end: 4u32 });
		assert!(pallet_scheduler::Agenda::<polimec_base_runtime::Runtime>::get(6u32).len() == 1);
	});

	// 4. Referendum is enacted
	run_gov_n_blocks(2);
	PolimecBase::execute_with(|| {
		assert_eq!(Balances::balance(&get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT")), 1000u128 * PLMC);
	});
}

// Test that electorate configuration calculates correctly.
// Electorate is the total issuance minus the sum of the Growth + Operational treasury.
#[test]
fn electorate_calculates_correctly() {
	PolimecBase::execute_with(|| {
		let total_issuance = Balances::total_issuance();
		assert_ok!(Balances::write_balance(&Treasury::account_id(), 1000 * PLMC));
		assert_ok!(Balances::write_balance(
			&<polimec_base_runtime::Runtime as pallet_parachain_staking::Config>::PayMaster::get(),
			1000 * PLMC
		));
		assert_eq!(
			<polimec_base_runtime::Runtime as pallet_democracy::Config>::Electorate::get_electorate(),
			total_issuance - 2000 * PLMC
		);
	})
}

/// Test that a user with staked balance can vote on a democracy proposal.
#[test]
fn user_can_vote_with_staked_balance() {
	// 1. Create a proposal to set the the balance of `account` to 1000 PLMC
	// 2. Account stakes 100 PLMC.
	PolimecBase::execute_with(|| {
		let account = create_vested_account();
		let bounded_call = Preimage::bound(<PolimecBase as xcm_emulator::Parachain>::RuntimeCall::Balances(
			pallet_balances::Call::force_set_balance { who: account.clone().into(), new_free: 1000u128 * PLMC },
		))
		.unwrap();
		assert_ok!(Democracy::propose(RuntimeOrigin::signed(account.clone()), bounded_call, 100 * PLMC));

		assert_ok!(ParachainStaking::delegate(
			RuntimeOrigin::signed(account.clone()),
			get_account_id_from_seed::<sr25519::Public>("COLL_1"),
			100 * PLMC,
			0,
			0
		));

		// Total PLMC reserved for staking (100) + creating proposal (100) = 200
		assert_eq!(Balances::reserved_balance(&account), 200 * PLMC)
	});

	run_gov_n_blocks(1);
	// 3. User votes on the proposal with 200 PLMC
	PolimecBase::execute_with(|| {
		let account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");
		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Democracy(pallet_democracy::FreezeReason::Vote),
				&account
			),
			0
		);
		do_vote(account.clone(), 0, true, 200 * PLMC);
		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Democracy(pallet_democracy::FreezeReason::Vote),
				&account
			),
			200 * PLMC
		);
	})
}

/// Test that treasury proposals can be directly accepted by the council without going through governance.
#[test]
fn treasury_proposal_accepted_by_council() {
	let alice = PolimecBase::account_id_of(ALICE);
	let bob = PolimecBase::account_id_of(BOB);
	let charlie = PolimecBase::account_id_of(CHARLIE);
	let dave = PolimecBase::account_id_of(DAVE);
	let eve = PolimecBase::account_id_of(EVE);
	let accounts = vec![(alice.clone(), true), (bob, true), (charlie, true), (dave, true), (eve, true)];
	PolimecBase::execute_with(|| {
		// 0. Set the treasury balance to 1000 PLMC
		assert_ok!(Balances::write_balance(&Treasury::account_id(), 1000 * PLMC));

		// 1. Create treasury proposal for 100 PLMC
		assert_ok!(Treasury::propose_spend(
			RuntimeOrigin::signed(alice.clone()),
			100 * PLMC,
			get_account_id_from_seed::<sr25519::Public>("Beneficiary").into()
		));
		assert_eq!(Treasury::proposal_count(), 1);

		// 2. Council will vote on the proposal
		let proposal =
			polimec_base_runtime::RuntimeCall::Treasury(pallet_treasury::Call::approve_proposal { proposal_id: 0 });
		assert_ok!(Council::propose(RuntimeOrigin::signed(alice.clone()), 5, Box::new(proposal.clone()), 100,));

		// 3. Council votes on the proposal
		let proposal_hash = <polimec_base_runtime::Runtime as frame_system::Config>::Hashing::hash_of(&proposal);
		do_council_vote_for(accounts.clone(), 0, proposal_hash);

		// 4. Proposal is approved
		assert_ok!(Council::close(
			RuntimeOrigin::signed(alice.clone()),
			proposal_hash,
			0,
			proposal.get_dispatch_info().weight,
			100,
		));
	});

	run_gov_n_blocks(3);

	PolimecBase::execute_with(|| {
		// 5. Beneficiary receives the funds
		assert_eq!(Balances::balance(&get_account_id_from_seed::<sr25519::Public>("Beneficiary")), 100 * PLMC);
	});
}

/// Test that treasury proposals can be directly rejected by the council without going through governance.
/// The treasury proposal deposit is slashed and sent to the treasury.
#[test]
fn slashed_treasury_proposal_funds_send_to_treasury() {
	let alice = PolimecBase::account_id_of(ALICE);
	PolimecBase::execute_with(|| {
		// 0. Set the treasury balance to 1000 PLMC
		assert_ok!(Balances::write_balance(&Treasury::account_id(), 1000 * PLMC));
		let alice_balance = Balances::balance(&alice);
		// 1. Create treasury proposal for 100 PLMC
		assert_ok!(Treasury::propose_spend(
			RuntimeOrigin::signed(alice.clone()),
			100 * PLMC,
			get_account_id_from_seed::<sr25519::Public>("Beneficiary").into()
		));

		// 2. Reject treasury proposal
		assert_ok!(Treasury::reject_proposal(
			pallet_collective::RawOrigin::<AccountId, pallet_collective::Instance1>::Members(5, 9).into(),
			0u32,
		));

		// 3. See that the funds are slashed and sent to treasury
		assert_eq!(Balances::balance(&Treasury::account_id()), 1050 * PLMC);
		assert_eq!(Balances::balance(&alice), alice_balance - 50 * PLMC);
	});
}

/// Test that users can vote in the election-phragmen pallet with their staked balance.
#[test]
fn user_can_vote_in_election_with_staked_balance() {
	let alice = PolimecBase::account_id_of(ALICE);
	PolimecBase::execute_with(|| {
		let account = create_vested_account();

		assert_ok!(ParachainStaking::delegate(
			RuntimeOrigin::signed(account.clone()),
			get_account_id_from_seed::<sr25519::Public>("COLL_1"),
			200 * PLMC,
			0,
			0
		));

		// Total PLMC reserved for staking (100) + creating proposal (100) = 200
		assert_eq!(Balances::reserved_balance(&account), 200 * PLMC);

		assert_ok!(Elections::vote(RuntimeOrigin::signed(account.clone()), vec![alice], 200 * PLMC,));

		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Elections(pallet_elections_phragmen::FreezeReason::Voting),
				&account
			),
			200 * PLMC
		);

		assert_noop!(
			Elections::remove_voter(RuntimeOrigin::signed(account.clone())),
			pallet_elections_phragmen::Error::<polimec_base_runtime::Runtime>::VotingPeriodNotEnded
		);
	});

	run_gov_n_blocks(5);

	PolimecBase::execute_with(|| {
		let account = get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");

		assert_ok!(Elections::remove_voter(RuntimeOrigin::signed(account.clone())));
		assert_eq!(
			Balances::balance_frozen(
				&polimec_base_runtime::RuntimeFreezeReason::Elections(pallet_elections_phragmen::FreezeReason::Voting),
				&account
			),
			0
		);
	});
}

/// Tests that the election works as expected.
/// 1. Register 32 candidates
/// 2. 8 accounts vote for 8 candidates
/// 3. Run the election
/// 4. Check that the 9 candidates with the most votes are elected
/// 5. Check that the 6 candidates with the next most votes are runners up
/// 6. Check that the remaining candidates have their funds slashed as they did not receive any votes
#[test]
fn election_phragmen_works() {
	let candidates = (1..=32)
		.into_iter()
		.map(|i| get_account_id_from_seed::<sr25519::Public>(format!("CANDIDATE_{}", i).as_str()))
		.collect::<Vec<AccountId>>();
	// 1. Register candidates for the election.
	PolimecBase::execute_with(|| {
		assert_eq!(Elections::candidates().len(), 0);
		// Alice .. Eve already selected members
		assert_eq!(Elections::members().len(), 5);
		assert_eq!(Elections::runners_up().len(), 0);

		for (i, candidate) in candidates.iter().enumerate() {
			assert_ok!(Balances::write_balance(&candidate, 1000 * PLMC + ED));
			assert_ok!(Elections::submit_candidacy(RuntimeOrigin::signed((*candidate).clone()), i as u32));
		}

		assert_eq!(Elections::candidates().len(), 32);

		for (i, voter) in vec![ALICE, BOB, CHARLIE, DAVE, EVE, FERDIE, ALICE_STASH, BOB_STASH].into_iter().enumerate() {
			let voter = PolimecBase::account_id_of(voter);
			assert_ok!(Elections::vote(
				RuntimeOrigin::signed(voter.clone()),
				candidates[i..(i + 8)].to_vec(),
				200 * PLMC,
			));
		}
	});

	run_gov_n_blocks(5);

	PolimecBase::execute_with(|| {
		assert_eq!(Elections::candidates().len(), 0);
		assert_eq!(Elections::members().len(), 9);
		assert_eq!(Elections::runners_up().len(), 6);

		let expected_runners_up = candidates[0..3].iter().cloned().chain(candidates[12..15].iter().cloned()).collect();
		assert_same_members(Elections::members().into_iter().map(|m| m.who).collect(), &(candidates[3..12].to_vec()));
		assert_same_members(Elections::runners_up().into_iter().map(|m| m.who).collect(), &expected_runners_up);

		// Check that the candidates that were not elected have their funds slashed
		for candidate in &candidates[15..32] {
			assert_eq!(Balances::total_balance(candidate), ED);
		}
		assert_eq!(Balances::balance(&Treasury::account_id()), 17 * 1000 * PLMC + ED)
	});
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

fn run_gov_n_blocks(n: usize) {
	for _ in 0..n {
		PolimecBase::execute_with(|| {
			let block_number = polimec_base_runtime::System::block_number();

			let header = polimec_base_runtime::System::finalize();

			let pre_digest = Digest { logs: vec![] };
			polimec_base_runtime::System::reset_events();

			let next_block_number = block_number + 1u32;
			polimec_base_runtime::Vesting::on_initialize(next_block_number);
			polimec_base_runtime::Elections::on_initialize(next_block_number);
			polimec_base_runtime::Council::on_initialize(next_block_number);
			polimec_base_runtime::TechnicalCommittee::on_initialize(next_block_number);
			polimec_base_runtime::Treasury::on_initialize(next_block_number);
			polimec_base_runtime::Democracy::on_initialize(next_block_number);
			polimec_base_runtime::Preimage::on_initialize(next_block_number);
			polimec_base_runtime::Scheduler::on_initialize(next_block_number);
			polimec_base_runtime::System::initialize(&next_block_number, &header.hash(), &pre_digest);
		});
	}
}

fn do_vote(account: AccountId, index: u32, approve: bool, amount: u128) {
	assert_ok!(Democracy::vote(
		RuntimeOrigin::signed(account.clone()),
		index,
		AccountVote::Standard { balance: amount, vote: Vote { aye: approve, conviction: Conviction::Locked1x } },
	));
}

fn do_council_vote_for(accounts: Vec<(AccountId, bool)>, index: u32, hash: polimec_base_runtime::Hash) {
	for (account, approve) in accounts {
		assert_ok!(Council::vote(RuntimeOrigin::signed(account.clone()), hash, index, approve,));
	}
}
