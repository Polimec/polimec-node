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
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use parity_scale_codec::alloc::collections::HashMap;
use polimec_runtime::{Oracle, RuntimeOrigin};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128};
use tests::defaults::*;

fn values(
	values: [f64; 4],
) -> BoundedVec<(u32, FixedU128), <polimec_runtime::Runtime as orml_oracle::Config<()>>::MaxFeedValues> {
	let [dot, usdc, usdt, plmc] = values;
	bounded_vec![
		(10u32, FixedU128::from_float(dot)),
		(1337u32, FixedU128::from_float(usdc)),
		(1984u32, FixedU128::from_float(usdt)),
		(3344u32, FixedU128::from_float(plmc))
	]
}

#[test]
fn members_can_feed_data() {
	let mut inst = IntegrationInstantiator::new(None);

	PolimecNet::execute_with(|| {
		// pallet_funding genesis builder already inputs prices, so we need to advance one block to feed new values.
		inst.advance_time(1u32);
		let alice = PolimecNet::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let expected_values = HashMap::from([
			(10u32, FixedU128::from_float(4.84)),
			(1337u32, FixedU128::from_float(1.0)),
			(1984u32, FixedU128::from_float(1.0)),
			(3344u32, FixedU128::from_float(0.4)),
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
			Oracle::feed_values(RuntimeOrigin::signed(dave.clone()), values([4.84, 1.0, 1.0, 0.4])),
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
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([1.0, 1.5, 1.1, 0.11111])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([2.0, 1.0, 1.2, 0.22222])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([3.0, 0.8, 1.1, 0.33333])));

		// Default CombineData implementation is the median value
		let expected_values = HashMap::from([
			(10u32, FixedU128::from_float(2.0)),
			(1337u32, FixedU128::from_float(1.0)),
			(1984u32, FixedU128::from_float(1.1)),
			(3344u32, FixedU128::from_float(0.22222)),
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
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let bob = PolimecNet::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let charlie = PolimecNet::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let _project_id = inst.create_finished_project(
			default_project_metadata(ISSUER.into()),
			ISSUER.into(),
			None,
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			vec![],
		);
	});
}
