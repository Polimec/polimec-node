use crate::{mock::*, CurrencyMetadata, Error, TradingStatus};
use frame_support::{assert_noop, assert_ok, traits::ConstU32, BoundedVec};

pub fn last_event() -> Event {
	frame_system::Pallet::<Test>::events().pop().expect("Event expected").event
}

pub fn build_currency_metadata() -> CurrencyMetadata<BoundedVec<u8, ConstU32<50>>> {
	CurrencyMetadata {
		name: b"My Token".to_vec().try_into().unwrap(),
		symbol: b"TKN_____".to_vec().try_into().unwrap(),
		decimals: 12,
	}
}

const ALICE: AccountId = 7;
const BOB: AccountId = 8;

const PLMC: [u8; 8] = [0; 8];
const TKN: [u8; 8] = [42; 8];

mod register {
	use super::*;

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency(TKN, ALICE))
			);
		})
	}

	#[test]
	fn cannot_change_native_currency() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();

			// You can't change the `NativeCurrency`, even if you are the `issuer`
			assert_noop!(
				MultiMintModule::register(Origin::signed(ALICE), PLMC, currency_metadata),
				Error::<Test>::NativeCurrencyCannotBeChanged
			);
		})
	}

	#[test]
	fn cannot_register_again() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(
				Origin::signed(ALICE),
				TKN,
				currency_metadata.clone()
			));

			// You can't register again the same currency
			assert_noop!(
				MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata),
				Error::<Test>::CurrencyAlreadyExists
			);
		})
	}

	#[test]
	fn metadata_in_storage() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let currency_info = MultiMintModule::currencies(TKN).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(currency_info.issuer, ALICE);
			// The trading is not enabled for the new registered currency
			assert!(currency_info.trading_enabled == TradingStatus::Disabled);
			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency(TKN, ALICE))
			);
		})
	}
}

mod mint {
	use super::*;

	#[test]
	fn must_be_root() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata));

			assert_ok!(MultiMintModule::mint(Origin::signed(ALICE), ALICE, TKN, 100));

			// The event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::MintedCurrency(TKN, 7, 100))
			);
		})
	}

	#[test]
	fn cannot_change_native_currency() {
		new_test_ext().execute_with(|| {
			// You can't change the `NativeCurrency`
			assert_noop!(
				MultiMintModule::mint(Origin::signed(ALICE), ALICE, PLMC, 100),
				Error::<Test>::NativeCurrencyCannotBeChanged
			);
		})
	}
}

mod unlock_trading {
	use super::*;
	use crate::TradingStatus;

	#[test]
	fn cannot_unlock_unregistered_currency() {
		new_test_ext().execute_with(|| {
			// The `currency` indexed by the id `TKN` don't exists
			assert_noop!(
				MultiMintModule::unlock_trading(Origin::signed(ALICE), TKN),
				Error::<Test>::CurrencyNotFound
			);
		})
	}

	#[test]
	fn unlock_currency() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let currency_info = MultiMintModule::currencies(TKN).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(currency_info.issuer, ALICE);
			// The trading is not enabled for the new registered currency
			assert!(currency_info.trading_enabled == TradingStatus::Disabled);
			// The `RegisteredCurrency` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::RegisteredCurrency(TKN, ALICE))
			);

			assert_ok!(MultiMintModule::unlock_trading(Origin::signed(ALICE), TKN));
			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let currency_info = MultiMintModule::currencies(TKN).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(currency_info.issuer, ALICE);
			// Now the currency trading should be enabled
			assert!(currency_info.trading_enabled == TradingStatus::Enabled);
			// The `ChangedTrading` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::ChangedTrading(TKN, TradingStatus::Enabled))
			);
		})
	}
}

mod lock_trading {
	use super::*;
	use crate::TradingStatus;

	#[test]
	fn cannot_lock_unregistered_currency() {
		new_test_ext().execute_with(|| {
			assert_noop!(
				MultiMintModule::unlock_trading(Origin::signed(ALICE), PLMC),
				Error::<Test>::CurrencyNotFound
			);
		})
	}

	#[test]
	fn lock_currency() {
		new_test_ext().execute_with(|| {
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(Origin::signed(ALICE), TKN, currency_metadata));

			assert_ok!(MultiMintModule::unlock_trading(Origin::signed(ALICE), TKN));

			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let currency_info = MultiMintModule::currencies(TKN).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(currency_info.issuer, ALICE);

			assert!(currency_info.trading_enabled == TradingStatus::Enabled);
			// The `UnlockedTrading` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::ChangedTrading(TKN, TradingStatus::Enabled))
			);

			assert_ok!(MultiMintModule::lock_trading(Origin::signed(ALICE), TKN));
			// Here `currency_metadata` is the StorageMap in `MultiMintModule`
			let currency_info = MultiMintModule::currencies(TKN).unwrap();
			// The issuer is the account specified in the register call.
			assert_eq!(currency_info.issuer, ALICE);
			// The trading is not enabled for the new registered currency
			assert!(currency_info.trading_enabled == TradingStatus::Disabled);
			// The `RegisteredCurrency` event was deposited
			assert_eq!(
				last_event(),
				Event::MultiMintModule(crate::Event::ChangedTrading(TKN, TradingStatus::Disabled))
			);
		})
	}
}

mod transfer {
	use super::*;

	#[test]
	fn it_works() {
		new_test_ext().execute_with(|| {
			let currency_id = TKN;
			let currency_metadata = build_currency_metadata();
			assert_ok!(MultiMintModule::register(
				Origin::signed(ALICE),
				currency_id,
				currency_metadata
			));

			assert_ok!(MultiMintModule::mint(Origin::signed(ALICE), ALICE, currency_id, 100));
			// TODO: Check https://github.com/open-web3-stack/open-runtime-module-library/blob/master/tokens/src/tests.rs
			// on how to test the transfer function from ORML

			assert_ok!(MultiMintModule::transfer(Origin::signed(ALICE), currency_id, BOB, 50));

			// TODO: Check ALICE and BOB's Balance
			// TODO: ALICE = 50
			// TODO: BOB = 50
		})
	}
}
