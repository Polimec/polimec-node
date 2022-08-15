use sp_std::prelude::*;

use frame_benchmarking::account;
use frame_support::storage::{StorageDoubleMap, StorageMap, StorageValue};
use frame_system::RawOrigin;

use orml_benchmarking::runtime_benchmarks;

use crate::{
	AccountId, Amount, Balance, Call, CouncilRegistrationFee, CurrencyId, IssuerCouncil, MultiStake, Origin, Runtime,
};

use super::utils::{fill_members, fill_proposals, set_balance, MAX_MEMBERS, MAX_PROPOSALS, NATIVE_CURRENCY_ID, SEED};

use issuer_council::{BondingConfig, CurrencyConfig, PayoutsEnabled, ProposalVotes, SlashReason};
use sp_runtime::Permill;

runtime_benchmarks! {
	{ Runtime, issuer_council }

	_ {}

	apply_for_seat {
		// important: we still need one empty slot for our new proposal!
		let p in 1..(MAX_PROPOSALS - 1) => ();

		fill_proposals(p as usize);

		let total_issuance: Amount = 100_000_000_000;
		let balance: Balance = CouncilRegistrationFee::get() + 100_000_000_000;
		let new_currency_id: CurrencyId = [0_u8, 0, 0, 0, 0, 0, 0, 0xFF];
		let applicant: AccountId = account("from", 0, SEED);
		set_balance(NATIVE_CURRENCY_ID, &applicant, balance);
	}: _(
		RawOrigin::Signed(applicant),
		applicant.clone(),
		total_issuance,
		new_currency_id,
		Permill::from_percent(10),
		()
	)
	verify {
		assert_eq!(IssuerCouncil::proposals().len(), (p + 1) as usize);
	}

	exit_council {
		let m in 1..MAX_MEMBERS => ();
		let p = 1;

		let member = fill_members(m as usize).pop().unwrap().account_id;
		fill_proposals(p as usize);

		assert_eq!(IssuerCouncil::members().len(), m as usize);
	}: _(RawOrigin::Signed(member))
	verify {
		assert_eq!(IssuerCouncil::members().len(), (m - 1) as usize);
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
	}

	expel_member {
		let m in 1..MAX_MEMBERS => ();
		let p = 1;
		let member = fill_members(m as usize).pop().unwrap().account_id;
		fill_proposals(p as usize);

		assert_eq!(IssuerCouncil::members().len(), m as usize);

	}: _(Origin::root(), member)
	verify {
		assert_eq!(IssuerCouncil::members().len(), (m - 1) as usize);
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
	}

	admit_new_member {
		let m in 1..MAX_MEMBERS => ();
		let p = 1;

		fill_members(m as usize);
		fill_proposals(p as usize);

		let applicant: AccountId = account("applicant", 0, SEED);
		let balance: Balance = CouncilRegistrationFee::get() + 100_000_000_000;
		let total_issuance: Amount = 100_000_000_000;
		let new_currency_id: CurrencyId = [0_u8, 0, 0, 0, 0, 0, 0, 0xFF];

		set_balance(NATIVE_CURRENCY_ID, &applicant, balance);
		assert_eq!(IssuerCouncil::members().len(), m as usize);
	}: _(
		Origin::root(),
		applicant.clone(),
		applicant.clone(),
		total_issuance,
		new_currency_id,
		Permill::from_percent(10)
	)
	verify {
		assert_eq!(IssuerCouncil::members().len(), (m + 1) as usize);
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
	}

	submit_proposal {
		let m in 1..MAX_MEMBERS => ();
		// important: we still need one empty slot for our new proposal!
		let p in 1..(MAX_PROPOSALS - 1) => ();

		let sender = fill_members(m as usize).pop().unwrap().account_id;
		fill_proposals(p as usize);

		let call = Box::new(Call::IssuerCouncil(
			issuer_council::Call::<Runtime>::expel_member(
				sender.clone()
			)
		));
	}: _(RawOrigin::Signed(sender), call, ())
	verify {
		assert_eq!(IssuerCouncil::members().len(), m as usize);
		assert_eq!(IssuerCouncil::proposals().len(), (p + 1) as usize);
	}

	slash_keep_member {
		let m in 1..MAX_MEMBERS => ();
		let p = 1;
		let member = fill_members(m as usize).pop().unwrap().account_id;
		fill_proposals(p as usize);

	}: slash(Origin::root(), member, SlashReason::FaultyBlock)
	verify {
		assert_eq!(IssuerCouncil::members().len(), m as usize);
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
	}

	slash_drop_member {
		let m in 1..MAX_MEMBERS => ();
		let p = 1;
		let mut members = fill_members(m as usize);
		fill_proposals(p as usize);

		// set last members points to 1
		if let Some(l) = members.last_mut() {l.points = 1};
		let member = members.last().unwrap().account_id.clone();
		issuer_council::Members::<Runtime>::set(members);

	}: slash(Origin::root(), member, SlashReason::FaultyBlock)
	verify {
		assert_eq!(IssuerCouncil::members().len(), (m - 1) as usize);
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
	}

	vote {
		let m = 10;
		let p = 1;
		let member = fill_members(m).pop().unwrap();
		let proposal = fill_proposals(p).pop().unwrap();

		let holder: AccountId = account("holder", 0, SEED);
		let balance: Balance = 100_000_000;
		set_balance(member.currency_id, &holder, balance);
		MultiStake::bond(RawOrigin::Signed(holder.clone()).into(), holder.clone(), member.currency_id, balance)?;
		CurrencyConfig::<Runtime>::insert(member.currency_id, BondingConfig {payout: PayoutsEnabled::No, vote: true});
	}: _(Origin::signed(holder), proposal.proposal_hash, member.currency_id, true)
	verify {
		assert_eq!(IssuerCouncil::proposals().len(), p as usize);
		assert_eq!(ProposalVotes::<Runtime>::get(proposal.proposal_hash, member.currency_id).yes_votes, balance);
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
	fn test_apply_for_seat() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_apply_for_seat());
		});
	}

	#[test]
	fn test_exit_council() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_exit_council());
		});
	}
	#[test]
	fn test_expel_member() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_expel_member());
		});
	}
	#[test]
	fn test_admit_new_member() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_admit_new_member());
		});
	}
	#[test]
	fn test_submit_proposal() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_submit_proposal());
		});
	}

	#[test]
	fn test_slash_keep_member() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_slash_keep_member());
		});
	}
	#[test]
	fn test_slash_drop_member() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_slash_drop_member());
		});
	}
	#[test]
	fn test_vote() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_vote());
		});
	}
}
