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
use sp_arithmetic::{FixedPointNumber, Percent, Perquintill};
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
		mainnet_token_max_supply: 10_000_000_0_000_000_000, // Made up, not in the Sheet.
		// Total Allocation of Contribution Tokens Available for the Funding Round
		total_allocation_size: 1_000_000_0_000_000_000,
		auction_round_allocation_percentage: Percent::from_percent(50u8),

		// Minimum Price per Contribution Token (in USDT)
		minimum_price: PriceOf::<PolimecRuntime>::from(10),
		round_ticket_sizes: RoundTicketSizes {
			bidding: BiddingTicketSizes {
				professional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				institutional: TicketSize::new(Some(5000 * US_DOLLAR), None),
				phantom: Default::default(),
			},
			contributing: ContributingTicketSizes {
				retail: TicketSize::new(None, None),
				professional: TicketSize::new(None, None),
				institutional: TicketSize::new(None, None),
				phantom: Default::default(),
			},
			phantom: Default::default(),
		},
		participation_currencies: vec![AcceptedFundingAsset::USDT].try_into().unwrap(),
		funding_destination_account: ISSUER.into(),
		offchain_information_hash: Some(metadata_hash),
	}
}

fn excel_evaluators() -> Vec<UserToUSDBalance<PolimecRuntime>> {
	vec![
		UserToUSDBalance::new(LINA.into(), 937540 * US_DOLLAR),
		UserToUSDBalance::new(MIA.into(), 1620 * US_DOLLAR),
		UserToUSDBalance::new(ALEXEY.into(), 74540 * US_DOLLAR),
		UserToUSDBalance::new(PAUL.into(), 81920 * US_DOLLAR),
		UserToUSDBalance::new(MARIA.into(), 111310 * US_DOLLAR),
		UserToUSDBalance::new(GEORGE.into(), 47650 * US_DOLLAR),
		UserToUSDBalance::new(CLARA.into(), 43630 * US_DOLLAR),
		UserToUSDBalance::new(RAMONA.into(), 41200 * US_DOLLAR),
		UserToUSDBalance::new(PASCAL.into(), 16260 * US_DOLLAR),
		UserToUSDBalance::new(EMMA.into(), 39960 * US_DOLLAR),
		UserToUSDBalance::new(BIBI.into(), 34410 * US_DOLLAR),
		UserToUSDBalance::new(AHMED.into(), 80480 * US_DOLLAR),
		UserToUSDBalance::new(HERBERT.into(), 25380 * US_DOLLAR),
		UserToUSDBalance::new(LENI.into(), 58030 * US_DOLLAR),
		UserToUSDBalance::new(XI.into(), 16690 * US_DOLLAR),
		UserToUSDBalance::new(TOM.into(), 65260 * US_DOLLAR),
	]
}

fn excel_bidders() -> Vec<BidParams<PolimecRuntime>> {
	vec![
		BidParams::new_with_defaults(ADAMS.into(), 7000 * ASSET_UNIT),
		BidParams::new_with_defaults(POLK.into(), 40000 * ASSET_UNIT),
		BidParams::new_with_defaults(MARKUS.into(), 300000 * ASSET_UNIT),
		BidParams::new_with_defaults(ELLA.into(), 7000 * ASSET_UNIT),
		BidParams::new_with_defaults(SKR.into(), 34000 * ASSET_UNIT),
		BidParams::new_with_defaults(ARTHUR.into(), 10000 * ASSET_UNIT),
		BidParams::new_with_defaults(MILA.into(), 84000 * ASSET_UNIT),
		BidParams::new_with_defaults(LINCOLN.into(), 8000 * ASSET_UNIT),
		BidParams::new_with_defaults(MONROE.into(), 13000 * ASSET_UNIT),
		BidParams::new_with_defaults(ARBRESHA.into(), 50000 * ASSET_UNIT),
		BidParams::new_with_defaults(ELDIN.into(), 6000 * ASSET_UNIT),
		BidParams::new_with_defaults(HARDING.into(), 8000 * ASSET_UNIT),
		BidParams::new_with_defaults(SOFIA.into(), 30000 * ASSET_UNIT),
		BidParams::new_with_defaults(DOMINIK.into(), 80000 * ASSET_UNIT),
		BidParams::new_with_defaults(NOLAND.into(), 9000 * ASSET_UNIT),
		BidParams::new_with_defaults(LINA.into(), 84000 * ASSET_UNIT),
		BidParams::new_with_defaults(LINA.into(), 10000 * ASSET_UNIT),
		BidParams::new_with_defaults(HANNAH.into(), 4000 * ASSET_UNIT),
		BidParams::new_with_defaults(HOOVER.into(), 20000 * ASSET_UNIT),
		BidParams::new_with_defaults(GIGI.into(), 6000 * ASSET_UNIT),
		BidParams::new_with_defaults(JEFFERSON.into(), 10000 * ASSET_UNIT),
		BidParams::new_with_defaults(JEFFERSON.into(), 20000 * ASSET_UNIT),
	]
}

fn excel_contributions() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::new_with_defaults(DRIN.into(), 6920 * US_DOLLAR),
		ContributionParams::new_with_defaults(PARI.into(), 2360 * US_DOLLAR),
		ContributionParams::new_with_defaults(TUTI.into(), 240 * US_DOLLAR),
		ContributionParams::new_with_defaults(BENITO.into(), 6880 * US_DOLLAR),
		ContributionParams::new_with_defaults(VANESSA.into(), 330 * US_DOLLAR),
		ContributionParams::new_with_defaults(ENES.into(), 11480 * US_DOLLAR),
		ContributionParams::new_with_defaults(RUDOLF.into(), 350 * US_DOLLAR),
		ContributionParams::new_with_defaults(CERTO.into(), 8400 * US_DOLLAR),
		ContributionParams::new_with_defaults(TIESTO.into(), 1320 * US_DOLLAR),
		ContributionParams::new_with_defaults(DAVID.into(), 210 * US_DOLLAR),
		ContributionParams::new_with_defaults(ATAKAN.into(), 590 * US_DOLLAR),
		ContributionParams::new_with_defaults(YANN.into(), 890 * US_DOLLAR),
		ContributionParams::new_with_defaults(ENIS.into(), 3320 * US_DOLLAR),
		ContributionParams::new_with_defaults(ALFREDO.into(), 81100 * US_DOLLAR),
		ContributionParams::new_with_defaults(QENDRIM.into(), 3940 * US_DOLLAR),
		ContributionParams::new_with_defaults(LEONARDO.into(), 8400 * US_DOLLAR),
		ContributionParams::new_with_defaults(KEN.into(), 3520 * US_DOLLAR),
		ContributionParams::new_with_defaults(LUCA.into(), 6400 * US_DOLLAR),
		// TODO: XI is a partipant in the Community Round AND an Evaluator. At the moment, this returns `InsufficientBalance` because it seems we don't mint to him enough USDT.
		// To be addressed and tested in a separate PR.
		//ContributionParams::from(XI, 5880 * US_DOLLAR),
		ContributionParams::new_with_defaults(FLAVIO.into(), 7920 * US_DOLLAR),
		ContributionParams::new_with_defaults(FREDI.into(), 9930 * US_DOLLAR),
		ContributionParams::new_with_defaults(ALI.into(), 7940 * US_DOLLAR),
		ContributionParams::new_with_defaults(DILARA.into(), 2560 * US_DOLLAR),
		ContributionParams::new_with_defaults(DAMIAN.into(), 4310 * US_DOLLAR),
		ContributionParams::new_with_defaults(KAYA.into(), 9350 * US_DOLLAR),
		ContributionParams::new_with_defaults(IAZI.into(), 1740 * US_DOLLAR),
		ContributionParams::new_with_defaults(CHRIGI.into(), 8770 * US_DOLLAR),
		ContributionParams::new_with_defaults(VALENTINA.into(), 9610 * US_DOLLAR),
		ContributionParams::new_with_defaults(ALMA.into(), 3940 * US_DOLLAR),
		ContributionParams::new_with_defaults(ALENA.into(), 4420 * US_DOLLAR),
		ContributionParams::new_with_defaults(PATRICK.into(), 4860 * US_DOLLAR),
		ContributionParams::new_with_defaults(ONTARIO.into(), 170 * US_DOLLAR),
		ContributionParams::new_with_defaults(RAKIA.into(), 94240 * US_DOLLAR),
		ContributionParams::new_with_defaults(HUBERT.into(), 140 * US_DOLLAR),
		ContributionParams::new_with_defaults(UTUS.into(), 49060 * US_DOLLAR),
		ContributionParams::new_with_defaults(TOME.into(), 680 * US_DOLLAR),
		ContributionParams::new_with_defaults(ZUBER.into(), 90370 * US_DOLLAR),
		ContributionParams::new_with_defaults(ADAM.into(), 4420 * US_DOLLAR),
		ContributionParams::new_with_defaults(STANI.into(), 400 * US_DOLLAR),
		ContributionParams::new_with_defaults(BETI.into(), 680 * US_DOLLAR),
		ContributionParams::new_with_defaults(HALIT.into(), 680 * US_DOLLAR),
		ContributionParams::new_with_defaults(DRAGAN.into(), 980 * US_DOLLAR),
		ContributionParams::new_with_defaults(LEA.into(), 170 * US_DOLLAR),
		ContributionParams::new_with_defaults(LUIS.into(), 4220 * US_DOLLAR),
	]
}

fn excel_remainders() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::new_with_defaults(JOEL.into(), 6920 * US_DOLLAR),
		ContributionParams::new_with_defaults(POLK.into(), 2360 * US_DOLLAR),
		ContributionParams::new_with_defaults(MALIK.into(), 240 * US_DOLLAR),
		ContributionParams::new_with_defaults(LEA.into(), 6880 * US_DOLLAR),
		ContributionParams::new_with_defaults(RAMONA.into(), 350 * US_DOLLAR),
		ContributionParams::new_with_defaults(SOLOMUN.into(), 8400 * US_DOLLAR),
		ContributionParams::new_with_defaults(JONAS.into(), 590 * US_DOLLAR),
	]
}

fn excel_ct_amounts() -> UserToCTBalance {
	vec![
		(LINA.into(), 429161341123360, 0),
		(MIA.into(), 326856851570, 0),
		(ALEXEY.into(), 14223295041230, 0),
		(PAUL.into(), 11648213132040, 0),
		(MARIA.into(), 15827180221290, 0),
		(GEORGE.into(), 6775358346460, 0),
		(CLARA.into(), 6203754137590, 0),
		(RAMONA.into(), 9358232190430, 0),
		(PASCAL.into(), 2312011053800, 0),
		(EMMA.into(), 5681916464310, 0),
		(BIBI.into(), 4892761399820, 0),
		(AHMED.into(), 11443459385580, 0),
		(HERBERT.into(), 3608784781390, 0),
		(LENI.into(), 8251291602200, 0),
		(XI.into(), 2373152797530, 0),
		(TOM.into(), 9279326037560, 0),
		(ADAMS.into(), 7000 * ASSET_UNIT, 0),
		(POLK.into(), 42360 * ASSET_UNIT, 0),
		(MARKUS.into(), 30000 * ASSET_UNIT, 0),
		(ELLA.into(), 7000 * ASSET_UNIT, 0),
		(SKR.into(), 34000 * ASSET_UNIT, 0),
		(ARTHUR.into(), 10000 * ASSET_UNIT, 0),
		(MILA.into(), 84000 * ASSET_UNIT, 0),
		(LINCOLN.into(), 8000 * ASSET_UNIT, 0),
		(MONROE.into(), 13000 * ASSET_UNIT, 0),
		(ARBRESHA.into(), 50000 * ASSET_UNIT, 0),
		(ELDIN.into(), 6000 * ASSET_UNIT, 0),
		(HARDING.into(), 8000 * ASSET_UNIT, 0),
		(SOFIA.into(), 30000 * ASSET_UNIT, 0),
		(DOMINIK.into(), 80000 * ASSET_UNIT, 0),
		(NOLAND.into(), 9000 * ASSET_UNIT, 0),
		(HANNAH.into(), 4000 * ASSET_UNIT, 0),
		(HOOVER.into(), 20000 * ASSET_UNIT, 0),
		(GIGI.into(), 6000 * ASSET_UNIT, 0),
		(JEFFERSON.into(), 30000 * ASSET_UNIT, 0),
		(DRIN.into(), 6920 * ASSET_UNIT, 0),
		(PARI.into(), 2360 * ASSET_UNIT, 0),
		(TUTI.into(), 240 * ASSET_UNIT, 0),
		(BENITO.into(), 6880 * ASSET_UNIT, 0),
		(VANESSA.into(), 330 * ASSET_UNIT, 0),
		(ENES.into(), 11480 * ASSET_UNIT, 0),
		(RUDOLF.into(), 350 * ASSET_UNIT, 0),
		(CERTO.into(), 8400 * ASSET_UNIT, 0),
		(TIESTO.into(), 1320 * ASSET_UNIT, 0),
		(DAVID.into(), 210 * ASSET_UNIT, 0),
		(ATAKAN.into(), 590 * ASSET_UNIT, 0),
		(YANN.into(), 890 * ASSET_UNIT, 0),
		(ENIS.into(), 3320 * ASSET_UNIT, 0),
		(ALFREDO.into(), 81100 * ASSET_UNIT, 0),
		(QENDRIM.into(), 3940 * ASSET_UNIT, 0),
		(LEONARDO.into(), 8400 * ASSET_UNIT, 0),
		(KEN.into(), 3520 * ASSET_UNIT, 0),
		(LUCA.into(), 6400 * ASSET_UNIT, 0),
		(FLAVIO.into(), 7920 * ASSET_UNIT, 0),
		(FREDI.into(), 9930 * ASSET_UNIT, 0),
		(ALI.into(), 7940 * ASSET_UNIT, 0),
		(DILARA.into(), 2560 * ASSET_UNIT, 0),
		(DAMIAN.into(), 4310 * ASSET_UNIT, 0),
		(KAYA.into(), 9350 * ASSET_UNIT, 0),
		(IAZI.into(), 1740 * ASSET_UNIT, 0),
		(CHRIGI.into(), 8770 * ASSET_UNIT, 0),
		(VALENTINA.into(), 9610 * ASSET_UNIT, 0),
		(ALMA.into(), 3940 * ASSET_UNIT, 0),
		(ALENA.into(), 4420 * ASSET_UNIT, 0),
		(PATRICK.into(), 4860 * ASSET_UNIT, 0),
		(ONTARIO.into(), 170 * ASSET_UNIT, 0),
		(RAKIA.into(), 94240 * ASSET_UNIT, 0),
		(HUBERT.into(), 140 * ASSET_UNIT, 0),
		(UTUS.into(), 49060 * ASSET_UNIT, 0),
		(TOME.into(), 680 * ASSET_UNIT, 0),
		(ZUBER.into(), 90370 * ASSET_UNIT, 0),
		(ADAM.into(), 4420 * ASSET_UNIT, 0),
		(STANI.into(), 400 * ASSET_UNIT, 0),
		(BETI.into(), 680 * ASSET_UNIT, 0),
		(HALIT.into(), 680 * ASSET_UNIT, 0),
		(DRAGAN.into(), 980 * ASSET_UNIT, 0),
		(LEA.into(), 7050 * ASSET_UNIT, 0),
		(LUIS.into(), 4220 * ASSET_UNIT, 0),
		(JOEL.into(), 6920 * ASSET_UNIT, 0),
		(MALIK.into(), 240 * ASSET_UNIT, 0),
		(SOLOMUN.into(), 8400 * ASSET_UNIT, 0),
		(JONAS.into(), 590 * ASSET_UNIT, 0),
	]
}

#[ignore]
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

#[ignore]
#[test]
fn auction_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);

	let issuer = ISSUER.into();
	let project = excel_project(inst.get_new_nonce());
	let evaluations = excel_evaluators();
	let bids = excel_bidders();

	Polimec::execute_with(|| {
		let project_id = inst.create_community_contributing_project(project, issuer, evaluations, bids);
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

#[ignore]
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
		})
	});
}

#[ignore]
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

#[ignore]
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
				PolimecForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), project_specific_account);
			let fund_raised_from_excel = 10053610.955;
			let fund_raised_to_substrate = FixedU128::from_float(fund_raised_from_excel);
			let fund_raised_as_fixed = FixedU128::from_rational(funding, ASSET_UNIT);
			let res = fund_raised_to_substrate.checked_sub(&fund_raised_as_fixed).unwrap();
			// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0003.
			assert!(res < FixedU128::from_float(0.001));
		})
	});
}

#[ignore]
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

#[ignore]
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
		let account_id: PolimecAccountId = ISSUER.into();
		assert_ok!(PolimecFunding::do_set_para_id_for_project(&ISSUER.into(), project_id, ParaId::from(6969u32),));

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
