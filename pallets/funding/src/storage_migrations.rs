//! A module that is responsible for migration of storage.
use super::*;
use frame_support::{
	pallet_prelude::*,
	traits::{tokens::Balance as BalanceT, StorageVersion},
};
use serde::{Deserialize, Serialize};
/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);
pub const LOG: &str = "runtime::funding::migration";
use frame_support::traits::OnRuntimeUpgrade;

pub mod v3tov4 {
	use super::*;
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldProjectDetails<
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
		pub status: OldProjectStatus,
		/// When the different project phases start and end
		pub phase_transition_points: PhaseTransitionPoints<BlockNumber>,
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
		/// ParaId of project
		pub parachain_id: Option<ParaId>,
		/// Migration readiness check
		pub migration_readiness_check: Option<MigrationReadinessCheck>,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct MigrationReadinessCheck {
		pub holding_check: (xcm::v3::QueryId, CheckOutcome),
		pub pallet_check: (xcm::v3::QueryId, CheckOutcome),
	}

	impl MigrationReadinessCheck {
		pub fn is_ready(&self) -> bool {
			self.holding_check.1 == CheckOutcome::Passed(None) &&
				matches!(self.pallet_check.1, CheckOutcome::Passed(Some(_)))
		}
	}

	pub type PalletIndex = u8;
	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum CheckOutcome {
		AwaitingResponse,
		Passed(Option<PalletIndex>),
		Failed,
	}

	#[derive(
		Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize,
	)]
	pub enum OldProjectStatus {
		#[default]
		Application,
		EvaluationRound,
		AuctionInitializePeriod,
		AuctionOpening,
		AuctionClosing,
		CalculatingWAP,
		CommunityRound,
		RemainderRound,
		FundingFailed,
		AwaitingProjectDecision,
		FundingSuccessful,
		ReadyToStartMigration,
		MigrationCompleted,
	}

	type OldProjectDetailsOf<T> =
		OldProjectDetails<AccountIdOf<T>, Did, BlockNumberFor<T>, PriceOf<T>, BalanceOf<T>, EvaluationRoundInfoOf<T>>;

	pub struct UncheckedMigrationToV4<T: Config>(PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for UncheckedMigrationToV4<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut items = 0;
			let mut translate = |_key, item: OldProjectDetailsOf<T>| -> Option<ProjectDetailsOf<T>> {
				items += 1;
				let new_status = match item.status {
					OldProjectStatus::Application => ProjectStatus::Application,
					OldProjectStatus::EvaluationRound => ProjectStatus::EvaluationRound,
					OldProjectStatus::AuctionInitializePeriod => ProjectStatus::AuctionInitializePeriod,
					OldProjectStatus::AuctionOpening => ProjectStatus::AuctionOpening,
					OldProjectStatus::AuctionClosing => ProjectStatus::AuctionClosing,
					OldProjectStatus::CalculatingWAP => ProjectStatus::CalculatingWAP,
					OldProjectStatus::CommunityRound => ProjectStatus::CommunityRound,
					OldProjectStatus::RemainderRound => ProjectStatus::RemainderRound,
					OldProjectStatus::FundingFailed => ProjectStatus::FundingFailed,
					OldProjectStatus::AwaitingProjectDecision => ProjectStatus::AwaitingProjectDecision,
					OldProjectStatus::FundingSuccessful => {
						debug_assert!(item.funding_end_block.is_none(), "Settlement shouldn't have started yet");
						ProjectStatus::FundingSuccessful
					},

					OldProjectStatus::ReadyToStartMigration => {
						debug_assert!(false, "No project should be in this state when upgrading to v4");
						ProjectStatus::CTMigrationStarted
					},
					OldProjectStatus::MigrationCompleted => {
						debug_assert!(false, "No project should be in this state when upgrading to v4");
						ProjectStatus::CTMigrationFinished
					},
				};
				Some(ProjectDetailsOf::<T> {
					issuer_account: item.issuer_account,
					issuer_did: item.issuer_did,
					is_frozen: item.is_frozen,
					weighted_average_price: item.weighted_average_price,
					status: new_status,
					phase_transition_points: item.phase_transition_points,
					fundraising_target_usd: item.fundraising_target_usd,
					remaining_contribution_tokens: item.remaining_contribution_tokens,
					funding_amount_reached_usd: item.funding_amount_reached_usd,
					evaluation_round_info: item.evaluation_round_info,
					usd_bid_on_oversubscription: item.usd_bid_on_oversubscription,
					funding_end_block: item.funding_end_block,
					migration_type: None,
				})
			};

			crate::ProjectsDetails::<T>::translate(|key, object: OldProjectDetailsOf<T>| translate(key, object));

			T::DbWeight::get().reads_writes(items, items)
		}
	}

	pub type MigrationToV4<T> = frame_support::migrations::VersionedMigration<
		3,
		4,
		UncheckedMigrationToV4<T>,
		crate::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}
