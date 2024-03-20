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
use frame_support::traits::tokens::Precision::Exact;
#[cfg(feature = "try-runtime")]
use log;

pub struct DepositDust;
impl frame_support::traits::OnRuntimeUpgrade for DepositDust {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
		log::info!("Pre-upgrade");
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
		log::info!("Post-upgrade");
		let total_issuance = Balances::total_issuance();
		assert_eq!(total_issuance, 100_000_000 * PLMC);
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// +1 R
		let total_issuance = Balances::total_issuance();

		// Idempotent check.
		if total_issuance != 100_000_000 * PLMC {
			log::info!("⚠️ Correcting total issuance from {} to {}", total_issuance, 100_000_000 * PLMC);
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

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(2, 1)
	}
}
