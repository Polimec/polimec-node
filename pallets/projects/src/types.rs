use frame_support::pallet_prelude::*;

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
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
	/// TODO: Use something like Vec<CurrencyIdOf<T>>
	pub participation_currencies: u128,
	/// Issuer destination accounts for each accepted token (for receiving participations)
	pub destinations_account: AccountId,
}

// Enums
// TODO: Use SCALE fixed indexes
// TODO: Check
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum Issuer {
	Kilt,
	Other,
}
