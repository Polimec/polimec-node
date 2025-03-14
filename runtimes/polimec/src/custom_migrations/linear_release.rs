use crate::{Balance, BlockNumber, Runtime, RuntimeHoldReason};
use alloc::vec::Vec;
use frame_support::{storage::storage_prefix, traits::OnRuntimeUpgrade, BoundedVec};
use pallet_linear_release::{MaxVestingSchedulesGet, VestingInfo};

#[cfg(feature = "try-runtime")]
use frame_support::{ensure, migrations::VersionedPostUpgradeData, traits::GetStorageVersion};
#[cfg(feature = "try-runtime")]
use itertools::Itertools;
#[cfg(feature = "try-runtime")]
use parity_scale_codec::{DecodeAll, Encode};

pub type Values = BoundedVec<VestingInfo<Balance, BlockNumber>, MaxVestingSchedulesGet<Runtime>>;

pub struct LinearReleaseVestingMigration;
impl OnRuntimeUpgrade for LinearReleaseVestingMigration {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		use crate::LinearRelease;

		let funding_on_chain_version = LinearRelease::on_chain_storage_version();
		if funding_on_chain_version == 0 {
			let storage = pallet_linear_release::Vesting::<Runtime>::iter().collect_vec();
			ensure!(storage.len() == 15, "LinearReleaseVestingMigration: Invalid storage length in pre_upgrade");
			Ok(VersionedPostUpgradeData::MigrationExecuted(Vec::new()).encode())
		} else {
			Ok(VersionedPostUpgradeData::Noop.encode())
		}
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();

		// Step 1: Get account count for weight calculation
		let account_count = pallet_linear_release::Vesting::<Runtime>::iter().count();

		// Initial weight: account_count reads + 1 clear_prefix operation
		let mut total_weight = db_weight.reads_writes(account_count as u64, 1); // For clear_prefix

		// Step 2: Collect all accounts and their hold amounts
		let mut account_holds = Vec::with_capacity(account_count);

		for (account, reason, vesting_info) in pallet_linear_release::Vesting::<Runtime>::iter() {
			if !vesting_info.is_empty() {
				log::info!(
					"Found vesting for account: {:?}, reason: {:?}, schedule count: {:?}",
					account,
					reason,
					vesting_info.len()
				);
				account_holds.push((account, reason, vesting_info[0]));
			} else {
				log::warn!("Empty vesting info found for account: {:?}, reason: {:?}", account, reason);
			}
		}

		// Step 3: Clear all corrupted vesting entries
		let pallet_prefix = storage_prefix(b"LinearRelease", b"Vesting");
		let removed_keys = frame_support::storage::unhashed::clear_prefix(&pallet_prefix, None, None);
		log::info!("Cleared {:#?} vesting storage keys", removed_keys.deconstruct());

		// Step 4: Create fresh vesting entries for all accounts with holds
		let mut success_count = 0u64;
		let mut failure_count = 0u64;

		for (account, _corrupted_reason, vesting_info) in account_holds {
			// Create a BoundedVec with this schedule - reuse original VestingInfo value, but set the k2 to the new pallet_funding HoldReason::Participation.
			let mut schedules = BoundedVec::<_, MaxVestingSchedulesGet<Runtime>>::default();

			match schedules.try_push(vesting_info) {
				Ok(_) => {
					pallet_linear_release::Vesting::<Runtime>::insert(
						&account,
						&RuntimeHoldReason::Funding(pallet_funding::HoldReason::Participation),
						schedules,
					);
					success_count += 1;
					total_weight = total_weight.saturating_add(db_weight.writes(1));
				},
				Err(_) => {
					log::error!("Failed to add vesting schedule to BoundedVec for account {:?}", account);
					failure_count += 1;
				},
			}
		}

		log::info!(
			"Migration complete. Successfully created {} new vesting entries. Failed: {}",
			success_count,
			failure_count
		);

		total_weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(versioned_post_upgrade_data_bytes: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let storage = pallet_linear_release::Vesting::<Runtime>::iter().collect_vec();
		ensure!(storage.len() == 15, "LinearReleaseVestingMigration: Invalid storage length in post_upgrade");

		match <VersionedPostUpgradeData>::decode_all(&mut &versioned_post_upgrade_data_bytes[..])
			.map_err(|_| "VersionedMigration post_upgrade failed to decode PreUpgradeData")?
		{
			VersionedPostUpgradeData::MigrationExecuted(_inner_bytes) => Ok(()),
			VersionedPostUpgradeData::Noop => Ok(()),
		}
	}
}
