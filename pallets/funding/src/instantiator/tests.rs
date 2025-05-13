use crate::{
	mock::{new_test_ext, TestRuntime},
	tests::{defaults::default_project_metadata, CT_DECIMALS, CT_UNIT},
	*,
};
use core::cell::RefCell;
use polimec_common::{assets::AcceptedFundingAsset, ProvideAssetPrice, USD_DECIMALS, USD_UNIT};
use sp_arithmetic::Perquintill;

#[test]
fn generate_bids_from_bucket() {
	let mut inst = tests::MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	// Has a min price of 10.0
	let project_metadata = default_project_metadata(0);
	let desired_real_wap = FixedU128::from_float(20.0f64);
	let desired_bucket_price_aware =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(desired_real_wap, USD_DECIMALS, CT_DECIMALS)
			.unwrap();
	let mut necessary_bucket = Pallet::<TestRuntime>::create_bucket_from_metadata(&project_metadata);
	necessary_bucket.current_price = desired_bucket_price_aware;
	necessary_bucket.amount_left = necessary_bucket.delta_amount;

	let bids = inst.generate_bids_from_bucket(project_metadata.clone(), necessary_bucket, AcceptedFundingAsset::USDT);
	let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
	let project_id = inst.create_finished_project(project_metadata.clone(), 0, None, evaluations, bids);
	let current_bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());
	assert_eq!(current_bucket.current_price, desired_bucket_price_aware);
}

#[test]
fn generate_bids_from_higher_usd_than_target() {
	let mut inst = tests::MockInstantiator::new(Some(RefCell::new(new_test_ext())));
	let mut project_metadata = default_project_metadata(0);
	project_metadata.total_allocation_size = 100_000 * CT_UNIT;

	const TARGET_USD: u128 = 1_500_000 * USD_UNIT;
	let bids = inst.generate_bids_from_higher_usd_than_target(project_metadata.clone(), TARGET_USD);
	let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 5);
	let project_id = inst.create_finished_project(project_metadata, 0, None, evaluations, bids);
	let project_details = inst.get_project_details(project_id);
	assert_close_enough!(project_details.funding_amount_reached_usd, TARGET_USD, Perquintill::from_float(0.9999));
}
