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

#[allow(unused_imports)]
use crate::*;

// Substrate
use frame_support::storage::unhashed;
use sp_runtime::AccountId32;

#[cfg(feature = "try-runtime")]
use sp_core::crypto::Ss58Codec;

#[cfg(feature = "try-runtime")]
use pallet_vesting::Vesting;

#[cfg(feature = "try-runtime")]
use log;

// The `VestingInfo` fields from `pallet_vesting` are private, so we need to define them here.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode, sp_runtime::RuntimeDebug, Eq, PartialEq)]
struct VestingInfo {
	locked: Balance,
	per_block: Balance,
	starting_block: BlockNumber,
}

pub struct UnhashedMigration;
impl frame_support::traits::OnRuntimeUpgrade for UnhashedMigration {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
		log::info!("Pre-upgrade");
		check_balances();
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
		log::info!("Post-upgrade");
		check_balances();
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// This account received a wrong vesting schedule.
		// Hex encoded representation of 5Ag8zhuoZjKzc3YzmkWFrrmU5GvxdHLtpAN425RW9ZgWS5V7.
		let acct: AccountId32 =
			hex_literal::hex!["c28dbf096b5acf3c0d87dd8ef8cabea0794cc72200a2368751a0fe470d5f9f69"].into();

		// The vesting.Vesting(5Ag8zhuoZjKzc3YzmkWFrrmU5GvxdHLtpAN425RW9ZgWS5V7) encoded storage key.
		const ENCODED_STORAGE_KEY: &str =
"0x5f27b51b5ec208ee9cb25b55d87282435f27b51b5ec208ee9cb25b55d872824334f5503ce555ea3ee18396f4bde1b40bc28dbf096b5acf3c0d87dd8ef8cabea0794cc72200a2368751a0fe470d5f9f69";

		if let Ok(k) = array_bytes::hex2bytes(ENCODED_STORAGE_KEY) {
			// If `is_some` which means it has a vesting schedule, that we could potentially have to correct.
			// +1 R
			if let Some(value) = unhashed::get::<Vec<VestingInfo>>(&k) {
				let v = vec![
					VestingInfo { locked: 119574300000000, per_block: 182000456, starting_block: 249000 },
					VestingInfo { locked: 6485400000000, per_block: 9870000, starting_block: 249000 },
				];
				// Idempotent check.
				if value != v {
					log::info!("⚠️ Correcting storage for {:?}", acct.encode());
					// +1 W
					unhashed::put::<Vec<VestingInfo>>(&k, &v);
				} else {
					log::info!("✅ Storage for {:?} is already correct", acct.encode());
				}
			}
		}

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}
}

#[cfg(feature = "try-runtime")]
fn check_balances() {
	let acct: AccountId32 =
		hex_literal::hex!["c28dbf096b5acf3c0d87dd8ef8cabea0794cc72200a2368751a0fe470d5f9f69"].into();
	let balance = Balances::balance(&acct);
	log::info!("Account: {} | Balance: {}", acct.to_ss58check(), balance);
	let vesting_stored = <Vesting<Runtime>>::get(acct.clone());
	if let Some(vesting) = vesting_stored {
		log::info!("Vesting: {:?}", vesting);
	} else {
		log::info!("Vesting: None");
	}
	let total_issuance = Balances::total_issuance();
	log::info!("Total Issuance: {}", total_issuance);
}
