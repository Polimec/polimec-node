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
use crate::PolitestRuntime;
use frame_support::BoundedVec;
pub use pallet_funding::instantiator::{BidParams, ContributionParams, UserToUSDBalance};
use pallet_funding::{
	AcceptedFundingAsset, BiddingTicketSizes, ContributingTicketSizes, CurrencyMetadata, PriceProviderOf,
	ProjectMetadata, ProjectMetadataOf, TicketSize,
};
use sp_arithmetic::{FixedPointNumber, Percent};

use macros::generate_accounts;
use pallet_funding::traits::ProvideAssetPrice;
use polimec_common::USD_DECIMALS;
use polimec_runtime::{PLMC, USD_UNIT};
use politest_runtime::AccountId;
use sp_runtime::{traits::ConstU32, Perquintill};

pub const IPFS_CID: &str = "QmeuJ24ffwLAZppQcgcggJs3n689bewednYkuc8Bx5Gngz";
pub const CT_DECIMALS: u8 = 18;
pub const CT_UNIT: u128 = 10_u128.pow(CT_DECIMALS as u32);

pub type IntegrationInstantiator = pallet_funding::instantiator::Instantiator<
	PolitestRuntime,
	<PolitestRuntime as pallet_funding::Config>::AllPalletsWithoutSystem,
	<PolitestRuntime as pallet_funding::Config>::RuntimeEvent,
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
pub fn default_weights() -> Vec<u8> {
	vec![20u8, 15u8, 10u8, 25u8, 30u8]
}
pub fn default_bidder_multipliers() -> Vec<u8> {
	vec![1u8, 6u8, 10u8, 8u8, 3u8]
}
pub fn default_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 1u8, 1u8, 1u8, 1u8]
}

pub fn default_project_metadata(issuer: AccountId) -> ProjectMetadataOf<politest_runtime::Runtime> {
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name(), symbol: bounded_symbol(), decimals: CT_DECIMALS },
		mainnet_token_max_supply: 8_000_000 * CT_UNIT,
		total_allocation_size: 1_000_000 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: PriceProviderOf::<PolitestRuntime>::calculate_decimals_aware_price(
			sp_runtime::FixedU128::from_float(10.0),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap(),
		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(Some(5000 * USD_UNIT), None),
			institutional: TicketSize::new(Some(5000 * USD_UNIT), None),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(None, None),
			professional: TicketSize::new(None, None),
			institutional: TicketSize::new(None, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: issuer,
		policy_ipfs_cid: Some(ipfs_hash()),
	}
}
pub fn default_evaluations() -> Vec<UserToUSDBalance<PolitestRuntime>> {
	vec![
		UserToUSDBalance::new(EVAL_1.into(), 500_000 * PLMC),
		UserToUSDBalance::new(EVAL_2.into(), 250_000 * PLMC),
		UserToUSDBalance::new(EVAL_3.into(), 320_000 * PLMC),
	]
}
pub fn default_bidders() -> Vec<AccountId> {
	vec![BIDDER_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(), BIDDER_5.into()]
}

pub fn default_bids() -> Vec<BidParams<PolitestRuntime>> {
	let inst = IntegrationInstantiator::new(None);
	let default_metadata = default_project_metadata(ISSUER.into());
	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let auction_90_percent = Perquintill::from_percent(90) * auction_allocation;
	let auction_usd_funding = default_metadata.minimum_price.saturating_mul_int(auction_90_percent);

	inst.generate_bids_from_total_usd(
		auction_usd_funding,
		default_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	)
}

pub fn default_community_contributions() -> Vec<ContributionParams<PolitestRuntime>> {
	let inst = IntegrationInstantiator::new(None);

	let default_metadata = default_project_metadata(ISSUER.into());

	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let contribution_allocation = default_metadata.total_allocation_size - auction_allocation;

	let eighty_percent_funding_ct = Perquintill::from_percent(80) * contribution_allocation;
	let eighty_percent_funding_usd = default_metadata.minimum_price.saturating_mul_int(eighty_percent_funding_ct);

	inst.generate_contributions_from_total_usd(
		eighty_percent_funding_usd,
		default_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_contributor_multipliers(),
	)
}

pub fn default_remainder_contributions() -> Vec<ContributionParams<PolitestRuntime>> {
	let inst = IntegrationInstantiator::new(None);

	let default_metadata = default_project_metadata(ISSUER.into());

	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let contribution_allocation = default_metadata.total_allocation_size - auction_allocation;

	let ten_percent_auction = Perquintill::from_percent(10) * auction_allocation;
	let ten_percent_auction_usd = default_metadata.minimum_price.saturating_mul_int(ten_percent_auction);
	let ten_percent_contribution = Perquintill::from_percent(10) * contribution_allocation;
	let ten_percent_contribution_usd = default_metadata.minimum_price.saturating_mul_int(ten_percent_contribution);

	inst.generate_contributions_from_total_usd(
		ten_percent_auction_usd + ten_percent_contribution_usd,
		default_metadata.minimum_price,
		vec![20u8, 15u8, 10u8, 25u8, 23u8, 7u8],
		default_remainder_contributors(),
		vec![1u8, 1u8, 1u8, 1u8, 1u8, 1u8],
	)
}
pub fn default_community_contributors() -> Vec<AccountId> {
	vec![BUYER_1.into(), BUYER_2.into(), BUYER_3.into(), BUYER_4.into(), BUYER_5.into()]
}

pub fn default_remainder_contributors() -> Vec<AccountId> {
	vec![EVAL_4.into(), BUYER_6.into(), BIDDER_6.into(), EVAL_1.into(), BUYER_1.into(), BIDDER_1.into()]
}
