//! A module that is responsible for migration of storage.
use crate::{
	AccountIdOf, BiddingTicketSizes, Config, CurrencyMetadata, FixedPointNumber, ParticipantsAccountType, PriceOf,
	ProjectMetadataOf, StringLimitOf,
};
use core::marker::PhantomData;
use frame_support::traits::{StorageVersion, UncheckedOnRuntimeUpgrade};
use polimec_common::{assets::AcceptedFundingAsset, credentials::Cid};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, Decode, Encode, Get, MaxEncodedLen, RuntimeDebug};
use sp_runtime::{BoundedVec, Percent};
extern crate alloc;
use alloc::vec::Vec;
use polimec_common::migration_types::{MigrationInfo, ParticipationType};
use xcm::v4::Location;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(7);
pub const LOG: &str = "runtime::funding::migration";
