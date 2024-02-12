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
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
use parachains_common::{DAYS, HOURS};
#[cfg(feature = "fast-mode")]
use parachains_common::{HOURS, MINUTES};
use sp_arithmetic::Permill;

#[cfg(feature = "instant-mode")]
pub const LAUNCH_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const LAUNCH_PERIOD: BlockNumber = 15 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const LAUNCH_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const VOTING_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const VOTING_PERIOD: BlockNumber = 30 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const VOTING_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 20 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const FAST_TRACK_VOTING_PERIOD: BlockNumber = 3 * HOURS;

#[cfg(feature = "instant-mode")]
pub const ENACTMENT_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const ENACTMENT_PERIOD: BlockNumber = 10 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const ENACTMENT_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const COOLOFF_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const COOLOFF_PERIOD: BlockNumber = 15 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const COOLOFF_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const SPEND_PERIOD: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const SPEND_PERIOD: BlockNumber = 1 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const SPEND_PERIOD: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const TERM_DURATION: BlockNumber = 5;
#[cfg(feature = "fast-mode")]
pub const TERM_DURATION: BlockNumber = 4 * HOURS;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const TERM_DURATION: BlockNumber = 28 * DAYS;

#[cfg(feature = "instant-mode")]
pub const ELECTION_VOTING_LOCK_DURATION: BlockNumber = 5;
#[cfg(feature = "fast-mode")]
pub const ELECTION_VOTING_LOCK_DURATION: BlockNumber = 30 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const ELECTION_VOTING_LOCK_DURATION: BlockNumber = 28 * DAYS;

#[cfg(feature = "instant-mode")]
pub const COUNCIL_MOTION_DURATION: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const COUNCIL_MOTION_DURATION: BlockNumber = 30 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const COUNCIL_MOTION_DURATION: BlockNumber = 7 * DAYS;

#[cfg(feature = "instant-mode")]
pub const TECHNICAL_MOTION_DURATION: BlockNumber = 2;
#[cfg(feature = "fast-mode")]
pub const TECHNICAL_MOTION_DURATION: BlockNumber = 30 * MINUTES;
#[cfg(not(any(feature = "fast-mode", feature = "instant-mode")))]
pub const TECHNICAL_MOTION_DURATION: BlockNumber = 7 * DAYS;

parameter_types! {
	// Democracy Pallet
	pub const LaunchPeriod: BlockNumber = LAUNCH_PERIOD;
	pub const VotingPeriod: BlockNumber = VOTING_PERIOD;
	pub const FastTrackVotingPeriod: BlockNumber = FAST_TRACK_VOTING_PERIOD;
	pub const MinimumDeposit: Balance = 100 * PLMC;
	pub const EnactmentPeriod: BlockNumber = ENACTMENT_PERIOD;
	pub const CooloffPeriod: BlockNumber = COOLOFF_PERIOD;

	// Council Pallet
	pub const CouncilMotionDuration: BlockNumber = COUNCIL_MOTION_DURATION;
	pub const CouncilMaxProposals: u32 = 7;
	pub const CouncilMaxMembers: u32 = 20;

	// Technical Committee
	pub const TechnicalMotionDuration: BlockNumber = TECHNICAL_MOTION_DURATION;
	pub const TechnicalMaxProposals: u32 = 7;
	pub const TechnicalMaxMembers: u32 = 5;

	// Extras
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const MaxProposals: u32 = 10;
	pub const MaxVotes: u32 = 100;
	pub const MaxBlacklisted: u32 = 100;
	pub const MaxDeposits: u32 = 100;

	//Treasury
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 50 * PLMC;
	pub const SpendPeriod: BlockNumber = SPEND_PERIOD;
	pub const Burn: Permill = Permill::zero();
	pub const MaxApprovals: u32 = 100;
	pub const TreasuryId: PalletId = PalletId(*b"plmc/tsy");

	// Elections phragmen
	pub const CandidacyBond: Balance = 1000 * PLMC;
	pub TermDuration: BlockNumber = TERM_DURATION;
	pub VotingLockPeriod: BlockNumber = ELECTION_VOTING_LOCK_DURATION;
	pub const DesiredMembers: u32 = 9;
	pub const DesiredRunnersUp: u32 = 20;
	pub const MaxCandidates: u32 = 1000;
	pub const MaxVoters: u32 = 10000;
	pub const MaxVotesPerVoter: u32 = 8;
}
