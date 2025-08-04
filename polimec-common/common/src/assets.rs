use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use variant_count::VariantCount;
use xcm::v4::prelude::{Ethereum, GeneralIndex, GlobalConsensus, Location, PalletInstance, Parachain};
extern crate alloc;
use alloc::{vec, vec::Vec};

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
	pub fn id(&self) -> Location {
		match self {
			Self::USDT => Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1984)]),
			Self::DOT => Location::parent(),
			Self::USDC => Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1337)]),
			Self::ETH => Location::new(2, [GlobalConsensus(Ethereum { chain_id: 1 })]),
		}
	}

	pub fn all_ids() -> Vec<Location> {
		vec![Self::USDT.id(), Self::DOT.id(), Self::USDC.id(), Self::ETH.id()]
	}
}
