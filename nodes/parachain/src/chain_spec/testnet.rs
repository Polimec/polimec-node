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
use runtime_common::{
	constants::{polimec_inflation_config, staking::MinCollatorStake, MAX_COLLATOR_STAKE, PLMC},
	AuthorityId,
};

use polimec_parachain_runtime::{
	AccountId, Balance, BalancesConfig, CouncilConfig, CredentialsConfig, GenesisConfig,
	InflationInfo, ParachainInfoConfig, ParachainStakingConfig, PolkadotXcmConfig, SessionConfig,
	SessionKeys, SudoConfig, SystemConfig, TechnicalCommitteeConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::sr25519;

use crate::chain_spec::{get_account_id_from_seed, get_from_seed, DEFAULT_PARA_ID, TELEMETRY_URL};

use super::{get_properties, Extensions};

const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub fn get_chain_spec_dev() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = WASM_BINARY.ok_or("No WASM")?;

	Ok(ChainSpec::from_genesis(
		"Polimec Develop",
		"polimec-dev",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm,
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						None,
						2 * MinCollatorStake::get(),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						None,
						2 * MinCollatorStake::get(),
					),
				],
				polimec_inflation_config(),
				MAX_COLLATOR_STAKE,
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_from_seed::<AuthorityId>("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_from_seed::<AuthorityId>("Bob"),
					),
				],
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Charlie"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Dave"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Eve"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Alice//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Bob//stash"), 10000000 * PLMC),
					(
						get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
						10000000 * PLMC,
					),
					(get_account_id_from_seed::<sr25519::Public>("Dave//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Eve//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"), 10000000 * PLMC),
				],
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

pub fn get_chain_spec() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = WASM_BINARY.ok_or("No WASM")?;
	let id: ParaId = 2105.into();

	Ok(ChainSpec::from_genesis(
		"Polimec",
		"polimec-prod",
		ChainType::Live,
		move || {
			testnet_genesis(
				wasm,
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						None,
						2 * MinCollatorStake::get(),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						None,
						2 * MinCollatorStake::get(),
					),
				],
				polimec_inflation_config(),
				MAX_COLLATOR_STAKE,
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_from_seed::<AuthorityId>("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_from_seed::<AuthorityId>("Bob"),
					),
				],
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Charlie"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Dave"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Eve"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Alice//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Bob//stash"), 10000000 * PLMC),
					(
						get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
						10000000 * PLMC,
					),
					(get_account_id_from_seed::<sr25519::Public>("Dave//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Eve//stash"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"), 10000000 * PLMC),
				],
				id,
			)
		},
		vec![],
		Some(
			TelemetryEndpoints::new(vec![(TELEMETRY_URL.to_string(), 0)])
				.expect("Polimec telemetry url is valid; qed"),
		),
		Some("polimec"),
		None,
		Some(properties),
		Extensions { relay_chain: "rococo-local".into(), para_id: id.into() },
	))
}

#[allow(clippy::too_many_arguments)]
fn testnet_genesis(
	wasm_binary: &[u8],
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo,
	max_candidate_stake: Balance,
	initial_authorities: Vec<(AccountId, AuthorityId)>,
	endowed_accounts: Vec<(AccountId, Balance)>,
	id: ParaId,
) -> GenesisConfig {
	// type VestingPeriod = BlockNumber;
	// type LockingPeriod = BlockNumber;

	// // vesting and locks as initially designed
	// let claimable_accounts_json = &include_bytes!("../../res/genesis/claimable-accounts.json")[..];
	// let claimable_accounts: Vec<(AccountId, Balance, VestingPeriod, LockingPeriod)> =
	// 	serde_json::from_slice(claimable_accounts_json)
	// 		.expect("The file genesis_accounts.json exists and is valid; qed");

	// // botlabs account should not be migrated but some have vesting
	// let owned_accounts_json = &include_bytes!("../../res/genesis/owned-accounts.json")[..];
	// let owned_accounts: Vec<(AccountId, Balance, VestingPeriod, LockingPeriod)> =
	// 	serde_json::from_slice(owned_accounts_json)
	// 		.expect("The file botlabs_accounts.json exists and is valid; qed");

	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();

	GenesisConfig {
		system: SystemConfig { code: wasm_binary.to_vec() },
		balances: BalancesConfig { balances: endowed_accounts.clone() },
		parachain_info: ParachainInfoConfig { parachain_id: id },
		parachain_staking: ParachainStakingConfig {
			stakers,
			inflation_config,
			max_candidate_stake,
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		credentials: CredentialsConfig {
			issuers: accounts.clone(),
			retails: accounts.clone(),
			professionals: accounts.clone(),
			institutionals: accounts.clone(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|(acc, key)| (acc.clone(), acc.clone(), SessionKeys { aura: key.clone() }))
				.collect::<Vec<_>>(),
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		treasury: Default::default(),
		sudo: SudoConfig { key: Some(accounts.first().expect("").to_owned()) },
		council: CouncilConfig {
			members: initial_authorities.iter().map(|(acc, _)| acc).cloned().collect(),
			phantom: Default::default(),
		},
		technical_committee: TechnicalCommitteeConfig {
			members: initial_authorities.iter().map(|(acc, _)| acc).cloned().collect(),
			phantom: Default::default(),
		},
		democracy: Default::default(),
	}
}
