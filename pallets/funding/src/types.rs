use frame_support::pallet_prelude::*;

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Project<AccountId, BoundedString> {
	/// The issuer of the  certificate
	pub issuer_certifcate: Issuer,
	/// Name of the issuer
	pub issuer_name: BoundedString,
	/// Token information
	pub token_information: CurrencyMetadata<BoundedString>,
	/// Total allocation of contribution tokens available for the funding round
	pub total_allocation_size: u128,
	/// Minimum price per contribution token
	/// TODO: This should be a float, can we use it?
	pub minimum_price: u128,
	/// Fundraising target amount in USD equivalent
	pub fundraising_target: u128,
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
	pub funding_times: FundingTimes,
	/// Additional metadata
	pub project_metadata: ProjectMetadata<BoundedString>,
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
	pub total_supply: u128,
	/// A link to the roadmap
	pub roadmap: BoundedString,
	/// A link to a decription on how the funds will be used
	pub usage_of_founds: BoundedString,
}

#[derive(Debug)]
pub enum ValidityError {
	NotEnoughParticipationCurrencies,
	NotEnoughParticipants,
	PriceTooLow,
	TicketSizeError,
	ParticipantsSizeError,
}

impl<AccountId, BoundedString> Project<AccountId, BoundedString> {
	// TODO: Perform a REAL validity cehck
	pub fn validity_check(&self) -> Result<(), ValidityError> {
		if self.minimum_price == 0 {
			return Err(ValidityError::PriceTooLow)
		}
		self.ticket_size.is_valid()?;
		self.participants_size.is_valid()?;
		Ok(())
	}
}

#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct TicketSize {
	minimum: Option<u32>,
	maximum: Option<u32>,
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
	minimum: Option<u32>,
	maximum: Option<u32>,
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
	retail: u32,
	professional: u32,
	institutional: u32,
}

// TODO: This is just a placeholder
// TODO: Implement the time logic
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FundingTimes {
	start: u32,
	stop: u32,
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
/// Native currency: `PLMC = [0; 8]`
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Currencies {
	DOT,
	KSM,
	#[default]
	USDC,
	USDT,
}
