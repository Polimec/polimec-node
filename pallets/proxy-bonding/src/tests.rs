use crate::{mock::*, AccountIdOf, Error, ReleaseType};
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{Inspect, InspectHold, Mutate as FungibleMutate},
		fungibles::{Inspect as FungiblesInspect, Mutate},
		Get,
	},
};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn locked_outcome() {
	new_test_ext().execute_with(|| {
		// User requesting the proxy bond
		let user = 42u64;
		let treasury = <TestRuntime as crate::Config>::Treasury::get();
		let fee_recipient = <TestRuntime as crate::Config>::FeeRecipient::get();
		let ed: u64 = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		let derivation_path: u32 = 0;

		// Some pallet calculates he needs to bond 100 native tokens to access some functionality
		let bond_amount = 200 * NATIVE_UNIT;
		let expected_fee = 5 * MOCK_FEE_ASSET_UNIT;

		// User decides to pay the fee with asset 1337
		let fee_asset = mock_fee_asset_id();

		let hold_reason = MockRuntimeHoldReason::Reason;

		// The user doesn't need to have 100 native tokens on his account
		<Balances as FungibleMutate<u64>>::set_balance(&user, 0u64);
		// The treasury should have enough native tokens to pay for the account + ED + ED for the sub-account
		<Balances as FungibleMutate<u64>>::set_balance(&treasury, bond_amount + ed * 2);
		// The fee recipient should exist to receive the USDT
		<Balances as FungibleMutate<u64>>::set_balance(&fee_recipient, ed);
		// Asset creator needs to pay for metadata which defines decimals
		<Balances as FungibleMutate<u64>>::set_balance(&1, 100 * NATIVE_UNIT);

		// // The user needs to have 5 + ED of the fee asset on his account
		// <Assets as Create<u64>>::create(fee_asset, 1, true, 100).unwrap();
		// // Asset creator sets decimals of USDT to 6, same as underlying USD used by the price provider.
		// <Assets as MetadataMutate<u64>>::set(
		// 	fee_asset,
		// 	&1,
		// 	"Tether USD".to_string().into_bytes(),
		// 	"USDT".to_string().into_bytes(),
		// 	6,
		// )
		// .unwrap();
		// User should have enough fee tokens to pay the fee, but also some amount for the ED, which is defined above as `min_balance`
		<Assets as Mutate<u64>>::mint_into(fee_asset.clone(), &user, expected_fee + 100).unwrap();

		// The user requests the proxy bond
		ProxyBonding::bond_on_behalf_of(derivation_path, user, bond_amount, fee_asset.clone(), hold_reason).unwrap();

		// The user has locked 100 native tokens on sub-account 0
		let sub_account_0: AccountIdOf<TestRuntime> = RootId::get().into_sub_account_truncating(0);
		assert_eq!(<Balances as Inspect<u64>>::balance(&treasury), ed);
		assert_eq!(<Balances as InspectHold<u64>>::balance_on_hold(&hold_reason, &sub_account_0), bond_amount);
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset.clone(), &sub_account_0), expected_fee);
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset.clone(), &user), 100);

		// Mark the release type as Locked, which sends the fee to the fee_recipient, and the bond to the treasury after the inner block number is passed
		ProxyBonding::set_release_type(derivation_path, hold_reason, ReleaseType::Locked(10));

		assert_noop!(
			ProxyBonding::transfer_bonds_back_to_treasury(RuntimeOrigin::signed(1), derivation_path, hold_reason),
			Error::<TestRuntime>::TooEarlyToUnlock
		);
		assert_ok!(ProxyBonding::transfer_fees_to_recipient(
			RuntimeOrigin::signed(1),
			derivation_path,
			hold_reason,
			fee_asset.clone()
		));
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset, &fee_recipient), expected_fee);

		System::set_block_number(10);

		assert_ok!(ProxyBonding::transfer_bonds_back_to_treasury(
			RuntimeOrigin::signed(1),
			derivation_path,
			hold_reason
		));
		assert_eq!(<Balances as Inspect<u64>>::balance(&treasury), ed + bond_amount);
	});
}

#[test]
fn refunded_outcome() {
	new_test_ext().execute_with(|| {
		// User requesting the proxy bond
		let user = 42u64;
		let treasury = <TestRuntime as crate::Config>::Treasury::get();
		let fee_recipient = <TestRuntime as crate::Config>::FeeRecipient::get();
		let ed: u64 = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
		let derivation_path: u32 = 0;

		// Some pallet calculates he needs to bond 100 native tokens to access some functionality
		let bond_amount = 200 * NATIVE_UNIT;
		let expected_fee = 5 * MOCK_FEE_ASSET_UNIT;

		let fee_asset = mock_fee_asset_id();

		let hold_reason = MockRuntimeHoldReason::Reason;

		// The user doesn't need to have 100 native tokens on his account
		<Balances as FungibleMutate<u64>>::set_balance(&user, 0u64);
		// The treasury should have enough native tokens to pay for the account + ED + ED for the sub-account
		<Balances as FungibleMutate<u64>>::set_balance(&treasury, bond_amount + ed * 2);
		// The fee recipient should exist to receive the USDT
		<Balances as FungibleMutate<u64>>::set_balance(&fee_recipient, ed);

		// User should have enough fee tokens to pay the fee, but also some amount for the ED, which is defined above as `min_balance`
		<Assets as Mutate<u64>>::mint_into(fee_asset.clone(), &user, expected_fee + 100).unwrap();

		// The user requests the proxy bond
		ProxyBonding::bond_on_behalf_of(derivation_path, user, bond_amount, fee_asset.clone(), hold_reason).unwrap();

		// The user has locked 100 native tokens on sub-account 0
		let sub_account_0: AccountIdOf<TestRuntime> = RootId::get().into_sub_account_truncating(0);
		assert_eq!(<Balances as Inspect<u64>>::balance(&treasury), ed);
		assert_eq!(<Balances as InspectHold<u64>>::balance_on_hold(&hold_reason, &sub_account_0), bond_amount);
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset.clone(), &sub_account_0), expected_fee);
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset.clone(), &user), 100);

		// Mark the release type as Refunded, which leaves the fee to subsequent `refund_fee` calls, and allows to send the bond immediately the treasury.
		ProxyBonding::set_release_type(derivation_path, hold_reason, ReleaseType::Refunded);

		assert_ok!(ProxyBonding::transfer_bonds_back_to_treasury(
			RuntimeOrigin::signed(1),
			derivation_path,
			hold_reason
		),);
		assert_eq!(<Balances as Inspect<u64>>::balance(&treasury), ed + bond_amount);
		assert_noop!(
			ProxyBonding::transfer_fees_to_recipient(
				RuntimeOrigin::signed(1),
				derivation_path,
				hold_reason,
				fee_asset.clone()
			),
			Error::<TestRuntime>::FeeToRecipientDisallowed
		);

		assert_ok!(ProxyBonding::refund_fee(derivation_path, &user, bond_amount, fee_asset.clone()));
		assert_eq!(<Assets as FungiblesInspect<u64>>::balance(fee_asset, &user), 100 + expected_fee);
	});
}
