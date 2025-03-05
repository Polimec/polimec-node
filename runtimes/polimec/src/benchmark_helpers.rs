extern crate alloc;

use crate::{Oracle, Runtime, RuntimeOrigin};
use alloc::vec;
use frame_support::instances::Instance1;
use pallet_funding::traits::SetPrices;
use polimec_common::assets::AcceptedFundingAsset;
use sp_runtime::{BoundedVec, FixedU128};
use xcm::v5::Location;

pub struct SetOraclePrices;
impl SetPrices for SetOraclePrices {
	fn set_prices() {
		let dot = (AcceptedFundingAsset::DOT.id(), FixedU128::from_rational(69, 1));
		let usdc = (AcceptedFundingAsset::USDC.id(), FixedU128::from_rational(1, 1));
		let usdt = (AcceptedFundingAsset::USDT.id(), FixedU128::from_rational(1, 1));
		let eth = (AcceptedFundingAsset::ETH.id(), FixedU128::from_rational(20_000, 1));
		let plmc = (Location::here(), FixedU128::from_rational(840, 100));

		let values: BoundedVec<(Location, FixedU128), <Runtime as orml_oracle::Config>::MaxFeedValues> =
			vec![dot, usdc, usdt, plmc, eth].try_into().expect("benchmarks can panic");

		let oracle_members = pallet_membership::Members::<crate::Runtime, Instance1>::get().to_vec();
		for member in oracle_members {
			frame_support::assert_ok!(Oracle::feed_values(
				RuntimeOrigin::signed(member.clone().into()),
				values.clone()
			));
		}
	}
}
