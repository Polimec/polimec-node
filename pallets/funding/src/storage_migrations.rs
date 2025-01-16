//! A module that is responsible for migration of storage.
use crate::{
	AccountIdOf, BiddingTicketSizes, Config, CurrencyMetadata, FixedPointNumber, ParticipantsAccountType, PriceOf,
	ProjectMetadataOf, StringLimitOf,
};
use core::marker::PhantomData;
use frame_support::traits::{StorageVersion, UncheckedOnRuntimeUpgrade};
use polimec_common::{assets::AcceptedFundingAsset, credentials::Cid};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, Decode, Encode, Get, MaxEncodedLen, RuntimeDebug};
use sp_runtime::{BoundedVec, Percent};
extern crate alloc;
use alloc::vec::Vec;
use polimec_common::migration_types::{MigrationInfo, ParticipationType};
use xcm::v4::Location;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(6);
pub const LOG: &str = "runtime::funding::migration";

pub mod v5_storage_items {

	use super::*;
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
	pub struct OldProjectMetadata<BoundedString, Balance: PartialOrd + Copy, Price: FixedPointNumber, AccountId, Cid> {
		/// Token Metadata
		pub token_information: CurrencyMetadata<BoundedString>,
		/// Mainnet Token Max Supply
		pub mainnet_token_max_supply: Balance,
		/// Total allocation of Contribution Tokens available for the Funding Round.
		pub total_allocation_size: Balance,
		/// Percentage of the total allocation of Contribution Tokens available for the Auction Round
		pub auction_round_allocation_percentage: Percent,
		/// The minimum price per token in USD, decimal-aware. See [`calculate_decimals_aware_price()`](crate::traits::ProvideAssetPrice::calculate_decimals_aware_price) for more information.
		pub minimum_price: Price,
		/// Maximum and minimum ticket sizes for auction round
		pub bidding_ticket_sizes: BiddingTicketSizes<Price>,
		/// Participation currencies (e.g stablecoin, DOT, KSM)
		/// e.g. https://github.com/paritytech/substrate/blob/427fd09bcb193c1e79dec85b1e207c718b686c35/frame/uniques/src/types.rs#L110
		/// For now is easier to handle the case where only just one Currency is accepted
		pub participation_currencies:
			BoundedVec<AcceptedFundingAsset, ConstU32<{ AcceptedFundingAsset::VARIANT_COUNT as u32 }>>,
		pub funding_destination_account: AccountId,
		/// Additional metadata
		pub policy_ipfs_cid: Option<Cid>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldMigrationOrigin {
		pub user: Location,
		pub id: u32,
		pub participation_type: ParticipationType,
	}
	impl PartialOrd for OldMigrationOrigin {
		fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
			Some(self.cmp(other))
		}
	}
	impl Ord for OldMigrationOrigin {
		fn cmp(&self, other: &Self) -> core::cmp::Ordering {
			if self.participation_type == other.participation_type {
				self.id.cmp(&other.id)
			} else {
				self.participation_type.cmp(&other.participation_type)
			}
		}
	}
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldMigration {
		pub origin: OldMigrationOrigin,
		pub info: MigrationInfo,
	}
}

pub mod v6 {
	use super::*;
	use crate::{storage_migrations::v5_storage_items::OldMigration, MaxParticipationsPerUser};
	use polimec_common::migration_types::{Migration, MigrationOrigin, MigrationStatus};

	type OldProjectMetadataOf<T> = super::v5_storage_items::OldProjectMetadata<
		BoundedVec<u8, StringLimitOf<T>>,
		<T as pallet_balances::Config>::Balance,
		PriceOf<T>,
		AccountIdOf<T>,
		Cid,
	>;

	pub struct UncheckedMigrationToV6<T: Config>(PhantomData<T>);
	impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV6<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut items = 0;
			log::info!("Starting migration to V5");
			let translate_project_details = |_key, item: OldProjectMetadataOf<T>| -> Option<ProjectMetadataOf<T>> {
				items += 1;

				Some(ProjectMetadataOf::<T> {
					token_information: item.token_information,
					mainnet_token_max_supply: item.mainnet_token_max_supply,
					total_allocation_size: item.total_allocation_size,
					minimum_price: item.minimum_price,
					bidding_ticket_sizes: item.bidding_ticket_sizes,
					participation_currencies: item.participation_currencies,
					funding_destination_account: item.funding_destination_account,
					policy_ipfs_cid: item.policy_ipfs_cid,
					participants_account_type: ParticipantsAccountType::Polkadot,
				})
			};
			crate::ProjectsMetadata::<T>::translate(translate_project_details);

			let translate_migration =
				|_keys,
				 (status, migrations): (MigrationStatus, BoundedVec<OldMigration, MaxParticipationsPerUser<T>>)|
				 -> Option<(MigrationStatus, BoundedVec<Migration, MaxParticipationsPerUser<T>>)> {
					let old_migrations = migrations.to_vec();
					let mut new_migrations = Vec::new();

					for mut old_migration in old_migrations {
						items += 1;
						let origin_junction = old_migration.origin.user.interior.take_first().unwrap();
						let new_origin = MigrationOrigin {
							user: origin_junction,
							id: old_migration.origin.id,
							participation_type: old_migration.origin.participation_type,
						};
						new_migrations.push(Migration { origin: new_origin, info: old_migration.info });
					}
					let new_migrations = new_migrations.try_into().ok()?;
					Some((status, new_migrations))
				};
			crate::UserMigrations::<T>::translate(translate_migration);

			log::info!("Migration to V5 completed. Migrated {} items", items);
			T::DbWeight::get().reads_writes(items, items)
		}
	}

	pub type MigrationToV6<T> = frame_support::migrations::VersionedMigration<
		5,
		6,
		UncheckedMigrationToV6<T>,
		crate::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}
