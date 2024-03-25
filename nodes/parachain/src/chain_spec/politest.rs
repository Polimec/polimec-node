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

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! Polimec Testnet chain specification

use cumulus_primitives_core::ParaId;
use politest_runtime::{
	pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		InflationInfo, Range,
	},
	AccountId, AuraId as AuthorityId, Balance, MinCandidateStk, OracleProvidersMembershipConfig, Runtime,
	RuntimeGenesisConfig, EXISTENTIAL_DEPOSIT, PLMC,
};
use sc_service::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{bounded_vec, traits::AccountIdConversion, Perbill, Percent};

use crate::chain_spec::{get_account_id_from_seed, GenericChainSpec, DEFAULT_PARA_ID};

use super::{get_properties, Extensions};

const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

const COLLATOR_COMMISSION: Perbill = Perbill::from_percent(30);
const PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
const BLOCKS_PER_ROUND: u32 = 2 * 10;
const NUM_SELECTED_CANDIDATES: u32 = 5;
pub fn polimec_inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
		)
	}

	let annual =
		Range { min: Perbill::from_percent(2), ideal: Perbill::from_percent(3), max: Perbill::from_percent(3) };

	InflationInfo {
		// staking expectations
		expect: Range { min: 100_000 * PLMC, ideal: 200_000 * PLMC, max: 500_000 * PLMC },
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

pub fn get_politest_session_keys(keys: AuthorityId) -> politest_runtime::SessionKeys {
	politest_runtime::SessionKeys { aura: keys }
}

#[cfg(feature = "std")]
pub fn get_populated_chain_spec() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = politest_runtime::WASM_BINARY.ok_or("No WASM")?;

	#[allow(deprecated)]
	Ok(ChainSpec::from_genesis(
		"Polimec Develop",
		"polimec",
		ChainType::Local,
		move || {
			testing_genesis(
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), None, 2 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), None, 2 * MinCandidateStk::get()),
				],
				polimec_inflation_config(),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
				],
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), 5 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), 5 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Charlie"), 5 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Dave"), 5 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Eve"), 5 * MinCandidateStk::get()),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 5 * MinCandidateStk::get()),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				DEFAULT_PARA_ID,
			)
		},
		vec![],
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "rococo-local".into(), para_id: DEFAULT_PARA_ID.into() },
		wasm,
	))
}

pub fn get_local_chain_spec() -> GenericChainSpec {
	let properties = get_properties("RLMC", 10, 41);

	GenericChainSpec::builder(
		politest_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "rococo-local".into(), para_id: DEFAULT_PARA_ID.into() },
	)
	.with_name("Rolimec Develop")
	.with_id("rolimec-dev")
	.with_chain_type(ChainType::Local)
	.with_protocol_id("polimec")
	.with_properties(properties)
	.with_genesis_config_patch(testnet_genesis(
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), None, 2 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), None, 2 * MinCandidateStk::get()),
		],
		polimec_inflation_config(),
		vec![get_account_id_from_seed::<sr25519::Public>("Alice"), get_account_id_from_seed::<sr25519::Public>("Bob")],
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), 5 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), 5 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Charlie"), 5 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Dave"), 5 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Eve"), 5 * MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 5 * MinCandidateStk::get()),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		DEFAULT_PARA_ID,
	))
	.build()
}

#[allow(clippy::too_many_arguments)]
fn testnet_genesis(
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo<Balance>,
	initial_authorities: Vec<AccountId>,
	mut endowed_accounts: Vec<(AccountId, Balance)>,
	sudo_account: AccountId,
	id: ParaId,
) -> serde_json::Value {
	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();

	let funding_accounts = vec![
		(<Runtime as pallet_funding::Config>::PalletId::get().into_account_truncating(), EXISTENTIAL_DEPOSIT),
		(politest_runtime::TreasuryAccount::get(), EXISTENTIAL_DEPOSIT),
	];
	endowed_accounts.append(&mut funding_accounts.clone());

	serde_json::json!({
		"balances": { "balances": endowed_accounts.clone() },
		"foreignAssets":  {
			"assets": vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				false,
				10,
			)],
		},
		"parachainInfo":  { "parachainId": id },
		"parachainStaking":  {
			"candidates": stakers.iter().map(|(accunt, _, balance)| (accunt.clone(), *balance)).collect::<Vec<_>>(),
			"inflationConfig": inflation_config,
			"collatorCommission": COLLATOR_COMMISSION,
			"parachainBondReservePercent": PARACHAIN_BOND_RESERVE_PERCENT,
			"blocksPerRound": BLOCKS_PER_ROUND,
			"numSelectedCandidates": NUM_SELECTED_CANDIDATES,
		},
		"session":  {
			"keys": initial_authorities
				.iter()
				.map(|acc| {
					(
						acc.clone(),
						acc.clone(),
						get_politest_session_keys(Into::<[u8; 32]>::into(acc.clone()).unchecked_into()),
					)
				})
				.collect::<Vec<_>>(),
		},
		"polkadotXcm":  { "safeXcmVersion": Some(SAFE_XCM_VERSION) },
		"sudo":  { "key": Some(sudo_account) },
		"council":  { "members": accounts.clone() },
		"technicalCommittee":  {
			"members": accounts.clone().into_iter().take(5).collect::<Vec<AccountId>>(),
		},
		"oracleProvidersMembership": OracleProvidersMembershipConfig {
			members: bounded_vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
			],
			phantom: Default::default(),
		},
	})
}

#[cfg(feature = "std")]
mod testing_helpers {
	use super::*;
	pub use macros::generate_accounts;
	pub use pallet_funding::{instantiator::UserToUSDBalance, AuctionPhase, ProjectStatus, *};
	pub use sp_core::H256;
	pub use sp_runtime::{
		traits::{ConstU32, Get, PhantomData},
		BoundedVec, FixedPointNumber,
	};

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

	generate_accounts!(
		ALICE, BOB, CHARLIE, ISSUER, ISSUER_1, ISSUER_2, ISSUER_3, ISSUER_4, ISSUER_5, ISSUER_6, EVAL_1, EVAL_2, EVAL_3, EVAL_4, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5,
		BIDDER_6, BUYER_1, BUYER_2, BUYER_3, BUYER_4, BUYER_5, BUYER_6,
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

	pub fn project_metadata(issuer: AccountId, nonce: u32) -> ProjectMetadataOf<politest_runtime::Runtime> {
		ProjectMetadata {
			token_information: CurrencyMetadata {
				name: bounded_name(),
				symbol: bounded_symbol(),
				decimals: ASSET_DECIMALS,
			},
			mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
			total_allocation_size: 1_000_000 * ASSET_UNIT,
			auction_round_allocation_percentage: Percent::from_percent(50u8),
			minimum_price: PriceOf::<politest_runtime::Runtime>::from_float(10.0),
			bidding_ticket_sizes: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
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
			offchain_information_hash: Some(metadata_hash(nonce)),
		}
	}
	pub fn default_evaluations() -> Vec<UserToUSDBalance<politest_runtime::Runtime>> {
		vec![
			UserToUSDBalance::new(EVAL_1.into(), 500_000 * PLMC),
			UserToUSDBalance::new(EVAL_2.into(), 250_000 * PLMC),
			UserToUSDBalance::new(EVAL_3.into(), 320_000 * PLMC),
		]
	}
	pub fn default_bidders() -> Vec<AccountId> {
		vec![BIDDER_1.into(), BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(), BIDDER_5.into()]
	}
	pub fn default_bidder_multipliers() -> Vec<u8> {
		vec![10u8, 3u8, 8u8, 7u8, 9u8]
	}
	pub fn default_community_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}
	pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 1u8, 1u8, 1u8, 1u8]
	}

	pub fn default_community_contributors() -> Vec<AccountId> {
		vec![BUYER_1.into(), BUYER_2.into(), BUYER_3.into(), BUYER_4.into(), BUYER_5.into()]
	}

	pub fn default_remainder_contributors() -> Vec<AccountId> {
		vec![EVAL_1.into(), BIDDER_3.into(), BUYER_4.into(), BUYER_6.into(), BIDDER_6.into()]
	}
	pub fn hashed(data: impl AsRef<[u8]>) -> sp_core::H256 {
		<sp_runtime::traits::BlakeTwo256 as sp_runtime::traits::Hash>::hash(data.as_ref())
	}

	use politest_runtime::Runtime as T;
	pub type GenesisInstantiator = pallet_funding::instantiator::Instantiator<
		T,
		<T as pallet_funding::Config>::AllPalletsWithoutSystem,
		<T as pallet_funding::Config>::RuntimeEvent,
	>;
}

#[cfg(feature = "std")]
#[allow(clippy::too_many_arguments)]
fn testing_genesis(
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo<Balance>,
	initial_authorities: Vec<AccountId>,
	mut endowed_accounts: Vec<(AccountId, Balance)>,
	sudo_account: AccountId,
	id: ParaId,
) -> RuntimeGenesisConfig {
	use pallet_funding::instantiator::TestProjectParams;
	use politest_runtime::{
		BalancesConfig, CouncilConfig, ForeignAssetsConfig, ParachainInfoConfig, ParachainStakingConfig,
		PolkadotXcmConfig, SessionConfig, SudoConfig, TechnicalCommitteeConfig,
	};
	use testing_helpers::*;

	// only used to generate some values, and not for chain interactions
	let default_project_metadata = project_metadata(ISSUER.into(), 0u32);
	let min_price = default_project_metadata.minimum_price;
	let usdt_funding_amount =
		default_project_metadata.minimum_price.checked_mul_int(default_project_metadata.total_allocation_size).unwrap();
	let evaluations = default_evaluations();
	let bids = GenesisInstantiator::generate_bids_from_total_usd(
		Percent::from_percent(40u8) * usdt_funding_amount,
		min_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = GenesisInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(50u8) * usdt_funding_amount,
		min_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = GenesisInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(5u8) * usdt_funding_amount,
		min_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();
	endowed_accounts
		.push((<Runtime as pallet_funding::Config>::PalletId::get().into_account_truncating(), EXISTENTIAL_DEPOSIT));
	endowed_accounts.push((<Runtime as pallet_funding::Config>::ContributionTreasury::get(), EXISTENTIAL_DEPOSIT));
	RuntimeGenesisConfig {
		system: Default::default(),
		oracle_providers_membership: OracleProvidersMembershipConfig {
			members: bounded_vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
			],
			phantom: Default::default(),
		},
		polimec_funding: politest_runtime::PolimecFundingConfig {
			starting_projects: vec![
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::FundingSuccessful,
					metadata: project_metadata(ISSUER_1.into(), 0u32),
					issuer: ISSUER_1.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: community_contributions.clone(),
					remainder_contributions: remainder_contributions.clone(),
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::RemainderRound,
					metadata: project_metadata(ISSUER_2.into(), 1u32),
					issuer: ISSUER_2.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: community_contributions.clone(),
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::CommunityRound,
					metadata: project_metadata(ISSUER_3.into(), 2u32),
					issuer: ISSUER_3.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
					metadata: project_metadata(ISSUER_4.into(), 3u32),
					issuer: ISSUER_4.into(),
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::EvaluationRound,
					metadata: project_metadata(ISSUER_5.into(), 4u32),
					issuer: ISSUER_5.into(),
					evaluations: vec![],
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::Application,
					metadata: project_metadata(ISSUER_6.into(), 5u32),
					issuer: ISSUER_6.into(),
					evaluations: vec![],
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
			],
			phantom: PhantomData,
		},
		balances: BalancesConfig { balances: endowed_accounts.clone() },
		foreign_assets: ForeignAssetsConfig {
			assets: vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(),
				<Runtime as pallet_funding::Config>::PalletId::get().into_account_truncating(),
				false,
				10,
			)],
			metadata: vec![],
			accounts: vec![],
		},
		parachain_info: ParachainInfoConfig { parachain_id: id, ..Default::default() },
		parachain_staking: ParachainStakingConfig {
			candidates: stakers.iter().map(|(accunt, _, balance)| (accunt.clone(), *balance)).collect::<Vec<_>>(),
			inflation_config,
			delegations: vec![],
			collator_commission: COLLATOR_COMMISSION,
			parachain_bond_reserve_percent: PARACHAIN_BOND_RESERVE_PERCENT,
			blocks_per_round: BLOCKS_PER_ROUND,
			num_selected_candidates: NUM_SELECTED_CANDIDATES,
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|acc| {
					(
						acc.clone(),
						acc.clone(),
						get_politest_session_keys(Into::<[u8; 32]>::into(acc.clone()).unchecked_into()),
					)
				})
				.collect::<Vec<_>>(),
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION), ..Default::default() },
		treasury: Default::default(),
		sudo: SudoConfig { key: Some(sudo_account) },
		council: CouncilConfig { members: accounts.clone(), phantom: Default::default() },
		technical_committee: TechnicalCommitteeConfig {
			members: accounts.clone().into_iter().take(5).collect(),
			phantom: Default::default(),
		},
		democracy: Default::default(),
		elections: Default::default(),
		vesting: Default::default(),
	}
}
