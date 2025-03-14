//! Migrations

use frame_support::traits::StorageVersion;

pub mod storage_migrations;
pub mod vesting_info;

/// Current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(7);
