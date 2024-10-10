use crate::{constants::*, *};

mod xcm_payment_api {
	use super::*;
	use sp_arithmetic::FixedU128;
	use xcm::v4::Junctions::X3;
	use xcm_fee_payment_runtime_api::fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1;

	#[test]
	fn query_acceptable_payment_assets() {
		PolimecNet::execute_with(|| {
			let accepted_payment_assets = PolimecRuntime::query_acceptable_payment_assets(4u32).unwrap();
			assert_eq!(
				accepted_payment_assets,
				vec![
					VersionedAssetId::V4(AssetId(Location { parents: 0, interior: Here },),),
					VersionedAssetId::V4(AssetId(Location { parents: 1, interior: Here },),),
					VersionedAssetId::V4(AssetId(Location {
						parents: 1,
						interior: X3([Parachain(1000,), PalletInstance(50,), GeneralIndex(1984,),].into(),),
					},),),
					VersionedAssetId::V4(AssetId(Location {
						parents: 1,
						interior: X3([Parachain(1000,), PalletInstance(50,), GeneralIndex(1337,),].into(),),
					},),),
				]
			);
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
			dbg!(plmc_fee);

			let dot_fee = PolimecRuntime::query_weight_to_asset_fee(
				compute_weight,
				VersionedAssetId::V4(AssetId(Location { parents: 1, interior: Here })),
			)
			.unwrap();
			dbg!(dot_fee);

			let usdt_fee = PolimecRuntime::query_weight_to_asset_fee(
				compute_weight,
				VersionedAssetId::V4(AssetId(Location {
					parents: 1,
					interior: X3([Parachain(1000), PalletInstance(50), GeneralIndex(1984)].into()),
				})),
			)
			.unwrap();
			dbg!(usdt_fee);

			// PLMC and dot have the same decimals, so a simple conversion is enough
			assert_eq!(dot_fee, plmc_fee / 20);
			// USDT has 6 decimals, so we need to divide by 10^(10-6)= 10_000
			assert_eq!(usdt_fee, plmc_fee / 2 / 10_000);
		});
	}
}
