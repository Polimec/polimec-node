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

pub use constants::{accounts::*, asset_hub, polimec, westend};
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_parachain, impl_assert_events_helpers_for_parachain, impl_assets_helpers_for_parachain,
	impl_foreign_assets_helpers_for_parachain, impl_xcm_helpers_for_parachain,
};
pub use frame_support::{assert_noop, assert_ok, pallet_prelude::Weight, parameter_types};
pub use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
use polkadot_primitives::runtime_api::runtime_decl_for_parachain_host::ParachainHostV13;
pub use sp_core::{sr25519, storage::Storage, Encode, Get};
pub use xcm::v4::*;
pub use xcm_emulator::{
	assert_expected_events, bx, decl_test_networks, decl_test_parachains, decl_test_relay_chains,
	helpers::{weight_within_threshold, within_threshold},
	BridgeMessageHandler, Chain, Network, OnInitialize, ParaId, Parachain, RelayChain, TestExt,
};

decl_test_relay_chains! {
	#[api_version(12)]
	pub struct PolkadotRelay {
			genesis = westend::genesis(),
			on_init = {
				westend_runtime::System::set_block_number(1);
			},
			runtime = westend_runtime,
			core = {
				SovereignAccountOf: westend_runtime::xcm_config::LocationConverter,
			},
			pallets = {
				System: westend_runtime::System,
				Balances: westend_runtime::Balances,
				XcmPallet: westend_runtime::XcmPallet,
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

// AssetHubWestend Parachain declaration
decl_test_parachains! {
	pub struct AssetHubWestend {
		genesis = asset_hub::genesis(),
		on_init = {
			asset_hub_westend_runtime::AuraExt::on_initialize(1);
		},
		runtime = asset_hub_westend_runtime,
		core = {
			XcmpMessageHandler: asset_hub_westend_runtime::XcmpQueue,
			LocationToAccountId: asset_hub_westend_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: asset_hub_westend_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: asset_hub_westend_runtime::PolkadotXcm,
			Assets: asset_hub_westend_runtime::Assets,
			ForeignAssets: asset_hub_westend_runtime::ForeignAssets,
			PoolAssets: asset_hub_westend_runtime::PoolAssets,
			AssetConversion: asset_hub_westend_runtime::AssetConversion,
			Balances: asset_hub_westend_runtime::Balances,
		}
	},
}

// AssetHubWestend implementation
impl_accounts_helpers_for_parachain!(AssetHubWestend);
impl_assert_events_helpers_for_parachain!(AssetHubWestend);
impl_assets_helpers_for_parachain!(AssetHubWestend);
impl_foreign_assets_helpers_for_parachain!(AssetHubWestend, xcm::v5::Location);
impl_xcm_helpers_for_parachain!(AssetHubWestend);

decl_test_networks! {
	pub struct PolkadotNet {
		relay_chain = PolkadotRelay,
		parachains = vec![
			Polimec,
			AssetHubWestend,
		],
		bridge = ()
	}
}

/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
	use super::{
		AssetHubWestend, AssetHubWestendParaPallet, Chain, Polimec, PolimecParaPallet, PolkadotNet,
		PolkadotRelay as Polkadot, PolkadotRelayRelayPallet,
	};

	pub type PolkaNet = Polkadot<PolkadotNet>;
	pub type PolimecNet = Polimec<PolkadotNet>;
	pub type AssetHubWestendNet = AssetHubWestend<PolkadotNet>;

	pub type PolkadotRuntime = <PolkaNet as Chain>::Runtime;
	pub type PolimecRuntime = <PolimecNet as Chain>::Runtime;
	pub type AssetHubRuntime = <AssetHubWestendNet as Chain>::Runtime;

	pub type PolimecFunding = <PolimecNet as PolimecParaPallet>::Funding;
	pub type PolimecDispenser = <PolimecNet as PolimecParaPallet>::Dispenser;
	pub type PolimecVesting = <PolimecNet as PolimecParaPallet>::Vesting;

	pub type PolkadotXcmPallet = <PolkaNet as PolkadotRelayRelayPallet>::XcmPallet;
	pub type PolimecXcmPallet = <PolimecNet as PolimecParaPallet>::PolkadotXcm;
	pub type AssetHubXcmPallet = <AssetHubWestendNet as AssetHubWestendParaPallet>::PolkadotXcm;

	pub type PolkadotBalances = <PolkaNet as PolkadotRelayRelayPallet>::Balances;
	pub type PolimecBalances = <PolimecNet as PolimecParaPallet>::Balances;
	pub type AssetHubBalances = <AssetHubWestendNet as AssetHubWestendParaPallet>::Balances;

	pub type PolimecForeignAssets = <PolimecNet as PolimecParaPallet>::ForeignAssets;

	pub type PolkadotOrigin = <PolkaNet as Chain>::RuntimeOrigin;
	pub type PolimecOrigin = <PolimecNet as Chain>::RuntimeOrigin;
	pub type AssetHubOrigin = <AssetHubWestendNet as Chain>::RuntimeOrigin;

	pub type PolkadotCall = <PolkaNet as Chain>::RuntimeCall;
	pub type PolimecCall = <PolimecNet as Chain>::RuntimeCall;
	pub type AssetHubCall = <AssetHubWestendNet as Chain>::RuntimeCall;

	pub type PolkadotAccountId = <PolkadotRuntime as frame_system::Config>::AccountId;
	pub type PolimecAccountId = <PolimecRuntime as frame_system::Config>::AccountId;
	pub type AssetHubAccountId = <AssetHubRuntime as frame_system::Config>::AccountId;

	pub type PolkadotEvent = <PolkaNet as Chain>::RuntimeEvent;
	pub type PolimecEvent = <PolimecNet as Chain>::RuntimeEvent;
	pub type AssetHubEvent = <AssetHubWestendNet as Chain>::RuntimeEvent;

	pub type PolkadotSystem = <PolkaNet as Chain>::System;
	pub type PolimecSystem = <PolimecNet as Chain>::System;
	pub type AssetHubSystem = <AssetHubWestendNet as Chain>::System;

	pub type PolimecParachainSystem = <PolimecNet as PolimecParaPallet>::ParachainSystem;
}

pub use shortcuts::*;
