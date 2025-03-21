use super::*;
use crate::{AccountIdOf, ReasonOf};
use core::marker::PhantomData;
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use sp_runtime::{traits::BlockNumberProvider, Saturating, Weight};

pub type Values<T> = BoundedVec<VestingInfoOf<T>, MaxVestingSchedulesGet<T>>;

const LOG: &str = "linear_release::migration::v1";
pub struct LinearReleaseVestingInfoMigration;

pub struct UncheckedMigrationToV1<T: Config>(PhantomData<T>);

impl<T: crate::Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV1<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		let migration_count = crate::Vesting::<T>::iter().count() as u32;
		log::info!(target: LOG, "Pre-upgrade: {} UserMigrations entries", migration_count);

		let vestings = crate::Vesting::<T>::iter().collect::<Vec<_>>();

		Ok((migration_count, vestings).encode())
	}

	fn on_runtime_upgrade() -> Weight {
		let mut items = 0u64;
		let translate_vesting_info =
			|_: AccountIdOf<T>, _: ReasonOf<T>, vesting_info: Values<T>| -> Option<Values<T>> {
				let migrated: Vec<_> = vesting_info
					.iter()
					.map(|vesting| {
						items = items.saturating_add(1);

						// adjust starting block to relay chain block number
						let relay_chain_now = T::BlockNumberProvider::current_block_number();

						let polimec_now = frame_system::Pallet::<T>::current_block_number();
						let two = 2_u32.into();

						let relay_chain_starting_block = if polimec_now < vesting.starting_block() {
							let blocks_diff = vesting.starting_block().saturating_sub(polimec_now);
							relay_chain_now.saturating_add(blocks_diff.saturating_mul(two))
						} else {
							let blocks_passed = polimec_now.saturating_sub(vesting.starting_block());
							relay_chain_now.saturating_sub(blocks_passed.saturating_mul(two))
						};

						let adjusted_per_block = vesting.per_block.saturating_mul(2_u32.into());

						VestingInfo {
							locked: vesting.locked,
							per_block: adjusted_per_block,
							starting_block: relay_chain_starting_block,
						}
					})
					.collect();

				Values::<T>::try_from(migrated).ok()
			};

		log::info!(target: LOG, "Starting linear release vesting time migration to V1");

		crate::Vesting::<T>::translate(translate_vesting_info);

		log::info!(target: LOG, "Migrated {} linear release vesting entries", items);

		T::DbWeight::get().reads_writes(items, items)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(pre_state: Vec<u8>) -> Result<(), DispatchError> {
		let (pre_migration_count, pre_vestings): (u32, Vec<((AccountIdOf<T>, ReasonOf<T>), Values<T>)>) =
			Decode::decode(&mut &pre_state[..]).expect("Failed to decode pre-migration state");

		let post_migration_count = crate::Vesting::<T>::iter().count() as u32;

		if pre_migration_count != post_migration_count {
			return Err("Migration count mismatch".into());
		}

		for ((account, reason), pre_vesting) in pre_vestings {
			let post_vesting = crate::Vesting::<T>::get(&account, &reason).unwrap_or_default();

			// check that the starting block has been adjusted
			let relay_chain_now = T::BlockNumberProvider::current_block_number();

			for (pre_vesting_info, post_vesting_info) in pre_vesting.iter().zip(post_vesting.iter()) {
				assert_ne!(
					pre_vesting_info.starting_block, post_vesting_info.starting_block,
					"Starting block not adjusted"
				);
				assert!(
					post_vesting_info.starting_block <= relay_chain_now.try_into().ok().expect("safe to convert; qed"),
					"Starting block not adjusted correctly"
				);

				assert!(
					post_vesting_info.per_block ==
						pre_vesting_info
							.per_block
							.saturating_mul(2_u32.try_into().ok().expect("safe to convert; qed")),
					"Per block not adjusted"
				);
			}
		}

		Ok(())
	}
}

pub type LinearReleaseVestingMigrationV1<T> =
	VersionedMigration<0, 1, UncheckedMigrationToV1<T>, crate::Pallet<T>, <T as frame_system::Config>::DbWeight>;

#[cfg(test)]
mod test {
	use super::*;
	use crate::{
		mock::{ExtBuilder, MockRuntimeHoldReason, System, Test},
		pallet::Vesting,
		AccountIdOf, BalanceOf, BlockNumberFor,
	};
	use frame_support::weights::RuntimeDbWeight;
	use sp_runtime::bounded_vec;

	// Helper to calculate expected results concisely
	// Now takes the *actual* polimec_now and the *fixed* relay_chain_now from the mock provider
	fn calculate_expected(
		vesting: &VestingInfo<BalanceOf<Test>, BlockNumberFor<Test>>,
		polimec_now: BlockNumberFor<Test>,
		relay_chain_now: BlockNumberFor<Test>,
	) -> VestingInfo<BalanceOf<Test>, BlockNumberFor<Test>> {
		let two: BlockNumberFor<Test> = 2u32.into();
		let expected_relay_start = if polimec_now < vesting.starting_block {
			let blocks_diff = vesting.starting_block.saturating_sub(polimec_now);
			relay_chain_now.saturating_add(blocks_diff.saturating_mul(two))
		} else {
			let blocks_passed = polimec_now.saturating_sub(vesting.starting_block);
			relay_chain_now.saturating_sub(blocks_passed.saturating_mul(two))
		};
		let expected_per_block = vesting.per_block.saturating_mul(two);

		VestingInfo {
			locked: vesting.locked, // Stays the same
			per_block: expected_per_block,
			starting_block: expected_relay_start,
		}
	}

	#[test]
	fn migration_v1_adjusts_schedules_correctly() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let polimec_now: BlockNumberFor<Test> = 100;
			System::set_block_number(polimec_now);

			// The relay chain block number is now fixed by the mock provider
			let relay_chain_now: BlockNumberFor<Test> = polimec_now;

			let account1: AccountIdOf<Test> = 3;
			let account2: AccountIdOf<Test> = 4;

			let reason1 = MockRuntimeHoldReason::Reason;
			let reason2 = MockRuntimeHoldReason::Reason2;

			// Schedule starting in the past relative to polimec_now
			let v_past = VestingInfo { locked: 1000, per_block: 10, starting_block: 50 };
			// Schedule starting in the future relative to polimec_now
			let v_future = VestingInfo { locked: 2000, per_block: 20, starting_block: 150 };
			// Schedule starting exactly now relative to polimec_now
			let v_now = VestingInfo { locked: 500, per_block: 5, starting_block: polimec_now };
			// Schedule starting at block 0 (edge case)
			let v_zero = VestingInfo { locked: 100, per_block: 1, starting_block: 0 };

			// Entry 1: Acc1, Reason1 -> Multiple schedules, covering past and future
			let schedules1: Values<Test> = bounded_vec![v_past.clone(), v_future.clone()];
			Vesting::<Test>::insert(account1, reason1.clone(), schedules1.clone());

			// Entry 2: Acc2, Reason1 -> Single schedule, covering 'now' case
			let schedules2: Values<Test> = bounded_vec![v_now.clone()];
			Vesting::<Test>::insert(account2, reason1.clone(), schedules2.clone());

			// Entry 3: Acc1, Reason2 -> Single schedule, edge case start block
			let schedules3: Values<Test> = bounded_vec![v_zero.clone()];
			Vesting::<Test>::insert(account1, reason2.clone(), schedules3.clone());

			// Verify initial counts
			let initial_storage_entries = Vesting::<Test>::iter_keys().count(); // Counts distinct (Acc, Reason) pairs
			let initial_schedules_count: u64 = Vesting::<Test>::iter_values().map(|v| v.len() as u64).sum();
			assert_eq!(initial_storage_entries, 6); // default already adds 3 entries
			assert_eq!(initial_schedules_count, 7); // 3 from default, 4 from our setup

			let weight = UncheckedMigrationToV1::<Test>::on_runtime_upgrade();

			assert_eq!(
				Vesting::<Test>::iter_keys().count(),
				initial_storage_entries,
				"Number of storage entries should not change"
			);

			let migrated1 = Vesting::<Test>::get(account1, reason1.clone()).unwrap();
			assert_eq!(migrated1.len(), 2);

			assert_eq!(migrated1[0], calculate_expected(&v_past, polimec_now, relay_chain_now));
			assert_eq!(migrated1[1], calculate_expected(&v_future, polimec_now, relay_chain_now));

			let migrated2 = Vesting::<Test>::get(account2, reason1.clone()).unwrap();
			assert_eq!(migrated2.len(), 1);
			assert_eq!(migrated2[0], calculate_expected(&v_now, polimec_now, relay_chain_now));

			let migrated3 = Vesting::<Test>::get(account1, reason2.clone()).unwrap();
			assert_eq!(migrated3.len(), 1);
			assert_eq!(migrated3[0], calculate_expected(&v_zero, polimec_now, relay_chain_now));

			let db_weight: RuntimeDbWeight = <Test as frame_system::Config>::DbWeight::get();
			let expected_weight = db_weight.reads_writes(initial_schedules_count, initial_schedules_count);

			assert_eq!(weight, expected_weight, "Weight should match items processed");

			// also verify default vesting entries that are migrated
			let default_vesting = Vesting::<Test>::get(1, MockRuntimeHoldReason::Reason).unwrap();
			assert_eq!(
				default_vesting[0],
				calculate_expected(
					&VestingInfo { starting_block: 0, per_block: 128, locked: 5 * 256 },
					polimec_now,
					relay_chain_now
				),
			);

			let default_vesting_2 = Vesting::<Test>::get(2, MockRuntimeHoldReason::Reason).unwrap();
			assert_eq!(
				default_vesting_2[0],
				calculate_expected(
					&VestingInfo { starting_block: 10, per_block: 256, locked: 20 * 256 },
					polimec_now,
					relay_chain_now
				),
			);
		});
	}
}
