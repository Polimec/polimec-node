use alloc::vec::Vec;
use frame_support::{traits::Currency, BoundedVec};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_vesting::{MaxVestingSchedulesGet, VestingInfo};
use parity_scale_codec::Encode;

#[cfg(feature = "try-runtime")]
use parity_scale_codec::Decode;
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;

type VestingInfoOf<T> = VestingInfo<
	<<T as pallet_vesting::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance,
	BlockNumberFor<T>,
>;
pub type Values<T> = BoundedVec<VestingInfoOf<T>, MaxVestingSchedulesGet<T>>;

pub mod v1 {
	use super::*;
	use core::marker::PhantomData;
	use frame_support::traits::OnRuntimeUpgrade;
	use parachains_common::impls::AccountIdOf;
	use sp_core::Get;
	use sp_runtime::{traits::BlockNumberProvider, Saturating, Weight};

	const LOG: &str = "pallet_vesting::migration::v1";

	/// Stores the status of vesting pallet migration to async backing. If it is populated, the migration already happened.
	pub const VESTING_ASYNC_BACKED_KEY: &[u8; 31] = b"vesting_async_backing_migration";

	pub struct UncheckedMigrationToV1<T: pallet_vesting::Config>(PhantomData<T>);

	impl<T: pallet_vesting::Config> OnRuntimeUpgrade for UncheckedMigrationToV1<T> {
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
			let translate_vesting_info = |_: AccountIdOf<T>, vesting_info: Values<T>| -> Option<Values<T>> {
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

						let adjusted_per_block = vesting.per_block().saturating_mul(2_u32.into());

						VestingInfo::new(vesting.locked(), adjusted_per_block, relay_chain_starting_block)
					})
					.collect();

				Values::<T>::try_from(migrated).ok()
			};

			log::info!(target: LOG, "Starting vesting time migration to V1");

			pallet_vesting::Vesting::<T>::translate(translate_vesting_info);

			log::info!(target: LOG, "Migrated {} vesting entries", items);

			sp_io::storage::set(VESTING_ASYNC_BACKED_KEY, &().encode()[..]);

			T::DbWeight::get().reads_writes(items, items)
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

			if pre_migration_count != post_migration_count {
				return Err("Migration count mismatch".into());
			}

			for (account, pre_vesting) in pre_vestings {
				let post_vesting = pallet_vesting::Vesting::<T>::get(&account).unwrap_or_default();

				// check that the starting block has been adjusted
				let relay_chain_now = T::BlockNumberProvider::current_block_number();

				for (pre_vesting_info, post_vesting_info) in pre_vesting.iter().zip(post_vesting.iter()) {
					assert_ne!(
						pre_vesting_info.starting_block(),
						post_vesting_info.starting_block(),
						"Starting block not adjusted"
					);
					assert!(
						post_vesting_info.starting_block() <=
							relay_chain_now.try_into().ok().expect("safe to convert; qed"),
						"Starting block not adjusted correctly"
					);

					assert!(
						post_vesting_info.per_block() ==
							pre_vesting_info
								.per_block()
								.saturating_mul(2_u32.try_into().ok().expect("safe to convert; qed")),
						"Per block not adjusted"
					);
				}
			}

			Ok(())
		}
	}
}
