// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::*;
use polimec_common::assets::AcceptedFundingAsset;
use polimec_runtime::{Oracle, RuntimeOrigin};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128};
use std::collections::BTreeMap;
use tests::defaults::*;
use AcceptedFundingAsset::{DOT, ETH, USDC, USDT};

fn values(
	values: [f64; 5],
) -> BoundedVec<(Location, FixedU128), <polimec_runtime::Runtime as orml_oracle::Config<()>>::MaxFeedValues> {
	let [dot, usdc, usdt, eth, plmc] = values;
	bounded_vec![
		(DOT.id(), FixedU128::from_float(dot)),
		(USDC.id(), FixedU128::from_float(usdc)),
		(USDT.id(), FixedU128::from_float(usdt)),
		(ETH.id(), FixedU128::from_float(eth)),
		(Location::here(), FixedU128::from_float(plmc))
	]
}

#[test]
fn members_can_feed_data() {
	let mut inst = IntegrationInstantiator::new(None);

	PolimecNet::execute_with(|| {
		// pallet_funding genesis builder already inputs prices, so we need to advance one block to feed new values.
		inst.advance_time(1u32);
		let alice = PolimecNet::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let expected_values = BTreeMap::from([
			(DOT.id(), FixedU128::from_float(4.84)),
			(USDC.id(), FixedU128::from_float(1.0)),
			(USDT.id(), FixedU128::from_float(1.0)),
			(ETH.id(), FixedU128::from_float(2500.0)),
			(Location::here(), FixedU128::from_float(0.4)),
		]);

		for (key, value) in Oracle::get_all_values() {
			assert!(value.is_some());
			assert_eq!(expected_values.get(&key).unwrap(), &value.unwrap().value);
		}
	})
}

#[test]
fn non_members_cannot_feed_data() {
	PolimecNet::execute_with(|| {
		let dave = PolimecNet::account_id_of(DAVE);
		assert_noop!(
			Oracle::feed_values(RuntimeOrigin::signed(dave.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])),
			orml_oracle::Error::<polimec_runtime::Runtime, ()>::NoPermission
		);
	});
}

#[test]
fn data_is_correctly_combined() {
	let mut inst = IntegrationInstantiator::new(None);
	PolimecNet::execute_with(|| {
		// pallet_funding genesis builder already inputs prices, so we need to advance one block to feed new values.
		inst.advance_time(1u32);

		let alice = PolimecNet::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([1.0, 1.5, 1.1, 2500.0, 0.11111])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([2.0, 1.0, 1.2, 2500.0, 0.22222])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(
			RuntimeOrigin::signed(charlie.clone()),
			values([3.0, 0.8, 1.1, 2500.0, 0.33333])
		));

		// Default CombineData implementation is the median value
		let expected_values = BTreeMap::from([
			(DOT.id(), FixedU128::from_float(2.0)),
			(USDC.id(), FixedU128::from_float(1.0)),
			(USDT.id(), FixedU128::from_float(1.1)),
			(ETH.id(), FixedU128::from_float(2500.0)),
			(Location::here(), FixedU128::from_float(0.22222)),
		]);

		for (key, value) in Oracle::get_all_values() {
			assert!(value.is_some());
			assert_eq!(expected_values.get(&key).unwrap(), &value.unwrap().value);
		}
	})
}

#[test]
fn pallet_funding_works() {
	let mut inst = IntegrationInstantiator::new(None);

	PolimecNet::execute_with(|| {
		// pallet_funding genesis builder already inputs prices, so we need to advance one block to feed new values.
		inst.advance_time(1u32);

		let alice = PolimecNet::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([4.84, 1.0, 1.0, 2500.0, 0.4])));

		let project_metadata = default_project_metadata(ISSUER.into());
		let evaluations = inst.generate_successful_evaluations(project_metadata.clone(), 10);
		let bids = inst.generate_bids_from_total_ct_percent(project_metadata.clone(), 95, 30);
		let _project_id = inst.create_finished_project(project_metadata, ISSUER.into(), None, evaluations, bids);
	});
}
