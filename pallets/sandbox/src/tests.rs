use frame_support::{assert_err, assert_noop, assert_ok};
use frame_support::dispatch::Weight;
use frame_support::traits::{OnFinalize, OnIdle, OnInitialize};
use sp_core::bounded::BoundedVec;
use sp_core::ConstU32;
use sp_runtime::BuildStorage;
use pallet_funding::{CurrencyMetadata, ParticipantsSize, Project, TicketSize};
use crate::mock::*;


#[test]
fn test_buy_if_popular() {
	new_test_ext().execute_with(|| {
		let creator = 1;
		let evaluator = 2;
		let bidder = 3;
		let contributor = 4;

		let project = default_project(0);
		assert_ok!(FundingModule::create(
			RuntimeOrigin::signed(creator),
			project.clone(),
		));
		assert_ok!(FundingModule::start_evaluation(RuntimeOrigin::signed(creator), 0));
		assert_ok!(FundingModule::bond_evaluation(RuntimeOrigin::signed(evaluator), 0, 120_000 * PLMC));

		// advance time
		for _block in 0..<TestRuntime as pallet_funding::Config>::EvaluationDuration::get() + 10 {
			<AllPalletsWithoutSystem as OnFinalize<u64>>::on_finalize(System::block_number());
			<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(
				System::block_number(),
				Weight::MAX,
			);
			System::set_block_number(System::block_number() + 1);
			<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
		}

		assert_ok!(FundingModule::start_auction(RuntimeOrigin::signed(creator), 0));

		// advance time
		for _block in 0..2 {
			<AllPalletsWithoutSystem as OnFinalize<u64>>::on_finalize(System::block_number());
			<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(
				System::block_number(),
				Weight::MAX,
			);
			System::set_block_number(System::block_number() + 1);
			<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
		}

		assert_ok!(FundingModule::bid(RuntimeOrigin::signed(bidder), 0, 1000, 100 * PLMC, None));


		// advance time
		for _block in 0..(<TestRuntime as pallet_funding::Config>::EnglishAuctionDuration::get() + <TestRuntime as pallet_funding::Config>::CandleAuctionDuration::get() + 5) {
			<AllPalletsWithoutSystem as OnFinalize<u64>>::on_finalize(System::block_number());
			<AllPalletsWithoutSystem as OnIdle<u64>>::on_idle(
				System::block_number(),
				Weight::MAX,
			);
			System::set_block_number(System::block_number() + 1);
			<AllPalletsWithSystem as OnInitialize<u64>>::on_initialize(System::block_number());
		}

		assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(contributor), 0, 1));

		assert!(Sandbox::buy_if_popular(RuntimeOrigin::signed(4), 0, 1000).is_err());

		assert_ok!(FundingModule::contribute(RuntimeOrigin::signed(contributor), 0, 10000));

		assert_ok!(Sandbox::buy_if_popular(RuntimeOrigin::signed(4), 0, 1000));

	});
}

const ASSET_DECIMALS: u8 = 12;
const METADATA: &str = r#"
{
	"whitepaper":"ipfs_url",
	"team_description":"ipfs_url",
	"tokenomics":"ipfs_url",
	"roadmap":"ipfs_url",
	"usage_of_founds":"ipfs_url"
}"#;



pub fn default_project(
	nonce: u64,
) -> Project<BoundedVec<u8, ConstU32<64>>, u128, sp_core::H256> {
	let bounded_name =
		BoundedVec::try_from("Contribution Token TEST".as_bytes().to_vec()).unwrap();
	let bounded_symbol = BoundedVec::try_from("CTEST".as_bytes().to_vec()).unwrap();
	let metadata_hash = hashed(format!("{}-{}", METADATA, nonce));
	Project {
		total_allocation_size: 1_000_000,
		minimum_price: 1 * PLMC,
		ticket_size: TicketSize { minimum: Some(1), maximum: None },
		participants_size: ParticipantsSize { minimum: Some(2), maximum: None },
		funding_thresholds: Default::default(),
		conversion_rate: 0,
		participation_currencies: Default::default(),
		metadata: Some(metadata_hash),
		token_information: CurrencyMetadata {
			name: bounded_name,
			symbol: bounded_symbol,
			decimals: ASSET_DECIMALS,
		},
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();

	GenesisConfig {
		balances: BalancesConfig { balances: vec![
			(1, 1_000_000 * PLMC),
			(2, 1_000_000 * PLMC),
			(3, 1_000_000 * PLMC),
			(4, 10_000_000 * PLMC),
		] },
		credentials: CredentialsConfig {
			issuers: vec![1, 16558220937623665250],
			retails: vec![2],
			professionals: vec![2, 3],
			institutionals: vec![4],
		},
		..Default::default()
	}
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	// In order to emit events the block number must be more than 0
	ext.execute_with(|| System::set_block_number(1));
	ext
}

