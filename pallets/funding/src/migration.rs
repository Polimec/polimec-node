//! A module that is responsible for migration of storage.
use frame_support::traits::StorageVersion;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
pub const LOG: &str = "runtime::funding::migration";
