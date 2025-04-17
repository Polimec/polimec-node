//! A module that is responsible for migration of storage.
use crate::{
	AccountIdOf, BiddingTicketSizes, Config, CurrencyMetadata, FixedPointNumber, ParticipantsAccountType, PriceOf,
	ProjectMetadataOf, StringLimitOf,
};
use core::marker::PhantomData;
use frame_support::traits::UncheckedOnRuntimeUpgrade;
use polimec_common::{assets::AcceptedFundingAsset, credentials::Cid};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::{ConstU32, Decode, Encode, Get, MaxEncodedLen, RuntimeDebug};
use sp_runtime::{BoundedVec, Percent};
extern crate alloc;
use alloc::vec::Vec;
use polimec_common::migration_types::{MigrationInfo, ParticipationType};
use xcm::v4::Location;

pub mod v5_storage_items {

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum CheckOutcome {
		AwaitingResponse,
		Passed(Option<PalletIndex>),
		Failed,
	}
	use super::*;
	use crate::{BlockNumberPair, FundingOutcome, Pallet, ProjectId, TicketSize};
	use frame_support::{pallet_prelude::NMapKey, storage_alias, Blake2_128Concat};
	use polimec_common::migration_types::MigrationStatus;
	use polkadot_parachain_primitives::primitives::Id as ParaId;
	use xcm::v4::QueryId;

	type Balance = u128;

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct HRMPChannelStatus {
		pub project_to_polimec: ChannelStatus,
		pub polimec_to_project: ChannelStatus,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum ChannelStatus {
		Closed,
		Open,
		AwaitingAcceptance,
	}
	pub type PalletIndex = u8;

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct PalletMigrationReadinessCheck {
		pub holding_check: (QueryId, CheckOutcome),
		pub pallet_check: (QueryId, CheckOutcome),
	}
	impl PalletMigrationReadinessCheck {
		pub fn is_ready(&self) -> bool {
			self.holding_check.1 == CheckOutcome::Passed(None) &&
				matches!(self.pallet_check.1, CheckOutcome::Passed(Some(_)))
		}
	}
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct PalletMigrationInfo {
		/// ParaId of project
		pub parachain_id: ParaId,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
		/// Migration readiness check
		pub migration_readiness_check: Option<PalletMigrationReadinessCheck>,
	}
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum MigrationType {
		Offchain,
		Pallet(PalletMigrationInfo),
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum OldProjectStatus<BlockNumber> {
		Application,
		EvaluationRound,
		AuctionRound,
		CommunityRound(BlockNumber),
		FundingFailed,
		FundingSuccessful,
		SettlementStarted(FundingOutcome),
		SettlementFinished(FundingOutcome),
		CTMigrationStarted,
		CTMigrationFinished,
	}
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldProjectDetails<AccountId, Did, BlockNumber, Price: FixedPointNumber, EvaluationRoundInfo> {
		pub issuer_account: AccountId,
		pub issuer_did: Did,
		/// Whether the project is frozen, so no `metadata` changes are allowed.
		pub is_frozen: bool,
		/// The price in USD per token decided after the Auction Round
		pub weighted_average_price: Option<Price>,
		/// The current status of the project
		pub status: OldProjectStatus<BlockNumber>,
		/// When the different project phases start and end
		pub round_duration: BlockNumberPair<BlockNumber>,
		/// Fundraising target amount in USD (6 decimals)
		pub fundraising_target_usd: Balance,
		/// The amount of Contribution Tokens that have not yet been sold
		pub remaining_contribution_tokens: Balance,
		/// Funding reached amount in USD (6 decimals)
		pub funding_amount_reached_usd: Balance,
		/// Information about the total amount bonded, and the outcome in regards to reward/slash/nothing
		pub evaluation_round_info: EvaluationRoundInfo,
		/// If the auction was oversubscribed, how much USD was raised across all winning bids
		pub usd_bid_on_oversubscription: Option<Balance>,
		/// When the Funding Round ends
		pub funding_end_block: Option<BlockNumber>,
		pub migration_type: Option<MigrationType>,
	}

	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct OldBiddingTicketSizes<Price: FixedPointNumber, Balance> {
		pub professional: TicketSize<Balance>,
		pub institutional: TicketSize<Balance>,
		pub phantom: PhantomData<(Price, Balance)>,
	}
	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct OldContributingTicketSizes<Price: FixedPointNumber, Balance> {
		pub retail: TicketSize<Balance>,
		pub professional: TicketSize<Balance>,
		pub institutional: TicketSize<Balance>,
		pub phantom: PhantomData<(Price, Balance)>,
	}

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
		pub bidding_ticket_sizes: OldBiddingTicketSizes<Price, Balance>,
		pub contributing_ticket_sizes: OldContributingTicketSizes<Price>,
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
		pub participation_type: OldParticipationType,
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

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum OldParticipationType {
		Evaluation,
		Bid,
		Contribution,
	}
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldMigration {
		pub origin: OldMigrationOrigin,
		pub info: MigrationInfo,
	}
	pub const MAX_PARTICIPATIONS_PER_USER: u32 = 16 + 16 + 16;

	#[storage_alias]
	pub type UserMigrations<T: Config> = StorageNMap<
		Pallet<T>,
		(NMapKey<Blake2_128Concat, ProjectId>, NMapKey<Blake2_128Concat, AccountIdOf<T>>),
		(MigrationStatus, BoundedVec<OldMigration, ConstU32<MAX_PARTICIPATIONS_PER_USER>>),
	>;
}

pub mod v6 {
	use super::{
		v5_storage_items::{OldMigration, OldProjectStatus, MAX_PARTICIPATIONS_PER_USER},
		*,
	};
	use crate::{EvaluationRoundInfo, ProjectDetailsOf, ProjectStatus, TicketSize};
	use frame_system::pallet_prelude::BlockNumberFor;
	use polimec_common::{
		credentials::Did,
		migration_types::{Migration, MigrationOrigin, MigrationStatus},
		USD_UNIT,
	};
	use sp_runtime::WeakBoundedVec;

	type Balance = u128;

	type OldProjectMetadataOf<T> = super::v5_storage_items::OldProjectMetadata<
		BoundedVec<u8, StringLimitOf<T>>,
		Balance,
		PriceOf<T>,
		AccountIdOf<T>,
		Cid,
	>;

	type OldProjectDetailsOf<T> = super::v5_storage_items::OldProjectDetails<
		AccountIdOf<T>,
		Did,
		BlockNumberFor<T>,
		PriceOf<T>,
		EvaluationRoundInfo,
	>;

	pub struct UncheckedMigrationToV6<T: Config>(PhantomData<T>);
	impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV6<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut items = 0;
			log::info!("Starting migration to V5");
			let translate_project_metadata = |_key, item: OldProjectMetadataOf<T>| -> Option<ProjectMetadataOf<T>> {
				items += 1;

				let new_bidding_ticket_sizes = BiddingTicketSizes {
					professional: item.bidding_ticket_sizes.professional,
					institutional: item.bidding_ticket_sizes.institutional,
					retail: TicketSize { usd_minimum_per_participation: 10 * USD_UNIT, usd_maximum_per_did: None },
					phantom: Default::default(),
				};

				Some(ProjectMetadataOf::<T> {
					token_information: item.token_information,
					mainnet_token_max_supply: item.mainnet_token_max_supply,
					total_allocation_size: item.total_allocation_size,
					minimum_price: item.minimum_price,
					bidding_ticket_sizes: new_bidding_ticket_sizes,
					participation_currencies: item.participation_currencies,
					funding_destination_account: item.funding_destination_account,
					policy_ipfs_cid: item.policy_ipfs_cid,
					participants_account_type: ParticipantsAccountType::Polkadot,
				})
			};
			crate::ProjectsMetadata::<T>::translate(translate_project_metadata);

			let translate_project_details = |_key, item: OldProjectDetailsOf<T>| -> Option<ProjectDetailsOf<T>> {
				items += 1;
				Some(ProjectDetailsOf::<T> {
					issuer_account: item.issuer_account,
					issuer_did: item.issuer_did,
					is_frozen: item.is_frozen,
					status: match item.status {
						OldProjectStatus::Application => ProjectStatus::Application,
						OldProjectStatus::EvaluationRound => ProjectStatus::EvaluationRound,
						OldProjectStatus::AuctionRound => ProjectStatus::AuctionRound,
						OldProjectStatus::FundingFailed => ProjectStatus::FundingFailed,
						OldProjectStatus::FundingSuccessful => ProjectStatus::FundingSuccessful,
						OldProjectStatus::SettlementStarted(outcome) => ProjectStatus::SettlementStarted(outcome),
						OldProjectStatus::SettlementFinished(outcome) => ProjectStatus::SettlementFinished(outcome),
						OldProjectStatus::CTMigrationStarted => ProjectStatus::CTMigrationStarted,
						OldProjectStatus::CTMigrationFinished => ProjectStatus::CTMigrationFinished,
						_ => {
							log::warn!("Unsupported project status: {:?}", item.status);
							return None;
						},
					},
					round_duration: item.round_duration,
					fundraising_target_usd: item.fundraising_target_usd,
					remaining_contribution_tokens: item.remaining_contribution_tokens,
					funding_amount_reached_usd: item.funding_amount_reached_usd,
					evaluation_round_info: item.evaluation_round_info,
					usd_bid_on_oversubscription: item.usd_bid_on_oversubscription,
					funding_end_block: item.funding_end_block,
				})
			};
			crate::ProjectsDetails::<T>::translate(translate_project_details);

			let mut translate_migration =
				|(status, migrations): (
					MigrationStatus,
					BoundedVec<OldMigration, ConstU32<MAX_PARTICIPATIONS_PER_USER>>,
				)|
				 -> Option<(MigrationStatus, WeakBoundedVec<Migration, ConstU32<10_000>>)> {
					let old_migrations = migrations.to_vec();
					let mut new_migrations = Vec::new();

					for mut old_migration in old_migrations {
						items += 1;
						let origin_junction = old_migration.origin.user.interior.take_first().unwrap();
						let new_origin = MigrationOrigin {
							user: origin_junction,
							participation_type: match old_migration.origin.participation_type {
								v5_storage_items::OldParticipationType::Evaluation => ParticipationType::Evaluation,
								v5_storage_items::OldParticipationType::Bid => ParticipationType::Bid,
								v5_storage_items::OldParticipationType::Contribution => ParticipationType::Bid,
							},
						};
						new_migrations.push(Migration { origin: new_origin, info: old_migration.info });
					}
					let new_migrations = new_migrations.try_into().ok()?;
					Some((status, new_migrations))
				};

			let old_migration_keys = v5_storage_items::UserMigrations::<T>::iter();

			for ((project_id, account), (status, migrations)) in old_migration_keys {
				log::info!("Read one old migration");
				v5_storage_items::UserMigrations::<T>::remove((project_id, account.clone()));
				log::info!("Removed one old migration");
				let maybe_new_migrations = translate_migration((status, migrations));
				if let Some(new_migrations) = maybe_new_migrations {
					crate::UserMigrations::<T>::insert((project_id, account.clone()), new_migrations);
					log::info!("Inserted a new migration");
				} else {
					log::error!(
						"Failed to migrate UserMigrations for project_id: {:?}, account: {:?}",
						project_id,
						account
					);
				}
			}

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
