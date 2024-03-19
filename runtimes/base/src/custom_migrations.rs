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
use frame_support::{
	log,
	storage::unhashed,
	traits::{tokens::Precision::Exact, GetStorageVersion, PalletInfoAccess, StorageVersion},
};
use sp_runtime::AccountId32;

#[cfg(feature = "try-runtime")]
use sp_core::crypto::Ss58Codec;

#[cfg(feature = "try-runtime")]
use pallet_vesting::Vesting;

#[cfg(feature = "try-runtime")]
use frame_support::dispatch::DispatchError;

pub struct InitializePallet<Pallet: GetStorageVersion<CurrentStorageVersion = StorageVersion> + PalletInfoAccess>(
	sp_std::marker::PhantomData<Pallet>,
);
impl<Pallet: GetStorageVersion<CurrentStorageVersion = StorageVersion> + PalletInfoAccess>
	frame_support::traits::OnRuntimeUpgrade for InitializePallet<Pallet>
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		log::info!("{} migrating from {:#?}", Pallet::name(), Pallet::on_chain_storage_version());
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		log::info!("{} migrated to {:#?}", Pallet::name(), Pallet::on_chain_storage_version());
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		if Pallet::on_chain_storage_version() == StorageVersion::new(0) {
			Pallet::current_storage_version().put::<Pallet>();
		}
		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 1)
	}
}

// The `VestingInfo` fields from `pallet_vesting` are private, so we need to define them here.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode, sp_runtime::RuntimeDebug, Eq, PartialEq)]
struct VestingInfo {
	locked: Balance,
	per_block: Balance,
	starting_block: BlockNumber,
}

// The vesting.Vesting(5Ag8zhuoZjKzc3YzmkWFrrmU5GvxdHLtpAN425RW9ZgWS5V7) encoded storage key.
const ENCODED_STORAGE_KEY: &str =
	"0x5f27b51b5ec208ee9cb25b55d87282435f27b51b5ec208ee9cb25b55d872824334f5503ce555ea3ee18396f4bde1b40bc28dbf096b5acf3c0d87dd8ef8cabea0794cc72200a2368751a0fe470d5f9f69";

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
		let total_issuance = Balances::total_issuance();
		assert_eq!(total_issuance, 100_000_000 * PLMC);
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// This account received a wrong vesting schedule.
		let acct: AccountId32 =
			hex_literal::hex!["c28dbf096b5acf3c0d87dd8ef8cabea0794cc72200a2368751a0fe470d5f9f69"].into();

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

		// +1 R
		let total_issuance = Balances::total_issuance();

		// Idempotent check.
		if total_issuance != 100_000_000 * PLMC {
			log::info!("⚠️ Correcting total issuance from {} to 100_000_000", total_issuance);
			// +1 R
			let treasury_account = PayMaster::get();
			// +1 W
			// The values are coming from these `DustLost` events:
			// - https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.polimec.org#/explorer/query/0x6fec4ce782f42afae1437f53e3382d9e6804692de868a28908ed6b9104bdd536
			// - https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Frpc.polimec.org#/explorer/query/0x390d04247334df9d9eb02e1dc7c6d01910c950d99a5d8d17441eb202cd751f42
			let _ = <Balances as tokens::fungible::Balanced<AccountId>>::deposit(
				&treasury_account,
				39988334 + 70094167,
				Exact,
			);
		}

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(3, 2)
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
