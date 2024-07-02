use super::*;

use crate::traits::{BondingRequirementCalculation, ProvideAssetPrice, VestingDurationCalculation};
use core::ops::Not;
use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::{Mutate, MutateHold as FungibleMutateHold},
		fungibles::{
			metadata::{Inspect as MetadataInspect, Mutate as MetadataMutate},
			Create, Inspect as FungibleInspect, Mutate as FungiblesMutate,
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
use sp_arithmetic::{
	traits::{CheckedDiv, CheckedSub, Zero},
	Percent, Perquintill,
};
use sp_runtime::traits::Convert;

const POLIMEC_PARA_ID: u32 = 3344u32;
const QUERY_RESPONSE_TIME_WINDOW_BLOCKS: u32 = 20u32;
#[path = "1_application.rs"]
mod application;
#[path = "3_auction.rs"]
mod auction;
#[path = "4_contribution.rs"]
mod contribution;
#[path = "7_ct_migration.rs"]
mod ct_migration;
#[path = "2_evaluation.rs"]
mod evaluation;
#[path = "5_funding_end.rs"]
mod funding_end;
mod misc;
#[path = "6_settlement.rs"]
mod settlement;
