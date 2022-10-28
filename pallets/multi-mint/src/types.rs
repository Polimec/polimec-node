use frame_support::pallet_prelude::*;

// Types
pub type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;
pub type AmountOf<T> = <T as orml_tokens::Config>::Amount;
pub type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

// Structs
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyMetadata<BoundedString> {
	/// The user friendly name of this asset. Limited in length by `StringLimit`.
	pub name: BoundedString,
	/// The ticker symbol for this asset. Limited in length by `StringLimit`.
	pub symbol: BoundedString,
	/// The number of decimals this asset uses to represent one unit.
	pub decimals: u8,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyInfo<AccountId> {
	///
	pub current_owner: AccountId,
	///
	pub issuer: AccountId,
	///
	pub transfers_enabled: TransferStatus,
	///
	pub trading_enabled: TradingStatus,
}

// Enums
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum TransferStatus {
	Enabled,
	Disabled,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum TradingStatus {
	Enabled,
	Disabled,
}
