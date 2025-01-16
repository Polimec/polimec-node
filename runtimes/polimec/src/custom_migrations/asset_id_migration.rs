use crate::{AccountId, Funding, Runtime};
use alloc::collections::BTreeMap;
use frame_support::{
	pallet_prelude::{NMapKey, ValueQuery},
	storage_alias,
	traits::{GetStorageVersion, OnRuntimeUpgrade},
	Blake2_128Concat,
};
use itertools::Itertools;
use pallet_assets::{Approval, AssetAccount, AssetDetails, AssetMetadata};
use polimec_common::assets::AcceptedFundingAsset;
use sp_api::runtime_decl_for_core::CoreV5;
use sp_runtime::BoundedVec;
use xcm::v4::Location;

// Storage items of pallet-assets are set to private for some reason. So we have to redefine them to get the same storage
// encoding and call the `translate` methods. -_-'
pub mod pallet_assets_storage_items {
	use super::*;

	type Balance = u128;

	pub type AssetAccountOf = AssetAccount<Balance, Balance, (), AccountId>;

	pub type AssetDetailsOf = AssetDetails<Balance, AccountId, Balance>;

	pub type AssetMetadataOf = AssetMetadata<Balance, BoundedVec<u8, crate::AssetsStringLimit>>;

	pub mod old_types {
		use super::*;

		type OldAssetId = u32;

		#[storage_alias]
		pub type Account =
			StorageDoubleMap<ForeignAssets, Blake2_128Concat, OldAssetId, Blake2_128Concat, AccountId, AssetAccountOf>;

		#[storage_alias]
		pub type Asset = StorageMap<ForeignAssets, Blake2_128Concat, OldAssetId, AssetDetailsOf>;

		#[storage_alias]
		pub type Approvals = StorageNMap<
			ForeignAssets,
			(
				NMapKey<Blake2_128Concat, OldAssetId>,
				NMapKey<Blake2_128Concat, AccountId>,
				NMapKey<Blake2_128Concat, AccountId>,
			),
			Approval<Balance, Balance>,
		>;

		#[storage_alias]
		pub type Metadata = StorageMap<ForeignAssets, Blake2_128Concat, OldAssetId, AssetMetadataOf, ValueQuery>;
	}

	pub mod new_types {
		use super::*;

		type NewAssetId = Location;

		#[storage_alias]
		pub type Account =
			StorageDoubleMap<ForeignAssets, Blake2_128Concat, NewAssetId, Blake2_128Concat, AccountId, AssetAccountOf>;

		#[storage_alias]
		pub type Asset = StorageMap<ForeignAssets, Blake2_128Concat, NewAssetId, AssetDetailsOf>;

		#[storage_alias]
		pub type Approvals = StorageNMap<
			ForeignAssets,
			(
				NMapKey<Blake2_128Concat, NewAssetId>,
				NMapKey<Blake2_128Concat, AccountId>,
				NMapKey<Blake2_128Concat, AccountId>,
			),
			Approval<Balance, Balance>,
		>;

		#[storage_alias]
		pub type Metadata = StorageMap<ForeignAssets, Blake2_128Concat, NewAssetId, AssetMetadataOf, ValueQuery>;
	}
}

pub mod orml_oracle_storage_items {
	use super::*;

	pub mod old_types {
		use super::*;
		use frame_support::Twox64Concat;
		use orml_oracle::TimestampedValue;
		use shared_configuration::Price;

		type TimeStampedValueOf = TimestampedValue<Price, u64>;

		#[storage_alias]
		pub type RawValues = StorageDoubleMap<Oracle, Twox64Concat, AccountId, Twox64Concat, u32, TimeStampedValueOf>;

		#[storage_alias]
		pub type Values = StorageMap<Oracle, Twox64Concat, u32, TimeStampedValueOf>;
	}
}

// This migration should be run right before the pallet_funding migration from v5 -> v6.
pub struct FromOldAssetIdMigration;
impl OnRuntimeUpgrade for FromOldAssetIdMigration {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<sp_std::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		let funding_on_chain_version = Funding::on_chain_storage_version();
		if funding_on_chain_version == 5 {
			Ok(VersionedPostUpgradeData::MigrationExecuted(Vec::new()).encode())
		} else {
			Ok(VersionedPostUpgradeData::Noop.encode())
		}
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let version = Funding::on_chain_storage_version();
		log::info!("funding version: {:?}", version);
		if version != 5 {
			log::info!("funding version is not 5");
			return frame_support::weights::Weight::zero();
		}
		let runtime_version = Runtime::version();
		let mut items = 0;
		if runtime_version.spec_version == 1_000_000 {
			let id_map = BTreeMap::from([
				(1984, AcceptedFundingAsset::USDT.id()),
				(1337, AcceptedFundingAsset::USDC.id()),
				(10, AcceptedFundingAsset::DOT.id()),
				(3344, Location::here()),
			]);

			let old_account_iterator = pallet_assets_storage_items::old_types::Account::iter().collect_vec();
			for (old_asset_id, account, account_info) in old_account_iterator {
				items += 1;
				log::info!("old_account item {:?}", items);
				pallet_assets_storage_items::new_types::Account::insert(
					id_map.get(&old_asset_id).unwrap(),
					account.clone(),
					account_info,
				);
				pallet_assets_storage_items::old_types::Account::remove(old_asset_id, account);
			}

			let old_asset_iterator = pallet_assets_storage_items::old_types::Asset::iter().collect_vec();
			for (old_asset_id, asset_info) in old_asset_iterator {
				items += 1;
				log::info!("old_asset item {:?}", items);
				pallet_assets_storage_items::new_types::Asset::insert(id_map.get(&old_asset_id).unwrap(), asset_info);
				pallet_assets_storage_items::old_types::Asset::remove(old_asset_id);
			}

			let old_approvals_iterator = pallet_assets_storage_items::old_types::Approvals::iter().collect_vec();
			for ((old_asset_id, owner, delegate), approval) in old_approvals_iterator {
				items += 1;
				log::info!("old_approvals item {:?}", items);
				pallet_assets_storage_items::new_types::Approvals::insert(
					(id_map.get(&old_asset_id).unwrap(), owner.clone(), delegate.clone()),
					approval,
				);
				pallet_assets_storage_items::old_types::Approvals::remove((old_asset_id, owner, delegate));
			}

			let old_metadata_iterator = pallet_assets_storage_items::old_types::Metadata::iter().collect_vec();
			for (old_asset_id, metadata) in old_metadata_iterator {
				items += 1;
				log::info!("old_metadata item {:?}", items);
				pallet_assets_storage_items::new_types::Metadata::insert(id_map.get(&old_asset_id).unwrap(), metadata);
				pallet_assets_storage_items::old_types::Metadata::remove(old_asset_id);
			}

			let old_oracle_raw_values_iterator = orml_oracle_storage_items::old_types::RawValues::iter().collect_vec();
			for (account, old_asset_id, raw_values) in old_oracle_raw_values_iterator {
				items += 1;
				log::info!("old_oracle_raw_values item {:?}", items);
				orml_oracle::RawValues::<Runtime>::insert(
					account.clone(),
					id_map.get(&old_asset_id).unwrap(),
					raw_values,
				);
				orml_oracle_storage_items::old_types::RawValues::remove(account, old_asset_id);
			}

			let old_oracle_values_iterator = orml_oracle_storage_items::old_types::Values::iter().collect_vec();
			for (old_asset_id, value) in old_oracle_values_iterator {
				items += 1;
				log::info!("old_oracle_values item {:?}", items);
				orml_oracle::Values::<Runtime>::insert(id_map.get(&old_asset_id).unwrap(), value);
				orml_oracle_storage_items::old_types::Values::remove(old_asset_id);
			}
		}

		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(items, items)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(
		versioned_post_upgrade_data_bytes: sp_std::vec::Vec<u8>,
	) -> Result<(), sp_runtime::TryRuntimeError> {
		use parity_scale_codec::DecodeAll;
		match <VersionedPostUpgradeData>::decode_all(&mut &versioned_post_upgrade_data_bytes[..])
			.map_err(|_| "VersionedMigration post_upgrade failed to decode PreUpgradeData")?
		{
			VersionedPostUpgradeData::MigrationExecuted(_inner_bytes) => Ok(()),
			VersionedPostUpgradeData::Noop => Ok(()),
		}
	}
}
