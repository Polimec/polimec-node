#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
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

pub mod constants {



















    use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
    use sp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
    pub use parachains_common::{
        AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber,
    };
    use polimec_parachain_runtime::{
        pallet_parachain_staking::{
            inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
            Range,
        },
        PLMC,
    };
    use polkadot_primitives::{AssignmentId, ValidatorId};
    pub use polkadot_runtime_parachains::configuration::HostConfiguration;
    use sc_consensus_grandpa::AuthorityId as GrandpaId;
    use sp_arithmetic::Percent;
    use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
    use sp_consensus_babe::AuthorityId as BabeId;
    use sp_core::{sr25519, storage::Storage, Pair, Public};
    use sp_runtime::{bounded_vec, BuildStorage, Perbill};
    pub use xcm;
    use xcm_emulator::{Chain, Parachain, helpers::get_account_id_from_seed};
    pub const XCM_V2: u32 = 3;
    pub const XCM_V3: u32 = 2;
    pub const REF_TIME_THRESHOLD: u64 = 33;
    pub const PROOF_SIZE_THRESHOLD: u64 = 33;
    pub const INITIAL_DEPOSIT: u128 = 420_0_000_000_000;
    const BLOCKS_PER_ROUND: u32 = 6 * 100;
    fn polimec_inflation_config()
        ->
            polimec_parachain_runtime::pallet_parachain_staking::InflationInfo<Balance> {
        fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
            perbill_annual_to_perbill_round(annual,
                BLOCKS_PER_YEAR / BLOCKS_PER_ROUND)
        }
        let annual =
            Range {
                min: Perbill::from_percent(2),
                ideal: Perbill::from_percent(3),
                max: Perbill::from_percent(3),
            };
        polimec_parachain_runtime::pallet_parachain_staking::InflationInfo {
            expect: Range {
                min: 100_000 * PLMC,
                ideal: 200_000 * PLMC,
                max: 500_000 * PLMC,
            },
            annual,
            round: to_round_inflation(annual),
        }
    }
    /// Helper function to generate a crypto pair from seed
    fn get_from_seed<TPublic: Public>(seed: &str)
        -> <TPublic::Pair as Pair>::Public {
        TPublic::Pair::from_string(&{
                            let res = ::alloc::fmt::format(format_args!("//{0}", seed));
                            res
                        }, None).expect("static values are valid; qed").public()
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
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([get_account_id_from_seed::<sr25519::Public>(ALICE),
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
                            get_account_id_from_seed::<sr25519::Public>(FERDIE_STASH)]))
        }
    }
    pub mod collators {
        use super::*;
        pub fn invulnerables_asset_hub()
            -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(get_account_id_from_seed::<sr25519::Public>("Alice"),
                                get_from_seed::<AssetHubPolkadotAuraId>("Alice")),
                            (get_account_id_from_seed::<sr25519::Public>("Bob"),
                                get_from_seed::<AssetHubPolkadotAuraId>("Bob"))]))
        }
        pub fn invulnerables() -> Vec<(AccountId, AuraId)> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(get_account_id_from_seed::<sr25519::Public>("Alice"),
                                get_from_seed::<AuraId>("Alice")),
                            (get_account_id_from_seed::<sr25519::Public>("Bob"),
                                get_from_seed::<AuraId>("Bob"))]))
        }
        pub fn initial_authorities() -> Vec<(AccountId, AuraId)> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(get_account_id_from_seed::<sr25519::Public>("COLL_1"),
                                get_from_seed::<AuraId>("COLL_1")),
                            (get_account_id_from_seed::<sr25519::Public>("COLL_2"),
                                get_from_seed::<AuraId>("COLL_2"))]))
        }
    }
    pub mod validators {
        use super::*;
        pub fn initial_authorities()
            ->
                Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId,
                ValidatorId, AssignmentId, AuthorityDiscoveryId, BeefyId)> {
            let seed = "Alice";
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(get_account_id_from_seed::<sr25519::Public>(&{
                                            let res =
                                                ::alloc::fmt::format(format_args!("{0}//stash", seed));
                                            res
                                        }), get_account_id_from_seed::<sr25519::Public>(seed),
                                get_from_seed::<BabeId>(seed),
                                get_from_seed::<GrandpaId>(seed),
                                get_from_seed::<ImOnlineId>(seed),
                                get_from_seed::<ValidatorId>(seed),
                                get_from_seed::<AssignmentId>(seed),
                                get_from_seed::<AuthorityDiscoveryId>(seed),
                                get_from_seed::<BeefyId>(seed))]))
        }
    }
    /// The default XCM version to set in genesis config.
    const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
    pub mod polkadot {
        use super::*;
        pub const ED: Balance =
            polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
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
        fn session_keys(babe: BabeId, grandpa: GrandpaId,
            im_online: ImOnlineId, para_validator: ValidatorId,
            para_assignment: AssignmentId,
            authority_discovery: AuthorityDiscoveryId, beefy: BeefyId)
            -> polkadot_runtime::SessionKeys {
            polkadot_runtime::SessionKeys {
                babe,
                grandpa,
                im_online,
                para_validator,
                para_assignment,
                authority_discovery,
                beefy,
            }
        }
        pub fn genesis() -> Storage {
            let genesis_config =
                polkadot_runtime::RuntimeGenesisConfig {
                    system: Default::default(),
                    balances: polkadot_runtime::BalancesConfig {
                        balances: accounts::init_balances().iter().cloned().map(|k|
                                    (k, INITIAL_DEPOSIT)).collect(),
                    },
                    session: polkadot_runtime::SessionConfig {
                        keys: validators::initial_authorities().iter().map(|x|
                                    {
                                        (x.0.clone(), x.0.clone(),
                                            polkadot::session_keys(x.2.clone(), x.3.clone(),
                                                x.4.clone(), x.5.clone(), x.6.clone(), x.7.clone(),
                                                x.8.clone()))
                                    }).collect::<Vec<_>>(),
                    },
                    staking: polkadot_runtime::StakingConfig {
                        validator_count: validators::initial_authorities().len() as
                            u32,
                        minimum_validator_count: 1,
                        stakers: validators::initial_authorities().iter().map(|x|
                                    (x.0.clone(), x.1.clone(), STASH,
                                        polkadot_runtime::StakerStatus::Validator)).collect(),
                        invulnerables: validators::initial_authorities().iter().map(|x|
                                    x.0.clone()).collect(),
                        force_era: pallet_staking::Forcing::ForceNone,
                        slash_reward_fraction: Perbill::from_percent(10),
                        ..Default::default()
                    },
                    babe: polkadot_runtime::BabeConfig {
                        authorities: Default::default(),
                        epoch_config: Some(polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG),
                        ..Default::default()
                    },
                    configuration: polkadot_runtime::ConfigurationConfig {
                        config: get_host_config(),
                    },
                    ..Default::default()
                };
            genesis_config.build_storage().unwrap()
        }
    }
    pub mod asset_hub {
        use super::*;
        use crate::{AssetHub, PolkadotNet};
        use xcm::v3::Parent;
        pub const PARA_ID: u32 = 1000;
        pub const ED: Balance =
            system_parachains_constants::polkadot::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT;
        pub fn genesis() -> Storage {
            let mut funded_accounts =
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(<AssetHub<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(penpal::PARA_ID)).into()),
                                    INITIAL_DEPOSIT),
                                (<AssetHub<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(polimec::PARA_ID)).into()),
                                    INITIAL_DEPOSIT)]));
            funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k|
                        (k, INITIAL_DEPOSIT)));
            let genesis_config =
                asset_hub_polkadot_runtime::RuntimeGenesisConfig {
                    system: Default::default(),
                    balances: asset_hub_polkadot_runtime::BalancesConfig {
                        balances: funded_accounts,
                    },
                    parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
                        parachain_id: PARA_ID.into(),
                        ..Default::default()
                    },
                    collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
                        invulnerables: collators::invulnerables_asset_hub().iter().cloned().map(|(acc,
                                        _)| acc).collect(),
                        candidacy_bond: ED * 16,
                        ..Default::default()
                    },
                    session: asset_hub_polkadot_runtime::SessionConfig {
                        keys: collators::invulnerables_asset_hub().into_iter().map(|(acc,
                                        aura)|
                                    {
                                        (acc.clone(), acc,
                                            asset_hub_polkadot_runtime::SessionKeys { aura })
                                    }).collect(),
                    },
                    aura: Default::default(),
                    aura_ext: Default::default(),
                    parachain_system: Default::default(),
                    polkadot_xcm: asset_hub_polkadot_runtime::PolkadotXcmConfig {
                        safe_xcm_version: Some(SAFE_XCM_VERSION),
                        ..Default::default()
                    },
                    assets: Default::default(),
                    foreign_assets: Default::default(),
                    transaction_payment: Default::default(),
                };
            genesis_config.build_storage().unwrap()
        }
    }
    pub mod polimec {
        use super::*;
        use crate::{Polimec, PolkadotNet};
        use pallet_funding::AcceptedFundingAsset;
        use xcm::v3::Parent;
        pub const PARA_ID: u32 = 3344;
        pub const ED: Balance =
            polimec_parachain_runtime::EXISTENTIAL_DEPOSIT;
        const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 1800;
        const GENESIS_COLLATOR_COMMISSION: Perbill =
            Perbill::from_percent(10);
        const GENESIS_PARACHAIN_BOND_RESERVE_PERCENT: Percent =
            Percent::from_percent(0);
        const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;
        pub fn genesis() -> Storage {
            let dot_asset_id = AcceptedFundingAsset::DOT.to_assethub_id();
            let usdt_asset_id = AcceptedFundingAsset::USDT.to_assethub_id();
            let mut funded_accounts =
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(<Polimec<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(penpal::PARA_ID)).into()),
                                    INITIAL_DEPOSIT),
                                (<Polimec<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(asset_hub::PARA_ID)).into()),
                                    INITIAL_DEPOSIT)]));
            let alice_account =
                <Polimec<PolkadotNet>>::account_id_of(accounts::ALICE);
            let bob_account: AccountId =
                <Polimec<PolkadotNet>>::account_id_of(accounts::BOB);
            let charlie_account: AccountId =
                <Polimec<PolkadotNet>>::account_id_of(accounts::CHARLIE);
            let dave_account: AccountId =
                <Polimec<PolkadotNet>>::account_id_of(accounts::DAVE);
            let eve_account: AccountId =
                <Polimec<PolkadotNet>>::account_id_of(accounts::EVE);
            funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k|
                        (k, INITIAL_DEPOSIT)));
            funded_accounts.extend(collators::initial_authorities().iter().cloned().map(|(acc,
                            _)| (acc, 20_005 * PLMC)));
            funded_accounts.push((get_account_id_from_seed::<sr25519::Public>("TREASURY_STASH"),
                    20_005 * PLMC));
            let genesis_config =
                polimec_parachain_runtime::RuntimeGenesisConfig {
                    system: Default::default(),
                    balances: polimec_parachain_runtime::BalancesConfig {
                        balances: funded_accounts,
                    },
                    parachain_info: polimec_parachain_runtime::ParachainInfoConfig {
                        parachain_id: PARA_ID.into(),
                        ..Default::default()
                    },
                    session: polimec_parachain_runtime::SessionConfig {
                        keys: collators::invulnerables().into_iter().map(|(acc,
                                        aura)|
                                    {
                                        (acc.clone(), acc,
                                            polimec_parachain_runtime::SessionKeys { aura })
                                    }).collect(),
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
                    treasury: Default::default(),
                    technical_committee: polimec_parachain_runtime::TechnicalCommitteeConfig {
                        members: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([alice_account.clone(),
                                        bob_account.clone(), charlie_account.clone(),
                                        dave_account.clone(), eve_account.clone()])),
                        ..Default::default()
                    },
                    elections: polimec_parachain_runtime::ElectionsConfig {
                        members: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(alice_account.clone(),
                                            0), (bob_account.clone(), 0), (charlie_account.clone(), 0),
                                        (dave_account.clone(), 0), (eve_account.clone(), 0)])),
                        ..Default::default()
                    },
                    oracle_providers_membership: polimec_parachain_runtime::OracleProvidersMembershipConfig {
                        members: {
                            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([alice_account.clone(),
                                                    bob_account, charlie_account])).try_into().unwrap()
                        },
                        ..Default::default()
                    },
                    parachain_staking: polimec_parachain_runtime::ParachainStakingConfig {
                        candidates: collators::initial_authorities().iter().map(|(acc,
                                        _)| (acc.clone(), 20_000 * PLMC)).collect(),
                        delegations: ::alloc::vec::Vec::new(),
                        inflation_config: polimec_inflation_config(),
                        collator_commission: GENESIS_COLLATOR_COMMISSION,
                        parachain_bond_reserve_percent: GENESIS_PARACHAIN_BOND_RESERVE_PERCENT,
                        blocks_per_round: GENESIS_BLOCKS_PER_ROUND,
                        num_selected_candidates: GENESIS_NUM_SELECTED_CANDIDATES,
                    },
                    foreign_assets: polimec_parachain_runtime::ForeignAssetsConfig {
                        assets: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(dot_asset_id,
                                            alice_account.clone(), true, 0_0_010_000_000u128),
                                        (usdt_asset_id, alice_account.clone(), true,
                                            0_0_010_000_000u128)])),
                        metadata: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(dot_asset_id,
                                            "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(),
                                            12),
                                        (usdt_asset_id, "Local USDT".as_bytes().to_vec(),
                                            "USDT".as_bytes().to_vec(), 12)])),
                        accounts: ::alloc::vec::Vec::new(),
                    },
                    polimec_funding: Default::default(),
                    vesting: Default::default(),
                };
            genesis_config.build_storage().unwrap()
        }
    }
    pub mod penpal {
        use super::*;
        use crate::{ParaId, Penpal, PolkadotNet};
        use xcm::v3::Parent;
        pub const PARA_ID: u32 = 6969;
        pub const ED: Balance = penpal_runtime::EXISTENTIAL_DEPOSIT;
        pub fn genesis() -> Storage {
            let mut funded_accounts =
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(<Penpal<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(asset_hub::PARA_ID)).into()),
                                    INITIAL_DEPOSIT),
                                (<Penpal<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(polimec::PARA_ID)).into()),
                                    2_000_000_0_000_000_000)]));
            funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k|
                        (k, INITIAL_DEPOSIT)));
            let genesis_config =
                penpal_runtime::RuntimeGenesisConfig {
                    system: Default::default(),
                    balances: penpal_runtime::BalancesConfig {
                        balances: funded_accounts,
                    },
                    parachain_info: penpal_runtime::ParachainInfoConfig {
                        parachain_id: ParaId::from(PARA_ID),
                        ..Default::default()
                    },
                    collator_selection: penpal_runtime::CollatorSelectionConfig {
                        invulnerables: collators::invulnerables().iter().cloned().map(|(acc,
                                        _)| acc).collect(),
                        candidacy_bond: ED * 16,
                        ..Default::default()
                    },
                    session: penpal_runtime::SessionConfig {
                        keys: collators::invulnerables().into_iter().map(|(acc,
                                        aura)|
                                    {
                                        (acc.clone(), acc, penpal_runtime::SessionKeys { aura })
                                    }).collect(),
                    },
                    aura: Default::default(),
                    aura_ext: Default::default(),
                    parachain_system: Default::default(),
                    polkadot_xcm: penpal_runtime::PolkadotXcmConfig {
                        safe_xcm_version: Some(SAFE_XCM_VERSION),
                        ..Default::default()
                    },
                    sudo: penpal_runtime::SudoConfig {
                        key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
                    },
                    ..Default::default()
                };
            genesis_config.build_storage().unwrap()
        }
    }
    pub mod polimec_base {
        use super::*;
        use crate::{PolimecBase, PolkadotNet};
        use pallet_funding::AcceptedFundingAsset;
        use xcm::v3::Parent;
        pub const PARA_ID: u32 = 3344;
        pub const ED: Balance = polimec_base_runtime::EXISTENTIAL_DEPOSIT;
        const GENESIS_BLOCKS_PER_ROUND: BlockNumber = 1800;
        const GENESIS_COLLATOR_COMMISSION: Perbill =
            Perbill::from_percent(10);
        const GENESIS_PARACHAIN_BOND_RESERVE_PERCENT: Percent =
            Percent::from_percent(0);
        const GENESIS_NUM_SELECTED_CANDIDATES: u32 = 5;
        pub fn genesis() -> Storage {
            let dot_asset_id = AcceptedFundingAsset::DOT.to_assethub_id();
            let usdt_asset_id = AcceptedFundingAsset::USDT.to_assethub_id();
            let usdc_asset_id = AcceptedFundingAsset::USDC.to_assethub_id();
            let mut funded_accounts =
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(<PolimecBase<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(penpal::PARA_ID)).into()),
                                    INITIAL_DEPOSIT),
                                (<PolimecBase<PolkadotNet>>::sovereign_account_id_of((Parent,
                                                xcm::prelude::Parachain(asset_hub::PARA_ID)).into()),
                                    INITIAL_DEPOSIT)]));
            let alice_account =
                <PolimecBase<PolkadotNet>>::account_id_of(accounts::ALICE);
            let bob_account: AccountId =
                <PolimecBase<PolkadotNet>>::account_id_of(accounts::BOB);
            let charlie_account: AccountId =
                <PolimecBase<PolkadotNet>>::account_id_of(accounts::CHARLIE);
            let dave_account: AccountId =
                <PolimecBase<PolkadotNet>>::account_id_of(accounts::DAVE);
            let eve_account: AccountId =
                <PolimecBase<PolkadotNet>>::account_id_of(accounts::EVE);
            funded_accounts.extend(accounts::init_balances().iter().cloned().map(|k|
                        (k, INITIAL_DEPOSIT)));
            funded_accounts.extend(collators::initial_authorities().iter().cloned().map(|(acc,
                            _)| (acc, 20_005 * PLMC)));
            funded_accounts.push((get_account_id_from_seed::<sr25519::Public>("TREASURY_STASH"),
                    20_005 * PLMC));
            let genesis_config =
                polimec_base_runtime::RuntimeGenesisConfig {
                    system: Default::default(),
                    balances: polimec_base_runtime::BalancesConfig {
                        balances: funded_accounts,
                    },
                    foreign_assets: polimec_base_runtime::ForeignAssetsConfig {
                        assets: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(dot_asset_id,
                                            alice_account.clone(), true, 0_0_010_000_000u128),
                                        (usdt_asset_id, alice_account.clone(), true,
                                            0_0_010_000_000u128),
                                        (usdc_asset_id, alice_account.clone(), true,
                                            0_0_010_000_000u128)])),
                        metadata: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(dot_asset_id,
                                            "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(),
                                            12),
                                        (usdt_asset_id, "Local USDT".as_bytes().to_vec(),
                                            "USDT".as_bytes().to_vec(), 6),
                                        (usdc_asset_id, "Local USDC".as_bytes().to_vec(),
                                            "USDC".as_bytes().to_vec(), 6)])),
                        accounts: ::alloc::vec::Vec::new(),
                    },
                    parachain_info: polimec_base_runtime::ParachainInfoConfig {
                        parachain_id: PARA_ID.into(),
                        ..Default::default()
                    },
                    session: polimec_base_runtime::SessionConfig {
                        keys: collators::invulnerables().into_iter().map(|(acc,
                                        aura)|
                                    {
                                        (acc.clone(), acc,
                                            polimec_base_runtime::SessionKeys { aura })
                                    }).collect(),
                    },
                    aura: Default::default(),
                    aura_ext: Default::default(),
                    council: Default::default(),
                    technical_committee: polimec_base_runtime::TechnicalCommitteeConfig {
                        members: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([alice_account.clone(),
                                        bob_account.clone(), charlie_account.clone(),
                                        dave_account.clone(), eve_account.clone()])),
                        ..Default::default()
                    },
                    elections: polimec_base_runtime::ElectionsConfig {
                        members: <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(alice_account.clone(),
                                            0), (bob_account.clone(), 0), (charlie_account.clone(), 0),
                                        (dave_account.clone(), 0), (eve_account.clone(), 0)])),
                        ..Default::default()
                    },
                    democracy: Default::default(),
                    parachain_system: Default::default(),
                    polkadot_xcm: polimec_base_runtime::PolkadotXcmConfig {
                        safe_xcm_version: Some(SAFE_XCM_VERSION),
                        ..Default::default()
                    },
                    sudo: polimec_base_runtime::SudoConfig {
                        key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
                    },
                    parachain_staking: polimec_base_runtime::ParachainStakingConfig {
                        candidates: collators::initial_authorities().iter().map(|(acc,
                                        _)| (acc.clone(), 20_000 * PLMC)).collect(),
                        delegations: ::alloc::vec::Vec::new(),
                        inflation_config: polimec_inflation_config(),
                        collator_commission: GENESIS_COLLATOR_COMMISSION,
                        parachain_bond_reserve_percent: GENESIS_PARACHAIN_BOND_RESERVE_PERCENT,
                        blocks_per_round: GENESIS_BLOCKS_PER_ROUND,
                        num_selected_candidates: GENESIS_NUM_SELECTED_CANDIDATES,
                    },
                    oracle_providers_membership: polimec_base_runtime::OracleProvidersMembershipConfig {
                        members: {
                            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([alice_account.clone(),
                                                    bob_account, charlie_account])).try_into().unwrap()
                        },
                        ..Default::default()
                    },
                    vesting: Default::default(),
                    transaction_payment: Default::default(),
                    treasury: Default::default(),
                };
            genesis_config.build_storage().unwrap()
        }
    }
}
mod tests {
    mod basic_comms {
        #[allow(unused_imports)]
        use crate::*;
        const MAX_REF_TIME: u64 = 300_000_000;
        const MAX_PROOF_SIZE: u64 = 10_000;
    }
    mod build_spec {}
    mod credentials {
        use crate::*;
        use frame_support::assert_ok;
        use polimec_common::credentials::InvestorType;
        use polimec_common_test_utils::{get_fake_jwt, get_test_jwt};
        use polimec_parachain_runtime::PolimecFunding;
        use sp_runtime::{AccountId32, DispatchError};
        use tests::defaults::*;
    }
    mod ct_migration {
        use crate::*;
        use pallet_funding::{
            assert_close_enough, traits::VestingDurationCalculation,
            AcceptedFundingAsset, BidStatus, EvaluatorsOutcome,
            MigrationStatus, Multiplier, MultiplierOf, ProjectId,
            RewardOrSlash,
        };
        use polimec_common::migration_types::{
            Migration, MigrationInfo, MigrationOrigin, Migrations,
            ParticipationType,
        };
        use polimec_parachain_runtime::PolimecFunding;
        use sp_runtime::{traits::Convert, FixedPointNumber, Perquintill};
        use std::collections::HashMap;
        use tests::defaults::*;
        fn execute_cleaner(inst: &mut IntegrationInstantiator) {
            PoliNet::execute_with(||
                    {
                        inst.advance_time(<PolimecRuntime as
                                            pallet_funding::Config>::SuccessToSettlementTime::get() +
                                    1u32).unwrap();
                    });
        }
        fn mock_hrmp_establishment(project_id: u32) {
            PoliNet::execute_with(||
                    {
                        let is =
                            PolimecFunding::do_set_para_id_for_project(&ISSUER.into(),
                                project_id, ParaId::from(6969u32));
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                        let open_channel_message =
                            xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
                                sender: 6969,
                                max_message_size: 102_300,
                                max_capacity: 1000,
                            };
                        let is =
                            PolimecFunding::do_handle_channel_open_request(open_channel_message);
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                        let channel_accepted_message =
                            xcm::v3::opaque::Instruction::HrmpChannelAccepted {
                                recipient: 6969u32,
                            };
                        let is =
                            PolimecFunding::do_handle_channel_accepted(channel_accepted_message);
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                    });
            PenNet::execute_with(||
                    {
                        { ::std::io::_print(format_args!("penpal events:\n")); };
                        match PenNet::events() {
                            tmp => {
                                {
                                    ::std::io::_eprint(format_args!("[{0}:{1}] {2} = {3:#?}\n",
                                            "integration-tests/src/tests/ct_migration.rs", 49u32,
                                            "PenNet::events()", &tmp));
                                };
                                tmp
                            }
                        };
                    });
        }
        fn assert_migration_is_ready(project_id: u32) {
            PoliNet::execute_with(||
                    {
                        let project_details =
                            pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
                        if !project_details.migration_readiness_check.unwrap().is_ready()
                                {
                                ::core::panicking::panic("assertion failed: project_details.migration_readiness_check.unwrap().is_ready()")
                            }
                    });
        }
        fn send_migrations(project_id: ProjectId, accounts: Vec<AccountId>)
            -> HashMap<AccountId, Migrations> {
            let mut output = HashMap::new();
            for account in accounts {
                let migrations =
                    PoliNet::execute_with(||
                            {
                                let is =
                                    PolimecFunding::migrate_one_participant(PolimecOrigin::signed(account.clone()),
                                        project_id, account.clone());
                                match is {
                                    Ok(_) => (),
                                    _ =>
                                        if !false {
                                                {
                                                    ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                            is));
                                                }
                                            },
                                };
                                let user_evaluations =
                                    pallet_funding::Evaluations::<PolimecRuntime>::iter_prefix_values((project_id,
                                            account.clone()));
                                let user_bids =
                                    pallet_funding::Bids::<PolimecRuntime>::iter_prefix_values((project_id,
                                                account.clone())).filter(|bid|
                                            match bid.status {
                                                BidStatus::Accepted | BidStatus::PartiallyAccepted(..) =>
                                                    true,
                                                _ => false,
                                            });
                                let user_contributions =
                                    pallet_funding::Contributions::<PolimecRuntime>::iter_prefix_values((project_id,
                                            account.clone()));
                                let evaluation_migrations =
                                    user_evaluations.map(|evaluation|
                                            {
                                                let evaluator_bytes =
                                                    <PolimecRuntime as
                                                            pallet_funding::Config>::AccountId32Conversion::convert(evaluation.evaluator.clone());
                                                if !match evaluation.ct_migration_status {
                                                                MigrationStatus::Sent(_) => true,
                                                                _ => false,
                                                            } {
                                                        {
                                                            ::core::panicking::panic_fmt(format_args!("{0:?}\'s evaluation was not sent {1:?}",
                                                                    names()[&evaluator_bytes], evaluation));
                                                        }
                                                    };
                                                if let Some(RewardOrSlash::Reward(amount)) =
                                                            evaluation.rewarded_or_slashed {
                                                        Migration {
                                                            info: MigrationInfo {
                                                                contribution_token_amount: amount,
                                                                vesting_time: Multiplier::new(1u8).unwrap().calculate_vesting_duration::<PolimecRuntime>().into(),
                                                            },
                                                            origin: MigrationOrigin {
                                                                user: account.clone().into(),
                                                                id: evaluation.id,
                                                                participation_type: ParticipationType::Evaluation,
                                                            },
                                                        }
                                                    } else {
                                                       {
                                                           ::core::panicking::panic_fmt(format_args!("should be rewarded"));
                                                       }
                                                   }
                                            });
                                let bid_migrations =
                                    user_bids.map(|bid|
                                            {
                                                if !match bid.ct_migration_status {
                                                                MigrationStatus::Sent(_) => true,
                                                                _ => false,
                                                            } {
                                                        ::core::panicking::panic("assertion failed: matches!(bid.ct_migration_status, MigrationStatus :: Sent(_))")
                                                    };
                                                Migration {
                                                    info: MigrationInfo {
                                                        contribution_token_amount: bid.final_ct_amount,
                                                        vesting_time: bid.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
                                                    },
                                                    origin: MigrationOrigin {
                                                        user: account.clone().into(),
                                                        id: bid.id,
                                                        participation_type: ParticipationType::Bid,
                                                    },
                                                }
                                            });
                                let contribution_migrations =
                                    user_contributions.map(|contribution|
                                            {
                                                if !match contribution.ct_migration_status {
                                                                MigrationStatus::Sent(_) => true,
                                                                _ => false,
                                                            } {
                                                        ::core::panicking::panic("assertion failed: matches!(contribution.ct_migration_status, MigrationStatus :: Sent(_))")
                                                    };
                                                Migration {
                                                    info: MigrationInfo {
                                                        contribution_token_amount: contribution.ct_amount,
                                                        vesting_time: contribution.multiplier.calculate_vesting_duration::<PolimecRuntime>().into(),
                                                    },
                                                    origin: MigrationOrigin {
                                                        user: account.clone().into(),
                                                        id: contribution.id,
                                                        participation_type: ParticipationType::Contribution,
                                                    },
                                                }
                                            });
                                evaluation_migrations.chain(bid_migrations).chain(contribution_migrations).collect::<Migrations>()
                            });
                if migrations.clone().inner().is_empty() {
                        {
                            ::core::panicking::panic_fmt(format_args!("no migrations for account: {0:?}",
                                    account));
                        }
                    }
                output.insert(account.clone(), migrations);
            }
            output
        }
        fn migrations_are_executed(grouped_migrations: Vec<Migrations>) {
            let all_migrations =
                grouped_migrations.iter().flat_map(|migrations|
                            migrations.clone().inner()).collect::<Vec<_>>();
            PenNet::execute_with(||
                    {
                        let mut message: Vec<String> = Vec::new();
                        let mut events =
                            <PenNet as ::xcm_emulator::Chain>::events();
                        let mut event_received = false;
                        let mut meet_conditions = true;
                        let mut index_match = 0;
                        let mut event_message: Vec<String> = Vec::new();
                        for (index, event) in events.iter().enumerate() {
                            meet_conditions = true;
                            match event {
                                PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationExecuted {
                                    migration }) => {
                                    event_received = true;
                                    let mut conditions_message: Vec<String> = Vec::new();
                                    if !all_migrations.contains(&migration) &&
                                                event_message.is_empty() {
                                            conditions_message.push({
                                                    let res =
                                                        ::alloc::fmt::format(format_args!(" - The attribute {0:?} = {1:?} did not met the condition {2:?}\n",
                                                                "migration", migration,
                                                                "all_migrations.contains(&migration)"));
                                                    res
                                                });
                                        }
                                    meet_conditions &= all_migrations.contains(&migration);
                                    if event_received && meet_conditions {
                                            index_match = index;
                                            break;
                                        } else { event_message.extend(conditions_message); }
                                }
                                _ => {}
                            }
                        }
                        if event_received && !meet_conditions {
                                message.push({
                                        let res =
                                            ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was received but some of its attributes did not meet the conditions:\n{2}",
                                                    "PenNet",
                                                    "PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationExecuted {\nmigration })",
                                                    event_message.concat()));
                                        res
                                    });
                            } else if !event_received {
                               message.push({
                                       let res =
                                           ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was never received. All events:\n{2:#?}",
                                                   "PenNet",
                                                   "PenpalEvent::PolimecReceiver(polimec_receiver::Event::MigrationExecuted {\nmigration })",
                                                   <PenNet as ::xcm_emulator::Chain>::events()));
                                       res
                                   });
                           } else { events.remove(index_match); }
                        if !message.is_empty() {
                                <PenNet as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::PenNet",
                                                                    "integration_tests::tests::ct_migration",
                                                                    "integration-tests/src/tests/ct_migration.rs"), 149u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                { ::core::panicking::panic_display(&message.concat()); }
                            };
                    });
            for migration_group in grouped_migrations {
                let user = migration_group.clone().inner()[0].origin.user;
                if !migration_group.origins().iter().all(|origin|
                                    origin.user == user) {
                        ::core::panicking::panic("assertion failed: migration_group.origins().iter().all(|origin| origin.user == user)")
                    };
                let user_info = PenNet::account_data_of(user.into());
                let real_parts;
                if user_info.free <= migration_group.total_ct_amount() {
                        real_parts =
                            Perquintill::from_rational(user_info.free,
                                migration_group.total_ct_amount());
                    } else {
                       real_parts =
                           Perquintill::from_rational(migration_group.total_ct_amount(),
                               user_info.free);
                   }
                let one = Perquintill::from_percent(100u64);
                let real_approximation = one - real_parts;
                if !(real_approximation <=
                                Perquintill::from_parts(10_000_000_000u64)) {
                        {
                            ::core::panicking::panic_fmt(format_args!("Approximation is too big: {0:?} > {1:?} for {2:?} and {3:?}",
                                    real_approximation,
                                    Perquintill::from_parts(10_000_000_000u64), user_info.free,
                                    migration_group.total_ct_amount()));
                        }
                    };
                ;
                let vest_scheduled_cts =
                    migration_group.inner().iter().filter_map(|migration|
                                {
                                    if migration.info.vesting_time > 1 {
                                            Some(migration.info.contribution_token_amount)
                                        } else { None }
                                }).sum::<u128>();
                let real_parts;
                if user_info.frozen <= vest_scheduled_cts {
                        real_parts =
                            Perquintill::from_rational(user_info.frozen,
                                vest_scheduled_cts);
                    } else {
                       real_parts =
                           Perquintill::from_rational(vest_scheduled_cts,
                               user_info.frozen);
                   }
                let one = Perquintill::from_percent(100u64);
                let real_approximation = one - real_parts;
                if !(real_approximation <=
                                Perquintill::from_parts(10_000_000_000_000u64)) {
                        {
                            ::core::panicking::panic_fmt(format_args!("Approximation is too big: {0:?} > {1:?} for {2:?} and {3:?}",
                                    real_approximation,
                                    Perquintill::from_parts(10_000_000_000_000u64),
                                    user_info.frozen, vest_scheduled_cts));
                        }
                    };
                ;
            }
        }
        fn migrations_are_confirmed(project_id: u32,
            grouped_migrations: Vec<Migrations>) {
            let ordered_grouped_origins =
                grouped_migrations.clone().into_iter().map(|group|
                            {
                                let mut origins = group.origins();
                                origins.sort();
                                origins
                            }).collect::<Vec<_>>();
            PoliNet::execute_with(||
                    {
                        let mut message: Vec<String> = Vec::new();
                        let mut events =
                            <PoliNet as ::xcm_emulator::Chain>::events();
                        let mut event_received = false;
                        let mut meet_conditions = true;
                        let mut index_match = 0;
                        let mut event_message: Vec<String> = Vec::new();
                        for (index, event) in events.iter().enumerate() {
                            meet_conditions = true;
                            match event {
                                PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed {
                                    project_id, migration_origins }) => {
                                    event_received = true;
                                    let mut conditions_message: Vec<String> = Vec::new();
                                    if !(project_id == project_id) && event_message.is_empty() {
                                            conditions_message.push({
                                                    let res =
                                                        ::alloc::fmt::format(format_args!(" - The attribute {0:?} = {1:?} did not met the condition {2:?}\n",
                                                                "project_id", project_id, "project_id == project_id"));
                                                    res
                                                });
                                        }
                                    meet_conditions &= project_id == project_id;
                                    if !{
                                                        let mut migration_origins = migration_origins.to_vec();
                                                        migration_origins.sort();
                                                        ordered_grouped_origins.contains(&migration_origins)
                                                    } && event_message.is_empty() {
                                            conditions_message.push({
                                                    let res =
                                                        ::alloc::fmt::format(format_args!(" - The attribute {0:?} = {1:?} did not met the condition {2:?}\n",
                                                                "migration_origins", migration_origins,
                                                                "{\n    let mut migration_origins = migration_origins.to_vec();\n    migration_origins.sort();\n    ordered_grouped_origins.contains(&migration_origins)\n}"));
                                                    res
                                                });
                                        }
                                    meet_conditions &=
                                        {
                                            let mut migration_origins = migration_origins.to_vec();
                                            migration_origins.sort();
                                            ordered_grouped_origins.contains(&migration_origins)
                                        };
                                    if event_received && meet_conditions {
                                            index_match = index;
                                            break;
                                        } else { event_message.extend(conditions_message); }
                                }
                                _ => {}
                            }
                        }
                        if event_received && !meet_conditions {
                                message.push({
                                        let res =
                                            ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was received but some of its attributes did not meet the conditions:\n{2}",
                                                    "PoliNet",
                                                    "PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed {\nproject_id, migration_origins })",
                                                    event_message.concat()));
                                        res
                                    });
                            } else if !event_received {
                               message.push({
                                       let res =
                                           ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was never received. All events:\n{2:#?}",
                                                   "PoliNet",
                                                   "PolimecEvent::PolimecFunding(pallet_funding::Event::MigrationsConfirmed {\nproject_id, migration_origins })",
                                                   <PoliNet as ::xcm_emulator::Chain>::events()));
                                       res
                                   });
                           } else { events.remove(index_match); }
                        if !message.is_empty() {
                                <PoliNet as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::PoliNet",
                                                                    "integration_tests::tests::ct_migration",
                                                                    "integration-tests/src/tests/ct_migration.rs"), 197u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                { ::core::panicking::panic_display(&message.concat()); }
                            };
                        let all_migration_origins =
                            grouped_migrations.iter().flat_map(|migrations|
                                        migrations.clone().origins()).collect::<Vec<_>>();
                        for migration_origin in all_migration_origins {
                            match migration_origin.participation_type {
                                ParticipationType::Evaluation => {
                                    let evaluation =
                                        pallet_funding::Evaluations::<PolimecRuntime>::get((project_id,
                                                    AccountId::from(migration_origin.user),
                                                    migration_origin.id)).unwrap();
                                    match (&evaluation.ct_migration_status,
                                            &MigrationStatus::Confirmed) {
                                        (left_val, right_val) => {
                                            if !(*left_val == *right_val) {
                                                    let kind = ::core::panicking::AssertKind::Eq;
                                                    ::core::panicking::assert_failed(kind, &*left_val,
                                                        &*right_val, ::core::option::Option::None);
                                                }
                                        }
                                    };
                                }
                                ParticipationType::Bid => {
                                    let bid =
                                        pallet_funding::Bids::<PolimecRuntime>::get((project_id,
                                                    AccountId::from(migration_origin.user),
                                                    migration_origin.id)).unwrap();
                                    match (&bid.ct_migration_status,
                                            &MigrationStatus::Confirmed) {
                                        (left_val, right_val) => {
                                            if !(*left_val == *right_val) {
                                                    let kind = ::core::panicking::AssertKind::Eq;
                                                    ::core::panicking::assert_failed(kind, &*left_val,
                                                        &*right_val, ::core::option::Option::None);
                                                }
                                        }
                                    };
                                }
                                ParticipationType::Contribution => {
                                    let contribution =
                                        pallet_funding::Contributions::<PolimecRuntime>::get((project_id,
                                                    AccountId::from(migration_origin.user),
                                                    migration_origin.id)).unwrap();
                                    match (&contribution.ct_migration_status,
                                            &MigrationStatus::Confirmed) {
                                        (left_val, right_val) => {
                                            if !(*left_val == *right_val) {
                                                    let kind = ::core::panicking::AssertKind::Eq;
                                                    ::core::panicking::assert_failed(kind, &*left_val,
                                                        &*right_val, ::core::option::Option::None);
                                                }
                                        }
                                    };
                                }
                            }
                        }
                    });
        }
        fn vest_migrations(grouped_migrations: Vec<Migrations>) {
            let biggest_time =
                grouped_migrations.iter().map(|migrations|
                                migrations.biggest_vesting_time()).max().unwrap();
            PenNet::execute_with(||
                    {
                        PenpalSystem::set_block_number(biggest_time as u32 + 1u32);
                    });
            for migration_group in grouped_migrations {
                let user = migration_group.clone().inner()[0].origin.user;
                if !migration_group.origins().iter().all(|origin|
                                    origin.user == user) {
                        ::core::panicking::panic("assertion failed: migration_group.origins().iter().all(|origin| origin.user == user)")
                    };
                let has_frozen_balance =
                    migration_group.inner().iter().any(|migration|
                            migration.info.vesting_time > 1);
                if has_frozen_balance {
                        PenNet::execute_with(||
                                {
                                    let is =
                                        pallet_vesting::Pallet::<PenpalRuntime>::vest(PenpalOrigin::signed(user.into()));
                                    match is {
                                        Ok(_) => (),
                                        _ =>
                                            if !false {
                                                    {
                                                        ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                                is));
                                                    }
                                                },
                                    };
                                });
                    }
            }
        }
        fn migrations_are_vested(grouped_migrations: Vec<Migrations>) {
            for migration_group in grouped_migrations {
                let user = migration_group.clone().inner()[0].origin.user;
                if !migration_group.origins().iter().all(|origin|
                                    origin.user == user) {
                        ::core::panicking::panic("assertion failed: migration_group.origins().iter().all(|origin| origin.user == user)")
                    };
                let user_info = PenNet::account_data_of(user.into());
                match (&user_info.frozen, &0) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(kind, &*left_val,
                                    &*right_val, ::core::option::Option::None);
                            }
                    }
                };
                match (&user_info.free, &migration_group.total_ct_amount()) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                                let kind = ::core::panicking::AssertKind::Eq;
                                ::core::panicking::assert_failed(kind, &*left_val,
                                    &*right_val, ::core::option::Option::None);
                            }
                    }
                };
            }
        }
    }
    mod defaults {
        use crate::PolimecRuntime;
        use frame_support::BoundedVec;
        pub use pallet_funding::instantiator::{
            BidParams, ContributionParams, UserToPLMCBalance, UserToUSDBalance,
        };
        use pallet_funding::{
            AcceptedFundingAsset, CurrencyMetadata, ParticipantsSize,
            ProjectMetadata, ProjectMetadataOf, TicketSize,
        };
        use sp_core::H256;
        use macros::generate_accounts;
        use polimec_parachain_runtime::AccountId;
        use sp_runtime::{traits::ConstU32, Perquintill};
        pub const METADATA: &str =
            r#"METADATA
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
        pub type IntegrationInstantiator =
            pallet_funding::instantiator::Instantiator<PolimecRuntime,
            <PolimecRuntime as
            pallet_funding::Config>::AllPalletsWithoutSystem,
            <PolimecRuntime as pallet_funding::Config>::RuntimeEvent>;
        pub fn hashed(data: impl AsRef<[u8]>) -> sp_core::H256 {
            <sp_runtime::traits::BlakeTwo256 as
                    sp_runtime::traits::Hash>::hash(data.as_ref())
        }
        pub const ISSUER: [u8; 32] =
            [73u8, 83u8, 83u8, 85u8, 69u8, 82u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EVAL_1: [u8; 32] =
            [69u8, 86u8, 65u8, 76u8, 95u8, 49u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EVAL_2: [u8; 32] =
            [69u8, 86u8, 65u8, 76u8, 95u8, 50u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EVAL_3: [u8; 32] =
            [69u8, 86u8, 65u8, 76u8, 95u8, 51u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EVAL_4: [u8; 32] =
            [69u8, 86u8, 65u8, 76u8, 95u8, 52u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_1: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 49u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_2: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 50u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_3: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 51u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_4: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 52u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_5: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 53u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIDDER_6: [u8; 32] =
            [66u8, 73u8, 68u8, 68u8, 69u8, 82u8, 95u8, 54u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_1: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 49u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_2: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 50u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_3: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 51u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_4: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 52u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_5: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 53u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUYER_6: [u8; 32] =
            [66u8, 85u8, 89u8, 69u8, 82u8, 95u8, 54u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub fn names() -> std::collections::HashMap<[u8; 32], &'static str> {
            let mut names = std::collections::HashMap::new();
            names.insert(ISSUER, "ISSUER");
            names.insert(EVAL_1, "EVAL_1");
            names.insert(EVAL_2, "EVAL_2");
            names.insert(EVAL_3, "EVAL_3");
            names.insert(EVAL_4, "EVAL_4");
            names.insert(BIDDER_1, "BIDDER_1");
            names.insert(BIDDER_2, "BIDDER_2");
            names.insert(BIDDER_3, "BIDDER_3");
            names.insert(BIDDER_4, "BIDDER_4");
            names.insert(BIDDER_5, "BIDDER_5");
            names.insert(BIDDER_6, "BIDDER_6");
            names.insert(BUYER_1, "BUYER_1");
            names.insert(BUYER_2, "BUYER_2");
            names.insert(BUYER_3, "BUYER_3");
            names.insert(BUYER_4, "BUYER_4");
            names.insert(BUYER_5, "BUYER_5");
            names.insert(BUYER_6, "BUYER_6");
            names
        }
        pub fn bounded_name() -> BoundedVec<u8, ConstU32<64>> {
            BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap()
        }
        pub fn bounded_symbol() -> BoundedVec<u8, ConstU32<64>> {
            BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap()
        }
        pub fn metadata_hash(nonce: u32) -> H256 {
            hashed({
                    let res =
                        ::alloc::fmt::format(format_args!("{0}-{1}", METADATA,
                                nonce));
                    res
                })
        }
        pub fn default_weights() -> Vec<u8> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([20u8, 15u8,
                            10u8, 25u8, 30u8]))
        }
        pub fn default_bidder_multipliers() -> Vec<u8> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([1u8, 6u8,
                            20u8, 12u8, 3u8]))
        }
        pub fn default_contributor_multipliers() -> Vec<u8> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([1u8, 2u8,
                            1u8, 4u8, 1u8]))
        }
        pub fn default_project(issuer: AccountId, nonce: u32)
            -> ProjectMetadataOf<PolimecRuntime> {
            ProjectMetadata {
                token_information: CurrencyMetadata {
                    name: bounded_name(),
                    symbol: bounded_symbol(),
                    decimals: ASSET_DECIMALS,
                },
                mainnet_token_max_supply: 8_000_000 * ASSET_UNIT,
                total_allocation_size: (50_000 * ASSET_UNIT,
                    50_000 * ASSET_UNIT),
                minimum_price: sp_runtime::FixedU128::from_float(1.0),
                ticket_size: TicketSize { minimum: Some(1), maximum: None },
                participants_size: ParticipantsSize {
                    minimum: Some(2),
                    maximum: None,
                },
                funding_thresholds: Default::default(),
                conversion_rate: 0,
                participation_currencies: AcceptedFundingAsset::USDT,
                funding_destination_account: issuer,
                offchain_information_hash: Some(metadata_hash(nonce)),
            }
        }
        pub fn default_evaluations()
            -> Vec<UserToUSDBalance<PolimecRuntime>> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([UserToUSDBalance::new(EVAL_1.into(),
                                50_000 * PLMC),
                            UserToUSDBalance::new(EVAL_2.into(), 25_000 * PLMC),
                            UserToUSDBalance::new(EVAL_3.into(), 32_000 * PLMC)]))
        }
        pub fn default_bidders() -> Vec<AccountId> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([BIDDER_1.into(),
                            BIDDER_2.into(), BIDDER_3.into(), BIDDER_4.into(),
                            BIDDER_5.into()]))
        }
        pub fn default_bids() -> Vec<BidParams<PolimecRuntime>> {
            let forty_percent_funding_usd =
                Perquintill::from_percent(40) * 100_000 * ASSET_UNIT;
            IntegrationInstantiator::generate_bids_from_total_usd(forty_percent_funding_usd,
                sp_runtime::FixedU128::from_float(1.0), default_weights(),
                default_bidders(), default_bidder_multipliers())
        }
        pub fn default_community_contributions()
            -> Vec<ContributionParams<PolimecRuntime>> {
            let fifty_percent_funding_usd =
                Perquintill::from_percent(50) * 100_000 * ASSET_UNIT;
            IntegrationInstantiator::generate_contributions_from_total_usd(fifty_percent_funding_usd,
                sp_runtime::FixedU128::from_float(1.0), default_weights(),
                default_community_contributors(),
                default_contributor_multipliers())
        }
        pub fn default_remainder_contributions()
            -> Vec<ContributionParams<PolimecRuntime>> {
            let fifty_percent_funding_usd =
                Perquintill::from_percent(5) * 100_000 * ASSET_UNIT;
            IntegrationInstantiator::generate_contributions_from_total_usd(fifty_percent_funding_usd,
                sp_runtime::FixedU128::from_float(1.0),
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([20u8,
                                15u8, 10u8, 25u8, 23u8, 7u8])),
                default_remainder_contributors(),
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([1u8,
                                2u8, 12u8, 1u8, 3u8, 10u8])))
        }
        pub fn default_community_contributors() -> Vec<AccountId> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([BUYER_1.into(),
                            BUYER_2.into(), BUYER_3.into(), BUYER_4.into(),
                            BUYER_5.into()]))
        }
        pub fn default_remainder_contributors() -> Vec<AccountId> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([EVAL_4.into(),
                            BUYER_6.into(), BIDDER_6.into(), EVAL_1.into(),
                            BUYER_1.into(), BIDDER_1.into()]))
        }
    }
    mod e2e {
        use crate::{tests::defaults::*, *};
        use frame_support::BoundedVec;
        use itertools::Itertools;
        use macros::generate_accounts;
        use pallet_funding::*;
        use polimec_parachain_runtime::{PolimecFunding, US_DOLLAR};
        use sp_arithmetic::{FixedPointNumber, Perquintill};
        use sp_runtime::{traits::CheckedSub, FixedU128};
        type UserToCTBalance =
            Vec<(AccountId, BalanceOf<PolimecRuntime>, ProjectId)>;
        pub const LINA: [u8; 32] =
            [76u8, 73u8, 78u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MIA: [u8; 32] =
            [77u8, 73u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALEXEY: [u8; 32] =
            [65u8, 76u8, 69u8, 88u8, 69u8, 89u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const PAUL: [u8; 32] =
            [80u8, 65u8, 85u8, 76u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MARIA: [u8; 32] =
            [77u8, 65u8, 82u8, 73u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const GEORGE: [u8; 32] =
            [71u8, 69u8, 79u8, 82u8, 71u8, 69u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const CLARA: [u8; 32] =
            [67u8, 76u8, 65u8, 82u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const RAMONA: [u8; 32] =
            [82u8, 65u8, 77u8, 79u8, 78u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const PASCAL: [u8; 32] =
            [80u8, 65u8, 83u8, 67u8, 65u8, 76u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EMMA: [u8; 32] =
            [69u8, 77u8, 77u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BIBI: [u8; 32] =
            [66u8, 73u8, 66u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const AHMED: [u8; 32] =
            [65u8, 72u8, 77u8, 69u8, 68u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HERBERT: [u8; 32] =
            [72u8, 69u8, 82u8, 66u8, 69u8, 82u8, 84u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LENI: [u8; 32] =
            [76u8, 69u8, 78u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const XI: [u8; 32] =
            [88u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const TOM: [u8; 32] =
            [84u8, 79u8, 77u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ADAMS: [u8; 32] =
            [65u8, 68u8, 65u8, 77u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const POLK: [u8; 32] =
            [80u8, 79u8, 76u8, 75u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MARKUS: [u8; 32] =
            [77u8, 65u8, 82u8, 75u8, 85u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ELLA: [u8; 32] =
            [69u8, 76u8, 76u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const SKR: [u8; 32] =
            [83u8, 75u8, 82u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ARTHUR: [u8; 32] =
            [65u8, 82u8, 84u8, 72u8, 85u8, 82u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MILA: [u8; 32] =
            [77u8, 73u8, 76u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LINCOLN: [u8; 32] =
            [76u8, 73u8, 78u8, 67u8, 79u8, 76u8, 78u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MONROE: [u8; 32] =
            [77u8, 79u8, 78u8, 82u8, 79u8, 69u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ARBRESHA: [u8; 32] =
            [65u8, 82u8, 66u8, 82u8, 69u8, 83u8, 72u8, 65u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ELDIN: [u8; 32] =
            [69u8, 76u8, 68u8, 73u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HARDING: [u8; 32] =
            [72u8, 65u8, 82u8, 68u8, 73u8, 78u8, 71u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const SOFIA: [u8; 32] =
            [83u8, 79u8, 70u8, 73u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DOMINIK: [u8; 32] =
            [68u8, 79u8, 77u8, 73u8, 78u8, 73u8, 75u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const NOLAND: [u8; 32] =
            [78u8, 79u8, 76u8, 65u8, 78u8, 68u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HANNAH: [u8; 32] =
            [72u8, 65u8, 78u8, 78u8, 65u8, 72u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HOOVER: [u8; 32] =
            [72u8, 79u8, 79u8, 86u8, 69u8, 82u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const GIGI: [u8; 32] =
            [71u8, 73u8, 71u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const JEFFERSON: [u8; 32] =
            [74u8, 69u8, 70u8, 70u8, 69u8, 82u8, 83u8, 79u8, 78u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LINDI: [u8; 32] =
            [76u8, 73u8, 78u8, 68u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const KEVIN: [u8; 32] =
            [75u8, 69u8, 86u8, 73u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ANIS: [u8; 32] =
            [65u8, 78u8, 73u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const RETO: [u8; 32] =
            [82u8, 69u8, 84u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HAALAND: [u8; 32] =
            [72u8, 65u8, 65u8, 76u8, 65u8, 78u8, 68u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const XENIA: [u8; 32] =
            [88u8, 69u8, 78u8, 73u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const EVA: [u8; 32] =
            [69u8, 86u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const SKARA: [u8; 32] =
            [83u8, 75u8, 65u8, 82u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ROOSEVELT: [u8; 32] =
            [82u8, 79u8, 79u8, 83u8, 69u8, 86u8, 69u8, 76u8, 84u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DRACULA: [u8; 32] =
            [68u8, 82u8, 65u8, 67u8, 85u8, 76u8, 65u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DURIM: [u8; 32] =
            [68u8, 85u8, 82u8, 73u8, 77u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HARRISON: [u8; 32] =
            [72u8, 65u8, 82u8, 82u8, 73u8, 83u8, 79u8, 78u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DRIN: [u8; 32] =
            [68u8, 82u8, 73u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const PARI: [u8; 32] =
            [80u8, 65u8, 82u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const TUTI: [u8; 32] =
            [84u8, 85u8, 84u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BENITO: [u8; 32] =
            [66u8, 69u8, 78u8, 73u8, 84u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const VANESSA: [u8; 32] =
            [86u8, 65u8, 78u8, 69u8, 83u8, 83u8, 65u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ENES: [u8; 32] =
            [69u8, 78u8, 69u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const RUDOLF: [u8; 32] =
            [82u8, 85u8, 68u8, 79u8, 76u8, 70u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const CERTO: [u8; 32] =
            [67u8, 69u8, 82u8, 84u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const TIESTO: [u8; 32] =
            [84u8, 73u8, 69u8, 83u8, 84u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DAVID: [u8; 32] =
            [68u8, 65u8, 86u8, 73u8, 68u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ATAKAN: [u8; 32] =
            [65u8, 84u8, 65u8, 75u8, 65u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const YANN: [u8; 32] =
            [89u8, 65u8, 78u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ENIS: [u8; 32] =
            [69u8, 78u8, 73u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALFREDO: [u8; 32] =
            [65u8, 76u8, 70u8, 82u8, 69u8, 68u8, 79u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const QENDRIM: [u8; 32] =
            [81u8, 69u8, 78u8, 68u8, 82u8, 73u8, 77u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LEONARDO: [u8; 32] =
            [76u8, 69u8, 79u8, 78u8, 65u8, 82u8, 68u8, 79u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const KEN: [u8; 32] =
            [75u8, 69u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LUCA: [u8; 32] =
            [76u8, 85u8, 67u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const FLAVIO: [u8; 32] =
            [70u8, 76u8, 65u8, 86u8, 73u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const FREDI: [u8; 32] =
            [70u8, 82u8, 69u8, 68u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALI: [u8; 32] =
            [65u8, 76u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DILARA: [u8; 32] =
            [68u8, 73u8, 76u8, 65u8, 82u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DAMIAN: [u8; 32] =
            [68u8, 65u8, 77u8, 73u8, 65u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const KAYA: [u8; 32] =
            [75u8, 65u8, 89u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const IAZI: [u8; 32] =
            [73u8, 65u8, 90u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const CHRIGI: [u8; 32] =
            [67u8, 72u8, 82u8, 73u8, 71u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const VALENTINA: [u8; 32] =
            [86u8, 65u8, 76u8, 69u8, 78u8, 84u8, 73u8, 78u8, 65u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALMA: [u8; 32] =
            [65u8, 76u8, 77u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALENA: [u8; 32] =
            [65u8, 76u8, 69u8, 78u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const PATRICK: [u8; 32] =
            [80u8, 65u8, 84u8, 82u8, 73u8, 67u8, 75u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ONTARIO: [u8; 32] =
            [79u8, 78u8, 84u8, 65u8, 82u8, 73u8, 79u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const RAKIA: [u8; 32] =
            [82u8, 65u8, 75u8, 73u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HUBERT: [u8; 32] =
            [72u8, 85u8, 66u8, 69u8, 82u8, 84u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const UTUS: [u8; 32] =
            [85u8, 84u8, 85u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const TOME: [u8; 32] =
            [84u8, 79u8, 77u8, 69u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ZUBER: [u8; 32] =
            [90u8, 85u8, 66u8, 69u8, 82u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ADAM: [u8; 32] =
            [65u8, 68u8, 65u8, 77u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const STANI: [u8; 32] =
            [83u8, 84u8, 65u8, 78u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BETI: [u8; 32] =
            [66u8, 69u8, 84u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const HALIT: [u8; 32] =
            [72u8, 65u8, 76u8, 73u8, 84u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const DRAGAN: [u8; 32] =
            [68u8, 82u8, 65u8, 71u8, 65u8, 78u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LEA: [u8; 32] =
            [76u8, 69u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LUIS: [u8; 32] =
            [76u8, 85u8, 73u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const TATI: [u8; 32] =
            [84u8, 65u8, 84u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const WEST: [u8; 32] =
            [87u8, 69u8, 83u8, 84u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MIRIJAM: [u8; 32] =
            [77u8, 73u8, 82u8, 73u8, 74u8, 65u8, 77u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const LIONEL: [u8; 32] =
            [76u8, 73u8, 79u8, 78u8, 69u8, 76u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const GIOVANNI: [u8; 32] =
            [71u8, 73u8, 79u8, 86u8, 65u8, 78u8, 78u8, 73u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const JOEL: [u8; 32] =
            [74u8, 79u8, 69u8, 76u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const POLKA: [u8; 32] =
            [80u8, 79u8, 76u8, 75u8, 65u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const MALIK: [u8; 32] =
            [77u8, 65u8, 76u8, 73u8, 75u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const ALEXANDER: [u8; 32] =
            [65u8, 76u8, 69u8, 88u8, 65u8, 78u8, 68u8, 69u8, 82u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const SOLOMUN: [u8; 32] =
            [83u8, 79u8, 76u8, 79u8, 77u8, 85u8, 78u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const JOHNNY: [u8; 32] =
            [74u8, 79u8, 72u8, 78u8, 78u8, 89u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const GRINGO: [u8; 32] =
            [71u8, 82u8, 73u8, 78u8, 71u8, 79u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const JONAS: [u8; 32] =
            [74u8, 79u8, 78u8, 65u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const BUNDI: [u8; 32] =
            [66u8, 85u8, 78u8, 68u8, 73u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const FELIX: [u8; 32] =
            [70u8, 69u8, 76u8, 73u8, 88u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub fn names() -> std::collections::HashMap<[u8; 32], &'static str> {
            let mut names = std::collections::HashMap::new();
            names.insert(LINA, "LINA");
            names.insert(MIA, "MIA");
            names.insert(ALEXEY, "ALEXEY");
            names.insert(PAUL, "PAUL");
            names.insert(MARIA, "MARIA");
            names.insert(GEORGE, "GEORGE");
            names.insert(CLARA, "CLARA");
            names.insert(RAMONA, "RAMONA");
            names.insert(PASCAL, "PASCAL");
            names.insert(EMMA, "EMMA");
            names.insert(BIBI, "BIBI");
            names.insert(AHMED, "AHMED");
            names.insert(HERBERT, "HERBERT");
            names.insert(LENI, "LENI");
            names.insert(XI, "XI");
            names.insert(TOM, "TOM");
            names.insert(ADAMS, "ADAMS");
            names.insert(POLK, "POLK");
            names.insert(MARKUS, "MARKUS");
            names.insert(ELLA, "ELLA");
            names.insert(SKR, "SKR");
            names.insert(ARTHUR, "ARTHUR");
            names.insert(MILA, "MILA");
            names.insert(LINCOLN, "LINCOLN");
            names.insert(MONROE, "MONROE");
            names.insert(ARBRESHA, "ARBRESHA");
            names.insert(ELDIN, "ELDIN");
            names.insert(HARDING, "HARDING");
            names.insert(SOFIA, "SOFIA");
            names.insert(DOMINIK, "DOMINIK");
            names.insert(NOLAND, "NOLAND");
            names.insert(HANNAH, "HANNAH");
            names.insert(HOOVER, "HOOVER");
            names.insert(GIGI, "GIGI");
            names.insert(JEFFERSON, "JEFFERSON");
            names.insert(LINDI, "LINDI");
            names.insert(KEVIN, "KEVIN");
            names.insert(ANIS, "ANIS");
            names.insert(RETO, "RETO");
            names.insert(HAALAND, "HAALAND");
            names.insert(XENIA, "XENIA");
            names.insert(EVA, "EVA");
            names.insert(SKARA, "SKARA");
            names.insert(ROOSEVELT, "ROOSEVELT");
            names.insert(DRACULA, "DRACULA");
            names.insert(DURIM, "DURIM");
            names.insert(HARRISON, "HARRISON");
            names.insert(DRIN, "DRIN");
            names.insert(PARI, "PARI");
            names.insert(TUTI, "TUTI");
            names.insert(BENITO, "BENITO");
            names.insert(VANESSA, "VANESSA");
            names.insert(ENES, "ENES");
            names.insert(RUDOLF, "RUDOLF");
            names.insert(CERTO, "CERTO");
            names.insert(TIESTO, "TIESTO");
            names.insert(DAVID, "DAVID");
            names.insert(ATAKAN, "ATAKAN");
            names.insert(YANN, "YANN");
            names.insert(ENIS, "ENIS");
            names.insert(ALFREDO, "ALFREDO");
            names.insert(QENDRIM, "QENDRIM");
            names.insert(LEONARDO, "LEONARDO");
            names.insert(KEN, "KEN");
            names.insert(LUCA, "LUCA");
            names.insert(FLAVIO, "FLAVIO");
            names.insert(FREDI, "FREDI");
            names.insert(ALI, "ALI");
            names.insert(DILARA, "DILARA");
            names.insert(DAMIAN, "DAMIAN");
            names.insert(KAYA, "KAYA");
            names.insert(IAZI, "IAZI");
            names.insert(CHRIGI, "CHRIGI");
            names.insert(VALENTINA, "VALENTINA");
            names.insert(ALMA, "ALMA");
            names.insert(ALENA, "ALENA");
            names.insert(PATRICK, "PATRICK");
            names.insert(ONTARIO, "ONTARIO");
            names.insert(RAKIA, "RAKIA");
            names.insert(HUBERT, "HUBERT");
            names.insert(UTUS, "UTUS");
            names.insert(TOME, "TOME");
            names.insert(ZUBER, "ZUBER");
            names.insert(ADAM, "ADAM");
            names.insert(STANI, "STANI");
            names.insert(BETI, "BETI");
            names.insert(HALIT, "HALIT");
            names.insert(DRAGAN, "DRAGAN");
            names.insert(LEA, "LEA");
            names.insert(LUIS, "LUIS");
            names.insert(TATI, "TATI");
            names.insert(WEST, "WEST");
            names.insert(MIRIJAM, "MIRIJAM");
            names.insert(LIONEL, "LIONEL");
            names.insert(GIOVANNI, "GIOVANNI");
            names.insert(JOEL, "JOEL");
            names.insert(POLKA, "POLKA");
            names.insert(MALIK, "MALIK");
            names.insert(ALEXANDER, "ALEXANDER");
            names.insert(SOLOMUN, "SOLOMUN");
            names.insert(JOHNNY, "JOHNNY");
            names.insert(GRINGO, "GRINGO");
            names.insert(JONAS, "JONAS");
            names.insert(BUNDI, "BUNDI");
            names.insert(FELIX, "FELIX");
            names
        }
        pub fn excel_project(nonce: u64)
            -> ProjectMetadataOf<PolimecRuntime> {
            let bounded_name =
                BoundedVec::try_from("Polimec".as_bytes().to_vec()).unwrap();
            let bounded_symbol =
                BoundedVec::try_from("PLMC".as_bytes().to_vec()).unwrap();
            let metadata_hash =
                hashed({
                        let res =
                            ::alloc::fmt::format(format_args!("{0}-{1}", METADATA,
                                    nonce));
                        res
                    });
            ProjectMetadata {
                token_information: CurrencyMetadata {
                    name: bounded_name,
                    symbol: bounded_symbol,
                    decimals: 10,
                },
                mainnet_token_max_supply: 1_000_000_0_000_000_000,
                total_allocation_size: (50_000_0_000_000_000,
                    50_000_0_000_000_000),
                minimum_price: PriceOf::<PolimecRuntime>::from(10),
                ticket_size: TicketSize { minimum: Some(1), maximum: None },
                participants_size: ParticipantsSize {
                    minimum: Some(2),
                    maximum: None,
                },
                funding_thresholds: Default::default(),
                conversion_rate: 1,
                participation_currencies: AcceptedFundingAsset::USDT,
                funding_destination_account: ISSUER.into(),
                offchain_information_hash: Some(metadata_hash),
            }
        }
        fn excel_evaluators() -> Vec<UserToUSDBalance<PolimecRuntime>> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([UserToUSDBalance::new(LINA.into(),
                                93754 * US_DOLLAR),
                            UserToUSDBalance::new(MIA.into(), 162 * US_DOLLAR),
                            UserToUSDBalance::new(ALEXEY.into(), 7454 * US_DOLLAR),
                            UserToUSDBalance::new(PAUL.into(), 8192 * US_DOLLAR),
                            UserToUSDBalance::new(MARIA.into(), 11131 * US_DOLLAR),
                            UserToUSDBalance::new(GEORGE.into(), 4765 * US_DOLLAR),
                            UserToUSDBalance::new(CLARA.into(), 4363 * US_DOLLAR),
                            UserToUSDBalance::new(RAMONA.into(), 4120 * US_DOLLAR),
                            UserToUSDBalance::new(PASCAL.into(), 1626 * US_DOLLAR),
                            UserToUSDBalance::new(EMMA.into(), 3996 * US_DOLLAR),
                            UserToUSDBalance::new(BIBI.into(), 3441 * US_DOLLAR),
                            UserToUSDBalance::new(AHMED.into(), 8048 * US_DOLLAR),
                            UserToUSDBalance::new(HERBERT.into(), 2538 * US_DOLLAR),
                            UserToUSDBalance::new(LENI.into(), 5803 * US_DOLLAR),
                            UserToUSDBalance::new(XI.into(), 1669 * US_DOLLAR),
                            UserToUSDBalance::new(TOM.into(), 6526 * US_DOLLAR)]))
        }
        fn excel_bidders() -> Vec<BidParams<PolimecRuntime>> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([BidParams::new_with_defaults(ADAMS.into(),
                                700 * ASSET_UNIT),
                            BidParams::new_with_defaults(POLK.into(),
                                4000 * ASSET_UNIT),
                            BidParams::new_with_defaults(MARKUS.into(),
                                3000 * ASSET_UNIT),
                            BidParams::new_with_defaults(ELLA.into(), 700 * ASSET_UNIT),
                            BidParams::new_with_defaults(SKR.into(), 3400 * ASSET_UNIT),
                            BidParams::new_with_defaults(ARTHUR.into(),
                                1000 * ASSET_UNIT),
                            BidParams::new_with_defaults(MILA.into(),
                                8400 * ASSET_UNIT),
                            BidParams::new_with_defaults(LINCOLN.into(),
                                800 * ASSET_UNIT),
                            BidParams::new_with_defaults(MONROE.into(),
                                1300 * ASSET_UNIT),
                            BidParams::new_with_defaults(ARBRESHA.into(),
                                5000 * ASSET_UNIT),
                            BidParams::new_with_defaults(ELDIN.into(),
                                600 * ASSET_UNIT),
                            BidParams::new_with_defaults(HARDING.into(),
                                800 * ASSET_UNIT),
                            BidParams::new_with_defaults(SOFIA.into(),
                                3000 * ASSET_UNIT),
                            BidParams::new_with_defaults(DOMINIK.into(),
                                8000 * ASSET_UNIT),
                            BidParams::new_with_defaults(NOLAND.into(),
                                900 * ASSET_UNIT),
                            BidParams::new_with_defaults(LINA.into(),
                                8400 * ASSET_UNIT),
                            BidParams::new_with_defaults(LINA.into(),
                                1000 * ASSET_UNIT),
                            BidParams::new_with_defaults(HANNAH.into(),
                                400 * ASSET_UNIT),
                            BidParams::new_with_defaults(HOOVER.into(),
                                2000 * ASSET_UNIT),
                            BidParams::new_with_defaults(GIGI.into(), 600 * ASSET_UNIT),
                            BidParams::new_with_defaults(JEFFERSON.into(),
                                1000 * ASSET_UNIT),
                            BidParams::new_with_defaults(JEFFERSON.into(),
                                2000 * ASSET_UNIT)]))
        }
        fn excel_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([ContributionParams::new_with_defaults(DRIN.into(),
                                692 * US_DOLLAR),
                            ContributionParams::new_with_defaults(PARI.into(),
                                236 * US_DOLLAR),
                            ContributionParams::new_with_defaults(TUTI.into(),
                                24 * US_DOLLAR),
                            ContributionParams::new_with_defaults(BENITO.into(),
                                688 * US_DOLLAR),
                            ContributionParams::new_with_defaults(VANESSA.into(),
                                33 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ENES.into(),
                                1148 * US_DOLLAR),
                            ContributionParams::new_with_defaults(RUDOLF.into(),
                                35 * US_DOLLAR),
                            ContributionParams::new_with_defaults(CERTO.into(),
                                840 * US_DOLLAR),
                            ContributionParams::new_with_defaults(TIESTO.into(),
                                132 * US_DOLLAR),
                            ContributionParams::new_with_defaults(DAVID.into(),
                                21 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ATAKAN.into(),
                                59 * US_DOLLAR),
                            ContributionParams::new_with_defaults(YANN.into(),
                                89 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ENIS.into(),
                                332 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ALFREDO.into(),
                                8110 * US_DOLLAR),
                            ContributionParams::new_with_defaults(QENDRIM.into(),
                                394 * US_DOLLAR),
                            ContributionParams::new_with_defaults(LEONARDO.into(),
                                840 * US_DOLLAR),
                            ContributionParams::new_with_defaults(KEN.into(),
                                352 * US_DOLLAR),
                            ContributionParams::new_with_defaults(LUCA.into(),
                                640 * US_DOLLAR),
                            ContributionParams::new_with_defaults(FLAVIO.into(),
                                792 * US_DOLLAR),
                            ContributionParams::new_with_defaults(FREDI.into(),
                                993 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ALI.into(),
                                794 * US_DOLLAR),
                            ContributionParams::new_with_defaults(DILARA.into(),
                                256 * US_DOLLAR),
                            ContributionParams::new_with_defaults(DAMIAN.into(),
                                431 * US_DOLLAR),
                            ContributionParams::new_with_defaults(KAYA.into(),
                                935 * US_DOLLAR),
                            ContributionParams::new_with_defaults(IAZI.into(),
                                174 * US_DOLLAR),
                            ContributionParams::new_with_defaults(CHRIGI.into(),
                                877 * US_DOLLAR),
                            ContributionParams::new_with_defaults(VALENTINA.into(),
                                961 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ALMA.into(),
                                394 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ALENA.into(),
                                442 * US_DOLLAR),
                            ContributionParams::new_with_defaults(PATRICK.into(),
                                486 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ONTARIO.into(),
                                17 * US_DOLLAR),
                            ContributionParams::new_with_defaults(RAKIA.into(),
                                9424 * US_DOLLAR),
                            ContributionParams::new_with_defaults(HUBERT.into(),
                                14 * US_DOLLAR),
                            ContributionParams::new_with_defaults(UTUS.into(),
                                4906 * US_DOLLAR),
                            ContributionParams::new_with_defaults(TOME.into(),
                                68 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ZUBER.into(),
                                9037 * US_DOLLAR),
                            ContributionParams::new_with_defaults(ADAM.into(),
                                442 * US_DOLLAR),
                            ContributionParams::new_with_defaults(STANI.into(),
                                40 * US_DOLLAR),
                            ContributionParams::new_with_defaults(BETI.into(),
                                68 * US_DOLLAR),
                            ContributionParams::new_with_defaults(HALIT.into(),
                                68 * US_DOLLAR),
                            ContributionParams::new_with_defaults(DRAGAN.into(),
                                98 * US_DOLLAR),
                            ContributionParams::new_with_defaults(LEA.into(),
                                17 * US_DOLLAR),
                            ContributionParams::new_with_defaults(LUIS.into(),
                                422 * US_DOLLAR)]))
        }
        fn excel_remainders() -> Vec<ContributionParams<PolimecRuntime>> {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([ContributionParams::new_with_defaults(JOEL.into(),
                                692 * US_DOLLAR),
                            ContributionParams::new_with_defaults(POLK.into(),
                                236 * US_DOLLAR),
                            ContributionParams::new_with_defaults(MALIK.into(),
                                24 * US_DOLLAR),
                            ContributionParams::new_with_defaults(LEA.into(),
                                688 * US_DOLLAR),
                            ContributionParams::new_with_defaults(RAMONA.into(),
                                35 * US_DOLLAR),
                            ContributionParams::new_with_defaults(SOLOMUN.into(),
                                840 * US_DOLLAR),
                            ContributionParams::new_with_defaults(JONAS.into(),
                                59 * US_DOLLAR)]))
        }
        fn excel_ct_amounts() -> UserToCTBalance {
            <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(LINA.into(),
                                42916134112336, 0), (MIA.into(), 32685685157, 0),
                            (ALEXEY.into(), 1422329504123, 0),
                            (PAUL.into(), 1164821313204, 0),
                            (MARIA.into(), 1582718022129, 0),
                            (GEORGE.into(), 677535834646, 0),
                            (CLARA.into(), 620375413759, 0),
                            (RAMONA.into(), 935823219043, 0),
                            (PASCAL.into(), 231201105380, 0),
                            (EMMA.into(), 568191646431, 0),
                            (BIBI.into(), 489276139982, 0),
                            (AHMED.into(), 1144345938558, 0),
                            (HERBERT.into(), 360878478139, 0),
                            (LENI.into(), 825129160220, 0),
                            (XI.into(), 237315279753, 0), (TOM.into(), 927932603756, 0),
                            (ADAMS.into(), 700 * ASSET_UNIT, 0),
                            (POLK.into(), 4236 * ASSET_UNIT, 0),
                            (MARKUS.into(), 3000 * ASSET_UNIT, 0),
                            (ELLA.into(), 700 * ASSET_UNIT, 0),
                            (SKR.into(), 3400 * ASSET_UNIT, 0),
                            (ARTHUR.into(), 1000 * ASSET_UNIT, 0),
                            (MILA.into(), 8400 * ASSET_UNIT, 0),
                            (LINCOLN.into(), 800 * ASSET_UNIT, 0),
                            (MONROE.into(), 1300 * ASSET_UNIT, 0),
                            (ARBRESHA.into(), 5000 * ASSET_UNIT, 0),
                            (ELDIN.into(), 600 * ASSET_UNIT, 0),
                            (HARDING.into(), 800 * ASSET_UNIT, 0),
                            (SOFIA.into(), 3000 * ASSET_UNIT, 0),
                            (DOMINIK.into(), 8000 * ASSET_UNIT, 0),
                            (NOLAND.into(), 900 * ASSET_UNIT, 0),
                            (HANNAH.into(), 400 * ASSET_UNIT, 0),
                            (HOOVER.into(), 2000 * ASSET_UNIT, 0),
                            (GIGI.into(), 600 * ASSET_UNIT, 0),
                            (JEFFERSON.into(), 3000 * ASSET_UNIT, 0),
                            (DRIN.into(), 692 * ASSET_UNIT, 0),
                            (PARI.into(), 236 * ASSET_UNIT, 0),
                            (TUTI.into(), 24 * ASSET_UNIT, 0),
                            (BENITO.into(), 688 * ASSET_UNIT, 0),
                            (VANESSA.into(), 33 * ASSET_UNIT, 0),
                            (ENES.into(), 1148 * ASSET_UNIT, 0),
                            (RUDOLF.into(), 35 * ASSET_UNIT, 0),
                            (CERTO.into(), 840 * ASSET_UNIT, 0),
                            (TIESTO.into(), 132 * ASSET_UNIT, 0),
                            (DAVID.into(), 21 * ASSET_UNIT, 0),
                            (ATAKAN.into(), 59 * ASSET_UNIT, 0),
                            (YANN.into(), 89 * ASSET_UNIT, 0),
                            (ENIS.into(), 332 * ASSET_UNIT, 0),
                            (ALFREDO.into(), 8110 * ASSET_UNIT, 0),
                            (QENDRIM.into(), 394 * ASSET_UNIT, 0),
                            (LEONARDO.into(), 840 * ASSET_UNIT, 0),
                            (KEN.into(), 352 * ASSET_UNIT, 0),
                            (LUCA.into(), 640 * ASSET_UNIT, 0),
                            (FLAVIO.into(), 792 * ASSET_UNIT, 0),
                            (FREDI.into(), 993 * ASSET_UNIT, 0),
                            (ALI.into(), 794 * ASSET_UNIT, 0),
                            (DILARA.into(), 256 * ASSET_UNIT, 0),
                            (DAMIAN.into(), 431 * ASSET_UNIT, 0),
                            (KAYA.into(), 935 * ASSET_UNIT, 0),
                            (IAZI.into(), 174 * ASSET_UNIT, 0),
                            (CHRIGI.into(), 877 * ASSET_UNIT, 0),
                            (VALENTINA.into(), 961 * ASSET_UNIT, 0),
                            (ALMA.into(), 394 * ASSET_UNIT, 0),
                            (ALENA.into(), 442 * ASSET_UNIT, 0),
                            (PATRICK.into(), 486 * ASSET_UNIT, 0),
                            (ONTARIO.into(), 17 * ASSET_UNIT, 0),
                            (RAKIA.into(), 9424 * ASSET_UNIT, 0),
                            (HUBERT.into(), 14 * ASSET_UNIT, 0),
                            (UTUS.into(), 4906 * ASSET_UNIT, 0),
                            (TOME.into(), 68 * ASSET_UNIT, 0),
                            (ZUBER.into(), 9037 * ASSET_UNIT, 0),
                            (ADAM.into(), 442 * ASSET_UNIT, 0),
                            (STANI.into(), 40 * ASSET_UNIT, 0),
                            (BETI.into(), 68 * ASSET_UNIT, 0),
                            (HALIT.into(), 68 * ASSET_UNIT, 0),
                            (DRAGAN.into(), 98 * ASSET_UNIT, 0),
                            (LEA.into(), 705 * ASSET_UNIT, 0),
                            (LUIS.into(), 422 * ASSET_UNIT, 0),
                            (JOEL.into(), 692 * ASSET_UNIT, 0),
                            (MALIK.into(), 24 * ASSET_UNIT, 0),
                            (SOLOMUN.into(), 840 * ASSET_UNIT, 0),
                            (JONAS.into(), 59 * ASSET_UNIT, 0)]))
        }
    }
    mod governance {
        use crate::{polimec_base::ED, *};
        /// Tests for the oracle pallet integration.
        /// Alice, Bob, Charlie are members of the OracleProvidersMembers.
        /// Only members should be able to feed data into the oracle.
        use frame_support::traits::{
            fungible::{
                BalancedHold, Inspect, MutateFreeze, MutateHold, Unbalanced,
            },
            WithdrawReasons, Hooks,
        };
        use macros::generate_accounts;
        use sp_runtime::{traits::Hash, Digest};
        use frame_support::{
            dispatch::GetDispatchInfo,
            traits::{
                fungible::InspectFreeze, tokens::Precision, Imbalance,
                LockableCurrency, ReservableCurrency, StorePreimage,
            },
        };
        use pallet_democracy::{
            AccountVote, Conviction, GetElectorate, ReferendumInfo, Vote,
        };
        use pallet_vesting::VestingInfo;
        use polimec_base_runtime::{
            Balances, Council, Democracy, Elections, ParachainStaking,
            Preimage, RuntimeOrigin, TechnicalCommittee, Treasury, Vesting,
        };
        use tests::defaults::*;
        use xcm_emulator::helpers::get_account_id_from_seed;
        pub const PEPE: [u8; 32] =
            [80u8, 69u8, 80u8, 69u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const CARLOS: [u8; 32] =
            [67u8, 65u8, 82u8, 76u8, 79u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub fn names() -> std::collections::HashMap<[u8; 32], &'static str> {
            let mut names = std::collections::HashMap::new();
            names.insert(PEPE, "PEPE");
            names.insert(CARLOS, "CARLOS");
            names
        }
        fn assert_same_members(expected: Vec<AccountId>,
            actual: &Vec<AccountId>) {
            match (&expected.len(), &actual.len()) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val, ::core::option::Option::None);
                        }
                }
            };
            for member in expected {
                if !actual.contains(&member) {
                        ::core::panicking::panic("assertion failed: actual.contains(&member)")
                    };
            }
        }
        fn create_vested_account() -> AccountId {
            let alice = BaseNet::account_id_of(ALICE);
            let new_account =
                get_account_id_from_seed::<sr25519::Public>("NEW_ACCOUNT");
            match (&Balances::balance(&new_account), &(0 * PLMC)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val, ::core::option::Option::None);
                        }
                }
            };
            let vesting_schedule = VestingInfo::new(200 * PLMC + ED, PLMC, 1);
            let is =
                Vesting::vested_transfer(RuntimeOrigin::signed(alice.clone()),
                    sp_runtime::MultiAddress::Id(new_account.clone()),
                    vesting_schedule);
            match is {
                Ok(_) => (),
                _ =>
                    if !false {
                            {
                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                        is));
                            }
                        },
            };
            new_account
        }
        fn run_gov_n_blocks(n: usize) {
            for _ in 0..n {
                BaseNet::execute_with(||
                        {
                            let block_number =
                                polimec_base_runtime::System::block_number();
                            let header = polimec_base_runtime::System::finalize();
                            let pre_digest = Digest { logs: ::alloc::vec::Vec::new() };
                            polimec_base_runtime::System::reset_events();
                            let next_block_number = block_number + 1u32;
                            polimec_base_runtime::Vesting::on_initialize(next_block_number);
                            polimec_base_runtime::Elections::on_initialize(next_block_number);
                            polimec_base_runtime::Council::on_initialize(next_block_number);
                            polimec_base_runtime::TechnicalCommittee::on_initialize(next_block_number);
                            polimec_base_runtime::Treasury::on_initialize(next_block_number);
                            polimec_base_runtime::Democracy::on_initialize(next_block_number);
                            polimec_base_runtime::Preimage::on_initialize(next_block_number);
                            polimec_base_runtime::Scheduler::on_initialize(next_block_number);
                            polimec_base_runtime::System::initialize(&next_block_number,
                                &header.hash(), &pre_digest);
                        });
            }
        }
        fn do_vote(account: AccountId, index: u32, approve: bool,
            amount: u128) {
            let is =
                Democracy::vote(RuntimeOrigin::signed(account.clone()), index,
                    AccountVote::Standard {
                        balance: amount,
                        vote: Vote {
                            aye: approve,
                            conviction: Conviction::Locked1x,
                        },
                    });
            match is {
                Ok(_) => (),
                _ =>
                    if !false {
                            {
                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                        is));
                            }
                        },
            };
        }
        fn do_council_vote_for(accounts: Vec<(AccountId, bool)>, index: u32,
            hash: polimec_base_runtime::Hash) {
            for (account, approve) in accounts {
                let is =
                    Council::vote(RuntimeOrigin::signed(account.clone()), hash,
                        index, approve);
                match is {
                    Ok(_) => (),
                    _ =>
                        if !false {
                                {
                                    ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                            is));
                                }
                            },
                };
            }
        }
    }
    mod oracle {
        use crate::*;
        /// Tests for the oracle pallet integration.
        /// Alice, Bob, Charlie are members of the OracleProvidersMembers.
        /// Only members should be able to feed data into the oracle.
        use parity_scale_codec::alloc::collections::HashMap;
        use polimec_parachain_runtime::{Oracle, RuntimeOrigin};
        use sp_runtime::{bounded_vec, BoundedVec, FixedU128};
        use tests::defaults::*;
        fn values(values: [f64; 4])
            ->
                BoundedVec<(u32, FixedU128),
                <polimec_parachain_runtime::Runtime as
                orml_oracle::Config<()>>::MaxFeedValues> {
            let [dot, usdc, usdt, plmc] = values;
            {
                <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(0u32,
                                            FixedU128::from_float(dot)),
                                        (1337u32, FixedU128::from_float(usdc)),
                                        (1984u32, FixedU128::from_float(usdt)),
                                        (2069u32,
                                            FixedU128::from_float(plmc))])).try_into().unwrap()
            }
        }
    }
    mod reserve_backed_transfers {
        use crate::*;
        use frame_support::{
            traits::{
                fungible::{Inspect as FungibleInspect, Unbalanced},
                fungibles::{Inspect, Mutate},
                PalletInfoAccess,
            },
            weights::WeightToFee,
        };
        use sp_runtime::DispatchError;
        const RESERVE_TRANSFER_AMOUNT: u128 = 10_0_000_000_000;
        const MAX_REF_TIME: u64 = 5_000_000_000;
        const MAX_PROOF_SIZE: u64 = 200_000;
        fn create_asset_on_asset_hub(asset_id: u32) {
            if asset_id == 0 { return; }
            let admin_account = AssetNet::account_id_of(FERDIE);
            AssetNet::execute_with(||
                    {
                        let is =
                            AssetHubAssets::force_create(AssetHubOrigin::root(),
                                asset_id.into(),
                                sp_runtime::MultiAddress::Id(admin_account.clone()), true,
                                0_0_010_000_000u128);
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                    });
        }
        fn mint_asset_on_asset_hub_to(asset_id: u32,
            recipient: &AssetHubAccountId, amount: u128) {
            AssetNet::execute_with(||
                    {
                        match asset_id {
                            0 => {
                                let is = AssetHubBalances::write_balance(recipient, amount);
                                match is {
                                    Ok(_) => (),
                                    _ =>
                                        if !false {
                                                {
                                                    ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                            is));
                                                }
                                            },
                                };
                            }
                            _ => {
                                let is =
                                    AssetHubAssets::mint_into(asset_id, recipient, amount);
                                match is {
                                    Ok(_) => (),
                                    _ =>
                                        if !false {
                                                {
                                                    ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                            is));
                                                }
                                            },
                                };
                            }
                        }
                        AssetHubSystem::reset_events();
                    });
        }
        fn get_polimec_balances(asset_id: u32, user_account: AccountId)
            -> (u128, u128, u128, u128) {
            BaseNet::execute_with(||
                    {
                        (BaseForeignAssets::balance(asset_id, user_account.clone()),
                            BaseBalances::balance(&user_account.clone()),
                            BaseForeignAssets::total_issuance(asset_id),
                            BaseBalances::total_issuance())
                    })
        }
        fn get_asset_hub_balances(asset_id: u32, user_account: AccountId,
            polimec_account: AccountId) -> (u128, u128, u128) {
            AssetNet::execute_with(||
                    {
                        match asset_id {
                            0 =>
                                (AssetHubBalances::balance(&user_account),
                                    AssetHubBalances::balance(&polimec_account),
                                    AssetHubBalances::total_issuance()),
                            _ =>
                                (AssetHubAssets::balance(asset_id, user_account.clone()),
                                    AssetHubAssets::balance(asset_id, polimec_account.clone()),
                                    AssetHubAssets::total_issuance(asset_id)),
                        }
                    })
        }
        /// Test the reserve based transfer from asset_hub to Polimec. Depending of the asset_id we
        /// transfer either USDT, USDC and DOT.
        fn test_reserve_to_polimec(asset_id: u32) {
            create_asset_on_asset_hub(asset_id);
            let asset_hub_asset_id: MultiLocation =
                match asset_id {
                    0 => Parent.into(),
                    _ =>
                        (PalletInstance(AssetHubAssets::index() as u8),
                                GeneralIndex(asset_id as u128)).into(),
                };
            let alice_account = BaseNet::account_id_of(ALICE);
            let polimec_sibling_account =
                AssetNet::sovereign_account_id_of((Parent,
                            Parachain(BaseNet::para_id().into())).into());
            let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);
            mint_asset_on_asset_hub_to(asset_id, &alice_account,
                100_0_000_000_000);
            let (polimec_prev_alice_asset_balance,
                    polimec_prev_alice_plmc_balance,
                    polimec_prev_asset_issuance, polimec_prev_plmc_issuance) =
                get_polimec_balances(asset_id, alice_account.clone());
            let (asset_hub_prev_alice_asset_balance,
                    asset_hub_prev_polimec_asset_balance,
                    asset_hub_prev_asset_issuance) =
                get_asset_hub_balances(asset_id, alice_account.clone(),
                    polimec_sibling_account.clone());
            AssetNet::execute_with(||
                    {
                        let asset_transfer: MultiAsset =
                            (asset_hub_asset_id, RESERVE_TRANSFER_AMOUNT).into();
                        let origin = AssetHubOrigin::signed(alice_account.clone());
                        let dest: VersionedMultiLocation =
                            ParentThen(X1(Parachain(BaseNet::para_id().into()))).into();
                        let beneficiary: VersionedMultiLocation =
                            AccountId32 {
                                    network: None,
                                    id: alice_account.clone().into(),
                                }.into();
                        let assets: VersionedMultiAssets = asset_transfer.into();
                        let fee_asset_item = 0;
                        let weight_limit = Unlimited;
                        let call =
                            AssetHubXcmPallet::limited_reserve_transfer_assets(origin,
                                Box::new(dest), Box::new(beneficiary), Box::new(assets),
                                fee_asset_item, weight_limit);
                        let is = call;
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                    });
            BaseNet::execute_with(||
                    {
                        let mut message: Vec<String> = Vec::new();
                        let mut events =
                            <BaseNet as ::xcm_emulator::Chain>::events();
                        let mut event_received = false;
                        let mut meet_conditions = true;
                        let mut index_match = 0;
                        let mut event_message: Vec<String> = Vec::new();
                        for (index, event) in events.iter().enumerate() {
                            meet_conditions = true;
                            match event {
                                BaseEvent::MessageQueue(pallet_message_queue::Event::Processed {
                                    success: true, .. }) => {
                                    event_received = true;
                                    let mut conditions_message: Vec<String> = Vec::new();
                                    if event_received && meet_conditions {
                                            index_match = index;
                                            break;
                                        } else { event_message.extend(conditions_message); }
                                }
                                _ => {}
                            }
                        }
                        if event_received && !meet_conditions {
                                message.push({
                                        let res =
                                            ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was received but some of its attributes did not meet the conditions:\n{2}",
                                                    "BaseNet",
                                                    "BaseEvent::MessageQueue(pallet_message_queue::Event::Processed {\nsuccess: true, .. })",
                                                    event_message.concat()));
                                        res
                                    });
                            } else if !event_received {
                               message.push({
                                       let res =
                                           ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was never received. All events:\n{2:#?}",
                                                   "BaseNet",
                                                   "BaseEvent::MessageQueue(pallet_message_queue::Event::Processed {\nsuccess: true, .. })",
                                                   <BaseNet as ::xcm_emulator::Chain>::events()));
                                       res
                                   });
                           } else { events.remove(index_match); }
                        if !message.is_empty() {
                                <BaseNet as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::BaseNet",
                                                                    "integration_tests::tests::reserve_backed_transfers",
                                                                    "integration-tests/src/tests/reserve_backed_transfers.rs"),
                                                            142u32, ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                { ::core::panicking::panic_display(&message.concat()); }
                            };
                    });
            let (polimec_post_alice_asset_balance,
                    polimec_post_alice_plmc_balance,
                    polimec_post_asset_issuance, polimec_post_plmc_issuance) =
                get_polimec_balances(asset_id, alice_account.clone());
            let (asset_hub_post_alice_asset_balance,
                    asset_hub_post_polimec_asset_balance,
                    asset_hub_post_asset_issuance) =
                get_asset_hub_balances(asset_id, alice_account.clone(),
                    polimec_sibling_account.clone());
            let polimec_delta_alice_asset_balance =
                polimec_post_alice_asset_balance.abs_diff(polimec_prev_alice_asset_balance);
            let polimec_delta_alice_plmc_balance =
                polimec_post_alice_plmc_balance.abs_diff(polimec_prev_alice_plmc_balance);
            let polimec_delta_asset_issuance =
                polimec_post_asset_issuance.abs_diff(polimec_prev_asset_issuance);
            let polimec_delta_plmc_issuance =
                polimec_post_plmc_issuance.abs_diff(polimec_prev_plmc_issuance);
            let asset_hub_delta_alice_asset_balance =
                asset_hub_post_alice_asset_balance.abs_diff(asset_hub_prev_alice_asset_balance);
            let asset_hub_delta_polimec_asset_balance =
                asset_hub_post_polimec_asset_balance.abs_diff(asset_hub_prev_polimec_asset_balance);
            let asset_hub_delta_asset_issuance =
                asset_hub_post_asset_issuance.abs_diff(asset_hub_prev_asset_issuance);
            if !(polimec_delta_alice_asset_balance >=
                                RESERVE_TRANSFER_AMOUNT -
                                    polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight)
                            &&
                            polimec_delta_alice_asset_balance <=
                                RESERVE_TRANSFER_AMOUNT) {
                    {
                        ::core::panicking::panic_fmt(format_args!("Polimec alice_account.clone() Asset balance should have increased by at least the transfer amount minus the XCM execution fee"));
                    }
                };
            if !(polimec_delta_asset_issuance >=
                                RESERVE_TRANSFER_AMOUNT -
                                    polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight)
                            && polimec_delta_asset_issuance <= RESERVE_TRANSFER_AMOUNT)
                    {
                    {
                        ::core::panicking::panic_fmt(format_args!("Polimec Asset issuance should have increased by at least the transfer amount minus the XCM execution fee"));
                    }
                };
            match (&asset_hub_delta_alice_asset_balance,
                    &RESERVE_TRANSFER_AMOUNT) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("AssetHub alice_account.clone() Asset balance should have decreased by the transfer amount")));
                        }
                }
            };
            if !(asset_hub_delta_polimec_asset_balance ==
                            RESERVE_TRANSFER_AMOUNT) {
                    {
                        ::core::panicking::panic_fmt(format_args!("The USDT balance of Polimec\'s sovereign account on AssetHub should receive the transfer amount"));
                    }
                };
            if !(asset_hub_delta_asset_issuance == 0u128) {
                    {
                        ::core::panicking::panic_fmt(format_args!("AssetHub\'s USDT issuance should not change, since it acts as a reserve for that asset"));
                    }
                };
            match (&polimec_delta_alice_plmc_balance, &0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec alice_account.clone() PLMC balance should not have changed")));
                        }
                }
            };
            match (&polimec_delta_plmc_issuance, &0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec PLMC issuance should not have changed")));
                        }
                }
            };
        }
        fn test_polimec_to_reserve(asset_id: u32) {
            create_asset_on_asset_hub(asset_id);
            let asset_hub_asset_id: MultiLocation =
                match asset_id {
                    0 => Parent.into(),
                    _ =>
                        ParentThen(X3(Parachain(AssetNet::para_id().into()),
                                    PalletInstance(AssetHubAssets::index() as u8),
                                    GeneralIndex(asset_id as u128))).into(),
                };
            let alice_account = BaseNet::account_id_of(ALICE);
            let polimec_sibling_account =
                AssetNet::sovereign_account_id_of((Parent,
                            Parachain(BaseNet::para_id().into())).into());
            let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);
            mint_asset_on_asset_hub_to(asset_id, &polimec_sibling_account,
                RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000);
            BaseNet::execute_with(||
                    {
                        let is =
                            BaseForeignAssets::mint_into(asset_id, &alice_account,
                                RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000);
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                    });
            let (polimec_prev_alice_asset_balance,
                    polimec_prev_alice_plmc_balance,
                    polimec_prev_asset_issuance, polimec_prev_plmc_issuance) =
                get_polimec_balances(asset_id, alice_account.clone());
            let (asset_hub_prev_alice_asset_balance,
                    asset_hub_prev_polimec_asset_balance,
                    asset_hub_prev_asset_issuance) =
                get_asset_hub_balances(asset_id, alice_account.clone(),
                    polimec_sibling_account.clone());
            let transferable_asset_plus_exec_fee: MultiAsset =
                (asset_hub_asset_id,
                        RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000).into();
            let mut asset_hub_exec_fee: MultiAsset =
                (asset_hub_asset_id, 1_0_000_000_000u128).into();
            asset_hub_exec_fee.reanchor(&(ParentThen(X1(Parachain(AssetNet::para_id().into()))).into()),
                    Here).unwrap();
            let transfer_xcm: Xcm<BaseCall> =
                Xcm(<[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([WithdrawAsset(transferable_asset_plus_exec_fee.clone().into()),
                                    BuyExecution {
                                        fees: transferable_asset_plus_exec_fee.clone(),
                                        weight_limit: Limited(max_weight),
                                    },
                                    InitiateReserveWithdraw {
                                        assets: All.into(),
                                        reserve: MultiLocation::new(1,
                                            X1(Parachain(AssetNet::para_id().into()))),
                                        xcm: Xcm(<[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([BuyExecution {
                                                                fees: asset_hub_exec_fee,
                                                                weight_limit: Limited(max_weight),
                                                            },
                                                            DepositAsset {
                                                                assets: All.into(),
                                                                beneficiary: MultiLocation::new(0,
                                                                    AccountId32 {
                                                                        network: None,
                                                                        id: alice_account.clone().into(),
                                                                    }),
                                                            }]))),
                                    }])));
            BaseNet::execute_with(||
                    {
                        let is =
                            BaseXcmPallet::execute(BaseOrigin::signed(alice_account.clone()),
                                Box::new(VersionedXcm::V3(transfer_xcm)), max_weight);
                        match is {
                            Ok(_) => (),
                            _ =>
                                if !false {
                                        {
                                            ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                    is));
                                        }
                                    },
                        };
                    });
            AssetNet::execute_with(||
                    {
                        let mut message: Vec<String> = Vec::new();
                        let mut events =
                            <AssetNet as ::xcm_emulator::Chain>::events();
                        let mut event_received = false;
                        let mut meet_conditions = true;
                        let mut index_match = 0;
                        let mut event_message: Vec<String> = Vec::new();
                        for (index, event) in events.iter().enumerate() {
                            meet_conditions = true;
                            match event {
                                AssetHubEvent::MessageQueue(pallet_message_queue::Event::Processed {
                                    success: true, .. }) => {
                                    event_received = true;
                                    let mut conditions_message: Vec<String> = Vec::new();
                                    if event_received && meet_conditions {
                                            index_match = index;
                                            break;
                                        } else { event_message.extend(conditions_message); }
                                }
                                _ => {}
                            }
                        }
                        if event_received && !meet_conditions {
                                message.push({
                                        let res =
                                            ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was received but some of its attributes did not meet the conditions:\n{2}",
                                                    "AssetNet",
                                                    "AssetHubEvent::MessageQueue(pallet_message_queue::Event::Processed {\nsuccess: true, .. })",
                                                    event_message.concat()));
                                        res
                                    });
                            } else if !event_received {
                               message.push({
                                       let res =
                                           ::alloc::fmt::format(format_args!("\n\n{0}::\u{{1b}}[31m{1}\u{{1b}}[0m was never received. All events:\n{2:#?}",
                                                   "AssetNet",
                                                   "AssetHubEvent::MessageQueue(pallet_message_queue::Event::Processed {\nsuccess: true, .. })",
                                                   <AssetNet as ::xcm_emulator::Chain>::events()));
                                       res
                                   });
                           } else { events.remove(index_match); }
                        if !message.is_empty() {
                                <AssetNet as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::AssetNet",
                                                                    "integration_tests::tests::reserve_backed_transfers",
                                                                    "integration-tests/src/tests/reserve_backed_transfers.rs"),
                                                            273u32, ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                { ::core::panicking::panic_display(&message.concat()); }
                            };
                    });
            let (polimec_post_alice_asset_balance,
                    polimec_post_alice_plmc_balance,
                    polimec_post_asset_issuance, polimec_post_plmc_issuance) =
                get_polimec_balances(asset_id, alice_account.clone());
            let (asset_hub_post_alice_asset_balance,
                    asset_hub_post_polimec_asset_balance,
                    asset_hub_post_asset_issuance) =
                get_asset_hub_balances(asset_id, alice_account.clone(),
                    polimec_sibling_account.clone());
            let polimec_delta_alice_asset_balance =
                polimec_post_alice_asset_balance.abs_diff(polimec_prev_alice_asset_balance);
            let polimec_delta_alice_plmc_balance =
                polimec_post_alice_plmc_balance.abs_diff(polimec_prev_alice_plmc_balance);
            let polimec_delta_asset_issuance =
                polimec_post_asset_issuance.abs_diff(polimec_prev_asset_issuance);
            let polimec_delta_plmc_issuance =
                polimec_post_plmc_issuance.abs_diff(polimec_prev_plmc_issuance);
            let asset_hub_delta_alice_asset_balance =
                asset_hub_post_alice_asset_balance.abs_diff(asset_hub_prev_alice_asset_balance);
            let asset_hub_delta_polimec_asset_balance =
                asset_hub_post_polimec_asset_balance.abs_diff(asset_hub_prev_polimec_asset_balance);
            let asset_hub_delta_asset_issuance =
                asset_hub_post_asset_issuance.abs_diff(asset_hub_prev_asset_issuance);
            match (&polimec_delta_alice_asset_balance,
                    &(RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec\'s alice_account Asset balance should decrease by the transfer amount")));
                        }
                }
            };
            match (&polimec_delta_asset_issuance,
                    &(RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000)) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec\'s Asset issuance should decrease by transfer amount due to burn")));
                        }
                }
            };
            match (&polimec_delta_plmc_issuance, &0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec\'s PLMC issuance should not change, since all xcm token transfer are done in Asset, and no fees are burnt since no extrinsics are dispatched")));
                        }
                }
            };
            match (&polimec_delta_alice_plmc_balance, &0) {
                (left_val, right_val) => {
                    if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(kind, &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(format_args!("Polimec\'s Alice PLMC should not change")));
                        }
                }
            };
            if !(asset_hub_delta_alice_asset_balance >=
                                RESERVE_TRANSFER_AMOUNT &&
                            asset_hub_delta_alice_asset_balance <=
                                RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000) {
                    {
                        ::core::panicking::panic_fmt(format_args!("AssetHub\'s alice_account Asset balance should increase by at least the transfer amount minus the max allowed fees"));
                    }
                };
            if !(asset_hub_delta_polimec_asset_balance >=
                                RESERVE_TRANSFER_AMOUNT &&
                            asset_hub_delta_polimec_asset_balance <=
                                RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000) {
                    {
                        ::core::panicking::panic_fmt(format_args!("Polimecs sovereign account on asset hub should have transferred Asset amount to Alice"));
                    }
                };
            if !(asset_hub_delta_asset_issuance <=
                            system_parachains_constants::polkadot::fee::WeightToFee::weight_to_fee(&max_weight))
                    {
                    {
                        ::core::panicking::panic_fmt(format_args!("AssetHub\'s Asset issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"));
                    }
                };
        }
    }
    mod vest {
        use crate::{polimec_base::ED, *};
        /// Tests for the oracle pallet integration.
        /// Alice, Bob, Charlie are members of the OracleProvidersMembers.
        /// Only members should be able to feed data into the oracle.
        use frame_support::traits::fungible::Inspect;
        use frame_support::traits::fungible::Mutate;
        use macros::generate_accounts;
        use pallet_funding::assert_close_enough;
        use pallet_vesting::VestingInfo;
        use polimec_base_runtime::{
            Balances, ParachainStaking, RuntimeOrigin, Vesting,
        };
        use sp_runtime::Perquintill;
        use tests::defaults::*;
        use xcm_emulator::helpers::get_account_id_from_seed;
        pub const PEPE: [u8; 32] =
            [80u8, 69u8, 80u8, 69u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub const CARLOS: [u8; 32] =
            [67u8, 65u8, 82u8, 76u8, 79u8, 83u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8];
        pub fn names() -> std::collections::HashMap<[u8; 32], &'static str> {
            let mut names = std::collections::HashMap::new();
            names.insert(PEPE, "PEPE");
            names.insert(CARLOS, "CARLOS");
            names
        }
    }
}
pub use constants::{
    accounts::*, asset_hub, penpal, polimec, polimec_base, polkadot,
};
pub use frame_support::{
    assert_noop, assert_ok, pallet_prelude::Weight, parameter_types,
    traits::Hooks,
};
pub use parachains_common::{
    AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber,
};
pub use sp_core::{sr25519, storage::Storage, Encode, Get};
pub use xcm::prelude::*;
pub use xcm_emulator::{
    assert_expected_events, bx, decl_test_networks, decl_test_parachains,
    decl_test_relay_chains, Chain,
    helpers::{weight_within_threshold, within_threshold},
    BridgeMessageHandler, Network, ParaId, Parachain, RelayChain, TestExt,
};
pub struct PolkadotRelay<N>(::xcm_emulator::PhantomData<N>);
#[automatically_derived]
impl<N: ::core::clone::Clone> ::core::clone::Clone for PolkadotRelay<N> {
    #[inline]
    fn clone(&self) -> PolkadotRelay<N> {
        PolkadotRelay(::core::clone::Clone::clone(&self.0))
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Chain for PolkadotRelay<N> {
    type Network = N;
    type Runtime = polkadot_runtime::Runtime;
    type RuntimeCall = polkadot_runtime::RuntimeCall;
    type RuntimeOrigin = polkadot_runtime::RuntimeOrigin;
    type RuntimeEvent = polkadot_runtime::RuntimeEvent;
    type System = ::xcm_emulator::SystemPallet<Self::Runtime>;
    fn account_data_of(account: ::xcm_emulator::AccountIdOf<Self::Runtime>)
        -> ::xcm_emulator::AccountData<::xcm_emulator::Balance> {
        <Self as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                ::xcm_emulator::SystemPallet::<Self::Runtime>::account(account).data.into())
    }
    fn events() -> Vec<<Self as ::xcm_emulator::Chain>::RuntimeEvent> {
        Self::System::events().iter().map(|record|
                    record.event.clone()).collect()
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::RelayChain for
    PolkadotRelay<N> {
    type SovereignAccountOf =
        polkadot_runtime::xcm_config::SovereignAccountOf;
    type MessageProcessor =
        ::xcm_emulator::DefaultRelayMessageProcessor<PolkadotRelay<N>>;
    fn init() {
        use ::xcm_emulator::TestExt;
        LOCAL_EXT_POLKADOTRELAY.with(|v|
                *v.borrow_mut() = Self::build_new_ext(polkadot::genesis()));
    }
}
pub trait PolkadotRelayRelayPallet {
    type System;
    type Balances;
    type XcmPallet;
}
impl<N: ::xcm_emulator::Network> PolkadotRelayRelayPallet for PolkadotRelay<N>
    {
    type System = polkadot_runtime::System;
    type Balances = polkadot_runtime::Balances;
    type XcmPallet = polkadot_runtime::XcmPallet;
}
pub const LOCAL_EXT_POLKADOTRELAY:
    ::std::thread::LocalKey<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
    =
    {
        #[inline]
        fn __init()
            -> ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities> {
            ::xcm_emulator::RefCell::new(::xcm_emulator::TestExternalities::new(polkadot::genesis()))
        }
        #[inline]
        unsafe fn __getit(init:
                ::std::option::Option<&mut ::std::option::Option<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>>)
            ->
                ::std::option::Option<&'static ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>> {
            #[thread_local]
            static __KEY:
                ::std::thread::local_impl::Key<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
                =
                ::std::thread::local_impl::Key::<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>::new();
            unsafe {
                __KEY.get(move ||
                        {
                            if let ::std::option::Option::Some(init) = init {
                                    if let ::std::option::Option::Some(value) = init.take() {
                                            return value;
                                        } else if true {
                                           {
                                               ::core::panicking::panic_fmt(format_args!("internal error: entered unreachable code: {0}",
                                                       format_args!("missing default value")));
                                           };
                                       }
                                }
                            __init()
                        })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub struct GLOBAL_EXT_POLKADOTRELAY {
    __private_field: (),
}
#[doc(hidden)]
pub static GLOBAL_EXT_POLKADOTRELAY: GLOBAL_EXT_POLKADOTRELAY =
    GLOBAL_EXT_POLKADOTRELAY { __private_field: () };
impl ::lazy_static::__Deref for GLOBAL_EXT_POLKADOTRELAY {
    type Target =
        ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
        ::xcm_emulator::TestExternalities>>>;
    fn deref(&self)
        ->
            &::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
            ::xcm_emulator::TestExternalities>>> {
        #[inline(always)]
        fn __static_ref_initialize()
            ->
                ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            ::xcm_emulator::Mutex::new(::xcm_emulator::RefCell::new(::xcm_emulator::HashMap::new()))
        }
        #[inline(always)]
        fn __stability()
            ->
                &'static ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            static LAZY:
                ::lazy_static::lazy::Lazy<::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>>> =
                ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for GLOBAL_EXT_POLKADOTRELAY {
    fn initialize(lazy: &Self) { let _ = &**lazy; }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::TestExt for PolkadotRelay<N>
    {
    fn build_new_ext(storage: ::xcm_emulator::Storage)
        -> ::xcm_emulator::TestExternalities {
        use ::xcm_emulator::{sp_tracing, Network, Chain, TestExternalities};
        let mut ext = TestExternalities::new(storage);
        ext.execute_with(||
                {

                    #[allow(clippy :: no_effect)]
                    ();
                    sp_tracing::try_init_simple();
                    let mut block_number =
                        <Self as Chain>::System::block_number();
                    block_number = std::cmp::max(1, block_number);
                    <Self as Chain>::System::set_block_number(block_number);
                });
        ext
    }
    fn new_ext() -> ::xcm_emulator::TestExternalities {
        Self::build_new_ext(polkadot::genesis())
    }
    fn move_ext_out(id: &'static str) {
        use ::xcm_emulator::Deref;
        let local_ext = LOCAL_EXT_POLKADOTRELAY.with(|v| { v.take() });
        let global_ext_guard = GLOBAL_EXT_POLKADOTRELAY.lock().unwrap();
        global_ext_guard.deref().borrow_mut().insert(id.to_string(),
            local_ext);
    }
    fn move_ext_in(id: &'static str) {
        use ::xcm_emulator::Deref;
        let mut global_ext_unlocked = false;
        while !global_ext_unlocked {
            let global_ext_result = GLOBAL_EXT_POLKADOTRELAY.try_lock();
            if let Ok(global_ext_guard) = global_ext_result {
                    if !global_ext_guard.deref().borrow().contains_key(id) {
                            drop(global_ext_guard);
                        } else { global_ext_unlocked = true; }
                }
        }
        let mut global_ext_guard = GLOBAL_EXT_POLKADOTRELAY.lock().unwrap();
        let global_ext = global_ext_guard.deref();
        LOCAL_EXT_POLKADOTRELAY.with(|v|
                { v.replace(global_ext.take().remove(id).unwrap()); });
    }
    fn reset_ext() {
        LOCAL_EXT_POLKADOTRELAY.with(|v|
                *v.borrow_mut() = Self::build_new_ext(polkadot::genesis()));
    }
    fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
        use ::xcm_emulator::{Chain, Network};
        <N>::init();
        let r =
            LOCAL_EXT_POLKADOTRELAY.with(|v|
                    v.borrow_mut().execute_with(execute));
        LOCAL_EXT_POLKADOTRELAY.with(|v|
                {
                    v.borrow_mut().execute_with(||
                            {
                                use ::xcm_emulator::polkadot_primitives::runtime_api::runtime_decl_for_parachain_host::ParachainHostV5;
                                for para_id in <N>::para_ids() {
                                    let downward_messages =
                                        <Self as
                                                        ::xcm_emulator::Chain>::Runtime::dmq_contents(para_id.into()).into_iter().map(|inbound|
                                                (inbound.sent_at, inbound.msg));
                                    if downward_messages.len() == 0 { continue; }
                                    <N>::send_downward_messages(para_id,
                                        downward_messages.into_iter());
                                }
                                Self::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::PolkadotRelay", "integration_tests",
                                                                    "integration-tests/src/lib.rs"), 33u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                <Self as Chain>::System::reset_events();
                            })
                });
        <N>::process_messages();
        r
    }
    fn ext_wrapper<R>(func: impl FnOnce() -> R) -> R {
        LOCAL_EXT_POLKADOTRELAY.with(|v|
                { v.borrow_mut().execute_with(|| { func() }) })
    }
}
impl<N, Origin, Destination, Hops, Args>
    ::xcm_emulator::CheckAssertion<Origin, Destination, Hops, Args> for
    PolkadotRelay<N> where N: ::xcm_emulator::Network,
    Origin: ::xcm_emulator::Chain + Clone,
    Destination: ::xcm_emulator::Chain + Clone,
    Origin::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Origin::Runtime>> + Clone,
    Destination::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Destination::Runtime>> + Clone, Hops: Clone,
    Args: Clone {
    fn check_assertion(test:
            ::xcm_emulator::Test<Origin, Destination, Hops, Args>) {
        use ::xcm_emulator::TestExt;
        let chain_name = std::any::type_name::<PolkadotRelay<N>>();
        <PolkadotRelay<N>>::execute_with(||
                {
                    if let Some(dispatchable) =
                                test.hops_dispatchable.get(chain_name) {
                            let is = dispatchable(test.clone());
                            match is {
                                Ok(_) => (),
                                _ =>
                                    if !false {
                                            {
                                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                        is));
                                            }
                                        },
                            };
                        }
                    if let Some(assertion) = test.hops_assertion.get(chain_name)
                            {
                            assertion(test);
                        }
                });
    }
}
pub struct Penpal<N>(::xcm_emulator::PhantomData<N>);
#[automatically_derived]
impl<N: ::core::clone::Clone> ::core::clone::Clone for Penpal<N> {
    #[inline]
    fn clone(&self) -> Penpal<N> {
        Penpal(::core::clone::Clone::clone(&self.0))
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Chain for Penpal<N> {
    type Runtime = penpal_runtime::Runtime;
    type RuntimeCall = penpal_runtime::RuntimeCall;
    type RuntimeOrigin = penpal_runtime::RuntimeOrigin;
    type RuntimeEvent = penpal_runtime::RuntimeEvent;
    type System = ::xcm_emulator::SystemPallet<Self::Runtime>;
    type Network = N;
    fn account_data_of(account: ::xcm_emulator::AccountIdOf<Self::Runtime>)
        -> ::xcm_emulator::AccountData<::xcm_emulator::Balance> {
        <Self as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                ::xcm_emulator::SystemPallet::<Self::Runtime>::account(account).data.into())
    }
    fn events() -> Vec<<Self as ::xcm_emulator::Chain>::RuntimeEvent> {
        Self::System::events().iter().map(|record|
                    record.event.clone()).collect()
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Parachain for Penpal<N> {
    type XcmpMessageHandler = penpal_runtime::XcmpQueue;
    type LocationToAccountId =
        penpal_runtime::xcm_config::LocationToAccountId;
    type ParachainSystem =
        ::xcm_emulator::ParachainSystemPallet<<Self as
        ::xcm_emulator::Chain>::Runtime>;
    type ParachainInfo = penpal_runtime::ParachainInfo;
    type MessageProcessor =
        ::xcm_emulator::DefaultParaMessageProcessor<Penpal<N>,
        cumulus_primitives_core::AggregateMessageOrigin>;
    fn init() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        LOCAL_EXT_PENPAL.with(|v|
                *v.borrow_mut() = Self::build_new_ext(penpal::genesis()));
        Self::set_last_head();
        Self::new_block();
        Self::finalize_block();
    }
    fn new_block() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let mut relay_block_number = N::relay_block_number();
                    relay_block_number += 1;
                    N::set_relay_block_number(relay_block_number);
                    let mut block_number =
                        <Self as Chain>::System::block_number();
                    block_number += 1;
                    let parent_head_data =
                        ::xcm_emulator::LAST_HEAD.with(|b|
                                b.borrow_mut().get_mut(N::name()).expect("network not initialized?").get(&para_id).expect("network not initialized?").clone());
                    <Self as
                            Chain>::System::initialize(&block_number,
                        &parent_head_data.hash(), &Default::default());
                    <<Self as Parachain>::ParachainSystem as
                            Hooks<::xcm_emulator::BlockNumber>>::on_initialize(block_number);
                    let _ =
                        <Self as
                                Parachain>::ParachainSystem::set_validation_data(<Self as
                                    Chain>::RuntimeOrigin::none(),
                            N::hrmp_channel_parachain_inherent_data(para_id,
                                relay_block_number, parent_head_data));
                });
    }
    fn finalize_block() {
        use ::xcm_emulator::{
            Chain, Encode, Hooks, Network, Parachain, TestExt,
        };
        Self::ext_wrapper(||
                {
                    let block_number = <Self as Chain>::System::block_number();
                    <Self as
                            Parachain>::ParachainSystem::on_finalize(block_number);
                });
        Self::set_last_head();
    }
    fn set_last_head() {
        use ::xcm_emulator::{
            Chain, Encode, HeadData, Network, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let created_header = <Self as Chain>::System::finalize();
                    ::xcm_emulator::LAST_HEAD.with(|b|
                            b.borrow_mut().get_mut(N::name()).expect("network not initialized?").insert(para_id,
                                HeadData(created_header.encode())));
                });
    }
}
pub trait PenpalParaPallet {
    type PolkadotXcm;
    type Assets;
    type Balances;
    type ParachainSystem;
    type ParachainInfo;
}
impl<N: ::xcm_emulator::Network> PenpalParaPallet for Penpal<N> {
    type PolkadotXcm = penpal_runtime::PolkadotXcm;
    type Assets = penpal_runtime::Assets;
    type Balances = penpal_runtime::Balances;
    type ParachainSystem = penpal_runtime::ParachainSystem;
    type ParachainInfo = penpal_runtime::ParachainInfo;
}
pub const LOCAL_EXT_PENPAL:
    ::std::thread::LocalKey<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
    =
    {
        #[inline]
        fn __init()
            -> ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities> {
            ::xcm_emulator::RefCell::new(::xcm_emulator::TestExternalities::new(penpal::genesis()))
        }
        #[inline]
        unsafe fn __getit(init:
                ::std::option::Option<&mut ::std::option::Option<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>>)
            ->
                ::std::option::Option<&'static ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>> {
            #[thread_local]
            static __KEY:
                ::std::thread::local_impl::Key<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
                =
                ::std::thread::local_impl::Key::<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>::new();
            unsafe {
                __KEY.get(move ||
                        {
                            if let ::std::option::Option::Some(init) = init {
                                    if let ::std::option::Option::Some(value) = init.take() {
                                            return value;
                                        } else if true {
                                           {
                                               ::core::panicking::panic_fmt(format_args!("internal error: entered unreachable code: {0}",
                                                       format_args!("missing default value")));
                                           };
                                       }
                                }
                            __init()
                        })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub struct GLOBAL_EXT_PENPAL {
    __private_field: (),
}
#[doc(hidden)]
pub static GLOBAL_EXT_PENPAL: GLOBAL_EXT_PENPAL =
    GLOBAL_EXT_PENPAL { __private_field: () };
impl ::lazy_static::__Deref for GLOBAL_EXT_PENPAL {
    type Target =
        ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
        ::xcm_emulator::TestExternalities>>>;
    fn deref(&self)
        ->
            &::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
            ::xcm_emulator::TestExternalities>>> {
        #[inline(always)]
        fn __static_ref_initialize()
            ->
                ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            ::xcm_emulator::Mutex::new(::xcm_emulator::RefCell::new(::xcm_emulator::HashMap::new()))
        }
        #[inline(always)]
        fn __stability()
            ->
                &'static ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            static LAZY:
                ::lazy_static::lazy::Lazy<::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>>> =
                ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for GLOBAL_EXT_PENPAL {
    fn initialize(lazy: &Self) { let _ = &**lazy; }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::TestExt for Penpal<N> {
    fn build_new_ext(storage: ::xcm_emulator::Storage)
        -> ::xcm_emulator::TestExternalities {
        let mut ext = ::xcm_emulator::TestExternalities::new(storage);
        ext.execute_with(||
                {

                    #[allow(clippy :: no_effect)]
                    penpal_runtime::AuraExt::on_initialize(1);
                    ::xcm_emulator::sp_tracing::try_init_simple();
                    let mut block_number =
                        <Self as ::xcm_emulator::Chain>::System::block_number();
                    block_number = std::cmp::max(1, block_number);
                    <Self as
                            ::xcm_emulator::Chain>::System::set_block_number(block_number);
                });
        ext
    }
    fn new_ext() -> ::xcm_emulator::TestExternalities {
        Self::build_new_ext(penpal::genesis())
    }
    fn move_ext_out(id: &'static str) {
        use ::xcm_emulator::Deref;
        let local_ext = LOCAL_EXT_PENPAL.with(|v| { v.take() });
        let global_ext_guard = GLOBAL_EXT_PENPAL.lock().unwrap();
        global_ext_guard.deref().borrow_mut().insert(id.to_string(),
            local_ext);
    }
    fn move_ext_in(id: &'static str) {
        use ::xcm_emulator::Deref;
        let mut global_ext_unlocked = false;
        while !global_ext_unlocked {
            let global_ext_result = GLOBAL_EXT_PENPAL.try_lock();
            if let Ok(global_ext_guard) = global_ext_result {
                    if !global_ext_guard.deref().borrow().contains_key(id) {
                            drop(global_ext_guard);
                        } else { global_ext_unlocked = true; }
                }
        }
        let mut global_ext_guard = GLOBAL_EXT_PENPAL.lock().unwrap();
        let global_ext = global_ext_guard.deref();
        LOCAL_EXT_PENPAL.with(|v|
                { v.replace(global_ext.take().remove(id).unwrap()); });
    }
    fn reset_ext() {
        LOCAL_EXT_PENPAL.with(|v|
                *v.borrow_mut() = Self::build_new_ext(penpal::genesis()));
    }
    fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
        use ::xcm_emulator::{Chain, Get, Hooks, Network, Parachain, Encode};
        <N>::init();
        Self::new_block();
        let r =
            LOCAL_EXT_PENPAL.with(|v| v.borrow_mut().execute_with(execute));
        Self::finalize_block();
        let para_id = Self::para_id().into();
        LOCAL_EXT_PENPAL.with(|v|
                {
                    v.borrow_mut().execute_with(||
                            {
                                let mock_header =
                                    ::xcm_emulator::HeaderT::new(0, Default::default(),
                                        Default::default(), Default::default(), Default::default());
                                let collation_info =
                                    <Self as
                                            Parachain>::ParachainSystem::collect_collation_info(&mock_header);
                                let relay_block_number = <N>::relay_block_number();
                                for msg in collation_info.upward_messages.clone() {
                                    <N>::send_upward_message(para_id, msg);
                                }
                                for msg in collation_info.horizontal_messages {
                                    <N>::send_horizontal_messages(msg.recipient.into(),
                                        <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(para_id.into(),
                                                                relay_block_number, msg.data)])).into_iter());
                                }
                                type NetworkBridge<N> =
                                    <N as ::xcm_emulator::Network>::Bridge;
                                let bridge_messages =
                                    <<NetworkBridge<N> as ::xcm_emulator::Bridge>::Handler as
                                            ::xcm_emulator::BridgeMessageHandler>::get_source_outbound_messages();
                                for msg in bridge_messages {
                                    <N>::send_bridged_messages(msg);
                                }
                                <Self as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::Penpal", "integration_tests",
                                                                    "integration-tests/src/lib.rs"), 50u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                <Self as ::xcm_emulator::Chain>::System::reset_events();
                            })
                });
        <N>::process_messages();
        r
    }
    fn ext_wrapper<R>(func: impl FnOnce() -> R) -> R {
        LOCAL_EXT_PENPAL.with(|v|
                { v.borrow_mut().execute_with(|| { func() }) })
    }
}
impl<N, Origin, Destination, Hops, Args>
    ::xcm_emulator::CheckAssertion<Origin, Destination, Hops, Args> for
    Penpal<N> where N: ::xcm_emulator::Network,
    Origin: ::xcm_emulator::Chain + Clone,
    Destination: ::xcm_emulator::Chain + Clone,
    Origin::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Origin::Runtime>> + Clone,
    Destination::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Destination::Runtime>> + Clone, Hops: Clone,
    Args: Clone {
    fn check_assertion(test:
            ::xcm_emulator::Test<Origin, Destination, Hops, Args>) {
        use ::xcm_emulator::TestExt;
        let chain_name = std::any::type_name::<Penpal<N>>();
        <Penpal<N>>::execute_with(||
                {
                    if let Some(dispatchable) =
                                test.hops_dispatchable.get(chain_name) {
                            let is = dispatchable(test.clone());
                            match is {
                                Ok(_) => (),
                                _ =>
                                    if !false {
                                            {
                                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                        is));
                                            }
                                        },
                            };
                        }
                    if let Some(assertion) = test.hops_assertion.get(chain_name)
                            {
                            assertion(test);
                        }
                });
    }
}
pub struct Polimec<N>(::xcm_emulator::PhantomData<N>);
#[automatically_derived]
impl<N: ::core::clone::Clone> ::core::clone::Clone for Polimec<N> {
    #[inline]
    fn clone(&self) -> Polimec<N> {
        Polimec(::core::clone::Clone::clone(&self.0))
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Chain for Polimec<N> {
    type Runtime = polimec_parachain_runtime::Runtime;
    type RuntimeCall = polimec_parachain_runtime::RuntimeCall;
    type RuntimeOrigin = polimec_parachain_runtime::RuntimeOrigin;
    type RuntimeEvent = polimec_parachain_runtime::RuntimeEvent;
    type System = ::xcm_emulator::SystemPallet<Self::Runtime>;
    type Network = N;
    fn account_data_of(account: ::xcm_emulator::AccountIdOf<Self::Runtime>)
        -> ::xcm_emulator::AccountData<::xcm_emulator::Balance> {
        <Self as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                ::xcm_emulator::SystemPallet::<Self::Runtime>::account(account).data.into())
    }
    fn events() -> Vec<<Self as ::xcm_emulator::Chain>::RuntimeEvent> {
        Self::System::events().iter().map(|record|
                    record.event.clone()).collect()
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Parachain for Polimec<N> {
    type XcmpMessageHandler = polimec_parachain_runtime::XcmpQueue;
    type LocationToAccountId =
        polimec_parachain_runtime::xcm_config::LocationToAccountId;
    type ParachainSystem =
        ::xcm_emulator::ParachainSystemPallet<<Self as
        ::xcm_emulator::Chain>::Runtime>;
    type ParachainInfo = polimec_parachain_runtime::ParachainInfo;
    type MessageProcessor =
        ::xcm_emulator::DefaultParaMessageProcessor<Polimec<N>,
        cumulus_primitives_core::AggregateMessageOrigin>;
    fn init() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        LOCAL_EXT_POLIMEC.with(|v|
                *v.borrow_mut() = Self::build_new_ext(polimec::genesis()));
        Self::set_last_head();
        Self::new_block();
        Self::finalize_block();
    }
    fn new_block() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let mut relay_block_number = N::relay_block_number();
                    relay_block_number += 1;
                    N::set_relay_block_number(relay_block_number);
                    let mut block_number =
                        <Self as Chain>::System::block_number();
                    block_number += 1;
                    let parent_head_data =
                        ::xcm_emulator::LAST_HEAD.with(|b|
                                b.borrow_mut().get_mut(N::name()).expect("network not initialized?").get(&para_id).expect("network not initialized?").clone());
                    <Self as
                            Chain>::System::initialize(&block_number,
                        &parent_head_data.hash(), &Default::default());
                    <<Self as Parachain>::ParachainSystem as
                            Hooks<::xcm_emulator::BlockNumber>>::on_initialize(block_number);
                    let _ =
                        <Self as
                                Parachain>::ParachainSystem::set_validation_data(<Self as
                                    Chain>::RuntimeOrigin::none(),
                            N::hrmp_channel_parachain_inherent_data(para_id,
                                relay_block_number, parent_head_data));
                });
    }
    fn finalize_block() {
        use ::xcm_emulator::{
            Chain, Encode, Hooks, Network, Parachain, TestExt,
        };
        Self::ext_wrapper(||
                {
                    let block_number = <Self as Chain>::System::block_number();
                    <Self as
                            Parachain>::ParachainSystem::on_finalize(block_number);
                });
        Self::set_last_head();
    }
    fn set_last_head() {
        use ::xcm_emulator::{
            Chain, Encode, HeadData, Network, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let created_header = <Self as Chain>::System::finalize();
                    ::xcm_emulator::LAST_HEAD.with(|b|
                            b.borrow_mut().get_mut(N::name()).expect("network not initialized?").insert(para_id,
                                HeadData(created_header.encode())));
                });
    }
}
pub trait PolimecParaPallet {
    type Balances;
    type ParachainSystem;
    type PolkadotXcm;
    type LocalAssets;
    type ForeignAssets;
    type FundingPallet;
}
impl<N: ::xcm_emulator::Network> PolimecParaPallet for Polimec<N> {
    type Balances = polimec_parachain_runtime::Balances;
    type ParachainSystem = polimec_parachain_runtime::ParachainSystem;
    type PolkadotXcm = polimec_parachain_runtime::PolkadotXcm;
    type LocalAssets = polimec_parachain_runtime::LocalAssets;
    type ForeignAssets = polimec_parachain_runtime::ForeignAssets;
    type FundingPallet = polimec_parachain_runtime::PolimecFunding;
}
pub const LOCAL_EXT_POLIMEC:
    ::std::thread::LocalKey<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
    =
    {
        #[inline]
        fn __init()
            -> ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities> {
            ::xcm_emulator::RefCell::new(::xcm_emulator::TestExternalities::new(polimec::genesis()))
        }
        #[inline]
        unsafe fn __getit(init:
                ::std::option::Option<&mut ::std::option::Option<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>>)
            ->
                ::std::option::Option<&'static ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>> {
            #[thread_local]
            static __KEY:
                ::std::thread::local_impl::Key<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
                =
                ::std::thread::local_impl::Key::<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>::new();
            unsafe {
                __KEY.get(move ||
                        {
                            if let ::std::option::Option::Some(init) = init {
                                    if let ::std::option::Option::Some(value) = init.take() {
                                            return value;
                                        } else if true {
                                           {
                                               ::core::panicking::panic_fmt(format_args!("internal error: entered unreachable code: {0}",
                                                       format_args!("missing default value")));
                                           };
                                       }
                                }
                            __init()
                        })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub struct GLOBAL_EXT_POLIMEC {
    __private_field: (),
}
#[doc(hidden)]
pub static GLOBAL_EXT_POLIMEC: GLOBAL_EXT_POLIMEC =
    GLOBAL_EXT_POLIMEC { __private_field: () };
impl ::lazy_static::__Deref for GLOBAL_EXT_POLIMEC {
    type Target =
        ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
        ::xcm_emulator::TestExternalities>>>;
    fn deref(&self)
        ->
            &::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
            ::xcm_emulator::TestExternalities>>> {
        #[inline(always)]
        fn __static_ref_initialize()
            ->
                ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            ::xcm_emulator::Mutex::new(::xcm_emulator::RefCell::new(::xcm_emulator::HashMap::new()))
        }
        #[inline(always)]
        fn __stability()
            ->
                &'static ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            static LAZY:
                ::lazy_static::lazy::Lazy<::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>>> =
                ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for GLOBAL_EXT_POLIMEC {
    fn initialize(lazy: &Self) { let _ = &**lazy; }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::TestExt for Polimec<N> {
    fn build_new_ext(storage: ::xcm_emulator::Storage)
        -> ::xcm_emulator::TestExternalities {
        let mut ext = ::xcm_emulator::TestExternalities::new(storage);
        ext.execute_with(||
                {

                    #[allow(clippy :: no_effect)]
                    polimec_parachain_runtime::AuraExt::on_initialize(1);
                    ::xcm_emulator::sp_tracing::try_init_simple();
                    let mut block_number =
                        <Self as ::xcm_emulator::Chain>::System::block_number();
                    block_number = std::cmp::max(1, block_number);
                    <Self as
                            ::xcm_emulator::Chain>::System::set_block_number(block_number);
                });
        ext
    }
    fn new_ext() -> ::xcm_emulator::TestExternalities {
        Self::build_new_ext(polimec::genesis())
    }
    fn move_ext_out(id: &'static str) {
        use ::xcm_emulator::Deref;
        let local_ext = LOCAL_EXT_POLIMEC.with(|v| { v.take() });
        let global_ext_guard = GLOBAL_EXT_POLIMEC.lock().unwrap();
        global_ext_guard.deref().borrow_mut().insert(id.to_string(),
            local_ext);
    }
    fn move_ext_in(id: &'static str) {
        use ::xcm_emulator::Deref;
        let mut global_ext_unlocked = false;
        while !global_ext_unlocked {
            let global_ext_result = GLOBAL_EXT_POLIMEC.try_lock();
            if let Ok(global_ext_guard) = global_ext_result {
                    if !global_ext_guard.deref().borrow().contains_key(id) {
                            drop(global_ext_guard);
                        } else { global_ext_unlocked = true; }
                }
        }
        let mut global_ext_guard = GLOBAL_EXT_POLIMEC.lock().unwrap();
        let global_ext = global_ext_guard.deref();
        LOCAL_EXT_POLIMEC.with(|v|
                { v.replace(global_ext.take().remove(id).unwrap()); });
    }
    fn reset_ext() {
        LOCAL_EXT_POLIMEC.with(|v|
                *v.borrow_mut() = Self::build_new_ext(polimec::genesis()));
    }
    fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
        use ::xcm_emulator::{Chain, Get, Hooks, Network, Parachain, Encode};
        <N>::init();
        Self::new_block();
        let r =
            LOCAL_EXT_POLIMEC.with(|v| v.borrow_mut().execute_with(execute));
        Self::finalize_block();
        let para_id = Self::para_id().into();
        LOCAL_EXT_POLIMEC.with(|v|
                {
                    v.borrow_mut().execute_with(||
                            {
                                let mock_header =
                                    ::xcm_emulator::HeaderT::new(0, Default::default(),
                                        Default::default(), Default::default(), Default::default());
                                let collation_info =
                                    <Self as
                                            Parachain>::ParachainSystem::collect_collation_info(&mock_header);
                                let relay_block_number = <N>::relay_block_number();
                                for msg in collation_info.upward_messages.clone() {
                                    <N>::send_upward_message(para_id, msg);
                                }
                                for msg in collation_info.horizontal_messages {
                                    <N>::send_horizontal_messages(msg.recipient.into(),
                                        <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(para_id.into(),
                                                                relay_block_number, msg.data)])).into_iter());
                                }
                                type NetworkBridge<N> =
                                    <N as ::xcm_emulator::Network>::Bridge;
                                let bridge_messages =
                                    <<NetworkBridge<N> as ::xcm_emulator::Bridge>::Handler as
                                            ::xcm_emulator::BridgeMessageHandler>::get_source_outbound_messages();
                                for msg in bridge_messages {
                                    <N>::send_bridged_messages(msg);
                                }
                                <Self as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::Polimec", "integration_tests",
                                                                    "integration-tests/src/lib.rs"), 50u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                <Self as ::xcm_emulator::Chain>::System::reset_events();
                            })
                });
        <N>::process_messages();
        r
    }
    fn ext_wrapper<R>(func: impl FnOnce() -> R) -> R {
        LOCAL_EXT_POLIMEC.with(|v|
                { v.borrow_mut().execute_with(|| { func() }) })
    }
}
impl<N, Origin, Destination, Hops, Args>
    ::xcm_emulator::CheckAssertion<Origin, Destination, Hops, Args> for
    Polimec<N> where N: ::xcm_emulator::Network,
    Origin: ::xcm_emulator::Chain + Clone,
    Destination: ::xcm_emulator::Chain + Clone,
    Origin::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Origin::Runtime>> + Clone,
    Destination::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Destination::Runtime>> + Clone, Hops: Clone,
    Args: Clone {
    fn check_assertion(test:
            ::xcm_emulator::Test<Origin, Destination, Hops, Args>) {
        use ::xcm_emulator::TestExt;
        let chain_name = std::any::type_name::<Polimec<N>>();
        <Polimec<N>>::execute_with(||
                {
                    if let Some(dispatchable) =
                                test.hops_dispatchable.get(chain_name) {
                            let is = dispatchable(test.clone());
                            match is {
                                Ok(_) => (),
                                _ =>
                                    if !false {
                                            {
                                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                        is));
                                            }
                                        },
                            };
                        }
                    if let Some(assertion) = test.hops_assertion.get(chain_name)
                            {
                            assertion(test);
                        }
                });
    }
}
pub struct AssetHub<N>(::xcm_emulator::PhantomData<N>);
#[automatically_derived]
impl<N: ::core::clone::Clone> ::core::clone::Clone for AssetHub<N> {
    #[inline]
    fn clone(&self) -> AssetHub<N> {
        AssetHub(::core::clone::Clone::clone(&self.0))
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Chain for AssetHub<N> {
    type Runtime = asset_hub_polkadot_runtime::Runtime;
    type RuntimeCall = asset_hub_polkadot_runtime::RuntimeCall;
    type RuntimeOrigin = asset_hub_polkadot_runtime::RuntimeOrigin;
    type RuntimeEvent = asset_hub_polkadot_runtime::RuntimeEvent;
    type System = ::xcm_emulator::SystemPallet<Self::Runtime>;
    type Network = N;
    fn account_data_of(account: ::xcm_emulator::AccountIdOf<Self::Runtime>)
        -> ::xcm_emulator::AccountData<::xcm_emulator::Balance> {
        <Self as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                ::xcm_emulator::SystemPallet::<Self::Runtime>::account(account).data.into())
    }
    fn events() -> Vec<<Self as ::xcm_emulator::Chain>::RuntimeEvent> {
        Self::System::events().iter().map(|record|
                    record.event.clone()).collect()
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Parachain for AssetHub<N> {
    type XcmpMessageHandler = asset_hub_polkadot_runtime::XcmpQueue;
    type LocationToAccountId =
        asset_hub_polkadot_runtime::xcm_config::LocationToAccountId;
    type ParachainSystem =
        ::xcm_emulator::ParachainSystemPallet<<Self as
        ::xcm_emulator::Chain>::Runtime>;
    type ParachainInfo = asset_hub_polkadot_runtime::ParachainInfo;
    type MessageProcessor =
        ::xcm_emulator::DefaultParaMessageProcessor<AssetHub<N>,
        cumulus_primitives_core::AggregateMessageOrigin>;
    fn init() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        LOCAL_EXT_ASSETHUB.with(|v|
                *v.borrow_mut() = Self::build_new_ext(asset_hub::genesis()));
        Self::set_last_head();
        Self::new_block();
        Self::finalize_block();
    }
    fn new_block() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let mut relay_block_number = N::relay_block_number();
                    relay_block_number += 1;
                    N::set_relay_block_number(relay_block_number);
                    let mut block_number =
                        <Self as Chain>::System::block_number();
                    block_number += 1;
                    let parent_head_data =
                        ::xcm_emulator::LAST_HEAD.with(|b|
                                b.borrow_mut().get_mut(N::name()).expect("network not initialized?").get(&para_id).expect("network not initialized?").clone());
                    <Self as
                            Chain>::System::initialize(&block_number,
                        &parent_head_data.hash(), &Default::default());
                    <<Self as Parachain>::ParachainSystem as
                            Hooks<::xcm_emulator::BlockNumber>>::on_initialize(block_number);
                    let _ =
                        <Self as
                                Parachain>::ParachainSystem::set_validation_data(<Self as
                                    Chain>::RuntimeOrigin::none(),
                            N::hrmp_channel_parachain_inherent_data(para_id,
                                relay_block_number, parent_head_data));
                });
    }
    fn finalize_block() {
        use ::xcm_emulator::{
            Chain, Encode, Hooks, Network, Parachain, TestExt,
        };
        Self::ext_wrapper(||
                {
                    let block_number = <Self as Chain>::System::block_number();
                    <Self as
                            Parachain>::ParachainSystem::on_finalize(block_number);
                });
        Self::set_last_head();
    }
    fn set_last_head() {
        use ::xcm_emulator::{
            Chain, Encode, HeadData, Network, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let created_header = <Self as Chain>::System::finalize();
                    ::xcm_emulator::LAST_HEAD.with(|b|
                            b.borrow_mut().get_mut(N::name()).expect("network not initialized?").insert(para_id,
                                HeadData(created_header.encode())));
                });
    }
}
pub trait AssetHubParaPallet {
    type Balances;
    type ParachainSystem;
    type PolkadotXcm;
    type ForeignAssets;
    type LocalAssets;
}
impl<N: ::xcm_emulator::Network> AssetHubParaPallet for AssetHub<N> {
    type Balances = asset_hub_polkadot_runtime::Balances;
    type ParachainSystem = asset_hub_polkadot_runtime::ParachainSystem;
    type PolkadotXcm = asset_hub_polkadot_runtime::PolkadotXcm;
    type ForeignAssets = asset_hub_polkadot_runtime::ForeignAssets;
    type LocalAssets = asset_hub_polkadot_runtime::Assets;
}
pub const LOCAL_EXT_ASSETHUB:
    ::std::thread::LocalKey<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
    =
    {
        #[inline]
        fn __init()
            -> ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities> {
            ::xcm_emulator::RefCell::new(::xcm_emulator::TestExternalities::new(asset_hub::genesis()))
        }
        #[inline]
        unsafe fn __getit(init:
                ::std::option::Option<&mut ::std::option::Option<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>>)
            ->
                ::std::option::Option<&'static ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>> {
            #[thread_local]
            static __KEY:
                ::std::thread::local_impl::Key<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
                =
                ::std::thread::local_impl::Key::<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>::new();
            unsafe {
                __KEY.get(move ||
                        {
                            if let ::std::option::Option::Some(init) = init {
                                    if let ::std::option::Option::Some(value) = init.take() {
                                            return value;
                                        } else if true {
                                           {
                                               ::core::panicking::panic_fmt(format_args!("internal error: entered unreachable code: {0}",
                                                       format_args!("missing default value")));
                                           };
                                       }
                                }
                            __init()
                        })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub struct GLOBAL_EXT_ASSETHUB {
    __private_field: (),
}
#[doc(hidden)]
pub static GLOBAL_EXT_ASSETHUB: GLOBAL_EXT_ASSETHUB =
    GLOBAL_EXT_ASSETHUB { __private_field: () };
impl ::lazy_static::__Deref for GLOBAL_EXT_ASSETHUB {
    type Target =
        ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
        ::xcm_emulator::TestExternalities>>>;
    fn deref(&self)
        ->
            &::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
            ::xcm_emulator::TestExternalities>>> {
        #[inline(always)]
        fn __static_ref_initialize()
            ->
                ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            ::xcm_emulator::Mutex::new(::xcm_emulator::RefCell::new(::xcm_emulator::HashMap::new()))
        }
        #[inline(always)]
        fn __stability()
            ->
                &'static ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            static LAZY:
                ::lazy_static::lazy::Lazy<::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>>> =
                ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for GLOBAL_EXT_ASSETHUB {
    fn initialize(lazy: &Self) { let _ = &**lazy; }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::TestExt for AssetHub<N> {
    fn build_new_ext(storage: ::xcm_emulator::Storage)
        -> ::xcm_emulator::TestExternalities {
        let mut ext = ::xcm_emulator::TestExternalities::new(storage);
        ext.execute_with(||
                {

                    #[allow(clippy :: no_effect)]
                    asset_hub_polkadot_runtime::AuraExt::on_initialize(1);
                    ::xcm_emulator::sp_tracing::try_init_simple();
                    let mut block_number =
                        <Self as ::xcm_emulator::Chain>::System::block_number();
                    block_number = std::cmp::max(1, block_number);
                    <Self as
                            ::xcm_emulator::Chain>::System::set_block_number(block_number);
                });
        ext
    }
    fn new_ext() -> ::xcm_emulator::TestExternalities {
        Self::build_new_ext(asset_hub::genesis())
    }
    fn move_ext_out(id: &'static str) {
        use ::xcm_emulator::Deref;
        let local_ext = LOCAL_EXT_ASSETHUB.with(|v| { v.take() });
        let global_ext_guard = GLOBAL_EXT_ASSETHUB.lock().unwrap();
        global_ext_guard.deref().borrow_mut().insert(id.to_string(),
            local_ext);
    }
    fn move_ext_in(id: &'static str) {
        use ::xcm_emulator::Deref;
        let mut global_ext_unlocked = false;
        while !global_ext_unlocked {
            let global_ext_result = GLOBAL_EXT_ASSETHUB.try_lock();
            if let Ok(global_ext_guard) = global_ext_result {
                    if !global_ext_guard.deref().borrow().contains_key(id) {
                            drop(global_ext_guard);
                        } else { global_ext_unlocked = true; }
                }
        }
        let mut global_ext_guard = GLOBAL_EXT_ASSETHUB.lock().unwrap();
        let global_ext = global_ext_guard.deref();
        LOCAL_EXT_ASSETHUB.with(|v|
                { v.replace(global_ext.take().remove(id).unwrap()); });
    }
    fn reset_ext() {
        LOCAL_EXT_ASSETHUB.with(|v|
                *v.borrow_mut() = Self::build_new_ext(asset_hub::genesis()));
    }
    fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
        use ::xcm_emulator::{Chain, Get, Hooks, Network, Parachain, Encode};
        <N>::init();
        Self::new_block();
        let r =
            LOCAL_EXT_ASSETHUB.with(|v| v.borrow_mut().execute_with(execute));
        Self::finalize_block();
        let para_id = Self::para_id().into();
        LOCAL_EXT_ASSETHUB.with(|v|
                {
                    v.borrow_mut().execute_with(||
                            {
                                let mock_header =
                                    ::xcm_emulator::HeaderT::new(0, Default::default(),
                                        Default::default(), Default::default(), Default::default());
                                let collation_info =
                                    <Self as
                                            Parachain>::ParachainSystem::collect_collation_info(&mock_header);
                                let relay_block_number = <N>::relay_block_number();
                                for msg in collation_info.upward_messages.clone() {
                                    <N>::send_upward_message(para_id, msg);
                                }
                                for msg in collation_info.horizontal_messages {
                                    <N>::send_horizontal_messages(msg.recipient.into(),
                                        <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(para_id.into(),
                                                                relay_block_number, msg.data)])).into_iter());
                                }
                                type NetworkBridge<N> =
                                    <N as ::xcm_emulator::Network>::Bridge;
                                let bridge_messages =
                                    <<NetworkBridge<N> as ::xcm_emulator::Bridge>::Handler as
                                            ::xcm_emulator::BridgeMessageHandler>::get_source_outbound_messages();
                                for msg in bridge_messages {
                                    <N>::send_bridged_messages(msg);
                                }
                                <Self as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::AssetHub", "integration_tests",
                                                                    "integration-tests/src/lib.rs"), 50u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                <Self as ::xcm_emulator::Chain>::System::reset_events();
                            })
                });
        <N>::process_messages();
        r
    }
    fn ext_wrapper<R>(func: impl FnOnce() -> R) -> R {
        LOCAL_EXT_ASSETHUB.with(|v|
                { v.borrow_mut().execute_with(|| { func() }) })
    }
}
impl<N, Origin, Destination, Hops, Args>
    ::xcm_emulator::CheckAssertion<Origin, Destination, Hops, Args> for
    AssetHub<N> where N: ::xcm_emulator::Network,
    Origin: ::xcm_emulator::Chain + Clone,
    Destination: ::xcm_emulator::Chain + Clone,
    Origin::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Origin::Runtime>> + Clone,
    Destination::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Destination::Runtime>> + Clone, Hops: Clone,
    Args: Clone {
    fn check_assertion(test:
            ::xcm_emulator::Test<Origin, Destination, Hops, Args>) {
        use ::xcm_emulator::TestExt;
        let chain_name = std::any::type_name::<AssetHub<N>>();
        <AssetHub<N>>::execute_with(||
                {
                    if let Some(dispatchable) =
                                test.hops_dispatchable.get(chain_name) {
                            let is = dispatchable(test.clone());
                            match is {
                                Ok(_) => (),
                                _ =>
                                    if !false {
                                            {
                                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                        is));
                                            }
                                        },
                            };
                        }
                    if let Some(assertion) = test.hops_assertion.get(chain_name)
                            {
                            assertion(test);
                        }
                });
    }
}
pub struct PolimecBase<N>(::xcm_emulator::PhantomData<N>);
#[automatically_derived]
impl<N: ::core::clone::Clone> ::core::clone::Clone for PolimecBase<N> {
    #[inline]
    fn clone(&self) -> PolimecBase<N> {
        PolimecBase(::core::clone::Clone::clone(&self.0))
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Chain for PolimecBase<N> {
    type Runtime = polimec_base_runtime::Runtime;
    type RuntimeCall = polimec_base_runtime::RuntimeCall;
    type RuntimeOrigin = polimec_base_runtime::RuntimeOrigin;
    type RuntimeEvent = polimec_base_runtime::RuntimeEvent;
    type System = ::xcm_emulator::SystemPallet<Self::Runtime>;
    type Network = N;
    fn account_data_of(account: ::xcm_emulator::AccountIdOf<Self::Runtime>)
        -> ::xcm_emulator::AccountData<::xcm_emulator::Balance> {
        <Self as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                ::xcm_emulator::SystemPallet::<Self::Runtime>::account(account).data.into())
    }
    fn events() -> Vec<<Self as ::xcm_emulator::Chain>::RuntimeEvent> {
        Self::System::events().iter().map(|record|
                    record.event.clone()).collect()
    }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::Parachain for PolimecBase<N>
    {
    type XcmpMessageHandler = polimec_base_runtime::XcmpQueue;
    type LocationToAccountId =
        polimec_base_runtime::xcm_config::LocationToAccountId;
    type ParachainSystem =
        ::xcm_emulator::ParachainSystemPallet<<Self as
        ::xcm_emulator::Chain>::Runtime>;
    type ParachainInfo = polimec_base_runtime::ParachainInfo;
    type MessageProcessor =
        ::xcm_emulator::DefaultParaMessageProcessor<PolimecBase<N>,
        cumulus_primitives_core::AggregateMessageOrigin>;
    fn init() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        LOCAL_EXT_POLIMECBASE.with(|v|
                *v.borrow_mut() =
                    Self::build_new_ext(polimec_base::genesis()));
        Self::set_last_head();
        Self::new_block();
        Self::finalize_block();
    }
    fn new_block() {
        use ::xcm_emulator::{
            Chain, HeadData, Network, Hooks, Encode, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let mut relay_block_number = N::relay_block_number();
                    relay_block_number += 1;
                    N::set_relay_block_number(relay_block_number);
                    let mut block_number =
                        <Self as Chain>::System::block_number();
                    block_number += 1;
                    let parent_head_data =
                        ::xcm_emulator::LAST_HEAD.with(|b|
                                b.borrow_mut().get_mut(N::name()).expect("network not initialized?").get(&para_id).expect("network not initialized?").clone());
                    <Self as
                            Chain>::System::initialize(&block_number,
                        &parent_head_data.hash(), &Default::default());
                    <<Self as Parachain>::ParachainSystem as
                            Hooks<::xcm_emulator::BlockNumber>>::on_initialize(block_number);
                    let _ =
                        <Self as
                                Parachain>::ParachainSystem::set_validation_data(<Self as
                                    Chain>::RuntimeOrigin::none(),
                            N::hrmp_channel_parachain_inherent_data(para_id,
                                relay_block_number, parent_head_data));
                });
    }
    fn finalize_block() {
        use ::xcm_emulator::{
            Chain, Encode, Hooks, Network, Parachain, TestExt,
        };
        Self::ext_wrapper(||
                {
                    let block_number = <Self as Chain>::System::block_number();
                    <Self as
                            Parachain>::ParachainSystem::on_finalize(block_number);
                });
        Self::set_last_head();
    }
    fn set_last_head() {
        use ::xcm_emulator::{
            Chain, Encode, HeadData, Network, Parachain, TestExt,
        };
        let para_id = Self::para_id().into();
        Self::ext_wrapper(||
                {
                    let created_header = <Self as Chain>::System::finalize();
                    ::xcm_emulator::LAST_HEAD.with(|b|
                            b.borrow_mut().get_mut(N::name()).expect("network not initialized?").insert(para_id,
                                HeadData(created_header.encode())));
                });
    }
}
pub trait PolimecBaseParaPallet {
    type Balances;
    type ParachainSystem;
    type PolkadotXcm;
    type ForeignAssets;
}
impl<N: ::xcm_emulator::Network> PolimecBaseParaPallet for PolimecBase<N> {
    type Balances = polimec_base_runtime::Balances;
    type ParachainSystem = polimec_base_runtime::ParachainSystem;
    type PolkadotXcm = polimec_base_runtime::PolkadotXcm;
    type ForeignAssets = polimec_base_runtime::ForeignAssets;
}
pub const LOCAL_EXT_POLIMECBASE:
    ::std::thread::LocalKey<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
    =
    {
        #[inline]
        fn __init()
            -> ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities> {
            ::xcm_emulator::RefCell::new(::xcm_emulator::TestExternalities::new(polimec_base::genesis()))
        }
        #[inline]
        unsafe fn __getit(init:
                ::std::option::Option<&mut ::std::option::Option<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>>)
            ->
                ::std::option::Option<&'static ::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>> {
            #[thread_local]
            static __KEY:
                ::std::thread::local_impl::Key<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>
                =
                ::std::thread::local_impl::Key::<::xcm_emulator::RefCell<::xcm_emulator::TestExternalities>>::new();
            unsafe {
                __KEY.get(move ||
                        {
                            if let ::std::option::Option::Some(init) = init {
                                    if let ::std::option::Option::Some(value) = init.take() {
                                            return value;
                                        } else if true {
                                           {
                                               ::core::panicking::panic_fmt(format_args!("internal error: entered unreachable code: {0}",
                                                       format_args!("missing default value")));
                                           };
                                       }
                                }
                            __init()
                        })
            }
        }
        unsafe { ::std::thread::LocalKey::new(__getit) }
    };
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub struct GLOBAL_EXT_POLIMECBASE {
    __private_field: (),
}
#[doc(hidden)]
pub static GLOBAL_EXT_POLIMECBASE: GLOBAL_EXT_POLIMECBASE =
    GLOBAL_EXT_POLIMECBASE { __private_field: () };
impl ::lazy_static::__Deref for GLOBAL_EXT_POLIMECBASE {
    type Target =
        ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
        ::xcm_emulator::TestExternalities>>>;
    fn deref(&self)
        ->
            &::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
            ::xcm_emulator::TestExternalities>>> {
        #[inline(always)]
        fn __static_ref_initialize()
            ->
                ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            ::xcm_emulator::Mutex::new(::xcm_emulator::RefCell::new(::xcm_emulator::HashMap::new()))
        }
        #[inline(always)]
        fn __stability()
            ->
                &'static ::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>> {
            static LAZY:
                ::lazy_static::lazy::Lazy<::xcm_emulator::Mutex<::xcm_emulator::RefCell<::xcm_emulator::HashMap<String,
                ::xcm_emulator::TestExternalities>>>> =
                ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for GLOBAL_EXT_POLIMECBASE {
    fn initialize(lazy: &Self) { let _ = &**lazy; }
}
impl<N: ::xcm_emulator::Network> ::xcm_emulator::TestExt for PolimecBase<N> {
    fn build_new_ext(storage: ::xcm_emulator::Storage)
        -> ::xcm_emulator::TestExternalities {
        let mut ext = ::xcm_emulator::TestExternalities::new(storage);
        ext.execute_with(||
                {

                    #[allow(clippy :: no_effect)]
                    polimec_base_runtime::AuraExt::on_initialize(1);
                    ::xcm_emulator::sp_tracing::try_init_simple();
                    let mut block_number =
                        <Self as ::xcm_emulator::Chain>::System::block_number();
                    block_number = std::cmp::max(1, block_number);
                    <Self as
                            ::xcm_emulator::Chain>::System::set_block_number(block_number);
                });
        ext
    }
    fn new_ext() -> ::xcm_emulator::TestExternalities {
        Self::build_new_ext(polimec_base::genesis())
    }
    fn move_ext_out(id: &'static str) {
        use ::xcm_emulator::Deref;
        let local_ext = LOCAL_EXT_POLIMECBASE.with(|v| { v.take() });
        let global_ext_guard = GLOBAL_EXT_POLIMECBASE.lock().unwrap();
        global_ext_guard.deref().borrow_mut().insert(id.to_string(),
            local_ext);
    }
    fn move_ext_in(id: &'static str) {
        use ::xcm_emulator::Deref;
        let mut global_ext_unlocked = false;
        while !global_ext_unlocked {
            let global_ext_result = GLOBAL_EXT_POLIMECBASE.try_lock();
            if let Ok(global_ext_guard) = global_ext_result {
                    if !global_ext_guard.deref().borrow().contains_key(id) {
                            drop(global_ext_guard);
                        } else { global_ext_unlocked = true; }
                }
        }
        let mut global_ext_guard = GLOBAL_EXT_POLIMECBASE.lock().unwrap();
        let global_ext = global_ext_guard.deref();
        LOCAL_EXT_POLIMECBASE.with(|v|
                { v.replace(global_ext.take().remove(id).unwrap()); });
    }
    fn reset_ext() {
        LOCAL_EXT_POLIMECBASE.with(|v|
                *v.borrow_mut() =
                    Self::build_new_ext(polimec_base::genesis()));
    }
    fn execute_with<R>(execute: impl FnOnce() -> R) -> R {
        use ::xcm_emulator::{Chain, Get, Hooks, Network, Parachain, Encode};
        <N>::init();
        Self::new_block();
        let r =
            LOCAL_EXT_POLIMECBASE.with(|v|
                    v.borrow_mut().execute_with(execute));
        Self::finalize_block();
        let para_id = Self::para_id().into();
        LOCAL_EXT_POLIMECBASE.with(|v|
                {
                    v.borrow_mut().execute_with(||
                            {
                                let mock_header =
                                    ::xcm_emulator::HeaderT::new(0, Default::default(),
                                        Default::default(), Default::default(), Default::default());
                                let collation_info =
                                    <Self as
                                            Parachain>::ParachainSystem::collect_collation_info(&mock_header);
                                let relay_block_number = <N>::relay_block_number();
                                for msg in collation_info.upward_messages.clone() {
                                    <N>::send_upward_message(para_id, msg);
                                }
                                for msg in collation_info.horizontal_messages {
                                    <N>::send_horizontal_messages(msg.recipient.into(),
                                        <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([(para_id.into(),
                                                                relay_block_number, msg.data)])).into_iter());
                                }
                                type NetworkBridge<N> =
                                    <N as ::xcm_emulator::Network>::Bridge;
                                let bridge_messages =
                                    <<NetworkBridge<N> as ::xcm_emulator::Bridge>::Handler as
                                            ::xcm_emulator::BridgeMessageHandler>::get_source_outbound_messages();
                                for msg in bridge_messages {
                                    <N>::send_bridged_messages(msg);
                                }
                                <Self as
                                                ::xcm_emulator::Chain>::events().iter().for_each(|event|
                                        {
                                            {
                                                let lvl = ::log::Level::Debug;
                                                if lvl <= ::log::STATIC_MAX_LEVEL &&
                                                            lvl <= ::log::max_level() {
                                                        ::log::__private_api::log(format_args!("{0:?}", event), lvl,
                                                            &("events::PolimecBase", "integration_tests",
                                                                    "integration-tests/src/lib.rs"), 50u32,
                                                            ::log::__private_api::Option::None);
                                                    }
                                            };
                                        });
                                <Self as ::xcm_emulator::Chain>::System::reset_events();
                            })
                });
        <N>::process_messages();
        r
    }
    fn ext_wrapper<R>(func: impl FnOnce() -> R) -> R {
        LOCAL_EXT_POLIMECBASE.with(|v|
                { v.borrow_mut().execute_with(|| { func() }) })
    }
}
impl<N, Origin, Destination, Hops, Args>
    ::xcm_emulator::CheckAssertion<Origin, Destination, Hops, Args> for
    PolimecBase<N> where N: ::xcm_emulator::Network,
    Origin: ::xcm_emulator::Chain + Clone,
    Destination: ::xcm_emulator::Chain + Clone,
    Origin::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Origin::Runtime>> + Clone,
    Destination::RuntimeOrigin: ::xcm_emulator::OriginTrait<AccountId =
    ::xcm_emulator::AccountIdOf<Destination::Runtime>> + Clone, Hops: Clone,
    Args: Clone {
    fn check_assertion(test:
            ::xcm_emulator::Test<Origin, Destination, Hops, Args>) {
        use ::xcm_emulator::TestExt;
        let chain_name = std::any::type_name::<PolimecBase<N>>();
        <PolimecBase<N>>::execute_with(||
                {
                    if let Some(dispatchable) =
                                test.hops_dispatchable.get(chain_name) {
                            let is = dispatchable(test.clone());
                            match is {
                                Ok(_) => (),
                                _ =>
                                    if !false {
                                            {
                                                ::core::panicking::panic_fmt(format_args!("Expected Ok(_). Got {0:#?}",
                                                        is));
                                            }
                                        },
                            };
                        }
                    if let Some(assertion) = test.hops_assertion.get(chain_name)
                            {
                            assertion(test);
                        }
                });
    }
}
pub struct PolkadotNet;
#[automatically_derived]
impl ::core::clone::Clone for PolkadotNet {
    #[inline]
    fn clone(&self) -> PolkadotNet { PolkadotNet }
}
impl ::xcm_emulator::Network for PolkadotNet {
    type Relay = PolkadotRelay<Self>;
    type Bridge = ();
    fn name() -> &'static str { ::xcm_emulator::type_name::<Self>() }
    fn reset() {
        use ::xcm_emulator::TestExt;
        ::xcm_emulator::INITIALIZED.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::DOWNWARD_MESSAGES.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::DMP_DONE.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::UPWARD_MESSAGES.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::HORIZONTAL_MESSAGES.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::BRIDGED_MESSAGES.with(|b|
                b.borrow_mut().remove(Self::name()));
        ::xcm_emulator::LAST_HEAD.with(|b|
                b.borrow_mut().remove(Self::name()));
        <PolkadotRelay<Self>>::reset_ext();
        <Polimec<Self>>::reset_ext();
        <Penpal<Self>>::reset_ext();
        <AssetHub<Self>>::reset_ext();
        <PolimecBase<Self>>::reset_ext();
    }
    fn init() {
        if ::xcm_emulator::INITIALIZED.with(|b|
                        b.borrow_mut().get(Self::name()).is_none()) {
                ::xcm_emulator::INITIALIZED.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(), true));
                ::xcm_emulator::DOWNWARD_MESSAGES.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::VecDeque::new()));
                ::xcm_emulator::DMP_DONE.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::VecDeque::new()));
                ::xcm_emulator::UPWARD_MESSAGES.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::VecDeque::new()));
                ::xcm_emulator::HORIZONTAL_MESSAGES.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::VecDeque::new()));
                ::xcm_emulator::BRIDGED_MESSAGES.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::VecDeque::new()));
                ::xcm_emulator::PARA_IDS.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            Self::para_ids()));
                ::xcm_emulator::LAST_HEAD.with(|b|
                        b.borrow_mut().insert(Self::name().to_string(),
                            ::xcm_emulator::HashMap::new()));
                <PolkadotRelay<Self> as ::xcm_emulator::RelayChain>::init();
                <Polimec<Self> as ::xcm_emulator::Parachain>::init();
                <Penpal<Self> as ::xcm_emulator::Parachain>::init();
                <AssetHub<Self> as ::xcm_emulator::Parachain>::init();
                <PolimecBase<Self> as ::xcm_emulator::Parachain>::init();
            }
    }
    fn para_ids() -> Vec<u32> {
        <[_]>::into_vec(#[rustc_box] ::alloc::boxed::Box::new([<Polimec<Self>
                                    as ::xcm_emulator::Parachain>::para_id().into(),
                        <Penpal<Self> as
                                    ::xcm_emulator::Parachain>::para_id().into(),
                        <AssetHub<Self> as
                                    ::xcm_emulator::Parachain>::para_id().into(),
                        <PolimecBase<Self> as
                                    ::xcm_emulator::Parachain>::para_id().into()]))
    }
    fn relay_block_number() -> u32 {
        <Self::Relay as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                {
                    <Self::Relay as
                            ::xcm_emulator::Chain>::System::block_number()
                })
    }
    fn set_relay_block_number(number: u32) {
        <Self::Relay as
                ::xcm_emulator::TestExt>::ext_wrapper(||
                {
                    <Self::Relay as
                            ::xcm_emulator::Chain>::System::set_block_number(number);
                })
    }
    fn process_messages() {
        while Self::has_unprocessed_messages() {
            Self::process_upward_messages();
            Self::process_horizontal_messages();
            Self::process_downward_messages();
            Self::process_bridged_messages();
        }
    }
    fn has_unprocessed_messages() -> bool {
        ::xcm_emulator::DOWNWARD_MESSAGES.with(|b|
                            !b.borrow_mut().get_mut(Self::name()).unwrap().is_empty())
                    ||
                    ::xcm_emulator::HORIZONTAL_MESSAGES.with(|b|
                            !b.borrow_mut().get_mut(Self::name()).unwrap().is_empty())
                ||
                ::xcm_emulator::UPWARD_MESSAGES.with(|b|
                        !b.borrow_mut().get_mut(Self::name()).unwrap().is_empty())
            ||
            ::xcm_emulator::BRIDGED_MESSAGES.with(|b|
                    !b.borrow_mut().get_mut(Self::name()).unwrap().is_empty())
    }
    fn process_downward_messages() {
        use ::xcm_emulator::{
            DmpMessageHandler, Bounded, Parachain, RelayChainBlockNumber,
            TestExt, Encode,
        };
        while let Some((to_para_id, messages)) =
                ::xcm_emulator::DOWNWARD_MESSAGES.with(|b|
                        b.borrow_mut().get_mut(Self::name()).unwrap().pop_front()) {
            let para_id: u32 = <Polimec<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    let mut msg_dedup: Vec<(RelayChainBlockNumber, Vec<u8>)> =
                        Vec::new();
                    for m in &messages { msg_dedup.push((m.0, m.1.clone())); }
                    msg_dedup.dedup();
                    let msgs =
                        msg_dedup.clone().into_iter().filter(|m|
                                    {
                                        !::xcm_emulator::DMP_DONE.with(|b|
                                                    b.borrow().get(Self::name()).unwrap_or(&mut ::xcm_emulator::VecDeque::new()).contains(&(to_para_id,
                                                                m.0, m.1.clone())))
                                    }).collect::<Vec<(RelayChainBlockNumber, Vec<u8>)>>();
                    use ::xcm_emulator::{
                        ProcessMessage, CumulusAggregateMessageOrigin, BoundedSlice,
                        WeightMeter,
                    };
                    for (block, msg) in msgs.clone().into_iter() {
                        let mut weight_meter = WeightMeter::new();
                        <Polimec<Self>>::ext_wrapper(||
                                {
                                    let _ =
                                        <Polimec<Self> as
                                                Parachain>::MessageProcessor::process_message(&msg[..],
                                            ::xcm_emulator::CumulusAggregateMessageOrigin::Parent.into(),
                                            &mut weight_meter,
                                            &mut msg.using_encoded(::xcm_emulator::blake2_256));
                                });
                        {
                            let lvl = ::log::Level::Debug;
                            if lvl <= ::log::STATIC_MAX_LEVEL &&
                                        lvl <= ::log::max_level() {
                                    ::log::__private_api::log(format_args!("DMP messages processed {0:?} to para_id {1:?}",
                                            msgs.clone(), &to_para_id), lvl,
                                        &("dmp::PolkadotNet", "integration_tests",
                                                "integration-tests/src/lib.rs"), 126u32,
                                        ::log::__private_api::Option::None);
                                }
                        };
                        ::xcm_emulator::DMP_DONE.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().push_back((to_para_id,
                                        block, msg)));
                    }
                }
            let para_id: u32 = <Penpal<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    let mut msg_dedup: Vec<(RelayChainBlockNumber, Vec<u8>)> =
                        Vec::new();
                    for m in &messages { msg_dedup.push((m.0, m.1.clone())); }
                    msg_dedup.dedup();
                    let msgs =
                        msg_dedup.clone().into_iter().filter(|m|
                                    {
                                        !::xcm_emulator::DMP_DONE.with(|b|
                                                    b.borrow().get(Self::name()).unwrap_or(&mut ::xcm_emulator::VecDeque::new()).contains(&(to_para_id,
                                                                m.0, m.1.clone())))
                                    }).collect::<Vec<(RelayChainBlockNumber, Vec<u8>)>>();
                    use ::xcm_emulator::{
                        ProcessMessage, CumulusAggregateMessageOrigin, BoundedSlice,
                        WeightMeter,
                    };
                    for (block, msg) in msgs.clone().into_iter() {
                        let mut weight_meter = WeightMeter::new();
                        <Penpal<Self>>::ext_wrapper(||
                                {
                                    let _ =
                                        <Penpal<Self> as
                                                Parachain>::MessageProcessor::process_message(&msg[..],
                                            ::xcm_emulator::CumulusAggregateMessageOrigin::Parent.into(),
                                            &mut weight_meter,
                                            &mut msg.using_encoded(::xcm_emulator::blake2_256));
                                });
                        {
                            let lvl = ::log::Level::Debug;
                            if lvl <= ::log::STATIC_MAX_LEVEL &&
                                        lvl <= ::log::max_level() {
                                    ::log::__private_api::log(format_args!("DMP messages processed {0:?} to para_id {1:?}",
                                            msgs.clone(), &to_para_id), lvl,
                                        &("dmp::PolkadotNet", "integration_tests",
                                                "integration-tests/src/lib.rs"), 126u32,
                                        ::log::__private_api::Option::None);
                                }
                        };
                        ::xcm_emulator::DMP_DONE.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().push_back((to_para_id,
                                        block, msg)));
                    }
                }
            let para_id: u32 = <AssetHub<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    let mut msg_dedup: Vec<(RelayChainBlockNumber, Vec<u8>)> =
                        Vec::new();
                    for m in &messages { msg_dedup.push((m.0, m.1.clone())); }
                    msg_dedup.dedup();
                    let msgs =
                        msg_dedup.clone().into_iter().filter(|m|
                                    {
                                        !::xcm_emulator::DMP_DONE.with(|b|
                                                    b.borrow().get(Self::name()).unwrap_or(&mut ::xcm_emulator::VecDeque::new()).contains(&(to_para_id,
                                                                m.0, m.1.clone())))
                                    }).collect::<Vec<(RelayChainBlockNumber, Vec<u8>)>>();
                    use ::xcm_emulator::{
                        ProcessMessage, CumulusAggregateMessageOrigin, BoundedSlice,
                        WeightMeter,
                    };
                    for (block, msg) in msgs.clone().into_iter() {
                        let mut weight_meter = WeightMeter::new();
                        <AssetHub<Self>>::ext_wrapper(||
                                {
                                    let _ =
                                        <AssetHub<Self> as
                                                Parachain>::MessageProcessor::process_message(&msg[..],
                                            ::xcm_emulator::CumulusAggregateMessageOrigin::Parent.into(),
                                            &mut weight_meter,
                                            &mut msg.using_encoded(::xcm_emulator::blake2_256));
                                });
                        {
                            let lvl = ::log::Level::Debug;
                            if lvl <= ::log::STATIC_MAX_LEVEL &&
                                        lvl <= ::log::max_level() {
                                    ::log::__private_api::log(format_args!("DMP messages processed {0:?} to para_id {1:?}",
                                            msgs.clone(), &to_para_id), lvl,
                                        &("dmp::PolkadotNet", "integration_tests",
                                                "integration-tests/src/lib.rs"), 126u32,
                                        ::log::__private_api::Option::None);
                                }
                        };
                        ::xcm_emulator::DMP_DONE.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().push_back((to_para_id,
                                        block, msg)));
                    }
                }
            let para_id: u32 = <PolimecBase<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    let mut msg_dedup: Vec<(RelayChainBlockNumber, Vec<u8>)> =
                        Vec::new();
                    for m in &messages { msg_dedup.push((m.0, m.1.clone())); }
                    msg_dedup.dedup();
                    let msgs =
                        msg_dedup.clone().into_iter().filter(|m|
                                    {
                                        !::xcm_emulator::DMP_DONE.with(|b|
                                                    b.borrow().get(Self::name()).unwrap_or(&mut ::xcm_emulator::VecDeque::new()).contains(&(to_para_id,
                                                                m.0, m.1.clone())))
                                    }).collect::<Vec<(RelayChainBlockNumber, Vec<u8>)>>();
                    use ::xcm_emulator::{
                        ProcessMessage, CumulusAggregateMessageOrigin, BoundedSlice,
                        WeightMeter,
                    };
                    for (block, msg) in msgs.clone().into_iter() {
                        let mut weight_meter = WeightMeter::new();
                        <PolimecBase<Self>>::ext_wrapper(||
                                {
                                    let _ =
                                        <PolimecBase<Self> as
                                                Parachain>::MessageProcessor::process_message(&msg[..],
                                            ::xcm_emulator::CumulusAggregateMessageOrigin::Parent.into(),
                                            &mut weight_meter,
                                            &mut msg.using_encoded(::xcm_emulator::blake2_256));
                                });
                        {
                            let lvl = ::log::Level::Debug;
                            if lvl <= ::log::STATIC_MAX_LEVEL &&
                                        lvl <= ::log::max_level() {
                                    ::log::__private_api::log(format_args!("DMP messages processed {0:?} to para_id {1:?}",
                                            msgs.clone(), &to_para_id), lvl,
                                        &("dmp::PolkadotNet", "integration_tests",
                                                "integration-tests/src/lib.rs"), 126u32,
                                        ::log::__private_api::Option::None);
                                }
                        };
                        ::xcm_emulator::DMP_DONE.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().push_back((to_para_id,
                                        block, msg)));
                    }
                }
        }
    }
    fn process_horizontal_messages() {
        use ::xcm_emulator::{
            XcmpMessageHandler, ServiceQueues, Bounded, Parachain, TestExt,
        };
        while let Some((to_para_id, messages)) =
                ::xcm_emulator::HORIZONTAL_MESSAGES.with(|b|
                        b.borrow_mut().get_mut(Self::name()).unwrap().pop_front()) {
            let iter =
                messages.iter().map(|(p, b, m)|
                                (*p, *b, &m[..])).collect::<Vec<_>>().into_iter();
            let para_id: u32 = <Polimec<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    <Polimec<Self>>::ext_wrapper(||
                            {
                                <Polimec<Self> as
                                        Parachain>::XcmpMessageHandler::handle_xcmp_messages(iter.clone(),
                                    ::xcm_emulator::Weight::MAX);
                                let _ =
                                    <Polimec<Self> as
                                            Parachain>::MessageProcessor::service_queues(::xcm_emulator::Weight::MAX);
                            });
                    {
                        let lvl = ::log::Level::Debug;
                        if lvl <= ::log::STATIC_MAX_LEVEL &&
                                    lvl <= ::log::max_level() {
                                ::log::__private_api::log(format_args!("HRMP messages processed {0:?} to para_id {1:?}",
                                        &messages, &to_para_id), lvl,
                                    &("hrmp::PolkadotNet", "integration_tests",
                                            "integration-tests/src/lib.rs"), 126u32,
                                    ::log::__private_api::Option::None);
                            }
                    };
                }
            let para_id: u32 = <Penpal<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    <Penpal<Self>>::ext_wrapper(||
                            {
                                <Penpal<Self> as
                                        Parachain>::XcmpMessageHandler::handle_xcmp_messages(iter.clone(),
                                    ::xcm_emulator::Weight::MAX);
                                let _ =
                                    <Penpal<Self> as
                                            Parachain>::MessageProcessor::service_queues(::xcm_emulator::Weight::MAX);
                            });
                    {
                        let lvl = ::log::Level::Debug;
                        if lvl <= ::log::STATIC_MAX_LEVEL &&
                                    lvl <= ::log::max_level() {
                                ::log::__private_api::log(format_args!("HRMP messages processed {0:?} to para_id {1:?}",
                                        &messages, &to_para_id), lvl,
                                    &("hrmp::PolkadotNet", "integration_tests",
                                            "integration-tests/src/lib.rs"), 126u32,
                                    ::log::__private_api::Option::None);
                            }
                    };
                }
            let para_id: u32 = <AssetHub<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    <AssetHub<Self>>::ext_wrapper(||
                            {
                                <AssetHub<Self> as
                                        Parachain>::XcmpMessageHandler::handle_xcmp_messages(iter.clone(),
                                    ::xcm_emulator::Weight::MAX);
                                let _ =
                                    <AssetHub<Self> as
                                            Parachain>::MessageProcessor::service_queues(::xcm_emulator::Weight::MAX);
                            });
                    {
                        let lvl = ::log::Level::Debug;
                        if lvl <= ::log::STATIC_MAX_LEVEL &&
                                    lvl <= ::log::max_level() {
                                ::log::__private_api::log(format_args!("HRMP messages processed {0:?} to para_id {1:?}",
                                        &messages, &to_para_id), lvl,
                                    &("hrmp::PolkadotNet", "integration_tests",
                                            "integration-tests/src/lib.rs"), 126u32,
                                    ::log::__private_api::Option::None);
                            }
                    };
                }
            let para_id: u32 = <PolimecBase<Self>>::para_id().into();
            if ::xcm_emulator::PARA_IDS.with(|b|
                                b.borrow_mut().get_mut(Self::name()).unwrap().contains(&to_para_id))
                        && para_id == to_para_id {
                    <PolimecBase<Self>>::ext_wrapper(||
                            {
                                <PolimecBase<Self> as
                                        Parachain>::XcmpMessageHandler::handle_xcmp_messages(iter.clone(),
                                    ::xcm_emulator::Weight::MAX);
                                let _ =
                                    <PolimecBase<Self> as
                                            Parachain>::MessageProcessor::service_queues(::xcm_emulator::Weight::MAX);
                            });
                    {
                        let lvl = ::log::Level::Debug;
                        if lvl <= ::log::STATIC_MAX_LEVEL &&
                                    lvl <= ::log::max_level() {
                                ::log::__private_api::log(format_args!("HRMP messages processed {0:?} to para_id {1:?}",
                                        &messages, &to_para_id), lvl,
                                    &("hrmp::PolkadotNet", "integration_tests",
                                            "integration-tests/src/lib.rs"), 126u32,
                                    ::log::__private_api::Option::None);
                            }
                    };
                }
        }
    }
    fn process_upward_messages() {
        use ::xcm_emulator::{Encode, ProcessMessage, TestExt, WeightMeter};
        while let Some((from_para_id, msg)) =
                ::xcm_emulator::UPWARD_MESSAGES.with(|b|
                        b.borrow_mut().get_mut(Self::name()).unwrap().pop_front()) {
            let mut weight_meter = WeightMeter::new();
            <PolkadotRelay<Self>>::ext_wrapper(||
                    {
                        let _ =
                            <PolkadotRelay<Self> as
                                    ::xcm_emulator::RelayChain>::MessageProcessor::process_message(&msg[..],
                                from_para_id.into(), &mut weight_meter,
                                &mut msg.using_encoded(::xcm_emulator::blake2_256));
                    });
            {
                let lvl = ::log::Level::Debug;
                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level()
                        {
                        ::log::__private_api::log(format_args!("Upward message processed {0:?} from para_id {1:?}",
                                &msg, &from_para_id), lvl,
                            &("ump::PolkadotNet", "integration_tests",
                                    "integration-tests/src/lib.rs"), 126u32,
                            ::log::__private_api::Option::None);
                    }
            };
        }
    }
    fn process_bridged_messages() {
        use ::xcm_emulator::{Bridge, BridgeMessageHandler, TestExt};
        <Self::Bridge as Bridge>::init();
        while let Some(msg) =
                ::xcm_emulator::BRIDGED_MESSAGES.with(|b|
                        b.borrow_mut().get_mut(Self::name()).unwrap().pop_front()) {
            let dispatch_result =
                <<Self::Bridge as Bridge>::Target as
                        TestExt>::ext_wrapper(||
                        {
                            <<Self::Bridge as Bridge>::Handler as
                                    BridgeMessageHandler>::dispatch_target_inbound_message(msg.clone())
                        });
            match dispatch_result {
                Err(e) => {
                    ::core::panicking::panic_fmt(format_args!("Error {0:?} processing bridged message: {1:?}",
                            e, msg.clone()));
                }
                Ok(()) => {
                    <<Self::Bridge as Bridge>::Source as
                            TestExt>::ext_wrapper(||
                            {
                                <<Self::Bridge as Bridge>::Handler as
                                        BridgeMessageHandler>::notify_source_message_delivery(msg.id);
                            });
                    {
                        let lvl = ::log::Level::Debug;
                        if lvl <= ::log::STATIC_MAX_LEVEL &&
                                    lvl <= ::log::max_level() {
                                ::log::__private_api::log(format_args!("Bridged message processed {0:?}",
                                        msg.clone()), lvl,
                                    &("bridge::PolkadotNet", "integration_tests",
                                            "integration-tests/src/lib.rs"), 126u32,
                                    ::log::__private_api::Option::None);
                            }
                    };
                }
            }
        }
    }
    fn hrmp_channel_parachain_inherent_data(para_id: u32,
        relay_parent_number: u32, parent_head_data: ::xcm_emulator::HeadData)
        -> ::xcm_emulator::ParachainInherentData {
        let mut sproof = ::xcm_emulator::RelayStateSproofBuilder::default();
        sproof.para_id = para_id.into();
        let e_index =
            sproof.hrmp_egress_channel_index.get_or_insert_with(Vec::new);
        for recipient_para_id in
            ::xcm_emulator::PARA_IDS.with(|b|
                    b.borrow_mut().get_mut(Self::name()).unwrap().clone()) {
            let recipient_para_id =
                ::xcm_emulator::ParaId::from(recipient_para_id);
            if let Err(idx) = e_index.binary_search(&recipient_para_id) {
                    e_index.insert(idx, recipient_para_id);
                }
            sproof.included_para_head = parent_head_data.clone().into();
            sproof.hrmp_channels.entry(::xcm_emulator::HrmpChannelId {
                        sender: sproof.para_id,
                        recipient: recipient_para_id,
                    }).or_insert_with(||
                    ::xcm_emulator::AbridgedHrmpChannel {
                        max_capacity: 1024,
                        max_total_size: 1024 * 1024,
                        max_message_size: 1024 * 1024,
                        msg_count: 0,
                        total_size: 0,
                        mqc_head: Option::None,
                    });
        }
        let (relay_storage_root, proof) = sproof.into_state_root_and_proof();
        ::xcm_emulator::ParachainInherentData {
            validation_data: ::xcm_emulator::PersistedValidationData {
                parent_head: Default::default(),
                relay_parent_number,
                relay_parent_storage_root: relay_storage_root,
                max_pov_size: Default::default(),
            },
            relay_chain_state: proof,
            downward_messages: Default::default(),
            horizontal_messages: Default::default(),
        }
    }
}
pub type PolkadotRelayRelay = PolkadotRelay<PolkadotNet>;
pub type PolimecPara = Polimec<PolkadotNet>;
pub type PenpalPara = Penpal<PolkadotNet>;
pub type AssetHubPara = AssetHub<PolkadotNet>;
pub type PolimecBasePara = PolimecBase<PolkadotNet>;
/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
    use super::{
        AssetHub, AssetHubParaPallet, Chain, Penpal, PenpalParaPallet,
        Polimec, PolimecParaPallet, PolimecBase, PolimecBaseParaPallet,
        PolkadotRelay as Polkadot, PolkadotRelayRelayPallet, PolkadotNet,
    };
    pub type PolkaNet = Polkadot<PolkadotNet>;
    pub type PoliNet = Polimec<PolkadotNet>;
    pub type PenNet = Penpal<PolkadotNet>;
    pub type AssetNet = AssetHub<PolkadotNet>;
    pub type BaseNet = PolimecBase<PolkadotNet>;
    pub type PolimecFundingPallet =
        <Polimec<PolkadotNet> as PolimecParaPallet>::FundingPallet;
    pub type PolkadotRuntime = <PolkaNet as Chain>::Runtime;
    pub type PolimecRuntime = <PoliNet as Chain>::Runtime;
    pub type PenpalRuntime = <PenNet as Chain>::Runtime;
    pub type AssetHubRuntime = <AssetNet as Chain>::Runtime;
    pub type BaseRuntime = <BaseNet as Chain>::Runtime;
    pub type PolkadotXcmPallet =
        <PolkaNet as PolkadotRelayRelayPallet>::XcmPallet;
    pub type PolimecXcmPallet = <PoliNet as PolimecParaPallet>::PolkadotXcm;
    pub type PenpalXcmPallet = <PenNet as PenpalParaPallet>::PolkadotXcm;
    pub type AssetHubXcmPallet =
        <AssetNet as AssetHubParaPallet>::PolkadotXcm;
    pub type BaseXcmPallet = <BaseNet as PolimecBaseParaPallet>::PolkadotXcm;
    pub type PolkadotBalances =
        <PolkaNet as PolkadotRelayRelayPallet>::Balances;
    pub type PolimecBalances = <PoliNet as PolimecParaPallet>::Balances;
    pub type PenpalBalances = <PenNet as PenpalParaPallet>::Balances;
    pub type AssetHubBalances = <AssetNet as AssetHubParaPallet>::Balances;
    pub type BaseBalances = <BaseNet as PolimecBaseParaPallet>::Balances;
    pub type PolimecLocalAssets = <PoliNet as PolimecParaPallet>::LocalAssets;
    pub type PolimecForeignAssets =
        <PoliNet as PolimecParaPallet>::ForeignAssets;
    pub type PenpalAssets = <PenNet as PenpalParaPallet>::Assets;
    pub type AssetHubAssets = <AssetNet as AssetHubParaPallet>::LocalAssets;
    pub type BaseForeignAssets =
        <BaseNet as PolimecBaseParaPallet>::ForeignAssets;
    pub type PolkadotOrigin = <PolkaNet as Chain>::RuntimeOrigin;
    pub type PolimecOrigin = <PoliNet as Chain>::RuntimeOrigin;
    pub type PenpalOrigin = <PenNet as Chain>::RuntimeOrigin;
    pub type AssetHubOrigin = <AssetNet as Chain>::RuntimeOrigin;
    pub type BaseOrigin = <BaseNet as Chain>::RuntimeOrigin;
    pub type PolkadotCall = <PolkaNet as Chain>::RuntimeCall;
    pub type PolimecCall = <PoliNet as Chain>::RuntimeCall;
    pub type PenpalCall = <PenNet as Chain>::RuntimeCall;
    pub type AssetHubCall = <AssetNet as Chain>::RuntimeCall;
    pub type BaseCall = <BaseNet as Chain>::RuntimeCall;
    pub type PolkadotAccountId =
        <PolkadotRuntime as frame_system::Config>::AccountId;
    pub type PolimecAccountId =
        <PolimecRuntime as frame_system::Config>::AccountId;
    pub type PenpalAccountId =
        <PenpalRuntime as frame_system::Config>::AccountId;
    pub type AssetHubAccountId =
        <AssetHubRuntime as frame_system::Config>::AccountId;
    pub type BaseAccountId = <BaseNet as frame_system::Config>::AccountId;
    pub type PolkadotEvent = <PolkaNet as Chain>::RuntimeEvent;
    pub type PolimecEvent = <PoliNet as Chain>::RuntimeEvent;
    pub type PenpalEvent = <PenNet as Chain>::RuntimeEvent;
    pub type AssetHubEvent = <AssetNet as Chain>::RuntimeEvent;
    pub type BaseEvent = <BaseNet as Chain>::RuntimeEvent;
    pub type PolkadotSystem = <PolkaNet as Chain>::System;
    pub type PolimecSystem = <PoliNet as Chain>::System;
    pub type PenpalSystem = <PenNet as Chain>::System;
    pub type AssetHubSystem = <AssetNet as Chain>::System;
    pub type BaseSystem = <BaseNet as Chain>::System;
}
pub use shortcuts::*;
