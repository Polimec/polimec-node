//! A module that is responsible for migration of storage.
use frame_support::traits::StorageVersion;
/// The current storage version
pub const STORAGE_VERSION: StorageVersion = StorageVersion::new(5);
pub const LOG: &str = "runtime::funding::migration";

pub mod v5 {
	use crate::{
		AccountIdOf, BalanceOf, BlockNumberPair, CheckOutcome, Config, EvaluationRoundInfoOf, EvaluatorsOutcome,
		FixedPointNumber, FundingOutcome, HRMPChannelStatus, Pallet, PriceOf, ProjectDetailsOf, ProjectStatus,
		RewardInfo,
	};
	use core::marker::PhantomData;
	use frame_support::traits::{tokens::Balance as BalanceT, UncheckedOnRuntimeUpgrade};
	use frame_system::pallet_prelude::BlockNumberFor;
	use polimec_common::credentials::Did;
	use polkadot_parachain_primitives::primitives::Id as ParaId;
	use scale_info::TypeInfo;
	use serde::{Deserialize, Serialize};
	use sp_core::{Decode, Encode, Get, MaxEncodedLen, RuntimeDebug};
	use sp_runtime::traits::One;

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
		pub phase_transition_points: OldPhaseTransitionPoints<BlockNumber>,
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
		pub migration_type: Option<OldMigrationType>,
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
		SettlementStarted(OldFundingOutcome),
		SettlementFinished(OldFundingOutcome),
		CTMigrationStarted,
		CTMigrationFinished,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
	pub enum OldFundingOutcome {
		FundingSuccessful,
		FundingFailed,
	}

	#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldPhaseTransitionPoints<BlockNumber> {
		pub application: OldBlockNumberPair<BlockNumber>,
		pub evaluation: OldBlockNumberPair<BlockNumber>,
		pub auction_initialize_period: OldBlockNumberPair<BlockNumber>,
		pub auction_opening: OldBlockNumberPair<BlockNumber>,
		pub random_closing_ending: Option<BlockNumber>,
		pub auction_closing: OldBlockNumberPair<BlockNumber>,
		pub community: OldBlockNumberPair<BlockNumber>,
		pub remainder: OldBlockNumberPair<BlockNumber>,
	}

	#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldBlockNumberPair<BlockNumber> {
		pub start: Option<BlockNumber>,
		pub end: Option<BlockNumber>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldEvaluationRoundInfo<Balance> {
		pub total_bonded_usd: Balance,
		pub total_bonded_plmc: Balance,
		pub evaluators_outcome: OldEvaluatorsOutcome<Balance>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum OldEvaluatorsOutcome<Balance> {
		Unchanged,
		Rewarded(RewardInfo<Balance>),
		Slashed,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum OldMigrationType {
		Offchain,
		Pallet(OldPalletMigrationInfo),
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct OldPalletMigrationInfo {
		/// ParaId of project
		pub parachain_id: ParaId,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
		/// Migration readiness check
		pub migration_readiness_check: Option<OldPalletMigrationReadinessCheck>,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct OldPalletMigrationReadinessCheck {
		pub holding_check: (xcm::v3::QueryId, CheckOutcome),
		pub pallet_check: (xcm::v3::QueryId, CheckOutcome),
	}

	type OldProjectDetailsOf<T> = OldProjectDetails<
		AccountIdOf<T>,
		Did,
		BlockNumberFor<T>,
		PriceOf<T>,
		BalanceOf<T>,
		OldEvaluationRoundInfo<BalanceOf<T>>,
	>;

	pub struct UncheckedMigrationToV5<T: Config>(PhantomData<T>);
	impl<T: Config> UncheckedOnRuntimeUpgrade for UncheckedMigrationToV5<T> {
		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			let mut items = 0;
			log::info!("Starting migration to V5");
			let mut translate_project_details = |key, item: OldProjectDetailsOf<T>| -> Option<ProjectDetailsOf<T>> {
				items += 1;
				log::info!("project_details item {:?}", items);
				let round_duration: BlockNumberPair<BlockNumberFor<T>>;
				let new_status = match item.status {
					OldProjectStatus::Application => {
						let start =
							item.phase_transition_points.application.start.expect("Application start block is missing");
						let end =
							item.phase_transition_points.application.end.expect("Application end block is missing");
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						ProjectStatus::Application
					},
					OldProjectStatus::EvaluationRound => {
						let start =
							item.phase_transition_points.evaluation.start.expect("Evaluation start block is missing");
						let end = item.phase_transition_points.evaluation.end.expect("Evaluation end block is missing");
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						ProjectStatus::EvaluationRound
					},
					OldProjectStatus::AuctionInitializePeriod => {
						let now = frame_system::Pallet::<T>::block_number();
						let start = now;
						let end = now + <T as Config>::AuctionRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						debug_assert!(false, "AuctionInitializePeriod is not supported in V5, no project should be in this state when upgrading storage");
						log::error!("AuctionInitializePeriod is not supported in V5, no project should be in this state when upgrading storage");
						ProjectStatus::AuctionRound
					},
					OldProjectStatus::AuctionOpening => {
						let start = item
							.phase_transition_points
							.auction_opening
							.start
							.expect("AuctionOpening start block is missing");
						let end = start + <T as Config>::AuctionRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						debug_assert!(false, "AuctionOpening is not supported in V5, no project should be in this state when upgrading storage");
						log::error!("AuctionOpening is not supported in V5, no project should be in this state when upgrading storage");
						ProjectStatus::AuctionRound
					},
					OldProjectStatus::AuctionClosing => {
						let start = item
							.phase_transition_points
							.auction_opening
							.start
							.expect("AuctionOpening start block is missing");
						let end = start + <T as Config>::AuctionRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						debug_assert!(false, "AuctionClosing is not supported in V5, no project should be in this state when upgrading storage");
						log::error!("AuctionClosing is not supported in V5, no project should be in this state when upgrading storage");
						ProjectStatus::AuctionRound
					},
					OldProjectStatus::CalculatingWAP => {
						let start = item
							.phase_transition_points
							.auction_opening
							.start
							.expect("AuctionOpening start block is missing");
						let end = start + <T as Config>::AuctionRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						debug_assert!(false, "CalculatingWAP is not supported in V5, no project should be in this state when upgrading storage");
						log::error!("CalculatingWAP is not supported in V5, no project should be in this state when upgrading storage");
						ProjectStatus::AuctionRound
					},
					OldProjectStatus::CommunityRound => {
						let start = item
							.phase_transition_points
							.community
							.start
							.expect("CommunityRound start block is missing");
						let end = start +
							<T as Config>::CommunityRoundDuration::get() +
							<T as Config>::RemainderRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						debug_assert!(
							false,
							"We should not upgrade runtime while a project is still in community round"
						);
						ProjectStatus::CommunityRound(
							item.phase_transition_points.community.end.expect("CommunityRound end block is missing") +
								One::one(),
						)
					},
					OldProjectStatus::RemainderRound => {
						let start = item
							.phase_transition_points
							.community
							.start
							.expect("CommunityRound start block is missing");
						let end = start +
							<T as Config>::CommunityRoundDuration::get() +
							<T as Config>::RemainderRoundDuration::get();
						round_duration = BlockNumberPair::new(Some(start), Some(end));
						ProjectStatus::CommunityRound(
							item.phase_transition_points.remainder.start.expect("Remainder start block is missing"),
						)
					},
					OldProjectStatus::FundingFailed => {
						round_duration = BlockNumberPair::new(None, None);
						ProjectStatus::SettlementStarted(FundingOutcome::Failure)
					},
					OldProjectStatus::AwaitingProjectDecision => {
						round_duration = BlockNumberPair::new(None, None);
						debug_assert!(false, "AwaitingProjectDecision is not supported in V5, no project should be in this state when upgrading storage");
						log::error!("AwaitingProjectDecision is not supported in V5, no project should be in this state when upgrading storage");
						ProjectStatus::FundingSuccessful
					},
					OldProjectStatus::FundingSuccessful => {
						round_duration = BlockNumberPair::new(None, None);
						ProjectStatus::SettlementStarted(FundingOutcome::Success)
					},
					OldProjectStatus::SettlementStarted(old_outcome) => {
						round_duration = BlockNumberPair::new(None, None);
						let outcome = match old_outcome {
							OldFundingOutcome::FundingSuccessful => FundingOutcome::Success,
							OldFundingOutcome::FundingFailed => FundingOutcome::Failure,
						};
						ProjectStatus::SettlementStarted(outcome)
					},
					OldProjectStatus::SettlementFinished(old_outcome) => {
						round_duration = BlockNumberPair::new(None, None);
						let outcome = match old_outcome {
							OldFundingOutcome::FundingSuccessful => FundingOutcome::Success,
							OldFundingOutcome::FundingFailed => FundingOutcome::Failure,
						};
						ProjectStatus::SettlementFinished(outcome)
					},
					OldProjectStatus::CTMigrationStarted => {
						round_duration = BlockNumberPair::new(None, None);
						ProjectStatus::CTMigrationStarted
					},
					OldProjectStatus::CTMigrationFinished => {
						round_duration = BlockNumberPair::new(None, None);
						ProjectStatus::CTMigrationFinished
					},
				};

				let evaluators_outcome = Some(match item.evaluation_round_info.evaluators_outcome {
					OldEvaluatorsOutcome::Unchanged => EvaluatorsOutcome::Rewarded(
						<Pallet<T>>::generate_evaluator_rewards_info(key)
							.expect("Evaluator rewards info should be generated"),
					),
					OldEvaluatorsOutcome::Rewarded(info) => EvaluatorsOutcome::<BalanceOf<T>>::Rewarded(info),
					OldEvaluatorsOutcome::Slashed => EvaluatorsOutcome::<BalanceOf<T>>::Slashed,
				});
				let evaluation_round_info = EvaluationRoundInfoOf::<T> {
					total_bonded_usd: item.evaluation_round_info.total_bonded_usd,
					total_bonded_plmc: item.evaluation_round_info.total_bonded_plmc,
					evaluators_outcome,
				};
				Some(ProjectDetailsOf::<T> {
					issuer_account: item.issuer_account,
					issuer_did: item.issuer_did,
					is_frozen: item.is_frozen,
					weighted_average_price: item.weighted_average_price,
					status: new_status,
					round_duration,
					fundraising_target_usd: item.fundraising_target_usd,
					remaining_contribution_tokens: item.remaining_contribution_tokens,
					funding_amount_reached_usd: item.funding_amount_reached_usd,
					evaluation_round_info,
					usd_bid_on_oversubscription: item.usd_bid_on_oversubscription,
					funding_end_block: item.funding_end_block,
					migration_type: None,
				})
			};
			crate::ProjectsDetails::<T>::translate(|key, object: OldProjectDetailsOf<T>| {
				translate_project_details(key, object)
			});

			T::DbWeight::get().reads_writes(items, items)
		}
	}

	pub type MigrationToV5<T> = frame_support::migrations::VersionedMigration<
		4,
		5,
		UncheckedMigrationToV5<T>,
		crate::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;
}
