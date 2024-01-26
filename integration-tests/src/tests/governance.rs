use crate::{polimec_base::ED, *};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use frame_support::traits::fungible::Inspect;
use frame_support::traits::{
	fungible::{BalancedHold, MutateHold, Unbalanced},
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
use pallet_democracy::{AccountVote, Conviction, ReferendumInfo, Vote};
use pallet_vesting::VestingInfo;
use polimec_base_runtime::{
	Balances, Council, Democracy, GrowthTreasury, Preimage, RuntimeOrigin, TechnicalCommittee, Vesting,
};
use tests::defaults::*;
use xcm_emulator::get_account_id_from_seed;
generate_accounts!(PEPE, CARLOS,);

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
		assert_ok!(Balances::write_balance(&GrowthTreasury::account_id(), 1000 * PLMC));

		// 1. Create treasury proposal for 100 PLMC
		assert_ok!(GrowthTreasury::propose_spend(
			RuntimeOrigin::signed(alice.clone()),
			100 * PLMC,
			get_account_id_from_seed::<sr25519::Public>("Beneficiary").into()
		));
		assert_eq!(GrowthTreasury::proposal_count(), 1);

		// 2. Council will vote on the proposal
		let proposal = polimec_base_runtime::RuntimeCall::GrowthTreasury(pallet_treasury::Call::approve_proposal {
			proposal_id: 0,
		});
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
			polimec_base_runtime::GrowthTreasury::on_initialize(next_block_number);
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
