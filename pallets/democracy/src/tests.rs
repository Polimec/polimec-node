// Copyright (C) Parity Technologies (UK) Ltd.

// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// This library includes code from Substrate, which is licensed
// under both the GNU General Public License version 3 (GPLv3) and the
// Apache License 2.0. You may choose to redistribute and/or modify this
// code under either the terms of the GPLv3 or the Apache 2.0 License,
// whichever suits your needs.

//! The crate's tests.

use super::*;
use crate as pallet_democracy;
use frame_support::{
	assert_noop, assert_ok, derive_impl, ord_parameter_types, parameter_types,
	traits::{
		fungible::InspectFreeze, ConstU32, ConstU64, Contains, EqualPrivilegeOnly, OnInitialize, SortedMembers,
		StorePreimage,
	},
	weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use sp_runtime::{
	traits::{BadOrigin, BlakeTwo256, Hash, IdentityLookup},
	BuildStorage, Perbill,
};
mod cancellation;
mod decoders;
mod delegation;
mod external_proposing;
mod fast_tracking;
mod lock_voting;
mod metadata;
mod public_proposals;
mod scheduling;
mod voting;

const AYE: Vote = Vote { aye: true, conviction: Conviction::None };
const NAY: Vote = Vote { aye: false, conviction: Conviction::None };
const BIG_AYE: Vote = Vote { aye: true, conviction: Conviction::Locked1x };
const BIG_NAY: Vote = Vote { aye: false, conviction: Conviction::Locked1x };

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Preimage: pallet_preimage,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>, HoldReason, FreezeReason },
	}
);

// Test that a filtered call can be dispatched.
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(call: &RuntimeCall) -> bool {
		!matches!(call, &RuntimeCall::Balances(pallet_balances::Call::force_set_balance { .. }))
	}
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			Weight::from_parts(frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
		);
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = u64;
	type BaseCallFilter = BaseFilter;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
}
parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pallet_preimage::Config for Test {
	type Consideration = ();
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<u64>;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

impl pallet_scheduler::Config for Test {
	type MaxScheduledPerBlock = ConstU32<100>;
	type MaximumWeight = MaximumSchedulerWeight;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type Preimages = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type ScheduleOrigin = EnsureRoot<u64>;
	type WeightInfo = ();
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<10>;
	type MaxLocks = ConstU32<10>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
}
parameter_types! {
	pub static PreimageByteDeposit: u64 = 0;
	pub static InstantAllowed: bool = false;
}
ord_parameter_types! {
	pub const One: u64 = 1;
	pub const Two: u64 = 2;
	pub const Three: u64 = 3;
	pub const Four: u64 = 4;
	pub const Five: u64 = 5;
	pub const Six: u64 = 6;
}
pub struct OneToFive;
impl SortedMembers<u64> for OneToFive {
	fn sorted_members() -> Vec<u64> {
		vec![1, 2, 3, 4, 5]
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u64) {}
}

pub struct Electorate;
impl GetElectorate<BalanceOf<Test>> for Electorate {
	fn get_electorate() -> BalanceOf<Test> {
		Balances::total_issuance()
	}
}

impl Config for Test {
	type BlacklistOrigin = EnsureRoot<u64>;
	type CancelProposalOrigin = EnsureRoot<u64>;
	type CancellationOrigin = EnsureSignedBy<Four, u64>;
	type CooloffPeriod = ConstU64<2>;
	type Electorate = Electorate;
	type EnactmentPeriod = ConstU64<2>;
	type ExternalDefaultOrigin = EnsureSignedBy<One, u64>;
	type ExternalMajorityOrigin = EnsureSignedBy<Three, u64>;
	type ExternalOrigin = EnsureSignedBy<Two, u64>;
	type FastTrackOrigin = EnsureSignedBy<Five, u64>;
	type FastTrackVotingPeriod = ConstU64<2>;
	type Fungible = Balances;
	type InstantAllowed = InstantAllowed;
	type InstantOrigin = EnsureSignedBy<Six, u64>;
	type LaunchPeriod = ConstU64<2>;
	type MaxBlacklisted = ConstU32<5>;
	type MaxDeposits = ConstU32<1000>;
	type MaxProposals = ConstU32<100>;
	type MaxVotes = ConstU32<100>;
	type MinimumDeposit = ConstU64<1>;
	type PalletsOrigin = OriginCaller;
	type Preimages = Preimage;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Scheduler = Scheduler;
	type Slash = ();
	type SubmitOrigin = EnsureSigned<Self::AccountId>;
	type VetoOrigin = EnsureSignedBy<OneToFive, u64>;
	type VoteLockingPeriod = ConstU64<3>;
	type VotingPeriod = ConstU64<2>;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> { balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)] }
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_democracy::GenesisConfig::<Test>::default().assimilate_storage(&mut t).unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[test]
fn params_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Democracy::referendum_count(), 0);
		assert_eq!(Balances::free_balance(42), 0);
		assert_eq!(Balances::total_issuance(), 210);
	});
}

fn set_balance_proposal(value: u64) -> BoundedCallOf<Test> {
	let inner = pallet_balances::Call::force_set_balance { who: 42, new_free: value };
	let outer = RuntimeCall::Balances(inner);
	Preimage::bound(outer).unwrap()
}

#[test]
fn set_balance_proposal_is_correctly_filtered_out() {
	for i in 0..10 {
		let call = Preimage::realize(&set_balance_proposal(i)).unwrap().0;
		assert!(!<Test as frame_system::Config>::BaseCallFilter::contains(&call));
	}
}

fn propose_set_balance(who: u64, value: u64, delay: u64) -> DispatchResult {
	Democracy::propose(RuntimeOrigin::signed(who), set_balance_proposal(value), delay)
}

fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
	Democracy::begin_block(System::block_number());
}

fn fast_forward_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

fn begin_referendum() -> ReferendumIndex {
	System::set_block_number(0);
	assert_ok!(propose_set_balance(1, 2, 1));
	fast_forward_to(2);
	0
}

fn balance_freezable_of(who: u64) -> u64 {
	Balances::balance_freezable(&who)
}

fn balance_frozen_of(who: u64) -> u64 {
	Balances::balance_frozen(&FreezeReason::Vote.into(), &who)
}

fn aye(who: u64) -> AccountVote<u64> {
	AccountVote::Standard { vote: AYE, balance: balance_freezable_of(who) }
}

fn nay(who: u64) -> AccountVote<u64> {
	AccountVote::Standard { vote: NAY, balance: balance_freezable_of(who) }
}

fn big_aye(who: u64) -> AccountVote<u64> {
	AccountVote::Standard { vote: BIG_AYE, balance: balance_freezable_of(who) }
}

fn big_nay(who: u64) -> AccountVote<u64> {
	AccountVote::Standard { vote: BIG_NAY, balance: balance_freezable_of(who) }
}

fn tally(r: ReferendumIndex) -> Tally<u64> {
	Democracy::referendum_status(r).unwrap().tally
}

/// note a new preimage without registering.
fn note_preimage(who: u64) -> <Test as frame_system::Config>::Hash {
	use std::sync::atomic::{AtomicU8, Ordering};
	// note a new preimage on every function invoke.
	static COUNTER: AtomicU8 = AtomicU8::new(0);
	let data = vec![COUNTER.fetch_add(1, Ordering::Relaxed)];
	assert_ok!(Preimage::note_preimage(RuntimeOrigin::signed(who), data.clone()));
	let hash = BlakeTwo256::hash(&data);
	assert!(!Preimage::is_requested(&hash));
	hash
}
