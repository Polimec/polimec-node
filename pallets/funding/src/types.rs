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

//! Types for Funding pallet.

use frame_support::{pallet_prelude::*, traits::tokens::Balance as BalanceT};

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Project<BoundedString, Balance: BalanceT, Hash> {
	/// Token Metadata
	pub token_information: CurrencyMetadata<BoundedString>,
	/// Total allocation of Contribution Tokens available for the Funding Round
	pub total_allocation_size: Balance,
	/// Minimum price per Contribution Token
	pub minimum_price: Balance,
	/// Maximum and/or minimum ticket size
	pub ticket_size: TicketSize<Balance>,
	/// Maximum and/or minimum number of participants for the Auction and Community Round
	pub participants_size: ParticipantsSize,
	/// Funding round thresholds for Retail, Professional and Institutional participants
	pub funding_thresholds: Thresholds,
	/// Conversion rate of contribution token to mainnet token
	pub conversion_rate: u32,
	/// Participation currencies (e.g stablecoin, DOT, KSM)
	/// TODO: PLMC-158. Use something like BoundedVec<Option<Currencies>, CurrenciesLimit>
	/// e.g. https://github.com/paritytech/substrate/blob/427fd09bcb193c1e79dec85b1e207c718b686c35/frame/uniques/src/types.rs#L110
	/// For now is easier to handle the case where only just one Currency is accepted
	pub participation_currencies: Currencies,
	/// Additional metadata
	pub metadata: Hash,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProjectInfo<BlockNumber, Balance: BalanceT> {
	/// Whether the project is frozen, so no `metadata` changes are allowed.
	pub is_frozen: bool,
	/// The price decided after the Auction Round
	pub weighted_average_price: Option<Balance>,
	/// The current status of the project
	pub project_status: ProjectStatus,
	/// When the different project phases start and end
	pub phase_transition_points: PhaseTransitionPoints<BlockNumber>,
	/// Fundraising target amount in USD equivalent
	pub fundraising_target: Balance,
}

#[derive(Debug)]
pub enum ValidityError {
	PriceTooLow,
	TicketSizeError,
	ParticipantsSizeError,
}

impl<BoundedString, Balance: BalanceT, Hash> Project<BoundedString, Balance, Hash> {
	// TODO: PLMC-162. Perform a REAL validity check
	pub fn validity_check(&self) -> Result<(), ValidityError> {
		if self.minimum_price == Balance::zero() {
			return Err(ValidityError::PriceTooLow)
		}
		self.ticket_size.is_valid()?;
		self.participants_size.is_valid()?;
		Ok(())
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct TicketSize<Balance: BalanceT> {
	pub minimum: Option<Balance>,
	pub maximum: Option<Balance>,
}

impl<Balance: BalanceT> TicketSize<Balance> {
	fn is_valid(&self) -> Result<(), ValidityError> {
		if self.minimum.is_some() && self.maximum.is_some() {
			if self.minimum < self.maximum {
				return Ok(())
			} else {
				return Err(ValidityError::TicketSizeError)
			}
		}
		if self.minimum.is_some() || self.maximum.is_some() {
			return Ok(())
		}

		Err(ValidityError::TicketSizeError)
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ParticipantsSize {
	pub minimum: Option<u32>,
	pub maximum: Option<u32>,
}

impl ParticipantsSize {
	fn is_valid(&self) -> Result<(), ValidityError> {
		match (self.minimum, self.maximum) {
			(Some(min), Some(max)) =>
				if min < max && min > 0 && max > 0 {
					Ok(())
				} else {
					Err(ValidityError::ParticipantsSizeError)
				},
			(Some(elem), None) | (None, Some(elem)) =>
				if elem > 0 {
					Ok(())
				} else {
					Err(ValidityError::ParticipantsSizeError)
				},
			(None, None) => Err(ValidityError::ParticipantsSizeError),
		}
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Thresholds {
	#[codec(compact)]
	retail: u8,
	#[codec(compact)]
	professional: u8,
	#[codec(compact)]
	institutional: u8,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyMetadata<BoundedString> {
	/// The user friendly name of this asset. Limited in length by `StringLimit`.
	pub name: BoundedString,
	/// The ticker symbol for this asset. Limited in length by `StringLimit`.
	pub symbol: BoundedString,
	/// The number of decimals this asset uses to represent one unit.
	pub decimals: u8,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct PhaseTransitionPoints<BlockNumber> {
	pub application_start_block: BlockNumber,
	pub application_end_block: Option<BlockNumber>,

	pub evaluation_start_block: Option<BlockNumber>,
	pub evaluation_end_block: Option<BlockNumber>,

	pub auction_initialize_period_start_block: Option<BlockNumber>,
	pub auction_initialize_period_end_block: Option<BlockNumber>,

	pub english_auction_start_block: Option<BlockNumber>,
	pub english_auction_end_block: Option<BlockNumber>,

	pub candle_auction_start_block: Option<BlockNumber>,
	pub candle_auction_end_block: Option<BlockNumber>,

	pub random_ending_block: Option<BlockNumber>,

	pub community_start_block: Option<BlockNumber>,
	pub community_end_block: Option<BlockNumber>,

	pub remainder_start_block: Option<BlockNumber>,
	pub remainder_end_block: Option<BlockNumber>,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BidInfo<Balance: BalanceT, AccountId, BlockNumber> {
	#[codec(compact)]
	pub amount: Balance,
	#[codec(compact)]
	pub price: Balance,
	#[codec(compact)]
	pub ticket_size: Balance,
	// Removed due to only being used in the price calculation, and it's not really needed there
	// pub ratio: Option<Perbill>,
	pub when: BlockNumber,
	pub bidder: AccountId,
	// TODO: PLMC-159. Not used yet, but will be used to check if the bid is funded after XCM is implemented
	pub funded: bool,
	pub multiplier: Balance,
	pub status: BidStatus<Balance>,
}

impl<Balance: BalanceT, AccountId, BlockNumber> BidInfo<Balance, AccountId, BlockNumber> {
	pub fn new(
		amount: Balance,
		price: Balance,
		when: BlockNumber,
		bidder: AccountId,
		multiplier: Balance,
	) -> Self {
		let ticket_size = amount.saturating_mul(price);
		Self {
			amount,
			price,
			ticket_size,
			// ratio: None,
			when,
			bidder,
			funded: false,
			multiplier,
			status: BidStatus::YetUnknown,
		}
	}
}

impl<Balance: BalanceT, AccountId: sp_std::cmp::Eq, BlockNumber: sp_std::cmp::Eq> sp_std::cmp::Ord
	for BidInfo<Balance, AccountId, BlockNumber>
{
	fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
		self.price.cmp(&other.price)
	}
}

impl<Balance: BalanceT, AccountId: sp_std::cmp::Eq, BlockNumber: sp_std::cmp::Eq>
	sp_std::cmp::PartialOrd for BidInfo<Balance, AccountId, BlockNumber>
{
	fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ContributionInfo<Balance: BalanceT> {
	#[codec(compact)]
	pub amount: Balance,
	pub can_claim: bool,
}

// TODO: PLMC-157. Use SCALE fixed indexes
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Currencies {
	DOT,
	KSM,
	#[default]
	USDC,
	USDT,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProjectStatus {
	#[default]
	Application,
	EvaluationRound,
	EvaluationEnded,
	EvaluationFailed,
	AuctionRound(AuctionPhase),
	CommunityRound,
	FundingEnded,
	ReadyToLaunch,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AuctionPhase {
	#[default]
	English,
	Candle,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum BidStatus<Balance: BalanceT> {
	/// The bid is not yet accepted or rejected
	#[default]
	YetUnknown,
	/// The bid is accepted
	Accepted,
	/// The bid is rejected, and the reason is provided
	Rejected(RejectionReason),
	/// The bid is partially accepted. The amount accepted and reason for rejection are provided
	PartiallyAccepted(Balance, RejectionReason),
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum RejectionReason {
	/// The bid was submitted after the candle auction ended
	AfterCandleEnd,
	/// The bid was accepted but too many tokens were requested. A partial amount was accepted
	NoTokensLeft,
}
