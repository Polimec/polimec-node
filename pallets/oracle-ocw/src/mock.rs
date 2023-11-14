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
	traits::{ConstU32, ConstU64, Everything, SortedMembers},
};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_core::{
	offchain::{testing::{self, OffchainState}, OffchainWorkerExt},
	H256
};
use std::sync::Arc;
use parking_lot::RwLock;
pub type AccountId = u128;


impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
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

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
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
		OracleOcw: pallet_oracle_ocw::{Pallet, Storage, Call, Event<T>},
	}
);

// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext_with_offchain_storage() -> (sp_io::TestExternalities, Arc<RwLock<OffchainState>>) {

	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	// let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	// let keystore = MemoryKeystore::new();
	let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut t: sp_io::TestExternalities = storage.into();
	t.register_extension(OffchainWorkerExt::new(offchain));
	// t.register_extension(TransactionPoolExt::new(pool));
	// t.register_extension(KeystoreExt::new(keystore));
	(t, offchain_state)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let (t, _) = new_test_ext_with_offchain_storage();
	t
}

pub fn price_oracle_response(state: &mut testing::OffchainState) {
	state.expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: "https://api.kraken.com/0/public/OHLC?pair=USDTZUSD".into(),
		response: Some(KRAKEN_CORRECT_RESPONSE.to_vec()),
		sent: true,
		..Default::default()
	});
}


const KRAKEN_CORRECT_RESPONSE: &[u8] = br#"{"error":[],"result":{"USDCUSD":[[1699976820,"1.0001","1.0001","1.0000","1.0000","1.0000","87.76140000",6],[1699976880,"1.0000","1.0000","1.0000","1.0000","0.0000","0.00000000",0],[1699976940,"1.0000","1.0000","1.0000","1.0000","0.0000","0.00000000",0],[1699977000,"1.0000","1.0000","1.0000","1.0000","1.0000","18.39960000",1],[1699977060,"1.0001","1.0001","1.0001","1.0001","1.0001","2399.55004500",1],[1699977120,"1.0001","1.0001","1.0000","1.0001","1.0000","30692.69650000",3],[1699977180,"1.0000","1.0000","1.0000","1.0000","1.0000","345.80540000",2],[1699977240,"1.0000","1.0000","1.0000","1.0000","1.0000","8.20910000",1],[1699977300,"1.0001","1.0001","1.0001","1.0001","1.0001","36.73683675",1],[1699977360,"1.0001","1.0001","1.0001","1.0001","0.0000","0.00000000",0]],"last":1699977300}}"#;
const KRAKEN_ERROR_RESPONSE: &[u8] = br#"{"error":["EQuery:Unknown asset pair"]}"#;