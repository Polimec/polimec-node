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

mod tests;

pub use constants::{accounts::*, asset_hub, penpal, polimec, polimec_base, polkadot};
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
	#[api_version(5)]
	pub struct PolkadotRelay {
			genesis = polkadot::genesis(),
			on_init = (),
			runtime = polkadot_runtime,
			core = {
				SovereignAccountOf: polkadot_runtime::xcm_config::SovereignAccountOf,
			},
			pallets = {
				System: polkadot_runtime::System,
				Balances: polkadot_runtime::Balances,
				XcmPallet: polkadot_runtime::XcmPallet,
			}
		}
}

decl_test_parachains! {
	pub struct Penpal {
		genesis = penpal::genesis(),
		on_init = penpal_runtime::AuraExt::on_initialize(1),
		runtime = penpal_runtime,
		core = {
			XcmpMessageHandler: penpal_runtime::XcmpQueue,
			LocationToAccountId: penpal_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: penpal_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: penpal_runtime::PolkadotXcm,
			Assets: penpal_runtime::Assets,
			Balances: penpal_runtime::Balances,
			ParachainSystem: penpal_runtime::ParachainSystem,
			ParachainInfo: penpal_runtime::ParachainInfo,
		}
	},
	pub struct Polimec {
		genesis = polimec::genesis(),
		on_init = polimec_parachain_runtime::AuraExt::on_initialize(1),
		runtime = polimec_parachain_runtime,
		core = {
			XcmpMessageHandler: polimec_parachain_runtime::XcmpQueue,
			LocationToAccountId: polimec_parachain_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: polimec_parachain_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			Balances: polimec_parachain_runtime::Balances,
			ParachainSystem: polimec_parachain_runtime::ParachainSystem,
			PolkadotXcm: polimec_parachain_runtime::PolkadotXcm,
			LocalAssets: polimec_parachain_runtime::LocalAssets,
			ForeignAssets: polimec_parachain_runtime::ForeignAssets,
			FundingPallet: polimec_parachain_runtime::PolimecFunding,
		}
	},
	pub struct AssetHub {
		genesis = asset_hub::genesis(),
		on_init = asset_hub_polkadot_runtime::AuraExt::on_initialize(1),
		runtime = asset_hub_polkadot_runtime,
		core = {
			XcmpMessageHandler: asset_hub_polkadot_runtime::XcmpQueue,
			LocationToAccountId: asset_hub_polkadot_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: asset_hub_polkadot_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			Balances: asset_hub_polkadot_runtime::Balances,
			ParachainSystem: asset_hub_polkadot_runtime::ParachainSystem,
			PolkadotXcm: asset_hub_polkadot_runtime::PolkadotXcm,
			ForeignAssets: asset_hub_polkadot_runtime::ForeignAssets,
			LocalAssets: asset_hub_polkadot_runtime::Assets,
		}
	},
	pub struct PolimecBase {
		genesis = polimec_base::genesis(),
		on_init = polimec_base_runtime::AuraExt::on_initialize(1),
		runtime = polimec_base_runtime,
		core = {
			XcmpMessageHandler: polimec_base_runtime::XcmpQueue,
			LocationToAccountId: polimec_base_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: polimec_base_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			Balances: polimec_base_runtime::Balances,
			ParachainSystem: polimec_base_runtime::ParachainSystem,
			PolkadotXcm: polimec_base_runtime::PolkadotXcm,
			ForeignAssets: polimec_base_runtime::ForeignAssets,
		}
	}
}

decl_test_networks! {
	pub struct PolkadotNet {
		relay_chain = PolkadotRelay,
		parachains = vec![
			Polimec,
			Penpal,
			AssetHub,
			PolimecBase,
		],
		bridge = ()
	}
}

/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
	use super::{
		AssetHub, AssetHubParaPallet, Chain, Penpal, PenpalParaPallet, Polimec, PolimecBase, PolimecBaseParaPallet,
		PolimecParaPallet, PolkadotNet, PolkadotRelay as Polkadot, PolkadotRelayRelayPallet,
	};

	pub type PolkaNet = Polkadot<PolkadotNet>;
	pub type PoliNet = Polimec<PolkadotNet>;
	pub type PenNet = Penpal<PolkadotNet>;
	pub type AssetNet = AssetHub<PolkadotNet>;
	pub type BaseNet = PolimecBase<PolkadotNet>;

	pub type PolimecFundingPallet = <Polimec<PolkadotNet> as PolimecParaPallet>::FundingPallet;

	pub type PolkadotRuntime = <PolkaNet as Chain>::Runtime;
	pub type PolimecRuntime = <PoliNet as Chain>::Runtime;
	pub type PenpalRuntime = <PenNet as Chain>::Runtime;
	pub type AssetHubRuntime = <AssetNet as Chain>::Runtime;
	pub type BaseRuntime = <BaseNet as Chain>::Runtime;

	pub type PolkadotXcmPallet = <PolkaNet as PolkadotRelayRelayPallet>::XcmPallet;
	pub type PolimecXcmPallet = <PoliNet as PolimecParaPallet>::PolkadotXcm;
	pub type PenpalXcmPallet = <PenNet as PenpalParaPallet>::PolkadotXcm;
	pub type AssetHubXcmPallet = <AssetNet as AssetHubParaPallet>::PolkadotXcm;
	pub type BaseXcmPallet = <BaseNet as PolimecBaseParaPallet>::PolkadotXcm;

	pub type PolkadotBalances = <PolkaNet as PolkadotRelayRelayPallet>::Balances;
	pub type PolimecBalances = <PoliNet as PolimecParaPallet>::Balances;
	pub type PenpalBalances = <PenNet as PenpalParaPallet>::Balances;
	pub type AssetHubBalances = <AssetNet as AssetHubParaPallet>::Balances;
	pub type BaseBalances = <BaseNet as PolimecBaseParaPallet>::Balances;

	pub type PolimecLocalAssets = <PoliNet as PolimecParaPallet>::LocalAssets;
	pub type PolimecForeignAssets = <PoliNet as PolimecParaPallet>::ForeignAssets;
	pub type PenpalAssets = <PenNet as PenpalParaPallet>::Assets;
	pub type AssetHubAssets = <AssetNet as AssetHubParaPallet>::LocalAssets;
	pub type BaseForeignAssets = <BaseNet as PolimecBaseParaPallet>::ForeignAssets;

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

	pub type PolkadotAccountId = <PolkadotRuntime as frame_system::Config>::AccountId;
	pub type PolimecAccountId = <PolimecRuntime as frame_system::Config>::AccountId;
	pub type PenpalAccountId = <PenpalRuntime as frame_system::Config>::AccountId;
	pub type AssetHubAccountId = <AssetHubRuntime as frame_system::Config>::AccountId;
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
