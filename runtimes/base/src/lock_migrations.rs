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

use frame_support::traits::{LockIdentifier, LockableCurrency};
// Substrate
use frame_support::{
	log,
	traits::{
		fungible::Inspect,
		tokens::{fungible, Preservation::Expendable},
	},
};
use pallet_vesting::Vesting;
use sp_runtime::traits::Zero;

#[cfg(feature = "try-runtime")]
use sp_core::crypto::Ss58Codec;

pub struct UnlockBalancesMigration;
impl frame_support::traits::OnRuntimeUpgrade for UnlockBalancesMigration {
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
		// The escrow account is the account that will receive the balances of the accounts that will be unlocked
		let escrow_account =
			hex_literal::hex!["7369626c100d0000000000000000000000000000000000000000000000000000"].into();
		let lock_id: LockIdentifier = *b"vesting ";
		let accounts = [
			// Rebalance
			hex_literal::hex!["5cb51d3f348d4ac4cbf6129bef89c638ef7ef02c2a416558410ddfa73a689046"].into(),
			hex_literal::hex!["c252c7a636a0ee7d8ecc1819fdf22df61688e03b12bca5688cc21f6122bdb253"].into(),
			hex_literal::hex!["9640685126bed91534360206d7222fec938200d2e57e8bd99b91bbba08035134"].into(),
			hex_literal::hex!["10b0040770714da70bc2e49e889e3aa93c9a0359fbadddb1869851ae7d68c333"].into(),
			hex_literal::hex!["ce8e89212bcd75d0d01dfb43e7b34e36ff7a95623785c188aa62e8e81e33804b"].into(),
			hex_literal::hex!["b682f75950d5d4e228a6fbf43bfe3df3c0353453a735cad35d24828a899e3f5b"].into(),
			hex_literal::hex!["3204a8b98fb4f3f48b861cd8e32cc9275db3aa03a2dfd343ae7470531da7c153"].into(),
			hex_literal::hex!["64da598cf2b7421ef8502982bc8388a5650f6e4bbdefe00d5d3cc4de98df3c54"].into(),
			hex_literal::hex!["20083e96559ce97b61eb5b048383463aafeef390897c6976f072d41615bc3c6e"].into(),
			hex_literal::hex!["82a42f11991ff3919dc65bba067e9ea22e66049f2b5b83f186961c71ee88637a"].into(),
			hex_literal::hex!["3a85bfd84282bc93af85a38733bad239f7f21b9ac0cfcd7e01059256e2298b47"].into(),
			// Collator
			hex_literal::hex!["820050e114404eec82932c59bedbfb6c1b58981e8f85af37e5d4f26a34226960"].into(),
		];
		// For each account, remove the lock and transfer the full balance to the escrow account
		accounts.iter().for_each(|acct| {
			// +1 R
			let full_balance = Balances::balance(&acct);
			// We need to check the balance for the idempotency of the migration
			if full_balance.is_zero() {
				return;
			}
			// Remove the lock for the acct
			// +1 W
			<Balances as LockableCurrency<_>>::remove_lock(lock_id, &acct);
			// If any, remove the vesting info for the acct
			// +1 W
			<Vesting<Runtime>>::mutate(acct.clone(), |vesting| {
				*vesting = None;
			});
			// Transfer the full balance to the escrow_account, killing the sender account
			// +2 W
			let res = <Balances as fungible::Mutate<_>>::transfer(&acct, &escrow_account, full_balance, Expendable);
			if let Err(e) = res {
				log::error!("Error transferring balance: {:?}", e);
			}
		});

		let receivers = [
			// (transfer_amount, destination_account, old_balance)
			(
				22462517500000000u128,
				hex_literal::hex!["7a0cf91995d4d20c9ceb4ba56962d734a0563dded63679c34c7b90d58ed435e5"].into(), //5935J2eYNyvi3bJXfuXp5xNgxn5eaXdYmKLwGuLyS83ronDt
				2111562123706227u128,
			),
			(
				1370512500000000u128,
				hex_literal::hex!["7b9768064aaf666b0198c15460e51c648147d920018689f0f0af11584fc747b8"].into(), //5956TxSm2oCyef5JJGxh73JTC5PCpyyUpYfJAitxEAxVa1oT
				17933380000000000u128,
			),
			(
				1970040000000000u128,
				hex_literal::hex!["1609dea5afc046ff93d1d772f1bb1224dae2050d5a922ad2b203b4fed343dc71"].into(), //56mwaq2HD2ByLx2r6rwiBt79qUyE6RFjKaRBBvTJ6gr5VkGj
				27216030000000000u128,
			),
			(
				2250690000000000u128,
				hex_literal::hex!["9acbdacedb7224269ed10c77945d428ad72f616751ec7115d04153335baf06e6"].into(), //59n1XibBkTigQD4EjUcrmU9Kq4MMY35TjpH6SxwfwtiE5bJr
				21202020000000000u128,
			),
			(
				2146290000000000u128,
				hex_literal::hex!["6c957a506f82fc88fbead76998305b349f8d850ff51a00864cf1b89dff75c4aa"].into(), //58jRB4wiuJLakuNtLAbJ91EfSvKcTzsroHj2oHoyPdFZowEk
				24733030000000000u128,
			),
		];
		// For each receiver, transfer the new amount to the destination account
		receivers.iter().for_each(|(amount, acct, old_amount)| {
			// +1 R
			let balance = Balances::balance(&acct);
			// We need to check the balance for the idempotency of the migration
			if balance == *old_amount {
				// +2 W
				let res = <Balances as fungible::Mutate<_>>::transfer(&escrow_account, &acct, *amount, Expendable);
				if let Err(e) = res {
					log::error!("Error transferring balance: {:?}", e);
				}
			}
		});

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(18, 60)
	}
}

#[cfg(feature = "try-runtime")]
fn check_balances() {
	let mut total_amount: Balance = Zero::zero();
	[
		// Rebalance
		hex_literal::hex!["5cb51d3f348d4ac4cbf6129bef89c638ef7ef02c2a416558410ddfa73a689046"].into(),
		hex_literal::hex!["c252c7a636a0ee7d8ecc1819fdf22df61688e03b12bca5688cc21f6122bdb253"].into(),
		hex_literal::hex!["9640685126bed91534360206d7222fec938200d2e57e8bd99b91bbba08035134"].into(),
		hex_literal::hex!["10b0040770714da70bc2e49e889e3aa93c9a0359fbadddb1869851ae7d68c333"].into(),
		hex_literal::hex!["ce8e89212bcd75d0d01dfb43e7b34e36ff7a95623785c188aa62e8e81e33804b"].into(),
		hex_literal::hex!["b682f75950d5d4e228a6fbf43bfe3df3c0353453a735cad35d24828a899e3f5b"].into(),
		hex_literal::hex!["3204a8b98fb4f3f48b861cd8e32cc9275db3aa03a2dfd343ae7470531da7c153"].into(),
		hex_literal::hex!["64da598cf2b7421ef8502982bc8388a5650f6e4bbdefe00d5d3cc4de98df3c54"].into(),
		hex_literal::hex!["20083e96559ce97b61eb5b048383463aafeef390897c6976f072d41615bc3c6e"].into(),
		hex_literal::hex!["82a42f11991ff3919dc65bba067e9ea22e66049f2b5b83f186961c71ee88637a"].into(),
		hex_literal::hex!["3a85bfd84282bc93af85a38733bad239f7f21b9ac0cfcd7e01059256e2298b47"].into(),
		// Collator
		hex_literal::hex!["820050e114404eec82932c59bedbfb6c1b58981e8f85af37e5d4f26a34226960"].into(),
		// Para ID
		hex_literal::hex!["7369626c100d0000000000000000000000000000000000000000000000000000"].into(),
	]
	.iter()
	.for_each(|acct| {
		let balance = Balances::balance(&acct);
		total_amount += balance;
		log::info!("Account: {} | Balance: {}", acct.to_ss58check(), balance);
		let vesting_stored = <Vesting<Runtime>>::get(acct.clone());
		if let Some(vesting) = vesting_stored {
			log::info!("Vesting: {:?}", vesting);
		} else {
			log::info!("Vesting: None");
		}
	});
	let total_issuance = Balances::total_issuance();
	log::info!("Total Issuance:                               {}", total_issuance);
	// The new vesting balance will be done subsequently using a force_vested_transfer from the escrow_account to the "Rebalance" accounts
	log::info!("Total Amount for re-distribution via vesting:   {}", total_amount);
}
