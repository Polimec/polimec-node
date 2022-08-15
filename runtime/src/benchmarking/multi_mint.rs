use sp_std::prelude::*;

pub use frame_benchmarking::{account, benchmarks};
use frame_support::storage::StorageMap;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;

use super::utils::SEED;
use crate::{AccountId, Balance, CurrencyId, Origin, PoliBalances, PreCurrencyMint, Runtime};
use multi_mint::CurrencyMetadata;
use orml_traits::MultiCurrency;

const CURRENCY_ID: CurrencyId = [0_u8, 0, 0, 0, 0, 0, 0, 0xFF];
const AMOUNT: Balance = 100_000_000_000;

runtime_benchmarks! {
	{ Runtime, multi_mint }

	_ {
	}

	register {
		let issuer: AccountId = account("issuer", 0, SEED);
	}: _(RawOrigin::Root, issuer, CURRENCY_ID)
	verify {
		assert!(CurrencyMetadata::<Runtime>::contains_key(CURRENCY_ID));
	}

	mint {
		let issuer: AccountId = account("issuer", 0, SEED);
		PreCurrencyMint::register(Origin::root(), issuer.clone(), CURRENCY_ID)?;
	}: _(RawOrigin::Root, issuer.clone(), CURRENCY_ID, AMOUNT as i128)
	verify {
		assert_eq!(<PoliBalances as MultiCurrency<_>>::free_balance(CURRENCY_ID, &issuer), AMOUNT);
	}

	unlock_trading {
		let issuer: AccountId = account("issuer", 0, SEED);
		PreCurrencyMint::register(Origin::root(), issuer.clone(), CURRENCY_ID)?;
	}: _(RawOrigin::Signed(issuer.clone()), CURRENCY_ID)
	verify {
		assert_eq!(CurrencyMetadata::<Runtime>::get(CURRENCY_ID), Some((issuer, true)));
	}

	lock_trading {
		let issuer: AccountId = account("issuer", 0, SEED);
		PreCurrencyMint::register(Origin::root(), issuer.clone(), CURRENCY_ID)?;
		PreCurrencyMint::unlock_trading(Origin::signed(issuer.clone()), CURRENCY_ID)?;
		let (_, locked_before) = CurrencyMetadata::<Runtime>::get(CURRENCY_ID).unwrap();
	}: _(RawOrigin::Signed(issuer.clone()), CURRENCY_ID)
	verify {
		assert!(locked_before);
		assert_eq!(CurrencyMetadata::<Runtime>::get(CURRENCY_ID), Some((issuer, false)));
	}

	transfer {
		let from: AccountId = account("from", 0, SEED);
		let to: AccountId = account("to", 0, SEED);
		PreCurrencyMint::register(Origin::root(), from.clone(), CURRENCY_ID)?;
		PreCurrencyMint::do_mint(from.clone(), &CURRENCY_ID, AMOUNT as i128)?;
		PreCurrencyMint::unlock_trading(Origin::signed(from.clone()), CURRENCY_ID)?;
		let balance_before = <PoliBalances as MultiCurrency<_>>::free_balance(CURRENCY_ID, &to);
	}: _(RawOrigin::Signed(from.clone()), CURRENCY_ID, to.clone(), AMOUNT)
	verify {
		let balance_after = <PoliBalances as MultiCurrency<_>>::free_balance(CURRENCY_ID, &to);
		assert!(balance_before < balance_after);
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
	fn register() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_register());
		});
	}
	#[test]
	fn mint() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_mint());
		});
	}
	#[test]
	fn unlock_trading() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_unlock_trading());
		});
	}
	#[test]
	fn lock_trading() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_lock_trading());
		});
	}
	#[test]
	fn transfer() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_transfer());
		});
	}
}
