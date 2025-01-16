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

//! Test the full flow using a complicated example from a Google sheets doc.
use crate::{constants::PricesBuilder, tests::defaults::*, *};
use frame_support::{
	traits::{
		fungible::InspectHold,
		fungibles::{metadata::Inspect, Inspect as OtherInspect},
	},
	BoundedVec,
};
use itertools::Itertools;
use macros::generate_accounts;
use pallet_funding::{traits::VestingDurationCalculation, *};
use polimec_common::{
	credentials::InvestorType,
	migration_types::{MigrationStatus, ParticipationType},
	ProvideAssetPrice, USD_DECIMALS, USD_UNIT,
};
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt, get_mock_jwt_with_cid};
use polimec_runtime::PLMC;
use sp_arithmetic::{FixedPointNumber, Percent, Perquintill};
use sp_runtime::FixedU128;
use AcceptedFundingAsset::{DOT, USDC, USDT};
use InvestorType::{Institutional, Professional, Retail};
use ParticipationMode::{Classic, OTM};

#[rustfmt::skip]
generate_accounts!(
	// Users that only evaluated
	ALMA, ALEX, ADAM, ALAN, ABEL, AMOS, ANNA, ABBY, ARIA,
	// Users that only bid
	BROCK, BEN, BILL, BRAD, BLAIR, BOB, BRETT, BLAKE, BRIAN, BELLA, BRUCE, BRENT,
	// Users that only contributed
	CLINT, CARL, CODY, COLE, CORY, CHAD, CRUZ, CASEY, CECIL, CINDY, CLARA, CARLY, CADEN, CRAIG, CAIN, CALEB, CAMI, CARA,
	CARY, CATHY, CESAR, CHEN, CHIP, CHLOE, CHONG, CONOR, CYRUS, CEDRIC, CHAIM, CHICO,
	// Users that evaluated and bid
	DOUG, DAVE,
	// Users that evaluated and contributed
	MASON, MIKE,
	// Users that bid and contributed
	GERALT, GEORGE, GINO,
	// Users that evaluated, bid and contributed
	STEVE, SAM,
);

pub fn project_metadata() -> ProjectMetadataOf<PolimecRuntime> {
	let bounded_name = BoundedVec::try_from("You Only Live Once".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("YOLO".as_bytes().to_vec()).unwrap();
	let metadata_hash = ipfs_hash();
	ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
		mainnet_token_max_supply: 10_000_000 * CT_UNIT, // Made up, not in the Sheet.
		// Total Allocation of Contribution Tokens Available for the Funding Round
		total_allocation_size: 100_000 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),

		// Minimum Price per Contribution Token (in USDT)
		minimum_price: PriceProviderOf::<PolimecRuntime>::calculate_decimals_aware_price(
			PriceOf::<PolimecRuntime>::from_float(10.0f64),
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
		participation_currencies: vec![
			AcceptedFundingAsset::USDT,
			AcceptedFundingAsset::USDC,
			AcceptedFundingAsset::DOT,
		]
		.try_into()
		.unwrap(),
		funding_destination_account: ISSUER.into(),
		policy_ipfs_cid: Some(metadata_hash),
		participants_account_type: ParticipantsAccountType::Polkadot,
	}
}

fn plmc_price() -> FixedU128 {
	FixedU128::from_float(0.1822)
}
fn dot_price() -> FixedU128 {
	FixedU128::from_float(4.65)
}
fn usdc_price() -> FixedU128 {
	FixedU128::from_float(1.0005)
}
fn usdt_price() -> FixedU128 {
	FixedU128::from_float(1f64)
}

fn evaluations() -> Vec<([u8; 32], InvestorType, u64, f64)> {
	// (User, Investor type, USD specified in extrinsic, PLMC bonded as a consequence)
	vec![
		(ALMA, Institutional, 93_754, 514_566.41),
		(ALEX, Professional, 162, 889.13),
		(ADAM, Retail, 7_454, 40_911.09),
		(ALAN, Retail, 8_192, 44_961.58),
		(ABEL, Professional, 11_131, 61_092.21),
		(AMOS, Professional, 4_765, 26_152.58),
		(ALMA, Institutional, 4_363, 23_946.21),
		(ANNA, Institutional, 4_120, 22_612.51),
		(ABBY, Retail, 1_626, 8_924.26),
		(ARIA, Retail, 3_996, 21_931.94),
		(MASON, Retail, 3_441, 18_885.84),
		(MIKE, Retail, 8_048, 44_171.24),
		(DOUG, Institutional, 2_538, 13_929.75),
		(DAVE, Professional, 5_803, 31_849.62),
		(STEVE, Professional, 1_669, 9_160.26),
		(SAM, Professional, 6_526, 35_817.78),
	]
}

fn pre_wap_bids() -> Vec<(u32, [u8; 32], ParticipationMode, InvestorType, u64, f64, AcceptedFundingAsset, f64, f64)> {
	// (bid_id, User, Participation mode, Investor type, CTs specified in extrinsic, CT Price, Participation Currency, Final participation currency ticket, PLMC bonded as a consequence)
	vec![
		(0, BROCK, OTM, Professional, 700, 10.0, USDC, 7_101.4493, 7_683.8639),
		(1, BEN, OTM, Professional, 4_000, 10.0, USDT, 40_600.0, 43_907.7936),
		(2, BILL, Classic(3), Professional, 3_000, 10.0, USDC, 29_985.0075, 54_884.7420),
		(3, BRAD, Classic(6), Professional, 700, 10.0, USDT, 7_000.0, 6_403.2199),
		(4, BROCK, Classic(9), Professional, 3_400, 10.0, USDT, 34_000.0, 20_734.2359),
		(5, BLAIR, Classic(8), Professional, 1_000, 10.0, USDT, 10_000.0, 6_860.5928),
		(6, BROCK, Classic(7), Professional, 8_400, 10.0, USDT, 84_000.0, 65_861.6905),
		(7, BOB, Classic(10), Professional, 800, 10.0, USDT, 8_000.0, 4_390.7794),
		(8, BRETT, Classic(2), Professional, 1_300, 10.0, DOT, 2_795.6989, 35_675.0823),
		(9, BLAKE, Classic(1), Professional, 5_000, 10.0, USDT, 50_000.0, 274_423.7101),
		(10, BRIAN, Classic(1), Institutional, 600, 10.0, USDT, 6_000.0, 32_930.8452),
		(11, BELLA, Classic(1), Professional, 800, 10.0, USDT, 8_000.0, 43_907.7936),
		(12, BRUCE, Classic(4), Institutional, 3_000, 10.0, USDT, 30_000.0, 41_163.5565),
		(13, BRENT, Classic(1), Institutional, 8_000, 10.0, USDT, 80_000.0, 439_077.9363),
		(14, DOUG, OTM, Institutional, 900, 10.0, USDT, 9_135.0, 9_879.2536),
		(15, DAVE, OTM, Professional, 8_400, 10.0, USDT, 85_260.0, 92_206.3666),
		(16, DAVE, OTM, Professional, 1_000, 11.0, USDT, 11_165.0, 12_074.6432),
		(17, GERALT, Classic(15), Institutional, 500, 11.0, USDT, 5_500.0, 2_012.4405),
		(18, GEORGE, Classic(20), Institutional, 1_900, 11.0, USDT, 20_900.0, 5_735.4555),
		(19, GINO, Classic(25), Institutional, 600, 11.0, USDT, 6_600.0, 1_448.9572),
		(20, STEVE, OTM, Professional, 1_000, 11.0, USDT, 11_165.0, 12_074.6432),
		(21, SAM, OTM, Professional, 2_000, 12.0, USDT, 24_360.0, 26_344.6762),
		(22, SAM, OTM, Professional, 2_200, 12.0, DOT, 5_762.5806, 28_979.1438),
	]
}

fn wap() -> f64 {
	10.30346708
}

#[allow(unused)]
fn post_wap_bids() -> Vec<(u32, [u8; 32], ParticipationMode, InvestorType, u64, f64, AcceptedFundingAsset, f64, f64)> {
	// (bid_id, User, Participation mode, Investor type, CTs specified in extrinsic, CT Price, Participation Currency, Final participation currency ticket, PLMC bonded as a consequence)
	vec![
		(21, SAM, OTM, Professional, 2_000, 10.303467, USDT, 20_916.0382, 22_620.1253),
		(22, SAM, OTM, Professional, 2_200, 10.303467, DOT, 4_947.8800, 24_882.1378),
		(16, DAVE, OTM, Professional, 1_000, 10.303467, USDT, 10_458.0191, 11_310.0627),
		(17, GERALT, Classic(15), Institutional, 500, 10.303467, USDT, 5_151.7335, 1_885.0104),
		(18, GEORGE, Classic(20), Institutional, 1_900, 10.303467, USDT, 19_576.5875, 5_372.2798),
		(19, GINO, Classic(25), Institutional, 600, 10.303467, USDT, 6_182.0802, 1_357.2075),
		(20, STEVE, OTM, Professional, 1_000, 10.303467, USDT, 10_458.0191, 11_310.0627),
		(0, BROCK, OTM, Professional, 700, 10.000000, USDC, 7_101.4493, 7_683.8639),
		(1, BEN, OTM, Professional, 4_000, 10.000000, USDT, 40_600.0000, 43_907.7936),
		(2, BILL, Classic(3), Professional, 3_000, 10.000000, USDC, 29_985.0075, 54_884.7420),
		(3, BRAD, Classic(6), Professional, 700, 10.000000, USDT, 7_000.0000, 6_403.2199),
		(4, BROCK, Classic(9), Professional, 3_400, 10.000000, USDT, 34_000.0000, 20_734.2359),
		(5, BLAIR, Classic(8), Professional, 1_000, 10.000000, USDT, 10_000.0000, 6_860.5928),
		(6, BROCK, Classic(7), Professional, 8_400, 10.000000, USDT, 84_000.0000, 65_861.6905),
		(7, BOB, Classic(10), Professional, 800, 10.000000, USDT, 8_000.0000, 4_390.7794),
		(8, BRETT, Classic(2), Professional, 1_300, 10.000000, DOT, 2_795.6989, 35_675.0823),
		(9, BLAKE, Classic(1), Professional, 5_000, 10.000000, USDT, 50_000.0000, 274_423.7102),
		(10, BRIAN, Classic(1), Institutional, 600, 10.000000, USDT, 6_000.0000, 32_930.8452),
		(11, BELLA, Classic(1), Professional, 800, 10.000000, USDT, 8_000.0000, 43_907.7936),
		(12, BRUCE, Classic(4), Institutional, 3_000, 10.000000, USDT, 30_000.0000, 41_163.5565),
		(13, BRENT, Classic(1), Institutional, 8_000, 10.000000, USDT, 80_000.0000, 439_077.9363),
		(14, DOUG, OTM, Institutional, 100, 10.000000, USDT, 1_015.0000, 5_488.4742),
		(15, DAVE, OTM, Professional, 0, 10.000000, USDT, 0.00, 0.00),
	]
}

fn community_contributions(
) -> Vec<(u32, [u8; 32], ParticipationMode, InvestorType, u64, AcceptedFundingAsset, f64, f64)> {
	// (contribution_id, User, Participation mode, Investor type, CTs specified in extrinsic, PLMC bonded as a consequence, Participation Currency, Final participation currency ticket)
	vec![
		(0, CLINT, OTM, Professional, 692, USDT, 7_236.949209, 7_826.5634),
		(1, CARL, Classic(1), Retail, 236, USDT, 2_431.618231, 13_345.8739),
		(2, CODY, Classic(2), Retail, 24, USDT, 247.28321, 678.6038),
		(3, COLE, OTM, Retail, 688, USDT, 7_195.117133, 7_781.3231),
		(4, CORY, OTM, Retail, 33, DOT, 74.21819998, 373.2321),
		(5, CHAD, Classic(1), Retail, 1_148, DOT, 2_543.73768, 64_919.7597),
		(6, CLINT, Classic(1), Retail, 35, DOT, 77.55297804, 1_979.2610),
		(7, CRUZ, Classic(4), Retail, 840, USDC, 8_650.587056, 11_875.5658),
		(8, CASEY, Classic(1), Retail, 132, USDC, 1_359.377966, 7_464.6414),
		(9, CECIL, OTM, Retail, 21, USDT, 219.6184009, 237.5113),
		(10, CINDY, Classic(1), Retail, 59, USDT, 607.9045578, 3_336.4685),
		(11, CHAD, Classic(3), Retail, 89, USDT, 917.0085703, 1_677.6593),
		(12, CLARA, Classic(1), Retail, 332, DOT, 735.6453917, 18_774.7040),
		(13, CARLY, OTM, Retail, 8_110, USDC, 84_772.14873, 91_724.6082),
		(14, CADEN, Classic(1), Retail, 394, USDC, 4_057.537262, 22_280.8234),
		(15, CRAIG, Classic(1), Professional, 840, USDC, 8_650.587056, 47_502.2632),
		(16, CAIN, Classic(1), Professional, 352, USDC, 3_625.007909, 19_905.7103),
		(17, CALEB, Classic(1), Retail, 640, DOT, 1_418.111598, 36_192.2005),
		(18, CAMI, Classic(1), Professional, 792, DOT, 1_754.913103, 44_787.8481),
		(19, CARA, OTM, Retail, 993, DOT, 2_233.293109, 11_230.0),
		(20, CARY, Classic(2), Retail, 794, USDT, 8_180.952863, 22_450.4744),
		(21, CATHY, Classic(1), Retail, 256, USDC, 2_636.369388, 14_476.8802),
		(22, CESAR, Classic(1), Retail, 431, USDC, 4_438.575025, 24_373.1850),
		(23, CHAD, Classic(1), Retail, 935, USDC, 9_628.927258, 52_874.5429),
		(24, CHEN, OTM, Retail, 174, USDC, 1_818.785928, 1_967.9509),
		(25, CHIP, OTM, Retail, 877, DOT, 1_972.40489, 9_918.9250),
		(26, CHLOE, OTM, Retail, 961, USDT, 10_050.15634, 10_868.9702),
		(27, CHONG, Classic(1), Retail, 394, DOT, 873.0249528, 22_280.8234),
	]
}

fn remainder_contributions(
) -> Vec<(u32, [u8; 32], ParticipationMode, InvestorType, u64, AcceptedFundingAsset, f64, f64)> {
	// (contribution_id User, Participation mode, Investor type, CTs specified in extrinsic, Participation Currency, Final participation currency ticket, PLMC bonded as a consequence, )
	vec![
		(28, CODY, Classic(1), Retail, 442, DOT, 979.3833227, 24_995.2385),
		(29, CRUZ, Classic(1), Retail, 486, DOT, 1_076.878495, 27_483.4523),
		(30, CONOR, Classic(1), Retail, 17, DOT, 37.66858933, 961.3553),
		(31, CYRUS, Classic(25), Institutional, 9_424, DOT, 20_881.69329, 21_317.2061),
		(32, CEDRIC, Classic(2), Retail, 1_400, USDT, 14_424.85392, 39_585.2193),
		(33, CHAIM, OTM, Retail, 4_906, USDT, 51_307.04165, 55_487.1674),
		(34, CHICO, Classic(17), Institutional, 68, USDT, 700.6357616, 226.2013),
		(35, CRUZ, OTM, Institutional, 9_037, USDT, 94_509.1185, 102_209.0362),
		(36, MASON, OTM, Retail, 442, USDT, 4_622.444437, 4_999.0477),
		(37, MIKE, OTM, Retail, 400, USDT, 4_183.207635, 4_524.0251),
		(38, GERALT, OTM, Professional, 68, USDT, 711.145298, 769.0843),
		(39, GEORGE, Classic(1), Professional, 68, USDT, 700.6357616, 3_845.4213),
		(40, GINO, OTM, Institutional, 98, USDT, 1_024.885871, 1_108.3861),
		// These two use their evaluation bond
		(41, STEVE, Classic(5), Professional, 500, USDT, 5_151.733541, 0.0),
		(42, SAM, Classic(1), Professional, 422, USDT, 4_348.063109, 0.0),
	]
}

// Includes evaluation rewards, participation purchases, and protocol fees.
fn cts_minted() -> f64 {
	108_938.93
}

fn usd_raised() -> f64 {
	1_008_177.9575
}

#[allow(unused)]
fn ct_fees() -> (f64, f64, f64) {
	// (LP, Evaluator Rewards, Long term holder bonus)
	(4944.466414, 2966.679848, 1977.786566)
}

fn issuer_payouts() -> (f64, f64, f64) {
	// (USDT, USDC, DOT)
	(646_218.88, 165_339.74, 42_265.73)
}

fn evaluator_reward_pots() -> (f64, f64, f64, f64) {
	// (CTs All, CTs Early, USD All, USD Early)
	(2373.343879, 593.3359697, 167_588.0, 100_000.0)
}

fn final_payouts() -> Vec<([u8; 32], f64, f64)> {
	// User, CT rewarded, PLMC Self Bonded (Classic mode) with mult > 1
	vec![
		(ALMA, 1945.787276, 0.00),
		(ALEX, 3.25541214, 0.00),
		(ADAM, 141.6604459, 0.00),
		(ALAN, 116.0132769, 0.00),
		(ABEL, 157.6347394, 0.00),
		(AMOS, 67.48086726, 0.00),
		(ANNA, 58.34652111, 0.00),
		(ABBY, 23.02704935, 0.00),
		(ARIA, 56.59046077, 0.00),
		(BROCK, 12_500.0, 86_595.93),
		(BEN, 4_000.0, 0.00),
		(BILL, 3_000.0, 54_884.74),
		(BRAD, 700.0, 6_403.22),
		(BLAIR, 1_000.0, 6_860.59),
		(BOB, 800.0, 4_390.78),
		(BRETT, 1_300.0, 35_675.08),
		(BLAKE, 5_000.0, 0.00),
		(BRIAN, 600.0, 0.00),
		(BELLA, 800.0, 0.00),
		(BRUCE, 3_000.0, 41_163.56),
		(BRENT, 8_000.0, 0.00),
		(CLINT, 727.0, 0.00),
		(CARL, 236.0, 0.00),
		(CODY, 466.0, 678.60),
		(COLE, 688.0, 0.00),
		(CORY, 33.0, 0.00),
		(CRUZ, 10_363.0, 11_875.57),
		(CASEY, 132.0, 0.00),
		(CECIL, 21.0, 0.00),
		(CINDY, 59.0, 0.00),
		(CHAD, 2_172.0, 1_677.66),
		(CLARA, 332.0, 0.00),
		(CARLY, 8_110.0, 0.00),
		(CADEN, 394.0, 0.00),
		(CRAIG, 840.0, 0.00),
		(CAIN, 352.0, 0.00),
		(CALEB, 640.0, 0.00),
		(CAMI, 792.0, 0.00),
		(CARA, 993.0, 0.00),
		(CARY, 794.0, 22_450.47),
		(CATHY, 256.0, 0.00),
		(CESAR, 431.0, 0.00),
		(CHEN, 174.0, 0.00),
		(CHIP, 877.0, 0.00),
		(CHLOE, 961.0, 0.00),
		(CHONG, 394.0, 0.00),
		(CONOR, 17.0, 0.00),
		(CYRUS, 9_424.0, 21_317.21),
		(CEDRIC, 1_400.0, 39_585.22),
		(CHAIM, 4_906.0, 0.0),
		(CHICO, 68.0, 226.20),
		(DOUG, 135.9425899, 0.00),
		(DAVE, 1_082.180792, 0.00),
		(MASON, 490.7306745, 0.00),
		(MIKE, 513.973981, 0.00),
		(GERALT, 568.0, 1_885.01),
		(GEORGE, 1_968.0, 5_372.28),
		(GINO, 698.0, 1_357.21),
		(STEVE, 1_523.636006, 5_655.03),
		(SAM, 4_714.419756, 0.0),
	]
}

fn otm_fee_recipient_balances() -> (f64, f64, f64) {
	// USDT, USDC, DOT
	(3908.966909, 1384.616511, 136.3713724)
}

fn otm_treasury_sub_account_plmc_held() -> f64 {
	544_177.11
}

fn participate_with_checks(
	mut inst: IntegrationInstantiator,
	project_id: ProjectId,
	participation_type: ParticipationType,
	participation_id: u32,
	user: [u8; 32],
	mode: ParticipationMode,
	investor_type: InvestorType,
	ct_amount: u64,
	funding_asset: AcceptedFundingAsset,
	expected_funding_asset_ticket: f64,
	expected_plmc_bonded: f64,
) -> IntegrationInstantiator {
	assert_ne!(participation_type, ParticipationType::Evaluation, "Only Bids and Contributions work here");
	let user: PolimecAccountId = user.into();
	let ct_amount = FixedU128::from_rational(ct_amount.into(), 1).saturating_mul_int(CT_UNIT);

	let plmc_ed = inst.get_ed();

	let funding_asset_unit = 10u128.pow(PolimecForeignAssets::decimals(funding_asset.id()) as u32);
	let funding_asset_ticket =
		FixedU128::from_float(expected_funding_asset_ticket).saturating_mul_int(funding_asset_unit);
	let plmc_bonded = FixedU128::from_float(expected_plmc_bonded).saturating_mul_int(PLMC);

	let user_jwt =
		get_mock_jwt_with_cid(user.clone(), investor_type, generate_did_from_account(user.clone()), ipfs_hash());
	// Add one more to account for rounding errors in the spreadsheet
	inst.mint_funding_asset_to(vec![
		(user.clone(), funding_asset_ticket + funding_asset_unit, funding_asset.id()).into()
	]);

	if let ParticipationMode::Classic(..) = mode {
		inst.mint_plmc_to(vec![(user.clone(), plmc_bonded + PLMC + plmc_ed).into()]);
	} else {
		let funding_asset_ed = inst.get_funding_asset_ed(funding_asset.id());
		inst.mint_funding_asset_to(vec![(user.clone(), funding_asset_ed, funding_asset.id()).into()]);
	}

	let prev_participation_free_plmc = PolimecBalances::free_balance(user.clone());
	let prev_participation_reserved_plmc = PolimecBalances::reserved_balance(user.clone());
	let prev_participation_funding_asset_balance = PolimecForeignAssets::balance(funding_asset.id(), user.clone());

	let sub_account = polimec_runtime::ProxyBonding::get_bonding_account(project_id);
	let prev_participation_treasury_held_plmc =
		PolimecBalances::balance_on_hold(&HoldReason::Participation.into(), &sub_account);

	if participation_type == ParticipationType::Bid {
		PolimecFunding::bid(PolimecOrigin::signed(user.clone()), user_jwt, project_id, ct_amount, mode, funding_asset)
			.unwrap();
		assert!(Bids::<PolimecRuntime>::get((project_id, user.clone(), participation_id)).is_some());
	} else {
		PolimecFunding::contribute(
			PolimecOrigin::signed(user.clone()),
			user_jwt,
			project_id,
			ct_amount,
			mode,
			funding_asset,
		)
		.unwrap();
		assert!(Contributions::<PolimecRuntime>::get((project_id, user.clone(), participation_id)).is_some());
	}

	let post_participation_free_plmc = PolimecBalances::free_balance(user.clone());
	let post_participation_reserved_plmc = PolimecBalances::reserved_balance(user.clone());
	let post_participation_funding_asset_balance = PolimecForeignAssets::balance(funding_asset.id(), user.clone());
	let post_participation_treasury_held_plmc =
		PolimecBalances::balance_on_hold(&HoldReason::Participation.into(), &sub_account);

	let free_plmc_delta = prev_participation_free_plmc - post_participation_free_plmc;
	let reserved_plmc_delta = post_participation_reserved_plmc - prev_participation_reserved_plmc;
	let funding_asset_delta = prev_participation_funding_asset_balance - post_participation_funding_asset_balance;
	let treasury_held_plmc_delta = post_participation_treasury_held_plmc - prev_participation_treasury_held_plmc;

	let expected_self_plmc_delta = if let ParticipationMode::Classic(..) = mode { plmc_bonded } else { 0u128 };
	let expected_treasury_plmc_delta = if let ParticipationMode::Classic(..) = mode { 0u128 } else { plmc_bonded };

	assert_close_enough!(free_plmc_delta, expected_self_plmc_delta, Perquintill::from_float(0.9999));
	assert_close_enough!(reserved_plmc_delta, expected_self_plmc_delta, Perquintill::from_float(0.9999));
	assert_close_enough!(funding_asset_delta, funding_asset_ticket, Perquintill::from_float(0.9999));
	assert_close_enough!(treasury_held_plmc_delta, expected_treasury_plmc_delta, Perquintill::from_float(0.9999));

	inst
}

#[test]
fn e2e_test() {
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
		let issuer_jwt = get_mock_jwt(issuer.clone(), Institutional, generate_did_from_account(issuer.clone()));

		PolimecFunding::start_evaluation(PolimecOrigin::signed(issuer.clone()), issuer_jwt.clone(), project_id)
			.unwrap();

		for (user, investor_type, usd_bond, plmc_bonded) in evaluations() {
			let user: PolimecAccountId = user.into();
			let usd_bond: u128 = usd_bond as u128 * USD_UNIT;
			let plmc_bonded: u128 = FixedU128::from_float(plmc_bonded).saturating_mul_int(PLMC);

			let user_jwt = get_mock_jwt_with_cid(
				user.clone(),
				investor_type,
				generate_did_from_account(user.clone()),
				ipfs_hash(),
			);

			// We add 1 PLMC to the mint to avoid rounding errors, and add ED to keep the account alive.
			inst.mint_plmc_to(vec![(user.clone(), plmc_bonded + PLMC + plmc_ed).into()]);

			let pre_evaluation_free_plmc = PolimecBalances::free_balance(user.clone());
			let pre_evaluation_reserved_plmc = PolimecBalances::reserved_balance(user.clone());

			PolimecFunding::evaluate(PolimecOrigin::signed(user.clone()), user_jwt, project_id, usd_bond).unwrap();

			let post_evaluation_free_plmc = PolimecBalances::free_balance(user.clone());
			let post_evaluation_reserved_plmc = PolimecBalances::reserved_balance(user.clone());

			let free_plmc_delta = pre_evaluation_free_plmc - post_evaluation_free_plmc;
			let reserved_plmc_delta = post_evaluation_reserved_plmc - pre_evaluation_reserved_plmc;

			assert_close_enough!(free_plmc_delta, plmc_bonded, Perquintill::from_float(0.9999));
			assert_close_enough!(reserved_plmc_delta, plmc_bonded, Perquintill::from_float(0.9999));
		}

		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::AuctionRound);

		for (bid_id, user, mode, investor_type, ct_amount, _price, funding_asset, funding_asset_ticket, plmc_bonded) in
			pre_wap_bids()
		{
			inst = participate_with_checks(
				inst,
				project_id,
				ParticipationType::Bid,
				bid_id,
				user,
				mode,
				investor_type,
				ct_amount,
				funding_asset,
				funding_asset_ticket,
				plmc_bonded,
			);
		}

		assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(..)));

		// Only Dave got a rejected bid, so they can settle it early to get refunded:
		let rejected_bidder = PolimecAccountId::from(DAVE);
		let rejected_bid_id = 15u32;

		let otm_project_sub_account: PolimecAccountId = polimec_runtime::ProxyBonding::get_bonding_account(project_id);
		let funding_escrow_account = PolimecFunding::fund_account_id(project_id);

		let prev_bid_free_plmc = PolimecBalances::free_balance(rejected_bidder.clone());
		let prev_bid_reserved_plmc = PolimecBalances::reserved_balance(rejected_bidder.clone());
		let prev_bid_usdt_balance = PolimecForeignAssets::balance(USDT.id(), rejected_bidder.clone());
		let prev_treasury_usdt_balance = PolimecForeignAssets::balance(USDT.id(), otm_project_sub_account.clone());
		let prev_escrow_usdt_balance = PolimecForeignAssets::balance(USDT.id(), funding_escrow_account.clone());

		PolimecFunding::settle_bid(
			PolimecOrigin::signed(rejected_bidder.clone()),
			project_id,
			rejected_bidder.clone(),
			rejected_bid_id,
		)
		.unwrap();

		let post_bid_free_plmc = PolimecBalances::free_balance(rejected_bidder.clone());
		let post_bid_reserved_plmc = PolimecBalances::reserved_balance(rejected_bidder.clone());
		let post_bid_funding_asset_balance = PolimecForeignAssets::balance(USDT.id(), rejected_bidder.clone());
		let post_treasury_usdt_balance = PolimecForeignAssets::balance(USDT.id(), otm_project_sub_account.clone());
		let post_escrow_usdt_balance = PolimecForeignAssets::balance(USDT.id(), funding_escrow_account.clone());

		let free_plmc_delta = post_bid_free_plmc - prev_bid_free_plmc;
		let reserved_plmc_delta = post_bid_reserved_plmc - prev_bid_reserved_plmc;
		let bidder_usdt_delta = post_bid_funding_asset_balance - prev_bid_usdt_balance;
		let treasury_usdt_delta = prev_treasury_usdt_balance - post_treasury_usdt_balance;
		let funding_escrow_usdt_delta = prev_escrow_usdt_balance - post_escrow_usdt_balance;

		// Bid was OTM, so that user's PLMC should be unchanged
		assert_close_enough!(free_plmc_delta, 0, Perquintill::from_float(0.9999));
		assert_close_enough!(reserved_plmc_delta, 0, Perquintill::from_float(0.9999));

		// They should have gotten their USDT back
		let usdt_unit = 10u128.pow(PolimecForeignAssets::decimals(USDT.id()) as u32);
		let expected_usdt = FixedU128::from_float(85_260.0).saturating_mul_int(usdt_unit);
		let expected_otm_fee = FixedU128::from_float(1260.0).saturating_mul_int(usdt_unit);

		assert_close_enough!(bidder_usdt_delta, expected_usdt, Perquintill::from_float(0.9999));
		assert_close_enough!(
			funding_escrow_usdt_delta,
			expected_usdt - expected_otm_fee,
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(treasury_usdt_delta, expected_otm_fee, Perquintill::from_float(0.9999));

		for (contribution_id, user, mode, investor_type, ct_amount, funding_asset, funding_asset_ticket, plmc_bonded) in
			community_contributions()
		{
			inst = participate_with_checks(
				inst,
				project_id,
				ParticipationType::Contribution,
				contribution_id,
				user,
				mode,
				investor_type,
				ct_amount,
				funding_asset,
				funding_asset_ticket,
				plmc_bonded,
			);
		}

		let remainder_block =
			if let ProjectStatus::CommunityRound(remainder_block) = inst.get_project_details(project_id).status {
				remainder_block
			} else {
				panic!("Expected CommunityRound");
			};

		inst.jump_to_block(remainder_block);

		for (contribution_id, user, mode, investor_type, ct_amount, funding_asset, funding_asset_ticket, plmc_bonded) in
			remainder_contributions()
		{
			inst = participate_with_checks(
				inst,
				project_id,
				ParticipationType::Contribution,
				contribution_id,
				user,
				mode,
				investor_type,
				ct_amount,
				funding_asset,
				funding_asset_ticket,
				plmc_bonded,
			);
		}

		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::FundingSuccessful);
		assert_eq!(inst.go_to_next_state(project_id), ProjectStatus::SettlementStarted(FundingOutcome::Success));

		// Used for checking CT migrations at the end
		let stored_evaluations = inst.get_evaluations(project_id);
		let stored_bids = inst.get_bids(project_id);
		let stored_contributions = inst.get_contributions(project_id);

		inst.settle_project(project_id, true);

		let project_details = inst.get_project_details(project_id);
		let project_metadata = inst.get_project_metadata(project_id);
		let stored_wap = project_details.weighted_average_price.unwrap();
		let expected_wap = PriceProviderOf::<PolimecRuntime>::calculate_decimals_aware_price(
			PriceOf::<PolimecRuntime>::from_float(wap()),
			USD_DECIMALS,
			CT_DECIMALS,
		)
		.unwrap();
		assert_close_enough!(
			stored_wap.saturating_mul_int(PLMC),
			expected_wap.saturating_mul_int(PLMC),
			Perquintill::from_float(0.9999)
		);

		let actual_cts_minted = polimec_runtime::ContributionTokens::total_issuance(project_id);
		let expected_cts_minted = FixedU128::from_float(cts_minted()).saturating_mul_int(CT_UNIT);
		assert_close_enough!(actual_cts_minted, expected_cts_minted, Perquintill::from_float(0.9999));

		assert_close_enough!(
			project_details.funding_amount_reached_usd,
			FixedU128::from_float(usd_raised()).saturating_mul_int(USD_UNIT),
			Perquintill::from_float(0.9999)
		);

		let issuer_usdt =
			PolimecForeignAssets::balance(USDT.id(), project_metadata.funding_destination_account.clone());
		let issuer_usdc =
			PolimecForeignAssets::balance(USDC.id(), project_metadata.funding_destination_account.clone());
		let issuer_dot = PolimecForeignAssets::balance(DOT.id(), project_metadata.funding_destination_account.clone());

		let usdt_unit = 10u128.pow(PolimecForeignAssets::decimals(USDT.id()) as u32);
		let usdc_unit = 10u128.pow(PolimecForeignAssets::decimals(USDC.id()) as u32);
		let dot_unit = 10u128.pow(PolimecForeignAssets::decimals(DOT.id()) as u32);

		assert_close_enough!(
			issuer_usdt,
			FixedU128::from_float(issuer_payouts().0).saturating_mul_int(usdt_unit),
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(
			issuer_usdc,
			FixedU128::from_float(issuer_payouts().1).saturating_mul_int(usdc_unit),
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(
			issuer_dot,
			FixedU128::from_float(issuer_payouts().2).saturating_mul_int(dot_unit),
			Perquintill::from_float(0.9999)
		);

		let EvaluationRoundInfo { evaluators_outcome, .. } = project_details.evaluation_round_info;
		let Some(EvaluatorsOutcome::Rewarded(RewardInfo {
			early_evaluator_reward_pot,
			normal_evaluator_reward_pot,
			early_evaluator_total_bonded_usd,
			normal_evaluator_total_bonded_usd,
		})) = evaluators_outcome
		else {
			panic!("Unexpected evaluators outcome")
		};

		assert_close_enough!(
			normal_evaluator_reward_pot,
			FixedU128::from_float(evaluator_reward_pots().0).saturating_mul_int(CT_UNIT),
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(
			early_evaluator_reward_pot,
			FixedU128::from_float(evaluator_reward_pots().1).saturating_mul_int(CT_UNIT),
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(
			normal_evaluator_total_bonded_usd,
			FixedU128::from_float(evaluator_reward_pots().2).saturating_mul_int(USD_UNIT),
			Perquintill::from_float(0.9999)
		);
		assert_close_enough!(
			early_evaluator_total_bonded_usd,
			FixedU128::from_float(evaluator_reward_pots().3).saturating_mul_int(USD_UNIT),
			Perquintill::from_float(0.9999)
		);

		for (user, ct_rewarded, plmc_bonded) in final_payouts() {
			let user: PolimecAccountId = user.into();
			let ct_rewarded = FixedU128::from_float(ct_rewarded).saturating_mul_int(CT_UNIT);
			let plmc_bonded = FixedU128::from_float(plmc_bonded).saturating_mul_int(PLMC);

			let reserved_plmc = PolimecBalances::reserved_balance(user.clone());
			let ct_balance = polimec_runtime::ContributionTokens::balance(project_id, user.clone());

			assert_close_enough!(ct_balance, ct_rewarded, Perquintill::from_float(0.9999));
			assert_close_enough!(reserved_plmc, plmc_bonded, Perquintill::from_float(0.9999));
		}

		let plmc_balance = otm_treasury_sub_account_plmc_held();
		let plmc_balance = FixedU128::from_float(plmc_balance).saturating_mul_int(PLMC);

		let (usdt_balance, usdc_balance, dot_balance) = otm_fee_recipient_balances();
		let usdt_balance = FixedU128::from_float(usdt_balance).saturating_mul_int(usdt_unit);
		let usdc_balance = FixedU128::from_float(usdc_balance).saturating_mul_int(usdc_unit);
		let dot_balance = FixedU128::from_float(dot_balance).saturating_mul_int(dot_unit);

		polimec_runtime::ProxyBonding::transfer_fees_to_recipient(
			PolimecOrigin::signed(BOB.into()),
			project_id,
			HoldReason::Participation.into(),
			USDT.id(),
		)
		.unwrap();
		polimec_runtime::ProxyBonding::transfer_fees_to_recipient(
			PolimecOrigin::signed(BOB.into()),
			project_id,
			HoldReason::Participation.into(),
			USDC.id(),
		)
		.unwrap();
		polimec_runtime::ProxyBonding::transfer_fees_to_recipient(
			PolimecOrigin::signed(BOB.into()),
			project_id,
			HoldReason::Participation.into(),
			DOT.id(),
		)
		.unwrap();

		let fee_recipient = <PolimecRuntime as pallet_proxy_bonding::Config>::FeeRecipient::get();
		let fee_recipient_usdt_balance = PolimecForeignAssets::balance(USDT.id(), fee_recipient.clone());
		let fee_recipient_usdc_balance = PolimecForeignAssets::balance(USDC.id(), fee_recipient.clone());
		let fee_recipient_dot_balance = PolimecForeignAssets::balance(DOT.id(), fee_recipient.clone());

		assert_close_enough!(fee_recipient_usdt_balance, usdt_balance, Perquintill::from_float(0.999));
		assert_close_enough!(fee_recipient_usdc_balance, usdc_balance, Perquintill::from_float(0.999));
		assert_close_enough!(fee_recipient_dot_balance, dot_balance, Perquintill::from_float(0.999));

		let sub_account_held_plmc =
			PolimecBalances::balance_on_hold(&HoldReason::Participation.into(), &otm_project_sub_account.clone());
		assert_close_enough!(sub_account_held_plmc, plmc_balance, Perquintill::from_float(0.999));

		let otm_duration = Multiplier::force_new(5).calculate_vesting_duration::<PolimecRuntime>();
		let now = PolimecSystem::block_number();
		inst.jump_to_block(now + otm_duration);

		let treasury_account = <PolimecRuntime as pallet_proxy_bonding::Config>::Treasury::get();
		let pre_treasury_free_balance = PolimecBalances::free_balance(treasury_account.clone());

		polimec_runtime::ProxyBonding::transfer_bonds_back_to_treasury(
			PolimecOrigin::signed(BOB.into()),
			project_id,
			HoldReason::Participation.into(),
		)
		.unwrap();

		let post_treasury_free_balance = PolimecBalances::free_balance(treasury_account.clone());
		let sub_account_held_plmc =
			PolimecBalances::balance_on_hold(&HoldReason::Participation.into(), &otm_project_sub_account.clone());

		assert_eq!(sub_account_held_plmc, 0);
		assert_close_enough!(
			post_treasury_free_balance,
			pre_treasury_free_balance + plmc_balance,
			Perquintill::from_float(0.999)
		);

		inst.assert_evaluations_migrations_created(project_id, stored_evaluations, true);
		inst.assert_bids_migrations_created(project_id, stored_bids, true);
		inst.assert_contributions_migrations_created(project_id, stored_contributions, true);

		PolimecFunding::start_offchain_migration(PolimecOrigin::signed(issuer.clone()), issuer_jwt, project_id)
			.unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CTMigrationStarted);

		for user in UserMigrations::<PolimecRuntime>::iter_key_prefix((project_id,)).collect_vec() {
			assert_ok!(PolimecFunding::confirm_offchain_migration(
				PolimecOrigin::signed(issuer.clone()),
				project_id,
				user
			));
		}
		PolimecFunding::mark_project_ct_migration_as_finished(PolimecOrigin::signed(issuer.clone()), project_id)
			.unwrap();

		assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CTMigrationFinished);

		for (user, expected_total_cts, _) in final_payouts() {
			let user: PolimecAccountId = user.into();
			let expected_total_cts = FixedU128::from_float(expected_total_cts).saturating_mul_int(CT_UNIT);
			let (status, ct_migrations) = UserMigrations::<PolimecRuntime>::get((project_id, user)).unwrap();
			assert_eq!(status, MigrationStatus::Confirmed);
			let stored_total_cts = ct_migrations.iter().map(|m| m.info.contribution_token_amount).sum::<u128>();
			assert_close_enough!(stored_total_cts, expected_total_cts, Perquintill::from_float(0.9999));
		}
	});
}
