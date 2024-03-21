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

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
use politest_runtime::{
	pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		Range,
	},
	PLMC,
};
use polkadot_primitives::{AssignmentId, ValidatorId};
pub use polkadot_runtime_parachains::configuration::HostConfiguration;
use polkadot_service::chain_spec::get_authority_keys_from_seed_no_beefy;
use sc_consensus_grandpa::AuthorityId as GrandpaId;
use sp_arithmetic::Percent;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, storage::Storage, Pair, Public};
use sp_runtime::{bounded_vec, BuildStorage, Perbill};

pub use xcm;
use xcm_emulator::get_account_id_from_seed;

pub const XCM_V2: u32 = 3;
pub const XCM_V3: u32 = 2;
pub const REF_TIME_THRESHOLD: u64 = 33;
pub const PROOF_SIZE_THRESHOLD: u64 = 33;
pub const INITIAL_DEPOSIT: u128 = 420_0_000_000_000;
const BLOCKS_PER_ROUND: u32 = 6 * 100;

fn polimec_inflation_config() -> politest_runtime::pallet_parachain_staking::InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
		)
	}

	let annual =
		Range { min: Perbill::from_percent(2), ideal: Perbill::from_percent(3), max: Perbill::from_percent(3) };

	politest_runtime::pallet_parachain_staking::InflationInfo {
		// staking expectations
		expect: Range { min: 100_000 * PLMC, ideal: 200_000 * PLMC, max: 500_000 * PLMC },
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None).expect("static values are valid; qed").public()
}

pub mod accounts {
	use super::*;
	pub const ALICE: &str = "Alice";
	pub const BOB: &str = "Bob";
	pub const CHARLIE: &str = "Charlie";
	pub const DAVE: &str = "Dave";
	pub const EVE: &str = "Eve";
	pub const FERDIE: &str = "Ferdei";
	pub const ALICE_STASH: &str = "Alice//stash";
	pub const BOB_STASH: &str = "Bob//stash";
	pub const CHARLIE_STASH: &str = "Charlie//stash";
	pub const DAVE_STASH: &str = "Dave//stash";
	pub const EVE_STASH: &str = "Eve//stash";
	pub const FERDIE_STASH: &str = "Ferdie//stash";

	pub fn init_balances() -> Vec<AccountId> {
		vec![
			get_account_id_from_seed::<sr25519::Public>(ALICE),
			get_account_id_from_seed::<sr25519::Public>(BOB),
			get_account_id_from_seed::<sr25519::Public>(CHARLIE),
			get_account_id_from_seed::<sr25519::Public>(DAVE),
			get_account_id_from_seed::<sr25519::Public>(EVE),
			get_account_id_from_seed::<sr25519::Public>(FERDIE),
			get_account_id_from_seed::<sr25519::Public>(ALICE_STASH),
			get_account_id_from_seed::<sr25519::Public>(BOB_STASH),
			get_account_id_from_seed::<sr25519::Public>(CHARLIE_STASH),
			get_account_id_from_seed::<sr25519::Public>(DAVE_STASH),
			get_account_id_from_seed::<sr25519::Public>(EVE_STASH),
			get_account_id_from_seed::<sr25519::Public>(FERDIE_STASH),
		]
	}
}

pub mod collators {
	use super::*;

	pub fn invulnerables_asset_hub() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), get_from_seed::<AssetHubPolkadotAuraId>("Alice")),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AssetHubPolkadotAuraId>("Bob")),
		]
	}

	pub fn invulnerables() -> Vec<(AccountId, AuraId)> {
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), get_from_seed::<AuraId>("Alice")),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
		]
	}

	pub fn initial_authorities() -> Vec<(AccountId, AuraId)> {
		vec![
			(get_account_id_from_seed::<sr25519::Public>("COLL_1"), get_from_seed::<AuraId>("COLL_1")),
			(get_account_id_from_seed::<sr25519::Public>("COLL_2"), get_from_seed::<AuraId>("COLL_2")),
		]
	}
}

pub mod validators {
	use super::*;

	pub fn initial_authorities(
	) -> Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, ValidatorId, AssignmentId, AuthorityDiscoveryId)> {
		vec![get_authority_keys_from_seed_no_beefy("Alice")]
	}
}

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

// Helper functions to calculate chain accounts

// Polkadot
pub mod polkadot {
	use super::*;
	pub const ED: Balance = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	const STASH: u128 = 100 * polkadot_runtime_constants::currency::UNITS;

	pub fn get_host_config() -> HostConfiguration<BlockNumber> {
		HostConfiguration {
			max_upward_queue_count: 10,
			max_upward_queue_size: 51200,
			max_upward_message_size: 51200,
			max_upward_message_num_per_candidate: 10,
			max_downward_message_size: 51200,
			..Default::default()
		}
	}

	fn session_keys(
		babe: BabeId,
		grandpa: GrandpaId,
		im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
	) -> polkadot_runtime::SessionKeys {
		polkadot_runtime::SessionKeys { babe, grandpa, im_online, para_validator, para_assignment, authority_discovery }
	}

	pub fn genesis() -> Storage {
		let genesis_config = polkadot_runtime::RuntimeGenesisConfig {
			system: polkadot_runtime::SystemConfig {
				code: polkadot_runtime::WASM_BINARY.unwrap().to_vec(),
				..Default::default()
			},
			balances: polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)).collect(),
			},
			session: polkadot_runtime::SessionConfig {
				keys: validators::initial_authorities()
					.iter()
					.map(|x| {
						(
							x.0.clone(),
							x.0.clone(),
							polkadot::session_keys(
								x.2.clone(),
								x.3.clone(),
								x.4.clone(),
								x.5.clone(),
								x.6.clone(),
								x.7.clone(),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			staking: polkadot_runtime::StakingConfig {
				validator_count: validators::initial_authorities().len() as u32,
				minimum_validator_count: 1,
				stakers: validators::initial_authorities()
					.iter()
					.map(|x| (x.0.clone(), x.1.clone(), STASH, polkadot_runtime::StakerStatus::Validator))
					.collect(),
				invulnerables: validators::initial_authorities().iter().map(|x| x.0.clone()).collect(),
				force_era: pallet_staking::Forcing::ForceNone,
				slash_reward_fraction: Perbill::from_percent(10),
				..Default::default()
			},
			babe: polkadot_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: Some(polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG),
				..Default::default()
			},
			configuration: polkadot_runtime::ConfigurationConfig { config: get_host_config() },
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// AssetHub
pub mod asset_hub {
	use super::*;
	use crate::AssetHub;
	use xcm::{prelude::Parachain, v3::Parent};

	pub const PARA_ID: u32 = 1000;
	pub const ED: Balance = asset_hub_polkadot_runtime::constants::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let mut funded_accounts = vec![
			(AssetHub::sovereign_account_id_of((Parent, Parachain(penpal::PARA_ID)).into()), INITIAL_DEPOSIT),
			(AssetHub::sovereign_account_id_of((Parent, Parachain(politest::PARA_ID)).into()), INITIAL_DEPOSIT),
		];
		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));

		let genesis_config = asset_hub_polkadot_runtime::RuntimeGenesisConfig {
			system: asset_hub_polkadot_runtime::SystemConfig {
				code: asset_hub_polkadot_runtime::WASM_BINARY
					.expect("WASM binary was not build, please build it!")
					.to_vec(),
				..Default::default()
			},
			balances: asset_hub_polkadot_runtime::BalancesConfig { balances: funded_accounts },
			parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables_asset_hub().iter().cloned().map(|(acc, _)| acc).collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: asset_hub_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables_asset_hub()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                      // account id
							acc,                                              // validator id
							asset_hub_polkadot_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			aura: Default::default(),
			aura_ext: Default::default(),
			parachain_system: Default::default(),
			polkadot_xcm: asset_hub_polkadot_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
		};

		genesis_config.build_storage().unwrap()
	}
}

// Polimec
pub mod politest {
	use super::*;
	use crate::{Politest, PolitestRuntime};
	use pallet_funding::AcceptedFundingAsset;
	use sp_runtime::traits::{AccountIdConversion, Get};
	use xcm::{prelude::Parachain, v3::Parent};

	pub const PARA_ID: u32 = 3344;
	pub const ED: Balance = politest_runtime::EXISTENTIAL_DEPOSIT;

	const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 1800;
	const GENESIS_COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
	const GENESIS_PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
	const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;

	pub fn genesis() -> Storage {
		let dot_asset_id = AcceptedFundingAsset::DOT.to_assethub_id();
		let usdt_asset_id = AcceptedFundingAsset::USDT.to_assethub_id();
		let mut funded_accounts = vec![
			(Politest::sovereign_account_id_of((Parent, Parachain(penpal::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Politest::sovereign_account_id_of((Parent, Parachain(asset_hub::PARA_ID)).into()), INITIAL_DEPOSIT),
			(<PolitestRuntime as pallet_funding::Config>::ContributionTreasury::get(), INITIAL_DEPOSIT),
			(<PolitestRuntime as pallet_funding::Config>::PalletId::get().into_account_truncating(), INITIAL_DEPOSIT),
		];
		let alice_account = Politest::account_id_of(accounts::ALICE);
		let bob_account: AccountId = Politest::account_id_of(accounts::BOB);
		let charlie_account: AccountId = Politest::account_id_of(accounts::CHARLIE);
		let dave_account: AccountId = Politest::account_id_of(accounts::DAVE);
		let eve_account: AccountId = Politest::account_id_of(accounts::EVE);

		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));
		funded_accounts.extend(collators::initial_authorities().iter().cloned().map(|(acc, _)| (acc, 20_005 * PLMC)));
		funded_accounts.push((get_account_id_from_seed::<sr25519::Public>("TREASURY_STASH"), 20_005 * PLMC));

		let genesis_config = politest_runtime::RuntimeGenesisConfig {
			system: politest_runtime::SystemConfig {
				code: politest_runtime::WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
				..Default::default()
			},
			balances: politest_runtime::BalancesConfig { balances: funded_accounts },
			parachain_info: politest_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			session: politest_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                            // account id
							acc,                                    // validator id
							politest_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			aura: Default::default(),
			aura_ext: Default::default(),
			parachain_system: Default::default(),
			polkadot_xcm: politest_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			sudo: politest_runtime::SudoConfig { key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")) },
			council: Default::default(),
			democracy: Default::default(),
			treasury: Default::default(),
			technical_committee: politest_runtime::TechnicalCommitteeConfig {
				members: vec![
					alice_account.clone(),
					bob_account.clone(),
					charlie_account.clone(),
					dave_account.clone(),
					eve_account.clone(),
				],
				..Default::default()
			},
			elections: politest_runtime::ElectionsConfig {
				members: vec![
					(alice_account.clone(), 0),
					(bob_account.clone(), 0),
					(charlie_account.clone(), 0),
					(dave_account.clone(), 0),
					(eve_account.clone(), 0),
				],
				..Default::default()
			},
			oracle_providers_membership: politest_runtime::OracleProvidersMembershipConfig {
				members: bounded_vec![alice_account.clone(), bob_account, charlie_account],
				..Default::default()
			},
			parachain_staking: politest_runtime::ParachainStakingConfig {
				candidates: collators::initial_authorities()
					.iter()
					.map(|(acc, _)| (acc.clone(), 20_000 * PLMC))
					.collect(),
				delegations: vec![],
				inflation_config: polimec_inflation_config(),
				collator_commission: GENESIS_COLLATOR_COMMISSION,
				parachain_bond_reserve_percent: GENESIS_PARACHAIN_BOND_RESERVE_PERCENT,
				blocks_per_round: GENESIS_BLOCKS_PER_ROUND,
				num_selected_candidates: GENESIS_NUM_SELECTED_CANDIDATES,
			},
			foreign_assets: politest_runtime::ForeignAssetsConfig {
				assets: vec![
					(dot_asset_id, alice_account.clone(), true, 0_0_010_000_000u128),
					(usdt_asset_id, alice_account.clone(), true, 0_0_010_000_000u128),
				],
				metadata: vec![
					(dot_asset_id, "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(), 12),
					(usdt_asset_id, "Local USDT".as_bytes().to_vec(), "USDT".as_bytes().to_vec(), 12),
				],
				accounts: vec![],
			},
			polimec_funding: Default::default(),
			vesting: Default::default(),
		};

		genesis_config.build_storage().unwrap()
	}
}

// Penpal
pub mod penpal {
	use super::*;
	use crate::{ParaId, Penpal};
	use xcm::{prelude::Parachain, v3::Parent};
	pub const PARA_ID: u32 = 6969;
	pub const ED: Balance = penpal_runtime::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let mut funded_accounts = vec![
			(Penpal::sovereign_account_id_of((Parent, Parachain(asset_hub::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Penpal::sovereign_account_id_of((Parent, Parachain(politest::PARA_ID)).into()), 2_000_000_0_000_000_000), // i.e the CTs sold on polimec
		];
		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));

		let genesis_config = penpal_runtime::RuntimeGenesisConfig {
			system: penpal_runtime::SystemConfig {
				code: penpal_runtime::WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
				..Default::default()
			},
			balances: penpal_runtime::BalancesConfig { balances: funded_accounts },
			parachain_info: penpal_runtime::ParachainInfoConfig {
				parachain_id: ParaId::from(PARA_ID),
				..Default::default()
			},
			collator_selection: penpal_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: penpal_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                          // account id
							acc,                                  // validator id
							penpal_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			aura: Default::default(),
			aura_ext: Default::default(),
			parachain_system: Default::default(),
			polkadot_xcm: penpal_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			sudo: penpal_runtime::SudoConfig { key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")) },
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Polimec Runtime
pub mod polimec {
	use super::*;
	use crate::Polimec;
	use pallet_funding::AcceptedFundingAsset;
	use polimec_base_runtime::PayMaster;
	use xcm::{prelude::Parachain, v3::Parent};

	pub const PARA_ID: u32 = 3344;
	pub const ED: Balance = polimec_runtime::EXISTENTIAL_DEPOSIT;

	const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 1800;
	const GENESIS_COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
	const GENESIS_PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
	const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;

	pub fn genesis() -> Storage {
		let dot_asset_id = AcceptedFundingAsset::DOT.to_assethub_id();
		let usdt_asset_id = AcceptedFundingAsset::USDT.to_assethub_id();
		let usdc_asset_id = AcceptedFundingAsset::USDC.to_assethub_id();
		let mut funded_accounts = vec![
			(Polimec::sovereign_account_id_of((Parent, Parachain(penpal::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Polimec::sovereign_account_id_of((Parent, Parachain(asset_hub::PARA_ID)).into()), INITIAL_DEPOSIT),
		];
		let alice_account = Polimec::account_id_of(accounts::ALICE);
		let bob_account: AccountId = Polimec::account_id_of(accounts::BOB);
		let charlie_account: AccountId = Polimec::account_id_of(accounts::CHARLIE);
		let dave_account: AccountId = Polimec::account_id_of(accounts::DAVE);
		let eve_account: AccountId = Polimec::account_id_of(accounts::EVE);

		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));
		funded_accounts.extend(collators::initial_authorities().iter().cloned().map(|(acc, _)| (acc, 20_005 * PLMC)));
		funded_accounts.push((get_account_id_from_seed::<sr25519::Public>("TREASURY_STASH"), 20_005 * PLMC));
		funded_accounts.push((PayMaster::get(), 20_005 * PLMC));

		let genesis_config = polimec_runtime::RuntimeGenesisConfig {
			system: polimec_runtime::SystemConfig {
				code: polimec_runtime::WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
				..Default::default()
			},
			balances: polimec_runtime::BalancesConfig { balances: funded_accounts },
			foreign_assets: polimec_runtime::ForeignAssetsConfig {
				assets: vec![
					(dot_asset_id, alice_account.clone(), true, 0_0_010_000_000u128),
					(usdt_asset_id, alice_account.clone(), true, 0_0_010_000_000u128),
					(usdc_asset_id, alice_account.clone(), true, 0_0_010_000_000u128),
				],
				metadata: vec![
					(dot_asset_id, "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(), 12),
					(usdt_asset_id, "Local USDT".as_bytes().to_vec(), "USDT".as_bytes().to_vec(), 6),
					(usdc_asset_id, "Local USDC".as_bytes().to_vec(), "USDC".as_bytes().to_vec(), 6),
				],
				accounts: vec![],
			},
			parachain_info: polimec_runtime::ParachainInfoConfig { parachain_id: PARA_ID.into(), ..Default::default() },
			session: polimec_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                           // account id
							acc,                                   // validator id
							polimec_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			aura: Default::default(),
			aura_ext: Default::default(),
			council: Default::default(),
			technical_committee: polimec_runtime::TechnicalCommitteeConfig {
				members: vec![
					alice_account.clone(),
					bob_account.clone(),
					charlie_account.clone(),
					dave_account.clone(),
					eve_account.clone(),
				],
				..Default::default()
			},
			elections: polimec_runtime::ElectionsConfig {
				members: vec![
					(alice_account.clone(), 0),
					(bob_account.clone(), 0),
					(charlie_account.clone(), 0),
					(dave_account.clone(), 0),
					(eve_account.clone(), 0),
				],
				..Default::default()
			},
			democracy: Default::default(),
			parachain_system: Default::default(),
			polkadot_xcm: polimec_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			parachain_staking: polimec_runtime::ParachainStakingConfig {
				candidates: collators::initial_authorities()
					.iter()
					.map(|(acc, _)| (acc.clone(), 20_000 * PLMC))
					.collect(),
				delegations: vec![],
				inflation_config: polimec_inflation_config(),
				collator_commission: GENESIS_COLLATOR_COMMISSION,
				parachain_bond_reserve_percent: GENESIS_PARACHAIN_BOND_RESERVE_PERCENT,
				blocks_per_round: GENESIS_BLOCKS_PER_ROUND,
				num_selected_candidates: GENESIS_NUM_SELECTED_CANDIDATES,
			},
			oracle_providers_membership: polimec_runtime::OracleProvidersMembershipConfig {
				members: bounded_vec![alice_account.clone(), bob_account, charlie_account],
				..Default::default()
			},
			vesting: Default::default(),
			transaction_payment: Default::default(),
			treasury: Default::default(),
		};

		genesis_config.build_storage().unwrap()
	}
}
