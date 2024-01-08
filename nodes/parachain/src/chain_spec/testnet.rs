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
use frame_benchmarking::frame_support::bounded_vec;
use polimec_parachain_runtime::{
	pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		InflationInfo, Range,
	},
	AccountId, AuraId as AuthorityId, Balance, BalancesConfig, CouncilConfig, LinearVestingConfig, MinCandidateStk,
	OracleProvidersMembershipConfig, ParachainInfoConfig, ParachainStakingConfig, PolkadotXcmConfig, Runtime,
	RuntimeGenesisConfig, SessionConfig, StatemintAssetsConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
	EXISTENTIAL_DEPOSIT, PLMC,
};
use sc_service::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{traits::AccountIdConversion, Perbill, Percent};

use crate::chain_spec::{get_account_id_from_seed, DEFAULT_PARA_ID};

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

pub fn get_testnet_session_keys(keys: AuthorityId) -> polimec_parachain_runtime::SessionKeys {
	polimec_parachain_runtime::SessionKeys { aura: keys }
}

#[cfg(feature = "std")]
pub fn get_chain_spec_testing() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = polimec_parachain_runtime::WASM_BINARY.ok_or("No WASM")?;

	Ok(ChainSpec::from_genesis(
		"Polimec Develop",
		"polimec",
		ChainType::Local,
		move || {
			testing_genesis(
				wasm,
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
	))
}

pub fn get_chain_spec_dev() -> Result<ChainSpec, String> {
	let properties = get_properties("RLMC", 10, 41);
	let wasm = polimec_parachain_runtime::WASM_BINARY.ok_or("No WASM")?;

	Ok(ChainSpec::from_genesis(
		"Rolimec Develop",
		"polimec",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm,
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
	))
}

pub fn get_prod_chain_spec() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = polimec_parachain_runtime::WASM_BINARY.ok_or("No WASM")?;

	// TODO: Update this after reserving a ParaId
	let id: u32 = 4261;

	const PLMC_SUDO_ACC: [u8; 32] =
		hex_literal::hex!["d4192a54c9caa4a38eeb3199232ed0d8568b22956cafb76c7d5a1afbf4e2dc38"];
	const PLMC_COL_ACC_1: [u8; 32] =
		hex_literal::hex!["6603f63a4091ba074b4384e64c6bba1dd96f6af49331ebda686b0a0f27dd961c"];
	const PLMC_COL_ACC_2: [u8; 32] =
		hex_literal::hex!["ba48ab77461ef53f9ebfdc94a12c780b57354f986e31eb2504b9e3ed580fab51"];

	Ok(ChainSpec::from_genesis(
		"Polimec Kusama Testnet",
		"polimec",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm,
				vec![
					(PLMC_COL_ACC_1.into(), None, 2 * MinCandidateStk::get()),
					(PLMC_COL_ACC_2.into(), None, 2 * MinCandidateStk::get()),
				],
				polimec_inflation_config(),
				vec![(PLMC_COL_ACC_1.into()), (PLMC_COL_ACC_2.into())],
				vec![
					(PLMC_COL_ACC_1.into(), 3 * MinCandidateStk::get()),
					(PLMC_COL_ACC_2.into(), 3 * MinCandidateStk::get()),
				],
				PLMC_SUDO_ACC.into(),
				id.into(),
			)
		},
		vec![],
		None,
		Some("polimec"),
		None,
		Some(properties),
		Extensions { relay_chain: "polkadot".into(), para_id: id },
	))
}

#[allow(clippy::too_many_arguments)]
fn testnet_genesis(
	wasm_binary: &[u8],
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo<Balance>,
	initial_authorities: Vec<AccountId>,
	mut endowed_accounts: Vec<(AccountId, Balance)>,
	sudo_account: AccountId,
	id: ParaId,
) -> RuntimeGenesisConfig {
	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();
	endowed_accounts
		.push((<Runtime as pallet_funding::Config>::PalletId::get().into_account_truncating(), EXISTENTIAL_DEPOSIT));
	RuntimeGenesisConfig {
		system: SystemConfig { code: wasm_binary.to_vec(), ..Default::default() },
		balances: BalancesConfig { balances: endowed_accounts.clone() },
		statemint_assets: StatemintAssetsConfig {
			assets: vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_statemint_id(),
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
		polimec_funding: Default::default(),
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
						get_testnet_session_keys(Into::<[u8; 32]>::into(acc.clone()).unchecked_into()),
					)
				})
				.collect::<Vec<_>>(),
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION), ..Default::default() },
		treasury: Default::default(),
		sudo: SudoConfig { key: Some(sudo_account) },
		council: CouncilConfig { members: accounts.clone(), phantom: Default::default() },
		technical_committee: TechnicalCommitteeConfig { members: accounts, phantom: Default::default() },
		democracy: Default::default(),
		linear_vesting: LinearVestingConfig { vesting: vec![] },
		oracle_providers_membership: OracleProvidersMembershipConfig {
			members: bounded_vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
			],
			phantom: Default::default(),
		},
	}
}

use pallet_funding::{instantiator::UserToUSDBalance, *};
use sp_runtime::BoundedVec;

#[cfg(feature = "std")]
mod testing_helpers {
	use super::*;
	use frame_benchmarking::frame_support::assert_ok;
	use macros::generate_accounts;
	use polimec_parachain_runtime::{AccountId, FixedU128};
	use sp_core::H256;
	use sp_runtime::traits::ConstU32;
	use std::collections::HashMap;

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
		ALICE, BOB, CHARLIE, ISSUER, EVAL_1, EVAL_2, EVAL_3, EVAL_4, BIDDER_1, BIDDER_2, BIDDER_3, BIDDER_4, BIDDER_5,
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
	pub fn default_bidder_multipliers() -> Vec<u8> {
		vec![20u8, 3u8, 15u8, 13u8, 9u8]
	}
	pub fn default_community_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 5u8, 3u8, 1u8, 2u8]
	}
	pub fn default_remainder_contributor_multipliers() -> Vec<u8> {
		vec![1u8, 10u8, 3u8, 2u8, 4u8]
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

	use polimec_parachain_runtime::Runtime as T;
	pub type GenesisInstantiator = pallet_funding::instantiator::Instantiator<
		T,
		<T as pallet_funding::Config>::AllPalletsWithoutSystem,
		<T as pallet_funding::Config>::RuntimeEvent,
	>;
}

#[cfg(feature = "std")]
#[allow(clippy::too_many_arguments)]
fn testing_genesis(
	wasm_binary: &[u8],
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo<Balance>,
	initial_authorities: Vec<AccountId>,
	mut endowed_accounts: Vec<(AccountId, Balance)>,
	sudo_account: AccountId,
	id: ParaId,
) -> RuntimeGenesisConfig {
	use instantiator::TestProjectParams;
	use sp_runtime::{traits::PhantomData, FixedPointNumber, Perquintill};
	use testing_helpers::*;

	// only used to generate some values, and not for chain interactions
	let funding_percent = 93u64;
	let project_metadata = default_project(ISSUER.into(), 0u32);
	let min_price = project_metadata.minimum_price;
	let twenty_percent_funding_usd = Perquintill::from_percent(funding_percent) *
		(project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size.0 + project_metadata.total_allocation_size.1)
			.unwrap());
	let evaluations = default_evaluations();
	let bids = GenesisInstantiator::generate_bids_from_total_usd(
		Percent::from_percent(50u8) * twenty_percent_funding_usd,
		min_price,
		default_weights(),
		default_bidders(),
		default_bidder_multipliers(),
	);
	let community_contributions = GenesisInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(30u8) * twenty_percent_funding_usd,
		min_price,
		default_weights(),
		default_community_contributors(),
		default_community_contributor_multipliers(),
	);
	let remainder_contributions = GenesisInstantiator::generate_contributions_from_total_usd(
		Percent::from_percent(20u8) * twenty_percent_funding_usd,
		min_price,
		default_weights(),
		default_remainder_contributors(),
		default_remainder_contributor_multipliers(),
	);

	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();
	endowed_accounts.push((<Runtime as Config>::PalletId::get().into_account_truncating(), EXISTENTIAL_DEPOSIT));
	RuntimeGenesisConfig {
		system: SystemConfig { code: wasm_binary.to_vec(), ..Default::default() },
		oracle_providers_membership: OracleProvidersMembershipConfig {
			members: bounded_vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Charlie"),
			],
			phantom: Default::default(),
		},
		polimec_funding: polimec_parachain_runtime::PolimecFundingConfig {
			starting_projects: vec![
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::FundingSuccessful,
					metadata: default_project(ISSUER.into(), 0u32),
					issuer: ISSUER.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: community_contributions.clone(),
					remainder_contributions: remainder_contributions.clone(),
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::RemainderRound,
					metadata: default_project(ISSUER.into(), 1u32),
					issuer: ISSUER.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: community_contributions.clone(),
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::CommunityRound,
					metadata: default_project(ISSUER.into(), 2u32),
					issuer: ISSUER.into(),
					evaluations: evaluations.clone(),
					bids: bids.clone(),
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::AuctionRound(AuctionPhase::English),
					metadata: default_project(ISSUER.into(), 3u32),
					issuer: ISSUER.into(),
					evaluations: evaluations.clone(),
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::EvaluationRound,
					metadata: default_project(ISSUER.into(), 4u32),
					issuer: ISSUER.into(),
					evaluations: vec![],
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
				TestProjectParams::<Runtime> {
					expected_state: ProjectStatus::Application,
					metadata: default_project(ISSUER.into(), 5u32),
					issuer: ISSUER.into(),
					evaluations: vec![],
					bids: vec![],
					community_contributions: vec![],
					remainder_contributions: vec![],
				},
			],
			phantom: PhantomData,
		},
		balances: BalancesConfig { balances: endowed_accounts.clone() },
		statemint_assets: StatemintAssetsConfig {
			assets: vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_statemint_id(),
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
						get_testnet_session_keys(Into::<[u8; 32]>::into(acc.clone()).unchecked_into()),
					)
				})
				.collect::<Vec<_>>(),
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION), ..Default::default() },
		treasury: Default::default(),
		sudo: SudoConfig { key: Some(sudo_account) },
		council: CouncilConfig { members: accounts.clone(), phantom: Default::default() },
		technical_committee: TechnicalCommitteeConfig { members: accounts, phantom: Default::default() },
		democracy: Default::default(),
		linear_vesting: LinearVestingConfig { vesting: vec![] },
	}
}
