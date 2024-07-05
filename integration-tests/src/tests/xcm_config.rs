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
	let dot_amount = MultiAsset { id: Concrete(MultiLocation::parent()), fun: Fungible(100_0_000_000_000) };
	let usdt_amount = MultiAsset {
		id: Concrete(MultiLocation {
			parents: 1,
			interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)),
		}),
		fun: Fungible(100_000_000),
	};
	let usdc_amount = MultiAsset {
		id: Concrete(MultiLocation {
			parents: 1,
			interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1337)),
		}),
		fun: Fungible(100_000_000),
	};

	let beneficiary: PolimecAccountId = [0u8; 32].into();

	let assert_reserve_asset_fee_goes_to_treasury = |multi_asset: MultiAsset| {
		let asset_multilocation =
			if let Concrete(asset_multilocation) = multi_asset.id { asset_multilocation } else { unreachable!() };
		let asset_id = SupportedAssets::convert(&asset_multilocation).unwrap();
		let asset_amount = if let Fungible(amount) = multi_asset.fun { amount } else { unreachable!() };

		let xcm = Xcm::<PolimecCall>(vec![
			ReserveAssetDeposited(vec![multi_asset.clone()].into()),
			ClearOrigin,
			BuyExecution { fees: multi_asset, weight_limit: Unlimited },
			DepositAsset {
				assets: WildMultiAsset::All.into(),
				beneficiary: MultiLocation::new(0, X1(AccountId32 { network: None, id: beneficiary.clone().into() })),
			},
		])
		.into();
		PolimecNet::execute_with(|| {
			let prev_treasury_balance = PolimecForeignAssets::balance(asset_id, TreasuryAccount::get());
			let prev_beneficiary_balance = PolimecForeignAssets::balance(asset_id, beneficiary.clone());

			let outcome = <PolimecRuntime as pallet_xcm::Config>::XcmExecutor::execute_xcm(
				MultiLocation::new(1, X1(Parachain(1000))),
				xcm.clone(),
				fake_message_hash(&xcm),
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
		let multi_asset = MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(asset_amount) };

		let xcm = Xcm::<PolimecCall>(vec![
			WithdrawAsset(vec![multi_asset.clone()].into()),
			BuyExecution { fees: multi_asset, weight_limit: Unlimited },
			DepositAsset {
				assets: WildMultiAsset::All.into(),
				beneficiary: MultiLocation::new(0, X1(AccountId32 { network: None, id: beneficiary.clone().into() })),
			},
		])
		.into();
		PolimecNet::execute_with(|| {
			let prev_treasury_balance = PolimecBalances::free_balance(TreasuryAccount::get());
			let prev_beneficiary_balance = PolimecBalances::free_balance(beneficiary.clone());

			let outcome = <PolimecRuntime as pallet_xcm::Config>::XcmExecutor::execute_xcm(
				MultiLocation::new(0, X1(AccountId32 { network: None, id: PolimecNet::account_id_of(ALICE).into() })),
				xcm.clone(),
				fake_message_hash(&xcm),
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
