use crate::{
	constants::PricesBuilder,
	tests::defaults::{default_evaluations, default_project_metadata, ipfs_hash, IntegrationInstantiator},
	*,
};
use frame_support::traits::fungibles::Inspect;
use macros::generate_accounts;
use pallet_funding::{traits::BondingRequirementCalculation, MultiplierOf, ParticipationMode};
use polimec_common::{
	assets::AcceptedFundingAsset, credentials::InvestorType, ProvideAssetPrice, PLMC_DECIMALS, USD_UNIT,
};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_core::bounded_vec;
use sp_runtime::TokenError;

generate_accounts!(ISSUER, BOBERT);

#[test]
fn otm_fee_below_min_amount_reverts() {
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

		let plmc_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			Location::here(),
			6,
			PLMC_DECIMALS,
		)
		.unwrap();

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations(),
			vec![],
		);

		let plmc_ed = inst.get_ed();

		let min_usd_contribution = USD_UNIT;
		let otm_multiplier: MultiplierOf<PolimecRuntime> = ParticipationMode::OTM.multiplier().try_into().unwrap();
		let min_usd_bond =
			otm_multiplier.calculate_usd_bonding_requirement::<PolimecRuntime>(min_usd_contribution).unwrap();
		let min_plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(min_usd_bond);
		let min_usd_otm_fee =
			polimec_runtime::ProxyBonding::calculate_fee(min_plmc_bond, AcceptedFundingAsset::USDT.id()).unwrap();

		let mut min_usdt_contribution = usdt_price.reciprocal().unwrap().saturating_mul_int(min_usd_contribution);
		while usdt_price.saturating_mul_int(min_usdt_contribution) < min_usd_contribution {
			min_usdt_contribution += 1;
		}

		let min_usdt_contribution_otm_fee = usdt_price.reciprocal().unwrap().saturating_mul_int(min_usd_otm_fee);

		let usdt_min_balance = inst.execute(|| PolimecForeignAssets::minimum_balance(AcceptedFundingAsset::USDT.id()));

		assert!(min_usdt_contribution_otm_fee < usdt_min_balance);

		let ct_for_min_usdt_contribution = PolimecFunding::funding_asset_to_ct_amount_classic(
			project_id,
			AcceptedFundingAsset::USDT,
			min_usdt_contribution,
		);

		let jwt = get_mock_jwt_with_cid(
			bobert.clone(),
			InvestorType::Retail,
			generate_did_from_account(bobert.clone()),
			ipfs_hash(),
		);

		inst.mint_plmc_to(vec![(bobert.clone(), plmc_ed).into()]);
		inst.mint_funding_asset_to(vec![(
			bobert.clone(),
			min_usdt_contribution + min_usdt_contribution_otm_fee + 10_000,
			AcceptedFundingAsset::USDT.id(),
		)
			.into()]);

		// Assert noop checks that storage had no changes
		assert_noop!(
			PolimecFunding::contribute(
				PolimecOrigin::signed(bobert.clone()),
				jwt.clone(),
				project_id,
				ct_for_min_usdt_contribution,
				ParticipationMode::OTM,
				AcceptedFundingAsset::USDT
			),
			TokenError::BelowMinimum
		);
	});
}

#[test]
fn after_otm_fee_user_goes_under_ed_reverts() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer: PolimecAccountId = ISSUER.into();
	let bobert: PolimecAccountId = BOBERT.into();

	polimec::set_prices(PricesBuilder::default_prices());
	PolimecNet::execute_with(|| {
		let project_metadata = default_project_metadata(issuer.clone());

		let project_id = inst.create_community_contributing_project(
			project_metadata.clone(),
			issuer.clone(),
			None,
			default_evaluations(),
			vec![],
		);

		let plmc_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			Location::here(),
			6,
			PLMC_DECIMALS,
		)
		.unwrap();
		let usdt_price = <PolimecRuntime as pallet_funding::Config>::PriceProvider::get_decimals_aware_price(
			AcceptedFundingAsset::USDT.id(),
			6,
			6,
		)
		.unwrap();

		let usd_contribution = 100 * USD_UNIT;
		let otm_multiplier: MultiplierOf<PolimecRuntime> = ParticipationMode::OTM.multiplier().try_into().unwrap();
		let usd_bond = otm_multiplier.calculate_usd_bonding_requirement::<PolimecRuntime>(usd_contribution).unwrap();
		let plmc_bond = plmc_price.reciprocal().unwrap().saturating_mul_int(usd_bond);
		let usd_otm_fee =
			polimec_runtime::ProxyBonding::calculate_fee(plmc_bond, AcceptedFundingAsset::USDT.id()).unwrap();

		let usdt_ed = inst.get_funding_asset_ed(AcceptedFundingAsset::USDT.id());
		let usdt_contribution = usdt_price.reciprocal().unwrap().saturating_mul_int(usd_contribution);
		let usdt_otm_fee = usdt_price.reciprocal().unwrap().saturating_mul_int(usd_otm_fee);

		let ct_for_contribution = PolimecFunding::funding_asset_to_ct_amount_classic(
			project_id,
			AcceptedFundingAsset::USDT,
			usdt_contribution,
		);
		let jwt = get_mock_jwt_with_cid(
			bobert.clone(),
			InvestorType::Retail,
			generate_did_from_account(bobert.clone()),
			ipfs_hash(),
		);

		inst.mint_funding_asset_to(vec![(
			bobert.clone(),
			usdt_contribution + usdt_otm_fee,
			AcceptedFundingAsset::USDT.id(),
		)
			.into()]);

		assert_noop!(
			PolimecFunding::contribute(
				PolimecOrigin::signed(bobert.clone()),
				jwt.clone(),
				project_id,
				ct_for_contribution,
				ParticipationMode::OTM,
				AcceptedFundingAsset::USDT,
			),
			pallet_funding::Error::<PolimecRuntime>::ParticipantNotEnoughFunds
		);

		inst.mint_funding_asset_to(vec![(bobert.clone(), usdt_ed, AcceptedFundingAsset::USDT.id()).into()]);

		assert_ok!(PolimecFunding::contribute(
			PolimecOrigin::signed(bobert.clone()),
			jwt.clone(),
			project_id,
			ct_for_contribution,
			ParticipationMode::OTM,
			AcceptedFundingAsset::USDT,
		));
	});
}
