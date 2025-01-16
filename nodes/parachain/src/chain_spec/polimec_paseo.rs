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

use sc_service::ChainType;

use crate::chain_spec::{
	common::{acc_from_ss58, alice, bob, charlie, dave, eve, genesis_config, GenesisConfigParams},
	get_properties, Extensions, GenericChainSpec, DEFAULT_PARA_ID,
};
use polimec_runtime::{AccountId, MinCandidateStk};

pub fn get_local_chain_spec() -> GenericChainSpec {
	let endowed_accounts = vec![
		alice(),
		bob(),
		charlie(),
		dave(),
		acc_from_ss58("5Do5UoayFvDrHroGS1YMqxTVUysSkrhNwVMzmj1foVb3vzzb"),
		acc_from_ss58("5E5E37FNZD9KVHyGgSHt8pc2kq8e3VUS5rf8GmrxCa7ySs8s"),
		acc_from_ss58("5ELLzYckeuomgTnv4Pf1aT4itxu35cn1KWNCGcftzv5N2x7o"),
	];
	let endowed_accounts =
		endowed_accounts.iter().map(|x| (x.clone(), MinCandidateStk::get() * 20)).collect::<Vec<_>>();
	let genesis_config_params = GenesisConfigParams {
		stakers: vec![alice(), bob()],
		council_members: vec![alice()],
		technical_committee_members: vec![alice()],
		oracle_members: vec![
			acc_from_ss58("5Do5UoayFvDrHroGS1YMqxTVUysSkrhNwVMzmj1foVb3vzzb"),
			acc_from_ss58("5E5E37FNZD9KVHyGgSHt8pc2kq8e3VUS5rf8GmrxCa7ySs8s"),
			acc_from_ss58("5ELLzYckeuomgTnv4Pf1aT4itxu35cn1KWNCGcftzv5N2x7o"),
		],
		endowed_accounts,
		funding_assets_owner: eve(),
		id: DEFAULT_PARA_ID,
	};

	GenericChainSpec::builder(
		polimec_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "paseo-local".into(), para_id: DEFAULT_PARA_ID.into() },
	)
	.with_name("Polimec Paseo Local")
	.with_id("polimec-paseo-local")
	.with_chain_type(ChainType::Local)
	.with_protocol_id("polimec")
	.with_properties(get_properties("PLMC", 10, 41))
	.with_genesis_config_patch(genesis_config(genesis_config_params))
	.build()
}

#[allow(unused)]
pub fn get_live_chain_spec() -> GenericChainSpec {
	let sudo_acc: AccountId =
		hex_literal::hex!["ba143e2096e073cb9cddc78e6f4969d8a02160d716a69e08214caf5339d88c42"].into();
	let col_acc_1: AccountId =
		hex_literal::hex!["342ff9c467eb02d4ef632e69dfe02d44abe2265fa7d9218aa9bd33e1d238c508"].into();
	let col_acc_2: AccountId =
		hex_literal::hex!["52599f31b46056fea6964a1abff785774a33c62e8d86cdfae256a8e722c2590f"].into();
	let col_acc_3: AccountId =
		hex_literal::hex!["76ae0ce1319c8f61850063441c106ee2d21da4ca9541d6d18a69852813753267"].into();

	let endowed_accounts = vec![sudo_acc.clone(), col_acc_1.clone(), col_acc_2.clone(), col_acc_3.clone()];
	let endowed_accounts = endowed_accounts.iter().map(|x| (x.clone(), MinCandidateStk::get() * 5)).collect::<Vec<_>>();

	let genesis_config_params = GenesisConfigParams {
		stakers: vec![col_acc_1.clone(), col_acc_2.clone(), col_acc_3.clone()],
		council_members: vec![sudo_acc.clone()],
		technical_committee_members: vec![sudo_acc.clone()],
		oracle_members: vec![col_acc_1, col_acc_2, col_acc_3],
		endowed_accounts,
		funding_assets_owner: sudo_acc,
		id: 3344u32.into(),
	};

	GenericChainSpec::builder(
		polimec_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "paseo".into(), para_id: genesis_config_params.id.into() },
	)
	.with_name("Polimec Paseo")
	.with_id("polimec-paseo")
	.with_chain_type(ChainType::Live)
	.with_properties(get_properties("PLMC", 10, 41))
	.with_genesis_config_patch(genesis_config(genesis_config_params))
	.build()
}
