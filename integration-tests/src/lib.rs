use frame_support::{pallet_prelude::Weight, traits::GenesisBuild};
use sp_runtime::AccountId32;

use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

decl_test_relay_chain! {
	pub struct Polkadot {
		Runtime = polkadot_runtime::Runtime,
		XcmConfig = polkadot_runtime::xcm_config::XcmConfig,
		new_ext = kusama_ext(),
	}
}

decl_test_parachain! {
	pub struct YayoiPumpkin {
		Runtime = yayoi::Runtime,
		RuntimeOrigin = yayoi::RuntimeOrigin,
		XcmpMessageHandler = yayoi::XcmpQueue,
		DmpMessageHandler = yayoi::DmpQueue,
		new_ext = yayoi_ext(1),
	}
}

decl_test_parachain! {
	pub struct YayoiMushroom {
		Runtime = yayoi::Runtime,
		RuntimeOrigin = yayoi::RuntimeOrigin,
		XcmpMessageHandler = yayoi::XcmpQueue,
		DmpMessageHandler = yayoi::DmpQueue,
		new_ext = yayoi_ext(2),
	}
}

decl_test_parachain! {
	pub struct YayoiOctopus {
		Runtime = yayoi::Runtime,
		RuntimeOrigin = yayoi::RuntimeOrigin,
		XcmpMessageHandler = yayoi::XcmpQueue,
		DmpMessageHandler = yayoi::DmpQueue,
		new_ext = yayoi_ext(3),
	}
}

decl_test_network! {
	pub struct Network {
		relay_chain = KusamaNet,
		parachains = vec![
			(1, YayoiPumpkin),
			(2, YayoiMushroom),
			(3, YayoiOctopus),
		],
	}
}

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const INITIAL_BALANCE: u128 = 1_000_000_000_000;

pub fn yayoi_ext(para_id: u32) -> sp_io::TestExternalities {
	use yayoi::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	let parachain_info_config = parachain_info::GenesisConfig {
		parachain_id: para_id.into(),
	};

	<parachain_info::GenesisConfig as GenesisBuild<Runtime, _>>::assimilate_storage(&parachain_info_config, &mut t)
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE)],
	}
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

fn default_parachains_host_configuration() -> polkadot_runtime_parachains::configuration::HostConfiguration<polkadot_primitives::v2::BlockNumber> {
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

pub fn kusama_ext() -> sp_io::TestExternalities {
	use polkadot_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, INITIAL_BALANCE)],
	}
		.assimilate_storage(&mut t)
		.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
		config: default_parachains_host_configuration(),
	}
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;

	use cumulus_primitives_core::ParaId;
	use frame_support::{assert_ok, traits::Currency};
	use sp_runtime::traits::AccountIdConversion;
	use xcm::{v3::prelude::*, VersionedMultiLocation, VersionedXcm};
	use xcm_emulator::TestExt;

	#[test]
	fn dmp() {
		Network::reset();

		let remark = yayoi::RuntimeCall::System(frame_system::Call::<yayoi::Runtime>::remark_with_event {
			remark: "Hello from Kusama!".as_bytes().to_vec(),
		});
		KusamaNet::execute_with(|| {
			assert_ok!(polkadot_runtime::XcmPallet::force_default_xcm_version(
				polkadot_runtime::RuntimeOrigin::root(),
				Some(3)
			));
			assert_ok!(polkadot_runtime::XcmPallet::send_xcm(
				Here,
				Parachain(1),
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
					call: remark.encode().into(),
				}]),
			));
		});

		YayoiPumpkin::execute_with(|| {
			use yayoi::{RuntimeEvent, System};
			System::events().iter().for_each(|r| println!(">>> {:?}", r.event));

			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
			)));
		});
	}

	#[test]
	fn ump() {
		Network::reset();

		KusamaNet::execute_with(|| {
			assert_ok!(polkadot_runtime::XcmPallet::force_default_xcm_version(
				polkadot_runtime::RuntimeOrigin::root(),
				Some(3)
			));
			let _ = polkadot_runtime::Balances::deposit_creating(
				&ParaId::from(1).into_account_truncating(),
				1_000_000_000_000,
			);
		});

		let remark =
			polkadot_runtime::RuntimeCall::System(frame_system::Call::<polkadot_runtime::Runtime>::remark_with_event {
				remark: "Hello from Pumpkin!".as_bytes().to_vec(),
			});
		YayoiPumpkin::execute_with(|| {
			assert_ok!(yayoi::PolkadotXcm::force_default_xcm_version(
				yayoi::RuntimeOrigin::root(),
				Some(3)
			));
			assert_ok!(yayoi::PolkadotXcm::send_xcm(
				Here,
				Parent,
				Xcm(vec![
					UnpaidExecution {
						weight_limit: Unlimited,
						check_origin: None,
					},
					Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
						call: remark.encode().into(),
					}
				]),
			));
		});

		KusamaNet::execute_with(|| {
			use polkadot_runtime::{RuntimeEvent, System};
			// TODO: https://github.com/paritytech/polkadot/pull/6824
			// assert!(System::events().iter().any(|r| matches!(
			// 	r.event,
			// 	RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
			// )));
			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::Ump(polkadot_runtime_parachains::ump::Event::ExecutedUpward(
					_,
					Outcome::Incomplete(_, XcmError::NoPermission)
				))
			)));
		});
	}

	#[test]
	fn xcmp() {
		Network::reset();

		let remark = yayoi::RuntimeCall::System(frame_system::Call::<yayoi::Runtime>::remark_with_event {
			remark: "Hello from Pumpkin!".as_bytes().to_vec(),
		});
		YayoiPumpkin::execute_with(|| {
			assert_ok!(yayoi::PolkadotXcm::send_xcm(
				Here,
				MultiLocation::new(1, X1(Parachain(2))),
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: 20_000_000.into(),
					call: remark.encode().into(),
				}]),
			));
		});

		YayoiMushroom::execute_with(|| {
			use yayoi::{RuntimeEvent, System};
			System::events().iter().for_each(|r| println!(">>> {:?}", r.event));

			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
			)));
		});
	}

	#[test]
	fn xcmp_through_a_parachain() {
		use yayoi::{PolkadotXcm, Runtime, RuntimeCall};

		Network::reset();

		// The message goes through: Pumpkin --> Mushroom --> Octopus
		let remark = RuntimeCall::System(frame_system::Call::<Runtime>::remark_with_event {
			remark: "Hello from Pumpkin!".as_bytes().to_vec(),
		});
		let send_xcm_to_octopus = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: Box::new(VersionedMultiLocation::V3(MultiLocation::new(1, X1(Parachain(3))))),
			message: Box::new(VersionedXcm::V3(Xcm(vec![Transact {
				origin_kind: OriginKind::SovereignAccount,
				require_weight_at_most: 10_000_000.into(),
				call: remark.encode().into(),
			}]))),
		});
		YayoiPumpkin::execute_with(|| {
			assert_ok!(PolkadotXcm::send_xcm(
				Here,
				MultiLocation::new(1, X1(Parachain(2))),
				Xcm(vec![Transact {
					origin_kind: OriginKind::SovereignAccount,
					// TODO: fix in 0.9.40, https://github.com/paritytech/polkadot/pull/6787
					// require_weight_at_most: 100_000_000.into(),
					require_weight_at_most: 200_000_000.into(),
					call: send_xcm_to_octopus.encode().into(),
				}]),
			));
		});

		YayoiMushroom::execute_with(|| {
			use yayoi::{RuntimeEvent, System};
			System::events().iter().for_each(|r| println!(">>> {:?}", r.event));

			assert!(System::events()
				.iter()
				.any(|r| matches!(r.event, RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent(_, _, _)))));
		});

		YayoiOctopus::execute_with(|| {
			use yayoi::{RuntimeEvent, System};
			// execution would fail, but good enough to check if the message is received
			System::events().iter().for_each(|r| println!(">>> {:?}", r.event));

			assert!(System::events().iter().any(|r| matches!(
				r.event,
				RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Fail { .. })
			)));
		});
	}

	#[test]
	fn deduplicate_dmp() {
		Network::reset();
		KusamaNet::execute_with(|| {
			assert_ok!(polkadot_runtime::XcmPallet::force_default_xcm_version(
				polkadot_runtime::RuntimeOrigin::root(),
				Some(3)
			));
		});

		kusama_send_rmrk("Kusama", 2);
		parachain_receive_and_reset_events(true);

		// a different dmp message in same relay-parent-block allow execution.
		kusama_send_rmrk("Polkadot", 1);
		parachain_receive_and_reset_events(true);

		// same dmp message with same relay-parent-block wouldn't execution
		kusama_send_rmrk("Kusama", 1);
		parachain_receive_and_reset_events(false);

		// different relay-parent-block allow dmp message execution
		KusamaNet::execute_with(|| polkadot_runtime::System::set_block_number(2));

		kusama_send_rmrk("Kusama", 1);
		parachain_receive_and_reset_events(true);

		// reset can send same dmp message again
		Network::reset();
		KusamaNet::execute_with(|| {
			assert_ok!(polkadot_runtime::XcmPallet::force_default_xcm_version(
				polkadot_runtime::RuntimeOrigin::root(),
				Some(3)
			));
		});

		kusama_send_rmrk("Kusama", 1);
		parachain_receive_and_reset_events(true);
	}

	fn kusama_send_rmrk(msg: &str, count: u32) {
		let remark = yayoi::RuntimeCall::System(frame_system::Call::<yayoi::Runtime>::remark_with_event {
			remark: msg.as_bytes().to_vec(),
		});
		KusamaNet::execute_with(|| {
			for _ in 0..count {
				assert_ok!(polkadot_runtime::XcmPallet::send_xcm(
					Here,
					Parachain(1),
					Xcm(vec![Transact {
						origin_kind: OriginKind::SovereignAccount,
						require_weight_at_most: Weight::from_parts(INITIAL_BALANCE as u64, 1024 * 1024),
						call: remark.encode().into(),
					}]),
				));
			}
		});
	}

	fn parachain_receive_and_reset_events(received: bool) {
		YayoiPumpkin::execute_with(|| {
			use yayoi::{RuntimeEvent, System};
			System::events().iter().for_each(|r| println!(">>> {:?}", r.event));

			if received {
				assert!(System::events().iter().any(|r| matches!(
					r.event,
					RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
				)));

				System::reset_events();
			} else {
				assert!(System::events().iter().all(|r| !matches!(
					r.event,
					RuntimeEvent::System(frame_system::Event::Remarked { sender: _, hash: _ })
				)));
			}
		});
	}
}