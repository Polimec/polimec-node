use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, error::BadOrigin};

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

mod register {
	use super::*;

	#[test]
	fn must_be_root() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_noop!(MultiMintModule::register(Origin::signed(1), 1, [1; 8]), BadOrigin);

			assert_ok!(MultiMintModule::register(Origin::root(), 4, [1; 8]));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency { 0: [1; 8], 1: 4 })
			);
		})
	}

	#[test]
	fn cannot_change_native_currency() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_noop!(MultiMintModule::register(Origin::signed(1), 1, [1; 8]), BadOrigin);

			// You can't change the `NativeCurrency`, even if you are `root`
			assert_noop!(
				MultiMintModule::register(Origin::root(), 1, [0; 8]),
				Error::<Test>::NativeCurrencyCannotBeChanged
			);
		})
	}

	#[test]
	fn cannot_register_again() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_ok!(MultiMintModule::register(Origin::root(), 1, [1; 8]));

			// You can't register again the same currency
			assert_noop!(
				MultiMintModule::register(Origin::root(), 1, [1; 8]),
				Error::<Test>::CurrencyAlreadyExists
			);
		})
	}

	#[test]
	fn metadata_in_storage() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_ok!(MultiMintModule::register(Origin::root(), 42, [1; 8]));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let (issuer, trading_enabled) = MultiMintModule::currency_metadata([1; 8]).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(issuer, 42);
			// The trading is not enabled for the new registered currency
			assert_eq!(trading_enabled, false);
			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency { 0: [1; 8], 1: 42 })
			);
		})
	}
}

mod mint {
	use super::*;

	#[test]
	fn must_be_root() {
		new_test_ext().execute_with(|| {
			assert_ok!(MultiMintModule::register(Origin::root(), 7, [42; 8]));

			// Only the `root` account can call the `mint` function
			assert_noop!(MultiMintModule::mint(Origin::signed(1), 7, [42; 8], 100), BadOrigin);

			assert_ok!(MultiMintModule::mint(Origin::root(), 7, [42; 8], 100));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::MintedCurrency { 0: [42; 8], 1: 7, 2: 100 })
			);
		})
	}

	#[test]
	fn cannot_change_native_currency() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_noop!(MultiMintModule::register(Origin::signed(1), 1, [1; 8]), BadOrigin);

			// You can't change the `NativeCurrency`, even if you are `root`
			assert_noop!(
				MultiMintModule::register(Origin::root(), 1, [0; 8]),
				Error::<Test>::NativeCurrencyCannotBeChanged
			);
		})
	}
}

mod unlock_trading {
	use super::*;

	#[test]
	fn unlock_currency() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_ok!(MultiMintModule::register(Origin::root(), 42, [1; 8]));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let (issuer, trading_enabled) = MultiMintModule::currency_metadata([1; 8]).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(issuer, 42);
			// The trading is not enabled for the new registered currency
			assert_eq!(trading_enabled, false);
			// The `RegisteredCurrency` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency { 0: [1; 8], 1: 42 })
			);

			assert_ok!(MultiMintModule::unlock_trading(Origin::signed(42), [1; 8]));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let (issuer, trading_enabled) = MultiMintModule::currency_metadata([1; 8]).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(issuer, 42);
			// The trading is not enabled for the new registered currency
			assert_eq!(trading_enabled, true);
			// The `UnlockedTrading` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::UnlockedTrading { 0: [1; 8] })
			);
		})
	}
}

mod lock_trading {
	use super::*;

	#[test]
	fn lock_currency() {
		new_test_ext().execute_with(|| {
			// Only the `root` account can call the `register` function
			assert_ok!(MultiMintModule::register(Origin::root(), 42, [1; 8]));

			assert_ok!(MultiMintModule::unlock_trading(Origin::signed(42), [1; 8]));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let (issuer, trading_enabled) = MultiMintModule::currency_metadata([1; 8]).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(issuer, 42);
			// The trading is not enabled for the new registered currency
			assert_eq!(trading_enabled, true);
			// The `UnlockedTrading` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::UnlockedTrading { 0: [1; 8] })
			);

			assert_ok!(MultiMintModule::lock_trading(Origin::signed(42), [1; 8]));
			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let (issuer, trading_enabled) = MultiMintModule::currency_metadata([1; 8]).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(issuer, 42);
			// The trading is not enabled for the new registered currency
			assert_eq!(trading_enabled, false);
			// The `RegisteredCurrency` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::LockedTrading { 0: [1; 8] })
			);
		})
	}
}

mod transfer {
	use super::*;

	#[test]
	fn must_be_root() {
		new_test_ext().execute_with(|| {
			let issuer = 7;
			let _receiver = 14;
			let currency_id = [42; 8];
			let mint_amount = 100;
			let _transfer_amount = 50;
			assert_ok!(MultiMintModule::register(Origin::root(), issuer, currency_id));
			assert_ok!(MultiMintModule::mint(Origin::root(), issuer, currency_id, mint_amount));
			// TODO: Check https://github.com/open-web3-stack/open-runtime-module-library/blob/master/tokens/src/tests.rs
			// on how to test the transfer function from ORML

			// The event was deposited
			// assert_eq!(
			// 	last_event(),
			// 	Event::MultiMintModule(crate::Event::RegisteredCurrency { 0: [1; 8], 1: 4 })
			// );
		})
	}
}
