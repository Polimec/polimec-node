use frame_support::pallet_prelude::*;
use sp_runtime::traits::Zero;

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Project<
	AccountId,
	BoundedString,
	BlockNumber,
	Balance: MaxEncodedLen + Zero + std::cmp::PartialEq,
> {
	/// The issuer of the  certificate
	pub issuer_certifcate: Issuer,
	/// Name of the issuer
	pub issuer_name: BoundedString,
	/// Token information
	pub token_information: CurrencyMetadata<BoundedString>,
	/// Total allocation of contribution tokens available for the funding round
	pub total_allocation_size: Balance,
	/// Minimum price per contribution token
	/// TODO: This should be a float, can we use it?
	/// TODO: Check how to handle that using smallest denomination
	pub minimum_price: Balance,
	/// Fundraising target amount in USD equivalent
	pub fundraising_target: Balance,
	/// Maximum and/or minimum ticket size
	pub ticket_size: TicketSize,
	/// Maximum and/or minimum number of participants for the auction and community round
	pub participants_size: ParticipantsSize,
	/// Funding round thresholds for retail, professional and institutional participants
	pub funding_thresholds: Thresholds,
	/// Conversion rate of contribution token to mainnet token
	pub conversion_rate: u32,
	/// Participation currencies (e.g stablecoins, DOT, KSM)
	/// TODO: Use something like BoundedVec<Option<Currencies>, StringLimit>
	/// e.g. https://github.com/paritytech/substrate/blob/427fd09bcb193c1e79dec85b1e207c718b686c35/frame/uniques/src/types.rs#L110
	/// For now is easier to handle the case where only just one Currency is accepted
	pub participation_currencies: Currencies,
	/// Issuer destination accounts for accepted participation currencies (for receiving
	/// contributions)
	pub destinations_account: AccountId,
	/// Date/time of funding round start and end
	pub funding_times: FundingTimes<BlockNumber>,
	/// Additional metadata
	pub metadata: ProjectMetadata<BoundedString>,
	/// When the project is created
	pub created_at: BlockNumber,

	// TODO: I don't like that `is_frozen` field is passed in input directly from the user, maybe
	// the current structure of projects (Project + ProjectMetadata) needs to be revised
	/// Whether the project is frozen, so no `metadata` changes are allowed.
	pub is_frozen: bool,
	// TODO: Check if it is better/cleaner to save the evaluation infomration inside the project
	// itself. pub evaluation_status: EvaluationMetadata<..., ...>,
	// TODO: Check if it is better/cleaner to save the auction infomration inside the project
	// itself. pub auctionn_status: AuctionMetadata<..., ...>,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProjectMetadata<BoundedString> {
	/// A link to the whitepaper
	pub whitepaper: BoundedString,
	/// A link to a team description
	pub team_description: BoundedString,
	/// A link to the tokenomics description
	pub tokenomics: BoundedString,
	/// Total supply of mainnet tokens
	// TODO: Maybe this has to become something similar to `pub total_supply: Balance`
	pub total_supply: u128,
	/// A link to the roadmap
	pub roadmap: BoundedString,
	/// A link to a decription on how the funds will be used
	pub usage_of_founds: BoundedString,
}

#[derive(Debug)]
pub enum ValidityError {
	PriceTooLow,
	TicketSizeError,
	ParticipantsSizeError,
}

impl<
		AccountId,
		BoundedString,
		BlockNumber,
		Balance: MaxEncodedLen + Zero + std::cmp::PartialEq,
	> Project<AccountId, BoundedString, BlockNumber, Balance>
{
	// TODO: Perform a REAL validity cehck
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
pub struct TicketSize {
	pub minimum: Option<u32>,
	pub maximum: Option<u32>,
}

impl TicketSize {
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

// TODO: This is just a placeholder
// TODO: Implement the time logic
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FundingTimes<BlockNumber> {
	start: BlockNumber,
	stop: BlockNumber,
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
pub struct EvaluationMetadata<BlockNumber, Balance: MaxEncodedLen> {
	// The current status in the evaluation phase
	pub evaluation_status: EvaluationStatus,
	// When (expressed in block numbers) the evaluation phase ends
	pub evaluation_period_ends: BlockNumber,
	// The amount of PLMC bonded in the project during the evaluation phase
	#[codec(compact)]
	pub amount_bonded: Balance,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AuctionMetadata<BlockNumber, Balance: MaxEncodedLen> {
	// The current status in the evaluation phase
	pub auction_status: AuctionStatus,
	// When (expressed in block numbers) the evaluation phase ends
	pub auction_starting_block: BlockNumber,
	// The amount of PLMC bonded in the project during the evaluation phase
	#[codec(compact)]
	pub amount_bonded: Balance,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BondingLedger<AccountId, Balance: MaxEncodedLen> {
	/// The account whose balance is actually locked and at bond.
	pub stash: AccountId,
	// The amount of PLMC bonded in the project during the evaluation phase
	#[codec(compact)]
	pub amount_bonded: Balance,
}

// Enums
// TODO: Use SCALE fixed indexes
// TODO: Check if it's correct
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Issuer {
	#[default]
	Kilt,
	Other,
}

// TODO: Use SCALE fixed indexes
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Currencies {
	DOT,
	KSM,
	#[default]
	USDC,
	USDT,
}
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum EvaluationStatus {
	#[default]
	NotYetStarted,
	Started,
	Ended,
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AuctionStatus {
	#[default]
	NotYetStarted,
	Started,
	Ended,
}
