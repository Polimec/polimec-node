use crate::{
	asset_hub,
	constants::PricesBuilder,
	polimec,
	tests::{
		defaults::*,
		e2e::{dot_price, evaluations, plmc_price, project_metadata, usdc_price, usdt_price},
		evaluator_slash_sideffects::BOB,
		transact::polimec_location,
	},
	AssetHubEvent, AssetHubOrigin, AssetHubRuntime, AssetHubSystem, AssetHubWestendNet, AssetHubXcmPallet,
	PolimecAccountId, PolimecBalances, PolimecEvent, PolimecForeignAssets, PolimecFunding, PolimecNet, PolimecOrigin,
	PolimecParachainSystem, PolimecRuntime, PolkaNet, PolkadotNet, PolkadotRelay, PolkadotSystem, ALICE,
};
use cumulus_pallet_parachain_system::RelaychainDataProvider;
use pallet_funding::{ParticipationMode, ProjectStatus};
use parity_scale_codec::Encode;
use polimec_common::{
	assets::AcceptedFundingAsset::USDT,
	credentials::{Institutional, InvestorType, UntrustedToken},
};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt, get_mock_jwt_with_cid};
use polimec_runtime::{Balances, RuntimeOrigin as RawOrigin, PLMC};
use sp_runtime::{traits::BlockNumberProvider, FixedPointNumber};
use sp_runtime::{traits::Convert, FixedU128};
use xcm::{v4::prelude::*, DoubleEncoded, VersionedLocation, VersionedXcm};
use xcm_emulator::{assert_ok, Chain, TestExt};

#[test]
fn transact_bid_with_receiving_account_from_ah() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer: PolimecAccountId = ISSUER.into();
	let plmc_ed = inst.get_ed();

	polimec::set_prices(
		PricesBuilder::new()
			.usdt(usdt_price().into())
			.usdc(usdc_price().into())
			.dot(dot_price().into())
			.plmc(plmc_price().into())
			.build(),
	);

	PolimecNet::execute_with(|| {
		let project_id = inst.create_new_project(project_metadata(), issuer.clone(), None);
		let issuer_jwt =
			get_mock_jwt(issuer.clone(), InvestorType::Institutional, generate_did_from_account(issuer.clone()));

		PolimecFunding::start_evaluation(PolimecOrigin::signed(issuer.clone()), issuer_jwt.clone(), project_id)
			.unwrap();

		for (user, investor_type, _usd_bond, plmc_bonded) in evaluations() {
			let user: PolimecAccountId = user.into();
			let plmc_bonded: u128 = FixedU128::from_float(plmc_bonded).saturating_mul_int(PLMC);

			let user_jwt = get_mock_jwt_with_cid(
				user.clone(),
				investor_type,
				generate_did_from_account(user.clone()),
				ipfs_hash(),
			);

			// We add 1 PLMC to the mint to avoid rounding errors, and add ED to keep the account alive.
			inst.mint_plmc_to(vec![(user.clone(), plmc_bonded + PLMC + plmc_ed).into()]);

			PolimecFunding::evaluate(PolimecOrigin::signed(user.clone()), user_jwt, project_id, plmc_bonded).unwrap();
		}
	});

	PolkaNet::execute_with(|| {
		PolkadotSystem::set_block_number(11);
	});

	PolimecNet::execute_with(|| {
		assert_eq!(inst.go_to_next_state(0), ProjectStatus::AuctionRound);
	});

	AssetHubWestendNet::execute_with(|| {
		let alice_sovereign_on_polimec = AssetHubWestendNet::account_id_of(ALICE);

		let project_id = 0;

		// TODO: use valid address
		let receiving_account = Junction::AccountKey20 { network: Some(Ethereum { chain_id: 1 }), key: [0u8; 20] };

		let jwt: UntrustedToken = get_mock_jwt_with_cid(
			alice_sovereign_on_polimec.clone(),
			InvestorType::Retail,
			generate_did_from_account(alice_sovereign_on_polimec.clone()),
			ipfs_hash(),
		)
		.into();
		let signature_bytes = [0u8; 65];
		let funding_asset_amount = 100_000_000;
		let funding_asset = polimec_common::assets::AcceptedFundingAsset::USDC;

		let bid_call =
			polimec_runtime::RuntimeCall::Funding(pallet_funding::Call::<PolimecRuntime>::bid_with_receiving_account {
				jwt,
				project_id,
				funding_asset_amount,
				mode: ParticipationMode::Classic(1),
				funding_asset,
				receiving_account,
				signature_bytes,
			});
		let encoded_bid_call: DoubleEncoded<()> = bid_call.encode().into();

		let fee_asset_id = Location::parent().into();
		let fees = 1_000_000_000;

		assert_ok!(AssetHubXcmPallet::send(
			AssetHubOrigin::signed(AssetHubWestendNet::account_id_of(ALICE)),
			Box::new(VersionedLocation::V4(polimec_location())),
			Box::new(VersionedXcm::V4(Xcm(vec![
				Instruction::BuyExecution {
					fees: Asset { id: fee_asset_id, fun: Fungibility::Fungible(fees) }.into(),
					weight_limit: WeightLimit::Unlimited,
				},
				Instruction::Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::MAX,
					call: encoded_bid_call,
				},
			]))),
		));

		assert!(AssetHubWestendNet::events()
			.iter()
			.any(|event| matches!(event, AssetHubEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }))));
	});

	PolimecNet::execute_with(|| {
		println!("what here");

		let alice_sovereign_on_polimec = AssetHubWestendNet::account_id_of(ALICE);
		let project_id = 0;

		assert!(
			PolimecNet::events().iter().any(|event| {
				matches!(event, PolimecEvent::MessageQueue(pallet_message_queue::Event::Processed { .. }))
			}),
			"XCM execution failed on Polimec"
		);

		for event in PolimecNet::events() {
			println!("event {:?}", event);
		}
		let bid_id = PolimecNet::events()
			.iter()
			.find_map(|event| match event {
				PolimecEvent::Funding(pallet_funding::Event::Bid {
					project_id: pid,
					bidder,
					ct_amount: _,
					id: bid_id,
					..
				}) if *pid == project_id && *bidder == alice_sovereign_on_polimec => Some(*bid_id),
				_ => None,
			})
			.expect("Expected BidPlaced event with correct parameters not found");

		let bid_details = pallet_funding::Bids::<PolimecRuntime>::get(project_id, bid_id).unwrap();
		assert_eq!(bid_details.bidder, alice_sovereign_on_polimec);
		assert_eq!(bid_details.mode, ParticipationMode::Classic(1));

		println!("what here");
	});
}
