use crate::{
	asset_hub, polimec, AssetHubEvent, AssetHubOrigin, AssetHubRuntime, AssetHubWestendNet, AssetHubXcmPallet,
	PolimecAccountId, PolimecEvent, PolimecNet, PolimecRuntime, ALICE,
};
use parity_scale_codec::Encode;
use sp_runtime::traits::Hash;
use xcm::{v4::prelude::*, DoubleEncoded, VersionedLocation, VersionedXcm};
use xcm_emulator::{Chain, ConvertLocation, TestExt};

fn polimec_location() -> Location {
	Location::new(1, [Parachain(polimec::PARA_ID)])
}

const MESSAGE: [u8; 20] = *b"Hello from Asset Hub";

#[test]
fn transact_from_asset_hub_to_polimec_works() {
	AssetHubWestendNet::execute_with(|| {
		let remark_call: DoubleEncoded<polimec_runtime::RuntimeCall> =
			polimec_runtime::RuntimeCall::System(frame_system::Call::<PolimecRuntime>::remark_with_event {
				remark: MESSAGE.to_vec(),
			})
			.encode()
			.into();

		AssetHubXcmPallet::send(
			AssetHubOrigin::signed(AssetHubWestendNet::account_id_of(ALICE)),
			Box::new(VersionedLocation::V4(polimec_location())),
			Box::new(VersionedXcm::V4(Xcm(vec![
				Instruction::BuyExecution {
					fees: Asset { id: Location::parent().into(), fun: Fungibility::Fungible(1_000_000_000) }.into(),
					weight_limit: WeightLimit::Unlimited,
				},
				Instruction::Transact {
					origin_kind: OriginKind::SovereignAccount,
					call: remark_call,
					require_weight_at_most: Weight::MAX,
				}
				.into(),
			]
			.into()))),
		)
		.unwrap();

		let events = AssetHubWestendNet::events();

		let contains_xcm_sent =
			events.iter().any(|event| matches!(event, AssetHubEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. })));

		assert!(contains_xcm_sent, "Expected an XCM sent event in AssetHubWestendNet events");
	});

	PolimecNet::execute_with(|| {
		use cumulus_primitives_core::{AccountId32, Junctions, Location, Parachain};

		let events = PolimecNet::events();
		let alice_westend = AssetHubWestendNet::account_id_of(ALICE);

		let sender_sovereign_account: PolimecAccountId =
			polimec_runtime::xcm_config::LocationToAccountId::convert_location(&Location {
				parents: 1,
				interior: Junctions::X2(
					[Parachain(asset_hub::PARA_ID), AccountId32 { network: None, id: alice_westend.into() }].into(),
				),
			})
			.expect("Failed to convert Location to AccountId32");

		let expected_hash = <AssetHubRuntime as frame_system::Config>::Hashing::hash(&MESSAGE);

		let contains_remark = events.iter().any(|event| {
			matches!(
				event,
				PolimecEvent::System(frame_system::Event::Remarked { sender: ref event_sender, hash })
				if event_sender == &sender_sovereign_account &&
				*hash == expected_hash
			)
		});

		assert!(contains_remark, "Expected a remark event in Polimec events");
	});
}

// TODO: @dastansam To update and fix.
// #[test]
// fn transact_bid_with_receiving_account_from_ah() {

// 			use cumulus_primitives_core::{AccountId32, Junctions, Location, Parachain};

// 	let mut project_id_holder: ProjectId = 0;
// 	let mut alice_sovereign_on_polimec_holder: Option<PolimecAccountId> = None;

// 	let issuer_account_bytes: [u8; 32] = DAVE;
// 	let receiving_account_bytes: [u8; 32] = BOB;

// 	// Polimec Setup Part 1
// 	PolimecNet::execute_with(|| {
// 		let issuer_polimec: PolimecAccountId = issuer_account_bytes.into();
// 		polimec::set_prices(
// 			PricesBuilder::new().usdt(usdt_price()).usdc(usdc_price()).dot(dot_price()).plmc(plmc_price()).build(),
// 		);

// 		let alice_on_asset_hub_bytes: [u8; 32] = AssetHubWestendNet::account_id_of(ALICE).into();
// 		let alice_sovereign_polimec_acct =
// 			polimec_runtime::xcm_config::LocationToAccountId::convert_location(&Location::new(
// 				1,
// 				[Parachain(asset_hub::PARA_ID), AccountId32 { network: None, id: alice_on_asset_hub_bytes }],
// 			))
// 			.unwrap();
// 		alice_sovereign_on_polimec_holder = Some(alice_sovereign_polimec_acct.clone());

// 		assert_ok!(PolimecFunding::set_polimec_bidder_account(
// 			RawOrigin::Root.into(),
// 			alice_sovereign_polimec_acct.clone()
// 		));

// 		let project_id = pallet_funding::NextProjectId::<PolimecRuntime>::get();
// 		project_id_holder = project_id;
// 		let metadata = project_metadata(); // From e2e_full_flow_tests
// 		assert_ok!(PolimecFunding::create_project(RawOrigin::Signed(issuer_polimec.clone()).into(), metadata, None));

// 		let issuer_jwt = get_mock_jwt(
// 			issuer_polimec.clone(),
// 			pallet_dispenser::InvestorType::Institutional,
// 			generate_did_from_account(issuer_polimec.clone()),
// 		);
// 		assert_ok!(PolimecFunding::start_evaluation(
// 			RawOrigin::Signed(issuer_polimec.clone()).into(),
// 			issuer_jwt.clone().into(), // Convert to UntrustedToken
// 			project_id
// 		));

// 		let eval_duration = <PolimecRuntime as pallet_funding::Config>::EvaluationRoundDuration::get();
// 		frame_system::Pallet::<PolimecRuntime>::set_block_number(
// 			frame_system::Pallet::<PolimecRuntime>::block_number() + eval_duration + 1,
// 		);
// 		assert_eq!(PolimecFunding::projects(project_id).unwrap().status, ProjectStatus::AuctionRound);

// 		let usdt_asset_id = AcceptedFundingAsset::USDT.id();
// 		let usdt_unit = 1_000_000; // 6 decimal places for USDT
// 		let funding_usdt_to_mint = 5000 * usdt_unit + (100 * usdt_unit); // 5000 USDT for bid + buffer
// 		PolimecForeignAssets::mint(usdt_asset_id, &alice_sovereign_polimec_acct, funding_usdt_to_mint).unwrap();

// 		let plmc_ed = <PolimecRuntime as pallet_balances::Config>::ExistentialDeposit::get();
// 		let plmc_to_mint = 3000 * PLMC + plmc_ed; // Approx 2744 PLMC needed for bond + ED
// 		PolimecBalances::mint_into(&alice_sovereign_polimec_acct, plmc_to_mint).unwrap();
// 	});

// 	// AssetHub XCM Part
// 	AssetHubWestendNet::execute_with(|| {
// 		let alice_sovereign_on_polimec = alice_sovereign_on_polimec_holder.clone().unwrap();
// 		let project_id = project_id_holder;

// 		let ct_unit = get_ct_unit_from_metadata();
// 		let ct_amount_for_bid = 500 * ct_unit;

// 		let receiving_account_polimec_id_bytes: [u8; 32] = receiving_account_bytes.into();

// 		let jwt_token: UntrustedToken = get_mock_jwt(
// 			alice_sovereign_on_polimec.clone(),
// 			InvestorType::Professional, // Matches bidding_ticket_sizes.professional in project_metadata
// 			generate_did_from_account(alice_sovereign_on_polimec.clone()),
// 		)
// 		.into();
// 		let signature_bytes = [0u8; 65]; // Dummy, will be skipped due to PolimecBidderAccountId

// 		let bid_call =
// 			polimec_runtime::RuntimeCall::Funding(pallet_funding::Call::<PolimecRuntime>::bid_with_receiving_account {
// 				jwt: jwt_token,
// 				project_id,
// 				ct_amount: ct_amount_for_bid,
// 				mode: ParticipationMode::OTM,
// 				funding_asset: AcceptedFundingAsset::USDT,
// 				receiving_account: Junction::AccountId32 { network: None, id: receiving_account_polimec_id_bytes },
// 				signature_bytes,
// 			});
// 		let encoded_bid_call: DoubleEncoded<polimec_runtime::RuntimeCall> = bid_call.encode().into();

// 		let fee_asset_id = Location::parent().into(); // Relay chain native token for fees
// 		let fees = 1_000_000_000; // From the other XCM test

// 		assert_ok!(AssetHubXcmPallet::send(
// 			AssetHubOrigin::signed(AssetHubWestendNet::account_id_of(ALICE)),
// 			Box::new(VersionedLocation::V4(polimec_location())),
// 			Box::new(VersionedXcm::V4(Xcm(vec![
// 				Instruction::BuyExecution {
// 					fees: Asset { id: fee_asset_id, fun: Fungibility::Fungible(fees) }.into(),
// 					weight_limit: WeightLimit::Unlimited,
// 				},
// 				Instruction::Transact {
// 					origin_kind: OriginKind::SovereignAccount,
// 					require_weight_at_most: Weight::MAX,
// 					call: encoded_bid_call,
// 				},
// 			]))),
// 		));

// 		assert!(AssetHubWestendNet::events()
// 			.iter()
// 			.any(|event| matches!(event, AssetHubEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }))));
// 	});

// 	// Polimec Verification Part 2
// 	PolimecNet::execute_with(|| {
// 		let alice_sovereign_on_polimec = alice_sovereign_on_polimec_holder.unwrap();
// 		let project_id = project_id_holder;

// 		let ct_unit = get_ct_unit_from_metadata();
// 		let expected_ct_amount_bid = 500 * ct_unit;

// 		assert!(
// 			PolimecNet::events().iter().any(|event| {
// 				matches!(event, PolimecEvent::MessageQueue(pallet_message_queue::Event::Success { .. }))
// 			}),
// 			"XCM execution failed on Polimec"
// 		);

// 		let bid_id = PolimecNet::events()
// 			.iter()
// 			.find_map(|event| match event {
// 				PolimecEvent::Funding(pallet_funding::Event::Bid {
// 					project_id: pid,
// 					bidder,
// 					ct_amount,
// 					id: bid_id,
// 					..
// 				}) if *pid == project_id &&
// 					*bidder == alice_sovereign_on_polimec &&
// 					*ct_amount == expected_ct_amount_bid =>
// 					Some(*bid_id),
// 				_ => None,
// 			})
// 			.expect("Expected BidPlaced event with correct parameters not found");

// 		let bid_details = pallet_funding::Bids::<PolimecRuntime>::get(project_id, bid_id).unwrap();
// 		assert_eq!(bid_details.bidder, alice_sovereign_on_polimec);
// 		assert_eq!(bid_details.original_ct_amount, expected_ct_amount_bid);
// 		assert_eq!(bid_details.mode, ParticipationMode::OTM);

// 		let bid_value_usd_float = 500.0 * 10.0; // 500 CTs * min_price 10 USD/CT
// 		let plmc_price_float = {
// 			// Convert FixedU128 to f64
// 			let val = plmc_price().into_inner();
// 			(val as f64) / (FixedU128::accuracy() as f64)
// 		};

// 		let expected_plmc_bond_float = (bid_value_usd_float / plmc_price_float);
// 		let expected_plmc_bond = FixedU128::from_float(expected_plmc_bond_float).saturating_mul_int(PLMC);

// 		let proxy_bonding_account = polimec_runtime::ProxyBonding::get_bonding_account(project_id);
// 		let funding_hold_reason_runtime: <PolimecRuntime as pallet_balances::Config>::RuntimeHoldReason =
// 			FundingHoldReason::Participation.into();
// 		let plmc_on_hold_in_proxy = Balances::balance_on_hold(funding_hold_reason_runtime, &proxy_bonding_account);

// 		let tolerance = Perquintill::from_percent(1);
// 		let lower_bound = expected_plmc_bond.saturating_sub(tolerance.mul_ceil(expected_plmc_bond));
// 		let upper_bound = expected_plmc_bond.saturating_add(tolerance.mul_ceil(expected_plmc_bond));
// 		assert!(
// 			plmc_on_hold_in_proxy >= lower_bound && plmc_on_hold_in_proxy <= upper_bound,
// 			"PLMC on hold {} not within 1% tolerance of expected {} (range: [{}, {}])",
// 			plmc_on_hold_in_proxy,
// 			expected_plmc_bond,
// 			lower_bound,
// 			upper_bound
// 		);
// 	});
// }
