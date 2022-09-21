use frame_support::pallet_prelude::*;

// Types
pub(super) type CurrencyIdOf<T> = <T as orml_tokens::Config>::CurrencyId;
pub(super) type AmountOf<T> = <T as orml_tokens::Config>::Amount;
pub(super) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

// Structs
#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyMetadata<DepositBalance, BoundedString> {
	/// This pays for the data stored in this struct.
	pub(super) deposit: DepositBalance,
	/// The user friendly name of this asset. Limited in length by `StringLimit`.
	pub(super) name: BoundedString,
	/// The ticker symbol for this asset. Limited in length by `StringLimit`.
	pub(super) symbol: BoundedString,
	/// The number of decimals this asset uses to represent one unit.
	pub(super) decimals: u8,
}

impl<DepositBalance, BoundedString> CurrencyMetadata<DepositBalance, BoundedString> {
	pub fn new(
		deposit: DepositBalance,
		name: BoundedString,
		symbol: BoundedString,
		decimals: u8,
	) -> Self {
		Self { deposit, name, symbol, decimals }
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CurrencyInfo<AccountId> {
	pub(super) issuer: AccountId,
	pub(super) transfers_frozen: bool,
	pub(super) trading_enabled: TradingStatus,
}

impl<AccountId> CurrencyInfo<AccountId> {
	pub fn new(issuer: AccountId, transfers_frozen: bool, trading_enabled: TradingStatus) -> Self {
		Self { issuer, transfers_frozen, trading_enabled }
	}
}

// Enums
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum TradingStatus {
	Enabled,
	Disabled,
}
