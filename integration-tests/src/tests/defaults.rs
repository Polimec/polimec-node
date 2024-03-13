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
pub use pallet_funding::instantiator::{BidParams, ContributionParams, UserToPLMCBalance, UserToUSDBalance};
use pallet_funding::{
	AcceptedFundingAsset, BiddingTicketSizes, ContributingTicketSizes, CurrencyMetadata, ProjectMetadata,
	ProjectMetadataOf, RoundTicketSizes, TicketSize,
};
use sp_arithmetic::{FixedPointNumber, Percent};
use sp_core::H256;

use macros::generate_accounts;
use polimec_parachain_runtime::AccountId;
use sp_runtime::{traits::ConstU32, Perquintill};
pub const METADATA: &str = r#"METADATA
        {
            "whitepaper":"ipfs_url",
            "team_description":"ipfs_url",
            "tokenomics":"ipfs_url",
            "roadmap":"ipfs_url",
            "usage_of_founds":"ipfs_url"
        }"#;
pub const ASSET_DECIMALS: u8 = 10;
pub const ASSET_UNIT: u128 = 10_u128.pow(10 as u32);
pub const PLMC: u128 = 10u128.pow(10);
pub const US_DOLLAR: u128 = 1_0_000_000_000;

pub type IntegrationInstantiator = pallet_funding::instantiator::Instantiator<
	PolimecRuntime,
	<PolimecRuntime as pallet_funding::Config>::AllPalletsWithoutSystem,
	<PolimecRuntime as pallet_funding::Config>::RuntimeEvent,
>;
pub fn hashed(data: impl AsRef<[u8]>) -> sp_core::H256 {
	<sp_runtime::traits::BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
}

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
pub fn metadata_hash(nonce: u32) -> H256 {
	hashed(format!("{}-{}", METADATA, nonce))
}
pub fn default_weights() -> Vec<u8> {
	vec![20u8, 15u8, 10u8, 25u8, 30u8]
}
pub fn default_bidder_multipliers() -> Vec<u8> {
	vec![1u8, 6u8, 20u8, 12u8, 3u8]
}
pub fn default_contributor_multipliers() -> Vec<u8> {
	vec![1u8, 2u8, 1u8, 4u8, 1u8]
}

pub fn default_project_metadata(
	nonce: u32,
	issuer: AccountId,
) -> ProjectMetadataOf<polimec_parachain_runtime::Runtime> {
	ProjectMetadata {
		token_information: CurrencyMetadata {
			name: bounded_name(),
			symbol: bounded_symbol(),
			decimals: ASSET_DECIMALS,
		},
		mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
		total_allocation_size: 1_000_000 * ASSET_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: sp_runtime::FixedU128::from_float(10.0),
		round_ticket_sizes: RoundTicketSizes {
			bidding: BiddingTicketSizes {
				professional: TicketSize::new(Some(500 * ASSET_UNIT), None),
				institutional: TicketSize::new(Some(500 * ASSET_UNIT), None),
				phantom: Default::default(),
			},
			contributing: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: issuer,
		offchain_information_hash: Some(metadata_hash(nonce)),
	}
}
pub fn default_evaluations() -> Vec<UserToUSDBalance<polimec_parachain_runtime::Runtime>> {
	vec![
		UserToUSDBalance::new(EVAL_1.into(), 500_000 * PLMC),
		UserToUSDBalance::new(EVAL_2.into(), 250_000 * PLMC),
		UserToUSDBalance::new(EVAL_3.into(), 320_000 * PLMC),
	]
}
pub fn default_bidders() -> Vec<AccountId> {
	vec![BIDDER_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(), BIDDER_5.into()]
}

pub fn default_bids() -> Vec<BidParams<PolimecRuntime>> {
	let default_metadata = default_project_metadata(0u32, ISSUER.into());
	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let auction_90_percent = Perquintill::from_percent(90) * auction_allocation;
	let auction_usd_funding = default_metadata.minimum_price.saturating_mul_int(auction_90_percent);

	IntegrationInstantiator::generate_bids_from_total_usd(
		auction_usd_funding,
		default_metadata.minimum_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	)
}

pub fn default_community_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	let default_metadata = default_project_metadata(0u32, ISSUER.into());

	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let contribution_allocation = default_metadata.total_allocation_size - auction_allocation;

	let eighty_percent_funding_ct = Perquintill::from_percent(80) * contribution_allocation;
	let eighty_percent_funding_usd = default_metadata.minimum_price.saturating_mul_int(eighty_percent_funding_ct);

	IntegrationInstantiator::generate_contributions_from_total_usd(
		eighty_percent_funding_usd,
		default_metadata.minimum_price,
		default_weights(),
		default_community_contributors(),
		default_contributor_multipliers(),
	)
}

pub fn default_remainder_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	let default_metadata = default_project_metadata(0u32, ISSUER.into());

	let auction_allocation =
		default_metadata.auction_round_allocation_percentage * default_metadata.total_allocation_size;
	let contribution_allocation = default_metadata.total_allocation_size - auction_allocation;

	let ten_percent_auction = Perquintill::from_percent(10) * auction_allocation;
	let ten_percent_auction_usd = default_metadata.minimum_price.saturating_mul_int(ten_percent_auction);
	let ten_percent_contribution = Perquintill::from_percent(10) * contribution_allocation;
	let ten_percent_contribution_usd = default_metadata.minimum_price.saturating_mul_int(ten_percent_contribution);

	IntegrationInstantiator::generate_contributions_from_total_usd(
		ten_percent_auction_usd + ten_percent_contribution_usd,
		default_metadata.minimum_price,
		vec![20u8, 15u8, 10u8, 25u8, 23u8, 7u8],
		default_remainder_contributors(),
		vec![1u8, 2u8, 12u8, 1u8, 3u8, 10u8],
	)
}
pub fn default_community_contributors() -> Vec<AccountId> {
	vec![BUYER_1.into(), BUYER_2.into(), BUYER_3.into(), BUYER_4.into(), BUYER_5.into()]
}

pub fn default_remainder_contributors() -> Vec<AccountId> {
	vec![EVAL_4.into(), BUYER_6.into(), BIDDER_6.into(), EVAL_1.into(), BUYER_1.into(), BIDDER_1.into()]
}
