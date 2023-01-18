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
use sp_arithmetic::Perbill;

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Project<BoundedString, Balance: BalanceT, Hash> {
	/// Token Metadata
	pub token_information: CurrencyMetadata<BoundedString>,
	/// Total allocation of Contribution Tokens available for the Funding Round
	pub total_allocation_size: Balance,
	/// Minimum price per Contribution Token
	pub minimum_price: Balance,
	/// Fundraising target amount in USD equivalent
	pub fundraising_target: Balance,
	/// Maximum and/or minimum ticket size
	pub ticket_size: TicketSize<Balance>,
	/// Maximum and/or minimum number of participants for the Auction and Community Round
	pub participants_size: ParticipantsSize,
	/// Funding round thresholds for Retail, Professional and Institutional participants
	pub funding_thresholds: Thresholds,
	/// Conversion rate of contribution token to mainnet token
	pub conversion_rate: u32,
	/// Participation currencies (e.g stablecoin, DOT, KSM)
	/// TODO: Use something like BoundedVec<Option<Currencies>, CurrenciesLimit>
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
	pub final_price: Option<Balance>,
	/// When the project is created
	pub created_at: BlockNumber,
	/// The current status of the project
	pub project_status: ProjectStatus,
	pub evaluation_period_ends: Option<BlockNumber>,
	pub auction_metadata: Option<AuctionMetadata<BlockNumber>>,
}

#[derive(Debug)]
pub enum ValidityError {
	PriceTooLow,
	TicketSizeError,
	ParticipantsSizeError,
}

impl<BoundedString, Balance: BalanceT, Hash>
	Project<BoundedString, Balance, Hash>
{
	// TODO: Perform a REAL validity check
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
		if self.minimum.is_some() && self.maximum.is_some() {
			if self.minimum < self.maximum {
				return Ok(())
			} else {
				return Err(ValidityError::ParticipantsSizeError)
			}
		}
		if self.minimum.is_some() || self.maximum.is_some() {
			return Ok(())
		}

		Err(ValidityError::ParticipantsSizeError)
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Thresholds {
	#[codec(compact)]
	retail: u64,
	#[codec(compact)]
	professional: u64,
	#[codec(compact)]
	institutional: u64,
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
pub struct AuctionMetadata<BlockNumber> {
	/// When (expressed in block numbers) the Auction Round started
	pub starting_block: BlockNumber,
	/// When (expressed in block numbers) the English Auction phase ends
	pub english_ending_block: BlockNumber,
	/// When (expressed in block numbers) the Candle Auction phase ends
	pub candle_ending_block: BlockNumber,
	/// When (expressed in block numbers) the Dutch Auction phase ends
	pub random_ending_block: Option<BlockNumber>,
	/// When (expressed in block numbers) the Community Round ends
	pub community_ending_block: BlockNumber,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BidInfo<Balance: BalanceT, AccountId, BlockNumber> {
	#[codec(compact)]
	pub market_cap: Balance,
	#[codec(compact)]
	pub amount: Balance,
	#[codec(compact)]
	pub ratio: Perbill,
	pub when: BlockNumber,
	pub bidder: AccountId,
	pub funded: bool,
	pub multiplier: u8,
}

impl<Balance: BalanceT + From<u64>, AccountId, BlockNumber>
	BidInfo<Balance, AccountId, BlockNumber>
{
	pub fn new(
		market_cap: Balance,
		amount: Balance,
		auction_taget: Balance,
		when: BlockNumber,
		bidder: AccountId,
		multiplier: u8,
	) -> Self {
		let ratio = Perbill::from_rational(amount, auction_taget);
		Self { market_cap, amount, ratio, when, bidder, funded: false, multiplier }
	}
}

impl<Balance: BalanceT + From<u64>, AccountId: sp_std::cmp::Eq, BlockNumber: sp_std::cmp::Eq>
	sp_std::cmp::Ord for BidInfo<Balance, AccountId, BlockNumber>
{
	fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
		let self_value = self.amount.saturating_mul(self.market_cap);
		let other_value = other.amount.saturating_mul(other.market_cap);
		self_value.cmp(&other_value)
	}
}

impl<Balance: BalanceT + From<u64>, AccountId: sp_std::cmp::Eq, BlockNumber: sp_std::cmp::Eq>
	sp_std::cmp::PartialOrd for BidInfo<Balance, AccountId, BlockNumber>
{
	fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
		Some(self.cmp(&other))
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ContributionInfo<Balance: BalanceT> {
	#[codec(compact)]
	pub amount: Balance,
	pub can_claim: bool,
}

// TODO: Use SCALE fixed indexes
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Currencies {
	DOT,
	KSM,
	USDC,
	#[default]
	USDT,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProjectStatus {
	#[default]
	Application,
	EvaluationRound,
	EvaluationEnded,
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
