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
use frame_support::BoundedVec;
use itertools::Itertools;
use macros::generate_accounts;
use pallet_funding::*;
use polimec_parachain_runtime::{PolimecFunding, US_DOLLAR};
use sp_arithmetic::{FixedPointNumber, Perquintill};
use sp_runtime::{traits::CheckedSub, FixedU128};

type UserToCTBalance = Vec<(AccountId, BalanceOf<PolimecRuntime>, ProjectId)>;

generate_accounts!(
	LINA, MIA, ALEXEY, PAUL, MARIA, GEORGE, CLARA, RAMONA, PASCAL, EMMA, BIBI, AHMED, HERBERT, LENI, XI, TOM, ADAMS,
	POLK, MARKUS, ELLA, SKR, ARTHUR, MILA, LINCOLN, MONROE, ARBRESHA, ELDIN, HARDING, SOFIA, DOMINIK, NOLAND, HANNAH,
	HOOVER, GIGI, JEFFERSON, LINDI, KEVIN, ANIS, RETO, HAALAND, XENIA, EVA, SKARA, ROOSEVELT, DRACULA, DURIM, HARRISON,
	DRIN, PARI, TUTI, BENITO, VANESSA, ENES, RUDOLF, CERTO, TIESTO, DAVID, ATAKAN, YANN, ENIS, ALFREDO, QENDRIM,
	LEONARDO, KEN, LUCA, FLAVIO, FREDI, ALI, DILARA, DAMIAN, KAYA, IAZI, CHRIGI, VALENTINA, ALMA, ALENA, PATRICK,
	ONTARIO, RAKIA, HUBERT, UTUS, TOME, ZUBER, ADAM, STANI, BETI, HALIT, DRAGAN, LEA, LUIS, TATI, WEST, MIRIJAM,
	LIONEL, GIOVANNI, JOEL, POLKA, MALIK, ALEXANDER, SOLOMUN, JOHNNY, GRINGO, JONAS, BUNDI, FELIX,
);

pub fn excel_project(nonce: u64) -> ProjectMetadataOf<PolimecRuntime> {
	let bounded_name = BoundedVec::try_from("Polimec".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("PLMC".as_bytes().to_vec()).unwrap();
	let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: 10 },
		mainnet_token_max_supply: 1_000_000_0_000_000_000, // Made up, not in the Sheet.
		// Total Allocation of Contribution Tokens Available for the Funding Round
		total_allocation_size: (50_000_0_000_000_000, 50_000_0_000_000_000),
		// Minimum Price per Contribution Token (in USDT)
		minimum_price: PriceOf::<PolimecRuntime>::from(10),
		ticket_size: TicketSize { minimum: Some(1), maximum: None },
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		funding_thresholds: Default::default(),
		conversion_rate: 1,
		participation_currencies: AcceptedFundingAsset::USDT,
		funding_destination_account: ISSUER.into(),
		offchain_information_hash: Some(metadata_hash),
	}
}

fn excel_evaluators() -> Vec<UserToUSDBalance<PolimecRuntime>> {
	vec![
		UserToUSDBalance::new(LINA.into(), 93754 * US_DOLLAR),
		UserToUSDBalance::new(MIA.into(), 162 * US_DOLLAR),
		UserToUSDBalance::new(ALEXEY.into(), 7454 * US_DOLLAR),
		UserToUSDBalance::new(PAUL.into(), 8192 * US_DOLLAR),
		UserToUSDBalance::new(MARIA.into(), 11131 * US_DOLLAR),
		UserToUSDBalance::new(GEORGE.into(), 4765 * US_DOLLAR),
		UserToUSDBalance::new(CLARA.into(), 4363 * US_DOLLAR),
		UserToUSDBalance::new(RAMONA.into(), 4120 * US_DOLLAR),
		UserToUSDBalance::new(PASCAL.into(), 1626 * US_DOLLAR),
		UserToUSDBalance::new(EMMA.into(), 3996 * US_DOLLAR),
		UserToUSDBalance::new(BIBI.into(), 3441 * US_DOLLAR),
		UserToUSDBalance::new(AHMED.into(), 8048 * US_DOLLAR),
		UserToUSDBalance::new(HERBERT.into(), 2538 * US_DOLLAR),
		UserToUSDBalance::new(LENI.into(), 5803 * US_DOLLAR),
		UserToUSDBalance::new(XI.into(), 1669 * US_DOLLAR),
		UserToUSDBalance::new(TOM.into(), 6526 * US_DOLLAR),
	]
}

fn excel_bidders() -> Vec<BidParams<PolimecRuntime>> {
	vec![
		BidParams::from(ADAMS.into(), 700 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(POLK.into(), 4000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MARKUS.into(), 3000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ELLA.into(), 700 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(SKR.into(), 3400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ARTHUR.into(), 1000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MILA.into(), 8400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINCOLN.into(), 800 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MONROE.into(), 1300 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ARBRESHA.into(), 5000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ELDIN.into(), 600 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(HARDING.into(), 800 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(SOFIA.into(), 3000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(DOMINIK.into(), 8000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(NOLAND.into(), 900 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINA.into(), 8400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINA.into(), 1000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(HANNAH.into(), 400 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(HOOVER.into(), 2000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(GIGI.into(), 600 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(JEFFERSON.into(), 1000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(JEFFERSON.into(), 2000 * ASSET_UNIT, FixedU128::from_float(12.0)),
	]
}

fn excel_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::from(DRIN.into(), 692 * US_DOLLAR),
		ContributionParams::from(PARI.into(), 236 * US_DOLLAR),
		ContributionParams::from(TUTI.into(), 24 * US_DOLLAR),
		ContributionParams::from(BENITO.into(), 688 * US_DOLLAR),
		ContributionParams::from(VANESSA.into(), 33 * US_DOLLAR),
		ContributionParams::from(ENES.into(), 1148 * US_DOLLAR),
		ContributionParams::from(RUDOLF.into(), 35 * US_DOLLAR),
		ContributionParams::from(CERTO.into(), 840 * US_DOLLAR),
		ContributionParams::from(TIESTO.into(), 132 * US_DOLLAR),
		ContributionParams::from(DAVID.into(), 21 * US_DOLLAR),
		ContributionParams::from(ATAKAN.into(), 59 * US_DOLLAR),
		ContributionParams::from(YANN.into(), 89 * US_DOLLAR),
		ContributionParams::from(ENIS.into(), 332 * US_DOLLAR),
		ContributionParams::from(ALFREDO.into(), 8110 * US_DOLLAR),
		ContributionParams::from(QENDRIM.into(), 394 * US_DOLLAR),
		ContributionParams::from(LEONARDO.into(), 840 * US_DOLLAR),
		ContributionParams::from(KEN.into(), 352 * US_DOLLAR),
		ContributionParams::from(LUCA.into(), 640 * US_DOLLAR),
		// TODO: XI is a partipant in the Community Round AND an Evaluator. At the moment, this returns `InsufficientBalance` because it seems we don't mint to him enough USDT.
		// To be addressed and tested in a separate PR.
		//ContributionParams::from(XI, 588 * US_DOLLAR),
		ContributionParams::from(FLAVIO.into(), 792 * US_DOLLAR),
		ContributionParams::from(FREDI.into(), 993 * US_DOLLAR),
		ContributionParams::from(ALI.into(), 794 * US_DOLLAR),
		ContributionParams::from(DILARA.into(), 256 * US_DOLLAR),
		ContributionParams::from(DAMIAN.into(), 431 * US_DOLLAR),
		ContributionParams::from(KAYA.into(), 935 * US_DOLLAR),
		ContributionParams::from(IAZI.into(), 174 * US_DOLLAR),
		ContributionParams::from(CHRIGI.into(), 877 * US_DOLLAR),
		ContributionParams::from(VALENTINA.into(), 961 * US_DOLLAR),
		ContributionParams::from(ALMA.into(), 394 * US_DOLLAR),
		ContributionParams::from(ALENA.into(), 442 * US_DOLLAR),
		ContributionParams::from(PATRICK.into(), 486 * US_DOLLAR),
		ContributionParams::from(ONTARIO.into(), 17 * US_DOLLAR),
		ContributionParams::from(RAKIA.into(), 9424 * US_DOLLAR),
		ContributionParams::from(HUBERT.into(), 14 * US_DOLLAR),
		ContributionParams::from(UTUS.into(), 4906 * US_DOLLAR),
		ContributionParams::from(TOME.into(), 68 * US_DOLLAR),
		ContributionParams::from(ZUBER.into(), 9037 * US_DOLLAR),
		ContributionParams::from(ADAM.into(), 442 * US_DOLLAR),
		ContributionParams::from(STANI.into(), 40 * US_DOLLAR),
		ContributionParams::from(BETI.into(), 68 * US_DOLLAR),
		ContributionParams::from(HALIT.into(), 68 * US_DOLLAR),
		ContributionParams::from(DRAGAN.into(), 98 * US_DOLLAR),
		ContributionParams::from(LEA.into(), 17 * US_DOLLAR),
		ContributionParams::from(LUIS.into(), 422 * US_DOLLAR),
	]
}

fn excel_remainders() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::from(JOEL.into(), 692 * US_DOLLAR),
		ContributionParams::from(POLK.into(), 236 * US_DOLLAR),
		ContributionParams::from(MALIK.into(), 24 * US_DOLLAR),
		ContributionParams::from(LEA.into(), 688 * US_DOLLAR),
		ContributionParams::from(RAMONA.into(), 35 * US_DOLLAR),
		ContributionParams::from(SOLOMUN.into(), 840 * US_DOLLAR),
		ContributionParams::from(JONAS.into(), 59 * US_DOLLAR),
	]
}

fn excel_ct_amounts() -> UserToCTBalance {
	vec![
		(LINA.into(), 42916134112336, 0),
		(MIA.into(), 32685685157, 0),
		(ALEXEY.into(), 1422329504123, 0),
		(PAUL.into(), 1164821313204, 0),
		(MARIA.into(), 1582718022129, 0),
		(GEORGE.into(), 677535834646, 0),
		(CLARA.into(), 620375413759, 0),
		(RAMONA.into(), 935823219043, 0),
		(PASCAL.into(), 231201105380, 0),
		(EMMA.into(), 568191646431, 0),
		(BIBI.into(), 489276139982, 0),
		(AHMED.into(), 1144345938558, 0),
		(HERBERT.into(), 360878478139, 0),
		(LENI.into(), 825129160220, 0),
		(XI.into(), 237315279753, 0),
		(TOM.into(), 927932603756, 0),
		(ADAMS.into(), 700 * ASSET_UNIT, 0),
		(POLK.into(), 4236 * ASSET_UNIT, 0),
		(MARKUS.into(), 3000 * ASSET_UNIT, 0),
		(ELLA.into(), 700 * ASSET_UNIT, 0),
		(SKR.into(), 3400 * ASSET_UNIT, 0),
		(ARTHUR.into(), 1000 * ASSET_UNIT, 0),
		(MILA.into(), 8400 * ASSET_UNIT, 0),
		(LINCOLN.into(), 800 * ASSET_UNIT, 0),
		(MONROE.into(), 1300 * ASSET_UNIT, 0),
		(ARBRESHA.into(), 5000 * ASSET_UNIT, 0),
		(ELDIN.into(), 600 * ASSET_UNIT, 0),
		(HARDING.into(), 800 * ASSET_UNIT, 0),
		(SOFIA.into(), 3000 * ASSET_UNIT, 0),
		(DOMINIK.into(), 8000 * ASSET_UNIT, 0),
		(NOLAND.into(), 900 * ASSET_UNIT, 0),
		(HANNAH.into(), 400 * ASSET_UNIT, 0),
		(HOOVER.into(), 2000 * ASSET_UNIT, 0),
		(GIGI.into(), 600 * ASSET_UNIT, 0),
		(JEFFERSON.into(), 3000 * ASSET_UNIT, 0),
		(DRIN.into(), 692 * ASSET_UNIT, 0),
		(PARI.into(), 236 * ASSET_UNIT, 0),
		(TUTI.into(), 24 * ASSET_UNIT, 0),
		(BENITO.into(), 688 * ASSET_UNIT, 0),
		(VANESSA.into(), 33 * ASSET_UNIT, 0),
		(ENES.into(), 1148 * ASSET_UNIT, 0),
		(RUDOLF.into(), 35 * ASSET_UNIT, 0),
		(CERTO.into(), 840 * ASSET_UNIT, 0),
		(TIESTO.into(), 132 * ASSET_UNIT, 0),
		(DAVID.into(), 21 * ASSET_UNIT, 0),
		(ATAKAN.into(), 59 * ASSET_UNIT, 0),
		(YANN.into(), 89 * ASSET_UNIT, 0),
		(ENIS.into(), 332 * ASSET_UNIT, 0),
		(ALFREDO.into(), 8110 * ASSET_UNIT, 0),
		(QENDRIM.into(), 394 * ASSET_UNIT, 0),
		(LEONARDO.into(), 840 * ASSET_UNIT, 0),
		(KEN.into(), 352 * ASSET_UNIT, 0),
		(LUCA.into(), 640 * ASSET_UNIT, 0),
		(FLAVIO.into(), 792 * ASSET_UNIT, 0),
		(FREDI.into(), 993 * ASSET_UNIT, 0),
		(ALI.into(), 794 * ASSET_UNIT, 0),
		(DILARA.into(), 256 * ASSET_UNIT, 0),
		(DAMIAN.into(), 431 * ASSET_UNIT, 0),
		(KAYA.into(), 935 * ASSET_UNIT, 0),
		(IAZI.into(), 174 * ASSET_UNIT, 0),
		(CHRIGI.into(), 877 * ASSET_UNIT, 0),
		(VALENTINA.into(), 961 * ASSET_UNIT, 0),
		(ALMA.into(), 394 * ASSET_UNIT, 0),
		(ALENA.into(), 442 * ASSET_UNIT, 0),
		(PATRICK.into(), 486 * ASSET_UNIT, 0),
		(ONTARIO.into(), 17 * ASSET_UNIT, 0),
		(RAKIA.into(), 9424 * ASSET_UNIT, 0),
		(HUBERT.into(), 14 * ASSET_UNIT, 0),
		(UTUS.into(), 4906 * ASSET_UNIT, 0),
		(TOME.into(), 68 * ASSET_UNIT, 0),
		(ZUBER.into(), 9037 * ASSET_UNIT, 0),
		(ADAM.into(), 442 * ASSET_UNIT, 0),
		(STANI.into(), 40 * ASSET_UNIT, 0),
		(BETI.into(), 68 * ASSET_UNIT, 0),
		(HALIT.into(), 68 * ASSET_UNIT, 0),
		(DRAGAN.into(), 98 * ASSET_UNIT, 0),
		(LEA.into(), 705 * ASSET_UNIT, 0),
		(LUIS.into(), 422 * ASSET_UNIT, 0),
		(JOEL.into(), 692 * ASSET_UNIT, 0),
		(MALIK.into(), 24 * ASSET_UNIT, 0),
		(SOLOMUN.into(), 840 * ASSET_UNIT, 0),
		(JONAS.into(), 59 * ASSET_UNIT, 0),
	]
}

#[test]
fn evaluation_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);

	let issuer = ISSUER.into();
	let project = excel_project(inst.get_new_nonce());
	let evaluations = excel_evaluators();

	Polimec::execute_with(|| {
		inst.create_auctioning_project(project, issuer, evaluations);
	});
}

#[test]
fn auction_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);

	let issuer = ISSUER.into();
	let project = excel_project(inst.get_new_nonce());
	let evaluations = excel_evaluators();
	let bids = excel_bidders();

	Polimec::execute_with(|| {
		//let filtered_bids = MockInstantiator::filter_bids_after_auction(bids.clone(), project.total_allocation_size.0);
		let (project_id, _) = inst.create_community_contributing_project(project, issuer, evaluations, bids);
		let wavgp_from_excel = 10.202357561;
		// Convert the float to a FixedU128
		let wavgp_to_substrate = FixedU128::from_float(wavgp_from_excel);
		dbg!(wavgp_to_substrate);
		let wavgp_from_chain = inst.get_project_details(project_id).weighted_average_price.unwrap();
		dbg!(wavgp_from_chain);
		let res = wavgp_from_chain.checked_sub(&wavgp_to_substrate).unwrap();
		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.00001.
		assert!(res < FixedU128::from_float(0.00001));
		let names = names();
		inst.execute(|| {
			let bids =
				Bids::<PolimecRuntime>::iter_prefix_values((0,)).sorted_by_key(|bid| bid.bidder.clone()).collect_vec();

			for bid in bids.clone() {
				let key: [u8; 32] = bid.bidder.clone().into();
				println!("{}: {}", names[&key], bid.funding_asset_amount_locked);
			}
			let total_participation = bids.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount_locked);
			dbg!(total_participation);
		})
	});
}

#[test]
fn community_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);

	Polimec::execute_with(|| {
		let _ = inst.create_remainder_contributing_project(
			excel_project(0),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
		);

		inst.execute(|| {
			let contributions = Contributions::<PolimecRuntime>::iter_prefix_values((0,))
				.sorted_by_key(|bid| bid.contributor.clone())
				.collect_vec();
			let total_contribution =
				contributions.clone().into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
			let total_contribution_as_fixed = FixedU128::from_rational(total_contribution, PLMC);
			dbg!(total_contribution_as_fixed);
			// In USD
			// let total_ct_amount = contributions.clone().into_iter().fold(0, |acc, bid| acc + bid.ct_amount);
			// let total_contribution_from_excel = 46821.0;
			// dbg!(total_contribution_from_excel);
			// let res = total_contribution_as_fixed - FixedU128::from_float(total_contribution_from_excel);
			// // We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.001.
			// assert!(res < FixedU128::from_float(0.001));
			// let total_ct_sold_from_excel = 46821;
			// assert_eq!(total_ct_amount / PLMC, total_ct_sold_from_excel);
		})
	});
}

#[test]
fn remainder_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);

	Polimec::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(0),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);

		let price = inst.get_project_details(project_id).weighted_average_price.unwrap();
		let price_as_u128 = price.checked_mul_int(1_0_000_000_000u128).unwrap();
		dbg!(price_as_u128);
		let funding_necessary_1 =
			IntegrationInstantiator::calculate_contributed_funding_asset_spent(excel_contributions(), price);
		let funding_necessary_2 =
			IntegrationInstantiator::calculate_contributed_funding_asset_spent(excel_remainders(), price);
		let mut total = 0u128;
		for item in funding_necessary_1 {
			total += item.asset_amount;
		}
		for item in funding_necessary_2 {
			total += item.asset_amount;
		}
		dbg!(total);
		let contributions = Contributions::<PolimecRuntime>::iter_prefix_values((0,))
			.sorted_by_key(|contribution| contribution.contributor.clone())
			.collect_vec();
		let total_contributions =
			contributions.into_iter().fold(0, |acc, contribution| acc + contribution.funding_asset_amount);
		let total_contributions_as_fixed = FixedU128::from_rational(total_contributions, PLMC);
		dbg!(total_contributions_as_fixed);
		let total_from_excel = 503945.4517;
		let total_to_substrate = FixedU128::from_float(total_from_excel);
		dbg!(total_to_substrate);
		let res = total_contributions_as_fixed.checked_sub(&total_to_substrate).unwrap();
		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0001.
		assert!(res < FixedU128::from_float(0.001));
	});
}

#[test]
fn funds_raised() {
	let mut inst = IntegrationInstantiator::new(None);

	Polimec::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(0),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);

		inst.execute(|| {
			let project_specific_account: AccountId = PolimecFunding::fund_account_id(project_id);
			let funding =
				PolimecStatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), project_specific_account);
			let fund_raised_from_excel = 1005361.955;
			let fund_raised_to_substrate = FixedU128::from_float(fund_raised_from_excel);
			let fund_raised_as_fixed = FixedU128::from_rational(funding, ASSET_UNIT);
			let res = fund_raised_to_substrate.checked_sub(&fund_raised_as_fixed).unwrap();
			// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0003.
			assert!(res < FixedU128::from_float(0.001));
		})
	});
}

#[test]
fn ct_minted() {
	let mut inst = IntegrationInstantiator::new(None);

	Polimec::execute_with(|| {
		let _ = inst.create_finished_project(
			excel_project(0),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);
		inst.advance_time(<PolimecRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10).unwrap();

		for (contributor, expected_amount, project_id) in excel_ct_amounts() {
			let minted = inst
				.execute(|| <PolimecRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
			assert_close_enough!(minted, expected_amount, Perquintill::from_parts(10_000_000_000u64));
		}
	});
}

#[test]
fn ct_migrated() {
	let mut inst = IntegrationInstantiator::new(None);

	let project_id = Polimec::execute_with(|| {
		let project_id = inst.create_finished_project(
			excel_project(0),
			ISSUER.into(),
			excel_evaluators(),
			excel_bidders(),
			excel_contributions(),
			excel_remainders(),
		);
		inst.advance_time(<PolimecRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

		inst.advance_time(10).unwrap();

		for (contributor, expected_amount, project_id) in excel_ct_amounts() {
			let minted = inst
				.execute(|| <PolimecRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
			assert_close_enough!(minted, expected_amount, Perquintill::from_parts(10_000_000_000u64));
		}

		project_id
	});

	let project_details = Polimec::execute_with(|| inst.get_project_details(project_id));
	assert!(matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Rewarded(_)));

	// Mock HRMP establishment
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::do_set_para_id_for_project(&ISSUER.into(), project_id, ParaId::from(6969u32)));

		let open_channel_message = xcm::v3::opaque::Instruction::HrmpNewChannelOpenRequest {
			sender: 6969,
			max_message_size: 102_300,
			max_capacity: 1000,
		};
		assert_ok!(PolimecFunding::do_handle_channel_open_request(open_channel_message));

		let channel_accepted_message = xcm::v3::opaque::Instruction::HrmpChannelAccepted { recipient: 6969u32 };
		assert_ok!(PolimecFunding::do_handle_channel_accepted(channel_accepted_message));
	});

	// Migration is ready
	Polimec::execute_with(|| {
		let project_details = pallet_funding::ProjectsDetails::<PolimecRuntime>::get(project_id).unwrap();
		assert!(project_details.migration_readiness_check.unwrap().is_ready())
	});

	excel_ct_amounts().iter().unique().for_each(|item| {
		let data = Penpal::account_data_of(item.0.clone());
		assert_eq!(data.free, 0u128, "Participant balances should be 0 before ct migration");
	});

	// Migrate CTs
	let accounts = excel_ct_amounts().iter().map(|item| item.0.clone()).unique().collect::<Vec<_>>();
	let total_ct_sold = excel_ct_amounts().iter().fold(0, |acc, item| acc + item.1);
	dbg!(total_ct_sold);
	let polimec_sov_acc = Penpal::sovereign_account_id_of((Parent, Parachain(polimec::PARA_ID)).into());
	let polimec_fund_balance = Penpal::account_data_of(polimec_sov_acc);
	dbg!(polimec_fund_balance);

	let names = names();

	for account in accounts {
		Polimec::execute_with(|| {
			assert_ok!(PolimecFunding::migrate_one_participant(
				PolimecOrigin::signed(account.clone()),
				project_id,
				account.clone()
			));
			let key: [u8; 32] = account.clone().into();
			println!("Migrated CTs for {}", names[&key]);
			inst.advance_time(1u32).unwrap();
		});
	}

	Penpal::execute_with(|| {
		dbg!(Penpal::events());
	});

	// Check balances after migration, before vesting
	excel_ct_amounts().iter().unique().for_each(|item| {
		let data = Penpal::account_data_of(item.0.clone());
		let key: [u8; 32] = item.0.clone().into();
		println!("Participant {} has {} CTs. Expected {}", names[&key], data.free.clone(), item.1);
		dbg!(data.clone());
		assert_close_enough!(
			data.free,
			item.1,
			Perquintill::from_parts(10_000_000_000u64),
			"Participant balances should be transfered to each account after ct migration, but be frozen"
		);
	});
}
