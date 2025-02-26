// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
use crate::PolimecRuntime;
use frame_support::BoundedVec;
pub use pallet_funding::instantiator::EvaluationParams;
use pallet_funding::{
	BiddingTicketSizes, CurrencyMetadata, ParticipantsAccountType, PriceProviderOf, ProjectMetadata, ProjectMetadataOf,
	TicketSize,
};

use macros::generate_accounts;
use polimec_common::{
	assets::AcceptedFundingAsset::{DOT, ETH, USDC, USDT},
	ProvideAssetPrice, USD_DECIMALS, USD_UNIT,
};
use polimec_runtime::AccountId;
use sp_runtime::traits::ConstU32;

pub const IPFS_CID: &str = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";
pub const CT_DECIMALS: u8 = 18;
pub const CT_UNIT: u128 = 10_u128.pow(CT_DECIMALS as u32);

pub type IntegrationInstantiator = pallet_funding::instantiator::Instantiator<
	PolimecRuntime,
	<PolimecRuntime as pallet_funding::Config>::AllPalletsWithoutSystem,
	<PolimecRuntime as pallet_funding::Config>::RuntimeEvent,
>;

generate_accounts!(
	ISSUER, EVAL_1, EVAL_2, EVAL_3, EVAL_4, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5, BIDDER_6, BUYER_1,
	BUYER_2, BUYER_3, BUYER_4, BUYER_5, BUYER_6,
);

pub fn bounded_name() -> BoundedVec<u8, ConstU32<64>> {
	BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap()
}
pub fn bounded_symbol() -> BoundedVec<u8, ConstU32<64>> {
	BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap()
}
pub fn ipfs_hash() -> BoundedVec<u8, ConstU32<96>> {
	BoundedVec::try_from(IPFS_CID.as_bytes().to_vec()).unwrap()
}

pub fn default_project_metadata(issuer: AccountId) -> ProjectMetadataOf<polimec_runtime::Runtime> {
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name(), symbol: bounded_symbol(), decimals: CT_DECIMALS },
		mainnet_token_max_supply: 8_000_000 * CT_UNIT,
		total_allocation_size: 1_000_000 * CT_UNIT,
		minimum_price: PriceProviderOf::<PolimecRuntime>::calculate_decimals_aware_price(
			sp_runtime::FixedU128::from_float(10.0),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap(),
		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(5000 * USD_UNIT, None),
			institutional: TicketSize::new(5000 * USD_UNIT, None),
			retail: TicketSize::new(100 * USD_UNIT, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![USDT, USDC, DOT, ETH].try_into().unwrap(),
		funding_destination_account: issuer,
		policy_ipfs_cid: Some(ipfs_hash()),
		participants_account_type: ParticipantsAccountType::Polkadot,
	}
}

#[test]
fn sandbox() {
	use pallet_funding::WeightInfo;

	let bid_weight = polimec_runtime::weights::pallet_funding::WeightInfo::<PolimecRuntime>::bid(0);
	dbg!(bid_weight);
}
