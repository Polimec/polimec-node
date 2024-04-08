//! A module that is responsible for migration of storage.

use crate::{
	types::{HRMPChannelStatus, MigrationReadinessCheck, PhaseTransitionPoints, ProjectStatus},
	AccountIdOf, BalanceOf, BlockNumberFor, Config, Did, EvaluationRoundInfoOf, Pallet, PriceOf, ProjectId,
};
use frame_support::{
	pallet_prelude::*,
	traits::{tokens::Balance as BalanceT, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use parity_scale_codec::{Decode, Encode};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_arithmetic::FixedPointNumber;
use sp_std::marker::PhantomData;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
pub const LOG: &str = "runtime::funding::migration";
