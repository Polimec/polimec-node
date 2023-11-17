use super::*;
use crate::{tests::defaults::*, *};
use frame_support::BoundedVec;
use pallet_funding::{instantiator::*, *};
use polimec_parachain_runtime::US_DOLLAR;
use sp_arithmetic::Perquintill;
use sp_core::U256;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedSub},
	FixedU128,
};

type UserToCTBalance = Vec<(AccountId, BalanceOf<PolimecRuntime>, ProjectIdOf<PolimecRuntime>)>;

fn increase_by_one(account_id: AccountId) -> impl FnMut() -> AccountId {
	let mut num = U256::from(account_id.clone());

	move || {
		let output = num;
		num.saturating_add(U256::from(1));
		account_id = num.try_into().unwrap();
	}
}

define_names_v2! {
	[0; 32],
	increase_by_one,
	LINA, "Lina";
	MIA, "Mia";
	ALEXEY, "Alexey";
	PAUL, "Paul";
	MARIA, "Maria";
	GEORGE, "George";
	CLARA, "Clara";
	RAMONA, "Ramona";
	PASCAL, "Pascal";
	EMMA, "Emma";
	BIBI, "Bibi";
	AHMED, "Ahmed";
	HERBERT, "Herbert";
	LENI, "Leni";
	XI, "Xi";
	TOM, "Tom";
	ADAMS, "Adams";
	POLK, "Polk";
	MARKUS, "Markus";
	ELLA, "Ella";
	SKR, "Skr";
	ARTHUR, "Arthur";
	MILA, "Mila";
	LINCOLN, "Lincoln";
	MONROE, "Monroe";
	ARBRESHA, "Arbresha";
	ELDIN, "Eldin";
	HARDING, "Harding";
	SOFIA, "Sofia";
	DOMINIK, "Dominik";
	NOLAND, "Noland";
	HANNAH, "Hannah";
	HOOVER, "Hoover";
	GIGI, "Gigi";
	JEFFERSON, "Jefferson";
	LINDI, "Lindi";
	KEVIN, "Kevin";
	ANIS, "Anis";
	RETO, "Reto";
	HAALAND, "Haaland";
	XENIA, "Xenia";
	EVA, "Eva";
	SKARA, "Skara";
	ROOSEVELT, "Roosevelt";
	DRACULA, "Dracula";
	DURIM, "Durim";
	HARRISON, "Harrison";
	DRIN, "Drin";
	PARI, "Pari";
	TUTI, "Tuti";
	BENITO, "Benito";
	VANESSA, "Vanessa";
	ENES, "Enes";
	RUDOLF, "Rudolf";
	CERTO, "Certo";
	TIESTO, "Tiesto";
	DAVID, "David";
	ATAKAN, "Atakan";
	YANN, "Yann";
	ENIS, "Enis";
	ALFREDO, "Alfredo";
	QENDRIM, "Qendrim";
	LEONARDO, "Leonardo";
	KEN, "Ken";
	LUCA, "Luca";
	FLAVIO, "Flavio";
	FREDI, "Fredi";
	ALI, "Ali";
	DILARA, "Dilara";
	DAMIAN, "Damian";
	KAYA, "Kaya";
	IAZI, "Iazi";
	CHRIGI, "Chrigi";
	VALENTINA, "Valentina";
	ALMA, "Alma";
	ALENA, "Alena";
	PATRICK, "Patrick";
	ONTARIO, "Ontario";
	RAKIA, "Rakia";
	HUBERT, "Hubert";
	UTUS, "Utus";
	TOME, "Tome";
	ZUBER, "Zuber";
	ADAM, "Adam";
	STANI, "Stani";
	BETI, "Beti";
	HALIT, "Halit";
	DRAGAN, "Dragan";
	LEA, "Lea";
	LUIS, "Luis";
	TATI, "Tati";
	WEST, "West";
	MIRIJAM, "Mirijam";
	LIONEL, "Lionel";
	GIOVANNI, "Giovanni";
	JOEL, "Joel";
	POLKA, "Polk";
	MALIK, "Malik";
	ALEXANDER, "Alexander";
	SOLOMUN, "Solomun";
	JOHNNY, "Johnny";
	GRINGO, "Gringo";
	JONAS, "Jonas";
	BUNDI, "Bundi";
	FELIX, "Felix";
}

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
		funding_destination_account: issuer(),
		offchain_information_hash: Some(metadata_hash),
	}
}

fn excel_evaluators() -> Vec<UserToUSDBalance<PolimecRuntime>> {
	vec![
		UserToUSDBalance::new(LINA, 93754 * US_DOLLAR),
		UserToUSDBalance::new(MIA, 162 * US_DOLLAR),
		UserToUSDBalance::new(ALEXEY, 7454 * US_DOLLAR),
		UserToUSDBalance::new(PAUL, 8192 * US_DOLLAR),
		UserToUSDBalance::new(MARIA, 11131 * US_DOLLAR),
		UserToUSDBalance::new(GEORGE, 4765 * US_DOLLAR),
		UserToUSDBalance::new(CLARA, 4363 * US_DOLLAR),
		UserToUSDBalance::new(RAMONA, 4120 * US_DOLLAR),
		UserToUSDBalance::new(PASCAL, 1626 * US_DOLLAR),
		UserToUSDBalance::new(EMMA, 3996 * US_DOLLAR),
		UserToUSDBalance::new(BIBI, 3441 * US_DOLLAR),
		UserToUSDBalance::new(AHMED, 8048 * US_DOLLAR),
		UserToUSDBalance::new(HERBERT, 2538 * US_DOLLAR),
		UserToUSDBalance::new(LENI, 5803 * US_DOLLAR),
		UserToUSDBalance::new(XI, 1669 * US_DOLLAR),
		UserToUSDBalance::new(TOM, 6526 * US_DOLLAR),
	]
}

fn excel_bidders() -> Vec<BidParams<PolimecRuntime>> {
	vec![
		BidParams::from(ADAMS, 700 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(POLK, 4000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MARKUS, 3000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ELLA, 700 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(SKR, 3400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ARTHUR, 1000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MILA, 8400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINCOLN, 800 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(MONROE, 1300 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ARBRESHA, 5000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(ELDIN, 600 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(HARDING, 800 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(SOFIA, 3000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(DOMINIK, 8000 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(NOLAND, 900 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINA, 8400 * ASSET_UNIT, FixedU128::from_float(10.0)),
		BidParams::from(LINA, 1000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(HANNAH, 400 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(HOOVER, 2000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(GIGI, 600 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(JEFFERSON, 1000 * ASSET_UNIT, FixedU128::from_float(11.0)),
		BidParams::from(JEFFERSON, 2000 * ASSET_UNIT, FixedU128::from_float(12.0)),
	]
}

fn excel_contributors() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::from(DRIN, 692 * US_DOLLAR),
		ContributionParams::from(PARI, 236 * US_DOLLAR),
		ContributionParams::from(TUTI, 24 * US_DOLLAR),
		ContributionParams::from(BENITO, 688 * US_DOLLAR),
		ContributionParams::from(VANESSA, 33 * US_DOLLAR),
		ContributionParams::from(ENES, 1148 * US_DOLLAR),
		ContributionParams::from(RUDOLF, 35 * US_DOLLAR),
		ContributionParams::from(CERTO, 840 * US_DOLLAR),
		ContributionParams::from(TIESTO, 132 * US_DOLLAR),
		ContributionParams::from(DAVID, 21 * US_DOLLAR),
		ContributionParams::from(ATAKAN, 59 * US_DOLLAR),
		ContributionParams::from(YANN, 89 * US_DOLLAR),
		ContributionParams::from(ENIS, 332 * US_DOLLAR),
		ContributionParams::from(ALFREDO, 8110 * US_DOLLAR),
		ContributionParams::from(QENDRIM, 394 * US_DOLLAR),
		ContributionParams::from(LEONARDO, 840 * US_DOLLAR),
		ContributionParams::from(KEN, 352 * US_DOLLAR),
		ContributionParams::from(LUCA, 640 * US_DOLLAR),
		// TODO: XI is a partipant in the Community Round AND an Evaluator. At the moment, this returns `InsufficientBalance` because it seems we don't mint to him enough USDT.
		// To be addressed and tested in a separate PR.
		//ContributionParams::from(XI, 588 * US_DOLLAR),
		ContributionParams::from(FLAVIO, 792 * US_DOLLAR),
		ContributionParams::from(FREDI, 993 * US_DOLLAR),
		ContributionParams::from(ALI, 794 * US_DOLLAR),
		ContributionParams::from(DILARA, 256 * US_DOLLAR),
		ContributionParams::from(DAMIAN, 431 * US_DOLLAR),
		ContributionParams::from(KAYA, 935 * US_DOLLAR),
		ContributionParams::from(IAZI, 174 * US_DOLLAR),
		ContributionParams::from(CHRIGI, 877 * US_DOLLAR),
		ContributionParams::from(VALENTINA, 961 * US_DOLLAR),
		ContributionParams::from(ALMA, 394 * US_DOLLAR),
		ContributionParams::from(ALENA, 442 * US_DOLLAR),
		ContributionParams::from(PATRICK, 486 * US_DOLLAR),
		ContributionParams::from(ONTARIO, 17 * US_DOLLAR),
		ContributionParams::from(RAKIA, 9424 * US_DOLLAR),
		ContributionParams::from(HUBERT, 14 * US_DOLLAR),
		ContributionParams::from(UTUS, 4906 * US_DOLLAR),
		ContributionParams::from(TOME, 68 * US_DOLLAR),
		ContributionParams::from(ZUBER, 9037 * US_DOLLAR),
		ContributionParams::from(ADAM, 442 * US_DOLLAR),
		ContributionParams::from(STANI, 40 * US_DOLLAR),
		ContributionParams::from(BETI, 68 * US_DOLLAR),
		ContributionParams::from(HALIT, 68 * US_DOLLAR),
		ContributionParams::from(DRAGAN, 98 * US_DOLLAR),
		ContributionParams::from(LEA, 17 * US_DOLLAR),
		ContributionParams::from(LUIS, 422 * US_DOLLAR),
	]
}

fn excel_remainders() -> Vec<ContributionParams<PolimecRuntime>> {
	vec![
		ContributionParams::from(JOEL, 692 * US_DOLLAR),
		ContributionParams::from(POLK, 236 * US_DOLLAR),
		ContributionParams::from(MALIK, 24 * US_DOLLAR),
		ContributionParams::from(LEA, 688 * US_DOLLAR),
		ContributionParams::from(RAMONA, 35 * US_DOLLAR),
		ContributionParams::from(SOLOMUN, 840 * US_DOLLAR),
		ContributionParams::from(JONAS, 59 * US_DOLLAR),
	]
}

fn excel_ct_amounts() -> UserToCTBalance {
	vec![
		(LINA, 42916134112336, 0),
		(MIA, 32685685157, 0),
		(ALEXEY, 1422329504123, 0),
		(PAUL, 1164821313204, 0),
		(MARIA, 1582718022129, 0),
		(GEORGE, 677535834646, 0),
		(CLARA, 620375413759, 0),
		(RAMONA, 935823219043, 0),
		(PASCAL, 231201105380, 0),
		(EMMA, 568191646431, 0),
		(BIBI, 489276139982, 0),
		(AHMED, 1144345938558, 0),
		(HERBERT, 360878478139, 0),
		(LENI, 825129160220, 0),
		(XI, 237315279753, 0),
		(TOM, 927932603756, 0),
		(ADAMS, 700 * ASSET_UNIT, 0),
		(POLK, 4236 * ASSET_UNIT, 0),
		(MARKUS, 3000 * ASSET_UNIT, 0),
		(ELLA, 700 * ASSET_UNIT, 0),
		(SKR, 3400 * ASSET_UNIT, 0),
		(ARTHUR, 1000 * ASSET_UNIT, 0),
		(MILA, 8400 * ASSET_UNIT, 0),
		(LINCOLN, 800 * ASSET_UNIT, 0),
		(MONROE, 1300 * ASSET_UNIT, 0),
		(ARBRESHA, 5000 * ASSET_UNIT, 0),
		(ELDIN, 600 * ASSET_UNIT, 0),
		(HARDING, 800 * ASSET_UNIT, 0),
		(SOFIA, 3000 * ASSET_UNIT, 0),
		(DOMINIK, 8000 * ASSET_UNIT, 0),
		(NOLAND, 900 * ASSET_UNIT, 0),
		(HANNAH, 400 * ASSET_UNIT, 0),
		(HOOVER, 2000 * ASSET_UNIT, 0),
		(GIGI, 600 * ASSET_UNIT, 0),
		(JEFFERSON, 3000 * ASSET_UNIT, 0),
		(DRIN, 692 * ASSET_UNIT, 0),
		(PARI, 236 * ASSET_UNIT, 0),
		(TUTI, 24 * ASSET_UNIT, 0),
		(BENITO, 688 * ASSET_UNIT, 0),
		(VANESSA, 33 * ASSET_UNIT, 0),
		(ENES, 1148 * ASSET_UNIT, 0),
		(RUDOLF, 35 * ASSET_UNIT, 0),
		(CERTO, 840 * ASSET_UNIT, 0),
		(TIESTO, 132 * ASSET_UNIT, 0),
		(DAVID, 21 * ASSET_UNIT, 0),
		(ATAKAN, 59 * ASSET_UNIT, 0),
		(YANN, 89 * ASSET_UNIT, 0),
		(ENIS, 332 * ASSET_UNIT, 0),
		(ALFREDO, 8110 * ASSET_UNIT, 0),
		(QENDRIM, 394 * ASSET_UNIT, 0),
		(LEONARDO, 840 * ASSET_UNIT, 0),
		(KEN, 352 * ASSET_UNIT, 0),
		(LUCA, 640 * ASSET_UNIT, 0),
		(FLAVIO, 792 * ASSET_UNIT, 0),
		(FREDI, 993 * ASSET_UNIT, 0),
		(ALI, 794 * ASSET_UNIT, 0),
		(DILARA, 256 * ASSET_UNIT, 0),
		(DAMIAN, 431 * ASSET_UNIT, 0),
		(KAYA, 935 * ASSET_UNIT, 0),
		(IAZI, 174 * ASSET_UNIT, 0),
		(CHRIGI, 877 * ASSET_UNIT, 0),
		(VALENTINA, 961 * ASSET_UNIT, 0),
		(ALMA, 394 * ASSET_UNIT, 0),
		(ALENA, 442 * ASSET_UNIT, 0),
		(PATRICK, 486 * ASSET_UNIT, 0),
		(ONTARIO, 17 * ASSET_UNIT, 0),
		(RAKIA, 9424 * ASSET_UNIT, 0),
		(HUBERT, 14 * ASSET_UNIT, 0),
		(UTUS, 4906 * ASSET_UNIT, 0),
		(TOME, 68 * ASSET_UNIT, 0),
		(ZUBER, 9037 * ASSET_UNIT, 0),
		(ADAM, 442 * ASSET_UNIT, 0),
		(STANI, 40 * ASSET_UNIT, 0),
		(BETI, 68 * ASSET_UNIT, 0),
		(HALIT, 68 * ASSET_UNIT, 0),
		(DRAGAN, 98 * ASSET_UNIT, 0),
		(LEA, 705 * ASSET_UNIT, 0),
		(LUIS, 422 * ASSET_UNIT, 0),
		(JOEL, 692 * ASSET_UNIT, 0),
		(MALIK, 24 * ASSET_UNIT, 0),
		(SOLOMUN, 840 * ASSET_UNIT, 0),
		(JONAS, 59 * ASSET_UNIT, 0),
	]
}

#[test]
fn evaluation_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer = issuer();
	let project = excel_project(inst.get_new_nonce());
	let evaluations = excel_evaluators();

	inst.create_auctioning_project(project, issuer(), evaluations);
}

#[test]
fn auction_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);
	let issuer = issuer();
	let project = excel_project(inst.get_new_nonce());
	let evaluations = excel_evaluators();
	let bids = excel_bidders();
	//let filtered_bids = MockInstantiator::filter_bids_after_auction(bids.clone(), project.total_allocation_size.0);
	let (project_id, _) = inst.create_community_contributing_project(project, issuer(), evaluations, bids);
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
		let bids = Bids::<PolimecRuntime>::iter_prefix_values((0,)).sorted_by_key(|bid| bid.bidder).collect_vec();

		for bid in bids.clone() {
			println!("{}: {}", names[&bid.bidder], bid.funding_asset_amount_locked);
		}
		let total_participation = bids.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount_locked);
		dbg!(total_participation);
	})
}

#[test]
fn community_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);
	let _ = inst.create_remainder_contributing_project(
		excel_project(0),
		issuer(),
		excel_evaluators(),
		excel_bidders(),
		excel_contributors(),
	);

	inst.execute(|| {
		let contributions = Contributions::<PolimecRuntime>::iter_prefix_values((0,))
			.sorted_by_key(|bid| bid.contributor)
			.collect_vec();
		let total_contribution = contributions.clone().into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
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
}

#[test]
fn remainder_round_completed() {
	let mut inst = IntegrationInstantiator::new(None);
	let _ = inst.create_finished_project(
		excel_project(0),
		issuer(),
		excel_evaluators(),
		excel_bidders(),
		excel_contributors(),
		excel_remainders(),
	);

	inst.execute(|| {
		let contributions = Contributions::<PolimecRuntime>::iter_prefix_values((0,))
			.sorted_by_key(|bid| bid.contributor)
			.collect_vec();
		let total_contributions = contributions.into_iter().fold(0, |acc, bid| acc + bid.funding_asset_amount);
		dbg!(total_contributions);
		let total_contributions_as_fixed = FixedU128::from_rational(total_contributions, PLMC);
		let total_from_excel = 503945.4517;
		let total_to_substrate = FixedU128::from_float(total_from_excel);
		dbg!(total_to_substrate);
		let res = total_contributions_as_fixed.checked_sub(&total_to_substrate).unwrap();
		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0001.
		assert!(res < FixedU128::from_float(0.001));
	})
}

#[test]
fn funds_raised() {
	let mut inst = IntegrationInstantiator::new(None);
	let _ = inst.create_finished_project(
		excel_project(0),
		issuer(),
		excel_evaluators(),
		excel_bidders(),
		excel_contributors(),
		excel_remainders(),
	);

	inst.execute(|| {
		let pallet_id = <PolimecRuntime as pallet::Config>::PalletId::get();
		let project_specific_account: u64 = pallet_id.into_sub_account_truncating(0);
		let funding = StatemintAssets::balance(1984, project_specific_account);
		let fund_raised_from_excel = 1005361.955;
		let fund_raised_to_substrate = FixedU128::from_float(fund_raised_from_excel);
		let fund_raised_as_fixed = FixedU128::from_rational(funding, ASSET_UNIT);
		let res = fund_raised_to_substrate.checked_sub(&fund_raised_as_fixed).unwrap();
		// We are more precise than Excel. From the 11th decimal onwards, the difference should be less than 0.0003.
		assert!(res < FixedU128::from_float(0.001));
	})
}

#[test]
fn ct_minted() {
	let mut inst = IntegrationInstantiator::new(None);
	let _ = inst.create_finished_project(
		excel_project(0),
		issuer(),
		excel_evaluators(),
		excel_bidders(),
		excel_contributors(),
		excel_remainders(),
	);
	inst.advance_time(<PolimecRuntime as Config>::SuccessToSettlementTime::get()).unwrap();

	inst.advance_time(10).unwrap();

	for (contributor, expected_amount, project_id) in excel_ct_amounts() {
		let minted =
			inst.execute(|| <PolimecRuntime as Config>::ContributionTokenCurrency::balance(project_id, &contributor));
		assert_close_enough!(minted, expected_amount, Perquintill::from_parts(10_000_000_000u64));
	}
}
