#[allow(clippy::wildcard_imports)]
use super::{traits::*, *};
use core::ops::Not;
use frame_support::{
	dispatch::{DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::{Mutate, MutateHold as FungibleMutateHold},
		fungibles::{
			metadata::{Inspect as MetadataInspect, Mutate as MetadataMutate},
			Create, Mutate as FungiblesMutate,
		},
		tokens::{Precision, Preservation},
		Get,
	},
	transactional,
};
use frame_system::pallet_prelude::BlockNumberFor;
use polimec_common::{
	credentials::{Did, InvestorType},
	migration_types::{MigrationInfo, Migrations},
	USD_DECIMALS,
};
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_runtime::traits::Convert;

const POLIMEC_PARA_ID: u32 = 3344u32;
#[path = "1_application.rs"]
mod application;
#[path = "3_auction.rs"]
mod auction;
#[path = "6_ct_migration.rs"]
mod ct_migration;
#[path = "2_evaluation.rs"]
mod evaluation;
#[path = "4_funding_end.rs"]
mod funding_end;
pub mod misc;
#[path = "5_settlement.rs"]
mod settlement;

pub mod runtime_api;
