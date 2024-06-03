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

use crate::{Balance, BlockNumber};
use frame_support::{parameter_types, PalletId};
use pallet_funding::types::AcceptedFundingAsset;
use parachains_common::AssetIdForTrustBackedAssets;
use polimec_common::USD_UNIT;
use sp_arithmetic::{FixedU128, Percent};
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

#[cfg(feature = "fast-mode")]
use parachains_common::HOURS;

#[cfg(feature = "instant-mode")]
pub const EVALUATION_DURATION: BlockNumber = 3;
#[cfg(feature = "fast-mode")]
pub const EVALUATION_DURATION: BlockNumber = 3 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const EVALUATION_DURATION: BlockNumber = 28 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 3;
#[cfg(feature = "fast-mode")]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 7 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const AUCTION_OPENING_DURATION: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const AUCTION_OPENING_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const AUCTION_OPENING_DURATION: BlockNumber = 2 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const AUCTION_CLOSING_DURATION: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const AUCTION_CLOSING_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const AUCTION_CLOSING_DURATION: BlockNumber = 3 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 3;
#[cfg(feature = "fast-mode")]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 5 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = 3;
#[cfg(feature = "fast-mode")]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 5;
#[cfg(feature = "fast-mode")]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 365 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const MANUAL_ACCEPTANCE_DURATION: BlockNumber = 3;
#[cfg(feature = "fast-mode")]
pub const MANUAL_ACCEPTANCE_DURATION: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const MANUAL_ACCEPTANCE_DURATION: BlockNumber = 3 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const SUCCESS_TO_SETTLEMENT_TIME: BlockNumber = 4;
#[cfg(feature = "fast-mode")]
pub const SUCCESS_TO_SETTLEMENT_TIME: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const SUCCESS_TO_SETTLEMENT_TIME: BlockNumber = 4 * crate::DAYS;

pub type ProjectIdentifier = u32;

parameter_types! {
	pub const EvaluationDuration: BlockNumber = EVALUATION_DURATION;
	pub const AuctionInitializePeriodDuration: BlockNumber = AUCTION_INITIALIZE_PERIOD_DURATION;
	pub const AuctionOpeningDuration: BlockNumber = AUCTION_OPENING_DURATION;
	pub const AuctionClosingDuration: BlockNumber = AUCTION_CLOSING_DURATION;
	pub const CommunityFundingDuration: BlockNumber = COMMUNITY_FUNDING_DURATION;
	pub const RemainderFundingDuration: BlockNumber = REMAINDER_FUNDING_DURATION;
	pub const ManualAcceptanceDuration: BlockNumber = MANUAL_ACCEPTANCE_DURATION;
	pub const SuccessToSettlementTime: BlockNumber = SUCCESS_TO_SETTLEMENT_TIME;
	pub const FundingPalletId: PalletId = PalletId(*b"plmc/fun");
	pub PriceMap: BTreeMap<AssetIdForTrustBackedAssets, FixedU128> = BTreeMap::from_iter(vec![
		(AcceptedFundingAsset::DOT.to_assethub_id(), FixedU128::from_rational(69, 1)), // DOT
		(AcceptedFundingAsset::USDC.to_assethub_id(), FixedU128::from_rational(100, 100)), // USDC
		(AcceptedFundingAsset::USDT.to_assethub_id(), FixedU128::from_rational(100, 100)), // USDT
		(pallet_funding::PLMC_FOREIGN_ID, FixedU128::from_rational(840, 100)), // PLMC
	]);
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * USD_UNIT),
		(Percent::from_percent(8), 4_000_000 * USD_UNIT),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
}
