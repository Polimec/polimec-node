//! A module that is responsible for migration of storage.
use frame_support::traits::StorageVersion;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);
pub const LOG: &str = "runtime::funding::migration";
//
// pub mod v2 {
// 	use crate::{AccountIdOf, BalanceOf, Config, ProjectsMetadata};
// 	use frame_support::{
// 		pallet_prelude::{Decode, Encode, MaxEncodedLen, RuntimeDebug, TypeInfo},
// 		traits::{Get, OnRuntimeUpgrade},
// 		BoundedVec,
// 	};
// 	use polimec_common::USD_DECIMALS;
// 	use sp_arithmetic::{FixedPointNumber, Percent};
// 	use sp_core::ConstU32;
// 	use sp_std::marker::PhantomData;
//
// 	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// 	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// 	pub struct OldTicketSize<Balance: PartialOrd + Copy> {
// 		pub usd_minimum_per_participation: Option<Balance>,
// 		pub usd_maximum_per_did: Option<Balance>,
// 	}
//
// 	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// 	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// 	pub struct OldBiddingTicketSizes<Price: FixedPointNumber, Balance: PartialOrd + Copy> {
// 		pub professional: OldTicketSize<Balance>,
// 		pub institutional: OldTicketSize<Balance>,
// 		pub phantom: PhantomData<(Price, Balance)>,
// 	}
//
// 	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// 	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// 	pub struct OldContributingTicketSizes<Price: FixedPointNumber, Balance: PartialOrd + Copy> {
// 		pub retail: OldTicketSize<Balance>,
// 		pub professional: OldTicketSize<Balance>,
// 		pub institutional: OldTicketSize<Balance>,
// 		pub phantom: PhantomData<(Price, Balance)>,
// 	}
//
// 	type OldProjectMetadataOf<T> = OldProjectMetadata<
// 		BoundedVec<u8, crate::StringLimitOf<T>>,
// 		BalanceOf<T>,
// 		crate::PriceOf<T>,
// 		AccountIdOf<T>,
// 		polimec_common::credentials::Cid,
// 	>;
// 	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// 	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// 	pub struct OldProjectMetadata<BoundedString, Balance: PartialOrd + Copy, Price: FixedPointNumber, AccountId, Cid> {
// 		/// Token Metadata
// 		pub token_information: crate::CurrencyMetadata<BoundedString>,
// 		/// Mainnet Token Max Supply
// 		pub mainnet_token_max_supply: Balance,
// 		/// Total allocation of Contribution Tokens available for the Funding Round.
// 		pub total_allocation_size: Balance,
// 		/// Percentage of the total allocation of Contribution Tokens available for the Auction Round
// 		pub auction_round_allocation_percentage: Percent,
// 		/// The minimum price per token in USD, decimal-aware. See [`calculate_decimals_aware_price()`](crate::traits::ProvideAssetPrice::calculate_decimals_aware_price) for more information.
// 		pub minimum_price: Price,
// 		/// Maximum and minimum ticket sizes for auction round
// 		pub bidding_ticket_sizes: OldBiddingTicketSizes<Price, Balance>,
// 		/// Maximum and minimum ticket sizes for community/remainder rounds
// 		pub contributing_ticket_sizes: OldContributingTicketSizes<Price, Balance>,
// 		/// Participation currencies (e.g stablecoin, DOT, KSM)
// 		pub participation_currencies:
// 			BoundedVec<crate::AcceptedFundingAsset, ConstU32<{ crate::AcceptedFundingAsset::VARIANT_COUNT as u32 }>>,
// 		pub funding_destination_account: AccountId,
// 		/// Additional metadata
// 		pub policy_ipfs_cid: Option<Cid>,
// 	}
//
// 	pub struct UncheckedMigrationToV2<T: Config>(PhantomData<T>);
// 	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrationToV2<T> {
// 		fn on_runtime_upgrade() -> frame_support::weights::Weight {
// 			let mut items = 0;
// 			let mut translate = |_key, item: OldProjectMetadataOf<T>| -> Option<crate::ProjectMetadataOf<T>> {
// 				items += 1;
// 				let usd_unit = sp_arithmetic::traits::checked_pow(BalanceOf::<T>::from(10u64), USD_DECIMALS as usize)?;
// 				Some(crate::ProjectMetadataOf::<T> {
// 					token_information: item.token_information,
// 					mainnet_token_max_supply: item.mainnet_token_max_supply,
// 					total_allocation_size: item.total_allocation_size,
// 					auction_round_allocation_percentage: item.auction_round_allocation_percentage,
// 					minimum_price: item.minimum_price,
// 					bidding_ticket_sizes: crate::BiddingTicketSizes {
// 						professional: crate::TicketSize {
// 							usd_minimum_per_participation: item
// 								.bidding_ticket_sizes
// 								.professional
// 								.usd_minimum_per_participation
// 								.unwrap_or_else(|| usd_unit),
// 							usd_maximum_per_did: item.bidding_ticket_sizes.professional.usd_maximum_per_did,
// 						},
// 						institutional: crate::TicketSize {
// 							usd_minimum_per_participation: item
// 								.bidding_ticket_sizes
// 								.institutional
// 								.usd_minimum_per_participation
// 								.unwrap_or_else(|| usd_unit),
// 							usd_maximum_per_did: item.bidding_ticket_sizes.institutional.usd_maximum_per_did,
// 						},
// 						phantom: Default::default(),
// 					},
// 					contributing_ticket_sizes: crate::ContributingTicketSizes {
// 						retail: crate::TicketSize {
// 							usd_minimum_per_participation: item
// 								.contributing_ticket_sizes
// 								.retail
// 								.usd_minimum_per_participation
// 								.unwrap_or_else(|| usd_unit),
// 							usd_maximum_per_did: item.contributing_ticket_sizes.retail.usd_maximum_per_did,
// 						},
// 						professional: crate::TicketSize {
// 							usd_minimum_per_participation: item
// 								.contributing_ticket_sizes
// 								.professional
// 								.usd_minimum_per_participation
// 								.unwrap_or_else(|| usd_unit),
// 							usd_maximum_per_did: item.contributing_ticket_sizes.professional.usd_maximum_per_did,
// 						},
// 						institutional: crate::TicketSize {
// 							usd_minimum_per_participation: item
// 								.contributing_ticket_sizes
// 								.institutional
// 								.usd_minimum_per_participation
// 								.unwrap_or_else(|| usd_unit),
// 							usd_maximum_per_did: item.contributing_ticket_sizes.institutional.usd_maximum_per_did,
// 						},
// 						phantom: Default::default(),
// 					},
// 					participation_currencies: item.participation_currencies,
// 					funding_destination_account: item.funding_destination_account,
// 					policy_ipfs_cid: item.policy_ipfs_cid,
// 				})
// 			};
//
// 			ProjectsMetadata::<T>::translate(|key, object: OldProjectMetadataOf<T>| translate(key, object));
//
// 			T::DbWeight::get().reads_writes(items, items)
// 		}
// 	}
//
// 	pub type MigrationToV2<T> = frame_support::migrations::VersionedMigration<
// 		1,
// 		2,
// 		UncheckedMigrationToV2<T>,
// 		crate::Pallet<T>,
// 		<T as frame_system::Config>::DbWeight,
// 	>;
// }
//
// pub mod v3 {
// 	use crate::{
// 		AccountIdOf, BalanceOf, Config, EvaluationRoundInfoOf, HRMPChannelStatus, MigrationReadinessCheck,
// 		PhaseTransitionPoints, PriceOf, ProjectDetailsOf, ProjectStatus,
// 	};
// 	use frame_support::{
// 		pallet_prelude::Get,
// 		traits::{tokens::Balance as BalanceT, OnRuntimeUpgrade},
// 	};
// 	use frame_system::pallet_prelude::BlockNumberFor;
// 	use polimec_common::credentials::Did;
// 	use polkadot_parachain_primitives::primitives::Id as ParaId;
// 	use scale_info::TypeInfo;
// 	use sp_arithmetic::FixedPointNumber;
// 	use sp_core::{Decode, Encode, MaxEncodedLen, RuntimeDebug};
// 	use sp_std::marker::PhantomData;
//
// 	#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
// 	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
// 	pub enum OldProjectStatus {
// 		#[default]
// 		Application,
// 		EvaluationRound,
// 		AuctionInitializePeriod,
// 		AuctionOpening,
// 		AuctionClosing,
// 		CommunityRound,
// 		RemainderRound,
// 		FundingFailed,
// 		AwaitingProjectDecision,
// 		FundingSuccessful,
// 		ReadyToStartMigration,
// 		MigrationCompleted,
// 	}
// 	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// 	pub struct OldProjectDetails<
// 		AccountId,
// 		Did,
// 		BlockNumber,
// 		Price: FixedPointNumber,
// 		Balance: BalanceT,
// 		EvaluationRoundInfo,
// 	> {
// 		pub issuer_account: AccountId,
// 		pub issuer_did: Did,
// 		/// Whether the project is frozen, so no `metadata` changes are allowed.
// 		pub is_frozen: bool,
// 		/// The price in USD per token decided after the Auction Round
// 		pub weighted_average_price: Option<Price>,
// 		/// The current status of the project
// 		pub status: OldProjectStatus,
// 		/// When the different project phases start and end
// 		pub phase_transition_points: PhaseTransitionPoints<BlockNumber>,
// 		/// Fundraising target amount in USD (6 decimals)
// 		pub fundraising_target_usd: Balance,
// 		/// The amount of Contribution Tokens that have not yet been sold
// 		pub remaining_contribution_tokens: Balance,
// 		/// Funding reached amount in USD (6 decimals)
// 		pub funding_amount_reached_usd: Balance,
// 		/// Information about the total amount bonded, and the outcome in regards to reward/slash/nothing
// 		pub evaluation_round_info: EvaluationRoundInfo,
// 		/// When the Funding Round ends
// 		pub funding_end_block: Option<BlockNumber>,
// 		/// ParaId of project
// 		pub parachain_id: Option<ParaId>,
// 		/// Migration readiness check
// 		pub migration_readiness_check: Option<MigrationReadinessCheck>,
// 		/// HRMP Channel status
// 		pub hrmp_channel_status: HRMPChannelStatus,
// 	}
// 	type OldProjectDetailsOf<T> =
// 		OldProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;
//
// 	pub struct UncheckedMigrationToV3<T: Config>(PhantomData<T>);
// 	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrationToV3<T> {
// 		fn on_runtime_upgrade() -> frame_support::weights::Weight {
// 			let mut items = 0;
// 			let mut translate = |_key, item: OldProjectDetailsOf<T>| -> Option<ProjectDetailsOf<T>> {
// 				items += 1;
// 				let new_status = match item.status {
// 					OldProjectStatus::Application => ProjectStatus::Application,
// 					OldProjectStatus::EvaluationRound => ProjectStatus::EvaluationRound,
// 					OldProjectStatus::AuctionInitializePeriod => ProjectStatus::AuctionInitializePeriod,
// 					OldProjectStatus::AuctionOpening => ProjectStatus::AuctionOpening,
// 					OldProjectStatus::AuctionClosing => ProjectStatus::AuctionClosing,
// 					OldProjectStatus::CommunityRound => ProjectStatus::CommunityRound,
// 					OldProjectStatus::RemainderRound => ProjectStatus::RemainderRound,
// 					OldProjectStatus::FundingFailed => ProjectStatus::FundingFailed,
// 					OldProjectStatus::AwaitingProjectDecision => ProjectStatus::AwaitingProjectDecision,
// 					OldProjectStatus::FundingSuccessful => ProjectStatus::FundingSuccessful,
// 					OldProjectStatus::ReadyToStartMigration => ProjectStatus::ReadyToStartMigration,
// 					OldProjectStatus::MigrationCompleted => ProjectStatus::MigrationCompleted,
// 				};
// 				Some(ProjectDetailsOf::<T> {
// 					issuer_account: item.issuer_account,
// 					issuer_did: item.issuer_did,
// 					is_frozen: item.is_frozen,
// 					weighted_average_price: item.weighted_average_price,
// 					status: new_status,
// 					phase_transition_points: item.phase_transition_points,
// 					fundraising_target_usd: item.fundraising_target_usd,
// 					remaining_contribution_tokens: item.remaining_contribution_tokens,
// 					funding_amount_reached_usd: item.funding_amount_reached_usd,
// 					evaluation_round_info: item.evaluation_round_info,
// 					usd_bid_on_oversubscription: None,
// 					funding_end_block: item.funding_end_block,
// 					parachain_id: item.parachain_id,
// 					migration_readiness_check: item.migration_readiness_check,
// 					hrmp_channel_status: item.hrmp_channel_status,
// 				})
// 			};
//
// 			crate::ProjectsDetails::<T>::translate(|key, object: OldProjectDetailsOf<T>| translate(key, object));
//
// 			T::DbWeight::get().reads_writes(items, items)
// 		}
// 	}
//
// 	pub type MigrationToV3<T> = frame_support::migrations::VersionedMigration<
// 		2,
// 		3,
// 		UncheckedMigrationToV3<T>,
// 		crate::Pallet<T>,
// 		<T as frame_system::Config>::DbWeight,
// 	>;
// }
