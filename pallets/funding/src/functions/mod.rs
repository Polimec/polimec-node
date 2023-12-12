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

// If you feel like getting in touch with us, you can do so at info@polimec.org

use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::MutateHold as FungibleMutateHold,
		fungibles::{metadata::Mutate as MetadataMutate, Create, Inspect, Mutate as FungiblesMutate},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
};
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use sp_arithmetic::{
	traits::{CheckedDiv, CheckedSub, Zero},
	Percent, Perquintill,
};
use sp_runtime::traits::{Convert, ConvertBack};
use sp_std::marker::PhantomData;
use xcm::v3::MaxDispatchErrorLen;

use crate::ProjectStatus::FundingSuccessful;
use polimec_traits::ReleaseSchedule;

use crate::traits::{BondingRequirementCalculation, ProvideStatemintPrice, VestingDurationCalculation};
use polimec_traits::migration_types::{MigrationInfo, MigrationOrigin, Migrations, ParticipationType};

use super::*;
const POLIMEC_PARA_ID: u32 = 3344u32;
const QUERY_RESPONSE_TIME_WINDOW_BLOCKS: u32 = 20u32;

// Create project
pub mod create_project;

// Evaluation Round
pub mod evaluation;

// Auction Round
pub mod auction;

// Contribution Round
pub mod contribution;

// Project Decision
pub mod end_funding;

// Settlement
pub mod settlement;

// Migration
pub mod migration;

// Helpers
pub mod helper;