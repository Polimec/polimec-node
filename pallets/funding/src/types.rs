use frame_support::{pallet_prelude::*, BoundedVec};

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ProjectMetadata<AccountId> {
	/// The issuer of the  certificate
	pub issuer_certifcate: Issuer,
	/// Minimum price per contribution token
	pub minimum_price: u128,
	/// Maximum ticket size
	pub maximum_ticket_size: u32,
	/// Minimum number of participants for the auction
	pub minimum_participants_size: u32,
	/// Total allocation of contribution tokens to be offered on Polimec
	pub total_allocation_size: u128,
	/// Smallest denomination
	pub decimals: u8,
	/// Funding round thresholds for retail-, professional- and institutional participants
	pub funding_thresholds: u128,
	/// Conversion rate of contribution token to mainnet token
	pub conversion_rate: u32,
	/// Participation currencies (e.g stablecoins, DOT, KSM)
	/// TODO: Use something like BoundedVec<Option<Currencies>, StringLimit>
	/// e.g. https://github.com/paritytech/substrate/blob/427fd09bcb193c1e79dec85b1e207c718b686c35/frame/uniques/src/types.rs#L110
	pub participation_currencies: BoundedVec<Option<Currencies>, ConstU32<4>>,
	/// Issuer destination accounts for each accepted token (for receiving participations)
	pub destinations_account: AccountId,
}
#[derive(Debug)]
pub enum ValidityError {
	NotEnoughParticipationCurrencies,
	NotEnoughParticipants,
}

impl<AccountId> ProjectMetadata<AccountId> {
	pub fn validity_check(&self) -> Result<(), ValidityError> {
		if self.minimum_participants_size == 0 {
			return Err(ValidityError::NotEnoughParticipants)
		}
		if !self
			.participation_currencies
			.iter()
			.any(|maybe_currency| maybe_currency.is_some())
		{
			return Err(ValidityError::NotEnoughParticipationCurrencies)
		}

		Ok(())
	}
}

// Enums
// TODO: Use SCALE fixed indexes
// TODO: Check if it's correct
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Issuer {
	Kilt,
	Other,
}

// TODO: Use SCALE fixed indexes
/// Native currency: `PLMC = [0; 8]`
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum Currencies {
	DOT,
	KSM,
	USDC,
	USDT,
}
