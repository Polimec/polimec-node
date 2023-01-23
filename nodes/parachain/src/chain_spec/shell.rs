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

use crate::chain_spec::Extensions;
use cumulus_primitives_core::ParaId;
use polimec_shell_runtime as shell_runtime;
use sc_service::ChainType;

/// Specialized `ChainSpec` for the shell parachain runtime.
pub type ShellChainSpec = sc_service::GenericChainSpec<shell_runtime::GenesisConfig, Extensions>;

pub fn get_local_shell_chain_spec() -> ShellChainSpec {
	ShellChainSpec::from_genesis(
		"Shell Local Testnet",
		"shell_local_testnet",
		ChainType::Local,
		move || shell_testnet_genesis(2105.into()),
		Vec::new(),
		None,
		Some("polimec"),
		None,
		None,
		Extensions { relay_chain: "rococo-local".into(), para_id: 2105 },
	)
}

pub fn get_live_shell_chain_spec() -> ShellChainSpec {
	ShellChainSpec::from_genesis(
		"Polimec Shell",
		"polimec-shell",
		ChainType::Live,
		move || shell_testnet_genesis(2105.into()),
		Vec::new(),
		None,
		Some("polimec"),
		None,
		None,
		Extensions { relay_chain: "polkadot".into(), para_id: 2105 },
	)
}

fn shell_testnet_genesis(parachain_id: ParaId) -> shell_runtime::GenesisConfig {
	shell_runtime::GenesisConfig {
		system: shell_runtime::SystemConfig {
			code: shell_runtime::WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
		},
		parachain_info: shell_runtime::ParachainInfoConfig { parachain_id },
		parachain_system: Default::default(),
	}
}
