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

use crate::{
	currency::{deposit, PLMC},
	Balance,
};
use frame_support::{parameter_types, PalletId};
use parachains_common::BlockNumber;
#[cfg(feature = "fast-gov")]
use parachains_common::MINUTES;
#[cfg(not(feature = "fast-gov"))]
use parachains_common::{DAYS, HOURS};
use sp_arithmetic::Permill;

pub const MIN_DEPOSIT: Balance = PLMC;

#[cfg(feature = "fast-gov")]
pub const LAUNCH_PERIOD: BlockNumber = 7 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const LAUNCH_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const VOTING_PERIOD: BlockNumber = 7 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const VOTING_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 3 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 3 * HOURS;

#[cfg(feature = "fast-gov")]
pub const ENACTMENT_PERIOD: BlockNumber = 8 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const ENACTMENT_PERIOD: BlockNumber = DAYS;

#[cfg(feature = "fast-gov")]
pub const COOLOFF_PERIOD: BlockNumber = 7 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const COOLOFF_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "fast-gov")]
pub const SPEND_PERIOD: BlockNumber = 6 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const SPEND_PERIOD: BlockNumber = 6 * DAYS;

#[cfg(feature = "fast-gov")]
pub const ROTATION_PERIOD: BlockNumber = 80 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const ROTATION_PERIOD: BlockNumber = 80 * HOURS;

#[cfg(feature = "fast-gov")]
pub const TERM_DURATION: BlockNumber = 15 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const TERM_DURATION: BlockNumber = DAYS;

#[cfg(feature = "fast-gov")]
pub const COUNCIL_MOTION_DURATION: BlockNumber = 4 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const COUNCIL_MOTION_DURATION: BlockNumber = 3 * DAYS;

#[cfg(feature = "fast-gov")]
pub const TECHNICAL_MOTION_DURATION: BlockNumber = 4 * MINUTES;
#[cfg(not(feature = "fast-gov"))]
pub const TECHNICAL_MOTION_DURATION: BlockNumber = 3 * DAYS;

parameter_types! {
	// Democracy Pallet
	pub const LaunchPeriod: BlockNumber = LAUNCH_PERIOD;
	pub const VotingPeriod: BlockNumber = VOTING_PERIOD;
	pub const FastTrackVotingPeriod: BlockNumber = FAST_TRACK_VOTING_PERIOD;
	pub const MinimumDeposit: Balance = MIN_DEPOSIT;
	pub const EnactmentPeriod: BlockNumber = ENACTMENT_PERIOD;
	pub const CooloffPeriod: BlockNumber = COOLOFF_PERIOD;
	// Council Pallet
	pub const CouncilMotionDuration: BlockNumber = COUNCIL_MOTION_DURATION;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
	// Technical Committee
	pub const TechnicalMotionDuration: BlockNumber = TECHNICAL_MOTION_DURATION;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
	// Tipper Group
	pub const TipperMaxMembers: u32 = 21;
	// Extras
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const MaxProposals: u32 = 100;
	//Treasury
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 20 * PLMC;
	pub const SpendPeriod: BlockNumber = SPEND_PERIOD;
	pub const Burn: Permill = Permill::zero();
	pub const MaxApprovals: u32 = 100;
	pub const TreasuryId: PalletId = PalletId(*b"plmc/tsy");
}
