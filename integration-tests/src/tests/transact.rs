use super::e2e::{BOB, DAVE};
use crate::{
	asset_hub, polimec,
	tests::defaults::*,
	tests::e2e::{dot_price, plmc_price, project_metadata, usdc_price, usdt_price},
	AssetHubEvent, AssetHubOrigin, AssetHubRuntime, AssetHubWestendNet, AssetHubXcmPallet, PolimecAccountId,
	PolimecBalances, PolimecEvent, PolimecForeignAssets, PolimecFunding, PolimecNet, PolimecRuntime, ALICE,
};
use frame_support::traits::fungible::InspectHold;
use pallet_funding::{HoldReason as FundingHoldReason, ParticipationMode, ProjectStatus};
use parity_scale_codec::Encode;
use polimec_common::credentials::{InvestorType, UntrustedToken};
use polimec_common::{assets::AcceptedFundingAsset, USD_UNIT};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};
use polimec_runtime::{Balances, RuntimeOrigin as RawOrigin};
use sp_runtime::{traits::Hash, FixedU128, Perquintill};
use xcm::{v4::prelude::*, DoubleEncoded, VersionedLocation, VersionedXcm};
use xcm_emulator::{assert_ok, Chain, ConvertLocation, TestExt};

pub(crate) fn polimec_location() -> Location {
	Location::new(1, [Parachain(polimec::PARA_ID)])
}

fn get_ct_unit_from_metadata() -> u128 {
	CT_UNIT
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
