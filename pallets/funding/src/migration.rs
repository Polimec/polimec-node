//! A module that is responsible for migration of storage.

use crate::{
	types::{HRMPChannelStatus, MigrationReadinessCheck, PhaseTransitionPoints, ProjectStatus},
	AccountIdOf, BalanceOf, BlockNumberFor, Config, Did, EvaluationRoundInfoOf, Pallet, PriceOf, ProjectId,
};
use frame_support::{
	pallet_prelude::*,
	traits::{tokens::Balance as BalanceT, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use parity_scale_codec::{Decode, Encode};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_arithmetic::FixedPointNumber;
use sp_std::marker::PhantomData;

/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);
pub const LOG: &str = "runtime::funding::migration";

mod v0 {
	use super::*;
	pub use cleaner::*;
	mod cleaner {
		use super::*;
		#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub struct Success;
		#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub struct Failure;
		#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub enum CleanerState<T> {
			Initialized(PhantomData<T>),
			// Success or Failure
			EvaluationRewardOrSlash(u64, PhantomData<T>),
			EvaluationUnbonding(u64, PhantomData<T>),
			// Branch
			// A. Success only
			BidCTMint(u64, PhantomData<T>),
			ContributionCTMint(u64, PhantomData<T>),
			StartBidderVestingSchedule(u64, PhantomData<T>),
			StartContributorVestingSchedule(u64, PhantomData<T>),
			BidFundingPayout(u64, PhantomData<T>),
			ContributionFundingPayout(u64, PhantomData<T>),
			// B. Failure only
			BidFundingRelease(u64, PhantomData<T>),
			BidUnbonding(u64, PhantomData<T>),
			ContributionFundingRelease(u64, PhantomData<T>),
			ContributionUnbonding(u64, PhantomData<T>),
			// Merge
			// Success or Failure
			Finished(PhantomData<T>),
		}
		#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
		pub enum Cleaner {
			NotReady,
			Success(CleanerState<Success>),
			Failure(CleanerState<Failure>),
		}
		impl TryFrom<ProjectStatus> for Cleaner {
			type Error = ();

			fn try_from(value: ProjectStatus) -> Result<Self, ()> {
				match value {
					ProjectStatus::FundingSuccessful => Ok(Cleaner::Success(CleanerState::Initialized(PhantomData))),
					ProjectStatus::FundingFailed | ProjectStatus::EvaluationFailed =>
						Ok(Cleaner::Failure(CleanerState::Initialized(PhantomData))),
					_ => Err(()),
				}
			}
		}
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct ProjectDetails<
		AccountId,
		Did,
		BlockNumber,
		Price: FixedPointNumber,
		Balance: BalanceT,
		EvaluationRoundInfo,
	> {
		pub issuer_account: AccountId,
		pub issuer_did: Did,
		/// Whether the project is frozen, so no `metadata` changes are allowed.
		pub is_frozen: bool,
		/// The price in USD per token decided after the Auction Round
		pub weighted_average_price: Option<Price>,
		/// The current status of the project
		pub status: ProjectStatus,
		/// When the different project phases start and end
		pub phase_transition_points: PhaseTransitionPoints<BlockNumber>,
		/// Fundraising target amount in USD equivalent
		pub fundraising_target: Balance,
		/// The amount of Contribution Tokens that have not yet been sold
		pub remaining_contribution_tokens: Balance,
		/// Funding reached amount in USD equivalent
		pub funding_amount_reached: Balance,
		/// Cleanup operations remaining
		pub cleanup: Cleaner,
		/// Information about the total amount bonded, and the outcome in regards to reward/slash/nothing
		pub evaluation_round_info: EvaluationRoundInfo,
		/// When the Funding Round ends
		pub funding_end_block: Option<BlockNumber>,
		/// ParaId of project
		pub parachain_id: Option<ParaId>,
		/// Migration readiness check
		pub migration_readiness_check: Option<MigrationReadinessCheck>,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
	}
	pub type ProjectDetailsOf<T> =
		ProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;

	#[frame_support::storage_alias]
	pub(crate) type ProjectsDetails<T: Config> =
		StorageMap<Pallet<T>, Blake2_128Concat, ProjectId, ProjectDetailsOf<T>>;
}

pub mod v1 {
	use super::*;
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct ProjectDetails<
		AccountId,
		Did,
		BlockNumber,
		Price: FixedPointNumber,
		Balance: BalanceT,
		EvaluationRoundInfo,
	> {
		pub issuer_account: AccountId,
		pub issuer_did: Did,
		/// Whether the project is frozen, so no `metadata` changes are allowed.
		pub is_frozen: bool,
		/// The price in USD per token decided after the Auction Round
		pub weighted_average_price: Option<Price>,
		/// The current status of the project
		pub status: ProjectStatus,
		/// When the different project phases start and end
		pub phase_transition_points: PhaseTransitionPoints<BlockNumber>,
		/// Fundraising target amount in USD equivalent
		pub fundraising_target: Balance,
		/// The amount of Contribution Tokens that have not yet been sold
		pub remaining_contribution_tokens: Balance,
		/// Funding reached amount in USD equivalent
		pub funding_amount_reached: Balance,
		/// Information about the total amount bonded, and the outcome in regards to reward/slash/nothing
		pub evaluation_round_info: EvaluationRoundInfo,
		/// When the Funding Round ends
		pub funding_end_block: Option<BlockNumber>,
		/// ParaId of project
		pub parachain_id: Option<ParaId>,
		/// Migration readiness check
		pub migration_readiness_check: Option<MigrationReadinessCheck>,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
	}
	pub type ProjectDetailsOf<T> =
		ProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;

	#[frame_support::storage_alias]
	pub(crate) type ProjectsDetails<T: Config> =
		StorageMap<Pallet<T>, Blake2_128Concat, ProjectId, ProjectDetailsOf<T>>;

	/// Migrates `ProjectDetails` from v0 to v1.
	pub struct UncheckedMigrationToV1<T>(sp_std::marker::PhantomData<T>);

	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrationToV1<T> {
		#[allow(deprecated)]
		fn on_runtime_upgrade() -> Weight {
			// cleaner field does not exist anymore so we ignore it
			let mut storage_translations = 0u64;
			let mut translate = |pre: v0::ProjectDetailsOf<T>| -> Option<v1::ProjectDetailsOf<T>> {
				storage_translations += 1;
				Some(v1::ProjectDetailsOf::<T> {
					issuer_account: pre.issuer_account,
					issuer_did: pre.issuer_did,
					is_frozen: pre.is_frozen,
					weighted_average_price: pre.weighted_average_price,
					status: pre.status,
					phase_transition_points: pre.phase_transition_points,
					fundraising_target: pre.fundraising_target,
					remaining_contribution_tokens: pre.remaining_contribution_tokens,
					funding_amount_reached: pre.funding_amount_reached,
					evaluation_round_info: pre.evaluation_round_info,
					funding_end_block: pre.funding_end_block,
					parachain_id: pre.parachain_id,
					migration_readiness_check: pre.migration_readiness_check,
					hrmp_channel_status: pre.hrmp_channel_status,
				})
			};

			v1::ProjectsDetails::<T>::translate(|_key, pre: v0::ProjectDetailsOf<T>| translate(pre));

			T::DbWeight::get().reads_writes(storage_translations, storage_translations)
		}
	}

	/// [`UncheckedMigrationToV1`] wrapped in a
	/// [`VersionedMigration`](frame_support::migrations::VersionedMigration), ensuring the
	/// migration is only performed when on-chain version is 0.
	#[allow(dead_code)]
	pub type MigrationToV1<T> = frame_support::migrations::VersionedMigration<
		0,
		1,
		UncheckedMigrationToV1<T>,
		Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}
