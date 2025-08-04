use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use variant_count::VariantCount;
use xcm::v4::prelude::{Ethereum, GeneralIndex, GlobalConsensus, Location, PalletInstance, Parachain};

#[derive(
	VariantCount,
	Clone,
	Copy,
	Encode,
	Decode,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	Serialize,
	Deserialize,
	DecodeWithMemTracking,
)]
pub enum AcceptedFundingAsset {
	#[codec(index = 0)]
	USDT,
	#[codec(index = 1)]
	USDC,
	#[codec(index = 2)]
	DOT,
	#[codec(index = 3)]
	ETH,
}
impl AcceptedFundingAsset {
	// Note: this should be synced with the AssetId in the Pallet Assets.
	pub fn id(&self) -> Location {
		match self {
			Self::USDT => Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1984)]),
			Self::DOT => Location::parent(),
			Self::USDC => Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1337)]),
			Self::ETH => Location::new(2, [GlobalConsensus(Ethereum { chain_id: 1 })]),
		}
	}

	// Note: this should be synced with the decimals in the Pallet Assets.
	pub const fn decimals(&self) -> u8 {
		match self {
			Self::USDT => 6,
			Self::USDC => 6,
			Self::DOT => 10,
			Self::ETH => 18,
		}
	}

	pub fn all_ids() -> [Location; AcceptedFundingAsset::VARIANT_COUNT] {
		[Self::USDT.id(), Self::USDC.id(), Self::DOT.id(), Self::ETH.id()]
	}

	pub fn all_ids_and_plmc() -> [Location; AcceptedFundingAsset::VARIANT_COUNT + 1] {
		[Self::USDT.id(), Self::USDC.id(), Self::DOT.id(), Self::ETH.id(), Location::here()]
	}
}
