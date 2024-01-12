use frame_support::{assert_ok, bounded_vec, BoundedVec};
pub use pallet_funding::instantiator::{BidParams, ContributionParams, UserToPLMCBalance, UserToUSDBalance};
use pallet_funding::{
	AcceptedFundingAsset, CurrencyMetadata, ParticipantsSize, ProjectMetadata, ProjectMetadataOf, TicketSize,
};
use sp_arithmetic::FixedU128;
use sp_core::H256;
use std::collections::HashMap;

use crate::PolimecOrigin;
use macros::generate_accounts;
use polimec_parachain_runtime::AccountId;
use sp_runtime::{traits::ConstU32, Perquintill};
use xcm_emulator::TestExt;

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

pub fn default_project(issuer: AccountId, nonce: u32) -> ProjectMetadataOf<polimec_parachain_runtime::Runtime> {
	ProjectMetadata {
		token_information: CurrencyMetadata {
			name: bounded_name(),
			symbol: bounded_symbol(),
			decimals: ASSET_DECIMALS,
		},
		mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
		total_allocation_size: (50_000 * ASSET_UNIT, 50_000 * ASSET_UNIT),
		minimum_price: sp_runtime::FixedU128::from_float(1.0),
		ticket_size: TicketSize { minimum: Some(1), maximum: None },
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		funding_thresholds: Default::default(),
		conversion_rate: 0,
		participation_currencies: AcceptedFundingAsset::USDT,
		funding_destination_account: issuer,
		offchain_information_hash: Some(metadata_hash(nonce)),
	}
}
pub fn default_evaluations() -> Vec<UserToUSDBalance<polimec_parachain_runtime::Runtime>> {
	vec![
		UserToUSDBalance::new(EVAL_1.into(), 50_000 * PLMC),
		UserToUSDBalance::new(EVAL_2.into(), 25_000 * PLMC),
		UserToUSDBalance::new(EVAL_3.into(), 32_000 * PLMC),
	]
}
pub fn default_bidders() -> Vec<AccountId> {
	vec![BIDDER_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(), BIDDER_5.into()]
}

pub fn default_bids() -> Vec<BidParams<PolimecRuntime>> {
	let forty_percent_funding_usd = Perquintill::from_percent(40) * 100_000 * ASSET_UNIT;

	IntegrationInstantiator::generate_bids_from_total_usd(
		forty_percent_funding_usd,
		sp_runtime::FixedU128::from_float(1.0),
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	)
}

pub fn default_community_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	let fifty_percent_funding_usd = Perquintill::from_percent(50) * 100_000 * ASSET_UNIT;

	IntegrationInstantiator::generate_contributions_from_total_usd(
		fifty_percent_funding_usd,
		sp_runtime::FixedU128::from_float(1.0),
		default_weights(),
		default_community_contributors(),
		default_contributor_multipliers(),
	)
}

pub fn default_remainder_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	let fifty_percent_funding_usd = Perquintill::from_percent(5) * 100_000 * ASSET_UNIT;

	IntegrationInstantiator::generate_contributions_from_total_usd(
		fifty_percent_funding_usd,
		sp_runtime::FixedU128::from_float(1.0),
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

use crate::{Polimec, PolimecRuntime, ALICE, BOB, CHARLIE};

pub fn set_oracle_prices() {
	Polimec::execute_with(|| {
		fn values(
			values: [f64; 4],
		) -> BoundedVec<(u32, FixedU128), <polimec_parachain_runtime::Runtime as orml_oracle::Config>::MaxFeedValues> {
			let [dot, usdc, usdt, plmc] = values;
			bounded_vec![
				(0u32, FixedU128::from_float(dot)),
				(420u32, FixedU128::from_float(usdc)),
				(1984u32, FixedU128::from_float(usdt)),
				(2069u32, FixedU128::from_float(plmc))
			]
		}

		let alice = Polimec::account_id_of(ALICE);
		assert_ok!(polimec_parachain_runtime::Oracle::feed_values(
			PolimecOrigin::signed(alice.clone()),
			values([4.84, 1.0, 1.0, 0.4])
		));

		let bob = Polimec::account_id_of(BOB);
		assert_ok!(polimec_parachain_runtime::Oracle::feed_values(
			PolimecOrigin::signed(bob.clone()),
			values([4.84, 1.0, 1.0, 0.4])
		));

		let charlie = Polimec::account_id_of(CHARLIE);
		assert_ok!(polimec_parachain_runtime::Oracle::feed_values(
			PolimecOrigin::signed(charlie.clone()),
			values([4.84, 1.0, 1.0, 0.4])
		));

		let expected_values = HashMap::from([
			(0u32, FixedU128::from_float(4.84)),
			(420u32, FixedU128::from_float(1.0)),
			(1984u32, FixedU128::from_float(1.0)),
			(2069u32, FixedU128::from_float(0.4)),
		]);

		for (key, value) in polimec_parachain_runtime::Oracle::get_all_values() {
			assert!(value.is_some());
			assert_eq!(expected_values.get(&key).unwrap(), &value.unwrap().value);
		}
	});
}

#[test]
fn something() {
	assert!(true);
}
