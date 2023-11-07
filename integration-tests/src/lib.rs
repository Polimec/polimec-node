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
mod defaults;
#[cfg(test)]
mod tests;

pub use constants::{accounts::*, penpal, polimec, polkadot, statemint};
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
pub use shortcuts::*;
