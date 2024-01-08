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
use frame_support::{dispatch::DispatchError, log, migration, storage::unhashed};

pub struct CustomOnRuntimeUpgrade;
impl frame_support::traits::OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		migrate()
	}
}

fn migrate() -> frame_support::weights::Weight {
	// Substrate
	use frame_support::traits::StorageVersion;

	// Some pallets are added on chain after the migration.
	// Thus, they never required the migration and we just missed to set the correct
	// `StorageVersion`.
	let old_version = StorageVersion::get::<Multisig>();
	log::info!("Multisig migrating from {:#?}", old_version);

	StorageVersion::new(1).put::<Multisig>();

	let new_version = StorageVersion::get::<Multisig>();
	log::info!("Multisig migrated to {:#?}", new_version);

	<Runtime as frame_system::Config>::DbWeight::get().reads_writes(0, 1)
}
