use crate::*;

const MAX_REF_TIME: u64 = 700_000_000;
const MAX_PROOF_SIZE: u64 = 10_000;
pub const REF_TIME_THRESHOLD: u64 = 33;
pub const PROOF_SIZE_THRESHOLD: u64 = 33;

#[test]
fn balance_query() {
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);
	let execution_currency: MultiAsset = (MultiLocation { parents: 0, interior: Here }, 1_0_000_000_000u128).into(); // 1 unit for executing
	let expected_currency: MultiAsset =
		(MultiLocation { parents: 0, interior: Here }, 1_000_000_0_000_000_000u128).into(); // 1MM units for migrations
	let xcm = Xcm(vec![
		UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		WithdrawAsset(vec![expected_currency.clone()].into()),
		ReportHolding {
			response_info: QueryResponseInfo {
				destination: ParentThen(Parachain(3355).into()).into(),
				query_id: 0,
				max_weight: max_weight.clone(),
			},
			assets: Wild(All),
		},
		DepositAsset { assets: Wild(All), beneficiary: ParentThen(Parachain(3355).into()).into() }
	]);
	let polimec_on_penpal = Penpal::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into());
	let balance_polimec = Penpal::account_data_of(polimec_on_penpal);

	let penpal_sov_acc = PolkadotRelay::sovereign_account_id_of(Parachain(Penpal::para_id().into()).into());
	PolkadotRelay::fund_accounts(vec![(penpal_sov_acc, 100_0_000_000_000u128)]);

	Polimec::execute_with(|| {
		assert_ok!(PolimecXcmPallet::send_xcm(Here, MultiLocation::from(ParentThen(X1(Parachain(Penpal::para_id().into())))), xcm));
		println!("polimec events:");
		dbg!(Polimec::events())
	});

	Penpal::execute_with(|| {
		println!("penpal events:");
		dbg!(Penpal::events());
	});

	Polimec::execute_with(|| {
		println!("Polimec events:");
		dbg!(Polimec::events());
	});


}
