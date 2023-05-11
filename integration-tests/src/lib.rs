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

use cumulus_pallet_xcmp_queue::Event as XcmpEvent;
use frame_support::{assert_ok, pallet_prelude::Weight, traits::GenesisBuild};
use parity_scale_codec::Encode;
use polimec_parachain_runtime as polimec_runtime;
use polkadot_parachain::primitives::{Id as ParaId, Sibling as SiblingId};
use shortcuts::*;
use sp_core::{ecdsa, ed25519, sr25519, Pair};
use sp_runtime::{traits::AccountIdConversion, AccountId32 as RuntimeAccountId32};
use xcm::{v3::prelude::*, VersionedMultiAssets, VersionedMultiLocation, VersionedXcm};
use xcm_emulator::{
	cumulus_pallet_xcmp_queue, decl_test_network, decl_test_parachain, decl_test_relay_chain,
	polkadot_primitives, TestExt,
};

// DIP Dependencies
use did::did_details::{DidDetails, DidEncryptionKey, DidVerificationKey};
use dip_provider_runtime_template::{DidIdentifier, DipProvider};
use dip_support::latest::Proof;
use kilt_support::deposit::Deposit;
use pallet_did_lookup::linkable_account::LinkableAccountId;
use runtime_common::dip::provider::{CompleteMerkleProof, DidMerkleRootGenerator};

const RELAY_ASSET_ID: u32 = 0;
const RESERVE_TRANSFER_AMOUNT: u128 = 10_0_000_000_000; //10 UNITS when 10 decimals
pub const INITIAL_BALANCE: u128 = 100_0_000_000_000;
pub const ALICE: RuntimeAccountId32 = RuntimeAccountId32::new([0u8; 32]);
pub const DISPATCHER_ACCOUNT: RuntimeAccountId32 = RuntimeAccountId32::new([90u8; 32]);

// TODO: What is a good value for this? Should we define a different limit for each test?
const MAX_XCM_WEIGHT: Weight = Weight::from_parts(100_000_000_000, 3_000_000);
const MAX_XCM_FEE: u128 = 5_000_000_000; // 0.5 UNITS when 10 decimals

decl_test_relay_chain! {
	pub struct PolkadotNet {
		Runtime = polkadot_runtime::Runtime,
		XcmConfig = polkadot_runtime::xcm_config::XcmConfig,
		new_ext = polkadot_ext(),
	}
}

decl_test_parachain! {
	pub struct PolimecNet {
		Runtime = polimec_runtime::Runtime,
		RuntimeOrigin = polimec_runtime::RuntimeOrigin,
		XcmpMessageHandler = polimec_runtime::XcmpQueue,
		DmpMessageHandler = polimec_runtime::DmpQueue,
		new_ext = polimec_ext(polimec_id()),
	}
}

decl_test_parachain! {
	pub struct StatemintNet {
		Runtime = statemint_runtime::Runtime,
		RuntimeOrigin = statemint_runtime::RuntimeOrigin,
		XcmpMessageHandler = statemint_runtime::XcmpQueue,
		DmpMessageHandler = statemint_runtime::DmpQueue,
		new_ext = statemint_ext(statemint_id()),
	}
}

decl_test_parachain! {
	pub struct PenpalNet {
		Runtime = penpal_runtime::Runtime,
		RuntimeOrigin = penpal_runtime::RuntimeOrigin,
		XcmpMessageHandler = penpal_runtime::XcmpQueue,
		DmpMessageHandler = penpal_runtime::DmpQueue,
		new_ext = penpal_ext(penpal_id()),
	}
}

decl_test_parachain! {
	pub struct ProviderParachain {
		Runtime = dip_provider_runtime_template::Runtime,
		RuntimeOrigin = dip_provider_runtime_template::RuntimeOrigin,
		XcmpMessageHandler = dip_provider_runtime_template::XcmpQueue,
		DmpMessageHandler = dip_provider_runtime_template::DmpQueue,
		new_ext = provider_ext(provider_id()),
	}
}

decl_test_network! {
	pub struct Network {
		relay_chain = PolkadotNet,
		parachains = vec![
			(2000u32, PolimecNet),
			(1000u32, StatemintNet),
			(3000u32, PenpalNet),
			(2001u32, ProviderParachain),
		],
	}
}

// Make sure the index reflects the definition order in network macro
fn polimec_id() -> u32 {
	_para_ids()[0]
}
fn statemint_id() -> u32 {
	_para_ids()[1]
}
fn penpal_id() -> u32 {
	_para_ids()[2]
}
fn provider_id() -> u32 {
	_para_ids()[3]
}

// Helper functions to calculate chain accounts
struct ParachainAccounts;
impl ParachainAccounts {
	fn polimec_child_account() -> RuntimeAccountId32 {
		ParaId::new(polimec_id()).into_account_truncating()
	}
	fn polimec_sibling_account() -> RuntimeAccountId32 {
		SiblingId::from(polimec_id()).into_account_truncating()
	}

	fn statemint_child_account() -> RuntimeAccountId32 {
		ParaId::from(statemint_id()).into_account_truncating()
	}
	fn statemint_sibling_account() -> RuntimeAccountId32 {
		SiblingId::from(statemint_id()).into_account_truncating()
	}

	fn penpal_child_account() -> RuntimeAccountId32 {
		ParaId::from(penpal_id()).into_account_truncating()
	}
	fn penpal_sibling_account() -> RuntimeAccountId32 {
		SiblingId::from(penpal_id()).into_account_truncating()
	}
	fn provider_sibling_account() -> RuntimeAccountId32 {
		SiblingId::from(provider_id()).into_account_truncating()
	}
}

fn default_parachains_host_configuration(
) -> polkadot_runtime_parachains::configuration::HostConfiguration<
	polkadot_primitives::v2::BlockNumber,
> {
	use polkadot_primitives::v2::{MAX_CODE_SIZE, MAX_POV_SIZE};

	polkadot_runtime_parachains::configuration::HostConfiguration {
		minimum_validation_upgrade_delay: 5,
		validation_upgrade_cooldown: 10u32,
		validation_upgrade_delay: 10,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		group_rotation_frequency: 20,
		chain_availability_period: 4,
		thread_availability_period: 4,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024,
		ump_service_total_weight: Weight::from_ref_time(4 * 1_000_000_000),
		max_upward_message_size: 50 * 1024,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_parachain_inbound_channels: 4,
		hrmp_max_parathread_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_parachain_outbound_channels: 4,
		hrmp_max_parathread_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		..Default::default()
	}
}

pub fn polkadot_ext() -> sp_io::TestExternalities {
	use polkadot_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(ParachainAccounts::polimec_child_account(), INITIAL_BALANCE),
			(ParachainAccounts::penpal_child_account(), INITIAL_BALANCE),
			(ParachainAccounts::statemint_child_account(), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
		config: default_parachains_host_configuration(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&xcm_config,
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn polimec_ext(para_id: u32) -> sp_io::TestExternalities {
	use polimec_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&xcm_config,
		&mut t,
	)
	.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(DISPATCHER_ACCOUNT, INITIAL_BALANCE),
			(ParachainAccounts::penpal_sibling_account(), INITIAL_BALANCE),
			(ParachainAccounts::statemint_sibling_account(), INITIAL_BALANCE),
			(ParachainAccounts::provider_sibling_account(), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_assets::GenesisConfig::<Runtime, polimec_runtime::StatemintAssetsInstance> {
		assets: vec![(
			RELAY_ASSET_ID,
			polimec_runtime::AssetsPalletId::get().into_account_truncating(),
			false,
			1_0_000_000_000,
		)],
		metadata: vec![(
			RELAY_ASSET_ID,
			"Local DOT".as_bytes().to_vec(),
			"DOT".as_bytes().to_vec(),
			12,
		)],
		accounts: vec![(RELAY_ASSET_ID, ALICE, INITIAL_BALANCE)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn statemint_ext(para_id: u32) -> sp_io::TestExternalities {
	use statemint_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&xcm_config,
		&mut t,
	)
	.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(ParachainAccounts::polimec_sibling_account(), INITIAL_BALANCE),
			(ParachainAccounts::penpal_sibling_account(), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn penpal_ext(para_id: u32) -> sp_io::TestExternalities {
	use penpal_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	let xcm_config = pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) };
	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&xcm_config,
		&mut t,
	)
	.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(ALICE, INITIAL_BALANCE),
			(ParachainAccounts::polimec_sibling_account(), INITIAL_BALANCE),
			(ParachainAccounts::statemint_sibling_account(), INITIAL_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn provider_ext(para_id: u32) -> sp_io::TestExternalities {
	use dip_provider_runtime_template::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	let parachain_info_config = parachain_info::GenesisConfig { parachain_id: para_id.into() };
	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(
		&parachain_info_config,
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	let did: DidIdentifier = did_auth_key().public().into();
	let details = generate_did_details();
	ext.execute_with(|| {
		did::pallet::Did::<Runtime>::insert(&did, details);
		System::set_block_number(1);
	});
	ext
}

pub(crate) fn did_auth_key() -> ed25519::Pair {
	ed25519::Pair::from_seed(&[200u8; 32])
}

fn generate_did_details() -> DidDetails<dip_provider_runtime_template::Runtime> {
	let auth_key: DidVerificationKey = did_auth_key().public().into();
	let att_key: DidVerificationKey = sr25519::Pair::from_seed(&[100u8; 32]).public().into();
	let del_key: DidVerificationKey = ecdsa::Pair::from_seed(&[101u8; 32]).public().into();

	let mut details = DidDetails::new(
		auth_key,
		0u32,
		Deposit {
			amount: 1u64.into(),
			owner: dip_provider_runtime_template::AccountId::new([1u8; 32]),
		},
	)
	.unwrap();
	details.update_attestation_key(att_key, 0u32).unwrap();
	details.update_delegation_key(del_key, 0u32).unwrap();
	details
		.add_key_agreement_key(DidEncryptionKey::X25519([100u8; 32]), 0u32)
		.unwrap();
	details
}

/// Shortcuts to reduce boilerplate on runtime types
pub mod shortcuts {
	use super::*;

	pub type PolkadotRuntime = polkadot_runtime::Runtime;
	pub type PolimecRuntime = polimec_runtime::Runtime;
	pub type StatemintRuntime = statemint_runtime::Runtime;
	pub type PenpalRuntime = penpal_runtime::Runtime;
	pub type ProviderRuntime = dip_provider_runtime_template::Runtime;

	pub type PolkadotXcmPallet = polkadot_runtime::XcmPallet;
	pub type PolimecXcmPallet = polimec_runtime::PolkadotXcm;
	pub type StatemintXcmPallet = statemint_runtime::PolkadotXcm;
	pub type PenpalXcmPallet = penpal_runtime::PolkadotXcm;

	pub type PolkadotBalances = pallet_balances::Pallet<PolkadotRuntime>;
	pub type PolimecBalances = pallet_balances::Pallet<PolimecRuntime>;
	pub type StatemintBalances = pallet_balances::Pallet<StatemintRuntime>;
	pub type PenpalBalances = pallet_balances::Pallet<PenpalRuntime>;

	pub type PolkadotAssets = pallet_assets::Pallet<PolkadotRuntime>;
	pub type PolimecAssets =
		pallet_assets::Pallet<PolimecRuntime, polimec_runtime::StatemintAssetsInstance>;
	pub type StatemintAssets = pallet_assets::Pallet<StatemintRuntime>;
	pub type PenpalAssets = pallet_assets::Pallet<PenpalRuntime>;

	pub type PolkadotOrigin = polkadot_runtime::RuntimeOrigin;
	pub type PolimecOrigin = polimec_runtime::RuntimeOrigin;
	pub type StatemintOrigin = statemint_runtime::RuntimeOrigin;
	pub type PenpalOrigin = penpal_runtime::RuntimeOrigin;

	pub type PolkadotCall = polkadot_runtime::RuntimeCall;
	pub type PolimecCall = polimec_runtime::RuntimeCall;
	pub type StatemintCall = statemint_runtime::RuntimeCall;
	pub type PenpalCall = penpal_runtime::RuntimeCall;

	pub type PolkadotAccountId = polkadot_primitives::AccountId;
	pub type PolimecAccountId = polkadot_primitives::AccountId;
	pub type StatemintAccountId = polkadot_primitives::AccountId;
	pub type PenpalAccountId = polkadot_primitives::AccountId;
	pub type ProviderAccountId = dip_provider_runtime_template::AccountId;
}

#[cfg(test)]
mod network_tests {
	use super::*;

	#[test]
	fn dmp() {
		Network::reset();

		let remark = PolimecCall::System(frame_system::Call::<PolimecRuntime>::remark_with_event {
			remark: "Hello from Polkadot!".as_bytes().to_vec(),
		});

		PolkadotNet::execute_with(|| {
			assert_ok!(PolkadotXcmPallet::send_xcm(
				Here,
				Parachain(polimec_id()),
				Xcm(vec![
					UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(
							INITIAL_BALANCE as u64,
							1024 * 1024
						),
						call: remark.encode().into(),
					}
				]),
			));
		});

		PolimecNet::execute_with(|| {
			use polimec_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
			)));
		});
	}

	#[test]
	fn ump() {
		Network::reset();

		let burn_transfer =
			PolkadotCall::Balances(pallet_balances::Call::<PolkadotRuntime>::transfer {
				dest: PolkadotAccountId::from([0u8; 32]).into(),
				value: 1_000,
			});

		let here_asset: MultiAsset = (MultiLocation::here(), INITIAL_BALANCE / 2).into();

		PolimecNet::execute_with(|| {
			assert_ok!(PolimecXcmPallet::force_default_xcm_version(PolimecOrigin::root(), Some(3)));

			assert_ok!(PolimecXcmPallet::send_xcm(
				Here,
				Parent,
				Xcm(vec![
					WithdrawAsset(vec![here_asset.clone()].into()),
					BuyExecution { fees: here_asset.clone(), weight_limit: Unlimited },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(
							INITIAL_BALANCE as u64,
							1024 * 1024
						),
						call: burn_transfer.encode().into(),
					}
				]),
			));
		});

		PolkadotNet::execute_with(|| {
			use polkadot_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Ump(polkadot_runtime_parachains::ump::Event::ExecutedUpward(
					_,
					Outcome::Complete(_),
				))
			)));
		});
	}

	#[test]
	fn xcmp() {
		Network::reset();

		let burn_transfer =
			PolimecCall::Balances(pallet_balances::Call::<PolimecRuntime>::transfer {
				dest: PolimecAccountId::from([0u8; 32]).into(),
				value: 1_000,
			});

		let here_asset: MultiAsset = (MultiLocation::here(), INITIAL_BALANCE / 2).into();

		PenpalNet::execute_with(|| {
			assert_ok!(PenpalXcmPallet::send_xcm(
				Here,
				MultiLocation::new(1, X1(Parachain(polimec_id()))),
				Xcm(vec![
					WithdrawAsset(vec![here_asset.clone()].into()),
					BuyExecution { fees: here_asset.clone(), weight_limit: Unlimited },
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(
							INITIAL_BALANCE as u64,
							1024 * 1024
						),
						call: burn_transfer.encode().into(),
					}
				]),
			));
		});

		PolimecNet::execute_with(|| {
			use polimec_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});
	}
}

#[cfg(test)]
mod reserve_backed_transfers {
	use super::*;
	use frame_support::{traits::fungibles::Inspect, weights::WeightToFee};

	#[test]
	fn reserve_to_polimec() {
		Network::reset();
		// asset to transfer
		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_prev_alice_dot_balance,
			polimec_prev_alice_plmc_balance,
			polimec_prev_dot_issuance,
			polimec_prev_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (
			statemint_prev_alice_dot_balance,
			statemint_prev_polimec_dot_balance,
			statemint_prev_dot_issuance,
		) = StatemintNet::execute_with(|| {
			(
				StatemintBalances::free_balance(ALICE),
				StatemintBalances::free_balance(ParachainAccounts::polimec_child_account()),
				StatemintBalances::total_issuance(),
			)
		});

		// do the transfer
		StatemintNet::execute_with(|| {
			assert_ok!(StatemintXcmPallet::limited_reserve_transfer_assets(
				StatemintOrigin::signed(ALICE),
				Box::new(VersionedMultiLocation::V3(MultiLocation::new(
					1,
					X1(Parachain(polimec_id())),
				))),
				Box::new(VersionedMultiLocation::V3(MultiLocation::from(AccountId32 {
					network: None,
					id: ALICE.into(),
				}))),
				Box::new(VersionedMultiAssets::V3(vec![dot.clone()].into())),
				0,
				Limited(MAX_XCM_WEIGHT),
			));
		});

		// check the transfer was not blocked by our our xcm config
		PolimecNet::execute_with(|| {
			use polimec_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Polimec's post transfer balances and issuance
		let (
			polimec_post_alice_dot_balance,
			polimec_post_alice_plmc_balance,
			polimec_post_dot_issuance,
			polimec_post_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			polimec_runtime::System::reset_events();
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's post transfer balances and issuance
		let (
			statemint_post_alice_dot_balance,
			statemint_post_polimec_dot_balance,
			statemint_post_dot_issuance,
		) = StatemintNet::execute_with(|| {
			statemint_runtime::System::reset_events();
			(
				StatemintBalances::free_balance(ALICE),
				StatemintBalances::free_balance(ParachainAccounts::polimec_child_account()),
				StatemintBalances::total_issuance(),
			)
		});

		let polimec_delta_alice_dot_balance =
			polimec_post_alice_dot_balance - polimec_prev_alice_dot_balance;
		let polimec_delta_dot_issuance = polimec_post_dot_issuance - polimec_prev_dot_issuance;
		let polimec_delta_alice_plmc_balance =
			polimec_post_alice_plmc_balance - polimec_prev_alice_plmc_balance;
		let polimec_delta_plmc_issuance = polimec_post_plmc_issuance - polimec_prev_plmc_issuance;

		let statemint_delta_alice_dot_balance =
			statemint_prev_alice_dot_balance - statemint_post_alice_dot_balance;
		let statemint_delta_polimec_dot_balance =
			statemint_post_polimec_dot_balance - statemint_prev_polimec_dot_balance;
		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;

		assert!(
			polimec_delta_alice_dot_balance >= RESERVE_TRANSFER_AMOUNT - polimec_runtime::WeightToFee::<PolimecRuntime>::weight_to_fee(&MAX_XCM_WEIGHT) &&
			polimec_delta_alice_dot_balance <= RESERVE_TRANSFER_AMOUNT,
			"Polimec ALICE DOT balance should have increased by at least the transfer amount minus the XCM execution fee"
		);

		assert!(
			polimec_delta_dot_issuance >= RESERVE_TRANSFER_AMOUNT - polimec_runtime::WeightToFee::<PolimecRuntime>::weight_to_fee(&MAX_XCM_WEIGHT) &&
			polimec_delta_dot_issuance <= RESERVE_TRANSFER_AMOUNT,
			"Polimec DOT issuance should have increased by at least the transfer amount minus the XCM execution fee"
		);

		assert_eq!(
			statemint_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Statemint ALICE DOT balance should have decreased by the transfer amount"
		);

		assert!(
			statemint_delta_polimec_dot_balance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Polimec's sovereign account on Statemint's balance of DOT should not change, (except for fees which are burnt)"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);

		assert_eq!(
			polimec_delta_alice_plmc_balance, 0,
			"Polimec ALICE PLMC balance should not have changed"
		);

		assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
	}

	#[test]
	fn polimec_to_reserve() {
		Network::reset();
		// asset to transfer
		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();
		// execution fee
		let execution_dot: MultiAsset = (MultiLocation::parent(), MAX_XCM_FEE).into();

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_prev_alice_dot_balance,
			polimec_prev_alice_plmc_balance,
			polimec_prev_dot_issuance,
			polimec_prev_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (statemint_prev_alice_dot_balance, statemint_prev_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// construct the XCM to transfer from Polimec to Statemint's reserve
		let transfer_xcm: Xcm<PolimecCall> = Xcm(vec![
			WithdrawAsset(vec![dot.clone()].into()),
			BuyExecution { fees: execution_dot.clone(), weight_limit: Limited(MAX_XCM_WEIGHT) },
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: MultiLocation::new(1, X1(Parachain(statemint_id()))),
				xcm: Xcm(vec![
					BuyExecution {
						fees: execution_dot.clone(),
						weight_limit: Limited(MAX_XCM_WEIGHT),
					},
					DepositAsset {
						assets: All.into(),
						beneficiary: MultiLocation::new(
							0,
							AccountId32 { network: None, id: ALICE.into() },
						),
					},
				]),
			},
		]);

		// do the transfer
		PolimecNet::execute_with(|| {
			assert_ok!(PolimecXcmPallet::execute(
				PolimecOrigin::signed(ALICE),
				Box::new(VersionedXcm::V3(transfer_xcm)),
				MAX_XCM_WEIGHT,
			));
		});

		// check that the xcm was not blocked
		StatemintNet::execute_with(|| {
			use statemint_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Polimec's post transfer balances and issuance
		let (
			polimec_post_alice_dot_balance,
			polimec_post_alice_plmc_balance,
			polimec_post_dot_issuance,
			polimec_post_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's post transfer balances and issuance
		let (statemint_post_alice_dot_balance, statemint_post_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		let polimec_delta_dot_issuance = polimec_prev_dot_issuance - polimec_post_dot_issuance;
		let polimec_delta_plmc_issuance = polimec_prev_plmc_issuance - polimec_post_plmc_issuance;
		let polimec_delta_alice_dot_balance =
			polimec_prev_alice_dot_balance - polimec_post_alice_dot_balance;
		let polimec_delta_alice_plmc_balance =
			polimec_prev_alice_plmc_balance - polimec_post_alice_plmc_balance;

		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;
		let statemint_delta_alice_dot_balance =
			statemint_post_alice_dot_balance - statemint_prev_alice_dot_balance;

		assert_eq!(
			polimec_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Polimec's ALICE DOT balance should decrease by the transfer amount"
		);

		assert_eq!(
			polimec_delta_dot_issuance, RESERVE_TRANSFER_AMOUNT,
			"Polimec's DOT issuance should decrease by transfer amount due to burn"
		);

		assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec's PLMC issuance should not change, since all xcm token transfer are done in DOT, and no fees are burnt since no extrinsics are dispatched");
		assert_eq!(polimec_delta_alice_plmc_balance, 0, "Polimec's Alice PLMC should not change");

		assert!(
			statemint_delta_alice_dot_balance  >= RESERVE_TRANSFER_AMOUNT - statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT) &&
				statemint_delta_alice_dot_balance <= RESERVE_TRANSFER_AMOUNT,
			"Statemint's ALICE DOT balance should increase by at least the transfer amount minus the max allowed fees"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);
	}

	#[test]
	fn reserve_to_penpal() {
		Network::reset();
		// asset to transfer
		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();

		// check Penpal's pre transfer balances and issuance
		let (penpal_prev_alice_dot_balance, penpal_prev_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// check Statemint's pre transfer balances and issuance
		let (
			statemint_prev_alice_dot_balance,
			statemint_prev_penpal_dot_balance,
			statemint_prev_dot_issuance,
		) = StatemintNet::execute_with(|| {
			(
				StatemintBalances::free_balance(ALICE),
				StatemintBalances::free_balance(ParachainAccounts::penpal_child_account()),
				StatemintBalances::total_issuance(),
			)
		});

		// do the transfer
		StatemintNet::execute_with(|| {
			assert_ok!(StatemintXcmPallet::limited_reserve_transfer_assets(
				StatemintOrigin::signed(ALICE),
				Box::new(VersionedMultiLocation::V3(MultiLocation::new(
					1,
					X1(Parachain(penpal_id())),
				))),
				Box::new(VersionedMultiLocation::V3(MultiLocation::from(AccountId32 {
					network: None,
					id: ALICE.into(),
				}))),
				Box::new(VersionedMultiAssets::V3(vec![dot.clone()].into())),
				0,
				Limited(MAX_XCM_WEIGHT),
			));
		});

		// check the transfer was not blocked by our our xcm config
		PenpalNet::execute_with(|| {
			use penpal_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Penpal's post transfer balances and issuance
		let (penpal_post_alice_dot_balance, penpal_post_dot_issuance) =
			PenpalNet::execute_with(|| {
				penpal_runtime::System::reset_events();
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// check Statemint's post transfer balances and issuance
		let (
			statemint_post_alice_dot_balance,
			statemint_post_penpal_dot_balance,
			statemint_post_dot_issuance,
		) = StatemintNet::execute_with(|| {
			statemint_runtime::System::reset_events();
			(
				StatemintBalances::free_balance(ALICE),
				StatemintBalances::free_balance(ParachainAccounts::penpal_child_account()),
				StatemintBalances::total_issuance(),
			)
		});

		let penpal_delta_alice_dot_balance =
			penpal_post_alice_dot_balance - penpal_prev_alice_dot_balance;
		let penpal_delta_dot_issuance = penpal_post_dot_issuance - penpal_prev_dot_issuance;

		let statemint_delta_alice_dot_balance =
			statemint_prev_alice_dot_balance - statemint_post_alice_dot_balance;
		let statemint_delta_penpal_dot_balance =
			statemint_post_penpal_dot_balance - statemint_prev_penpal_dot_balance;
		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;

		assert!(
			penpal_delta_alice_dot_balance >= RESERVE_TRANSFER_AMOUNT - penpal_runtime::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT) &&
				penpal_delta_alice_dot_balance <= RESERVE_TRANSFER_AMOUNT,
			"Penpal ALICE DOT balance should have increased by at least the transfer amount minus the XCM execution fee"
		);

		assert!(
			penpal_delta_dot_issuance >= RESERVE_TRANSFER_AMOUNT - penpal_runtime::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT) &&
				penpal_delta_dot_issuance <= RESERVE_TRANSFER_AMOUNT,
			"Penpal DOT issuance should have increased by at least the transfer amount minus the XCM execution fee"
		);

		assert_eq!(
			statemint_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Statemint ALICE DOT balance should have decreased by the transfer amount"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);

		assert!(
			statemint_delta_penpal_dot_balance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Penpal's sovereign account on Statemint's balance of DOT should not change, (except for fees which are burnt)"
		);
	}

	#[test]
	fn penpal_to_reserve() {
		Network::reset();
		// asset to transfer
		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();
		// execution fee
		let execution_dot: MultiAsset = (MultiLocation::parent(), MAX_XCM_FEE).into();

		// check Penpal's pre transfer balances and issuance
		let (penpal_prev_alice_dot_balance, penpal_prev_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// check Statemint's pre transfer balances and issuance
		let (statemint_prev_alice_dot_balance, statemint_prev_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// construct the XCM to transfer from Penpal to Statemint's reserve
		let transfer_xcm: Xcm<PenpalCall> = Xcm(vec![
			WithdrawAsset(vec![dot.clone()].into()),
			BuyExecution { fees: execution_dot.clone(), weight_limit: Limited(MAX_XCM_WEIGHT) },
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: MultiLocation::new(1, X1(Parachain(statemint_id()))),
				xcm: Xcm(vec![
					BuyExecution {
						fees: execution_dot.clone(),
						weight_limit: Limited(MAX_XCM_WEIGHT),
					},
					DepositAsset {
						assets: All.into(),
						beneficiary: MultiLocation::new(
							0,
							AccountId32 { network: None, id: ALICE.into() },
						),
					},
				]),
			},
		]);

		// do the transfer
		PenpalNet::execute_with(|| {
			assert_ok!(PenpalXcmPallet::execute(
				PenpalOrigin::signed(ALICE),
				Box::new(VersionedXcm::V3(transfer_xcm)),
				MAX_XCM_WEIGHT,
			));
		});

		// check that the xcm was not blocked
		StatemintNet::execute_with(|| {
			use statemint_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Penpal's post transfer balances and issuance
		let (penpal_post_alice_dot_balance, penpal_post_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// check Statemint's post transfer balances and issuance
		let (statemint_post_alice_dot_balance, statemint_post_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		let penpal_delta_dot_issuance = penpal_prev_dot_issuance - penpal_post_dot_issuance;
		let penpal_delta_alice_dot_balance =
			penpal_prev_alice_dot_balance - penpal_post_alice_dot_balance;

		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;
		let statemint_delta_alice_dot_balance =
			statemint_post_alice_dot_balance - statemint_prev_alice_dot_balance;

		assert_eq!(
			penpal_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Penpal's ALICE DOT balance should decrease by the transfer amount"
		);

		assert_eq!(
			penpal_delta_dot_issuance, RESERVE_TRANSFER_AMOUNT,
			"Penpal's DOT issuance should decrease by transfer amount due to burn"
		);

		assert!(
			statemint_delta_alice_dot_balance  >= RESERVE_TRANSFER_AMOUNT - statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT) &&
				statemint_delta_alice_dot_balance <= RESERVE_TRANSFER_AMOUNT,
			"Statemint's ALICE DOT balance should increase by at least the transfer amount minus the max allowed fees"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);
	}

	#[test]
	fn polimec_to_penpal() {
		Network::reset();

		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();
		let execution_dot: MultiAsset = (MultiLocation::parent(), MAX_XCM_FEE).into();

		let transfer_xcm: Xcm<PolimecCall> = Xcm(vec![
			WithdrawAsset(vec![dot.clone()].into()),
			BuyExecution { fees: execution_dot.clone(), weight_limit: Limited(MAX_XCM_WEIGHT) },
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: MultiLocation::new(1, X1(Parachain(statemint_id()))),
				xcm: Xcm::<()>(vec![
					BuyExecution {
						fees: execution_dot.clone(),
						weight_limit: Limited(MAX_XCM_WEIGHT),
					},
					DepositReserveAsset {
						assets: All.into(),
						dest: MultiLocation::new(1, X1(Parachain(penpal_id()))),
						xcm: Xcm::<()>(vec![
							BuyExecution {
								fees: execution_dot.clone(),
								weight_limit: Limited(MAX_XCM_WEIGHT),
							},
							DepositAsset {
								assets: All.into(),
								beneficiary: X1(AccountId32 { network: None, id: ALICE.into() })
									.into(),
							},
						]),
					},
				]),
			},
		]);

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_prev_alice_dot_balance,
			polimec_prev_alice_plmc_balance,
			polimec_prev_dot_issuance,
			polimec_prev_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (statemint_prev_alice_dot_balance, statemint_prev_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// check Penpal's pre transfer balances and issuance
		let (penpal_prev_alice_dot_balance, penpal_prev_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// send the XCM message
		PolimecNet::execute_with(|| {
			assert_ok!(PolimecXcmPallet::execute(
				PolimecOrigin::signed(ALICE),
				Box::new(VersionedXcm::V3(transfer_xcm)),
				MAX_XCM_WEIGHT
			));
		});

		StatemintNet::execute_with(|| {
			use statemint_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		PenpalNet::execute_with(|| {
			use penpal_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_post_alice_dot_balance,
			polimec_post_alice_plmc_balance,
			polimec_post_dot_issuance,
			polimec_post_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (statemint_post_alice_dot_balance, statemint_post_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// check Penpal's pre transfer balances and issuance
		let (penpal_post_alice_dot_balance, penpal_post_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		let penpal_delta_dot_issuance = penpal_post_dot_issuance - penpal_prev_dot_issuance;
		let penpal_delta_alice_dot_balance =
			penpal_post_alice_dot_balance - penpal_prev_alice_dot_balance;

		let polimec_delta_dot_issuance = polimec_prev_dot_issuance - polimec_post_dot_issuance;
		let polimec_delta_alice_dot_balance =
			polimec_prev_alice_dot_balance - polimec_post_alice_dot_balance;
		let polimec_delta_plmc_issuance = polimec_prev_plmc_issuance - polimec_post_plmc_issuance;
		let polimec_delta_alice_plmc_balance =
			polimec_prev_alice_plmc_balance - polimec_post_alice_plmc_balance;

		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;
		let statemint_delta_alice_dot_balance =
			statemint_prev_alice_dot_balance - statemint_post_alice_dot_balance;

		assert!(
			penpal_delta_alice_dot_balance > RESERVE_TRANSFER_AMOUNT - MAX_XCM_FEE * 3
				&& penpal_delta_alice_dot_balance < RESERVE_TRANSFER_AMOUNT,
			"Expected funds are not received by Alice on Penpal"
		);

		assert_eq!(
			penpal_delta_dot_issuance, penpal_delta_alice_dot_balance,
			"Expected Penpal's DOT issuance to increase by the same amount as Alice's DOT balance"
		);

		assert_eq!(
			polimec_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Polimec ALICE DOT balance should change by the transfer amount"
		);

		assert_eq!(
			polimec_delta_dot_issuance, RESERVE_TRANSFER_AMOUNT,
			"Polimec DOT issuance should change by the transfer amount"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);

		assert_eq!(
			statemint_delta_alice_dot_balance, 0,
			"ALICE account on Statemint should not have changed"
		);

		assert_eq!(
			polimec_delta_alice_plmc_balance, 0,
			"Polimec ALICE PLMC balance should not have changed"
		);

		assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
	}

	#[test]
	fn penpal_to_polimec() {
		Network::reset();

		let dot: MultiAsset = (MultiLocation::parent(), RESERVE_TRANSFER_AMOUNT).into();
		let execution_dot: MultiAsset = (MultiLocation::parent(), MAX_XCM_FEE).into();

		let transfer_xcm: Xcm<PenpalCall> = Xcm(vec![
			WithdrawAsset(vec![dot.clone()].into()),
			BuyExecution { fees: execution_dot.clone(), weight_limit: Limited(MAX_XCM_WEIGHT) },
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: MultiLocation::new(1, X1(Parachain(statemint_id()))),
				xcm: Xcm::<()>(vec![
					BuyExecution {
						fees: execution_dot.clone(),
						weight_limit: Limited(MAX_XCM_WEIGHT),
					},
					DepositReserveAsset {
						assets: All.into(),
						dest: MultiLocation::new(1, X1(Parachain(polimec_id()))),
						xcm: Xcm::<()>(vec![
							BuyExecution {
								fees: execution_dot.clone(),
								weight_limit: Limited(MAX_XCM_WEIGHT),
							},
							DepositAsset {
								assets: All.into(),
								beneficiary: X1(AccountId32 { network: None, id: ALICE.into() })
									.into(),
							},
						]),
					},
				]),
			},
		]);

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_prev_alice_dot_balance,
			polimec_prev_alice_plmc_balance,
			polimec_prev_dot_issuance,
			polimec_prev_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (statemint_prev_alice_dot_balance, statemint_prev_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// check Penpal's pre transfer balances and issuance
		let (penpal_prev_alice_dot_balance, penpal_prev_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		// send the XCM message
		PenpalNet::execute_with(|| {
			assert_ok!(PenpalXcmPallet::execute(
				PenpalOrigin::signed(ALICE),
				Box::new(VersionedXcm::V3(transfer_xcm)),
				MAX_XCM_WEIGHT
			));
		});

		StatemintNet::execute_with(|| {
			use statemint_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		PolimecNet::execute_with(|| {
			use polimec_runtime::{RuntimeEvent, System};
			let events = System::events();
			assert!(events.iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. })
			)));
		});

		// check Polimec's pre transfer balances and issuance
		let (
			polimec_post_alice_dot_balance,
			polimec_post_alice_plmc_balance,
			polimec_post_dot_issuance,
			polimec_post_plmc_issuance,
		) = PolimecNet::execute_with(|| {
			(
				PolimecAssets::balance(RELAY_ASSET_ID, ALICE),
				PolimecBalances::free_balance(ALICE),
				PolimecAssets::total_issuance(RELAY_ASSET_ID),
				PolimecBalances::total_issuance(),
			)
		});

		// check Statemint's pre transfer balances and issuance
		let (statemint_post_alice_dot_balance, statemint_post_dot_issuance) =
			StatemintNet::execute_with(|| {
				(StatemintBalances::free_balance(ALICE), StatemintBalances::total_issuance())
			});

		// check Penpal's pre transfer balances and issuance
		let (penpal_post_alice_dot_balance, penpal_post_dot_issuance) =
			PenpalNet::execute_with(|| {
				(PenpalBalances::free_balance(ALICE), PenpalBalances::total_issuance())
			});

		let penpal_delta_dot_issuance = penpal_prev_dot_issuance - penpal_post_dot_issuance;
		let penpal_delta_alice_dot_balance =
			penpal_prev_alice_dot_balance - penpal_post_alice_dot_balance;

		let polimec_delta_dot_issuance = polimec_post_dot_issuance - polimec_prev_dot_issuance;
		let polimec_delta_alice_dot_balance =
			polimec_post_alice_dot_balance - polimec_prev_alice_dot_balance;
		let polimec_delta_plmc_issuance = polimec_post_plmc_issuance - polimec_prev_plmc_issuance;
		let polimec_delta_alice_plmc_balance =
			polimec_post_alice_plmc_balance - polimec_prev_alice_plmc_balance;

		let statemint_delta_dot_issuance =
			statemint_prev_dot_issuance - statemint_post_dot_issuance;
		let statemint_delta_alice_dot_balance =
			statemint_prev_alice_dot_balance - statemint_post_alice_dot_balance;

		assert!(
			polimec_delta_alice_dot_balance > RESERVE_TRANSFER_AMOUNT - MAX_XCM_FEE * 3
				&& polimec_delta_alice_dot_balance < RESERVE_TRANSFER_AMOUNT,
			"Expected funds are not received by Alice on Polimec"
		);

		assert_eq!(
			polimec_delta_dot_issuance, polimec_delta_alice_dot_balance,
			"Expected Polimec's DOT issuance to increase by the same amount as Alice's DOT balance"
		);

		assert_eq!(
			penpal_delta_alice_dot_balance, RESERVE_TRANSFER_AMOUNT,
			"Penpal ALICE DOT balance should change by the transfer amount"
		);

		assert_eq!(
			penpal_delta_dot_issuance, RESERVE_TRANSFER_AMOUNT,
			"Penpal DOT issuance should change by the transfer amount"
		);

		assert!(
			statemint_delta_dot_issuance <= statemint_runtime::constants::fee::WeightToFee::weight_to_fee(&MAX_XCM_WEIGHT),
			"Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
		);

		assert_eq!(
			statemint_delta_alice_dot_balance, 0,
			"ALICE account on Statemint should not have changed"
		);

		assert_eq!(
			polimec_delta_alice_plmc_balance, 0,
			"Polimec ALICE PLMC balance should not have changed"
		);

		assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
	}

	#[test]
	fn commit_identity() {
		Network::reset();

		let did: DidIdentifier = did_auth_key().public().into();

		// 1. Send identity proof from DIP provider to DIP consumer.
		ProviderParachain::execute_with(|| {
			use frame_system::RawOrigin;

			assert_ok!(DipProvider::commit_identity(
				RawOrigin::Signed(ProviderAccountId::from([0u8; 32])).into(),
				did.clone(),
				Box::new(ParentThen(X1(Parachain(polimec_id()))).into()),
				Box::new((Here, 1_000_000_000).into()),
				Weight::from_ref_time(4_000),
			));
		});
		// 2. Verify that the proof has made it to the DIP consumer.
		PolimecNet::execute_with(|| {
			use polimec_parachain_runtime::{RuntimeEvent, System};
			// TODO: Remove this once we resolve the panic.
			let events = System::events();
			for elem in events {
				println!("{:?}", elem.event);
			}
			// 2.1 Verify that there was no XCM error.
			assert!(!System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(XcmpEvent::Fail { error: _, message_hash: _, weight: _ })
			)));

			// 2.2 Verify the proof digest was stored correctly.
			assert!(polimec_parachain_runtime::DipConsumer::identity_proofs(&did).is_some());
		});
		// 3. Call an extrinsic on the consumer chain with a valid proof
		let did_details = ProviderParachain::execute_with(|| {
			use did::Did;
			Did::get(&did).expect("DID details should be stored on the provider chain.")
		});
		// 3.1 Generate a proof
		let CompleteMerkleProof { proof, .. } =
			DidMerkleRootGenerator::<ProviderRuntime>::generate_proof(
				&did_details,
				[did_details.authentication_key].iter(),
			)
			.expect("Proof generation should not fail");
		// 3.2 Call the `dispatch_as` extrinsic on the consumer chain with the generated
		// proof
		PolimecNet::execute_with(|| {
			use frame_system::RawOrigin;
			use polimec_parachain_runtime::{DidLookup, DipConsumer, RuntimeCall};

			assert_ok!(DipConsumer::dispatch_as(
				RawOrigin::Signed(DISPATCHER_ACCOUNT).into(),
				did.clone(),
				Proof { blinded: proof.blinded, revealed: proof.revealed }.into(),
				Box::new(RuntimeCall::DidLookup(
					pallet_did_lookup::Call::<PolimecRuntime>::associate_sender {}
				)),
			));
			// Verify the account -> DID link exists and contains the right information
			let linked_did =
				DidLookup::connected_dids::<LinkableAccountId>(DISPATCHER_ACCOUNT.into())
					.map(|link| link.did);
			assert_eq!(linked_did, Some(did));
		});
	}
}
