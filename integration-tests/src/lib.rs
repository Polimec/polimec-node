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

pub mod constants;
#[cfg(test)]
mod tests;

pub use constants::{accounts, penpal, polimec, polkadot, statemint};
pub use frame_support::{assert_ok, pallet_prelude::Weight, parameter_types, sp_io, sp_tracing};
pub use parachains_common::{AccountId, AuraId, Balance, BlockNumber, StatemintAuraId};
pub use sp_core::{sr25519, storage::Storage, Encode, Get};
pub use xcm::prelude::*;
pub use xcm_emulator::{
	assert_expected_events, bx, decl_test_networks, decl_test_parachains, decl_test_relay_chains,
	helpers::{weight_within_threshold, within_threshold},
	Network, ParaId, Parachain, RelayChain, TestExt,
};
pub use xcm_executor::traits::Convert;

decl_test_relay_chains! {
	pub struct PolkadotRelay {
		genesis = polkadot::genesis(),
		on_init = (),
		runtime = {
			Runtime: polkadot_runtime::Runtime,
			RuntimeOrigin: polkadot_runtime::RuntimeOrigin,
			RuntimeCall: polkadot_runtime::RuntimeCall,
			RuntimeEvent: polkadot_runtime::RuntimeEvent,
			MessageQueue: polkadot_runtime::MessageQueue,
			XcmConfig: polkadot_runtime::xcm_config::XcmConfig,
			SovereignAccountOf: polkadot_runtime::xcm_config::SovereignAccountOf,
			System: polkadot_runtime::System,
			Balances: polkadot_runtime::Balances,
		},
		pallets_extra = {
			XcmPallet: polkadot_runtime::XcmPallet,
		}
	}
}
decl_test_parachains! {
	pub struct Penpal {
		genesis = penpal::genesis(),
		on_init = (),
		runtime = {
			Runtime: penpal_runtime::Runtime,
			RuntimeOrigin: penpal_runtime::RuntimeOrigin,
			RuntimeCall: penpal_runtime::RuntimeCall,
			RuntimeEvent: penpal_runtime::RuntimeEvent,
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			DmpMessageHandler: penpal_runtime::DmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			System: penpal_runtime::System,
			Balances: penpal_runtime::Balances,
			ParachainSystem: penpal_runtime::ParachainSystem,
			ParachainInfo: penpal_runtime::ParachainInfo,
		},
		pallets_extra = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
		}
	},
	pub struct Polimec {
		genesis = polimec::genesis(),
		on_init = (),
		runtime = {
			Runtime: polimec_parachain_runtime::Runtime,
			RuntimeOrigin: polimec_parachain_runtime::RuntimeOrigin,
			RuntimeCall: polimec_parachain_runtime::RuntimeCall,
			RuntimeEvent: polimec_parachain_runtime::RuntimeEvent,
			XcmpMessageHandler: polimec_parachain_runtime::XcmpQueue,
			DmpMessageHandler: polimec_parachain_runtime::DmpQueue,
			LocationToAccountId: polimec_parachain_runtime::xcm_config::LocationToAccountId,
			System: polimec_parachain_runtime::System,
			Balances: polimec_parachain_runtime::Balances,
			ParachainSystem: polimec_parachain_runtime::ParachainSystem,
			ParachainInfo: polimec_parachain_runtime::ParachainInfo,
		},
		pallets_extra = {
			PolkadotXcm: polimec_parachain_runtime::PolkadotXcm,
			LocalAssets: polimec_parachain_runtime::LocalAssets,
			StatemintAssets: polimec_parachain_runtime::StatemintAssets,
			FundingPallet: polimec_parachain_runtime::PolimecFunding,
		}
	},
	pub struct Statemint {
		genesis = statemint::genesis(),
		on_init = (),
		runtime = {
			Runtime: statemint_runtime::Runtime,
			RuntimeOrigin: statemint_runtime::RuntimeOrigin,
			RuntimeCall: statemint_runtime::RuntimeCall,
			RuntimeEvent: statemint_runtime::RuntimeEvent,
			XcmpMessageHandler: statemint_runtime::XcmpQueue,
			DmpMessageHandler: statemint_runtime::DmpQueue,
			LocationToAccountId: statemint_runtime::xcm_config::LocationToAccountId,
			System: statemint_runtime::System,
			Balances: statemint_runtime::Balances,
			ParachainSystem: statemint_runtime::ParachainSystem,
			ParachainInfo: statemint_runtime::ParachainInfo,
		},
		pallets_extra = {
			PolkadotXcm: statemint_runtime::PolkadotXcm,
			LocalAssets: statemint_runtime::Assets,
		}
	}
}
decl_test_networks! {
	pub struct PolkadotNet {
		relay_chain = PolkadotRelay,
		parachains = vec![
			Polimec,
			Penpal,
			Statemint,
		],
	}
}

/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
	use super::{
		Parachain, Penpal, Polimec, PolimecPallet, PolkadotRelay as Polkadot, PolkadotRelayPallet as PolkadotPallet,
		RelayChain, Statemint, StatemintPallet,
	};
	use crate::PenpalPallet;

	pub type PolimecFundingPallet = <Polimec as PolimecPallet>::FundingPallet;
	pub type PolkadotRuntime = <Polkadot as RelayChain>::Runtime;
	pub type PolimecRuntime = <Polimec as Parachain>::Runtime;
	pub type PenpalRuntime = <Penpal as Parachain>::Runtime;
	pub type StatemintRuntime = <Statemint as Parachain>::Runtime;

	pub type PolkadotXcmPallet = <Polkadot as PolkadotPallet>::XcmPallet;
	pub type PolimecXcmPallet = <Polimec as PolimecPallet>::PolkadotXcm;
	pub type PenpalXcmPallet = <Penpal as PenpalPallet>::PolkadotXcm;
	pub type StatemintXcmPallet = <Statemint as StatemintPallet>::PolkadotXcm;
	//
	pub type PolkadotBalances = <Polkadot as RelayChain>::Balances;
	pub type PolimecBalances = <Polimec as Parachain>::Balances;
	pub type PenpalBalances = <Penpal as Parachain>::Balances;
	pub type StatemintBalances = <Statemint as Parachain>::Balances;
	//
	pub type PolimecLocalAssets = <Polimec as PolimecPallet>::LocalAssets;
	pub type PolimecStatemintAssets = <Polimec as PolimecPallet>::StatemintAssets;
	pub type PenpalAssets = <Penpal as PenpalPallet>::Assets;
	pub type StatemintAssets = <Statemint as StatemintPallet>::LocalAssets;

	pub type PolkadotOrigin = <Polkadot as RelayChain>::RuntimeOrigin;
	pub type PolimecOrigin = <Polimec as Parachain>::RuntimeOrigin;
	pub type PenpalOrigin = <Penpal as Parachain>::RuntimeOrigin;
	pub type StatemintOrigin = <Statemint as Parachain>::RuntimeOrigin;

	pub type PolkadotCall = <Polkadot as RelayChain>::RuntimeCall;
	pub type PolimecCall = <Polimec as Parachain>::RuntimeCall;
	pub type PenpalCall = <Penpal as Parachain>::RuntimeCall;
	pub type StatemintCall = <Statemint as Parachain>::RuntimeCall;

	pub type PolkadotAccountId = <PolkadotRuntime as frame_system::Config>::AccountId;
	pub type PolimecAccountId = <PolimecRuntime as frame_system::Config>::AccountId;
	pub type PenpalAccountId = <PenpalRuntime as frame_system::Config>::AccountId;
	pub type StatemintAccountId = <StatemintRuntime as frame_system::Config>::AccountId;

	pub type PolkadotEvent = <Polkadot as RelayChain>::RuntimeEvent;
	pub type PolimecEvent = <Polimec as Parachain>::RuntimeEvent;
	pub type PenpalEvent = <Penpal as Parachain>::RuntimeEvent;
	pub type StatemintEvent = <Statemint as Parachain>::RuntimeEvent;

	pub type PolkadotSystem = <Polkadot as RelayChain>::System;
	pub type PolimecSystem = <Polimec as Parachain>::System;
	pub type PenpalSystem = <Penpal as Parachain>::System;
	pub type StatemintSystem = <Statemint as Parachain>::System;
}
use shortcuts::*;

//
// #[cfg(test)]
// mod reserve_backed_transfers {
// 	use super::*;
// 	use frame_support::{traits::fungibles::Inspect, weights::WeightToFee};
//

//
//
//
//
//
//
// decl_test_parachains! {
// 	pub struct PolimecNet {
// 		Runtime = polimec_runtime::Runtime,
// 		RuntimeOrigin = polimec_runtime::RuntimeOrigin,
// 		XcmpMessageHandler = polimec_runtime::XcmpQueue,
// 		DmpMessageHandler = polimec_runtime::DmpQueue,
// 		new_ext = polimec_ext(polimec_id()),
// 	}
// }
//
// decl_test_parachains! {
// 	pub struct StatemintNet {
// 		Runtime = statemint_runtime::Runtime,
// 		RuntimeOrigin = statemint_runtime::RuntimeOrigin,
// 		XcmpMessageHandler = statemint_runtime::XcmpQueue,
// 		DmpMessageHandler = statemint_runtime::DmpQueue,
// 		new_ext = statemint_ext(statemint_id()),
// 	}
// }
//
// decl_test_parachains! {
// 	pub struct PenpalNet {
// 		Runtime = penpal_runtime::Runtime,
// 		RuntimeOrigin = penpal_runtime::RuntimeOrigin,
// 		XcmpMessageHandler = penpal_runtime::XcmpQueue,
// 		DmpMessageHandler = penpal_runtime::DmpQueue,
// 		new_ext = penpal_ext(penpal_id()),
// 	}
// }
//
// decl_test_networks! {
// 	pub struct Network {
// 		relay_chain = PolkadotNet,
// 		parachains = vec![
// 			(2000u32, PolimecNet),
// 			(1000u32, StatemintNet),
// 			(3000u32, PenpalNet),
// 		],
// 	}
// }
//
// Make sure the index reflects the definition order in network macro
// fn polimec_id() -> u32 {
// 	_para_ids()[0]
// }
// fn statemint_id() -> u32 {
// 	_para_ids()[1]
// }
// fn penpal_id() -> u32 {
// 	_para_ids()[2]
// }
//
// Helper functions to calculate chain accounts
// struct ParachainAccounts;
// impl ParachainAccounts {
// 	fn polimec_child_account() -> RuntimeAccountId32 {
// 		ParaId::new(polimec_id()).into_account_truncating()
// 	}
//
// 	fn polimec_sibling_account() -> RuntimeAccountId32 {
// 		SiblingId::from(polimec_id()).into_account_truncating()
// 	}
//
// 	fn statemint_child_account() -> RuntimeAccountId32 {
// 		ParaId::from(statemint_id()).into_account_truncating()
// 	}
//
// 	fn statemint_sibling_account() -> RuntimeAccountId32 {
// 		SiblingId::from(statemint_id()).into_account_truncating()
// 	}
//
// 	fn penpal_child_account() -> RuntimeAccountId32 {
// 		ParaId::from(penpal_id()).into_account_truncating()
// 	}
//
// 	fn penpal_sibling_account() -> RuntimeAccountId32 {
// 		SiblingId::from(penpal_id()).into_account_truncating()
// 	}
// }
//
// fn default_parachains_host_configuration(
// ) -> polkadot_runtime_parachains::configuration::HostConfiguration<polkadot_primitives::v4::BlockNumber> {
// 	use polkadot_primitives::v4::{MAX_CODE_SIZE, MAX_POV_SIZE};
//
// 	polkadot_runtime_parachains::configuration::HostConfiguration {
// 		minimum_validation_upgrade_delay: 5,
// 		validation_upgrade_cooldown: 10u32,
// 		validation_upgrade_delay: 10,
// 		code_retention_period: 1200,
// 		max_code_size: MAX_CODE_SIZE,
// 		max_pov_size: MAX_POV_SIZE,
// 		max_head_data_size: 32 * 1024,
// 		group_rotation_frequency: 20,
// 		chain_availability_period: 4,
// 		thread_availability_period: 4,
// 		max_upward_queue_count: 8,
// 		max_upward_queue_size: 1024 * 1024,
// 		max_downward_message_size: 1024,
// 		ump_service_total_weight: Weight::from_parts(4 * 1_000_000_000, 0),
// 		max_upward_message_size: 50 * 1024,
// 		max_upward_message_num_per_candidate: 5,
// 		hrmp_sender_deposit: 0,
// 		hrmp_recipient_deposit: 0,
// 		hrmp_channel_max_capacity: 8,
// 		hrmp_channel_max_total_size: 8 * 1024,
// 		hrmp_max_parachain_inbound_channels: 4,
// 		hrmp_max_parathread_inbound_channels: 4,
// 		hrmp_channel_max_message_size: 1024 * 1024,
// 		hrmp_max_parachain_outbound_channels: 4,
// 		hrmp_max_parathread_outbound_channels: 4,
// 		hrmp_max_message_num_per_candidate: 5,
// 		dispute_period: 6,
// 		no_show_slots: 2,
// 		n_delay_tranches: 25,
// 		needed_approvals: 2,
// 		relay_vrf_modulo_samples: 2,
// 		zeroth_delay_tranche_width: 0,
// 		..Default::default()
// 	}
// }
//
// pub fn polkadot_genesis() -> sp_runtime::Storage {
// 	let genesis_config = polkadot_runtime::GenesisConfig {
// 		system: polkadot_runtime::SystemConfig { code: polkadot_runtime::WASM_BINARY.unwrap().to_vec() },
// 		balances: polkadot_runtime::BalancesConfig {
// 			balances: vec![
// 				(ALICE, INITIAL_BALANCE),
// 				(ParachainAccounts::polimec_child_account(), INITIAL_BALANCE),
// 				(ParachainAccounts::penpal_child_account(), INITIAL_BALANCE),
// 				(ParachainAccounts::statemint_child_account(), INITIAL_BALANCE),
// 			],
// 		},
// 		xcm: pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) },
// 		..Default::default()
// 	};
//
// 	genesis_config.build_storage().unwrap()
// }
//
// pub fn polimec_ext(para_id: u32) -> sp_io::TestExternalities {
// 	use polimec_runtime::{Runtime, System};
//
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
//
// 	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
//
// 	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
// 		.unwrap();
//
// 	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
// 	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&xcm_config, &mut t).unwrap();
//
// 	pallet_balances::GenesisConfig::<Runtime> {
// 		balances: vec![
// 			(ALICE, INITIAL_BALANCE),
// 			(DISPATCHER_ACCOUNT, INITIAL_BALANCE),
// 			(ParachainAccounts::penpal_sibling_account(), INITIAL_BALANCE),
// 			(ParachainAccounts::statemint_sibling_account(), INITIAL_BALANCE),
// 		],
// 	}
// 	.assimilate_storage(&mut t)
// 	.unwrap();
//
// 	pallet_assets::GenesisConfig::<Runtime, polimec_runtime::StatemintAssetsInstance> {
// 		metadata: vec![(RELAY_ASSET_ID, "Local DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(), 12)],
// 		accounts: vec![(RELAY_ASSET_ID, ALICE, INITIAL_BALANCE)],
// 		assets: vec![(
// 			RELAY_ASSET_ID,
// 			frame_support::PalletId(*b"assetsid").into_account_truncating(),
// 			false,
// 			1_0_000_000_000,
// 		)],
// 	}
// 	.assimilate_storage(&mut t)
// 	.unwrap();
//
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }
//
// pub fn statemint_ext(para_id: u32) -> sp_io::TestExternalities {
// 	use statemint_runtime::{Runtime, System};
//
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
//
// 	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
//
// 	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
// 		.unwrap();
//
// 	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
// 	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&xcm_config, &mut t).unwrap();
//
// 	pallet_balances::GenesisConfig::<Runtime> {
// 		balances: vec![
// 			(ALICE, INITIAL_BALANCE),
// 			(ParachainAccounts::polimec_sibling_account(), INITIAL_BALANCE),
// 			(ParachainAccounts::penpal_sibling_account(), INITIAL_BALANCE),
// 		],
// 	}
// 	.assimilate_storage(&mut t)
// 	.unwrap();
//
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }
//
// pub fn penpal_ext(para_id: u32) -> sp_io::TestExternalities {
// 	use penpal_runtime::{Runtime, System};
//
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();
//
// 	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
// 	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
// 		.unwrap();
//
// 	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
// 	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&xcm_config, &mut t).unwrap();
//
// 	pallet_balances::GenesisConfig::<Runtime> {
// 		balances: vec![
// 			(ALICE, INITIAL_BALANCE),
// 			(ParachainAccounts::polimec_sibling_account(), INITIAL_BALANCE),
// 			(ParachainAccounts::statemint_sibling_account(), INITIAL_BALANCE),
// 		],
// 	}
// 	.assimilate_storage(&mut t)
// 	.unwrap();
//
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| System::set_block_number(1));
// 	ext
// }
