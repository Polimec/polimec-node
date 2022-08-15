#![cfg_attr(not(feature = "std"), no_std)]

/// Expose actively bonded balance
pub trait BondedAmount<AccountId, CurrencyId, Balance> {
	fn get_active(stash: &AccountId, currency_id: &CurrencyId) -> Option<Balance>;
}

/// Expose a user vote
pub trait BondedVote<AccountId, CurrencyId, Balance> {
	fn update_amount(
		controller: &AccountId,
		currency_id: &CurrencyId,
		amount_new: &Balance,
		amount_old: &Balance,
	) -> u32;
}

pub trait PayoutPool<CurrencyId, Balance> {
	fn get_amount(currency_id: &CurrencyId) -> Balance;
	fn set_amount(currency_id: &CurrencyId, amount: &Balance);
	fn set_rate(currency_id: &CurrencyId, rate: &sp_runtime::Permill);
}
