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

//! Polimec Shell chain specification

use crate::chain_spec::Extensions;
use cumulus_primitives_core::ParaId;
use polimec_base_runtime as base_runtime;
use sc_service::ChainType;
use sp_core::sr25519;

use crate::chain_spec::{get_account_id_from_seed, get_from_seed, get_properties, DEFAULT_PARA_ID};
use base_runtime::{
	polimec_inflation_config, staking::MinCollatorStake, AccountId, AuraId as AuthorityId, Balance,
	BalancesConfig, GenesisConfig, InflationInfo, ParachainInfoConfig, ParachainStakingConfig,
	PolkadotXcmConfig, SessionConfig, SudoConfig, SystemConfig, MAX_COLLATOR_STAKE, PLMC,
};

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Specialized `ChainSpec` for the shell parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub fn get_local_base_chain_spec() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = base_runtime::WASM_BINARY.ok_or("No WASM")?;

	Ok(ChainSpec::from_genesis(
		"Polimec Base Develop",
		"polimec",
		ChainType::Local,
		move || {
			base_testnet_genesis(
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
		Some("polimec"),
		None,
		Some(properties),
		Extensions { relay_chain: "rococo_local_testnet".into(), para_id: DEFAULT_PARA_ID.into() },
	))
}

pub fn get_live_base_chain_spec() -> Result<ChainSpec, String> {
	let properties = get_properties("PLMC", 10, 41);
	let wasm = base_runtime::WASM_BINARY.ok_or("No WASM")?;

	// TODO: Update this after reserving a ParaId
	let id: u32 = 2105;

	Ok(ChainSpec::from_genesis(
		"Polimec Base",
		"polimec",
		ChainType::Live,
		move || {
			base_testnet_genesis(
				wasm,
				// TODO: Update stakers
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
				// TODO: Update initial authorities
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
				// TODO: Update initial balances
				vec![
					(get_account_id_from_seed::<sr25519::Public>("Alice"), 10000000 * PLMC),
					(get_account_id_from_seed::<sr25519::Public>("Bob"), 10000000 * PLMC),
				],
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

fn base_testnet_genesis(
	wasm_binary: &[u8],
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo,
	max_candidate_stake: Balance,
	initial_authorities: Vec<(AccountId, AuthorityId)>,
	endowed_accounts: Vec<(AccountId, Balance)>,
	id: ParaId,
) -> GenesisConfig {
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
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|(acc, key)| {
					(acc.clone(), acc.clone(), base_runtime::SessionKeys { aura: key.clone() })
				})
				.collect::<Vec<_>>(),
		},
		treasury: Default::default(),
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		sudo: SudoConfig { key: Some(accounts.first().expect("").to_owned()) },
		transaction_payment: Default::default(),
	}
}
