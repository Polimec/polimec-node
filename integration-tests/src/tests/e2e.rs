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

use crate::{tests::defaults::*, *};
use frame_support::{
	traits::{
		fungible::Mutate,
		fungibles::{metadata::Inspect, Inspect as OtherInspect},
	},
	BoundedVec,
};
use itertools::Itertools;
use macros::generate_accounts;
use pallet_funding::{traits::ProvideAssetPrice, *};
use polimec_common::{USD_DECIMALS, USD_UNIT};
use sp_arithmetic::{traits::Zero, Percent, Perquintill};
use sp_runtime::{FixedPointNumber, FixedU128};
use xcm_emulator::log;

type UserToCTBalance = Vec<(AccountId, FixedU128, ProjectId)>;

generate_accounts!(
	LINA, MIA, ALEXEY, PAUL, MARIA, GEORGE, CLARA, RAMONA, PASCAL, EMMA, BIBI, AHMED, HERBERT, LENI, XI, TOM, ADAMS,
	POLK, MARKUS, ELLA, SKR, ARTHUR, MILA, LINCOLN, MONROE, ARBRESHA, ELDIN, HARDING, SOFIA, DOMINIK, NOLAND, HANNAH,
	HOOVER, GIGI, JEFFERSON, LINDI, KEVIN, ANIS, RETO, HAALAND, XENIA, EVA, SKARA, ROOSEVELT, DRACULA, DURIM, HARRISON,
	PARI, TUTI, BENITO, VANESSA, ENES, RUDOLF, CERTO, TIESTO, DAVID, ATAKAN, YANN, ENIS, ALFREDO, QENDRIM, LEONARDO,
	KEN, LUCA, FLAVIO, FREDI, ALI, DILARA, DAMIAN, KAYA, IAZI, CHRIGI, VALENTINA, ALMA, ALENA, PATRICK, ONTARIO, RAKIA,
	HUBERT, UTUS, TOME, ZUBER, ADAM, STANI, BETI, HALIT, DRAGAN, LEA, LUIS, TATI, WEST, MIRIJAM, LIONEL, GIOVANNI,
	JOEL, POLKA, MALIK, ALEXANDER, SOLOMUN, JOHNNY, GRINGO, JONAS, BUNDI, FELIX,
);

pub fn excel_project() -> ProjectMetadataOf<PolitestRuntime> {
	let bounded_name = BoundedVec::try_from("Polimec".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("PLMC".as_bytes().to_vec()).unwrap();
	let metadata_hash = ipfs_hash();
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
		mainnet_token_max_supply: 10_000_000 * CT_UNIT, // Made up, not in the Sheet.
		// Total Allocation of Contribution Tokens Available for the Funding Round
		total_allocation_size: 100_000 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),

		// Minimum Price per Contribution Token (in USDT)
		minimum_price: PriceProviderOf::<PolitestRuntime>::calculate_decimals_aware_price(
			PriceOf::<PolitestRuntime>::from_float(10.0f64),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap(),
		bidding_ticket_sizes: BiddingTicketSizes {
			professional: TicketSize::new(5000 * USD_UNIT, None),
			institutional: TicketSize::new(5000 * USD_UNIT, None),
			phantom: Default::default(),
		},
		contributing_ticket_sizes: ContributingTicketSizes {
			retail: TicketSize::new(USD_UNIT, None),
			professional: TicketSize::new(USD_UNIT, None),
			institutional: TicketSize::new(USD_UNIT, None),
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: ISSUER.into(),
		policy_ipfs_cid: Some(metadata_hash),
	}
}

fn excel_evaluators() -> Vec<UserToUSDBalance<PolitestRuntime>> {
	vec![
		(LINA.into(), 93754 * USD_UNIT).into(),
		(MIA.into(), 162 * USD_UNIT).into(),
		(ALEXEY.into(), 7454 * USD_UNIT).into(),
		(PAUL.into(), 8192 * USD_UNIT).into(),
		(MARIA.into(), 11131 * USD_UNIT).into(),
		(GEORGE.into(), 4765 * USD_UNIT).into(),
		(CLARA.into(), 4363 * USD_UNIT).into(),
		(RAMONA.into(), 4120 * USD_UNIT).into(),
		(PASCAL.into(), 1626 * USD_UNIT).into(),
		(EMMA.into(), 3996 * USD_UNIT).into(),
		(BIBI.into(), 3441 * USD_UNIT).into(),
		(AHMED.into(), 8048 * USD_UNIT).into(),
		(HERBERT.into(), 2538 * USD_UNIT).into(),
		(LENI.into(), 5803 * USD_UNIT).into(),
		(XI.into(), 1669 * USD_UNIT).into(),
		(TOM.into(), 6526 * USD_UNIT).into(),
	]
}

fn excel_bidders() -> Vec<BidParams<PolitestRuntime>> {
	vec![
		(ADAMS.into(), 700 * CT_UNIT).into(),
		(POLK.into(), 4000 * CT_UNIT).into(),
		(MARKUS.into(), 3000 * CT_UNIT).into(),
		(ELLA.into(), 700 * CT_UNIT).into(),
		(SKR.into(), 3400 * CT_UNIT).into(),
		(ARTHUR.into(), 1000 * CT_UNIT).into(),
		(MILA.into(), 8400 * CT_UNIT).into(),
		(LINCOLN.into(), 800 * CT_UNIT).into(),
		(MONROE.into(), 1300 * CT_UNIT).into(),
		(ARBRESHA.into(), 5000 * CT_UNIT).into(),
		(ELDIN.into(), 600 * CT_UNIT).into(),
		(HARDING.into(), 800 * CT_UNIT).into(),
		(SOFIA.into(), 3000 * CT_UNIT).into(),
		(DOMINIK.into(), 8000 * CT_UNIT).into(),
		(NOLAND.into(), 900 * CT_UNIT).into(),
		(LINA.into(), 8400 * CT_UNIT).into(),
		(LINA.into(), 1000 * CT_UNIT).into(),
		(HANNAH.into(), 500 * CT_UNIT).into(),
		(HOOVER.into(), 1900 * CT_UNIT).into(),
		(GIGI.into(), 600 * CT_UNIT).into(),
		(JEFFERSON.into(), 1000 * CT_UNIT).into(),
		(JEFFERSON.into(), 2000 * CT_UNIT).into(),
	]
}

fn excel_contributions() -> Vec<ContributionParams<PolitestRuntime>> {
	vec![
		(XI.into(), 692 * CT_UNIT).into(),
		(PARI.into(), 236 * CT_UNIT).into(),
		(TUTI.into(), 24 * CT_UNIT).into(),
		(BENITO.into(), 688 * CT_UNIT).into(),
		(VANESSA.into(), 33 * CT_UNIT).into(),
		(ENES.into(), 1148 * CT_UNIT).into(),
		(RUDOLF.into(), 35 * CT_UNIT).into(),
		(CERTO.into(), 840 * CT_UNIT).into(),
		(TIESTO.into(), 132 * CT_UNIT).into(),
		(DAVID.into(), 21 * CT_UNIT).into(),
		(ATAKAN.into(), 59 * CT_UNIT).into(),
		(YANN.into(), 89 * CT_UNIT).into(),
		(ENIS.into(), 332 * CT_UNIT).into(),
		(ALFREDO.into(), 8110 * CT_UNIT).into(),
		(QENDRIM.into(), 394 * CT_UNIT).into(),
		(LEONARDO.into(), 840 * CT_UNIT).into(),
		(KEN.into(), 352 * CT_UNIT).into(),
		(LUCA.into(), 640 * CT_UNIT).into(),
		(FLAVIO.into(), 792 * CT_UNIT).into(),
		(FREDI.into(), 993 * CT_UNIT).into(),
		(ALI.into(), 794 * CT_UNIT).into(),
		(DILARA.into(), 256 * CT_UNIT).into(),
		(DAMIAN.into(), 431 * CT_UNIT).into(),
		(KAYA.into(), 935 * CT_UNIT).into(),
		(IAZI.into(), 174 * CT_UNIT).into(),
		(CHRIGI.into(), 877 * CT_UNIT).into(),
		(VALENTINA.into(), 961 * CT_UNIT).into(),
		(ALMA.into(), 394 * CT_UNIT).into(),
		(ALENA.into(), 442 * CT_UNIT).into(),
		(PATRICK.into(), 486 * CT_UNIT).into(),
		(ONTARIO.into(), 17 * CT_UNIT).into(),
		(RAKIA.into(), 9424 * CT_UNIT).into(),
		(HUBERT.into(), 14 * CT_UNIT).into(),
		(UTUS.into(), 4906 * CT_UNIT).into(),
		(TOME.into(), 68 * CT_UNIT).into(),
		(ZUBER.into(), 9037 * CT_UNIT).into(),
		(ADAM.into(), 442 * CT_UNIT).into(),
		(STANI.into(), 40 * CT_UNIT).into(),
		(BETI.into(), 68 * CT_UNIT).into(),
		(HALIT.into(), 68 * CT_UNIT).into(),
		(DRAGAN.into(), 98 * CT_UNIT).into(),
		(LEA.into(), 17 * CT_UNIT).into(),
		(LUIS.into(), 422 * CT_UNIT).into(),
	]
}
fn excel_remainders() -> Vec<ContributionParams<PolitestRuntime>> {
	vec![
		(JOEL.into(), 692 * CT_UNIT).into(),
		(POLK.into(), 236 * CT_UNIT).into(),
		(MALIK.into(), 24 * CT_UNIT).into(),
		(LEA.into(), 688 * CT_UNIT).into(),
		(RAMONA.into(), 35 * CT_UNIT).into(),
		(SOLOMUN.into(), 840 * CT_UNIT).into(),
		(JONAS.into(), 59 * CT_UNIT).into(),
	]
}
fn excel_ct_amounts() -> UserToCTBalance {
	vec![
		(LINA.into(), FixedU128::from_float(4292.3_120_710_000f64), 0),
		(MIA.into(), FixedU128::from_float(3.2_697_757_490f64), 0),
		(ALEXEY.into(), FixedU128::from_float(142.2_854_836_000f64), 0),
		(PAUL.into(), FixedU128::from_float(116.5_251_535_000f64), 0),
		(MARIA.into(), FixedU128::from_float(158.3_302_593_000f64), 0),
		(GEORGE.into(), FixedU128::from_float(67.7_786_079_900f64), 0),
		(CLARA.into(), FixedU128::from_float(62.0_604_547_000f64), 0),
		(RAMONA.into(), FixedU128::from_float(93.6_039_590_600f64), 0),
		(PASCAL.into(), FixedU128::from_float(23.1_286_498_600f64), 0),
		(EMMA.into(), FixedU128::from_float(56.8_401_505_800f64), 0),
		(BIBI.into(), FixedU128::from_float(48.9_456_852_200f64), 0),
		(AHMED.into(), FixedU128::from_float(114.4_768_598_000f64), 0),
		(HERBERT.into(), FixedU128::from_float(36.1_011_767_200f64), 0),
		(LENI.into(), FixedU128::from_float(82.5_433_918_500f64), 0),
		(XI.into(), FixedU128::from_float(715.7_402_931_000f64), 0),
		(TOM.into(), FixedU128::from_float(92.8_275_332_100f64), 0),
		(ADAMS.into(), FixedU128::from_float(700f64), 0),
		(POLK.into(), FixedU128::from_float(4236f64), 0),
		(MARKUS.into(), FixedU128::from_float(3000f64), 0),
		(ELLA.into(), FixedU128::from_float(700f64), 0),
		(SKR.into(), FixedU128::from_float(3400f64), 0),
		(ARTHUR.into(), FixedU128::from_float(1000f64), 0),
		(MILA.into(), FixedU128::from_float(8400f64), 0),
		(LINCOLN.into(), FixedU128::from_float(800f64), 0),
		(MONROE.into(), FixedU128::from_float(1300f64), 0),
		(ARBRESHA.into(), FixedU128::from_float(5000f64), 0),
		(ELDIN.into(), FixedU128::from_float(600f64), 0),
		(HARDING.into(), FixedU128::from_float(800f64), 0),
		(SOFIA.into(), FixedU128::from_float(3000f64), 0),
		(DOMINIK.into(), FixedU128::from_float(8000f64), 0),
		(NOLAND.into(), FixedU128::from_float(900f64), 0),
		(HANNAH.into(), FixedU128::from_float(500f64), 0),
		(HOOVER.into(), FixedU128::from_float(1900f64), 0),
		(GIGI.into(), FixedU128::from_float(600f64), 0),
		(JEFFERSON.into(), FixedU128::from_float(3000f64), 0),
		(PARI.into(), FixedU128::from_float(236f64), 0),
		(TUTI.into(), FixedU128::from_float(24f64), 0),
		(BENITO.into(), FixedU128::from_float(688f64), 0),
		(VANESSA.into(), FixedU128::from_float(33f64), 0),
		(ENES.into(), FixedU128::from_float(1148f64), 0),
		(RUDOLF.into(), FixedU128::from_float(35f64), 0),
		(CERTO.into(), FixedU128::from_float(840f64), 0),
		(TIESTO.into(), FixedU128::from_float(132f64), 0),
		(DAVID.into(), FixedU128::from_float(21f64), 0),
		(ATAKAN.into(), FixedU128::from_float(59f64), 0),
		(YANN.into(), FixedU128::from_float(89f64), 0),
		(ENIS.into(), FixedU128::from_float(332f64), 0),
		(ALFREDO.into(), FixedU128::from_float(8110f64), 0),
		(QENDRIM.into(), FixedU128::from_float(394f64), 0),
		(LEONARDO.into(), FixedU128::from_float(840f64), 0),
		(KEN.into(), FixedU128::from_float(352f64), 0),
		(LUCA.into(), FixedU128::from_float(640f64), 0),
		(FLAVIO.into(), FixedU128::from_float(792f64), 0),
		(FREDI.into(), FixedU128::from_float(993f64), 0),
		(ALI.into(), FixedU128::from_float(794f64), 0),
		(DILARA.into(), FixedU128::from_float(256f64), 0),
		(DAMIAN.into(), FixedU128::from_float(431f64), 0),
		(KAYA.into(), FixedU128::from_float(935f64), 0),
		(IAZI.into(), FixedU128::from_float(174f64), 0),
		(CHRIGI.into(), FixedU128::from_float(877f64), 0),
		(VALENTINA.into(), FixedU128::from_float(961f64), 0),
		(ALMA.into(), FixedU128::from_float(394f64), 0),
		(ALENA.into(), FixedU128::from_float(442f64), 0),
		(PATRICK.into(), FixedU128::from_float(486f64), 0),
		(ONTARIO.into(), FixedU128::from_float(17f64), 0),
		(RAKIA.into(), FixedU128::from_float(9424f64), 0),
		(HUBERT.into(), FixedU128::from_float(14f64), 0),
		(UTUS.into(), FixedU128::from_float(4906f64), 0),
		(TOME.into(), FixedU128::from_float(68f64), 0),
		(ZUBER.into(), FixedU128::from_float(9037f64), 0),
		(ADAM.into(), FixedU128::from_float(442f64), 0),
		(STANI.into(), FixedU128::from_float(40f64), 0),
		(BETI.into(), FixedU128::from_float(68f64), 0),
		(HALIT.into(), FixedU128::from_float(68f64), 0),
		(DRAGAN.into(), FixedU128::from_float(98f64), 0),
		(LEA.into(), FixedU128::from_float(705f64), 0),
		(LUIS.into(), FixedU128::from_float(422f64), 0),
		(JOEL.into(), FixedU128::from_float(692f64), 0),
		(MALIK.into(), FixedU128::from_float(24f64), 0),
		(SOLOMUN.into(), FixedU128::from_float(840f64), 0),
		(JONAS.into(), FixedU128::from_float(59f64), 0),
	]
}

#[test]
fn evaluation_round_completed() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	let issuer = ISSUER.into();
	let project = excel_project();
	let evaluations = excel_evaluators();

	PolitestNet::execute_with(|| {
		inst.create_auctioning_project(project, issuer, evaluations);
	});
}

#[test]
fn auction_round_completed() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	let issuer = ISSUER.into();
	let project = excel_project();
	let evaluations = excel_evaluators();
	let bids = excel_bidders();

	PolitestNet::execute_with(|| {
		let project_id = inst.create_community_contributing_project(project, issuer, evaluations, bids);
		let excel_wap_fixed = FixedU128::from_float(10.202357561f64);
		let excel_wap_usd = excel_wap_fixed.saturating_mul_int(USD_UNIT);

		let stored_wap_fixed = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let stored_wap_fixed_decimal_unaware = PriceProviderOf::<PolitestRuntime>::convert_back_to_normal_price(
			stored_wap_fixed,
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap();
		let stored_wap_usd = stored_wap_fixed_decimal_unaware.saturating_mul_int(USD_UNIT);

		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.00001.
		assert_close_enough!(stored_wap_usd, excel_wap_usd, Perquintill::from_float(0.999));
		let names = names();
		inst.execute(|| {
			let bids =
				Bids::<PolitestRuntime>::iter_prefix_values((0,)).sorted_by_key(|bid| bid.bidder.clone()).collect_vec();

			for bid in bids.clone() {
				let key: [u8; 32] = bid.bidder.clone().into();
				println!("{}: {}", names[&key], bid.funding_asset_amount_locked);
			}
		})
	});
}

#[test]
fn community_round_completed() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	PolitestNet::execute_with(|| {
		let _ = inst.create_remainder_contributing_project(
			excel_project(),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
		);

		inst.execute(|| {
			let contributions = Contributions::<PolitestRuntime>::iter_prefix_values((0,))
				.sorted_by_key(|bid| bid.contributor.clone())
				.collect_vec();
			let _total_contribution =
				contributions.clone().into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
			// TODO: add test for exact amount contributed
		})
	});
}

#[test]
fn remainder_round_completed() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	PolitestNet::execute_with(|| {
		inst.create_finished_project(
			excel_project(),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);

		let contributions = Contributions::<PolitestRuntime>::iter_prefix_values((0,))
			.sorted_by_key(|contribution| contribution.contributor.clone())
			.collect_vec();
		let total_stored =
			contributions.into_iter().fold(0, |acc, contribution| acc + contribution.funding_asset_amount);

		let usdt_decimals = <PolitestRuntime as pallet_funding::Config>::FundingCurrency::decimals(
			AcceptedFundingAsset::USDT.to_assethub_id(),
		);
		let usdt_total_from_excel_f64 = 503_945.4_517_000_000f64;
		let usdt_total_from_excel_fixed = FixedU128::from_float(usdt_total_from_excel_f64);
		let usdt_total_from_excel = usdt_total_from_excel_fixed.saturating_mul_int(10u128.pow(usdt_decimals as u32));

		assert_close_enough!(total_stored, usdt_total_from_excel, Perquintill::from_float(0.999));
	});
}

#[test]
fn funds_raised() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	PolitestNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);

		inst.execute(|| {
			let project_specific_account: AccountId = PolitestFundingPallet::fund_account_id(project_id);
			let stored_usdt_funded =
				PolitestForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), project_specific_account);
			let excel_usdt_funded_f64 = 1_004_256.0_140_000_000f64;
			let excet_usdt_funding_fixed = FixedU128::from_float(excel_usdt_funded_f64);
			let usdt_decimals = <PolitestRuntime as pallet_funding::Config>::FundingCurrency::decimals(
				AcceptedFundingAsset::USDT.to_assethub_id(),
			);
			let excel_usdt_funded = excet_usdt_funding_fixed.saturating_mul_int(10u128.pow(usdt_decimals as u32));
			assert_close_enough!(stored_usdt_funded, excel_usdt_funded, Perquintill::from_float(0.99));
		})
	});
}

#[test]
fn ct_minted() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	PolitestNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);
		inst.advance_time(<PolitestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.settle_project(project_id).unwrap();

		for (contributor, expected_amount_fixed, project_id) in excel_ct_amounts() {
			let minted = inst
				.execute(|| <PolitestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
			let expected_amount = expected_amount_fixed.saturating_mul_int(CT_UNIT);
			assert_close_enough!(minted, expected_amount, Perquintill::from_float(0.99));
		}
	});
}

#[test]
fn ct_migrated() {
	politest::set_prices();

	let mut inst = IntegrationInstantiator::new(None);

	let project_id = PolitestNet::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);
		inst.advance_time(<PolitestRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.settle_project(project_id).unwrap();

		for (contributor, expected_amount_fixed, project_id) in excel_ct_amounts() {
			let minted = inst
				.execute(|| <PolitestRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
			let expected_amount = expected_amount_fixed.saturating_mul_int(CT_UNIT);
			assert_close_enough!(minted, expected_amount, Perquintill::from_float(0.99));
		}

		project_id
	});

	let project_details = PolitestNet::execute_with(|| inst.get_project_details(project_id));
	assert!(matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Rewarded(_)));
	let ct_issued = PolitestNet::execute_with(|| {
		<PolitestRuntime as Config>::ContributionTokenCurrency::total_issuance(project_id)
	});

	PenNet::execute_with(|| {
		let polimec_sovereign_account =
			<Penpal<PolkadotNet>>::sovereign_account_id_of((Parent, xcm::prelude::Parachain(polimec::PARA_ID)).into());
		PenpalBalances::set_balance(&polimec_sovereign_account, ct_issued + inst.get_ed());
	});

	// Mock HRMP establishment
	PolitestNet::execute_with(|| {
		let _account_id: PolitestAccountId = ISSUER.into();
		assert_ok!(PolitestFundingPallet::do_set_para_id_for_project(
			&ISSUER.into(),
			project_id,
			ParaId::from(6969u32),
		));
		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolitestFundingPallet::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolitestFundingPallet::do_handle_channel_accepted(channel_accepted_message));
	});

	PenNet::execute_with(|| {
		println!("penpal events:");
		dbg!(PenNet::events());
	});

	// Migration is ready
	PolitestNet::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolitestRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	excel_ct_amounts().iter().map(|tup| tup.0.clone()).unique().for_each(|account| {
		let data = PenNet::account_data_of(account.clone());
		assert_eq!(data.free, 0u128, "Participant balances should be 0 before ct migration");
	});

	// Migrate CTs
	let accounts = excel_ct_amounts().iter().map(|item| item.0.clone()).unique().collect::<Vec<_>>();
	let total_ct_sold = excel_ct_amounts().iter().fold(FixedU128::zero(), |acc, item| acc + item.1);
	dbg!(total_ct_sold);
	let polimec_sov_acc = PenNet::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into());
	let polimec_fund_balance = PenNet::account_data_of(polimec_sov_acc);
	dbg!(polimec_fund_balance);

	let names = names();

	for account in accounts {
		PolitestNet::execute_with(|| {
			assert_ok!(PolitestFundingPallet::migrate_one_participant(
				PolitestOrigin::signed(account.clone()),
				project_id,
				account.clone()
			));
			let key: [u8; 32] = account.clone().into();
			println!("Migrated CTs for {}", names[&key]);
			inst.advance_time(1u32).unwrap();
		});
	}

	PenNet::execute_with(|| {
		dbg!(PenNet::events());
	});

	let total_ct_amounts = excel_ct_amounts().iter().fold(FixedU128::zero(), |acc, item| acc + item.1);
	log::info!("excel ct amounts total: {}", total_ct_amounts);
	// Check balances after migration, before vesting
	excel_ct_amounts().iter().for_each(|item| {
		let data = PenNet::account_data_of(item.0.clone());
		let key: [u8; 32] = item.0.clone().into();
		println!("Participant {} has {} CTs. Expected {}", names[&key], data.free.clone(), item.1);
		dbg!(data.clone());
		let amount_as_balance = item.1.saturating_mul_int(CT_UNIT);
		assert_close_enough!(
			data.free,
			amount_as_balance,
			Perquintill::from_float(0.99),
			"Participant balances should be transfered to each account after ct migration"
		);
	});
}
