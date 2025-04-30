use super::*;
use core::marker::PhantomData;
use frame_support::{migrations::VersionedMigration, traits::UncheckedOnRuntimeUpgrade};
use sp_runtime::{traits::BlockNumberProvider, Saturating, Weight};

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
		// Fetch block numbers once per entry for consistency
		let relay_chain_now = T::BlockNumberProvider::current_block_number();
		let polimec_now = frame_system::Pallet::<T>::current_block_number();

		// Define constants with explicit types
		let two_bn: BlockNumberFor<T> = 2u32.into();
		let two_balance: BalanceOf<T> = 2u32.into();

		let translate_vesting_info = |vesting_info: EntriesOf<T>| -> Option<EntriesOf<T>> {
			let migrated: Vec<_> = vesting_info
				.iter()
				.map(|vesting| {
					items = items.saturating_add(1);

					let relay_chain_starting_block = if polimec_now < vesting.starting_block {
						let blocks_diff = vesting.starting_block.saturating_sub(polimec_now);
						relay_chain_now.saturating_add(blocks_diff.saturating_mul(two_bn))
					} else {
						let blocks_passed = polimec_now.saturating_sub(vesting.starting_block);
						relay_chain_now.saturating_sub(blocks_passed.saturating_mul(two_bn))
					};

					let adjusted_per_block = vesting
						.per_block
						.checked_div(&two_balance)
						// Division by constant 2 is safe.
						// unwrap_or_default handles Balance=0 and Balance=1 correctly (result 0).
						.unwrap_or_default();

					if adjusted_per_block.is_zero() && !vesting.per_block.is_zero() {
						log::warn!(
							target: LOG,
							"Vesting schedule per_block reduced to zero due to division. Original: {:?}",
							vesting.per_block
						);
					}
					VestingInfo {
						locked: vesting.locked,
						per_block: adjusted_per_block,
						starting_block: relay_chain_starting_block,
					}
				})
				.collect();

			log::info!(target: LOG, "Vesting schedules migrated: {:?}", migrated);

			EntriesOf::<T>::try_from(migrated).ok()
		};

		log::info!(target: LOG, "Starting linear release vesting time migration to V1");

		crate::Vesting::<T>::translate_values(translate_vesting_info);

		log::info!(target: LOG, "Migrated {} linear release vesting entries", items);

		T::DbWeight::get().reads_writes(items, items)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(pre_state: Vec<u8>) -> Result<(), DispatchError> {
		let (pre_migration_count, pre_vestings): (u32, Vec<((AccountIdOf<T>, ReasonOf<T>), EntriesOf<T>)>) =
			Decode::decode(&mut &pre_state[..]).expect("Failed to decode pre-migration state");

		let post_migration_count = crate::Vesting::<T>::iter().count() as u32;

		ensure!(pre_migration_count == post_migration_count, "Migration count mismatch");

		// Define two_balance for the check
		let two_balance: BalanceOf<T> = 2u32.into();

		for ((account, reason), pre_vesting_schedules) in pre_vestings {
			let post_vesting_schedules =
				crate::Vesting::<T>::get(&account, &reason).expect("Vesting entry should still exist post-migration");

			ensure!(
				pre_vesting_schedules.len() == post_vesting_schedules.len(),
				"Vesting schedule count mismatch for account/reason"
			);

			for (pre_info, post_info) in pre_vesting_schedules.iter().zip(post_vesting_schedules.iter()) {
				// Check starting block changed (if not originally zero)
				// Precise check is hard without knowing migration block numbers.
				if !pre_info.starting_block.is_zero() {
					assert_ne!(
						pre_info.starting_block, post_info.starting_block,
						"Starting block should have been adjusted"
					);
				}
				// Removed the potentially incorrect check: post_info.starting_block <= relay_chain_now

				let expected_post_per_block = pre_info.per_block.checked_div(&two_balance).unwrap_or_default();

				assert_eq!(post_info.per_block, expected_post_per_block, "Per block not adjusted correctly (halved)");

				// Check locked amount remains the same
				assert_eq!(pre_info.locked, post_info.locked, "Locked amount changed during migration");
			}
		}

		log::info!(target: LOG, "Post-upgrade checks passed for linear release migration.");

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
		AccountIdOf, BalanceOf, BlockNumberFor, VestingInfo,
	};
	use frame_support::weights::RuntimeDbWeight;
	use sp_runtime::{bounded_vec, traits::BlockNumberProvider};

	// Helper to calculate expected results concisely
	// Now takes the *actual* polimec_now and the *fixed* relay_chain_now from the mock provider
	fn calculate_expected(
		vesting: &VestingInfo<BalanceOf<Test>, BlockNumberFor<Test>>,
		polimec_now: BlockNumberFor<Test>,
		relay_chain_now: BlockNumberFor<Test>,
	) -> VestingInfo<BalanceOf<Test>, BlockNumberFor<Test>> {
		let two_bn: BlockNumberFor<Test> = 2u32.into();
		let two_balance: BalanceOf<Test> = 2u32.into();

		let expected_relay_start = if polimec_now < vesting.starting_block {
			let blocks_diff = vesting.starting_block.saturating_sub(polimec_now);
			relay_chain_now.saturating_add(blocks_diff.saturating_mul(two_bn))
		} else {
			let blocks_passed = polimec_now.saturating_sub(vesting.starting_block);
			relay_chain_now.saturating_sub(blocks_passed.saturating_mul(two_bn))
		};

		let expected_per_block = vesting.per_block.checked_div(two_balance).unwrap_or_default();

		VestingInfo { locked: vesting.locked, per_block: expected_per_block, starting_block: expected_relay_start }
	}

	#[test]
	fn migration_v1_adjusts_schedules_correctly() {
		ExtBuilder::default().existential_deposit(256).build().execute_with(|| {
			let polimec_now: BlockNumberFor<Test> = 100;
			System::set_block_number(polimec_now);

			let relay_chain_now: BlockNumberFor<Test> = <Test as Config>::BlockNumberProvider::current_block_number();
			assert_eq!(relay_chain_now, polimec_now, "Test assumes relay_chain_now == polimec_now from provider");

			let account1: AccountIdOf<Test> = 3;
			let account2: AccountIdOf<Test> = 4;
			let default_account1: AccountIdOf<Test> = 1;
			let default_account2: AccountIdOf<Test> = 2;
			let default_account12: AccountIdOf<Test> = 12;

			let reason1 = MockRuntimeHoldReason::Reason;
			let reason2 = MockRuntimeHoldReason::Reason2;

			let default_schedules1_pre =
				Vesting::<Test>::get(default_account1, reason1.clone()).expect("Default schedule 1 missing");
			let default_schedules2_pre =
				Vesting::<Test>::get(default_account2, reason1.clone()).expect("Default schedule 2 missing");
			let default_schedules12_pre =
				Vesting::<Test>::get(default_account12, reason1.clone()).expect("Default schedule 12 missing");

			// Test schedules setup (remains the same)
			let v_past = VestingInfo { locked: 1000, per_block: 10, starting_block: 50 };
			// Schedule starting in the future relative to polimec_now
			let v_future = VestingInfo { locked: 2000, per_block: 20, starting_block: 150 };
			// Schedule starting exactly now relative to polimec_now
			let v_now = VestingInfo { locked: 500, per_block: 5, starting_block: polimec_now };
			// Schedule starting at block 0 (edge case)
			let v_zero = VestingInfo { locked: 100, per_block: 1, starting_block: 0 };

			// Entry 1: Acc1, Reason1 -> Multiple schedules, covering past and future
			let schedules1: EntriesOf<Test> = bounded_vec![v_past.clone(), v_future.clone()];
			Vesting::<Test>::insert(account1, reason1.clone(), schedules1.clone());

			// Entry 2: Acc2, Reason1 -> Single schedule, covering 'now' case
			let schedules2: EntriesOf<Test> = bounded_vec![v_now.clone()];
			Vesting::<Test>::insert(account2, reason1.clone(), schedules2.clone());

			// Entry 3: Acc1, Reason2 -> Single schedule, edge case start block
			let schedules3: EntriesOf<Test> = bounded_vec![v_zero.clone()];
			Vesting::<Test>::insert(account1, reason2.clone(), schedules3.clone());

			// Genesis adds 3 entries (1, 2, 12). Test adds 3 more (3/R1, 4/R1, 3/R2). Total = 6.
			let initial_storage_entries = Vesting::<Test>::iter_keys().count();
			assert_eq!(initial_storage_entries, 6, "Check 3 default genesis + 3 added entries");

			// Genesis adds 3 schedules. Test adds 4 more (2+1+1). Total = 7.
			let initial_schedules_count: u64 = Vesting::<Test>::iter_values().map(|v| v.len() as u64).sum();
			assert_eq!(initial_schedules_count, 7, "Check 3 default schedules + 4 added schedules");

			let weight = UncheckedMigrationToV1::<Test>::on_runtime_upgrade();

			// Check counts remain consistent
			assert_eq!(
				Vesting::<Test>::iter_keys().count(),
				initial_storage_entries,
				"Number of storage entries should not change post-migration"
			);
			let post_schedules_count: u64 = Vesting::<Test>::iter_values().map(|v| v.len() as u64).sum();
			assert_eq!(
				post_schedules_count, initial_schedules_count,
				"Number of schedules should not change post-migration"
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
			let expected_zero = calculate_expected(&v_zero, polimec_now, relay_chain_now);
			assert_eq!(expected_zero.per_block, 0, "Expected per_block to become 0 for v_zero");
			assert_eq!(migrated3[0], expected_zero);

			let default_migrated1 = Vesting::<Test>::get(default_account1, reason1.clone()).unwrap();
			assert_eq!(default_migrated1.len(), 1);
			assert_eq!(
				default_migrated1[0],
				// Use the fetched pre-migration state
				calculate_expected(&default_schedules1_pre[0], polimec_now, relay_chain_now),
				"Default schedule 1 migrated incorrectly"
			);

			let default_migrated2 = Vesting::<Test>::get(default_account2, reason1.clone()).unwrap();
			assert_eq!(default_migrated2.len(), 1);
			assert_eq!(
				default_migrated2[0],
				// Use the fetched pre-migration state
				calculate_expected(&default_schedules2_pre[0], polimec_now, relay_chain_now),
				"Default schedule 2 migrated incorrectly"
			);
			let default_migrated12 = Vesting::<Test>::get(default_account12, reason1.clone()).unwrap();
			assert_eq!(default_migrated12.len(), 1);
			assert_eq!(
				default_migrated12[0],
				// Use the fetched pre-migration state
				calculate_expected(&default_schedules12_pre[0], polimec_now, relay_chain_now),
				"Default schedule 12 migrated incorrectly"
			);

			let db_weight: RuntimeDbWeight = <Test as frame_system::Config>::DbWeight::get();
			// Weight based on number of *schedules* processed (initial_schedules_count = 7)
			let expected_weight = db_weight.reads_writes(initial_schedules_count, initial_schedules_count);
			assert_eq!(weight, expected_weight, "Weight should match total schedules processed");
		});
	}
}
