use crate::{
	instantiator::{UserToFundingAsset, UserToPLMCBalance},
	mock::{new_test_ext, TestRuntime, PLMC},
	tests::{
		defaults::{bounded_name, bounded_symbol, default_evaluations, default_project_metadata, ipfs_hash},
		CT_DECIMALS, CT_UNIT,
	},
	*,
};
use core::cell::RefCell;
use itertools::Itertools;
use polimec_common::{ProvideAssetPrice, USD_DECIMALS, USD_UNIT};
use sp_arithmetic::Percent;

#[test]
fn dry_run_wap() {
	let mut inst = tests::MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	const ADAM: AccountIdOf<TestRuntime> = 60;
	const TOM: AccountIdOf<TestRuntime> = 61;
	const SOFIA: AccountIdOf<TestRuntime> = 62;
	const FRED: AccountIdOf<TestRuntime> = 63;
	const ANNA: AccountIdOf<TestRuntime> = 64;
	const DAMIAN: AccountIdOf<TestRuntime> = 65;

	let accounts = [ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];

	let bounded_name = bounded_name();
	let bounded_symbol = bounded_symbol();
	let metadata_hash = ipfs_hash();
	let normalized_price = PriceOf::<TestRuntime>::from_float(10.0);
	let decimal_aware_price =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(normalized_price, USD_DECIMALS, CT_DECIMALS)
			.unwrap();
	let project_metadata = ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
		mainnet_token_max_supply: 8_000_000 * CT_UNIT,
		total_allocation_size: 100_000 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: decimal_aware_price,
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
		funding_destination_account: 0,
		policy_ipfs_cid: Some(metadata_hash),
		participants_account_type: ParticipantsAccountType::Polkadot,
	};

	// overfund with plmc
	let plmc_fundings =
		accounts.iter().map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: PLMC * 1_000_000 }).collect_vec();
	let usdt_fundings = accounts
		.iter()
		.map(|acc| UserToFundingAsset {
			account: *acc,
			asset_amount: USD_UNIT * 1_000_000,
			asset_id: AcceptedFundingAsset::USDT.id(),
		})
		.collect_vec();
	inst.mint_plmc_to(plmc_fundings);
	inst.mint_funding_asset_to(usdt_fundings);

	let project_id = inst.create_auctioning_project(project_metadata.clone(), 0, None, default_evaluations());

	let bids = vec![
		(ADAM, 10_000 * CT_UNIT).into(),
		(TOM, 20_000 * CT_UNIT).into(),
		(SOFIA, 20_000 * CT_UNIT).into(),
		(FRED, 10_000 * CT_UNIT).into(),
		(ANNA, 5_000 * CT_UNIT).into(),
		(DAMIAN, 5_000 * CT_UNIT).into(),
	];

	inst.bid_for_users(project_id, bids).unwrap();

	assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));

	let project_details = inst.get_project_details(project_id);
	let wap = project_details.weighted_average_price.unwrap();
	let bucket = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());
	let dry_run_price = bucket
		.calculate_wap(project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size);

	assert_eq!(dry_run_price, wap);
}

#[test]
fn find_bucket_for_wap() {
	let mut inst = tests::MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	const ADAM: AccountIdOf<TestRuntime> = 60;
	const TOM: AccountIdOf<TestRuntime> = 61;
	const SOFIA: AccountIdOf<TestRuntime> = 62;
	const FRED: AccountIdOf<TestRuntime> = 63;
	const ANNA: AccountIdOf<TestRuntime> = 64;
	const DAMIAN: AccountIdOf<TestRuntime> = 65;

	let accounts = [ADAM, TOM, SOFIA, FRED, ANNA, DAMIAN];

	let bounded_name = bounded_name();
	let bounded_symbol = bounded_symbol();
	let metadata_hash = ipfs_hash();
	let normalized_price = PriceOf::<TestRuntime>::from_float(10.0);
	let decimal_aware_price =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(normalized_price, USD_DECIMALS, CT_DECIMALS)
			.unwrap();
	let project_metadata = ProjectMetadata {
		token_information: CurrencyMetadata { name: bounded_name, symbol: bounded_symbol, decimals: CT_DECIMALS },
		mainnet_token_max_supply: 8_000_000 * CT_UNIT,
		total_allocation_size: 100_000 * CT_UNIT,
		auction_round_allocation_percentage: Percent::from_percent(50u8),
		minimum_price: decimal_aware_price,
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
		funding_destination_account: 0,
		policy_ipfs_cid: Some(metadata_hash),
		participants_account_type: ParticipantsAccountType::Polkadot,
	};

	// overfund with plmc
	let plmc_fundings =
		accounts.iter().map(|acc| UserToPLMCBalance { account: *acc, plmc_amount: PLMC * 1_000_000 }).collect_vec();
	let usdt_fundings = accounts
		.iter()
		.map(|acc| UserToFundingAsset {
			account: *acc,
			asset_amount: USD_UNIT * 1_000_000,
			asset_id: AcceptedFundingAsset::USDT.id(),
		})
		.collect_vec();
	inst.mint_plmc_to(plmc_fundings);
	inst.mint_funding_asset_to(usdt_fundings);

	let project_id = inst.create_auctioning_project(project_metadata.clone(), 0, None, default_evaluations());

	let bids = vec![
		(ADAM, 10_000 * CT_UNIT).into(),
		(TOM, 20_000 * CT_UNIT).into(),
		(SOFIA, 20_000 * CT_UNIT).into(),
		(FRED, 10_000 * CT_UNIT).into(),
		(ANNA, 5_000 * CT_UNIT).into(),
		(DAMIAN, 5_000 * CT_UNIT).into(),
	];

	inst.bid_for_users(project_id, bids).unwrap();

	assert!(matches!(inst.go_to_next_state(project_id), ProjectStatus::CommunityRound(_)));

	let project_details = inst.get_project_details(project_id);
	let wap = project_details.weighted_average_price.unwrap();
	let bucket_stored = inst.execute(|| Buckets::<TestRuntime>::get(project_id).unwrap());

	let bucket_found = inst.find_bucket_for_wap(project_metadata.clone(), wap);
	assert_eq!(bucket_found, bucket_stored);

	let wap_found = bucket_found
		.calculate_wap(project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size);
	assert_eq!(wap_found, wap);
}

#[test]
fn generate_bids_from_bucket() {
	let mut inst = tests::MockInstantiator::new(Some(RefCell::new(new_test_ext())));

	// Has a min price of 10.0
	let project_metadata = default_project_metadata(0);
	let desired_real_wap = FixedU128::from_float(20.0f64);
	let desired_price_aware_wap =
		PriceProviderOf::<TestRuntime>::calculate_decimals_aware_price(desired_real_wap, USD_DECIMALS, CT_DECIMALS)
			.unwrap();
	let necessary_bucket = inst.find_bucket_for_wap(project_metadata.clone(), desired_price_aware_wap);
	let bids = inst.generate_bids_from_bucket(
		project_metadata.clone(),
		necessary_bucket,
		420,
		|x| x + 1,
		AcceptedFundingAsset::USDT,
	);
	let project_id =
		inst.create_community_contributing_project(project_metadata.clone(), 0, None, default_evaluations(), bids);
	let project_details = inst.get_project_details(project_id);
	let wap = project_details.weighted_average_price.unwrap();
	assert_eq!(wap, desired_price_aware_wap);
}
