use crate::{PolimecAccountId, PolimecBalances, PolimecCall, PolimecForeignAssets, PolimecNet, PolimecRuntime, ALICE};
use parity_scale_codec::Encode;
use polimec_runtime::{xcm_config::SupportedAssets, TreasuryAccount};
use sp_runtime::traits::MaybeEquivalence;
use xcm::prelude::*;
use xcm_emulator::{Chain, TestExt};
pub fn fake_message_hash<T>(message: &Xcm<T>) -> XcmHash {
	message.using_encoded(sp_io::hashing::blake2_256)
}
#[test]
fn execution_fees_go_to_treasury() {
	let dot_amount = Asset { id: AssetId(Location::parent()), fun: Fungible(100_0_000_000_000) };
	let usdt_amount = Asset {
		id: AssetId(Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1984)])),
		fun: Fungible(100_000_000),
	};
	let usdc_amount = Asset {
		id: AssetId(Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1337)])),
		fun: Fungible(100_000_000),
	};

	let beneficiary: PolimecAccountId = [0u8; 32].into();

	let assert_reserve_asset_fee_goes_to_treasury = |asset: Asset| {
		let asset_location = asset.id.0.clone();
		let asset_id = SupportedAssets::convert(&asset_location).unwrap();
		let asset_amount = if let Fungible(amount) = asset.fun { amount } else { unreachable!() };

		let xcm = Xcm::<PolimecCall>(vec![
			ReserveAssetDeposited(vec![asset.clone()].into()),
			ClearOrigin,
			BuyExecution { fees: asset, weight_limit: Unlimited },
			DepositAsset {
				assets: WildAsset::All.into(),
				beneficiary: Location::new(0, [AccountId32 { network: None, id: beneficiary.clone().into() }]),
			},
		]);
		let weighted_xcm = xcm_executor::WeighedMessage::new(Weight::MAX, xcm.clone()); // TODO: Check how much weight we need.
		PolimecNet::execute_with(|| {
			let prev_treasury_balance = PolimecForeignAssets::balance(asset_id, TreasuryAccount::get());
			let prev_beneficiary_balance = PolimecForeignAssets::balance(asset_id, beneficiary.clone());

			let outcome = <PolimecRuntime as pallet_xcm::Config>::XcmExecutor::execute(
				Location::new(1, [Parachain(1000)]),
				weighted_xcm,
				&mut fake_message_hash(&xcm),
				Weight::MAX,
			);
			assert!(outcome.ensure_complete().is_ok());

			let post_treasury_balance = PolimecForeignAssets::balance(asset_id, TreasuryAccount::get());
			let post_beneficiary_balance = PolimecForeignAssets::balance(asset_id, beneficiary.clone());

			let net_treasury_balance = post_treasury_balance - prev_treasury_balance;
			let net_beneficiary_balance = post_beneficiary_balance - prev_beneficiary_balance;

			let net_total = net_treasury_balance + net_beneficiary_balance;

			assert_eq!(net_total, asset_amount);
			assert!(net_treasury_balance > 0);
		});
	};

	let assert_plmc_fee_goes_to_treasury = || {
		let asset_amount = 100_0_000_000_000;
		let asset = Asset { id: AssetId(Location::here()), fun: Fungible(asset_amount) };

		let xcm = Xcm::<PolimecCall>(vec![
			WithdrawAsset(vec![asset.clone()].into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			DepositAsset {
				assets: WildAsset::All.into(),
				beneficiary: Location::new(0, [AccountId32 { network: None, id: beneficiary.clone().into() }]),
			},
		]);
		let weighted_xcm = xcm_executor::WeighedMessage::new(Weight::MAX, xcm.clone()); // TODO: Check how much weight we need.

		PolimecNet::execute_with(|| {
			let prev_treasury_balance = PolimecBalances::free_balance(TreasuryAccount::get());
			let prev_beneficiary_balance = PolimecBalances::free_balance(beneficiary.clone());

			let outcome = <PolimecRuntime as pallet_xcm::Config>::XcmExecutor::execute(
				Location::new(0, [AccountId32 { network: None, id: PolimecNet::account_id_of(ALICE).into() }]),
				weighted_xcm,
				&mut fake_message_hash(&xcm),
				Weight::MAX,
			);
			assert!(outcome.ensure_complete().is_ok());

			let post_treasury_balance = PolimecBalances::free_balance(TreasuryAccount::get());
			let post_beneficiary_balance = PolimecBalances::free_balance(beneficiary.clone());

			let net_treasury_balance = post_treasury_balance - prev_treasury_balance;
			let net_beneficiary_balance = post_beneficiary_balance - prev_beneficiary_balance;

			let net_total = net_treasury_balance + net_beneficiary_balance;

			assert_eq!(net_total, asset_amount);
			assert!(net_treasury_balance > 0);
		});
	};

	assert_reserve_asset_fee_goes_to_treasury(dot_amount);
	assert_reserve_asset_fee_goes_to_treasury(usdt_amount);
	assert_reserve_asset_fee_goes_to_treasury(usdc_amount);
	assert_plmc_fee_goes_to_treasury();
}
