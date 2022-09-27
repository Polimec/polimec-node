use frame_support::pallet_prelude::*;

// Types
pub(super) type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;
pub(super) type AmountOf<T> = <T as orml_tokens::Config>::Amount;
pub(super) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

// Structs
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyMetadata<BoundedString> {
	/// The user friendly name of this asset. Limited in length by `StringLimit`.
	pub(super) name: BoundedString,
	/// The ticker symbol for this asset. Limited in length by `StringLimit`.
	pub(super) symbol: BoundedString,
	/// The number of decimals this asset uses to represent one unit.
	pub(super) decimals: u8,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyInfo<AccountId> {
	///
	pub(super) current_owner: AccountId,
	///
	pub(super) issuer: AccountId,
	///
	pub(super) transfers_enabled: TransferStatus,
	///
	pub(super) trading_enabled: TradingStatus,
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
