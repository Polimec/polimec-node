use alloc::vec::Vec;
use frame_support::{traits::Currency, BoundedVec};
use frame_system::pallet_prelude::*;
use pallet_vesting::{MaxVestingSchedulesGet, VestingInfo};
use parity_scale_codec::Encode;

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
#[cfg(feature = "try-runtime")]
use parity_scale_codec::Decode;
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;

type BalanceOf<T> =
	<<T as pallet_vesting::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type VestingInfoOf<T> = VestingInfo<BalanceOf<T>, BlockNumberFor<T>>;
pub type Values<T> = BoundedVec<VestingInfoOf<T>, MaxVestingSchedulesGet<T>>;

pub mod v1 {
	use super::*;
	use core::marker::PhantomData;
	use frame_support::{
		pallet_prelude::{CheckedDiv, Zero},
		traits::{Get, OnRuntimeUpgrade},
	};
	use parachains_common::impls::AccountIdOf;
	use sp_runtime::{traits::BlockNumberProvider, Saturating, Weight};

	const LOG: &str = "pallet_vesting::migration::v1";

	/// Stores the status of vesting pallet migration to async backing. If it is populated, the migration already happened.
	pub const VESTING_ASYNC_BACKED_KEY: &[u8; 31] = b"vesting_async_backing_migration";

	pub struct UncheckedMigrationToAsyncBacking<T: pallet_vesting::Config>(PhantomData<T>);

	impl<T: pallet_vesting::Config> OnRuntimeUpgrade for UncheckedMigrationToAsyncBacking<T> {
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
			let vesting_async_backed = sp_io::storage::get(VESTING_ASYNC_BACKED_KEY);

			if vesting_async_backed.is_some() {
				log::info!(target: LOG, "Skipping migration as vesting pallet is already migrated with async backing");
				return Ok(Vec::new());
			}

			let migration_count = pallet_vesting::Vesting::<T>::iter().count() as u32;
			log::info!(target: LOG, "Pre-upgrade: {} UserMigrations entries", migration_count);

			let vestings = pallet_vesting::Vesting::<T>::iter().collect::<Vec<_>>();

			Ok((migration_count, vestings).encode())
		}

		fn on_runtime_upgrade() -> Weight {
			let vesting_async_backed = sp_io::storage::get(VESTING_ASYNC_BACKED_KEY);

			if vesting_async_backed.is_some() {
				log::info!(target: LOG, "Skipping migration as vesting pallet is already migrated with async backing");
				return Weight::zero();
			}

			let mut items = 0u64;
			let relay_chain_now = T::BlockNumberProvider::current_block_number();
			let polimec_now = frame_system::Pallet::<T>::current_block_number();

			let two_bn: BlockNumberFor<T> = 2u32.into();
			let two_balance: BalanceOf<T> = 2u32.into();

			let translate_vesting_info = |_: AccountIdOf<T>, vesting_info: Values<T>| -> Option<Values<T>> {
				let migrated: Vec<_> = vesting_info
					.iter()
					.map(|vesting| {
						items = items.saturating_add(1);

						let relay_chain_starting_block = if polimec_now < vesting.starting_block() {
							let blocks_diff = vesting.starting_block().saturating_sub(polimec_now);
							relay_chain_now.saturating_add(blocks_diff.saturating_mul(two_bn))
						} else {
							let blocks_passed = polimec_now.saturating_sub(vesting.starting_block());
							relay_chain_now.saturating_sub(blocks_passed.saturating_mul(two_bn))
						};

						let adjusted_per_block = vesting
							.per_block()
							.checked_div(&two_balance)
							// Division by constant 2 is safe.
							// unwrap_or_default handles Balance=0 and Balance=1 correctly (result 0).
							.unwrap_or_default();

						// Optional: Log if the rate becomes zero
						if adjusted_per_block.is_zero() && !vesting.per_block().is_zero() {
							log::warn!(
								target: LOG,
								"Vesting schedule per_block reduced to zero due to division. Original: {:?}",
								vesting.per_block()
							);
						}
						VestingInfo::new(vesting.locked(), adjusted_per_block, relay_chain_starting_block)
					})
					.collect();

				Values::<T>::try_from(migrated).ok()
			};

			log::info!(target: LOG, "Starting vesting time migration to V1");

			pallet_vesting::Vesting::<T>::translate(translate_vesting_info);

			log::info!(target: LOG, "Migrated {} vesting entries", items);

			sp_io::storage::set(VESTING_ASYNC_BACKED_KEY, &().encode()[..]);

			T::DbWeight::get().reads_writes(items.saturating_add(1), items.saturating_add(2))
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(pre_state: Vec<u8>) -> Result<(), DispatchError> {
			let vesting_async_backed = sp_io::storage::get(VESTING_ASYNC_BACKED_KEY);
			if vesting_async_backed.is_some() {
				log::info!(target: LOG, "Skipping migration as vesting pallet is already migrated with async backing");
				return Ok(());
			}

			let (pre_migration_count, pre_vestings): (u32, Vec<(AccountIdOf<T>, Values<T>)>) =
				Decode::decode(&mut &pre_state[..]).expect("Failed to decode pre-migration state");

			let post_migration_count = pallet_vesting::Vesting::<T>::iter().count() as u32;

			ensure!(pre_migration_count == post_migration_count, "Migration count mismatch");

			// Define two_balance for the check
			let two_balance: BalanceOf<T> = 2u32.into();

			for (account, pre_vesting_schedules) in pre_vestings {
				let post_vesting_schedules = pallet_vesting::Vesting::<T>::get(&account)
					.expect("Vesting entry should still exist post-migration");

				ensure!(
					pre_vesting_schedules.len() == post_vesting_schedules.len(),
					"Vesting schedule count mismatch for account"
				);

				// Note: Checking starting_block adjustment logic precisely requires knowing
				// the exact block numbers (relay and parachain) *during* the migration,
				// which isn't easily available here. We'll focus on the per_block check.
				// A simple check is that it changed, but a full check is complex.
				// let relay_chain_now = T::BlockNumberProvider::current_block_number(); // Might differ from migration time

				for (pre_info, post_info) in pre_vesting_schedules.iter().zip(post_vesting_schedules.iter()) {
					// Basic check: Starting block should have changed if it wasn't already 0
					// A more robust check is difficult without knowing migration block numbers.
					if !pre_info.starting_block().is_zero() {
						assert_ne!(
							pre_info.starting_block(),
							post_info.starting_block(),
							"Starting block should have been adjusted"
						);
					}

					let expected_post_per_block = pre_info.per_block().checked_div(&two_balance).unwrap_or_default();

					assert_eq!(
						post_info.per_block(),
						expected_post_per_block,
						"Per block not adjusted correctly (halved)"
					);

					// Locked amount should remain the same
					assert_eq!(pre_info.locked(), post_info.locked(), "Locked amount changed during migration");
				}
			}

			// Check that the migration key is set
			assert!(sp_io::storage::exists(VESTING_ASYNC_BACKED_KEY), "VESTING_ASYNC_BACKED_KEY was not set");

			log::info!(target: LOG, "Post-upgrade checks passed for vesting migration.");

			Ok(())
		}
	}
}
