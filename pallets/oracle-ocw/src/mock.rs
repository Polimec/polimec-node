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
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, Hooks, IsInVec, Time},
};
use parking_lot::RwLock;
use sp_core::{
	offchain::{
		testing::{self, OffchainState, PoolState},
		OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
	},
	sr25519::Signature,
	Pair, Public,
};
use sp_keystore::{testing::MemoryKeystore, Keystore, KeystoreExt};
use sp_runtime::{
	testing::TestXt,
	traits::{Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage,
};
use sp_std::cell::RefCell;
use std::sync::Arc;

type Block = frame_system::mocking::MockBlock<Test>;
pub type Extrinsic = TestXt<RuntimeCall, ()>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type AccountPublic = <Signature as Verify>::Signer;
type OracleKey = u64;
type OracleValue = FixedU128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = sp_core::sr25519::Public;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Nonce = u64;
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
			AssetName::USDC => (1337, price),
			AssetName::USDT => (1984, price),
			AssetName::PLMC => (3344, price),
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
	type AppCrypto = crate::crypto::Polimec;
	type ConvertAssetPricePair = AssetPriceConverter;
	type FetchInterval = ConstU64<5u64>;
	type FetchWindow = ConstU64<1u64>;
	type Members = IsInVec<Members>;
	type RuntimeEvent = RuntimeEvent;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	RuntimeCall: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = RuntimeCall;
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

construct_runtime!(
	pub enum Test
	{
		System: frame_system::{Pallet, Call, Storage, Config<T>, Event<T>},
		Oracle: orml_oracle::{Pallet, Storage, Call, Event<T>},
		OracleOcw: pallet_oracle_ocw::{Pallet, Event<T>},

	}
);

// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext_with_offchain_storage(
) -> (sp_io::TestExternalities, Arc<RwLock<OffchainState>>, Arc<RwLock<PoolState>>) {
	const PHRASE: &str = "//Alice";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	// let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = MemoryKeystore::new();
	keystore.sr25519_generate_new(crate::crypto::POLIMEC_ORACLE, Some(&format!("{}", PHRASE))).unwrap();

	let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
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
			uri: format!("https://api.kraken.com/0/public/OHLC?pair={}&interval=1", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}

	for (asset, response) in BITFINEX_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://api-pub.bitfinex.com/v2/candles/trade%3A1m%3At{}/hist?limit=15", asset).into(),
			response: Some(response.to_vec()),
			sent: true,
			..Default::default()
		});
	}
	for (asset, response) in BITSTAMP_RESPONSES.iter() {
		state.expect_request(testing::PendingRequest {
			method: "GET".into(),
			uri: format!("https://www.bitstamp.net/api/v2/ohlc/{}/?step=60&limit=15", asset).into(),
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
	state.expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: "https://sapi.xt.com/v4/public/kline?symbol=plmc_usdt&interval=30m&limit=10".into(),
		response: Some(XT_PLMC_CORRECT.to_vec()),
		sent: true,
		..Default::default()
	});
	state.expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: "https://api.mexc.com/api/v3/klines?symbol=PLMCUSDT&interval=30m&limit=10".into(),
		response: Some(MEXC_PLMC_CORRECT.to_vec()),
		sent: true,
		..Default::default()
	});
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
	TPublic::Pair::from_string(&format!("//{}", seed), None).expect("static values are valid; qed").public()
}

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn assert_close_enough(a: FixedU128, b: FixedU128) {
	match a > b {
		true => assert!(a.saturating_sub(b) < FixedU128::from_float(0.001)),
		false => assert!(b.saturating_sub(a) < FixedU128::from_float(0.001)),
	}
}

pub(crate) const KRAKEN_RESPONSES: &[(&str, &[u8])] =
	&[("USDTZUSD", KRAKEN_USDT_CORRECT), ("USDCUSD", KRAKEN_USDC_CORRECT), ("DOTUSD", KRAKEN_DOT_CORRECT)];
const KRAKEN_USDT_CORRECT: &[u8] = br#"{"error":[],"result":{"USDTZUSD":[[1701877920,"1.00009","1.00011","1.00008","1.00009","1.00010","58759.32214931",36],[1701877980,"1.00009","1.00011","1.00009","1.00010","1.00010","17156.51835679",18],[1701878040,"1.00011","1.00011","1.00010","1.00010","1.00010","231514.66903930",13],[1701878100,"1.00010","1.00015","1.00010","1.00014","1.00012","10577.17236868",27],[1701878160,"1.00015","1.00020","1.00015","1.00019","1.00017","1026827.06857105",67],[1701878220,"1.00019","1.00019","1.00018","1.00019","1.00018","44228.73461655",28],[1701878280,"1.00018","1.00018","1.00015","1.00015","1.00016","41144.63245059",23],[1701878340,"1.00014","1.00015","1.00013","1.00013","1.00013","252283.11050904",67],[1701878400,"1.00014","1.00014","1.00012","1.00014","1.00012","34519.85524461",23],[1701878460,"1.00013","1.00013","1.00008","1.00009","1.00010","49702.48469208",40],[1701878520,"1.00009","1.00016","1.00009","1.00016","1.00012","83532.48937609",43],[1701878580,"1.00016","1.00018","1.00015","1.00018","1.00017","340329.29664927",27],[1701878640,"1.00018","1.00018","1.00015","1.00015","1.00016","125875.61559451",33],[1701878700,"1.00015","1.00015","1.00010","1.00011","1.00012","63925.70403795",32],[1701878760,"1.00010","1.00010","1.00008","1.00008","1.00009","53316.20999461",26]],"last":1699977300}}"#;
const KRAKEN_USDC_CORRECT: &[u8] = br#"{"error":[],"result":{"USDCUSD":[[1701878040,"1.0001","1.0001","1.0000","1.0000","1.0000","2210.00000000",2],[1701878100,"1.0002","1.0002","1.0002","1.0002","1.0002","999.00000000",1],[1701878160,"1.0001","1.0002","1.0001","1.0002","1.0001","7201.85053234",9],[1701878220,"1.0001","1.0001","1.0001","1.0001","1.0001","15.71930681",1],[1701878280,"1.0000","1.0001","1.0000","1.0001","1.0000","102108.24129487",5],[1701878340,"1.0001","1.0001","1.0001","1.0001","0.0000","0.00000000",0],[1701878400,"1.0001","1.0001","1.0001","1.0001","1.0001","1451.37880000",1],[1701878460,"1.0001","1.0001","1.0000","1.0000","1.0000","11005.00000000",2],[1701878520,"1.0001","1.0001","1.0000","1.0000","1.0000","6760.93865300",3],[1701878580,"1.0000","1.0000","1.0000","1.0000","0.0000","0.00000000",0],[1701878640,"1.0000","1.0001","1.0000","1.0001","1.0000","1290.84392400",4],[1701878700,"1.0000","1.0001","1.0000","1.0001","1.0000","53.03306930",2],[1701878760,"1.0000","1.0000","1.0000","1.0000","1.0000","16711.33870874",7],[1701878820,"1.0000","1.0000","1.0000","1.0000","1.0000","10007.53328427",2],[1701878880,"0.9999","0.9999","0.9999","0.9999","0.9999","1000.00000000",1]],"last":1699977300}}"#;
const KRAKEN_DOT_CORRECT: &[u8] = br#"{"error":[],"result":{"DOTUSD":[[1701878100,"6.1473","6.1474","6.1473","6.1474","6.1473","102.00000000",2],[1701878160,"6.1446","6.1446","6.1378","6.1409","6.1399","56.11963595",4],[1701878220,"6.1251","6.1473","6.1233","6.1473","6.1268","992.18112927",12],[1701878280,"6.1468","6.1500","6.1383","6.1383","6.1463","365.21179340",29],[1701878340,"6.1401","6.1401","6.1378","6.1378","6.1393","57.06050109",5],[1701878400,"6.1298","6.1380","6.1279","6.1380","6.1361","968.44275786",8],[1701878460,"6.1403","6.1407","6.1390","6.1390","6.1400","507.81565634",8],[1701878520,"6.1391","6.1500","6.1385","6.1500","6.1422","344.07407967",5],[1701878580,"6.1499","6.1573","6.1473","6.1573","6.1491","3563.21894841",32],[1701878640,"6.1594","6.1602","6.1552","6.1552","6.1591","1461.51440086",22],[1701878700,"6.1612","6.1622","6.1544","6.1544","6.1598","447.90016651",9],[1701878760,"6.1452","6.1452","6.1407","6.1407","6.1421","225.30037904",6],[1701878820,"6.1192","6.1192","6.1044","6.1044","6.1145","154.45052403",8],[1701878880,"6.1111","6.1126","6.1082","6.1124","6.1116","186.62943447",4],[1701878940,"6.1126","6.1199","6.1124","6.1197","6.1160","145.34596966",7]],"last":1699977300}}"#;

pub(crate) const BITFINEX_RESPONSES: &[(&str, &[u8])] =
	&[("USTUSD", BITFINEX_USDT_CORRECT), ("UDCUSD", BITFINEX_USDC_CORRECT), ("DOTUSD", BITFINEX_DOT_CORRECT)];
const BITFINEX_USDT_CORRECT: &[u8] = br#"[[1701878700000,1.0005,1.0006,1.0006,1.0005,338.36072124],[1701878640000,1.0007,1.0005,1.0007,1.0005,63517.500237629996],[1701878580000,1.0007,1.0006,1.0007,1.0006,2007.06330507],[1701878520000,1.0007,1.0007,1.0007,1.0006,9546.62273159],[1701878460000,1.0008,1.0007,1.0008,1.0006,132234.98119663],[1701878400000,1.0008,1.0007,1.0008,1.0007,10224.08007082],[1701878340000,1.0008,1.0008,1.0008,1.0007,8716.53280425],[1701878280000,1.0008,1.0008,1.0008,1.0007,37436.46172385],[1701878220000,1.0007,1.0008,1.0008,1.0007,13436.41180859],[1701878160000,1.0008,1.0007,1.0008,1.0007,17947.59874696],[1701878100000,1.0008,1.0008,1.0008,1.0008,8238.112989],[1701878040000,1.0007,1.0008,1.0008,1.0007,4367.83340022],[1701877980000,1.0007,1.0007,1.0008,1.0007,2848.19766728],[1701877920000,1.0008,1.0008,1.0009,1.0008,171039.30620532],[1701877860000,1.0008,1.0008,1.0008,1.0007,184640.26643653]]"#;
const BITFINEX_USDC_CORRECT: &[u8] = br#"[[1701878160000,1.0008,1.0008,1.0008,1.0008,119.85145068],[1701877020000,1.0004,1.0004,1.0004,1.0004,9637.55201485],[1701876660000,1.0005,0.99958,1.0005,0.99958,79514.655813],[1701876480000,1.0006,1.0006,1.0006,1.0006,17539.09],[1701876420000,1.0006,1.0006,1.0006,1.0006,2448.91],[1701876300000,1.0005,1.0005,1.0005,1.0005,144.71014286],[1701875940000,1.0002,1.0002,1.0002,1.0002,692.470892],[1701875700000,1.0002,1.0001,1.0002,1.0001,21000],[1701874920000,1,1,1,1,82.43673123],[1701874020000,1,0.99993,1,0.99993,10990],[1701873840000,1.0003,1.0003,1.0003,1.0003,154.23081876],[1701869880000,1.0003,1.0003,1.0003,1.0003,93.78810427],[1701869220000,1.0003,1.0003,1.0003,1.0003,182.41150567],[1701866580000,0.99988,0.99988,0.99988,0.99988,80.20076041],[1701866040000,0.99982,0.99982,0.99982,0.99982,5000]]"#;
const BITFINEX_DOT_CORRECT: &[u8] = br#"[[1701878460000,6.1441,6.144,6.1441,6.144,8.4],[1701878400000,6.1377,6.1405,6.1405,6.1377,77.36041884],[1701878220000,6.1239,6.1239,6.1239,6.1239,5],[1701878160000,6.1546,6.1546,6.1546,6.1546,72.54925],[1701878100000,6.1574,6.1586,6.1586,6.1574,534.96073475],[1701878040000,6.1602,6.1602,6.1602,6.1602,26.5],[1701877920000,6.1454,6.1454,6.1454,6.1454,3.5],[1701877860000,6.1401,6.1401,6.1401,6.1401,59.49967],[1701877680000,6.118,6.118,6.118,6.118,0.5],[1701877620000,6.1416,6.1416,6.1416,6.1416,72.75138],[1701877020000,6.125,6.125,6.125,6.125,72.89985],[1701876900000,6.1426,6.1426,6.1426,6.1426,33.46011493],[1701876720000,6.1207,6.1207,6.1207,6.1207,4.5],[1701876540000,6.1033,6.1033,6.1033,6.1033,3.9],[1701876480000,6.1098,6.1067,6.1098,6.1067,53]]"#;

pub(crate) const BITSTAMP_RESPONSES: &[(&str, &[u8])] =
	&[("usdtusd", BITSTAMP_USDT_CORRECT), ("usdcusd", BITSTAMP_USDC_CORRECT), ("dotusd", BITSTAMP_DOT_CORRECT)];
const BITSTAMP_USDT_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "1.00010", "high": "1.00010", "low": "1.00010", "open": "1.00010", "timestamp": "1701877320", "volume": "44595.11593"}, {"close": "1.00008", "high": "1.00012", "low": "1.00008", "open": "1.00012", "timestamp": "1701877380", "volume": "4919.14926"}, {"close": "1.00008", "high": "1.00008", "low": "1.00008", "open": "1.00008", "timestamp": "1701877440", "volume": "4211.12929"}, {"close": "1.00009", "high": "1.00009", "low": "1.00009", "open": "1.00009", "timestamp": "1701877500", "volume": "4166.66667"}, {"close": "1.00010", "high": "1.00010", "low": "1.00010", "open": "1.00010", "timestamp": "1701877560", "volume": "4166.66667"}, {"close": "1.00011", "high": "1.00011", "low": "1.00009", "open": "1.00009", "timestamp": "1701877620", "volume": "4217.35212"}, {"close": "1.00011", "high": "1.00011", "low": "1.00011", "open": "1.00011", "timestamp": "1701877680", "volume": "4166.66667"}, {"close": "1.00011", "high": "1.00011", "low": "1.00011", "open": "1.00011", "timestamp": "1701877740", "volume": "4166.66667"}, {"close": "1.00010", "high": "1.00010", "low": "1.00010", "open": "1.00010", "timestamp": "1701877800", "volume": "4166.66666"}, {"close": "1.00009", "high": "1.00009", "low": "1.00008", "open": "1.00008", "timestamp": "1701877860", "volume": "4443.33076"}, {"close": "1.00009", "high": "1.00010", "low": "1.00008", "open": "1.00010", "timestamp": "1701877920", "volume": "4191.94439"}, {"close": "1.00009", "high": "1.00009", "low": "1.00007", "open": "1.00007", "timestamp": "1701877980", "volume": "4166.66666"}, {"close": "1.00009", "high": "1.00009", "low": "1.00009", "open": "1.00009", "timestamp": "1701878040", "volume": "4340.19105"}, {"close": "1.00010", "high": "1.00010", "low": "1.00009", "open": "1.00009", "timestamp": "1701878100", "volume": "6293.29493"}, {"close": "1.00010", "high": "1.00010", "low": "1.00010", "open": "1.00010", "timestamp": "1701878160", "volume": "0.00000"}], "pair": "USDT/USD"}}"#;
const BITSTAMP_USDC_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "1.00000", "high": "1.00000", "low": "1.00000", "open": "1.00000", "timestamp": "1701877380", "volume": "46.46040"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877440", "volume": "87.71000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877500", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877560", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877620", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877680", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877740", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877800", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877860", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877920", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701877980", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701878040", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701878100", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701878160", "volume": "0.00000"}, {"close": "1.00001", "high": "1.00001", "low": "1.00001", "open": "1.00001", "timestamp": "1701878220", "volume": "0.00000"}], "pair": "USDC/USD"}}"#;
const BITSTAMP_DOT_CORRECT: &[u8] = br#"{"data": {"ohlc": [{"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877440", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877500", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877560", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877620", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877680", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877740", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877800", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877860", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877920", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701877980", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701878040", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701878100", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701878160", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701878220", "volume": "0.00"}, {"close": "6.075", "high": "6.075", "low": "6.075", "open": "6.075", "timestamp": "1701878280", "volume": "0.00"}], "pair": "DOT/USD"}}"#;

pub(crate) const COINBASE_RESPONSES: &[(&str, &[u8])] =
	&[("USDT-USD", COINBASE_USDT_CORRECT), ("DOT-USD", COINBASE_DOT_CORRECT)];
const COINBASE_USDT_CORRECT: &[u8] = br#"[[1701879000,1.00004,1.00005,1.00004,1.00005,186523.02],[1701878940,1.00004,1.00007,1.00006,1.00004,308533.8],[1701878880,1.00006,1.00007,1.00007,1.00007,139383.41],[1701878820,1.00005,1.00008,1.00008,1.00006,670513.37],[1701878760,1.00006,1.00008,1.00007,1.00007,321319.05],[1701878700,1.00006,1.00007,1.00007,1.00007,154885.47],[1701878640,1.00006,1.00007,1.00006,1.00006,138635.74],[1701878580,1.00006,1.00007,1.00007,1.00007,101704.53],[1701878520,1,1.00007,1,1.00007,476611.79],[1701878460,1,1.00006,1.00006,1.00001,469500.28],[1701878400,1.00005,1.00006,1.00005,1.00005,243522.52],[1701878340,1.00005,1.00007,1.00006,1.00006,209765.12],[1701878280,1.00006,1.00007,1.00006,1.00006,166088.23],[1701878220,1.00006,1.00007,1.00006,1.00006,187521.23],[1701878160,1.00006,1.00007,1.00007,1.00006,237587.95]]"#;
const COINBASE_DOT_CORRECT: &[u8] = br#"[[1701879120,6.138,6.15,6.142,6.15,446.879],[1701879060,6.128,6.141,6.13,6.141,988.289],[1701879000,6.118,6.131,6.12,6.13,418.213],[1701878940,6.108,6.119,6.109,6.119,1185.35],[1701878880,6.106,6.116,6.109,6.109,1666.431],[1701878820,6.103,6.124,6.124,6.108,5914.445],[1701878760,6.125,6.149,6.149,6.125,2211.268],[1701878700,6.147,6.163,6.159,6.147,2177.716],[1701878640,6.155,6.166,6.161,6.162,2345.747],[1701878580,6.146,6.16,6.154,6.16,3245.27],[1701878520,6.132,6.153,6.136,6.153,4739.668],[1701878460,6.137,6.145,6.137,6.138,2349.617],[1701878400,6.125,6.145,6.127,6.141,6026.081],[1701878340,6.125,6.141,6.136,6.125,1725.614],[1701878280,6.133,6.156,6.145,6.133,2828.037]]"#;

pub(crate) const XT_PLMC_CORRECT: &[u8] = br#"{"rc":0,"mc":"SUCCESS","ma":[],"result":[{"t":1711612800000,"o":"0.413","c":"0.413","h":"0.413","l":"0.413","q":"310.3300","v":"128.16629"},{"t":1711609200000,"o":"0.413","c":"0.413","h":"0.413","l":"0.413","q":"1650.0000","v":"681.450"},{"t":1711605600000,"o":"0.413","c":"0.413","h":"0.413","l":"0.413","q":"157.4729","v":"65.0363077"},{"t":1711604700000,"o":"0.417","c":"0.414","h":"0.417","l":"0.414","q":"181.6950","v":"75.4584304"},{"t":1711603800000,"o":"0.413","c":"0.416","h":"0.416","l":"0.413","q":"195.6683","v":"81.1338045"},{"t":1711602900000,"o":"0.410","c":"0.412","h":"0.412","l":"0.410","q":"186.1684","v":"76.4780487"},{"t":1711602000000,"o":"0.407","c":"0.407","h":"0.407","l":"0.407","q":"2.8544","v":"1.1617408"},{"t":1711601100000,"o":"0.418","c":"0.410","h":"0.418","l":"0.404","q":"1433.2745","v":"586.6181797"},{"t":1711599300000,"o":"0.410","c":"0.420","h":"0.420","l":"0.410","q":"897.5851","v":"374.447996"},{"t":1711598400000,"o":"0.408","c":"0.408","h":"0.408","l":"0.408","q":"12.6320","v":"5.153856"}]}"#;

pub(crate) const MEXC_PLMC_CORRECT: &[u8] = br#"[[1711610100000,"0.415","0.415","0.415","0.415","0.0",1711611000000,"0.0"],[1711611000000,"0.415","0.415","0.415","0.415","19.64",1711611900000,"8.1506"],[1711611900000,"0.415","0.4176","0.415","0.4176","391.68",1711612800000,"163.5655"],[1711612800000,"0.4176","0.4194","0.4176","0.4194","526.57",1711613700000,"220.298"],[1711613700000,"0.4194","0.4194","0.4194","0.4194","0.0",1711614600000,"0.0"],[1711614600000,"0.4194","0.4194","0.4194","0.4194","0.0",1711615500000,"0.0"],[1711615500000,"0.4194","0.4194","0.4194","0.4194","0.0",1711616400000,"0.0"],[1711616400000,"0.4194","0.4194","0.4194","0.4194","0.0",1711617300000,"0.0"],[1711617300000,"0.4194","0.4194","0.4194","0.4194","0.0",1711618200000,"0.0"],[1711618200000,"0.4194","0.4194","0.4194","0.4194","0.0",1711619100000,"0.0"]]"#;
