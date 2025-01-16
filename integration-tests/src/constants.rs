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

use frame_support::BoundedVec;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_parachain_staking::inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR};
pub use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
use polimec_common::assets::AcceptedFundingAsset;
use polimec_runtime::{pallet_parachain_staking::Range, PLMC};
use polkadot_primitives::{AssignmentId, ValidatorId};
pub use polkadot_runtime_parachains::configuration::HostConfiguration;
use sc_consensus_grandpa::AuthorityId as GrandpaId;
use sp_arithmetic::{FixedU128, Percent};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use sp_core::{sr25519, storage::Storage, Pair, Public};
use sp_runtime::{bounded_vec, BuildStorage, Perbill};
pub use xcm;
use xcm_emulator::{helpers::get_account_id_from_seed, Chain, Parachain};

pub const XCM_V2: u32 = 3;
pub const XCM_V3: u32 = 2;
pub const REF_TIME_THRESHOLD: u64 = 33;
pub const PROOF_SIZE_THRESHOLD: u64 = 33;
pub const INITIAL_DEPOSIT: u128 = 420_0_000_000_000;
const BLOCKS_PER_ROUND: u32 = 6 * 100;

fn polimec_inflation_config() -> polimec_runtime::pallet_parachain_staking::InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
		)
	}

	let annual =
		Range { min: Perbill::from_percent(2), ideal: Perbill::from_percent(3), max: Perbill::from_percent(3) };

	polimec_runtime::pallet_parachain_staking::InflationInfo {
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

pub struct Prices {
	pub dot: FixedU128,
	pub usdc: FixedU128,
	pub usdt: FixedU128,
	pub plmc: FixedU128,
	pub weth: FixedU128,
}

// PricesBuilder for optional fields before building Prices
#[derive(Clone, Copy, Default)]
pub struct PricesBuilder {
	dot: Option<FixedU128>,
	usdc: Option<FixedU128>,
	usdt: Option<FixedU128>,
	plmc: Option<FixedU128>,
	weth: Option<FixedU128>,
}

impl PricesBuilder {
	// Initialize a new builder with None for each field
	pub fn new() -> Self {
		Self { dot: None, usdc: None, usdt: None, plmc: None, weth: None }
	}

	pub fn default_prices() -> Prices {
		Prices {
			dot: FixedU128::from_rational(69, 1),
			usdc: FixedU128::from_rational(1, 1),
			usdt: FixedU128::from_rational(1, 1),
			plmc: FixedU128::from_rational(840, 100),
			weth: FixedU128::from_rational(3620, 1),
		}
	}

	// Setters that take FixedU128 and return &mut self for chaining
	pub fn dot(&mut self, price: FixedU128) -> &mut Self {
		self.dot = Some(price);
		self
	}

	pub fn usdc(&mut self, price: FixedU128) -> &mut Self {
		self.usdc = Some(price);
		self
	}

	pub fn usdt(&mut self, price: FixedU128) -> &mut Self {
		self.usdt = Some(price);
		self
	}

	pub fn plmc(&mut self, price: FixedU128) -> &mut Self {
		self.plmc = Some(price);
		self
	}

	pub fn weth(&mut self, price: FixedU128) -> &mut Self {
		self.weth = Some(price);
		self
	}

	// Build Prices using provided values or default values
	pub fn build(self) -> Prices {
		Prices {
			dot: self.dot.unwrap_or(FixedU128::from_rational(69, 1)), // Default DOT price
			usdc: self.usdc.unwrap_or(FixedU128::from_rational(1, 1)), // Default USDC price
			usdt: self.usdt.unwrap_or(FixedU128::from_rational(1, 1)), // Default USDT price
			plmc: self.plmc.unwrap_or(FixedU128::from_rational(840, 100)), // Default PLMC price
			weth: self.weth.unwrap_or(FixedU128::from_rational(3620, 1)), // Default WETH price
		}
	}
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

	#[allow(clippy::type_complexity)]
	pub fn initial_authorities() -> Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)> {
		let seed = "Alice";
		vec![(
			get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
			get_account_id_from_seed::<sr25519::Public>(seed),
			get_from_seed::<BabeId>(seed),
			get_from_seed::<GrandpaId>(seed),
			get_from_seed::<ImOnlineId>(seed),
			get_from_seed::<ValidatorId>(seed),
			get_from_seed::<AssignmentId>(seed),
			get_from_seed::<AuthorityDiscoveryId>(seed),
			get_from_seed::<BeefyId>(seed),
		)]
	}
}

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

// Helper functions to calculate chain accounts

// Polkadot
pub mod polkadot {
	use super::*;
	pub const ED: Balance = rococo_runtime_constants::currency::EXISTENTIAL_DEPOSIT;

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
		_im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
		beefy: BeefyId,
	) -> rococo_runtime::SessionKeys {
		rococo_runtime::SessionKeys { babe, grandpa, para_validator, para_assignment, authority_discovery, beefy }
	}

	pub fn genesis() -> Storage {
		let genesis_config = rococo_runtime::RuntimeGenesisConfig {
			balances: rococo_runtime::BalancesConfig {
				balances: accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)).collect(),
			},
			session: rococo_runtime::SessionConfig {
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
								x.8.clone(),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			babe: rococo_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: rococo_runtime::BABE_GENESIS_EPOCH_CONFIG,
				..Default::default()
			},
			configuration: rococo_runtime::ConfigurationConfig { config: get_host_config() },
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Penpal
pub mod penpal {
	use super::*;
	use crate::{ParaId, Penpal, PolkadotNet};
	use xcm::v4::Parent;
	pub const PARA_ID: u32 = 6969;
	pub const ED: Balance = penpal_runtime::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let mut funded_accounts = vec![(
			<Penpal<PolkadotNet>>::sovereign_account_id_of((Parent, xcm::prelude::Parachain(1000)).into()),
			INITIAL_DEPOSIT,
		)];
		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));

		let genesis_config = penpal_runtime::RuntimeGenesisConfig {
			system: Default::default(),
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

// Polimec
pub mod polimec {
	use super::*;
	use crate::{PolimecNet, PolimecOrigin, PolimecRuntime};
	use polimec_runtime::{BlockchainOperationTreasury, TreasuryAccount};
	use xcm::v4::{Location, Parent};
	use xcm_emulator::TestExt;

	pub const PARA_ID: u32 = 3344;
	pub const ED: Balance = polimec_runtime::EXISTENTIAL_DEPOSIT;

	const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 1800;
	const GENESIS_COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
	const GENESIS_PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
	const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;

	pub fn set_prices(prices: Prices) {
		PolimecNet::execute_with(|| {
			let dot = (AcceptedFundingAsset::DOT.id(), prices.dot);
			let usdc = (AcceptedFundingAsset::USDC.id(), prices.usdc);
			let usdt = (AcceptedFundingAsset::USDT.id(), prices.usdt);
			let plmc = (Location::here(), prices.plmc);
			let weth = (AcceptedFundingAsset::WETH.id(), prices.weth);

			let values: BoundedVec<(Location, FixedU128), <PolimecRuntime as orml_oracle::Config>::MaxFeedValues> =
				vec![dot, usdc, usdt, plmc, weth].try_into().expect("benchmarks can panic");
			let alice: [u8; 32] = [
				212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130, 44, 133, 88, 133, 76, 205,
				227, 154, 86, 132, 231, 165, 109, 162, 125,
			];
			let bob: [u8; 32] = [
				142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161, 126, 37, 252, 82, 135, 97, 54, 147, 201, 18, 144,
				156, 178, 38, 170, 71, 148, 242, 106, 72,
			];
			let charlie: [u8; 32] = [
				144, 181, 171, 32, 92, 105, 116, 201, 234, 132, 27, 230, 136, 134, 70, 51, 220, 156, 168, 163, 87, 132,
				62, 234, 207, 35, 20, 100, 153, 101, 254, 34,
			];

			frame_support::assert_ok!(orml_oracle::Pallet::<PolimecRuntime>::feed_values(
				PolimecOrigin::signed(alice.into()),
				values.clone()
			));

			frame_support::assert_ok!(orml_oracle::Pallet::<PolimecRuntime>::feed_values(
				PolimecOrigin::signed(bob.into()),
				values.clone()
			));

			frame_support::assert_ok!(orml_oracle::Pallet::<PolimecRuntime>::feed_values(
				PolimecOrigin::signed(charlie.into()),
				values.clone()
			));
		});
	}

	pub fn genesis() -> Storage {
		let dot_asset_id = AcceptedFundingAsset::DOT.id();
		let usdt_asset_id = AcceptedFundingAsset::USDT.id();
		let usdc_asset_id = AcceptedFundingAsset::USDC.id();
		let weth_asset_id = AcceptedFundingAsset::WETH.id();

		let mut funded_accounts = vec![
			(
				PolimecNet::sovereign_account_id_of((Parent, xcm::prelude::Parachain(penpal::PARA_ID)).into()),
				INITIAL_DEPOSIT,
			),
			(PolimecNet::sovereign_account_id_of((Parent, xcm::prelude::Parachain(1000)).into()), INITIAL_DEPOSIT),
		];
		let alice_account = PolimecNet::account_id_of(accounts::ALICE);
		let bob_account: AccountId = PolimecNet::account_id_of(accounts::BOB);
		let charlie_account: AccountId = PolimecNet::account_id_of(accounts::CHARLIE);
		let dave_account: AccountId = PolimecNet::account_id_of(accounts::DAVE);
		let eve_account: AccountId = PolimecNet::account_id_of(accounts::EVE);

		funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k| (k, INITIAL_DEPOSIT)));
		funded_accounts.extend(collators::initial_authorities().iter().cloned().map(|(acc, _)| (acc, 20_005 * PLMC)));
		funded_accounts.push((TreasuryAccount::get(), 20_000_000 * PLMC));
		funded_accounts.push((BlockchainOperationTreasury::get(), 20_005 * PLMC));
		// Treasury account needs PLMC for the One Token Model participations
		funded_accounts.push((polimec_runtime::FeeRecipient::get(), INITIAL_DEPOSIT));

		let genesis_config = polimec_runtime::RuntimeGenesisConfig {
			system: Default::default(),
			balances: polimec_runtime::BalancesConfig { balances: funded_accounts },
			contribution_tokens: Default::default(),
			foreign_assets: polimec_runtime::ForeignAssetsConfig {
				assets: vec![
					(dot_asset_id.clone(), alice_account.clone(), true, 100_000_000),
					(usdt_asset_id.clone(), alice_account.clone(), true, 70_000),
					(usdc_asset_id.clone(), alice_account.clone(), true, 70_000),
					(weth_asset_id.clone(), alice_account.clone(), true, 0_000_041_000_000_000_000),
				],
				metadata: vec![
					(dot_asset_id, "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(), 10),
					(usdt_asset_id, "Local USDT".as_bytes().to_vec(), "USDT".as_bytes().to_vec(), 6),
					(usdc_asset_id.clone(), "Local USDC".as_bytes().to_vec(), "USDC".as_bytes().to_vec(), 6),
					(weth_asset_id, "Local WETH".as_bytes().to_vec(), "WETH".as_bytes().to_vec(), 18),
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
