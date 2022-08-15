use crate::{
	AccountId, Amount, Balance, Call, CouncilRegistrationFee, CurrencyId, GetNativeCurrencyId, IssuerCouncil,
	MaxProposals, Origin, PoliBalances, Runtime, TechnicalMaxMembers, ValidatorId,
};

use frame_benchmarking::account;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_runtime::traits::SaturatedConversion;
use sp_runtime::traits::StaticLookup;
use sp_runtime::Permill;
use sp_std::vec::Vec;

use issuer_council::CouncilMember;

pub const NATIVE_CURRENCY_ID: CurrencyId = GetNativeCurrencyId::get();
pub const MAX_MEMBERS: u32 = TechnicalMaxMembers::get();
pub const MAX_PROPOSALS: u32 = MaxProposals::get();
pub const SEED: u32 = 0;

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	let _ = <PoliBalances as MultiCurrencyExtended<_>>::update_balance(currency_id, &who, balance.saturated_into());
	assert_eq!(
		<PoliBalances as MultiCurrency<_>>::free_balance(currency_id, who),
		balance
	);
}

pub fn add_member(account_id: AccountId, validator_id: ValidatorId, total_issuance: Amount, currency_id: CurrencyId) {
	let result = IssuerCouncil::admit_new_member(
		Origin::root(),
		account_id,
		validator_id,
		total_issuance,
		currency_id,
		Permill::from_percent(10),
	);
	assert!(result.is_ok(), "Should have been Ok! {:?}", result);
}

/// fills the council until `member_count` seats are occupied. Will do nothing
/// if members contains already enough entries.
pub fn fill_members(member_count: usize) -> Vec<CouncilMember<Runtime>> {
	let current_members_count = IssuerCouncil::members().len();
	let members = IssuerCouncil::members();
	if members.len() > member_count {
		return members;
	}

	for m_i in current_members_count..member_count {
		let member: AccountId = account("member", m_i as u32, SEED);
		let balance: Balance = CouncilRegistrationFee::get() + 100_000_000_000;
		let total_issuance: Amount = 100_000_000_000;

		let mut new_currency_id = [0xFF_u8; 8];
		let m_i_1 = m_i + 1;
		let y = m_i_1.to_be_bytes();
		let length = y.len().min(new_currency_id.len());
		new_currency_id[..length].copy_from_slice(&y[..length]);

		set_balance(NATIVE_CURRENCY_ID, &member, balance);
		add_member(member.clone(), member.clone(), total_issuance, new_currency_id);
	}
	let members = IssuerCouncil::members();
	assert_eq!(members.len(), member_count);
	members
}

/// fill_proposals adds proposals until `proposal_count` proposals are listed.
/// Will do nothing if proposals contains already enough entries.
pub fn fill_proposals(proposal_count: usize) -> Vec<issuer_council::CouncilProposal<Runtime>> {
	let proposals = IssuerCouncil::proposals();
	let current_proposal_count = proposals.len();
	if proposals.len() > current_proposal_count {
		return proposals;
	}

	for i in current_proposal_count..proposal_count {
		let remark = i.to_be_bytes().to_vec();
		let call = Call::System(frame_system::Call::<Runtime>::remark(remark));
		let result = IssuerCouncil::add_proposal(call, ());
		assert!(result.is_ok(), "Should be Ok: {:?}", result);
	}
	let proposals = IssuerCouncil::proposals();
	assert_eq!(proposals.len(), proposal_count);

	proposals
}

/// Get lookup of an account, required for staking.
pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Config>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Config>::Lookup::unlookup(who)
}

/// Grab a funded user.
pub fn create_funded_user(string: &'static str, currency_id: CurrencyId, amount: Balance, n: u32) -> AccountId {
	let user = account(string, n, SEED);
	set_balance(currency_id, &user, amount);
	user
}
