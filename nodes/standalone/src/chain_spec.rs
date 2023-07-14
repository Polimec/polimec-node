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

use polimec_standalone_runtime::{
	AccountId, BalancesConfig, GenesisConfig, SessionConfig, Signature, SudoConfig, SystemConfig,
	WASM_BINARY,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

fn polimec_properties() -> Properties {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "PLMC".into());
	properties.insert("tokenDecimals".into(), 10.into());
	// Information taken from https://github.com/paritytech/ss58-registry/blob/main/ss58-registry.json
	properties.insert("ss58Format".into(), 41.into());

	properties
}

/// Helper function to generate a crypto pair from seed
fn get_from_secret<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(seed, None)
		.unwrap_or_else(|_| panic!("Invalid string '{seed}'"))
		.public()
}

/// Helper function to generate an account ID from seed
fn get_account_id_from_secret<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_secret::<TPublic>(seed)).into_account()
}

/// Helper function to generate an authority key for Aura
fn get_authority_keys_from_secret(seed: &str) -> (AccountId, AuraId, GrandpaId) {
	(
		get_account_id_from_secret::<sr25519::Public>(seed),
		get_from_secret::<AuraId>(seed),
		get_from_secret::<GrandpaId>(seed),
	)
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

type AccountPublic = <Signature as Verify>::Signer;

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Polimec Development",
		// ID
		"polimec-dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![get_authority_keys_from_secret("//Alice")],
				// Sudo account
				get_account_id_from_secret::<sr25519::Public>("//Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_secret::<sr25519::Public>("//Alice"),
					get_account_id_from_secret::<sr25519::Public>("//Bob"),
					get_account_id_from_secret::<sr25519::Public>("//Charlie"),
					get_account_id_from_secret::<sr25519::Public>("//Dave"),
					get_account_id_from_secret::<sr25519::Public>("//Eve"),
					get_account_id_from_secret::<sr25519::Public>("//Ferdie"),
				],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(polimec_properties()),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Polimec Local Testnet",
		// ID
		"polimec-local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					get_authority_keys_from_secret("//Alice"),
					get_authority_keys_from_secret("//Bob"),
				],
				// Sudo account
				get_account_id_from_secret::<sr25519::Public>("//Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_secret::<sr25519::Public>("//Alice"),
					get_account_id_from_secret::<sr25519::Public>("//Bob"),
					get_account_id_from_secret::<sr25519::Public>("//Charlie"),
					get_account_id_from_secret::<sr25519::Public>("//Dave"),
					get_account_id_from_secret::<sr25519::Public>("//Eve"),
					get_account_id_from_secret::<sr25519::Public>("//Ferdie"),
				],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		None,
		Some(polimec_properties()),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 42)).collect(),
		},
		aura: Default::default(),
		grandpa: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		council: Default::default(),
		technical_committee: Default::default(),
		democracy: Default::default(),
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						polimec_standalone_runtime::opaque::SessionKeys {
							aura: x.1.clone(),
							grandpa: x.2.clone(),
						},
					)
				})
				.collect::<Vec<_>>(),
		},
		..Default::default()
	}
}
