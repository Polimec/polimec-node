// storage_migrations.rs

use crate::Config;
use alloc::vec::Vec;
use frame_support::{pallet_prelude::*, traits::UncheckedOnRuntimeUpgrade, weights::Weight};
use polimec_common::migration_types::{Migration, MigrationInfo, MigrationStatus};
use sp_runtime::{traits::ConstU32, WeakBoundedVec};

pub mod v7 {
	use crate::{AccountIdOf, ProjectId, UserMigrations};

	use super::*;

	const LOG: &str = "funding::migration::v7";
	pub const MAX_PARTICIPATIONS_PER_USER: u32 = 10_000;

	type UserMigrationsKey<T> = (ProjectId, AccountIdOf<T>);
	type UserMigrationsValue = (MigrationStatus, WeakBoundedVec<Migration, ConstU32<MAX_PARTICIPATIONS_PER_USER>>);

	pub struct UncheckedMigrationToV7<T: Config>(PhantomData<T>);
	impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV7<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let migration_count = UserMigrations::<T>::iter().count() as u32;
			log::info!(target: LOG, "Pre-upgrade: {} UserMigrations entries", migration_count);

			// Encode counts and sample pre-migration vesting times for validation
			let user_migrations = UserMigrations::<T>::iter().collect::<Vec<_>>();

			Ok((migration_count, user_migrations).encode())
		}

		fn on_runtime_upgrade() -> Weight {
			let mut items = 0u64;
			log::info!(target: LOG, "Starting vesting time migration to V7");

			let translate_migrations =
				|_: UserMigrationsKey<T>, migration_value: UserMigrationsValue| -> Option<UserMigrationsValue> {
					let (status, migrations) = migration_value;
					let migrated: Vec<_> = migrations
						.iter()
						.map(|migration| {
							items = items.saturating_add(1);

							Migration {
								info: MigrationInfo {
									vesting_time: migration.info.vesting_time.saturating_mul(2),
									..migration.info
								},
								..migration.clone()
							}
						})
						.collect();

					let bounded = WeakBoundedVec::try_from(migrated)
						.unwrap_or_else(|_| panic!("MaxParticipationsPerUser exceeded during migration"));

					Some((status, bounded))
				};

			UserMigrations::<T>::translate(translate_migrations);

			log::info!(target: LOG, "Migrated {} vesting entries", items);
			T::DbWeight::get().reads_writes(items, items)
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(pre_state: Vec<u8>) -> Result<(), DispatchError> {
			let (pre_migration_count, pre_user_migrations): (u32, Vec<(UserMigrationsKey<T>, UserMigrationsValue)>) =
				Decode::decode(&mut &pre_state[..]).expect("Failed to decode pre-migration state");

			let post_migration_count = UserMigrations::<T>::iter().count() as u32;

			if pre_migration_count != post_migration_count {
				return Err("Migration count mismatch".into());
			}

			for (key, pre_value) in pre_user_migrations {
				let post_value = UserMigrations::<T>::get(key.clone())
					.unwrap_or_else(|| panic!("Post-migration UserMigrations entry not found for {:?}", key));

				for (pre_migration, post_migration) in pre_value.1.iter().zip(post_value.1.iter()) {
					if pre_migration.info.vesting_time.saturating_mul(2) != post_migration.info.vesting_time {
						return Err("Migration vesting time mismatch".into());
					}
				}
			}

			Ok(())
		}
	}

	pub type MigrationToV8<T> = frame_support::migrations::VersionedMigration<
		6,
		7,
		UncheckedMigrationToV7<T>,
		crate::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		mock::{new_test_ext, TestRuntime as Test},
		UserMigrations,
	};
	use polimec_common::migration_types::MigrationOrigin;
	use v7::{UncheckedMigrationToV7, MAX_PARTICIPATIONS_PER_USER};
	use xcm::v4::Junction;

	#[test]
	fn migration_to_v7() {
		let mut ext = new_test_ext();
		ext.execute_with(|| {
			assert_eq!(UserMigrations::<Test>::iter().count(), 0);

			let mut migrations = Vec::new();
			for i in 0..MAX_PARTICIPATIONS_PER_USER {
				migrations.push(Migration {
					info: MigrationInfo { vesting_time: i as u64, contribution_token_amount: 1_u128 },
					origin: MigrationOrigin {
						user: Junction::OnlyChild,
						participation_type: polimec_common::migration_types::ParticipationType::Bid,
					},
				});
			}

			let bounded_migrations = WeakBoundedVec::try_from(migrations).unwrap();

			UserMigrations::<Test>::insert((1, 1), (MigrationStatus::Confirmed, bounded_migrations));

			assert_eq!(UserMigrations::<Test>::iter().count(), 1);

			let weight = UncheckedMigrationToV7::<Test>::on_runtime_upgrade();
			assert_eq!(UserMigrations::<Test>::iter().count(), 1);

			for (_, (_, migrations)) in UserMigrations::<Test>::iter() {
				for (i, migration) in migrations.iter().enumerate() {
					assert_eq!(migration.info.vesting_time, i as u64 * 2);
				}
			}

			assert!(weight.is_zero());
		});
	}
}
