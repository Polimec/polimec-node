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

#![cfg(test)]

use super::*;
use crate as pallet_oracle_ocw;


use frame_support::{
	construct_runtime, parameter_types,
	traits::{ConstU32, ConstU64, Everything, IsInVec, Hooks, Time},
};
use sp_core::{
	offchain::{testing::{self, OffchainState, PoolState}, OffchainWorkerExt, TransactionPoolExt, OffchainDbExt},
	sr25519::Signature,
	Pair,
	H256, Public
};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};
use sp_std::cell::RefCell;
use std::sync::Arc;
use parking_lot::RwLock;


pub type Extrinsic = TestXt<RuntimeCall, ()>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type AccountPublic = <Signature as Verify>::Signer;
type OracleKey = u64;
type OracleValue = FixedU128;

impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}


thread_local! {
	static TIME: RefCell<u32> = RefCell::new(0);
}

pub struct Timestamp;
impl Time for Timestamp {
	type Moment = u32;

	fn now() -> Self::Moment {
		TIME.with(|v| *v.borrow())
	}
}

impl Timestamp {
	pub fn set_timestamp(val: u32) {
		TIME.with(|v| *v.borrow_mut() = val);
	}
}

parameter_types! {
	pub RootOperatorAccountId: AccountId = AccountId::from_raw([0xffu8; 32]);
	pub const MaxFeedValues: u32 = 4; // max 4 values allowd to feed in one call (USDT, USDC, DOT, PLMC).
}

impl orml_oracle::Config for Test {
	type CombineData = orml_oracle::DefaultCombineData<Test, ConstU32<3>, ConstU32<10>, ()>;
	type MaxFeedValues = MaxFeedValues;
	type MaxHasDispatchedSize = ConstU32<20>;
	type Members = IsInVec<Members>;
	type OnNewData = ();
	type OracleKey = OracleKey;
	type OracleValue = OracleValue;
	type RootOperatorAccountId = RootOperatorAccountId;
	type RuntimeEvent = RuntimeEvent;
	type Time = Timestamp;
	// TODO Add weight info
	type WeightInfo = ();
}

pub struct AssetPriceConverter;
impl Convert<(AssetName, FixedU128), (OracleKey, OracleValue)> for AssetPriceConverter {
	fn convert((asset, price): (AssetName, FixedU128)) -> (OracleKey, OracleValue) {
		match asset {
			AssetName::DOT => (0, price),
			AssetName::USDC => (420, price),
			AssetName::USDT => (1984, price),
			AssetName::PLMC => (2069, price),
		}
	}
}

parameter_types! {
	pub static Members: Vec<AccountId> = vec![
		get_account_id_from_seed::<crate::crypto::AuthorityId>("Alice"),
		get_account_id_from_seed::<crate::crypto::AuthorityId>("Bob"),
		get_account_id_from_seed::<crate::crypto::AuthorityId>("Charlie"),	
	];
}
impl Config for Test {
	type AuthorityId = crate::crypto::AuthorityId;
	type AppCrypto = crate::crypto::PolimecCrypto;
	type RuntimeEvent = RuntimeEvent;
	type Members = IsInVec<Members>;
	type GracePeriod = ConstU64<5u64>;
	type ConvertAssetPricePair = AssetPriceConverter;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Oracle: orml_oracle::{Pallet, Storage, Call, Event<T>},
		OracleOcw: pallet_oracle_ocw::{Pallet, Storage, Call, Event<T>},

	}
);

// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext_with_offchain_storage() -> (sp_io::TestExternalities, Arc<RwLock<OffchainState>>, Arc<RwLock<PoolState>>) {
	const PHRASE: &str =
	"//Alice";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	// let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = MemoryKeystore::new();
	keystore
		.sr25519_generate_new(crate::crypto::POLIMEC_ORACLE, Some(&format!("{}", PHRASE)))
		.unwrap();

	let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut t: sp_io::TestExternalities = storage.into();
	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt::new(keystore));
	t.register_extension(OffchainDbExt::new(offchain.clone()));
	(t, offchain_state, pool_state)
}

pub fn price_oracle_response(state: &mut testing::OffchainState) {
	for (asset, response) in KRAKEN_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://api.kraken.com/0/public/OHLC?pair={}", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}

	for (asset, response) in BITFINEX_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3At{}/hist?limit=10", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}
	for (asset, response) in BITSTAMP_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://www.bitstamp.net/api/v2/ohlc/{}/?step=60&limit=10", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}
	for (asset, response) in COINBASE_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://api.exchange.coinbase.com/products/{}/candles?granularity=60", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}

}
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		OracleOcw::offchain_worker(System::block_number());
		Oracle::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Oracle::on_initialize(System::block_number());
	}
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn assert_close_enough(a: FixedU128, b: FixedU128) {
	match a > b {
		true => assert!(a.saturating_sub(b) < FixedU128::from_float(0.0001)),
		false => assert!(b.saturating_sub(a) < FixedU128::from_float(0.0001)),
	}
}

const KRAKEN_RESPONSES: &[(&str, &[u8])] = &[("USDTZUSD", KRAKEN_USDT_CORRECT), ("USDCUSD", KRAKEN_USDC_CORRECT), ("DOTUSD", KRAKEN_DOT_CORRECT)];
const KRAKEN_USDT_CORRECT: &[u8] = br#"{"error":[],"result":{"USDTZUSD":[[1700475540,"1.00066","1.00075","1.00065","1.00075","1.00068","73854.10842030",35],[1700475600,"1.00067","1.00067","1.00066","1.00067","1.00066","3465.67416068",22],[1700475660,"1.00067","1.00069","1.00066","1.00069","1.00067","10753.75324059",23],[1700475720,"1.00069","1.00069","1.00068","1.00069","1.00068","25295.84864035",5],[1700475780,"1.00069","1.00070","1.00068","1.00069","1.00068","17999.31561162",12],[1700475840,"1.00069","1.00069","1.00068","1.00069","1.00068","15709.87536875",19],[1700475900,"1.00069","1.00073","1.00068","1.00070","1.00070","11172.25768848",24],[1700475960,"1.00071","1.00071","1.00068","1.00068","1.00069","134962.72513223",33],[1700476020,"1.00069","1.00074","1.00068","1.00071","1.00072","43299.87049457",31],[1700476080,"1.00071","1.00072","1.00068","1.00068","1.00068","25357.91891412",18]],"last":1699977300}}"#;
const KRAKEN_USDC_CORRECT: &[u8] = br#"{"error":[],"result":{"USDCUSD":[[1700475240,"1.00057","1.00057","1.00057","1.00057","1.00057","1255.78439392",4],[1700475300,"1.00057","1.00067","1.00056","1.00061","1.00061","212769.41344068",51],[1700475360,"1.00062","1.00064","1.00062","1.00064","1.00063","3196.83054742",10],[1700475420,"1.00065","1.00065","1.00064","1.00065","1.00064","8228.32358743",10],[1700475480,"1.00064","1.00066","1.00064","1.00065","1.00064","38634.74502224",12],[1700475540,"1.00066","1.00075","1.00065","1.00075","1.00068","73854.10842030",35],[1700475600,"1.00067","1.00067","1.00066","1.00067","1.00066","3465.67416068",22],[1700475660,"1.00067","1.00069","1.00066","1.00069","1.00067","10753.75324059",23],[1700475720,"1.00069","1.00069","1.00068","1.00069","1.00068","25295.84864035",5],[1700475780,"1.00069","1.00069","1.00068","1.00069","1.00068","4424.45753386",4]],"last":1699977300}}"#;
const KRAKEN_DOT_CORRECT: &[u8] = br#"{"error":[],"result":{"DOTUSD":[[1700475660,"5.5215","5.5215","5.5215","5.5215","5.5215","2.00000000",1],[1700475720,"5.5230","5.5230","5.5230","5.5230","5.5230","2.00000000",1],[1700475780,"5.5207","5.5207","5.5207","5.5207","5.5207","2.00000000",1],[1700475840,"5.5196","5.5196","5.5196","5.5196","5.5196","2.00000000",1],[1700475900,"5.5196","5.5196","5.5196","5.5196","5.5196","2.00000000",1],[1700475960,"5.5196","5.5196","5.5196","5.5196","5.5196","2.00000000",1],[1700476020,"5.5196","5.5196","5.5171","5.5171","5.5193","286.20415870",4],[1700476080,"5.5171","5.5171","5.5171","5.5171","5.5171","2.00000000",1],[1700476140,"5.5171","5.5171","5.5150","5.5155","5.5161","563.21856413",5],[1700476200,"5.5183","5.5183","5.5183","5.5183","5.5183","2.00000000",1]],"last":1699977300}}"#;


const BITFINEX_RESPONSES: &[(&str, &[u8])] = &[("USTUSD", BITFINEX_USDT_CORRECT), ("UDCUSD", BITFINEX_USDC_CORRECT), ("DOTUSD", BITFINEX_DOT_CORRECT)];
const BITFINEX_USDT_CORRECT: &[u8] = br#"[[1700478840000,1.0007,1.0007,1.0008,1.0007,1548.35810321],[1700478780000,1.0008,1.0008,1.0008,1.0008,37.270741],[1700478660000,1.0007,1.0007,1.0007,1.0007,61],[1700478600000,1.0008,1.0008,1.0008,1.0007,394.56929104],[1700478540000,1.0008,1.0007,1.0008,1.0007,3393.71293792],[1700478480000,1.0007,1.0008,1.0008,1.0007,1460.53265868],[1700478360000,1.0007,1.0008,1.0008,1.0007,333.66388341],[1700478240000,1.0008,1.0007,1.0008,1.0007,1052.32905892],[1700478120000,1.0007,1.0008,1.0008,1.0007,7580.65],[1700478060000,1.0007,1.0007,1.0007,1.0007,7612]]"#;
const BITFINEX_USDC_CORRECT: &[u8] = br#"[[1700478480000,1.0003,1.0003,1.0003,1.0003,463.99162003],[1700476020000,1.0002,1.0002,1.0002,1.0002,463.96861848],[1700475960000,1.0003,1.0003,1.0003,1.0003,400],[1700475540000,1.0002,1.0002,1.0002,1.0002,15000],[1700473740000,1.0002,1.0002,1.0002,1.0002,112.66045577],[1700472240000,1.0002,1.0002,1.0002,1.0002,80.12814816],[1700470620000,1.0002,1.0002,1.0002,1.0002,2170],[1700470020000,1.0002,1.0002,1.0002,1.0002,463.99527657],[1700469900000,1.0002,1.0002,1.0002,1.0002,463.99356619],[1700469780000,1.0002,0.99995,1.0002,0.99995,927.94126093]]"#;
const BITFINEX_DOT_CORRECT: &[u8] = br#"[[1700478300000,5.5449,5.5449,5.5449,5.5449,7.80409445],[1700478000000,5.5213,5.5213,5.5213,5.5213,5.01764941],[1700477880000,5.5257,5.5257,5.5257,5.5257,31.67912468],[1700476560000,5.5303,5.5303,5.5303,5.5303,31.65219237],[1700476500000,5.5393,5.5304,5.5393,5.5304,292.43230468],[1700476440000,5.5244,5.5298,5.5339,5.5244,1076.07937896],[1700475900000,5.5213,5.5213,5.5213,5.5213,1057.10158839],[1700475840000,5.5213,5.5213,5.5213,5.5213,750],[1700475720000,5.5261,5.53,5.53,5.5261,182.04953265],[1700475420000,5.5213,5.5213,5.5213,5.5213,7.78129792]]"#;

const BITSTAMP_RESPONSES: &[(&str, &[u8])] = &[("usdtusd", BITSTAMP_USDT_CORRECT), ("usdcusd", BITSTAMP_USDC_CORRECT), ("dotusd", BITSTAMP_DOT_CORRECT)];
const BITSTAMP_USDT_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "1.00064", "high": "1.00064", "low": "1.00064", "open": "1.00064", "timestamp": "1700480580", "volume": "429.09102"}, {"close": "1.00064", "high": "1.00064", "low": "1.00064", "open": "1.00064", "timestamp": "1700480640", "volume": "0.00000"}, {"close": "1.00063", "high": "1.00064", "low": "1.00063", "open": "1.00064", "timestamp": "1700480700", "volume": "3419.48810"}, {"close": "1.00075", "high": "1.00075", "low": "1.00055", "open": "1.00063", "timestamp": "1700480760", "volume": "31990.03850"}, {"close": "1.00076", "high": "1.00076", "low": "1.00076", "open": "1.00076", "timestamp": "1700480820", "volume": "215.12491"}, {"close": "1.00068", "high": "1.00068", "low": "1.00068", "open": "1.00068", "timestamp": "1700480880", "volume": "50.01603"}, {"close": "1.00075", "high": "1.00075", "low": "1.00075", "open": "1.00075", "timestamp": "1700480940", "volume": "227.50203"}, {"close": "1.00075", "high": "1.00075", "low": "1.00067", "open": "1.00067", "timestamp": "1700481000", "volume": "36984.27943"}, {"close": "1.00075", "high": "1.00075", "low": "1.00075", "open": "1.00075", "timestamp": "1700481060", "volume": "0.00000"}, {"close": "1.00075", "high": "1.00075", "low": "1.00075", "open": "1.00075", "timestamp": "1700481120", "volume": "0.00000"}], "pair": "USDT/USD"}}"#;
const BITSTAMP_USDC_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700480940", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481000", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481060", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481120", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481180", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481240", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481300", "volume": "211.05991"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481360", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481420", "volume": "0.00000"}, {"close": "0.99999", "high": "0.99999", "low": "0.99999", "open": "0.99999", "timestamp": "1700481480", "volume": "0.00000"}], "pair": "USDC/USD"}}"#;
const BITSTAMP_DOT_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700480940", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481000", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481060", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481120", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481180", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481240", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481300", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481360", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481420", "volume": "0.00"}, {"close": "5.528", "high": "5.528", "low": "5.528", "open": "5.528", "timestamp": "1700481480", "volume": "0.00"}], "pair": "DOT/USD"}}"#;

const COINBASE_RESPONSES: &[(&str, &[u8])] = &[("USDT-USD", COINBASE_USDT_CORRECT), ("DOT-USD", COINBASE_DOT_CORRECT)];
const COINBASE_DOT_CORRECT: &[u8] = br#"[[1700816520,5.186,5.186,5.186,5.186,94.746],[1700816460,5.184,5.184,5.184,5.184,275.511],[1700816400,5.184,5.187,5.187,5.184,141.659],[1700816340,5.187,5.19,5.19,5.188,137.801],[1700816280,5.186,5.19,5.19,5.189,836.536],[1700816220,5.188,5.191,5.188,5.191,276.465],[1700816160,5.188,5.191,5.188,5.191,275.596],[1700816100,5.187,5.195,5.195,5.188,347.517],[1700816040,5.191,5.195,5.193,5.195,1228.117],[1700815980,5.193,5.195,5.193,5.195,348.33]]"#;
const COINBASE_USDT_CORRECT: &[u8] = br#"[[1700816760,1.00019,1.0002,1.00019,1.00019,1906.98],[1700816700,1.00019,1.0002,1.00019,1.00019,12777.93],[1700816640,1.00019,1.0002,1.0002,1.00019,44539.34],[1700816580,1.00019,1.0002,1.0002,1.0002,41082.59],[1700816520,1.00019,1.0002,1.00019,1.0002,35282.02],[1700816460,1.00019,1.0002,1.00019,1.00019,810310.54],[1700816400,1.00019,1.0002,1.00019,1.0002,6861.88],[1700816340,1.00018,1.0002,1.00019,1.00019,2004241.23],[1700816280,1.00018,1.00019,1.00019,1.00019,62291.07],[1700816220,1.00018,1.00019,1.00019,1.00019,46835.67]]"#;