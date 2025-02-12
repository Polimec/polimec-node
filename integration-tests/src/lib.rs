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

pub use constants::{accounts::*, polimec, polkadot};
pub use frame_support::{assert_noop, assert_ok, pallet_prelude::Weight, parameter_types, traits::Hooks};
pub use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
pub use sp_core::{sr25519, storage::Storage, Encode, Get};
pub use xcm::prelude::*;
pub use xcm_emulator::{
	assert_expected_events, bx, decl_test_networks, decl_test_parachains, decl_test_relay_chains,
	helpers::{weight_within_threshold, within_threshold},
	BridgeMessageHandler, Chain, Network, ParaId, Parachain, RelayChain, TestExt,
};

decl_test_relay_chains! {
	#[api_version(11)]
	pub struct PolkadotRelay {
			genesis = polkadot::genesis(),
			on_init = {
				rococo_runtime::System::set_block_number(1);
			},
			runtime = rococo_runtime,
			core = {
				SovereignAccountOf: rococo_runtime::xcm_config::LocationConverter,
			},
			pallets = {
				System: rococo_runtime::System,
				Balances: rococo_runtime::Balances,
				XcmPallet: rococo_runtime::XcmPallet,
			}
		}
}

decl_test_parachains! {
	pub struct Polimec {
		genesis = polimec::genesis(),
		on_init = polimec_runtime::AuraExt::on_initialize(1),
		runtime = polimec_runtime,
		core = {
			XcmpMessageHandler: polimec_runtime::XcmpQueue,
			LocationToAccountId: polimec_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: polimec_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			Balances: polimec_runtime::Balances,
			ParachainSystem: polimec_runtime::ParachainSystem,
			PolkadotXcm: polimec_runtime::PolkadotXcm,
			ForeignAssets: polimec_runtime::ForeignAssets,
			Funding: polimec_runtime::Funding,
			Dispenser: polimec_runtime::Dispenser,
			Vesting: polimec_runtime::Vesting,
		}
	}
}

decl_test_networks! {
	pub struct PolkadotNet {
		relay_chain = PolkadotRelay,
		parachains = vec![
			Polimec,
		],
		bridge = ()
	}
}

/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
	use super::{Chain, Polimec, PolimecParaPallet, PolkadotNet, PolkadotRelay as Polkadot, PolkadotRelayRelayPallet};

	pub type PolkaNet = Polkadot<PolkadotNet>;
	pub type PolimecNet = Polimec<PolkadotNet>;

	pub type PolkadotRuntime = <PolkaNet as Chain>::Runtime;
	pub type PolimecRuntime = <PolimecNet as Chain>::Runtime;

	pub type PolimecFunding = <PolimecNet as PolimecParaPallet>::Funding;
	pub type PolimecDispenser = <PolimecNet as PolimecParaPallet>::Dispenser;
	pub type PolimecVesting = <PolimecNet as PolimecParaPallet>::Vesting;

	pub type PolkadotXcmPallet = <PolkaNet as PolkadotRelayRelayPallet>::XcmPallet;
	pub type PolimecXcmPallet = <PolimecNet as PolimecParaPallet>::PolkadotXcm;

	pub type PolkadotBalances = <PolkaNet as PolkadotRelayRelayPallet>::Balances;
	pub type PolimecBalances = <PolimecNet as PolimecParaPallet>::Balances;

	pub type PolimecForeignAssets = <PolimecNet as PolimecParaPallet>::ForeignAssets;

	pub type PolkadotOrigin = <PolkaNet as Chain>::RuntimeOrigin;
	pub type PolimecOrigin = <PolimecNet as Chain>::RuntimeOrigin;

	pub type PolkadotCall = <PolkaNet as Chain>::RuntimeCall;
	pub type PolimecCall = <PolimecNet as Chain>::RuntimeCall;

	pub type PolkadotAccountId = <PolkadotRuntime as frame_system::Config>::AccountId;
	pub type PolimecAccountId = <PolimecRuntime as frame_system::Config>::AccountId;

	pub type PolkadotEvent = <PolkaNet as Chain>::RuntimeEvent;
	pub type PolimecEvent = <PolimecNet as Chain>::RuntimeEvent;

	pub type PolkadotSystem = <PolkaNet as Chain>::System;
	pub type PolimecSystem = <PolimecNet as Chain>::System;
}
pub use shortcuts::*;
