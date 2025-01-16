use crate::{constants::*, *};
use assets_common::runtime_api::runtime_decl_for_fungibles_api::FungiblesApiV2;
use frame_support::traits::{fungible::Inspect, fungibles::Mutate};
use polimec_common::assets::AcceptedFundingAsset;
use sp_arithmetic::FixedU128;
use xcm::v4::Junctions::X3;
use xcm_fee_payment_runtime_api::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;
mod xcm_payment_api {
	use super::*;
	use itertools::Itertools;

	#[test]
	fn query_acceptable_payment_assets() {
		PolimecNet::execute_with(|| {
			let accepted_payment_assets = PolimecRuntime::query_acceptable_payment_assets(4u32).unwrap();
			let versioned_funding_assets = AcceptedFundingAsset::all_ids()
				.into_iter()
				.map(|loc| AssetId::from(loc))
				.map(|a| VersionedAssetId::from(a))
				.collect_vec();

			for asset in versioned_funding_assets {
				assert!(accepted_payment_assets.contains(&asset));
			}
		});
	}

	#[test]
	fn query_weight_to_asset_fee() {
		polimec::set_prices(
			PricesBuilder::new()
				.usdt(FixedU128::from_float(1.0f64))
				.plmc(FixedU128::from_float(0.5f64))
				.dot(FixedU128::from_float(10.0f64))
				.build(),
		);

		let compute_weight = Weight::from_parts(100_000_000, 20_000);

		// Native Asset
		PolimecNet::execute_with(|| {
			let plmc_fee = PolimecRuntime::query_weight_to_asset_fee(
				compute_weight,
				VersionedAssetId::V4(AssetId(Location { parents: 0, interior: Here })),
			)
			.unwrap();

			let dot_fee = PolimecRuntime::query_weight_to_asset_fee(
				compute_weight,
				VersionedAssetId::V4(AssetId(Location { parents: 1, interior: Here })),
			)
			.unwrap();

			let usdt_fee = PolimecRuntime::query_weight_to_asset_fee(
				compute_weight,
				VersionedAssetId::V4(AssetId(Location {
					parents: 1,
					interior: X3([Parachain(1000), PalletInstance(50), GeneralIndex(1984)].into()),
				})),
			)
			.unwrap();

			// PLMC and dot have the same decimals, so a simple conversion is enough
			assert_eq!(dot_fee, plmc_fee / 20);
			// USDT has 6 decimals, so we need to divide by 10^(10-6)= 10_000
			assert_eq!(usdt_fee, plmc_fee / 2 / 10_000);
		});
	}
}

mod fungibles_api {
	use super::*;

	#[test]
	fn query_account_assets() {
		PolimecNet::execute_with(|| {
			let alice_account = PolimecNet::account_id_of(accounts::ALICE);

			assert_ok!(PolimecForeignAssets::mint_into(
				AcceptedFundingAsset::DOT.id(),
				&alice_account,
				100_0_000_000_000
			));
			assert_ok!(PolimecForeignAssets::mint_into(AcceptedFundingAsset::USDT.id(), &alice_account, 100_000));
			assert_ok!(PolimecForeignAssets::mint_into(AcceptedFundingAsset::USDC.id(), &alice_account, 100_000));
			assert_ok!(PolimecForeignAssets::mint_into(
				AcceptedFundingAsset::WETH.id(),
				&alice_account,
				100_000_000_000_000
			));

			let alice_assets = PolimecRuntime::query_account_balances(alice_account);
			dbg!(alice_assets).unwrap();
		});
	}
}
