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

use crate::{BlockNumber, DAYS};
use frame_support::{parameter_types, PalletId};

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

parameter_types! {
	pub const EvaluationDuration: BlockNumber = EVALUATION_DURATION;
	pub const AuctionInitializePeriodDuration: BlockNumber = AUCTION_INITIALIZE_PERIOD_DURATION;
	pub const EnglishAuctionDuration: BlockNumber = ENGLISH_AUCTION_DURATION;
	pub const CandleAuctionDuration: BlockNumber = CANDLE_AUCTION_DURATION;
	pub const CommunityFundingDuration: BlockNumber = COMMUNITY_FUNDING_DURATION;
	pub const RemainderFundingDuration: BlockNumber = REMAINDER_FUNDING_DURATION;
	pub const ContributionVestingDuration: BlockNumber = CONTRIBUTION_VESTING_DURATION;
	pub const FundingPalletId: PalletId = PalletId(*b"py/cfund");
}
