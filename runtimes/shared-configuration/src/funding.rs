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
use polimec_common::{assets::AcceptedFundingAsset, USD_UNIT};
use sp_arithmetic::{FixedU128, Percent};
use sp_runtime::Perquintill;
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};
use xcm::v4::Location;

#[cfg(feature = "instant-mode")]
pub const EVALUATION_ROUND_DURATION: BlockNumber = 7;
#[cfg(feature = "fast-mode")]
pub const EVALUATION_ROUND_DURATION: BlockNumber = 10 * crate::MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const EVALUATION_ROUND_DURATION: BlockNumber = 7 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const AUCTION_ROUND_DURATION: BlockNumber = 7;
#[cfg(feature = "fast-mode")]
pub const AUCTION_ROUND_DURATION: BlockNumber = 30 * crate::MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const AUCTION_ROUND_DURATION: BlockNumber = 7 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const COMMUNITY_ROUND_DURATION: BlockNumber = 5;
#[cfg(feature = "fast-mode")]
pub const COMMUNITY_ROUND_DURATION: BlockNumber = 30 * crate::MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const COMMUNITY_ROUND_DURATION: BlockNumber = 5 * crate::DAYS;

#[cfg(feature = "instant-mode")]
pub const REMAINDER_ROUND_DURATION: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const REMAINDER_ROUND_DURATION: BlockNumber = 15 * crate::MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const REMAINDER_ROUND_DURATION: BlockNumber = 2 * crate::DAYS;

pub type ProjectIdentifier = u32;

parameter_types! {
	pub const EvaluationRoundDuration: BlockNumber = EVALUATION_ROUND_DURATION;
	pub const AuctionRoundDuration: BlockNumber = AUCTION_ROUND_DURATION;
	pub const CommunityRoundDuration: BlockNumber = COMMUNITY_ROUND_DURATION;
	pub const RemainderRoundDuration: BlockNumber = REMAINDER_ROUND_DURATION;
	pub const FundingPalletId: PalletId = PalletId(*b"plmc/fun");
	pub PriceMap: BTreeMap<Location, FixedU128> = BTreeMap::from_iter(vec![
		(AcceptedFundingAsset::DOT.id(), FixedU128::from_rational(69, 1)), // DOT
		(AcceptedFundingAsset::USDC.id(), FixedU128::from_rational(100, 100)), // USDC
		(AcceptedFundingAsset::USDT.id(), FixedU128::from_rational(100, 100)), // USDT
		(Location::here(), FixedU128::from_rational(840, 100)), // PLMC
	]);
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * USD_UNIT),
		(Percent::from_percent(8), 4_000_000 * USD_UNIT),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
	pub FundingSuccessThreshold: Perquintill = Perquintill::from_percent(33);
}
