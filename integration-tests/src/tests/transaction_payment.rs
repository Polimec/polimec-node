use crate::{
	constants, constants::PricesBuilder, polimec, PolimecAccountId, PolimecBalances, PolimecCall, PolimecForeignAssets,
	PolimecNet, PolimecOrigin, PolimecRuntime, PolimecSystem,
};
use frame_support::{
	dispatch::GetDispatchInfo,
	pallet_prelude::TransactionSource,
	traits::{
		fungible::Mutate as FungibleMutate,
		fungibles::{self, Mutate as FungiblesMutate},
	},
};
use macros::generate_accounts;
use pallet_asset_tx_payment::Val;
use pallet_funding::PriceProviderOf;
use pallet_transaction_payment::FeeDetails;
use parity_scale_codec::Encode;
use polimec_common::{assets::AcceptedFundingAsset, ProvideAssetPrice, PLMC_DECIMALS, USD_DECIMALS};
use polimec_runtime::Header;
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_core::H256;
use sp_runtime::{
	traits::{Dispatchable, Header as HeaderT, TransactionExtension, TxBaseImplication},
	DigestItem,
};
use xcm_emulator::TestExt;
generate_accounts!(ALICE, AUTHOR);
use frame_support::traits::fungible::Inspect;
use xcm::{
	v3::{Junction::*, Junctions::*, MultiLocation},
	v5::Location,
};

// Setup code inspired by pallet-authorship tests
fn seal_header(mut header: Header, aura_index: u64) -> Header {
	{
		let digest = header.digest_mut();
		digest.logs.push(DigestItem::PreRuntime(sp_consensus_aura::AURA_ENGINE_ID, aura_index.encode()));
		digest.logs.push(DigestItem::Seal(sp_consensus_aura::AURA_ENGINE_ID, aura_index.encode()));
	}

	header
}

fn create_header(number: u32, parent_hash: H256, state_root: H256) -> Header {
	Header::new(number, Default::default(), state_root, parent_hash, Default::default())
}
// Make sure to run this inside externalities environment. Can only be called one time per test.
fn set_author(aura_index: u64) {
	let mut header = seal_header(create_header(1, Default::default(), [1; 32].into()), aura_index);

	header.digest_mut().pop(); // pop the seal off.
	PolimecSystem::reset_events();
	PolimecSystem::initialize(&1, &Default::default(), header.digest());
}

#[test]
fn fee_paid_with_foreign_assets() {
	polimec::set_prices(
		PricesBuilder::new().usdt(FixedU128::from_float(1.0f64)).plmc(FixedU128::from_float(0.5f64)).build(),
	);

	PolimecNet::execute_with(|| {
		let alice: PolimecAccountId = ALICE.into();

		let (block_author, _) = &constants::collators::invulnerables()[1];
		// Block author's aura index is 1
		set_author(1u64);
		assert_eq!(polimec_runtime::Authorship::author(), Some(block_author.clone()));

		let usdt_id = AcceptedFundingAsset::USDT.id();
		let usdt_multilocation =
			MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)) };
		let usdt_decimals =
			<PolimecForeignAssets as fungibles::metadata::Inspect<PolimecAccountId>>::decimals(usdt_id.clone());
		let usdt_unit = 10u128.pow(usdt_decimals as u32);
		let plmc_decimals = PLMC_DECIMALS;
		let plmc_unit = 10u128.pow(plmc_decimals as u32);

		PolimecBalances::set_balance(&alice, 0u128);
		PolimecForeignAssets::set_balance(usdt_id.clone(), &alice, 100 * usdt_unit);
		// Fees are usually very small, so we need to give the treasury an ED.
		PolimecForeignAssets::set_balance(
			usdt_id.clone(),
			&polimec_runtime::BlockchainOperationTreasury::get(),
			100 * usdt_unit,
		);
		// Block author doesn't need to have any balance, as the tip is bigger than ED.

		let paid_call = PolimecCall::System(frame_system::Call::remark { remark: vec![69, 69] });
		let paid_call_len = paid_call.encode().len();
		type TxPaymentExtension = pallet_asset_tx_payment::ChargeAssetTxPayment<PolimecRuntime>;

		// Tips are always defined in the native asset, and then converted to the fee asset if the second field is `Some`.
		// Here a user wants to tip 10 PLMC in USDT.
		let tip = 10 * plmc_unit;
		let signed_extension =
			pallet_asset_tx_payment::ChargeAssetTxPayment::<PolimecRuntime>::from(tip, Some(usdt_multilocation));

		let dispatch_info = paid_call.get_dispatch_info();
		let FeeDetails { inclusion_fee, tip } = polimec_runtime::TransactionPayment::compute_fee_details(
			paid_call_len as u32,
			&dispatch_info,
			10u128 * plmc_unit,
		);
		let expected_plmc_fee = inclusion_fee.expect("call should charge a fee").inclusion_fee();
		let expected_plmc_tip = tip;

		let plmc_price_decimal_aware =
			<PriceProviderOf<PolimecRuntime>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, plmc_decimals)
				.expect("Price irretrievable");

		// USDT should be configured with the same decimals as our underlying USD unit, and we set the price to 1USD at the beginning of this test.
		let expected_usd_fee = plmc_price_decimal_aware.saturating_mul_int(expected_plmc_fee);
		let expected_usd_tip = plmc_price_decimal_aware.saturating_mul_int(expected_plmc_tip);

		let prev_alice_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), alice.clone());
		let prev_alice_plmc_balance = PolimecBalances::balance(&alice);
		let prev_blockchain_operation_treasury_usdt_balance =
			PolimecForeignAssets::balance(usdt_id.clone(), polimec_runtime::BlockchainOperationTreasury::get());
		let prev_blockchain_operation_treasury_plmc_balance =
			PolimecBalances::balance(&polimec_runtime::BlockchainOperationTreasury::get());
		let prev_block_author_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), block_author.clone());
		let prev_block_author_plmc_balance = PolimecBalances::balance(&block_author.clone());

		// Executes the `pre_dispatch` check for the transaction using the signed extension.
		// Dispatches the `paid_call` signed by `alice` and ensures it executes successfully.
		// Runs the `post_dispatch` logic to verify correct post-transaction handling,
		// including fee calculation and cleanup using the `TxPaymentExtension`.
		let (_, val, _) = signed_extension
			.validate(
				polimec_runtime::RuntimeOrigin::signed(alice.clone()),
				&paid_call,
				&dispatch_info,
				paid_call_len,
				signed_extension.implicit().unwrap(),
				&TxBaseImplication(()),
				TransactionSource::Local,
			)
			.expect("tx extension validation failed");

		let pre = signed_extension
			.prepare(
				val,
				&polimec_runtime::RuntimeOrigin::signed(alice.clone()),
				&paid_call,
				&dispatch_info,
				paid_call_len,
			)
			.expect("shouldn't fail here; qed");
		let mut post_info =
			paid_call.clone().dispatch(PolimecOrigin::signed(alice.clone())).expect("call dispatch failed");

		TxPaymentExtension::post_dispatch(pre, &dispatch_info, &mut post_info, paid_call_len, &Ok(())).unwrap();

		let post_alice_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), alice.clone());
		let post_alice_plmc_balance = PolimecBalances::balance(&alice);
		let post_blockchain_operation_treasury_usdt_balance =
			PolimecForeignAssets::balance(usdt_id.clone(), polimec_runtime::BlockchainOperationTreasury::get());
		let post_blockchain_operation_treasury_plmc_balance =
			PolimecBalances::balance(&polimec_runtime::BlockchainOperationTreasury::get());
		let post_block_author_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), block_author.clone());
		let post_block_author_plmc_balance = PolimecBalances::balance(&block_author.clone());

		assert_eq!(prev_alice_usdt_balance - post_alice_usdt_balance, expected_usd_fee + expected_usd_tip);
		assert_eq!(post_alice_plmc_balance, prev_alice_plmc_balance);
		assert_eq!(
			post_blockchain_operation_treasury_usdt_balance - prev_blockchain_operation_treasury_usdt_balance,
			expected_usd_fee
		);
		assert_eq!(post_blockchain_operation_treasury_plmc_balance, prev_blockchain_operation_treasury_plmc_balance);
		assert_eq!(post_block_author_usdt_balance - prev_block_author_usdt_balance, expected_usd_tip);
		assert_eq!(post_block_author_plmc_balance, prev_block_author_plmc_balance);

		// * Now we check if the same behavior but using PLMC as a fee asset, produces the same results. (2plmc=1usdt) *
		PolimecBalances::set_balance(&alice, 100 * plmc_unit);
		PolimecForeignAssets::set_balance(usdt_id.clone(), &alice, 0u128);
		// Fees are usually very small, so we need to give the treasury an ED.
		PolimecBalances::set_balance(&polimec_runtime::BlockchainOperationTreasury::get(), 100 * plmc_unit);
		// Block author doesn't need to have any balance, as the tip is bigger than ED.

		// Now we set the fee asset to None, so the fee is paid with PLMC
		let tip = 10 * plmc_unit;
		let signed_extension = pallet_asset_tx_payment::ChargeAssetTxPayment::<PolimecRuntime>::from(tip, None);

		let prev_alice_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), alice.clone());
		let prev_alice_plmc_balance = PolimecBalances::balance(&alice);
		let prev_blockchain_operation_treasury_usdt_balance =
			PolimecForeignAssets::balance(usdt_id.clone(), polimec_runtime::BlockchainOperationTreasury::get());
		let prev_blockchain_operation_treasury_plmc_balance =
			PolimecBalances::balance(&polimec_runtime::BlockchainOperationTreasury::get());
		let prev_block_author_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), block_author.clone());
		let prev_block_author_plmc_balance = PolimecBalances::balance(&block_author.clone());

		let (_, val, _) = signed_extension
			.validate(
				polimec_runtime::RuntimeOrigin::signed(alice.clone()),
				&paid_call,
				&dispatch_info,
				paid_call_len,
				signed_extension.implicit().unwrap(),
				&TxBaseImplication(()),
				TransactionSource::Local,
			)
			.expect("tx extension validation failed");

		let pre = signed_extension
			.prepare(
				val,
				&polimec_runtime::RuntimeOrigin::signed(alice.clone()),
				&paid_call,
				&dispatch_info,
				paid_call_len,
			)
			.unwrap();
		let mut post_info = paid_call.dispatch(PolimecOrigin::signed(alice.clone())).expect("call dispatch failed");

		TxPaymentExtension::post_dispatch(pre, &dispatch_info, &mut post_info, paid_call_len, &Ok(())).unwrap();

		let post_alice_usdt_balance = PolimecForeignAssets::balance(usdt_id.clone(), alice.clone());
		let post_alice_plmc_balance = PolimecBalances::balance(&alice);
		let post_blockchain_operation_treasury_usdt_balance =
			PolimecForeignAssets::balance(usdt_id.clone(), polimec_runtime::BlockchainOperationTreasury::get());
		let post_blockchain_operation_treasury_plmc_balance =
			PolimecBalances::balance(&polimec_runtime::BlockchainOperationTreasury::get());
		let post_block_author_usdt_balance = PolimecForeignAssets::balance(usdt_id, block_author.clone());
		let post_block_author_plmc_balance = PolimecBalances::balance(&block_author.clone());

		assert_eq!(post_alice_usdt_balance, prev_alice_usdt_balance);
		assert_eq!(prev_alice_plmc_balance - post_alice_plmc_balance, expected_plmc_fee + expected_plmc_tip);
		assert_eq!(post_blockchain_operation_treasury_usdt_balance, prev_blockchain_operation_treasury_usdt_balance);
		assert_eq!(
			post_blockchain_operation_treasury_plmc_balance - prev_blockchain_operation_treasury_plmc_balance,
			expected_plmc_fee
		);
		assert_eq!(post_block_author_usdt_balance, prev_block_author_usdt_balance);
		assert_eq!(post_block_author_plmc_balance - prev_block_author_plmc_balance, expected_plmc_tip);
	});
}
