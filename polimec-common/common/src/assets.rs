use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use variant_count::VariantCount;
use xcm::v5::prelude::{AccountKey20, Ethereum, GeneralIndex, GlobalConsensus, Location, PalletInstance, Parachain};
extern crate alloc;
use alloc::{vec, vec::*};

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
)]
pub enum AcceptedFundingAsset {
	#[codec(index = 0)]
	USDT,
	#[codec(index = 1)]
	USDC,
	#[codec(index = 2)]
	DOT,
	#[codec(index = 3)]
	WETH,
}
impl AcceptedFundingAsset {
	pub fn id(&self) -> Location {
		match self {
			AcceptedFundingAsset::USDT =>
				Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1984)]).into(),
			AcceptedFundingAsset::DOT => Location::parent(),
			AcceptedFundingAsset::USDC =>
				Location::new(1, [Parachain(1000), PalletInstance(50), GeneralIndex(1337)]).into(),
			AcceptedFundingAsset::WETH => Location::new(
				2,
				[
					GlobalConsensus(Ethereum { chain_id: 1 }),
					AccountKey20 { network: None, key: hex_literal::hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2") },
				],
			),
		}
	}

	pub fn all_ids() -> Vec<Location> {
		vec![Self::USDT.id(), Self::DOT.id(), Self::USDC.id(), Self::WETH.id()]
	}
}
