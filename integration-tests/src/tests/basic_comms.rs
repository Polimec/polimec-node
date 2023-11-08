use crate::*;

const MAX_REF_TIME: u64 = 300_000_000;
const MAX_PROOF_SIZE: u64 = 10_000;

#[test]
fn dmp() {
	let remark = PolimecCall::System(frame_system::Call::<PolimecRuntime>::remark_with_event {
		remark: "Hello from Polkadot!".as_bytes().to_vec(),
	});
	let sudo_origin = PolkadotOrigin::root();
	let para_id = 3344;
	let xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE),
			call: remark.encode().into(),
		},
	]));

	PolkadotRelay::execute_with(|| {
		assert_ok!(PolkadotXcmPallet::send(sudo_origin, bx!(Parachain(para_id).into()), bx!(xcm),));

		assert_expected_events!(
			PolkadotRelay,
			vec![
				PolkadotEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Polimec::execute_with(|| {
		let events = Polimec::events();
		assert_expected_events!(
			Polimec,
			vec![
				PolimecEvent::System(frame_system::Event::Remarked { sender: _, hash: _ }) => {},
			]
		);
	});
}

#[test]
fn ump() {
	use polkadot_runtime_parachains::inclusion::{AggregateMessageOrigin, UmpQueueId};
	let burn_transfer = PolkadotCall::Balances(pallet_balances::Call::<PolkadotRuntime>::transfer {
		dest: PolkadotAccountId::from([0u8; 32]).into(),
		value: 1_000,
	});

	let here_asset: MultiAsset = (MultiLocation::here(), 1_0_000_000_000u128).into();

	Polimec::execute_with(|| {
		assert_ok!(PolimecXcmPallet::force_default_xcm_version(PolimecOrigin::root(), Some(3)));

		assert_ok!(PolimecXcmPallet::send_xcm(
			Here,
			Parent,
			Xcm(vec![
				WithdrawAsset(vec![here_asset.clone()].into()),
				BuyExecution { fees: here_asset.clone(), weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE),
					call: burn_transfer.encode().into(),
				}
			]),
		));
	});

	PolkadotRelay::execute_with(|| {
		assert_expected_events!(
			PolkadotRelay,
			vec![
				PolkadotEvent::MessageQueue(pallet_message_queue::Event::Processed {
					id: _,
					origin: AggregateMessageOrigin::Ump(
						UmpQueueId::Para(_para_id)
					),
					weight_used: _,
					success: false
				}) => {},
			]
		);
	});
}

#[test]
fn xcmp() {
	let burn_transfer = PolimecCall::Balances(pallet_balances::Call::<PolimecRuntime>::transfer {
		dest: PolimecAccountId::from([0u8; 32]).into(),
		value: 1_000,
	});

	let here_asset: MultiAsset = (MultiLocation::here(), 1_0_000_000_000u128).into();

	Penpal::execute_with(|| {
		assert_ok!(PenpalXcmPallet::send_xcm(
			Here,
			MultiLocation::new(1, X1(Parachain(Polimec::para_id().into()))),
			Xcm(vec![
				WithdrawAsset(vec![here_asset.clone()].into()),
				BuyExecution { fees: here_asset.clone(), weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE),
					call: burn_transfer.encode().into(),
				}
			]),
		));
	});

	let penpal_account = Polimec::sovereign_account_id_of((Parent, Parachain(Penpal::para_id().into())).into());
	let penpal_balance = Polimec::account_data_of(penpal_account.clone()).free;
	dbg!(penpal_account.clone());
	dbg!(penpal_balance);

	Polimec::execute_with(|| {
		assert_expected_events!(
			Polimec,
			vec![
				PolimecEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. }) => {},
			]
		);
	});
}
