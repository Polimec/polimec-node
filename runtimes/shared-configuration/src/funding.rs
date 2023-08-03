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

use crate::{currency::US_DOLLAR, Balance, BlockNumber, DAYS};
use frame_support::{parameter_types, PalletId};
use parachains_common::AssetIdForTrustBackedAssets;
use sp_arithmetic::{FixedU128, Percent};
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};

#[cfg(feature = "fast-gov")]
pub const EVALUATION_DURATION: BlockNumber = 28;
#[cfg(not(feature = "fast-gov"))]
pub const EVALUATION_DURATION: BlockNumber = 28 * DAYS;

#[cfg(feature = "fast-gov")]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 7;
#[cfg(not(feature = "fast-gov"))]
pub const AUCTION_INITIALIZE_PERIOD_DURATION: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const ENGLISH_AUCTION_DURATION: BlockNumber = 2 * DAYS;

#[cfg(feature = "fast-gov")]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 5;
#[cfg(not(feature = "fast-gov"))]
pub const CANDLE_AUCTION_DURATION: BlockNumber = 3 * DAYS;

#[cfg(feature = "fast-gov")]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const COMMUNITY_FUNDING_DURATION: BlockNumber = 5 * DAYS;

#[cfg(feature = "fast-gov")]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = 10;
#[cfg(not(feature = "fast-gov"))]
pub const REMAINDER_FUNDING_DURATION: BlockNumber = DAYS;

#[cfg(feature = "fast-gov")]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 365;
#[cfg(not(feature = "fast-gov"))]
pub const CONTRIBUTION_VESTING_DURATION: BlockNumber = 365 * DAYS;

#[cfg(feature = "fast-gov")]
pub const MANUAL_ACCEPTANCE_DURATION: BlockNumber = 3;
#[cfg(not(feature = "fast-gov"))]
pub const MANUAL_ACCEPTANCE_DURATION: BlockNumber = 3 * DAYS;

#[cfg(feature = "fast-gov")]
pub const SUCCESS_TO_SETTLEMENT_TIME: BlockNumber = 4;
#[cfg(not(feature = "fast-gov"))]
pub const SUCCESS_TO_SETTLEMENT_TIME: BlockNumber = 4 * DAYS;

parameter_types! {
	pub const EvaluationDuration: BlockNumber = EVALUATION_DURATION;
	pub const AuctionInitializePeriodDuration: BlockNumber = AUCTION_INITIALIZE_PERIOD_DURATION;
	pub const EnglishAuctionDuration: BlockNumber = ENGLISH_AUCTION_DURATION;
	pub const CandleAuctionDuration: BlockNumber = CANDLE_AUCTION_DURATION;
	pub const CommunityFundingDuration: BlockNumber = COMMUNITY_FUNDING_DURATION;
	pub const RemainderFundingDuration: BlockNumber = REMAINDER_FUNDING_DURATION;
	pub const ContributionVestingDuration: BlockNumber = CONTRIBUTION_VESTING_DURATION;
	pub const ManualAcceptanceDuration: BlockNumber = MANUAL_ACCEPTANCE_DURATION;
	pub const SuccessToSettlementTime: BlockNumber = SUCCESS_TO_SETTLEMENT_TIME;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
	pub PriceMap: BTreeMap<AssetIdForTrustBackedAssets, FixedU128> = BTreeMap::from_iter(vec![
		(0u32, FixedU128::from_rational(69, 1)), // DOT
		(420u32, FixedU128::from_rational(97, 100)), // USDC
		(1984u32, FixedU128::from_rational(95, 100)), // USDT
		(2069u32, FixedU128::from_rational(840, 100)), // PLMC
	]);
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * US_DOLLAR),
		(Percent::from_percent(8), 5_000_000 * US_DOLLAR),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
}
