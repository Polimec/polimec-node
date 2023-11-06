use crate::{
	accounts::{ALICE, BOB, CHARLIE, DAVE},
	*,
};
/// Tests for the oracle pallet integration.
/// Alice, Bob, Charlie are members of the OracleProvidersMembers.
/// Only members should be able to feed data into the oracle.
use parity_scale_codec::alloc::collections::HashMap;
use polimec_parachain_runtime::{Oracle, RuntimeOrigin};
use sp_runtime::{bounded_vec, BoundedVec, FixedU128};

fn values(
	values: [f64; 4],
) -> BoundedVec<
	(u32, FixedU128),
	<polimec_parachain_runtime::Runtime as orml_oracle::Config<orml_oracle::Instance1>>::MaxFeedValues,
> {
	let [dot, usdc, usdt, plmc] = values;
	bounded_vec![
		(0u32, FixedU128::from_float(dot)),
		(420u32, FixedU128::from_float(usdc)),
		(1984u32, FixedU128::from_float(usdt)),
		(2069u32, FixedU128::from_float(plmc))
	]
}

#[test]
fn members_can_feed_data() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let bob = Polimec::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let charlie = Polimec::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([4.84, 1.0, 1.0, 0.4])));

		let expected_values = HashMap::from([
			(0u32, FixedU128::from_float(4.84)),
			(420u32, FixedU128::from_float(1.0)),
			(1984u32, FixedU128::from_float(1.0)),
			(2069u32, FixedU128::from_float(0.4)),
		]);

		for (key, value) in Oracle::get_all_values() {
			assert!(value.is_some());
			assert_eq!(expected_values.get(&key).unwrap(), &value.unwrap().value);
		}
	})
}

#[test]
fn non_members_cannot_feed_data() {
	Polimec::execute_with(|| {
		let dave = Polimec::account_id_of(DAVE);
		assert_noop!(
			Oracle::feed_values(RuntimeOrigin::signed(dave.clone()), values([4.84, 1.0, 1.0, 0.4])),
			orml_oracle::Error::<polimec_parachain_runtime::Runtime, orml_oracle::Instance1>::NoPermission
		);
	});
}

#[test]
fn data_is_correctly_combined() {
	Polimec::execute_with(|| {
		let alice = Polimec::account_id_of(ALICE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(alice.clone()), values([1.0, 1.5, 1.1, 0.11111])));

		let bob = Polimec::account_id_of(BOB);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(bob.clone()), values([2.0, 1.0, 1.2, 0.22222])));

		let charlie = Polimec::account_id_of(CHARLIE);
		assert_ok!(Oracle::feed_values(RuntimeOrigin::signed(charlie.clone()), values([3.0, 0.8, 1.1, 0.33333])));

		// Default CombineData implementation is the median value
		let expected_values = HashMap::from([
			(0u32, FixedU128::from_float(2.0)),
			(420u32, FixedU128::from_float(1.0)),
			(1984u32, FixedU128::from_float(1.1)),
			(2069u32, FixedU128::from_float(0.22222)),
		]);

		for (key, value) in Oracle::get_all_values() {
			assert!(value.is_some());
			assert_eq!(expected_values.get(&key).unwrap(), &value.unwrap().value);
		}
	})
}
