use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
use polkadot_primitives::{AssignmentId, ValidatorId};
pub use polkadot_runtime_parachains::configuration::HostConfiguration;
use polkadot_service::chain_spec::get_authority_keys_from_seed_no_beefy;
use sc_consensus_grandpa::AuthorityId as GrandpaId;
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

	pub fn invulnerables_statemint() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
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

// Statemint
pub mod statemint {
	use super::*;
	use crate::Statemint;
	use xcm::{prelude::Parachain, v3::Parent};

	pub const PARA_ID: u32 = 1000;
	pub const ED: Balance = asset_hub_polkadot_runtime::constants::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let mut funded_accounts = vec![
			(Statemint::sovereign_account_id_of((Parent, Parachain(penpal::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Statemint::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into()), INITIAL_DEPOSIT),
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
				invulnerables: collators::invulnerables_statemint().iter().cloned().map(|(acc, _)| acc).collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: asset_hub_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables_statemint()
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
pub mod polimec {
	use super::*;
	use crate::Polimec;
	use pallet_funding::AcceptedFundingAsset;
	use xcm::{prelude::Parachain, v3::Parent};

	pub const PARA_ID: u32 = 3344;
	pub const ED: Balance = polimec_parachain_runtime::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let dot_asset_id = AcceptedFundingAsset::DOT.to_statemint_id();
		let usdt_asset_id = AcceptedFundingAsset::USDT.to_statemint_id();
		let mut funded_accounts = vec![
			(Polimec::sovereign_account_id_of((Parent, Parachain(penpal::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Polimec::sovereign_account_id_of((Parent, Parachain(statemint::PARA_ID)).into()), INITIAL_DEPOSIT),
		];
		let alice_account = Polimec::account_id_of(accounts::ALICE);
		let bob_account: AccountId = Polimec::account_id_of(accounts::BOB);
		let charlie_account: AccountId = Polimec::account_id_of(accounts::CHARLIE);

		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));
		let genesis_config = polimec_parachain_runtime::RuntimeGenesisConfig {
			system: polimec_parachain_runtime::SystemConfig {
				code: polimec_parachain_runtime::WASM_BINARY
					.expect("WASM binary was not build, please build it!")
					.to_vec(),
				..Default::default()
			},
			polimec_funding: Default::default(),
			balances: polimec_parachain_runtime::BalancesConfig { balances: funded_accounts },
			parachain_info: polimec_parachain_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			session: polimec_parachain_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                     // account id
							acc,                                             // validator id
							polimec_parachain_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			aura: Default::default(),
			aura_ext: Default::default(),
			parachain_system: Default::default(),
			polkadot_xcm: polimec_parachain_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			sudo: polimec_parachain_runtime::SudoConfig {
				key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
			},
			council: Default::default(),
			democracy: Default::default(),
			oracle_providers_membership: polimec_parachain_runtime::OracleProvidersMembershipConfig {
				members: bounded_vec![alice_account.clone(), bob_account, charlie_account],
				..Default::default()
			},
			parachain_staking: Default::default(),
			statemint_assets: polimec_parachain_runtime::StatemintAssetsConfig {
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
			technical_committee: Default::default(),
			treasury: Default::default(),
			linear_vesting: Default::default(),
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
			(Penpal::sovereign_account_id_of((Parent, Parachain(statemint::PARA_ID)).into()), INITIAL_DEPOSIT),
			(Penpal::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into()), 2_000_000_0_000_000_000), // i.e the CTs sold on polimec
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
