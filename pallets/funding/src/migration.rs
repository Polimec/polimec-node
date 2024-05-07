//! A module that is responsible for migration of storage.
use frame_support::traits::StorageVersion;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);
pub const LOG: &str = "runtime::funding::migration";

pub mod v2 {
	use super::*;
	use crate::{AccountIdOf, BalanceOf, Config, ProjectMetadata, ProjectsMetadata};
	use frame_support::{
		pallet_prelude::{Decode, Encode, MaxEncodedLen, RuntimeDebug, TypeInfo},
		traits::{Get, OnRuntimeUpgrade},
		BoundedVec,
	};
	use polimec_common::{USD_DECIMALS, USD_UNIT};
	use sp_arithmetic::{traits::Zero, FixedPointNumber, Percent};
	use sp_core::ConstU32;
	use sp_std::marker::PhantomData;
	use xcm::v3::Weight;

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	pub struct OldTicketSize<Balance: PartialOrd + Copy> {
		pub usd_minimum_per_participation: Option<Balance>,
		pub usd_maximum_per_did: Option<Balance>,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	pub struct OldBiddingTicketSizes<Price: FixedPointNumber, Balance: PartialOrd + Copy> {
		pub professional: OldTicketSize<Balance>,
		pub institutional: OldTicketSize<Balance>,
		pub phantom: PhantomData<(Price, Balance)>,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	pub struct OldContributingTicketSizes<Price: FixedPointNumber, Balance: PartialOrd + Copy> {
		pub retail: OldTicketSize<Balance>,
		pub professional: OldTicketSize<Balance>,
		pub institutional: OldTicketSize<Balance>,
		pub phantom: PhantomData<(Price, Balance)>,
	}

	type OldProjectMetadataOf<T> = OldProjectMetadata<
		BoundedVec<u8, crate::StringLimitOf<T>>,
		BalanceOf<T>,
		crate::PriceOf<T>,
		AccountIdOf<T>,
		polimec_common::credentials::Cid,
	>;
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	pub struct OldProjectMetadata<BoundedString, Balance: PartialOrd + Copy, Price: FixedPointNumber, AccountId, Cid> {
		/// Token Metadata
		pub token_information: crate::CurrencyMetadata<BoundedString>,
		/// Mainnet Token Max Supply
		pub mainnet_token_max_supply: Balance,
		/// Total allocation of Contribution Tokens available for the Funding Round.
		pub total_allocation_size: Balance,
		/// Percentage of the total allocation of Contribution Tokens available for the Auction Round
		pub auction_round_allocation_percentage: Percent,
		/// The minimum price per token in USD, decimal-aware. See [`calculate_decimals_aware_price()`](crate::traits::ProvideAssetPrice::calculate_decimals_aware_price) for more information.
		pub minimum_price: Price,
		/// Maximum and minimum ticket sizes for auction round
		pub bidding_ticket_sizes: OldBiddingTicketSizes<Price, Balance>,
		/// Maximum and minimum ticket sizes for community/remainder rounds
		pub contributing_ticket_sizes: OldContributingTicketSizes<Price, Balance>,
		/// Participation currencies (e.g stablecoin, DOT, KSM)
		pub participation_currencies:
			BoundedVec<crate::AcceptedFundingAsset, ConstU32<{ crate::AcceptedFundingAsset::VARIANT_COUNT as u32 }>>,
		pub funding_destination_account: AccountId,
		/// Additional metadata
		pub policy_ipfs_cid: Option<Cid>,
	}

	pub struct UncheckedMigrationToV2<T: Config>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrationToV2<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut items = 0;
			let mut translate = |_key, item: OldProjectMetadataOf<T>| -> Option<crate::ProjectMetadataOf<T>> {
				items += 1;
				let usd_unit = sp_arithmetic::traits::checked_pow(BalanceOf::<T>::from(10u64), USD_DECIMALS as usize)?;
				Some(crate::ProjectMetadataOf::<T> {
					token_information: item.token_information,
					mainnet_token_max_supply: item.mainnet_token_max_supply,
					total_allocation_size: item.total_allocation_size,
					auction_round_allocation_percentage: item.auction_round_allocation_percentage,
					minimum_price: item.minimum_price,
					bidding_ticket_sizes: crate::BiddingTicketSizes {
						professional: crate::TicketSize {
							usd_minimum_per_participation: item
								.bidding_ticket_sizes
								.professional
								.usd_minimum_per_participation
								.unwrap_or_else(|| usd_unit),
							usd_maximum_per_did: item.bidding_ticket_sizes.professional.usd_maximum_per_did,
						},
						institutional: crate::TicketSize {
							usd_minimum_per_participation: item
								.bidding_ticket_sizes
								.institutional
								.usd_minimum_per_participation
								.unwrap_or_else(|| usd_unit),
							usd_maximum_per_did: item.bidding_ticket_sizes.institutional.usd_maximum_per_did,
						},
						phantom: Default::default(),
					},
					contributing_ticket_sizes: crate::ContributingTicketSizes {
						retail: crate::TicketSize {
							usd_minimum_per_participation: item
								.contributing_ticket_sizes
								.retail
								.usd_minimum_per_participation
								.unwrap_or_else(|| usd_unit),
							usd_maximum_per_did: item.contributing_ticket_sizes.retail.usd_maximum_per_did,
						},
						professional: crate::TicketSize {
							usd_minimum_per_participation: item
								.contributing_ticket_sizes
								.professional
								.usd_minimum_per_participation
								.unwrap_or_else(|| usd_unit),
							usd_maximum_per_did: item.contributing_ticket_sizes.professional.usd_maximum_per_did,
						},
						institutional: crate::TicketSize {
							usd_minimum_per_participation: item
								.contributing_ticket_sizes
								.institutional
								.usd_minimum_per_participation
								.unwrap_or_else(|| usd_unit),
							usd_maximum_per_did: item.contributing_ticket_sizes.institutional.usd_maximum_per_did,
						},
						phantom: Default::default(),
					},
					participation_currencies: item.participation_currencies,
					funding_destination_account: item.funding_destination_account,
					policy_ipfs_cid: item.policy_ipfs_cid,
				})
			};

			ProjectsMetadata::<T>::translate(|key, object: OldProjectMetadataOf<T>| translate(key, object));

			T::DbWeight::get().reads_writes(items, items)
		}
	}

	pub type MigrationToV2<T> = frame_support::migrations::VersionedMigration<
		1,
		2,
		UncheckedMigrationToV2<T>,
		crate::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}
