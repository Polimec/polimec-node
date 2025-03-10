// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
#[allow(clippy::wildcard_imports)]
use crate::{traits::*, *};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Inspect as FungibleInspect, InspectHold as FungibleInspectHold, Mutate as FungibleMutate},
		fungibles::{
			metadata::Inspect as MetadataInspect, roles::Inspect as RolesInspect, Inspect as FungiblesInspect,
			Mutate as FungiblesMutate,
		},
		Get, OnFinalize, OnIdle, OnInitialize,
	},
	weights::Weight,
	Parameter,
};
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use parity_scale_codec::Decode;
use polimec_common::{credentials::InvestorType, migration_types::MigrationOrigin};
use sp_arithmetic::{
	traits::{SaturatedConversion, Saturating, Zero},
	FixedPointNumber, Percent, Perquintill,
};
use sp_runtime::traits::{Convert, Member, One};
use sp_std::{
	cell::RefCell,
	collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	iter::zip,
	marker::PhantomData,
};

#[cfg(any(test, feature = "std", feature = "runtime-benchmarks"))]
use polimec_common_test_utils::generate_did_from_account;

#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub mod macros;

pub mod types;
pub use types::*;

pub mod traits;
pub use traits::*;

#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub mod calculations;

#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub mod chain_interactions;

#[cfg(test)]
mod tests;
