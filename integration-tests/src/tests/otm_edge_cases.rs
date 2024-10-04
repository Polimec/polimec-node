use crate::{
	constants::PricesBuilder,
	tests::defaults::{default_evaluations, default_project_metadata, ipfs_hash, IntegrationInstantiator},
	*,
};
use frame_support::traits::fungibles::Inspect;
use macros::generate_accounts;
use pallet_funding::{AcceptedFundingAsset, MultiplierOf, ParticipationMode, PriceProviderOf};
use polimec_common::{credentials::InvestorType, ProvideAssetPrice, PLMC_DECIMALS, PLMC_FOREIGN_ID, USD_UNIT};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use polimec_runtime::OraclePriceProvider;
use sp_arithmetic::{FixedPointNumber, FixedU128, Perbill};
use sp_core::bounded_vec;
use sp_runtime::TokenError;
generate_accounts!(ISSUER, BOBERT);
use pallet_funding::traits::BondingRequirementCalculation;

#[test]
fn cannot_have_otm_fee_below_min_amount() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer: PolimecAccountId = ISSUER.into();
	let bobert: PolimecAccountId = BOBERT.into();

	let prices = PricesBuilder::new()
		.plmc(FixedU128::from_float(0.17f64))
		.usdt(FixedU128::from_float(0.9999f64))
		.usdc(FixedU128::from_float(1.0001f64))
		.dot(FixedU128::from_float(4.0f64))
		.build();

	polimec::set_prices(prices);

	PolimecNet::execute_with(|| {
		let mut project_metadata = default_project_metadata(issuer.clone());
		project_metadata.participation_currencies =
			bounded_vec![AcceptedFundingAsset::USDT, AcceptedFundingAsset::USDC, AcceptedFundingAsset::DOT,];

		let usdt_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			AcceptedFundingAsset::USDT.id(),
			6,
			6,
		)
		.unwrap();
		let usdc_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			AcceptedFundingAsset::USDC.id(),
			6,
			6,
		)
		.unwrap();
		let dot_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			AcceptedFundingAsset::USDC.id(),
			6,
			10,
		)
		.unwrap();

		let plmc_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			PLMC_FOREIGN_ID,
			6,
			PLMC_DECIMALS,
		).unwrap();

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations(),
			vec![],
		);

		let plmc_ed = inst.get_ed();

		let min_usd_contribution = 10 * USD_UNIT;
		let otm_multiplier: MultiplierOf<PolimecRuntime> = ParticipationMode::OTM.multiplier().try_into().unwrap();
		let min_usd_bond =
			otm_multiplier.calculate_usd_bonding_requirement::<PolimecRuntime>(min_usd_contribution).unwrap();
		let min_plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(min_usd_bond);
		let min_usd_otm_fee = polimec_runtime::ProxyBonding::calculate_fee(min_plmc_bond, AcceptedFundingAsset::USDT.id()).unwrap();

		let mut min_usdt_contribution = usdt_price.reciprocal().unwrap().saturating_mul_int(min_usd_contribution);
		while usdt_price.saturating_mul_int(min_usdt_contribution) < min_usd_contribution {
			min_usdt_contribution += 1;
		}

		let mut min_usdc_contribution = usdc_price.reciprocal().unwrap().saturating_mul_int(min_usd_contribution);
		while usdc_price.saturating_mul_int(min_usdc_contribution) < min_usd_contribution {
			min_usdc_contribution += 1;
		}

		let mut min_dot_contribution = dot_price.reciprocal().unwrap().saturating_mul_int(min_usd_contribution);
		while dot_price.saturating_mul_int(min_dot_contribution) < min_usd_contribution {
			min_dot_contribution += 1;
		}

		let min_usdt_contribution_otm_fee = usdt_price.reciprocal().unwrap().saturating_mul_int(min_usd_otm_fee);
		let min_usdc_contribution_otm_fee = usdc_price.reciprocal().unwrap().saturating_mul_int(min_usd_otm_fee);
		let min_dot_contribution_otm_fee = dot_price.reciprocal().unwrap().saturating_mul_int(min_usd_otm_fee);

		let usdt_min_balance = inst.execute(|| PolimecForeignAssets::minimum_balance(AcceptedFundingAsset::USDT.id()));
		let usdc_min_balance = inst.execute(|| PolimecForeignAssets::minimum_balance(AcceptedFundingAsset::USDC.id()));
		let dot_min_balance = inst.execute(|| PolimecForeignAssets::minimum_balance(AcceptedFundingAsset::DOT.id()));

		assert!(min_usdt_contribution_otm_fee > usdt_min_balance);
		assert!(min_usdc_contribution_otm_fee > usdc_min_balance);
		assert!(min_dot_contribution_otm_fee > dot_min_balance);

		let ct_for_min_usdt_contribution =
			PolimecFunding::funding_asset_to_ct_amount(project_id, AcceptedFundingAsset::USDT, min_usdt_contribution);
		let ct_for_min_usdc_contribution =
			PolimecFunding::funding_asset_to_ct_amount(project_id, AcceptedFundingAsset::USDC, min_usdc_contribution);
		let ct_for_min_dot_contribution =
			PolimecFunding::funding_asset_to_ct_amount(project_id, AcceptedFundingAsset::DOT, min_dot_contribution);

		let jwt = get_mock_jwt_with_cid(
			bobert.clone(),
			InvestorType::Retail,
			generate_did_from_account(bobert.clone()),
			ipfs_hash(),
		);

		inst.mint_plmc_to(vec![(bobert.clone(), plmc_ed).into()]);
		inst.mint_funding_asset_to(vec![
			(
				bobert.clone(),
				min_usdt_contribution + min_usdt_contribution_otm_fee + 10_000,
				AcceptedFundingAsset::USDT.id(),
			)
				.into(),
			(
				bobert.clone(),
				min_usdc_contribution + min_usdc_contribution_otm_fee + 10_000,
				AcceptedFundingAsset::USDC.id(),
			)
				.into(),
			(
				bobert.clone(),
				min_dot_contribution + min_dot_contribution_otm_fee + 10_000,
				AcceptedFundingAsset::DOT.id(),
			)
				.into(),
		]);

		let contribute_is_ok_with = |asset, ct_amount| {
			assert_ok!(PolimecFunding::contribute(
				PolimecOrigin::signed(bobert.clone()),
				jwt.clone(),
				project_id,
				ct_amount,
				ParticipationMode::OTM,
				asset
			));
		};
		contribute_is_ok_with(AcceptedFundingAsset::USDT, ct_for_min_usdt_contribution);
		contribute_is_ok_with(AcceptedFundingAsset::USDC, ct_for_min_usdc_contribution);
		contribute_is_ok_with(AcceptedFundingAsset::DOT, ct_for_min_dot_contribution);
	});
}

#[test]
fn after_otm_fee_user_goes_under_ed() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer: PolimecAccountId = ISSUER.into();
	let bobert: PolimecAccountId = BOBERT.into();

	polimec::set_prices(PricesBuilder::default());
	PolimecNet::execute_with(|| {
		let mut project_metadata = default_project_metadata(issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations(),
			vec![],
		);

		// default price of usdt = 1usd
		let usdt_contribution = 100 * USD_UNIT;
		let contribution_otm_fee = polimec_runtime::FeePercentage::get() * usdt_contribution;

		let usdt_min_balance = inst.execute(|| PolimecForeignAssets::minimum_balance(AcceptedFundingAsset::USDT.id()));
		assert!(contribution_otm_fee > usdt_min_balance);

		let ct_for_contribution =
			PolimecFunding::funding_asset_to_ct_amount(project_id, AcceptedFundingAsset::USDT, usdt_contribution);
		let jwt = get_mock_jwt_with_cid(
			bobert.clone(),
			InvestorType::Retail,
			generate_did_from_account(bobert.clone()),
			ipfs_hash(),
		);

		let usdt_mint = usdt_contribution + contribution_otm_fee;
		inst.mint_funding_asset_to(vec![(bobert.clone(), usdt_mint, AcceptedFundingAsset::USDT.id()).into()]);

		assert_ok!(PolimecFunding::contribute(
			PolimecOrigin::signed(bobert.clone()),
			jwt.clone(),
			project_id,
			ct_for_contribution,
			ParticipationMode::OTM,
			AcceptedFundingAsset::USDT,
		));

		// Somehow still exists because sufficients =  1
		assert!(PolimecSystem::account_exists(&bobert));
		dbg!(PolimecSystem::account(&bobert));
	});
}
