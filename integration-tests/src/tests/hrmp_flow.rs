// use crate::*;

// const MAX_REF_TIME: u64 = 700_000_000;
// const MAX_PROOF_SIZE: u64 = 10_000;
// pub const REF_TIME_THRESHOLD: u64 = 33;
// pub const PROOF_SIZE_THRESHOLD: u64 = 33;

// use polkadot_runtime_parachains::{hrmp as parachain_hrmp, origin as parachains_origin, paras as parachains_paras};

// #[test]
// fn hrmp_notification() {
// 	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);
// 	let execution_dot: MultiAsset = (MultiLocation { parents: 0, interior: Here }, 1_0_000_000_000u128).into();
// 	let call = polkadot_runtime::RuntimeCall::Hrmp(parachain_hrmp::Call::<PolkadotRuntime>::hrmp_init_open_channel{
// 		recipient: Polimec::para_id(),
// 		proposed_max_capacity: 1024,
// 		proposed_max_message_size: 1024 * 1024,
// 	});
// 	// let hrmp_init_open_call = parachain_hrmp::Call::<PolkadotRuntime>::hrmp_init_open_channel{
// 	// 	recipient: Polimec::para_id(),
// 	// 	proposed_max_capacity: 1024,
// 	// 	proposed_max_message_size: 1024 * 1024,
// 	// };
// 	let encoded_call = call.encode();

// 	PolkadotRelay::execute_with(||{
// 		let x = parachains_paras::ParaLifecycles::<PolkadotRuntime>::iter();
// 		let y = 10;
// 	});

// 	let xcm = VersionedXcm::from(Xcm(vec![
// 		WithdrawAsset(vec![execution_dot.clone()].into()),
// 		BuyExecution { fees: execution_dot.clone(), weight_limit: Unlimited },
// 		Transact { origin_kind: OriginKind::Native, require_weight_at_most: max_weight, call: encoded_call.into() },
// 		RefundSurplus,
// 		DepositAsset {
// 			assets: Wild(All),
// 			beneficiary: MultiLocation {
// 				parents: 0,
// 				interior: X1(Parachain(Penpal::para_id().into())),
// 			},
// 		},
// 	]));

// 	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
// 	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

// 	Penpal::execute_with(|| {
// 		assert_ok!(PenpalXcmPallet::send(PenpalOrigin::root(), bx!(Penpal::parent_location().into()), bx!(xcm),));
// 		println!("penpal events:");
// 		dbg!(Penpal::events())
// 	});

// 	PolkadotRelay::execute_with(|| {
// 		println!("polkadot events:");
// 		dbg!(PolkadotRelay::events());
// 	});

// 	Polimec::execute_with(|| {
// 		println!("polimec events:");
// 		dbg!(Polimec::events());
// 	});

// }
