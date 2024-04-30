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

use cumulus_primitives_core::ParaId;
use sc_service::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{traits::AccountIdConversion, Perbill, Percent};

use crate::chain_spec::{get_account_id_from_seed, get_properties, Extensions, GenericChainSpec, DEFAULT_PARA_ID};
use polimec_runtime::{
	pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		InflationInfo, Range,
	},
	AccountId, AuraId as AuthorityId, Balance, MinCandidateStk, OracleProvidersMembershipConfig, Runtime,
	RuntimeGenesisConfig, PLMC,
};

pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
const COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
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

pub fn get_polimec_session_keys(keys: AuthorityId) -> polimec_runtime::SessionKeys {
	polimec_runtime::SessionKeys { aura: keys }
}

pub fn get_local_chain_spec() -> GenericChainSpec {
	let properties = get_properties("PLMC", 10, 41);
	// This account is derived from PalletId("plmc/stk") in the pallet-parachain-staking runtime config.
	// This operation can be done using https://www.shawntabrizi.com/substrate-js-utilities/
	// 1. "Module ID" to Address plmc/stk -> 5EYCAe5ij8xKJ2biBy4zUGNwdNhpz3BaS5iiuseJqTEtWQTc
	// 2. AccountId to Hex -> 0x6d6f646c706c6d632f73746b0000000000000000000000000000000000000000
	const BLOCKCHAIN_OPERATION_TREASURY: [u8; 32] =
		hex_literal::hex!["6d6f646c706c6d632f73746b0000000000000000000000000000000000000000"];

	GenericChainSpec::builder(
		polimec_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "rococo_local_testnet".into(), para_id: DEFAULT_PARA_ID.into() },
	)
	.with_name("Polimec Base Develop")
	.with_id("polimec-base")
	.with_chain_type(ChainType::Local)
	.with_protocol_id("polimec")
	.with_properties(properties)
	.with_genesis_config_patch(base_testnet_genesis(
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), None, MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), None, MinCandidateStk::get()),
			(get_account_id_from_seed::<sr25519::Public>("Charlie"), None, MinCandidateStk::get()),
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
			(BLOCKCHAIN_OPERATION_TREASURY.into(), 10_000_000 * PLMC),
		],
		DEFAULT_PARA_ID,
	))
	.build()
}

/// This was used to generate the original genesis config for the Polimec parachain.
/// Since then, the genesis `RuntimeGenesisConfig` has been updated.
/// This function is kept for historical purposes.
#[allow(unused)]
pub fn get_polkadot_base_chain_spec() -> GenericChainSpec {
	let properties = get_properties("PLMC", 10, 41);
	let id: u32 = 3344;

	const PLMC_SUDO_ACC: [u8; 32] =
		hex_literal::hex!["d4192a54c9caa4a38eeb3199232ed0d8568b22956cafb76c7d5a1afbf4e2dc38"];
	const PLMC_COL_ACC_1: [u8; 32] =
		hex_literal::hex!["6603f63a4091ba074b4384e64c6bba1dd96f6af49331ebda686b0a0f27dd961c"];
	const PLMC_COL_ACC_2: [u8; 32] =
		hex_literal::hex!["ba48ab77461ef53f9ebfdc94a12c780b57354f986e31eb2504b9e3ed580fab51"];

	GenericChainSpec::builder(
		polimec_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "polkadot".into(), para_id: id },
	)
	.with_name("Polimec Polkadot")
	.with_id("polimec-base")
	.with_chain_type(ChainType::Live)
	.with_protocol_id("polimec")
	.with_properties(properties)
	.with_genesis_config_patch(base_testnet_genesis(
		vec![
			(PLMC_COL_ACC_1.into(), None, 2 * MinCandidateStk::get()),
			(PLMC_COL_ACC_2.into(), None, 2 * MinCandidateStk::get()),
		],
		polimec_inflation_config(),
		vec![(PLMC_COL_ACC_1.into()), (PLMC_COL_ACC_2.into())],
		vec![
			(PLMC_COL_ACC_1.into(), 4 * MinCandidateStk::get()),
			(PLMC_COL_ACC_2.into(), 4 * MinCandidateStk::get()),
			(PLMC_SUDO_ACC.into(), 4 * MinCandidateStk::get()),
		],
		id.into(),
	))
	.build()
}

pub fn get_rococo_chain_spec() -> GenericChainSpec {
	let properties = get_properties("RLMC", 10, 41);
	let id: u32 = 3344;

	const PLMC_COL_ACC_1: [u8; 32] =
		hex_literal::hex!["6603f63a4091ba074b4384e64c6bba1dd96f6af49331ebda686b0a0f27dd961c"];
	const PLMC_COL_ACC_2: [u8; 32] =
		hex_literal::hex!["ba48ab77461ef53f9ebfdc94a12c780b57354f986e31eb2504b9e3ed580fab51"];

	GenericChainSpec::builder(
		polimec_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "rococo".into(), para_id: id },
	)
	.with_name("Rolimec Rococo")
	.with_id("polimec-base")
	.with_chain_type(ChainType::Live)
	.with_protocol_id("polimec")
	.with_properties(properties)
	.with_genesis_config_patch(base_testnet_genesis(
		vec![
			(PLMC_COL_ACC_1.into(), None, 2 * MinCandidateStk::get()),
			(PLMC_COL_ACC_2.into(), None, 2 * MinCandidateStk::get()),
		],
		polimec_inflation_config(),
		vec![(PLMC_COL_ACC_1.into()), (PLMC_COL_ACC_2.into())],
		vec![(PLMC_COL_ACC_1.into(), 4 * MinCandidateStk::get()), (PLMC_COL_ACC_2.into(), 4 * MinCandidateStk::get())],
		id.into(),
	))
	.build()
}

fn base_testnet_genesis(
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	inflation_config: InflationInfo<Balance>,
	initial_authorities: Vec<AccountId>,
	endowed_accounts: Vec<(AccountId, Balance)>,
	id: ParaId,
) -> serde_json::Value {
	const ENDOWMENT: Balance = 10_000_000 * PLMC;
	const STASH: Balance = ENDOWMENT / 1000;

	#[cfg(not(feature = "runtime-benchmarks"))]
	let staking_candidates = stakers.iter().map(|(accunt, _, balance)| (accunt.clone(), *balance)).collect::<Vec<_>>();
	#[cfg(feature = "runtime-benchmarks")]
	let staking_candidates: Vec<(AccountId, Balance)> = vec![];

	let accounts = endowed_accounts.iter().map(|(account, _)| account.clone()).collect::<Vec<_>>();

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.clone()
		},
		"parachainInfo": {
			"parachainId": id
		},
		"foreignAssets":  {
			"assets": vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			),
			(
				pallet_funding::types::AcceptedFundingAsset::USDC.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			),
			(
				pallet_funding::types::AcceptedFundingAsset::DOT.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			)],
			// (id, name, symbol, decimals)
			"metadata": vec![
				(pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(), b"Local USDT", b"USDT", 6),
				(pallet_funding::types::AcceptedFundingAsset::USDC.to_assethub_id(), b"Local USDC", b"USDC", 6),
				(pallet_funding::types::AcceptedFundingAsset::DOT.to_assethub_id(), b"Local DOT ", b"DOT ", 10)
			],
			// (id, account_id, amount)
			"accounts": vec![
				(pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(), accounts[0].clone(), 1000000000000u64),
				(pallet_funding::types::AcceptedFundingAsset::USDC.to_assethub_id(), accounts[0].clone(), 1000000000000u64),
				(pallet_funding::types::AcceptedFundingAsset::DOT.to_assethub_id(), accounts[0].clone(), 1000000000000u64)
			],
		},
		"parachainStaking": {
			"candidates": staking_candidates,
			"inflationConfig": inflation_config,
			"delegations": [],
			"collatorCommission": COLLATOR_COMMISSION,
			"parachainBondReservePercent": PARACHAIN_BOND_RESERVE_PERCENT,
			"blocksPerRound": BLOCKS_PER_ROUND,
			"numSelectedCandidates": NUM_SELECTED_CANDIDATES
		},
		"session": {
			"keys": initial_authorities.iter().map(|acc| {
				(
					acc.clone(),
					acc.clone(),
					get_polimec_session_keys(Into::<[u8; 32]>::into(acc.clone()).unchecked_into())
				)
			}).collect::<Vec<_>>()
		},
		"polkadotXcm": {
			"safeXcmVersion": SAFE_XCM_VERSION
		},
		"oracleProvidersMembership": OracleProvidersMembershipConfig {
			members:  accounts.clone().into_iter().take(3).collect::<Vec<AccountId>>().try_into().unwrap(),
			phantom: Default::default(),
		},
		"elections": {
			"members": endowed_accounts.iter().map(|(member, _)| member).take((endowed_accounts.len() + 1) / 2).cloned().map(|member| (member, STASH)).collect::<Vec<_>>()
		}
	})
}
