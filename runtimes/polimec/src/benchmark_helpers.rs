extern crate alloc;

use crate::{Oracle, Runtime, RuntimeOrigin};
use alloc::vec;
use pallet_funding::traits::SetPrices;
use polimec_common::assets::AcceptedFundingAsset;
use sp_runtime::{BoundedVec, FixedU128};
use xcm::v4::Location;

pub struct SetOraclePrices;
impl SetPrices for SetOraclePrices {
	fn set_prices() {
		let dot = (AcceptedFundingAsset::DOT.id(), FixedU128::from_rational(69, 1));
		let usdc = (AcceptedFundingAsset::USDC.id(), FixedU128::from_rational(1, 1));
		let usdt = (AcceptedFundingAsset::USDT.id(), FixedU128::from_rational(1, 1));
		let plmc = (Location::here(), FixedU128::from_rational(840, 100));

		let values: BoundedVec<(Location, FixedU128), <Runtime as orml_oracle::Config>::MaxFeedValues> =
			vec![dot, usdc, usdt, plmc].try_into().expect("benchmarks can panic");
		let alice: [u8; 32] = [
			212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205,
			227, 154, 86, 132, 231, 165, 109, 162, 125,
		];
		let bob: [u8; 32] = [
			142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161, 126, 37, 252, 82, 135, 97, 54, 147, 201, 18, 144,
			156, 178, 38, 170, 71, 148, 242, 106, 72,
		];
		let charlie: [u8; 32] = [
			144, 181, 171, 32, 92, 105, 116, 201, 234, 132, 27, 230, 136, 134, 70, 51, 220, 156, 168, 163, 87, 132, 62,
			234, 207, 35, 20, 100, 153, 101, 254, 34,
		];

		frame_support::assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone().into()), values.clone()));

		frame_support::assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone().into()), values.clone()));

		frame_support::assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone().into()), values.clone()));
	}
}
