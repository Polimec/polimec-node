use crate::*;
use tests::defaults::*;
use std::cell::RefCell;
use polimec_parachain_runtime::PolimecFunding;

const MAX_REF_TIME: u64 = 700_000_000;
const MAX_PROOF_SIZE: u64 = 10_000;
pub const REF_TIME_THRESHOLD: u64 = 33;
pub const PROOF_SIZE_THRESHOLD: u64 = 33;

#[test]
fn migration_check() {
	let mut inst = IntegrationInstantiator::new(None);
	let project_id = Polimec::execute_with(|| {
		inst.create_finished_project(
			default_project(issuer(), 0),
			issuer(),
			default_evaluations(),
			default_bids(),
			default_community_contributions(),
			vec![],
		)
	});
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);
	let expected_currency: MultiAsset =
		(MultiLocation { parents: 0, interior: Here }, 1_000_000_0_000_000_000u128).into(); // 1MM units for migrations
	// let xcm = Xcm(vec![
	// 	UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
	// 	WithdrawAsset(vec![expected_currency.clone()].into()),
	// 	ReportHolding {
	// 		response_info: QueryResponseInfo {
	// 			destination: ParentThen(Parachain(3355).into()).into(),
	// 			query_id: 0,
	// 			max_weight: max_weight.clone(),
	// 		},
	// 		assets: Wild(All),
	// 	},
	// 	QueryPallet {
	// 		module_name: Vec::from("polimec_receiver"),
	// 		response_info: QueryResponseInfo {
	// 			destination: ParentThen(Parachain(3355).into()).into(),
	// 			query_id: 1,
	// 			max_weight: max_weight.clone(),
	// 		},
	// 	},
	// 	DepositAsset { assets: Wild(All), beneficiary: ParentThen(Parachain(3355).into()).into() },
	// ]);
	let polimec_on_penpal = Penpal::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into());
	let balance_polimec = Penpal::account_data_of(polimec_on_penpal);

	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);
	use xcm::v3::PalletInfo;

	// Mock HRMP establishment
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::do_set_para_id_for_project(
			&issuer(),
			project_id,
			ParaId::from(6969u32)
		));

		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));

	});

	// Polimec::execute_with(|| {
	// 	let penpal_loc: MultiLocation = MultiLocation::from(ParentThen(X1(Parachain(Penpal::para_id().into()))));
	// 	let now = PolimecSystem::block_number();
	// 	// the parameters of the call are not relevant since they will be stripped and replaced by the query result
	// 	let call = PolimecCall::PolimecFunding(pallet_funding::Call::migration_check_response {
	// 		query_id: Default::default(),
	// 		response: Default::default(),
	// 	});
	// 	let query_id_0 = PolimecXcmPallet::new_notify_query(penpal_loc, call.clone(), now + 20u32, Here);
	// 	let query_id_1 = PolimecXcmPallet::new_notify_query(penpal_loc, call, now + 20u32, Here);
	// 	assert_ok!(PolimecXcmPallet::send_xcm(Here, penpal_loc, xcm));
	// 	println!("polimec events:");
	// 	dbg!(Polimec::events())
	// });

	Penpal::execute_with(|| {
		println!("penpal events:");
		dbg!(Penpal::events());
	});

	Polimec::execute_with(|| {
		println!("Polimec events:");
		dbg!(Polimec::events());
	});
}
