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
#[allow(unused_imports)]
use frame_support::{
	migration,
	storage::unhashed,
	traits::{GetStorageVersion, PalletInfoAccess, StorageVersion},
};
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
#[cfg(feature = "try-runtime")]
use log;

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
