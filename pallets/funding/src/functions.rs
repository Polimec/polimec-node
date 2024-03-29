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

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! Functions for the Funding pallet.
use crate::ProjectStatus::FundingSuccessful;
use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::{Mutate, MutateHold as FungibleMutateHold},
		fungibles::{metadata::Mutate as MetadataMutate, Create, Inspect, Mutate as FungiblesMutate},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
	transactional,
};
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use polimec_common::{
	credentials::{Did, InvestorType},
	ReleaseSchedule,
};
use sp_arithmetic::{
	traits::{CheckedDiv, CheckedSub, Zero},
	Percent, Perquintill,
};
use sp_runtime::traits::{Convert, ConvertBack};
use sp_std::{marker::PhantomData, ops::Not};
use xcm::v3::MaxDispatchErrorLen;

use super::*;
use crate::traits::{BondingRequirementCalculation, ProvideAssetPrice, VestingDurationCalculation};
use polimec_common::migration_types::{MigrationInfo, MigrationOrigin, Migrations, ParticipationType};
const POLIMEC_PARA_ID: u32 = 3344u32;
const QUERY_RESPONSE_TIME_WINDOW_BLOCKS: u32 = 20u32;

// Round transitions
impl<T: Config> Pallet<T> {
	/// Called by user extrinsic
	/// Starts the evaluation round of a project. It needs to be called by the project issuer.
	///
	/// # Arguments
	/// * `project_id` - The id of the project to start the evaluation round for.
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Checking and updating the round status, transition points and freezing the project.
	/// * [`ProjectsToUpdate`] - Scheduling the project for automatic transition by on_initialize later on.
	///
	/// # Success path
	/// The project information is found, its round status was in Application round, and It's not yet frozen.
	/// The pertinent project info is updated on the storage, and the project is scheduled for automatic transition by on_initialize.
	///
	/// # Next step
	/// Users will pond PLMC for this project, and when the time comes, the project will be transitioned
	/// to the next round by `on_initialize` using [`do_evaluation_end`](Self::do_evaluation_end)
	#[transactional]
	pub fn do_start_evaluation(caller: AccountIdOf<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer_account == caller, Error::<T>::NotAllowed);
		ensure!(project_details.status == ProjectStatus::Application, Error::<T>::ProjectNotInApplicationRound);
		ensure!(!project_details.is_frozen, Error::<T>::ProjectAlreadyFrozen);
		ensure!(project_metadata.offchain_information_hash.is_some(), Error::<T>::MetadataNotProvided);

		// * Calculate new variables *
		let evaluation_end_block = now + T::EvaluationDuration::get();
		project_details.phase_transition_points.application.update(None, Some(now));
		project_details.phase_transition_points.evaluation.update(Some(now + 1u32.into()), Some(evaluation_end_block));
		project_details.is_frozen = true;
		project_details.status = ProjectStatus::EvaluationRound;

		// * Update storage *
		ProjectsDetails::<T>::insert(project_id, project_details);
		let actual_insertion_attempts = match Self::add_to_update_store(
			evaluation_end_block + 1u32.into(),
			(&project_id, UpdateType::EvaluationEnd),
		) {
			Ok(insertions) => insertions,
			Err(insertions) =>
				return Err(DispatchErrorWithPostInfo {
					post_info: PostDispatchInfo {
						actual_weight: Some(WeightInfoOf::<T>::start_evaluation(insertions)),
						pays_fee: Pays::Yes,
					},
					error: Error::<T>::TooManyInsertionAttempts.into(),
				}),
		};

		// * Emit events *
		Self::deposit_event(Event::EvaluationStarted { project_id });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_evaluation(actual_insertion_attempts)),
			pays_fee: Pays::Yes,
		})
	}

	/// Called automatically by on_initialize.
	/// Ends the evaluation round, and sets the current round to `AuctionInitializePeriod` if it
	/// reached enough PLMC bonding, or to `EvaluationFailed` if it didn't.
	///
	/// # Arguments
	/// * `project_id` - The id of the project to end the evaluation round for.
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Checking the round status and transition points for validity, and updating
	/// the round status and transition points in case of success or failure of the evaluation.
	/// * [`Evaluations`] - Checking that the threshold for PLMC bonded was reached, to decide
	/// whether the project failed or succeeded.
	///
	/// # Possible paths
	/// * Project achieves its evaluation goal. >=10% of the target funding was reached through bonding,
	/// so the project is transitioned to the [`AuctionInitializePeriod`](ProjectStatus::AuctionInitializePeriod) round. The project information
	/// is updated with the new transition points and round status.
	///
	/// * Project doesn't reach the evaluation goal - <10% of the target funding was reached
	/// through bonding, so the project is transitioned to the `EvaluationFailed` round. The project
	/// information is updated with the new rounds status and it is scheduled for automatic unbonding.
	///
	/// # Next step
	/// * Bonding achieved - The issuer calls an extrinsic within the set period to initialize the
	/// auction round. `auction` is called
	///
	/// * Bonding failed - `on_idle` at some point checks for failed evaluation projects, and
	/// unbonds the evaluators funds.
	#[transactional]
	pub fn do_evaluation_end(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_end_block =
			project_details.phase_transition_points.evaluation.end().ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target_usd = project_details.fundraising_target;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::ProjectNotInEvaluationRound);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// * Calculate new variables *
		let usd_total_amount_bonded = project_details.evaluation_round_info.total_bonded_usd;
		let evaluation_target_usd = <T as Config>::EvaluationSuccessThreshold::get() * fundraising_target_usd;

		let auction_initialize_period_start_block = now + 1u32.into();
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block + T::AuctionInitializePeriodDuration::get();

		// Check which logic path to follow
		let is_funded = usd_total_amount_bonded >= evaluation_target_usd;

		// * Branch in possible project paths *
		// Successful path
		if is_funded {
			// * Update storage *
			project_details
				.phase_transition_points
				.auction_initialize_period
				.update(Some(auction_initialize_period_start_block), Some(auction_initialize_period_end_block));
			project_details.status = ProjectStatus::AuctionInitializePeriod;
			ProjectsDetails::<T>::insert(project_id, project_details);
			let insertion_attempts = match Self::add_to_update_store(
				auction_initialize_period_end_block + 1u32.into(),
				(&project_id, UpdateType::EnglishAuctionStart),
			) {
				Ok(insertions) => insertions,
				Err(_insertions) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};

			// * Emit events *
			Self::deposit_event(Event::AuctionInitializePeriod {
				project_id,
				start_block: auction_initialize_period_start_block,
				end_block: auction_initialize_period_end_block,
			});

			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_evaluation_success(insertion_attempts)),
				pays_fee: Pays::Yes,
			});

		// Unsuccessful path
		} else {
			// * Update storage *
			project_details.status = ProjectStatus::EvaluationFailed;
			project_details.cleanup = Cleaner::Failure(CleanerState::Initialized(PhantomData::<Failure>));
			ProjectsDetails::<T>::insert(project_id, project_details.clone());
			let issuer_did = project_details.issuer_did.clone();
			DidWithActiveProjects::<T>::set(issuer_did, None);

			// * Emit events *
			Self::deposit_event(Event::EvaluationFailed { project_id });
			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_evaluation_failure()),
				pays_fee: Pays::Yes,
			});
		}
	}

	/// Called by user extrinsic
	/// Starts the auction round for a project. From the next block forward, any professional or
	/// institutional user can set bids for a token_amount/token_price pair.
	/// Any bids from this point until the candle_auction starts, will be considered as valid.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block is between the defined start and end blocks of the initialize period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the English Auction round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// english auction round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the [`bid`](Self::bid) extrinsic.
	/// Later on, `on_initialize` transitions the project into the candle auction round, by calling
	/// [`do_candle_auction`](Self::do_candle_auction).
	#[transactional]
	pub fn do_english_auction(caller: AccountIdOf<T>, project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_initialize_period_start_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.start()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;

		// * Validity checks *
		ensure!(
			caller == T::PalletId::get().into_account_truncating() || caller == project_details.issuer_account,
			Error::<T>::NotAllowed
		);

		ensure!(now >= auction_initialize_period_start_block, Error::<T>::TooEarlyForEnglishAuctionStart);
		// If the auction is first manually started, the automatic transition fails here. This
		// behaviour is intended, as it gracefully skips the automatic transition if the
		// auction was started manually.
		ensure!(
			project_details.status == ProjectStatus::AuctionInitializePeriod,
			Error::<T>::ProjectNotInAuctionInitializePeriodRound
		);

		// * Calculate new variables *
		let english_start_block = now + 1u32.into();
		let english_end_block = now + T::EnglishAuctionDuration::get();

		// * Update Storage *
		project_details
			.phase_transition_points
			.english_auction
			.update(Some(english_start_block), Some(english_end_block));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::English);
		ProjectsDetails::<T>::insert(project_id, project_details);

		let insertion_attempts;
		// Schedule for automatic transition to candle auction round
		match Self::add_to_update_store(english_end_block + 1u32.into(), (&project_id, UpdateType::CandleAuctionStart))
		{
			Ok(iterations) => {
				insertion_attempts = iterations;
			},
			Err(insertion_attempts) =>
				return Err(DispatchErrorWithPostInfo {
					post_info: PostDispatchInfo {
						actual_weight: Some(WeightInfoOf::<T>::start_auction_manually(insertion_attempts)),
						pays_fee: Pays::Yes,
					},
					error: Error::<T>::TooManyInsertionAttempts.into(),
				}),
		};

		// * Emit events *
		Self::deposit_event(Event::EnglishAuctionStarted { project_id, when: now });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_auction_manually(insertion_attempts)),
			pays_fee: Pays::Yes,
		})
	}

	/// Called automatically by on_initialize
	/// Starts the candle round for a project.
	/// Any bids from this point until the candle round ends, are not guaranteed. Only bids
	/// made before the random ending block between the candle start and end will be considered
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block after the english auction end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Candle Auction round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// candle auction round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the `bid` extrinsic,
	/// but now their bids are not guaranteed.
	/// Later on, `on_initialize` ends the candle auction round and starts the community round,
	/// by calling [`do_community_funding`](Self::do_community_funding).
	#[transactional]
	pub fn do_candle_auction(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let english_end_block =
			project_details.phase_transition_points.english_auction.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > english_end_block, Error::<T>::TooEarlyForCandleAuctionStart);
		ensure!(
			project_details.status == ProjectStatus::AuctionRound(AuctionPhase::English),
			Error::<T>::ProjectNotInEnglishAuctionRound
		);

		// * Calculate new variables *
		let candle_start_block = now + 1u32.into();
		let candle_end_block = now + T::CandleAuctionDuration::get();

		// * Update Storage *
		project_details.phase_transition_points.candle_auction.update(Some(candle_start_block), Some(candle_end_block));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic check by on_initialize. Success depending on enough funding reached
		let insertion_iterations = match Self::add_to_update_store(
			candle_end_block + 1u32.into(),
			(&project_id, UpdateType::CommunityFundingStart),
		) {
			Ok(iterations) => iterations,
			Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
		};

		// * Emit events *
		Self::deposit_event(Event::CandleAuctionStarted { project_id, when: now });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_candle_phase(insertion_iterations)),
			pays_fee: Pays::Yes,
		})
	}

	/// Called automatically by on_initialize
	/// Starts the community round for a project.
	/// Retail users now buy tokens instead of bidding on them. The price of the tokens are calculated
	/// based on the available bids, using the function [`calculate_weighted_average_price`](Self::calculate_weighted_average_price).
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, and the current block is after the candle auction end period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Community Funding round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// round.
	///
	/// # Next step
	/// Retail users buy tokens at the price set on the auction round.
	/// Later on, `on_initialize` ends the community round by calling [`do_remainder_funding`](Self::do_remainder_funding) and
	/// starts the remainder round, where anyone can buy at that price point.
	#[transactional]
	pub fn do_community_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_candle_start_block =
			project_details.phase_transition_points.candle_auction.start().ok_or(Error::<T>::FieldIsNone)?;
		let auction_candle_end_block =
			project_details.phase_transition_points.candle_auction.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > auction_candle_end_block, Error::<T>::TooEarlyForCommunityRoundStart);
		ensure!(
			project_details.status == ProjectStatus::AuctionRound(AuctionPhase::Candle),
			Error::<T>::ProjectNotInCandleAuctionRound
		);

		// * Calculate new variables *
		let end_block = Self::select_random_block(auction_candle_start_block, auction_candle_end_block);
		let community_start_block = now + 1u32.into();
		let community_end_block = now + T::CommunityFundingDuration::get();

		// * Update Storage *
		let calculation_result = Self::calculate_weighted_average_price(
			project_id,
			end_block,
			project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
		);
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		match calculation_result {
			Err(pallet_error) if pallet_error == Error::<T>::NoBidsFound.into() => {
				ProjectsDetails::<T>::insert(project_id, project_details);
				let insertion_iterations = match Self::add_to_update_store(
					<frame_system::Pallet<T>>::block_number() + 1u32.into(),
					(&project_id, UpdateType::FundingEnd),
				) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

				// * Emit events *
				Self::deposit_event(Event::AuctionFailed { project_id });

				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::start_community_funding_failure(insertion_iterations)),
					pays_fee: Pays::Yes,
				})
			},
			Err(e) => return Err(DispatchErrorWithPostInfo { post_info: ().into(), error: e }),
			Ok((accepted_bids_count, rejected_bids_count)) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_candle_ending = Some(end_block);
				project_details
					.phase_transition_points
					.community
					.update(Some(community_start_block), Some(community_end_block));
				project_details.status = ProjectStatus::CommunityRound;
				ProjectsDetails::<T>::insert(project_id, project_details);

				let insertion_iterations = match Self::add_to_update_store(
					community_end_block + 1u32.into(),
					(&project_id, UpdateType::RemainderFundingStart),
				) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

				// * Emit events *
				Self::deposit_event(Event::CommunityFundingStarted { project_id });

				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::start_community_funding_success(
						insertion_iterations,
						accepted_bids_count,
						rejected_bids_count,
					)),
					pays_fee: Pays::Yes,
				})
			},
		}
	}

	/// Called automatically by on_initialize
	/// Starts the remainder round for a project.
	/// Anyone can now buy tokens, until they are all sold out, or the time is reached.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, the current block is after the community funding end period, and there are still tokens left to sell.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the Remainder Funding round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// round.
	///
	/// # Next step
	/// Any users can now buy tokens at the price set on the auction round.
	/// Later on, `on_initialize` ends the remainder round, and finalizes the project funding, by calling
	/// [`do_end_funding`](Self::do_end_funding).
	#[transactional]
	pub fn do_remainder_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block =
			project_details.phase_transition_points.community.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::ProjectNotInCommunityRound);

		// Transition to remainder round was initiated by `do_community_funding`, but the ct
		// tokens where already sold in the community round. This transition is obsolete.
		ensure!(
			project_details.remaining_contribution_tokens > 0u32.into(),
			Error::<T>::RoundTransitionAlreadyHappened
		);

		// * Calculate new variables *
		let remainder_start_block = now + 1u32.into();
		let remainder_end_block = now + T::RemainderFundingDuration::get();

		// * Update Storage *
		project_details
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block));
		project_details.status = ProjectStatus::RemainderRound;
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic transition by `on_initialize`
		let insertion_iterations =
			match Self::add_to_update_store(remainder_end_block + 1u32.into(), (&project_id, UpdateType::FundingEnd)) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};

		// * Emit events *
		Self::deposit_event(Event::RemainderFundingStarted { project_id });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::start_remainder_funding(insertion_iterations)),
			pays_fee: Pays::Yes,
		})
	}

	/// Called automatically by on_initialize
	/// Ends the project funding, and calculates if the project was successfully funded or not.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Get the project information, and check if the project is in the correct
	/// round, the current block is after the remainder funding end period.
	/// Update the project information with the new round status.
	///
	/// # Success Path
	/// The validity checks pass, and either of 2 paths happen:
	///
	/// * Project achieves its funding target - the project info is set to a successful funding state,
	/// and the contribution token asset class is created with the same id as the project.
	///
	/// * Project doesn't achieve its funding target - the project info is set to an unsuccessful funding state.
	///
	/// # Next step
	/// If **successful**, bidders can claim:
	///	* Contribution tokens with [`vested_contribution_token_bid_mint_for`](Self::vested_contribution_token_bid_mint_for)
	/// * Bonded plmc with [`vested_plmc_bid_unbond_for`](Self::vested_plmc_bid_unbond_for)
	///
	/// And contributors can claim:
	/// * Contribution tokens with [`vested_contribution_token_purchase_mint_for`](Self::vested_contribution_token_purchase_mint_for)
	/// * Bonded plmc with [`vested_plmc_purchase_unbond_for`](Self::vested_plmc_purchase_unbond_for)
	///
	/// If **unsuccessful**, users every user should have their PLMC vesting unbonded.
	#[transactional]
	pub fn do_end_funding(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let remainder_end_block = project_details.phase_transition_points.remainder.end();
		let now = <frame_system::Pallet<T>>::block_number();
		let issuer_did = project_details.issuer_did.clone();

		// * Validity checks *
		ensure!(
			// Can end due to running out of CTs
			remaining_cts == Zero::zero() ||
				// or the auction being empty
				project_details.status == ProjectStatus::AuctionRound(AuctionPhase::Candle) ||
				// or the last funding round ending
				matches!(remainder_end_block, Some(end_block) if now > end_block),
			Error::<T>::TooEarlyForFundingEnd
		);
		// do_end_funding was already executed, but automatic transition was included in the
		// do_remainder_funding function. We gracefully skip the this transition.
		ensure!(
			!matches!(
				project_details.status,
				ProjectStatus::FundingSuccessful |
					ProjectStatus::FundingFailed |
					ProjectStatus::AwaitingProjectDecision
			),
			Error::<T>::RoundTransitionAlreadyHappened
		);

		// * Calculate new variables *
		let funding_target = project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let funding_reached = project_details.funding_amount_reached;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		DidWithActiveProjects::<T>::set(issuer_did, None);
		if funding_ratio <= Perquintill::from_percent(33u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			let insertion_iterations = Self::make_project_funding_fail(
				project_id,
				project_details,
				FailureReason::TargetNotReached,
				1u32.into(),
			)?;
			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_automatically_rejected_evaluators_slashed(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			});
		} else if funding_ratio <= Perquintill::from_percent(75u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			let insertion_iterations = match Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_slashed(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			})
		} else if funding_ratio < Perquintill::from_percent(90u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Unchanged;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			let insertion_iterations = match Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_awaiting_decision_evaluators_unchanged(
					insertion_iterations,
				)),
				pays_fee: Pays::Yes,
			})
		} else {
			let (reward_info, evaluations_count) = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Rewarded(reward_info);

			let insertion_iterations = Self::make_project_funding_successful(
				project_id,
				project_details,
				SuccessReason::ReachedTarget,
				T::SuccessToSettlementTime::get(),
			)?;
			return Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::end_funding_automatically_accepted_evaluators_rewarded(
					insertion_iterations,
					evaluations_count,
				)),
				pays_fee: Pays::Yes,
			});
		}
	}

	#[transactional]
	pub fn do_project_decision(project_id: ProjectId, decision: FundingOutcomeDecision) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		ensure!(
			project_details.status == ProjectStatus::AwaitingProjectDecision,
			Error::<T>::RoundTransitionAlreadyHappened
		);

		// * Update storage *
		match decision {
			FundingOutcomeDecision::AcceptFunding => {
				Self::make_project_funding_successful(
					project_id,
					project_details,
					SuccessReason::ProjectDecision,
					T::SuccessToSettlementTime::get(),
				)?;
				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::project_decision_accept_funding()),
					pays_fee: Pays::Yes,
				})
			},
			FundingOutcomeDecision::RejectFunding => {
				Self::make_project_funding_fail(
					project_id,
					project_details,
					FailureReason::ProjectDecision,
					T::SuccessToSettlementTime::get(),
				)?;
				Ok(PostDispatchInfo {
					actual_weight: Some(WeightInfoOf::<T>::project_decision_reject_funding()),
					pays_fee: Pays::Yes,
				})
			},
		}
	}

	#[transactional]
	pub fn do_start_settlement(project_id: ProjectId) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let token_information =
			ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?.token_information;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful ||
				project_details.status == ProjectStatus::FundingFailed,
			Error::<T>::NotAllowed
		);

		// * Calculate new variables *
		project_details.cleanup =
			Cleaner::try_from(project_details.status.clone()).map_err(|_| Error::<T>::NotAllowed)?;
		project_details.funding_end_block = Some(now);

		// * Update storage *
		ProjectsDetails::<T>::insert(project_id, &project_details);

		let escrow_account = Self::fund_account_id(project_id);
		if project_details.status == ProjectStatus::FundingSuccessful {
			T::ContributionTokenCurrency::create(project_id, escrow_account.clone(), false, 1_u32.into())?;
			T::ContributionTokenCurrency::set(
				project_id,
				&escrow_account.clone(),
				token_information.name.into(),
				token_information.symbol.into(),
				token_information.decimals,
			)?;

			let contribution_token_treasury_account = T::ContributionTreasury::get();
			T::ContributionTokenCurrency::touch(
				project_id,
				&contribution_token_treasury_account,
				&contribution_token_treasury_account,
			)?;

			let (liquidity_pools_ct_amount, long_term_holder_bonus_ct_amount) =
				Self::generate_liquidity_pools_and_long_term_holder_rewards(project_id)?;

			T::ContributionTokenCurrency::mint_into(
				project_id,
				&contribution_token_treasury_account,
				long_term_holder_bonus_ct_amount,
			)?;
			T::ContributionTokenCurrency::mint_into(
				project_id,
				&contribution_token_treasury_account,
				liquidity_pools_ct_amount,
			)?;

			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::start_settlement_funding_success()),
				pays_fee: Pays::Yes,
			})
		} else {
			Ok(PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::start_settlement_funding_failure()),
				pays_fee: Pays::Yes,
			})
		}
	}
}

// Extrinsics and HRMP interactions
impl<T: Config> Pallet<T> {
	fn project_validation(
		metadata: &ProjectMetadataOf<T>,
		issuer: AccountIdOf<T>,
		did: Did,
	) -> Result<(ProjectDetailsOf<T>, BucketOf<T>), DispatchError> {
		if let Err(error) = metadata.is_valid() {
			return match error {
				ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
				ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
				ValidityError::ParticipationCurrenciesError => Err(Error::<T>::ParticipationCurrenciesError.into()),
			};
		}
		let total_allocation_size = metadata.total_allocation_size;

		// * Calculate new variables *
		let fundraising_target =
			metadata.minimum_price.checked_mul_int(total_allocation_size).ok_or(Error::<T>::BadMath)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let project_details = ProjectDetails {
			issuer_account: issuer.clone(),
			issuer_did: did.clone(),
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints::new(now),
			remaining_contribution_tokens: metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<T>::zero(),
			cleanup: Cleaner::NotReady,
			evaluation_round_info: EvaluationRoundInfoOf::<T> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
			parachain_id: None,
			migration_readiness_check: None,
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Closed,
				polimec_to_project: ChannelStatus::Closed,
			},
		};

		let bucket: BucketOf<T> = Self::create_bucket_from_metadata(&metadata)?;

		Ok((project_details, bucket))
	}

	/// Called by user extrinsic
	/// Creates a project and assigns it to the `issuer` account.
	///
	/// # Arguments
	/// * `issuer` - The account that will be the issuer of the project.
	/// * `project` - The project struct containing all the necessary information.
	///
	/// # Storage access
	/// * [`ProjectsMetadata`] - Inserting the main project information. 1 to 1 with the `project` argument.
	/// * [`ProjectsDetails`] - Inserting the project information. constructed from the `project` argument.
	/// * [`NextProjectId`] - Getting the next usable id, and updating it for the next project.
	///
	/// # Success path
	/// The `project` argument is valid. A ProjectInfo struct is constructed, and the storage is updated
	/// with the new structs and mappings to reflect the new project creation
	///
	/// # Next step
	/// The issuer will call an extrinsic to start the evaluation round of the project.
	/// [`do_start_evaluation`](Self::do_start_evaluation) will be executed.
	#[transactional]
	pub fn do_create_project(
		issuer: &AccountIdOf<T>,
		initial_metadata: ProjectMetadataOf<T>,
		did: Did,
	) -> DispatchResult {
		// * Get variables *
		let project_id = NextProjectId::<T>::get();
		let maybe_active_project = DidWithActiveProjects::<T>::get(did.clone());

		// * Validity checks *
		ensure!(maybe_active_project == None, Error::<T>::IssuerHasActiveProjectAlready);

		let (project_details, bucket) = Self::project_validation(&initial_metadata, issuer.clone(), did.clone())?;

		// Each project needs an escrow system account to temporarily hold the USDT/USDC. We need to create it by depositing `ED` amount of PLMC into it.
		// This should be paid by the issuer.
		let escrow_account = Self::fund_account_id(project_id);
		// transfer ED from issuer to escrow
		T::NativeCurrency::transfer(
			issuer,
			&escrow_account,
			<T as pallet_balances::Config>::ExistentialDeposit::get(),
			Preservation::Preserve,
		)
		.map_err(|_| Error::<T>::NotEnoughFundsForEscrowCreation)?;

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, &initial_metadata);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());
		DidWithActiveProjects::<T>::set(did, Some(project_id));

		// * Emit events *
		Self::deposit_event(Event::ProjectCreated { project_id, issuer: issuer.clone() });

		Ok(())
	}

	#[transactional]
	pub fn do_remove_project(issuer: &AccountIdOf<T>, project_id: ProjectId, did: Did) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(&project_details.issuer_account == issuer, Error::<T>::NotAllowed);
		ensure!(project_details.is_frozen.not(), Error::<T>::Frozen);

		// * Update storage *
		ProjectsDetails::<T>::remove(project_id);
		ProjectsMetadata::<T>::remove(project_id);
		DidWithActiveProjects::<T>::set(did, None);
		Buckets::<T>::remove(project_id);

		// * Emit events *
		Self::deposit_event(Event::ProjectRemoved { project_id });

		Ok(())
	}

	/// Change the metadata hash of a project
	///
	/// # Arguments
	/// * `issuer` - The project issuer account
	/// * `project_id` - The project identifier
	/// * `project_metadata_hash` - The hash of the image that contains the metadata
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Check that the project is not frozen
	/// * [`ProjectsMetadata`] - Update the metadata hash
	#[transactional]
	pub fn do_edit_metadata(
		issuer: AccountIdOf<T>,
		project_id: ProjectId,
		new_project_metadata: ProjectMetadataOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer_account == issuer, Error::<T>::NotAllowed);
		ensure!(!project_details.is_frozen, Error::<T>::Frozen);

		// * Calculate new variables *
		let (project_details, bucket) =
			Self::project_validation(&new_project_metadata, issuer.clone(), project_details.issuer_did.clone())?;

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, new_project_metadata.clone());
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);

		// * Emit events *
		Self::deposit_event(Event::MetadataEdited { project_id, metadata: new_project_metadata });

		Ok(())
	}

	// Note: usd_amount needs to have the same amount of decimals as PLMC, so when multiplied by the plmc-usd price, it gives us the PLMC amount with the decimals we wanted.
	#[transactional]
	pub fn do_evaluate(
		evaluator: &AccountIdOf<T>,
		project_id: ProjectId,
		usd_amount: BalanceOf<T>,
		did: Did,
		investor_type: InvestorType,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = NextEvaluationId::<T>::get();
		let caller_existing_evaluations: Vec<(u32, EvaluationInfoOf<T>)> =
			Evaluations::<T>::iter_prefix((project_id, evaluator)).collect();
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target;
		let evaluation_round_info = &mut project_details.evaluation_round_info;
		let evaluations_count = EvaluationCounts::<T>::get(project_id);

		// * Validity Checks *
		ensure!(project_details.issuer_did != did, Error::<T>::ParticipationToThemselves);
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::EvaluationNotStarted);
		ensure!(evaluations_count < T::MaxEvaluationsPerProject::get(), Error::<T>::TooManyEvaluationsForProject);

		// * Calculate new variables *
		if investor_type == InvestorType::Retail {
			RetailParticipations::<T>::mutate(&did, |project_participations| {
				if project_participations.contains(&project_id).not() {
					// We don't care if it fails, since it means the user already has access to the max multiplier
					let _ = project_participations.try_push(project_id);
				}
			});
		}
		let plmc_bond = plmc_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_amount)
			.ok_or(Error::<T>::BadMath)?;
		let previous_total_evaluation_bonded_usd = evaluation_round_info.total_bonded_usd;

		let remaining_bond_to_reach_threshold =
			early_evaluation_reward_threshold_usd.saturating_sub(previous_total_evaluation_bonded_usd);

		let early_usd_amount = if usd_amount <= remaining_bond_to_reach_threshold {
			usd_amount
		} else {
			remaining_bond_to_reach_threshold
		};

		let late_usd_amount = usd_amount.checked_sub(&early_usd_amount).ok_or(Error::<T>::BadMath)?;

		let new_evaluation = EvaluationInfoOf::<T> {
			id: evaluation_id,
			project_id,
			evaluator: evaluator.clone(),
			original_plmc_bond: plmc_bond,
			current_plmc_bond: plmc_bond,
			early_usd_amount,
			late_usd_amount,
			when: now,
			rewarded_or_slashed: None,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		if caller_existing_evaluations.len() < T::MaxEvaluationsPerUser::get() as usize {
			T::NativeCurrency::hold(&HoldReason::Evaluation(project_id).into(), evaluator, plmc_bond)?;
		} else {
			let (low_id, lowest_evaluation) = caller_existing_evaluations
				.iter()
				.min_by_key(|(_, evaluation)| evaluation.original_plmc_bond)
				.ok_or(Error::<T>::ImpossibleState)?;

			ensure!(lowest_evaluation.original_plmc_bond < plmc_bond, Error::<T>::EvaluationBondTooLow);
			ensure!(
				lowest_evaluation.original_plmc_bond == lowest_evaluation.current_plmc_bond,
				"Using evaluation funds for participating should not be possible in the evaluation round"
			);

			T::NativeCurrency::release(
				&HoldReason::Evaluation(project_id).into(),
				&lowest_evaluation.evaluator,
				lowest_evaluation.original_plmc_bond,
				Precision::Exact,
			)?;

			T::NativeCurrency::hold(&HoldReason::Evaluation(project_id).into(), evaluator, plmc_bond)?;

			Evaluations::<T>::remove((project_id, evaluator, low_id));
			EvaluationCounts::<T>::mutate(project_id, |c| *c -= 1);
		}

		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), new_evaluation);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));
		evaluation_round_info.total_bonded_usd += usd_amount;
		evaluation_round_info.total_bonded_plmc += plmc_bond;
		ProjectsDetails::<T>::insert(project_id, project_details);
		EvaluationCounts::<T>::mutate(project_id, |c| *c += 1);

		// * Emit events *
		Self::deposit_event(Event::FundsBonded { project_id, amount: plmc_bond, bonder: evaluator.clone() });

		let existing_evaluations_count = caller_existing_evaluations.len() as u32;
		let actual_weight = if existing_evaluations_count < T::MaxEvaluationsPerUser::get() {
			WeightInfoOf::<T>::evaluation_to_limit(existing_evaluations_count)
		} else {
			WeightInfoOf::<T>::evaluation_over_limit()
		};

		Ok(PostDispatchInfo { actual_weight: Some(actual_weight), pays_fee: Pays::Yes })
	}

	/// Bid for a project in the bidding stage.
	///
	/// # Arguments
	/// * `bidder` - The account that is bidding
	/// * `project_id` - The project to bid for
	/// * `amount` - The amount of tokens that the bidder wants to buy
	/// * `multiplier` - Used for calculating how much PLMC needs to be bonded to spend this much money (in USD)
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Check that the project is in the bidding stage
	/// * [`BiddingBonds`] - Update the storage with the bidder's PLMC bond for that bid
	/// * [`Bids`] - Check previous bids by that user, and update the storage with the new bid
	#[transactional]
	pub fn do_bid(
		bidder: &AccountIdOf<T>,
		project_id: ProjectId,
		ct_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).ok_or(Error::<T>::PriceNotFound)?;
		let existing_bids = Bids::<T>::iter_prefix_values((project_id, bidder)).collect::<Vec<_>>();
		let bid_count = BidCounts::<T>::get(project_id);
		// User will spend at least this amount of USD for his bid(s). More if the bid gets split into different buckets
		let min_total_ticket_size =
			project_metadata.minimum_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		// weight return variables
		let mut perform_bid_calls = 0;
		let existing_bids_amount = existing_bids.len() as u32;
		let metadata_bidder_ticket_size_bounds = match investor_type {
			InvestorType::Institutional => project_metadata.bidding_ticket_sizes.institutional,
			InvestorType::Professional => project_metadata.bidding_ticket_sizes.professional,
			_ => return Err(Error::<T>::NotAllowed.into()),
		};
		let max_multiplier = match investor_type {
			InvestorType::Professional => PROFESSIONAL_MAX_MULTIPLIER,
			InvestorType::Institutional => INSTITUTIONAL_MAX_MULTIPLIER,
			// unreachable
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		// * Validity checks *
		ensure!(
			matches!(investor_type, InvestorType::Institutional | InvestorType::Professional),
			DispatchError::from("Retail investors are not allowed to bid")
		);

		ensure!(ct_amount > Zero::zero(), Error::<T>::BidTooLow);
		ensure!(bid_count < T::MaxBidsPerProject::get(), Error::<T>::TooManyBidsForProject);
		ensure!(did != project_details.issuer_did, Error::<T>::ParticipationToThemselves);
		ensure!(matches!(project_details.status, ProjectStatus::AuctionRound(_)), Error::<T>::AuctionNotStarted);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);

		ensure!(
			metadata_bidder_ticket_size_bounds.usd_ticket_above_minimum_per_participation(min_total_ticket_size),
			Error::<T>::BidTooLow
		);
		ensure!(multiplier.into() <= max_multiplier && multiplier.into() > 0u8, Error::<T>::ForbiddenMultiplier);

		// Note: We limit the CT Amount to the auction allocation size, to avoid long running loops.
		ensure!(
			ct_amount <= project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
			Error::<T>::NotAllowed
		);
		ensure!(existing_bids.len() < T::MaxBidsPerUser::get() as usize, Error::<T>::TooManyBidsForUser);

		// Fetch current bucket details and other required info
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let mut amount_to_bid = ct_amount;

		// While there's a remaining amount to bid for
		while !amount_to_bid.is_zero() {
			let bid_amount = if amount_to_bid <= current_bucket.amount_left {
				// Simple case, the bucket has enough to cover the bid
				amount_to_bid
			} else {
				// The bucket doesn't have enough to cover the bid, so we bid the remaining amount of the current bucket
				current_bucket.amount_left
			};
			let bid_id = NextBidId::<T>::get();

			Self::perform_do_bid(
				bidder,
				project_id,
				bid_amount,
				current_bucket.current_price,
				multiplier,
				funding_asset,
				bid_id,
				now,
				plmc_usd_price,
				did.clone(),
				metadata_bidder_ticket_size_bounds,
			)?;

			perform_bid_calls += 1;

			// Update the current bucket and reduce the amount to bid by the amount we just bid
			current_bucket.update(bid_amount);
			amount_to_bid.saturating_reduce(bid_amount);
		}

		// Note: If the bucket has been exhausted, the 'update' function has already made the 'current_bucket' point to the next one.
		Buckets::<T>::insert(project_id, current_bucket);

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::bid(existing_bids_amount, perform_bid_calls)),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	fn perform_do_bid(
		bidder: &AccountIdOf<T>,
		project_id: ProjectId,
		ct_amount: BalanceOf<T>,
		ct_usd_price: T::Price,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		bid_id: u32,
		now: BlockNumberFor<T>,
		plmc_usd_price: T::Price,
		did: Did,
		metadata_ticket_size_bounds: TicketSizeOf<T>,
	) -> Result<BidInfoOf<T>, DispatchError> {
		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;

		let total_usd_bid_by_did = AuctionBoughtUSD::<T>::get((project_id, did.clone()));
		ensure!(
			metadata_ticket_size_bounds
				.usd_ticket_below_maximum_per_did(total_usd_bid_by_did.saturating_add(ticket_size)),
			Error::<T>::BidTooHigh
		);

		let funding_asset_usd_price =
			T::PriceProvider::get_price(funding_asset.to_assethub_id()).ok_or(Error::<T>::PriceNotFound)?;

		// * Calculate new variables *
		let plmc_bond =
			Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price).map_err(|_| Error::<T>::BadMath)?;

		let funding_asset_amount_locked =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_assethub_id();

		let new_bid = BidInfoOf::<T> {
			id: bid_id,
			project_id,
			bidder: bidder.clone(),
			did: did.clone(),
			status: BidStatus::YetUnknown,
			original_ct_amount: ct_amount,
			original_ct_usd_price: ct_usd_price,
			final_ct_amount: ct_amount,
			final_ct_usd_price: ct_usd_price,
			funding_asset,
			funding_asset_amount_locked,
			multiplier,
			plmc_bond,
			plmc_vesting_info: None,
			when: now,
			funds_released: false,
			ct_minted: false,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		Self::try_plmc_participation_lock(bidder, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(bidder, project_id, funding_asset_amount_locked, asset_id)?;

		Bids::<T>::insert((project_id, bidder, bid_id), &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));
		BidCounts::<T>::mutate(project_id, |c| *c += 1);
		AuctionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount += ticket_size);

		Self::deposit_event(Event::Bid { project_id, amount: ct_amount, price: ct_usd_price, multiplier });

		Ok(new_bid)
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens the contributor tries to buy. Tokens
	///   are limited by the total amount of tokens available in the Community Round.
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	/// * asset: The asset used for the contribution
	#[transactional]
	pub fn do_community_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
	) -> DispatchResultWithPostInfo {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let did_has_winning_bid = DidWithWinningBids::<T>::get(project_id, did.clone());

		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::AuctionNotStarted);
		ensure!(did_has_winning_bid.not(), Error::<T>::UserHasWinningBids);

		let buyable_tokens = token_amount.min(project_details.remaining_contribution_tokens);
		project_details.remaining_contribution_tokens.saturating_reduce(buyable_tokens);

		Self::do_contribute(
			contributor,
			project_id,
			&mut project_details,
			buyable_tokens,
			multiplier,
			asset,
			investor_type,
			did,
		)
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens the contributor tries to buy. Tokens
	///   are limited by the total amount of tokens available after the Auction and Community rounds.
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	/// * asset: The asset used for the contribution
	#[transactional]
	pub fn do_remaining_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
		did: Did,
		investor_type: InvestorType,
	) -> DispatchResultWithPostInfo {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.status == ProjectStatus::RemainderRound, Error::<T>::AuctionNotStarted);
		let buyable_tokens = token_amount.min(project_details.remaining_contribution_tokens);

		let before = project_details.remaining_contribution_tokens;
		let remaining_cts_in_round = before.saturating_sub(buyable_tokens);
		project_details.remaining_contribution_tokens = remaining_cts_in_round;

		Self::do_contribute(
			contributor,
			project_id,
			&mut project_details,
			token_amount,
			multiplier,
			asset,
			investor_type,
			did,
		)
	}

	#[transactional]
	fn do_contribute(
		contributor: &AccountIdOf<T>,
		project_id: ProjectId,
		project_details: &mut ProjectDetailsOf<T>,
		buyable_tokens: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		investor_type: InvestorType,
		did: Did,
	) -> DispatchResultWithPostInfo {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let caller_existing_contributions =
			Contributions::<T>::iter_prefix_values((project_id, contributor)).collect::<Vec<_>>();
		let total_usd_bought_by_did = ContributionBoughtUSD::<T>::get((project_id, did.clone()));
		let now = <frame_system::Pallet<T>>::block_number();
		let ct_usd_price = project_details.weighted_average_price.ok_or(Error::<T>::AuctionNotStarted)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).ok_or(Error::<T>::PriceNotFound)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(funding_asset.to_assethub_id()).ok_or(Error::<T>::PriceNotFound)?;

		let ticket_size = ct_usd_price.checked_mul_int(buyable_tokens).ok_or(Error::<T>::BadMath)?;
		let contributor_ticket_size = match investor_type {
			InvestorType::Institutional => project_metadata.contributing_ticket_sizes.institutional,
			InvestorType::Professional => project_metadata.contributing_ticket_sizes.professional,
			InvestorType::Retail => project_metadata.contributing_ticket_sizes.retail,
		};
		let max_multiplier = match investor_type {
			InvestorType::Retail => {
				RetailParticipations::<T>::mutate(&did, |project_participations| {
					if project_participations.contains(&project_id).not() {
						// We don't care if it fails, since it means the user already has access to the max multiplier
						let _ = project_participations.try_push(project_id);
					}
					retail_max_multiplier_for_participations(project_participations.len() as u8)
				})
			},

			InvestorType::Professional => PROFESSIONAL_MAX_MULTIPLIER,
			InvestorType::Institutional => INSTITUTIONAL_MAX_MULTIPLIER,
		};
		// * Validity checks *
		ensure!(multiplier.into() <= max_multiplier && multiplier.into() > 0u8, Error::<T>::ForbiddenMultiplier);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);
		ensure!(did.clone() != project_details.issuer_did, Error::<T>::ParticipationToThemselves);
		ensure!(
			caller_existing_contributions.len() < T::MaxContributionsPerUser::get() as usize,
			Error::<T>::TooManyContributionsForUser
		);
		ensure!(
			contributor_ticket_size.usd_ticket_above_minimum_per_participation(ticket_size),
			Error::<T>::ContributionTooLow
		);
		ensure!(
			contributor_ticket_size.usd_ticket_below_maximum_per_did(total_usd_bought_by_did + ticket_size),
			Error::<T>::ContributionTooHigh
		);
		ensure!(
			project_metadata.participation_currencies.contains(&funding_asset),
			Error::<T>::FundingAssetNotAccepted
		);
		ensure!(did.clone() != project_details.issuer_did, Error::<T>::ParticipationToThemselves);
		ensure!(
			caller_existing_contributions.len() < T::MaxContributionsPerUser::get() as usize,
			Error::<T>::TooManyContributionsForUser
		);
		ensure!(
			contributor_ticket_size.usd_ticket_above_minimum_per_participation(ticket_size),
			Error::<T>::ContributionTooLow
		);
		ensure!(
			contributor_ticket_size.usd_ticket_below_maximum_per_did(total_usd_bought_by_did + ticket_size),
			Error::<T>::ContributionTooHigh
		);

		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price)?;
		let funding_asset_amount =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_assethub_id();

		let contribution_id = NextContributionId::<T>::get();
		let new_contribution = ContributionInfoOf::<T> {
			id: contribution_id,
			project_id,
			contributor: contributor.clone(),
			ct_amount: buyable_tokens,
			usd_contribution_amount: ticket_size,
			multiplier,
			funding_asset,
			funding_asset_amount,
			plmc_bond,
			plmc_vesting_info: None,
			funds_released: false,
			ct_minted: false,
			ct_migration_status: MigrationStatus::NotStarted,
		};

		// Try adding the new contribution to the system
		Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;

		Contributions::<T>::insert((project_id, contributor, contribution_id), &new_contribution);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));
		ContributionBoughtUSD::<T>::mutate((project_id, did), |amount| *amount += ticket_size);

		let remaining_cts_after_purchase = project_details.remaining_contribution_tokens;
		project_details.funding_amount_reached.saturating_accrue(new_contribution.usd_contribution_amount);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// If no CTs remain, end the funding phase

		let mut weight_round_end_flag: Option<u32> = None;
		if remaining_cts_after_purchase.is_zero() {
			let fully_filled_vecs_from_insertion =
				match Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::FundingEnd)) {
					Ok(iterations) => iterations,
					Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
				};

			weight_round_end_flag = Some(fully_filled_vecs_from_insertion);
		}

		// * Emit events *
		Self::deposit_event(Event::Contribution {
			project_id,
			contributor: contributor.clone(),
			amount: buyable_tokens,
			multiplier,
		});

		// return correct weight function
		let actual_weight = match weight_round_end_flag {
			None => Some(WeightInfoOf::<T>::contribution(caller_existing_contributions.len() as u32)),
			Some(fully_filled_vecs_from_insertion) => Some(WeightInfoOf::<T>::contribution_ends_round(
				caller_existing_contributions.len() as u32,
				fully_filled_vecs_from_insertion,
			)),
		};

		Ok(PostDispatchInfo { actual_weight, pays_fee: Pays::Yes })
	}

	#[transactional]
	pub fn do_decide_project_outcome(
		issuer: AccountIdOf<T>,
		project_id: ProjectId,
		decision: FundingOutcomeDecision,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer_account == issuer, Error::<T>::NotAllowed);
		ensure!(project_details.status == ProjectStatus::AwaitingProjectDecision, Error::<T>::NotAllowed);

		// * Update storage *
		let insertion_attempts: u32;
		match Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::ProjectDecision(decision))) {
			Ok(iterations) => insertion_attempts = iterations,
			Err(iterations) =>
				return Err(DispatchErrorWithPostInfo {
					post_info: PostDispatchInfo {
						actual_weight: Some(WeightInfoOf::<T>::decide_project_outcome(iterations)),
						pays_fee: Pays::Yes,
					},
					error: Error::<T>::TooManyInsertionAttempts.into(),
				}),
		};

		Self::deposit_event(Event::ProjectOutcomeDecided { project_id, decision });

		Ok(PostDispatchInfo {
			actual_weight: Some(WeightInfoOf::<T>::decide_project_outcome(insertion_attempts)),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	pub fn do_bid_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: ProjectId,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = bid.final_ct_amount;

		// weight return variables
		let mut ct_account_created = false;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!bid.ct_minted, Error::<T>::NotAllowed);
		ensure!(matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)), Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		bid.ct_minted = true;

		// * Update storage *
		if !T::ContributionTokenCurrency::contains(&project_id, &bid.bidder) {
			ct_account_created = true;
			T::ContributionTokenCurrency::touch(project_id, &bid.bidder, &bid.bidder)?;
		}
		T::ContributionTokenCurrency::mint_into(project_id, &bid.bidder, ct_amount)?;
		Bids::<T>::insert((project_id, bidder, bid_id), &bid);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id: bid.project_id,
			claimer: bidder.clone(),
			amount: ct_amount,
		});

		Ok(PostDispatchInfo {
			actual_weight: Some(if ct_account_created {
				WeightInfoOf::<T>::bid_ct_mint_for_with_ct_account_creation()
			} else {
				WeightInfoOf::<T>::bid_ct_mint_for_no_ct_account_creation()
			}),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	pub fn do_contribution_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: ProjectId,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = contribution.ct_amount;

		// weight return variables
		let mut ct_account_created = false;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!contribution.ct_minted, Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		contribution.ct_minted = true;

		// * Update storage *
		if !T::ContributionTokenCurrency::contains(&project_id, &contribution.contributor) {
			ct_account_created = true;
			T::ContributionTokenCurrency::touch(project_id, &contribution.contributor, &contribution.contributor)?;
		}
		T::ContributionTokenCurrency::mint_into(project_id, &contribution.contributor, ct_amount)?;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id,
			claimer: contributor.clone(),
			amount: ct_amount,
		});

		Ok(PostDispatchInfo {
			actual_weight: Some(if ct_account_created {
				WeightInfoOf::<T>::contribution_ct_mint_for_with_ct_account_creation()
			} else {
				WeightInfoOf::<T>::contribution_ct_mint_for_no_ct_account_creation()
			}),
			pays_fee: Pays::Yes,
		})
	}

	#[transactional]
	pub fn do_evaluation_unbond_for(
		releaser: &AccountIdOf<T>,
		project_id: ProjectId,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut released_evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;
		let release_amount = released_evaluation.current_plmc_bond;

		// * Validity checks *
		ensure!(
			(project_details.evaluation_round_info.evaluators_outcome == EvaluatorsOutcomeOf::<T>::Unchanged ||
				released_evaluation.rewarded_or_slashed.is_some()) &&
				matches!(
					project_details.status,
					ProjectStatus::EvaluationFailed | ProjectStatus::FundingFailed | ProjectStatus::FundingSuccessful
				),
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(
			&HoldReason::Evaluation(project_id).into(),
			evaluator,
			released_evaluation.current_plmc_bond,
			Precision::Exact,
		)?;

		released_evaluation.current_plmc_bond = Zero::zero();
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), released_evaluation);

		// FIXME: same question as removing bid
		// Evaluations::<T>::remove((project_id, evaluator, evaluation_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: release_amount,
			bonder: evaluator.clone(),
			releaser: releaser.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_evaluation_reward_payout_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let reward_info =
			if let EvaluatorsOutcome::Rewarded(info) = project_details.evaluation_round_info.evaluators_outcome {
				info
			} else {
				return Err(Error::<T>::NotAllowed.into());
			};
		let mut evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// weight return variables
		let mut ct_account_created = false;

		// * Validity checks *
		ensure!(
			evaluation.rewarded_or_slashed.is_none() &&
				matches!(project_details.status, ProjectStatus::FundingSuccessful),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let early_reward_weight =
			Perquintill::from_rational(evaluation.early_usd_amount, reward_info.early_evaluator_total_bonded_usd);
		let normal_reward_weight = Perquintill::from_rational(
			evaluation.late_usd_amount.saturating_add(evaluation.early_usd_amount),
			reward_info.normal_evaluator_total_bonded_usd,
		);
		let early_evaluators_rewards = early_reward_weight * reward_info.early_evaluator_reward_pot;
		let normal_evaluators_rewards = normal_reward_weight * reward_info.normal_evaluator_reward_pot;
		let total_reward_amount = early_evaluators_rewards.saturating_add(normal_evaluators_rewards);

		// * Update storage *
		if !T::ContributionTokenCurrency::contains(&project_id, &evaluation.evaluator) {
			ct_account_created = true;
			T::ContributionTokenCurrency::touch(project_id, &evaluation.evaluator, &evaluation.evaluator)?;
		}
		T::ContributionTokenCurrency::mint_into(project_id, &evaluation.evaluator, total_reward_amount)?;
		evaluation.rewarded_or_slashed = Some(RewardOrSlash::Reward(total_reward_amount));
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), evaluation);

		// * Emit events *
		Self::deposit_event(Event::EvaluationRewarded {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			amount: total_reward_amount,
			caller: caller.clone(),
		});

		Ok(if ct_account_created {
			PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::evaluation_reward_payout_for_with_ct_account_creation()),
				pays_fee: Pays::Yes,
			}
		} else {
			PostDispatchInfo {
				actual_weight: Some(WeightInfoOf::<T>::evaluation_reward_payout_for_no_ct_account_creation()),
				pays_fee: Pays::Yes,
			}
		})
	}

	#[transactional]
	pub fn do_evaluation_slash_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::ProtocolGrowthTreasury::get();

		let mut evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			evaluation.rewarded_or_slashed.is_none() &&
				matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		// * Update storage *
		evaluation.rewarded_or_slashed = Some(RewardOrSlash::Slash(slashed_amount));

		T::NativeCurrency::transfer_on_hold(
			&HoldReason::Evaluation(project_id).into(),
			evaluator,
			&treasury_account,
			slashed_amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;

		evaluation.current_plmc_bond.saturating_reduce(slashed_amount);
		Evaluations::<T>::insert((project_id, evaluator, evaluation.id), evaluation);

		// * Emit events *
		Self::deposit_event(Event::EvaluationSlashed {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			amount: slashed_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_start_bid_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;

		// * Validity checks *
		ensure!(
			bid.plmc_vesting_info.is_none() &&
				project_details.status == ProjectStatus::FundingSuccessful &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let vest_info =
			Self::calculate_vesting_info(bidder, bid.multiplier, bid.plmc_bond).map_err(|_| Error::<T>::BadMath)?;
		bid.plmc_vesting_info = Some(vest_info);

		// * Update storage *
		T::Vesting::add_release_schedule(
			bidder,
			vest_info.total_amount,
			vest_info.amount_per_block,
			funding_end_block,
			HoldReason::Participation(project_id).into(),
		)?;
		Bids::<T>::insert((project_id, bidder, bid_id), bid);

		// * Emit events *
		Self::deposit_event(Event::BidPlmcVestingScheduled {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: vest_info.total_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_start_contribution_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
		let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;

		// * Validity checks *
		ensure!(
			contribution.plmc_vesting_info.is_none() && project_details.status == ProjectStatus::FundingSuccessful,
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let vest_info = Self::calculate_vesting_info(contributor, contribution.multiplier, contribution.plmc_bond)
			.map_err(|_| Error::<T>::BadMath)?;
		contribution.plmc_vesting_info = Some(vest_info);

		// * Update storage *
		T::Vesting::add_release_schedule(
			contributor,
			vest_info.total_amount,
			vest_info.amount_per_block,
			funding_end_block,
			HoldReason::Participation(project_id).into(),
		)?;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionPlmcVestingScheduled {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: vest_info.total_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_vest_plmc_for(
		caller: AccountIdOf<T>,
		project_id: ProjectId,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(matches!(project_details.status, ProjectStatus::FundingSuccessful), Error::<T>::NotAllowed);

		// * Update storage *
		let vested_amount = T::Vesting::vest(participant.clone(), HoldReason::Participation(project_id).into())?;

		// * Emit events *
		Self::deposit_event(Event::ParticipantPlmcVested { project_id, participant, amount: vested_amount, caller });

		Ok(())
	}

	#[transactional]
	pub fn do_release_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingFailed &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = bid.funding_asset_amount_locked;
		let payout_asset = bid.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_assethub_id(),
			&project_pot,
			bidder,
			payout_amount,
			Preservation::Expendable,
		)?;
		bid.funds_released = true;
		Bids::<T>::insert((project_id, bidder, bid_id), bid);

		// * Emit events *
		Self::deposit_event(Event::BidFundingReleased {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	// Unbond the PLMC of a bid instantly, following a failed funding outcome.
	// Unbonding of PLMC in a successful funding outcome is handled by the vesting schedule.
	#[transactional]
	pub fn do_bid_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingFailed &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)) &&
				bid.funds_released,
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(
			&HoldReason::Participation(project_id).into(),
			bidder,
			bid.plmc_bond,
			Precision::Exact,
		)?;

		Bids::<T>::remove((project_id, bidder, bid_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: bid.plmc_bond,
			bonder: bidder.clone(),
			releaser: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_release_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution = Contributions::<T>::get((project_id, contributor, contribution_id))
			.ok_or(Error::<T>::ContributionNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingFailed, Error::<T>::NotAllowed);

		// * Calculate variables *
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = contribution.funding_asset_amount;
		let payout_asset = contribution.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_assethub_id(),
			&project_pot,
			contributor,
			payout_amount,
			Preservation::Expendable,
		)?;
		contribution.funds_released = true;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionFundingReleased {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	// Unbond the PLMC of a contribution instantly, following a failed funding outcome.
	// Unbonding of PLMC in a successful funding outcome is handled by the vesting schedule.
	#[transactional]
	pub fn do_contribution_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let bid = Contributions::<T>::get((project_id, contributor, contribution_id))
			.ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingFailed, Error::<T>::NotAllowed);

		// * Update Storage *
		T::NativeCurrency::release(
			&HoldReason::Participation(project_id).into(),
			contributor,
			bid.plmc_bond,
			Precision::Exact,
		)?;

		Contributions::<T>::remove((project_id, contributor, contribution_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: bid.plmc_bond,
			bonder: contributor.clone(),
			releaser: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_payout_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let issuer = project_details.issuer_account;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = bid.funding_asset_amount_locked;
		let payout_asset = bid.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_assethub_id(),
			&project_pot,
			&issuer,
			payout_amount,
			Preservation::Expendable,
		)?;
		bid.funds_released = true;
		Bids::<T>::insert((project_id, bidder, bid_id), &bid);

		// * Emit events *
		Self::deposit_event(Event::BidFundingPaidOut {
			project_id,
			bidder: bidder.clone(),
			id: bid_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_payout_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);

		// * Calculate variables *
		let issuer = project_details.issuer_account;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = contribution.funding_asset_amount;
		let payout_asset = contribution.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_assethub_id(),
			&project_pot,
			&issuer,
			payout_amount,
			Preservation::Expendable,
		)?;
		contribution.funds_released = true;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionFundingPaidOut {
			project_id,
			contributor: contributor.clone(),
			id: contribution_id,
			amount: payout_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	#[transactional]
	pub fn do_set_para_id_for_project(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		para_id: ParaId,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(&(project_details.issuer_account) == caller, Error::<T>::NotAllowed);

		// * Update storage *
		project_details.parachain_id = Some(para_id);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::ProjectParaIdSet { project_id, para_id, caller: caller.clone() });

		Ok(())
	}

	pub fn do_handle_channel_open_request(message: Instruction) -> XcmResult {
		// TODO: set these constants with a proper value
		const EXECUTION_DOT: MultiAsset = MultiAsset {
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
			fun: Fungible(1_0_000_000_000u128),
		};
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		let max_message_size_thresholds = T::MaxMessageSizeThresholds::get();
		let max_capacity_thresholds = T::MaxCapacityThresholds::get();

		log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest received: {:?}", message);

		match message {
			Instruction::HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity }
				if max_message_size >= max_message_size_thresholds.0 &&
					max_message_size <= max_message_size_thresholds.1 &&
					max_capacity >= max_capacity_thresholds.0 &&
					max_capacity <= max_capacity_thresholds.1 =>
			{
				log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest accepted");

				let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
					.find(|(_id, details)| {
						details.parachain_id == Some(ParaId::from(sender)) && details.status == FundingSuccessful
					})
					.ok_or(XcmError::BadOrigin)?;

				let mut accept_channel_relay_call = vec![60u8, 1];
				let sender_id = ParaId::from(sender).encode();
				accept_channel_relay_call.extend_from_slice(&sender_id);

				let mut request_channel_relay_call = vec![60u8, 0];
				let recipient = ParaId::from(sender).encode();
				request_channel_relay_call.extend_from_slice(&recipient);
				let proposed_max_capacity = T::RequiredMaxCapacity::get().encode();
				request_channel_relay_call.extend_from_slice(&proposed_max_capacity);
				let proposed_max_message_size = T::RequiredMaxMessageSize::get().encode();
				request_channel_relay_call.extend_from_slice(&proposed_max_message_size);

				let xcm: Xcm<()> = Xcm(vec![
					WithdrawAsset(vec![EXECUTION_DOT.clone()].into()),
					BuyExecution { fees: EXECUTION_DOT.clone(), weight_limit: Unlimited },
					Transact {
						origin_kind: OriginKind::Native,
						require_weight_at_most: MAX_WEIGHT,
						call: accept_channel_relay_call.into(),
					},
					Transact {
						origin_kind: OriginKind::Native,
						require_weight_at_most: MAX_WEIGHT,
						call: request_channel_relay_call.into(),
					},
					RefundSurplus,
					DepositAsset {
						assets: Wild(All),
						beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(POLIMEC_PARA_ID)) },
					},
				]);
				let mut message = Some(xcm);

				let dest_loc = MultiLocation { parents: 1, interior: Here };
				let mut destination = Some(dest_loc);
				let (ticket, _price) = T::XcmRouter::validate(&mut destination, &mut message)?;

				match T::XcmRouter::deliver(ticket) {
					Ok(_) => {
						log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest: acceptance successfully sent");
						project_details.hrmp_channel_status.project_to_polimec = ChannelStatus::Open;
						project_details.hrmp_channel_status.polimec_to_project = ChannelStatus::AwaitingAcceptance;
						ProjectsDetails::<T>::insert(project_id, project_details);

						Pallet::<T>::deposit_event(Event::<T>::HrmpChannelAccepted {
							project_id,
							para_id: ParaId::from(sender),
						});
						Ok(())
					},
					Err(e) => {
						log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest: acceptance sending failed - {:?}", e);
						Err(XcmError::Unimplemented)
					},
				}
			},
			instr => {
				log::trace!(target: "pallet_funding::hrmp", "Bad instruction: {:?}", instr);
				Err(XcmError::Unimplemented)
			},
		}
	}

	pub fn do_handle_channel_accepted(message: Instruction) -> XcmResult {
		match message {
			Instruction::HrmpChannelAccepted { recipient } => {
				log::trace!(target: "pallet_funding::hrmp", "HrmpChannelAccepted received: {:?}", message);
				let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
					.find(|(_id, details)| {
						details.parachain_id == Some(ParaId::from(recipient)) && details.status == FundingSuccessful
					})
					.ok_or(XcmError::BadOrigin)?;

				project_details.hrmp_channel_status.polimec_to_project = ChannelStatus::Open;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Pallet::<T>::deposit_event(Event::<T>::HrmpChannelEstablished {
					project_id,
					para_id: ParaId::from(recipient),
				});

				Pallet::<T>::do_start_migration_readiness_check(
					&(T::PalletId::get().into_account_truncating()),
					project_id,
				)
				.map_err(|_| XcmError::NoDeal)?;
				Ok(())
			},
			instr => {
				log::trace!(target: "pallet_funding::hrmp", "Bad instruction: {:?}", instr);
				Err(XcmError::Unimplemented)
			},
		}
	}

	#[transactional]
	pub fn do_start_migration_readiness_check(caller: &AccountIdOf<T>, project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let parachain_id: u32 = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?.into();
		let project_multilocation = ParentThen(X1(Parachain(parachain_id)));
		let now = <frame_system::Pallet<T>>::block_number();

		// TODO: check these values
		let max_weight = Weight::from_parts(700_000_000, 10_000);

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(
			project_details.hrmp_channel_status ==
				HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Open,
					polimec_to_project: ChannelStatus::Open
				},
			Error::<T>::CommsNotEstablished
		);
		if project_details.migration_readiness_check.is_none() {
			ensure!(caller.clone() == T::PalletId::get().into_account_truncating(), Error::<T>::NotAllowed);
		} else if matches!(
			project_details.migration_readiness_check,
			Some(MigrationReadinessCheck {
				holding_check: (_, CheckOutcome::Failed),
				pallet_check: (_, CheckOutcome::Failed),
				..
			})
		) {
			ensure!(caller == &project_details.issuer_account, Error::<T>::NotAllowed);
		}

		// * Update storage *
		let call = Call::<T>::migration_check_response { query_id: Default::default(), response: Default::default() };

		let query_id_holdings = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			<T as Config>::RuntimeCall::from(call.clone()),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);
		let query_id_pallet = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			<T as Config>::RuntimeCall::from(call),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);

		project_details.migration_readiness_check = Some(MigrationReadinessCheck {
			holding_check: (query_id_holdings, CheckOutcome::AwaitingResponse),
			pallet_check: (query_id_pallet, CheckOutcome::AwaitingResponse),
		});
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Send the migration query *
		let expected_tokens: MultiAsset =
			(MultiLocation { parents: 0, interior: Here }, 1_000_000_0_000_000_000u128).into(); // 1MM units for migrations
		let xcm = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			WithdrawAsset(vec![expected_tokens].into()),
			ReportHolding {
				response_info: QueryResponseInfo {
					destination: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into(),
					query_id: 0,
					max_weight,
				},
				assets: Wild(All),
			},
			QueryPallet {
				module_name: Vec::from("polimec_receiver"),
				response_info: QueryResponseInfo {
					destination: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into(),
					query_id: 1,
					max_weight,
				},
			},
			DepositAsset { assets: Wild(All), beneficiary: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into() },
		]);
		<pallet_xcm::Pallet<T>>::send_xcm(Here, project_multilocation, xcm).map_err(|_| Error::<T>::XcmFailed)?;

		// * Emit events *
		Self::deposit_event(Event::<T>::MigrationReadinessCheckStarted { project_id, caller: caller.clone() });

		Ok(())
	}

	#[transactional]
	pub fn do_migration_check_response(
		location: MultiLocation,
		query_id: xcm::v3::QueryId,
		response: xcm::v3::Response,
	) -> DispatchResult {
		use xcm::v3::prelude::*;
		// TODO: check if this is too low performance. Maybe we want a new map of query_id -> project_id
		let (project_id, mut project_details, mut migration_check) = ProjectsDetails::<T>::iter()
			.find_map(|(project_id, details)| {
				if let Some(check @ MigrationReadinessCheck { holding_check, pallet_check }) =
					details.migration_readiness_check
				{
					if holding_check.0 == query_id || pallet_check.0 == query_id {
						return Some((project_id, details, check));
					}
				}
				None
			})
			.ok_or(Error::<T>::NotAllowed)?;

		let para_id = if let MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } = location {
			ParaId::from(para_id)
		} else {
			return Err(Error::<T>::NotAllowed.into());
		};

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let contribution_tokens_sold =
			project_metadata.total_allocation_size.saturating_sub(project_details.remaining_contribution_tokens);
		ensure!(project_details.parachain_id == Some(para_id), Error::<T>::NotAllowed);

		match (response.clone(), migration_check) {
			(
				Response::Assets(assets),
				MigrationReadinessCheck { holding_check: (_, CheckOutcome::AwaitingResponse), .. },
			) => {
				let ct_sold_as_u128: u128 = contribution_tokens_sold.try_into().map_err(|_| Error::<T>::BadMath)?;
				let assets: Vec<MultiAsset> = assets.into_inner();
				let asset_1 = assets[0].clone();
				match asset_1 {
					MultiAsset {
						id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(pid)) }),
						fun: Fungible(amount),
					} if amount >= ct_sold_as_u128 && pid == u32::from(para_id) => {
						migration_check.holding_check.1 = CheckOutcome::Passed;
						Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted {
							project_id,
							query_id,
							response,
						});
					},
					_ => {
						migration_check.holding_check.1 = CheckOutcome::Failed;
						Self::deposit_event(Event::<T>::MigrationCheckResponseRejected {
							project_id,
							query_id,
							response,
						});
					},
				}
			},

			(
				Response::PalletsInfo(pallets_info),
				MigrationReadinessCheck { pallet_check: (_, CheckOutcome::AwaitingResponse), .. },
			) =>
				if pallets_info.len() == 1 && pallets_info[0] == T::PolimecReceiverInfo::get() {
					migration_check.pallet_check.1 = CheckOutcome::Passed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted { project_id, query_id, response });
				} else {
					migration_check.pallet_check.1 = CheckOutcome::Failed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseRejected { project_id, query_id, response });
				},
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		project_details.migration_readiness_check = Some(migration_check);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Ok(())
	}

	#[transactional]
	pub fn do_start_migration(caller: &AccountIdOf<T>, project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_readiness_check = project_details.migration_readiness_check.ok_or(Error::<T>::NotAllowed)?;

		// * Validity Checks *
		ensure!(caller.clone() == project_details.issuer_account, Error::<T>::NotAllowed);

		ensure!(migration_readiness_check.is_ready(), Error::<T>::NotAllowed);

		// Start automated migration process

		// * Emit events *
		Self::deposit_event(Event::<T>::MigrationStarted { project_id });

		Ok(())
	}

	#[transactional]
	pub fn do_migrate_one_participant(
		caller: AccountIdOf<T>,
		project_id: ProjectId,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_readiness_check = project_details.migration_readiness_check.ok_or(Error::<T>::NotAllowed)?;
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, participant.clone()));
		let user_bids = Bids::<T>::iter_prefix_values((project_id, participant.clone()));
		let user_contributions = Contributions::<T>::iter_prefix_values((project_id, participant.clone()));
		let project_para_id = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity Checks *
		ensure!(migration_readiness_check.is_ready(), Error::<T>::NotAllowed);

		// * Process Data *
		// u128 is a balance, u64 is now a BlockNumber, but will be a Moment/Timestamp in the future
		let evaluation_migrations =
			user_evaluations.filter_map(|evaluation| MigrationGenerator::<T>::evaluation_migration(evaluation));
		let bid_migrations = user_bids.filter_map(|bid| MigrationGenerator::<T>::bid_migration(bid));
		let contribution_migrations =
			user_contributions.filter_map(|contribution| MigrationGenerator::<T>::contribution_migration(contribution));

		let migrations = evaluation_migrations.chain(bid_migrations).chain(contribution_migrations).collect_vec();
		let migrations = Migrations::from(migrations);

		let constructed_migrations = Self::construct_migration_xcm_messages(migrations);
		for (migrations, xcm) in constructed_migrations {
			let project_multilocation = MultiLocation { parents: 1, interior: X1(Parachain(project_para_id.into())) };
			let project_migration_origins = ProjectMigrationOriginsOf::<T> {
				project_id,
				migration_origins: migrations
					.origins()
					.try_into()
					.expect("construct function uses same constraint T::MaxMigrationsPerXcm"),
			};

			let call: <T as Config>::RuntimeCall =
				Call::confirm_migrations { query_id: Default::default(), response: Default::default() }.into();
			let transact_response_query_id =
				pallet_xcm::Pallet::<T>::new_notify_query(project_multilocation, call.into(), now + 20u32.into(), Here);
			// TODO: check these values
			let max_weight = Weight::from_parts(700_000_000, 10_000);

			let mut instructions = xcm.into_inner();
			instructions.push(ReportTransactStatus(QueryResponseInfo {
				destination: ParentThen(X1(Parachain(POLIMEC_PARA_ID))).into(),
				query_id: transact_response_query_id,
				max_weight,
			}));
			let xcm = Xcm(instructions);

			<pallet_xcm::Pallet<T>>::send_xcm(Here, project_multilocation, xcm).map_err(|_| Error::<T>::XcmFailed)?;
			Self::mark_migrations_as_sent(project_migration_origins.clone(), transact_response_query_id);
			UnconfirmedMigrations::<T>::insert(transact_response_query_id, project_migration_origins);

			Self::deposit_event(Event::<T>::UserMigrationSent {
				project_id,
				caller: caller.clone(),
				participant: participant.clone(),
			});
		}
		Ok(())
	}

	#[transactional]
	pub fn do_confirm_migrations(location: MultiLocation, query_id: QueryId, response: Response) -> DispatchResult {
		use xcm::v3::prelude::*;
		let unconfirmed_migrations = UnconfirmedMigrations::<T>::take(query_id).ok_or(Error::<T>::NotAllowed)?;
		let project_id = unconfirmed_migrations.project_id;
		let para_id = if let MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } = location {
			ParaId::from(para_id)
		} else {
			return Err(Error::<T>::NotAllowed.into());
		};
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.parachain_id == Some(para_id), Error::<T>::NotAllowed);

		match response {
			Response::DispatchResult(MaybeErrorCode::Success) => {
				Self::mark_migrations_as_confirmed(unconfirmed_migrations.clone());
				Self::deposit_event(Event::MigrationsConfirmed {
					project_id,
					migration_origins: unconfirmed_migrations.migration_origins,
				});
				// Self::deposit_event(Event::MigrationsConfirmed { project_id });
				Ok(())
			},
			Response::DispatchResult(MaybeErrorCode::Error(e)) |
			Response::DispatchResult(MaybeErrorCode::TruncatedError(e)) => {
				Self::mark_migrations_as_failed(unconfirmed_migrations.clone(), e);
				Self::deposit_event(Event::MigrationsFailed {
					project_id,
					migration_origins: unconfirmed_migrations.migration_origins,
				});
				// Self::deposit_event(Event::MigrationsFailed { project_id});
				Ok(())
			},
			_ => Err(Error::<T>::NotAllowed.into()),
		}
	}
}

// Helper functions
// ATTENTION: if this is called directly, it will not be transactional
impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: ProjectId) -> AccountIdOf<T> {
		// since the project_id starts at 0, we need to add 1 to get a different sub_account than the pallet account.
		T::PalletId::get().into_sub_account_truncating(index.saturating_add(One::one()))
	}

	/// Adds a project to the ProjectsToUpdate storage, so it can be updated at some later point in time.
	pub fn add_to_update_store(block_number: BlockNumberFor<T>, store: (&ProjectId, UpdateType)) -> Result<u32, u32> {
		// Try to get the project into the earliest possible block to update.
		// There is a limit for how many projects can update each block, so we need to make sure we don't exceed that limit
		let mut block_number = block_number;
		for i in 1..T::MaxProjectsToUpdateInsertionAttempts::get() + 1 {
			if ProjectsToUpdate::<T>::try_append(block_number, store.clone()).is_err() {
				block_number += 1u32.into();
			} else {
				return Ok(i);
			}
		}
		return Err(T::MaxProjectsToUpdateInsertionAttempts::get());
	}

	pub fn create_bucket_from_metadata(metadata: &ProjectMetadataOf<T>) -> Result<BucketOf<T>, DispatchError> {
		let auction_allocation_size = metadata.auction_round_allocation_percentage * metadata.total_allocation_size;
		let bucket_delta_amount = Percent::from_percent(10) * auction_allocation_size;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).ok_or(Error::<T>::BadMath)?;
		let bucket_delta_price: <T as Config>::Price = metadata.minimum_price.saturating_mul(ten_percent_in_price);

		let bucket: BucketOf<T> =
			Bucket::new(auction_allocation_size, metadata.minimum_price, bucket_delta_price, bucket_delta_amount);

		Ok(bucket)
	}

	pub fn calculate_plmc_bond(
		ticket_size: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		plmc_price: PriceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let usd_bond = multiplier.calculate_bonding_requirement::<T>(ticket_size).map_err(|_| Error::<T>::BadMath)?;
		plmc_price.reciprocal().ok_or(Error::<T>::BadMath)?.checked_mul_int(usd_bond).ok_or(Error::<T>::BadMath.into())
	}

	// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_info(
		_caller: &AccountIdOf<T>,
		multiplier: MultiplierOf<T>,
		bonded_amount: BalanceOf<T>,
	) -> Result<VestingInfo<BlockNumberFor<T>, BalanceOf<T>>, DispatchError> {
		// TODO: duration should depend on `_multiplier` and `_caller` credential
		let duration: BlockNumberFor<T> = multiplier.calculate_vesting_duration::<T>();
		let duration_as_balance = T::BlockNumberToBalance::convert(duration);
		let amount_per_block = if duration_as_balance == Zero::zero() {
			bonded_amount
		} else {
			bonded_amount.checked_div(&duration_as_balance).ok_or(Error::<T>::BadMath)?
		};

		Ok(VestingInfo { total_amount: bonded_amount, amount_per_block, duration })
	}

	/// Calculates the price (in USD) of contribution tokens for the Community and Remainder Rounds
	pub fn calculate_weighted_average_price(
		project_id: ProjectId,
		end_block: BlockNumberFor<T>,
		auction_allocation_size: BalanceOf<T>,
	) -> Result<(u32, u32), DispatchError> {
		// Get all the bids that were made before the end of the candle
		let mut bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = Zero::zero();
		// temp variable to store the total value of the bids (i.e price * amount = Cumulative Ticket Size)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();
		let project_account = Self::fund_account_id(project_id);
		let plmc_price = T::PriceProvider::get_price(PLMC_FOREIGN_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;

		// Weight calculation variables
		let mut accepted_bids_count = 0u32;
		let mut rejected_bids_count = 0u32;

		// sort bids by price, and equal prices sorted by id
		bids.sort_by(|a, b| b.cmp(a));
		// accept only bids that were made before `end_block` i.e end of candle auction
		let bids: Result<Vec<_>, DispatchError> = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					rejected_bids_count += 1;
					return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::AfterCandleEnd)
						.and(Ok(bid));
				}
				let buyable_amount = auction_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount.is_zero() {
					rejected_bids_count += 1;
					return Self::refund_bid(&mut bid, project_id, &project_account, RejectionReason::NoTokensLeft)
						.and(Ok(bid));
				} else if bid.original_ct_amount <= buyable_amount {
					accepted_bids_count += 1;
					let ticket_size = bid.original_ct_usd_price.saturating_mul_int(bid.original_ct_amount);
					bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
					bid_usd_value_sum.saturating_accrue(ticket_size);
					bid.status = BidStatus::Accepted;
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
				} else {
					accepted_bids_count += 1;
					let ticket_size = bid.original_ct_usd_price.saturating_mul_int(buyable_amount);
					bid_usd_value_sum.saturating_accrue(ticket_size);
					bid_token_amount_sum.saturating_accrue(buyable_amount);
					bid.status = BidStatus::PartiallyAccepted(buyable_amount, RejectionReason::NoTokensLeft);
					DidWithWinningBids::<T>::mutate(project_id, bid.did.clone(), |flag| {
						*flag = true;
					});
					bid.final_ct_amount = buyable_amount;

					let funding_asset_price = T::PriceProvider::get_price(bid.funding_asset.to_assethub_id())
						.ok_or(Error::<T>::PriceNotFound)?;
					let funding_asset_amount_needed = funding_asset_price
						.reciprocal()
						.ok_or(Error::<T>::BadMath)?
						.checked_mul_int(ticket_size)
						.ok_or(Error::<T>::BadMath)?;
					T::FundingCurrency::transfer(
						bid.funding_asset.to_assethub_id(),
						&project_account,
						&bid.bidder,
						bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed),
						Preservation::Preserve,
					)?;

					let usd_bond_needed = bid
						.multiplier
						.calculate_bonding_requirement::<T>(ticket_size)
						.map_err(|_| Error::<T>::BadMath)?;
					let plmc_bond_needed = plmc_price
						.reciprocal()
						.ok_or(Error::<T>::BadMath)?
						.checked_mul_int(usd_bond_needed)
						.ok_or(Error::<T>::BadMath)?;
					T::NativeCurrency::release(
						&HoldReason::Participation(project_id.into()).into(),
						&bid.bidder,
						bid.plmc_bond.saturating_sub(plmc_bond_needed),
						Precision::Exact,
					)?;

					bid.funding_asset_amount_locked = funding_asset_amount_needed;
					bid.plmc_bond = plmc_bond_needed;
				}

				Ok(bid)
			})
			.collect();
		let bids = bids?;
		// Calculate the weighted price of the token for the next funding rounds, using winning bids.
		// for example: if there are 3 winning bids,
		// A: 10K tokens @ USD15 per token = 150K USD value
		// B: 20K tokens @ USD20 per token = 400K USD value
		// C: 20K tokens @ USD10 per token = 200K USD value,

		// then the weight for each bid is:
		// A: 150K / (150K + 400K + 200K) = 0.20
		// B: 400K / (150K + 400K + 200K) = 0.533...
		// C: 200K / (150K + 400K + 200K) = 0.266...

		// then multiply each weight by the price of the token to get the weighted price per bid
		// A: 0.20 * 15 = 3
		// B: 0.533... * 20 = 10.666...
		// C: 0.266... * 10 = 2.666...

		// lastly, sum all the weighted prices to get the final weighted price for the next funding round
		// 3 + 10.6 + 2.6 = 16.333...
		let current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let is_first_bucket = current_bucket.current_price == project_metadata.minimum_price;

		let calc_weighted_price_fn = |bid: &BidInfoOf<T>, amount: BalanceOf<T>| -> Option<PriceOf<T>> {
			let ticket_size = bid.original_ct_usd_price.saturating_mul_int(amount);
			let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(ticket_size, bid_usd_value_sum);
			let weighted_price = bid.original_ct_usd_price.saturating_mul(bid_weight);
			Some(weighted_price)
		};
		let weighted_token_price = match is_first_bucket && !bids.is_empty() {
			true => project_metadata.minimum_price,
			false => bids
				.iter()
				.filter_map(|bid| match bid.status {
					BidStatus::Accepted => calc_weighted_price_fn(bid, bid.original_ct_amount),
					BidStatus::PartiallyAccepted(amount, _) => calc_weighted_price_fn(bid, amount),
					_ => None,
				})
				.reduce(|a, b| a.saturating_add(b))
				.ok_or(Error::<T>::NoBidsFound)?,
		};

		let mut final_total_funding_reached_by_bids = BalanceOf::<T>::zero();
		// Update the bid in the storage
		for mut bid in bids.into_iter() {
			if bid.final_ct_usd_price > weighted_token_price {
				bid.final_ct_usd_price = weighted_token_price;
				let new_ticket_size =
					weighted_token_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;

				let funding_asset_price =
					T::PriceProvider::get_price(bid.funding_asset.to_assethub_id()).ok_or(Error::<T>::PriceNotFound)?;
				let funding_asset_amount_needed = funding_asset_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(new_ticket_size)
					.ok_or(Error::<T>::BadMath)?;

				T::FundingCurrency::transfer(
					bid.funding_asset.to_assethub_id(),
					&project_account,
					&bid.bidder,
					bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed),
					Preservation::Preserve,
				)?;

				bid.funding_asset_amount_locked = funding_asset_amount_needed;

				let usd_bond_needed = bid
					.multiplier
					.calculate_bonding_requirement::<T>(new_ticket_size)
					.map_err(|_| Error::<T>::BadMath)?;
				let plmc_bond_needed = plmc_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(usd_bond_needed)
					.ok_or(Error::<T>::BadMath)?;

				T::NativeCurrency::release(
					&HoldReason::Participation(project_id).into(),
					&bid.bidder,
					bid.plmc_bond.saturating_sub(plmc_bond_needed),
					Precision::Exact,
				)?;

				bid.plmc_bond = plmc_bond_needed;
			}
			let final_ticket_size =
				bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;
			final_total_funding_reached_by_bids.saturating_accrue(final_ticket_size);
			Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
		}

		// Update storage
		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> DispatchResult {
			if let Some(info) = maybe_info {
				info.weighted_average_price = Some(weighted_token_price);
				info.remaining_contribution_tokens.saturating_reduce(bid_token_amount_sum);
				info.funding_amount_reached.saturating_accrue(final_total_funding_reached_by_bids);
				Ok(())
			} else {
				Err(Error::<T>::ProjectNotFound.into())
			}
		})?;

		Ok((accepted_bids_count, rejected_bids_count))
	}

	/// Refund a bid because of `reason`.
	fn refund_bid<'a>(
		bid: &'a mut BidInfoOf<T>,
		project_id: ProjectId,
		project_account: &'a AccountIdOf<T>,
		reason: RejectionReason,
	) -> Result<(), DispatchError> {
		bid.status = BidStatus::Rejected(reason);
		bid.final_ct_amount = Zero::zero();
		bid.final_ct_usd_price = Zero::zero();

		T::FundingCurrency::transfer(
			bid.funding_asset.to_assethub_id(),
			project_account,
			&bid.bidder,
			bid.funding_asset_amount_locked,
			Preservation::Preserve,
		)?;
		T::NativeCurrency::release(
			&HoldReason::Participation(project_id).into(),
			&bid.bidder,
			bid.plmc_bond,
			Precision::Exact,
		)?;
		bid.funding_asset_amount_locked = Zero::zero();
		bid.plmc_bond = Zero::zero();

		Ok(())
	}

	pub fn select_random_block(
		candle_starting_block: BlockNumberFor<T>,
		candle_ending_block: BlockNumberFor<T>,
	) -> BlockNumberFor<T> {
		let nonce = Self::get_and_increment_nonce();
		let (random_value, _known_since) = T::Randomness::random(&nonce);
		let random_block = <BlockNumberFor<T>>::decode(&mut random_value.as_ref())
			.expect("secure hashes should always be bigger than the block number; qed");
		let block_range = candle_ending_block - candle_starting_block;

		candle_starting_block + (random_block % block_range)
	}

	fn get_and_increment_nonce() -> Vec<u8> {
		let nonce = Nonce::<T>::get();
		Nonce::<T>::put(nonce.wrapping_add(1));
		nonce.encode()
	}

	/// People that contributed to the project during the Funding Round can claim their Contribution Tokens
	// This function is kept separate from the `do_claim_contribution_tokens` for easier testing the logic
	#[inline(always)]
	pub fn calculate_claimable_tokens(
		contribution_amount: BalanceOf<T>,
		weighted_average_price: BalanceOf<T>,
	) -> FixedU128 {
		FixedU128::saturating_from_rational(contribution_amount, weighted_average_price)
	}

	pub fn try_plmc_participation_lock(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		// Check if the user has already locked tokens in the evaluation period
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, who));

		let mut to_convert = amount;
		for mut evaluation in user_evaluations {
			if to_convert == Zero::zero() {
				break;
			}
			let slash_deposit = <T as Config>::EvaluatorSlash::get() * evaluation.original_plmc_bond;
			let available_to_convert = evaluation.current_plmc_bond.saturating_sub(slash_deposit);
			let converted = to_convert.min(available_to_convert);
			evaluation.current_plmc_bond = evaluation.current_plmc_bond.saturating_sub(converted);
			Evaluations::<T>::insert((project_id, who, evaluation.id), evaluation);
			T::NativeCurrency::release(&HoldReason::Evaluation(project_id).into(), who, converted, Precision::Exact)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&HoldReason::Participation(project_id).into(), who, converted)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&HoldReason::Participation(project_id).into(), who, to_convert)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: BalanceOf<T>,
		asset_id: AssetIdOf<T>,
	) -> DispatchResult {
		let fund_account = Self::fund_account_id(project_id);
		// Why `Preservation::Expendable`?
		// the min_balance of funding assets (e.g USDT) are low enough so we don't expect users to care about their balance being dusted.
		// We do think the UX would be bad if they cannot use all of their available tokens.
		// Specially since a new funding asset account can be easily created by increasing the provider reference
		T::FundingCurrency::transfer(asset_id, who, &fund_account, amount, Preservation::Expendable)
			.map_err(|_| Error::<T>::NotEnoughFunds)?;

		Ok(())
	}

	/// Calculate the total fees based on the funding reached.
	pub fn calculate_fees(funding_reached: BalanceOf<T>) -> Perquintill {
		let total_fee = Self::compute_total_fee_from_brackets(funding_reached);
		Perquintill::from_rational(total_fee, funding_reached)
	}

	/// Computes the total fee from all defined fee brackets.
	fn compute_total_fee_from_brackets(funding_reached: BalanceOf<T>) -> BalanceOf<T> {
		let mut remaining_for_fee = funding_reached;

		T::FeeBrackets::get()
			.into_iter()
			.map(|(fee, limit)| Self::compute_fee_for_bracket(&mut remaining_for_fee, fee, limit))
			.fold(BalanceOf::<T>::zero(), |acc, fee| acc.saturating_add(fee))
	}

	/// Calculate the fee for a particular bracket.
	fn compute_fee_for_bracket(
		remaining_for_fee: &mut BalanceOf<T>,
		fee: Percent,
		limit: BalanceOf<T>,
	) -> BalanceOf<T> {
		if let Some(amount_to_bid) = remaining_for_fee.checked_sub(&limit) {
			*remaining_for_fee = amount_to_bid;
			fee * limit
		} else {
			let fee_for_this_bracket = fee * *remaining_for_fee;
			*remaining_for_fee = BalanceOf::<T>::zero();
			fee_for_this_bracket
		}
	}

	/// Generate and return evaluator rewards based on a project's funding status.
	///
	/// The function calculates rewards based on several metrics: funding achieved,
	/// total allocations, and issuer fees. It also differentiates between early and
	/// normal evaluators for reward distribution.
	///
	/// Note: Consider refactoring the `RewardInfo` struct to make it more generic and
	/// reusable, not just for evaluator rewards.
	pub fn generate_evaluator_rewards_info(project_id: ProjectId) -> Result<(RewardInfoOf<T>, u32), DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let evaluations = Evaluations::<T>::iter_prefix((project_id,)).collect::<Vec<_>>();
		// used for weight calculation
		let evaluations_count = evaluations.len() as u32;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached;
		let fundraising_target = project_details.fundraising_target;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		let initial_token_allocation_size = project_metadata.total_allocation_size;
		let final_remaining_contribution_tokens = project_details.remaining_contribution_tokens;

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);

		// Calculate rewards.
		let evaluator_rewards = percentage_of_target_funding * Perquintill::from_percent(30) * total_fee_allocation;

		// Distribute rewards between early and normal evaluators.
		let early_evaluator_reward_pot = Perquintill::from_percent(20) * evaluator_rewards;
		let normal_evaluator_reward_pot = Perquintill::from_percent(80) * evaluator_rewards;

		// Sum up the total bonded USD amounts for both early and late evaluators.
		let early_evaluator_total_bonded_usd =
			evaluations.iter().fold(BalanceOf::<T>::zero(), |acc, ((_evaluator, _id), evaluation)| {
				acc.saturating_add(evaluation.early_usd_amount)
			});
		let late_evaluator_total_bonded_usd =
			evaluations.iter().fold(BalanceOf::<T>::zero(), |acc, ((_evaluator, _id), evaluation)| {
				acc.saturating_add(evaluation.late_usd_amount)
			});

		let normal_evaluator_total_bonded_usd =
			early_evaluator_total_bonded_usd.saturating_add(late_evaluator_total_bonded_usd);

		// Construct the reward information object.
		let reward_info = RewardInfo {
			early_evaluator_reward_pot,
			normal_evaluator_reward_pot,
			early_evaluator_total_bonded_usd,
			normal_evaluator_total_bonded_usd,
		};

		Ok((reward_info, evaluations_count))
	}

	pub fn generate_liquidity_pools_and_long_term_holder_rewards(
		project_id: ProjectId,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached;
		let fundraising_target = project_details.fundraising_target;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		let initial_token_allocation_size = project_metadata.total_allocation_size;
		let final_remaining_contribution_tokens = project_details.remaining_contribution_tokens;

		// Calculate the number of tokens sold for the project.
		let token_sold = initial_token_allocation_size
			.checked_sub(&final_remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(initial_token_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		// A.K.A variable "Y" in the documentation.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);
		let inverse_percentage_of_target_funding = Perquintill::from_percent(100) - percentage_of_target_funding;

		let liquidity_pools_percentage = Perquintill::from_percent(50);
		let liquidity_pools_reward_pot = liquidity_pools_percentage * total_fee_allocation;

		let long_term_holder_percentage = if percentage_of_target_funding < Perquintill::from_percent(90) {
			Perquintill::from_percent(50)
		} else {
			Perquintill::from_percent(20) + Perquintill::from_percent(30) * inverse_percentage_of_target_funding
		};
		let long_term_holder_reward_pot = long_term_holder_percentage * total_fee_allocation;

		Ok((liquidity_pools_reward_pot, long_term_holder_reward_pot))
	}

	pub fn make_project_funding_successful(
		project_id: ProjectId,
		mut project_details: ProjectDetailsOf<T>,
		reason: SuccessReason,
		settlement_delta: BlockNumberFor<T>,
	) -> Result<u32, DispatchError> {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingSuccessful;
		ProjectsDetails::<T>::insert(project_id, project_details);

		let insertion_iterations =
			match Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement)) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};

		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Success(reason) });

		Ok(insertion_iterations)
	}

	pub fn make_project_funding_fail(
		project_id: ProjectId,
		mut project_details: ProjectDetailsOf<T>,
		reason: FailureReason,
		settlement_delta: BlockNumberFor<T>,
	) -> Result<u32, DispatchError> {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingFailed;
		ProjectsDetails::<T>::insert(project_id, project_details);

		let insertion_iterations =
			match Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement)) {
				Ok(iterations) => iterations,
				Err(_iterations) => return Err(Error::<T>::TooManyInsertionAttempts.into()),
			};
		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Failure(reason) });
		Ok(insertion_iterations)
	}

	pub fn migrations_per_xcm_message_allowed() -> u32 {
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		let one_migration_bytes = (0u128, 0u64).encode().len() as u32;

		// our encoded call starts with pallet index 51, and call index 0
		let mut encoded_call = vec![51u8, 0];
		let encoded_first_param = [0u8; 32].encode();
		let encoded_second_param = Vec::<MigrationInfo>::new().encode();
		// we append the encoded parameters, with our migrations vec being empty for now
		encoded_call.extend_from_slice(encoded_first_param.as_slice());
		encoded_call.extend_from_slice(encoded_second_param.as_slice());

		let base_xcm_message: Xcm<()> = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Native, require_weight_at_most: MAX_WEIGHT, call: encoded_call.into() },
			ReportTransactStatus(QueryResponseInfo {
				destination: Parachain(3344).into(),
				query_id: 0,
				max_weight: MAX_WEIGHT,
			}),
		]);
		let xcm_size = base_xcm_message.encode().len();

		let available_bytes_for_migration_per_message =
			T::RequiredMaxMessageSize::get().saturating_sub(xcm_size as u32);

		available_bytes_for_migration_per_message.saturating_div(one_migration_bytes)
	}

	pub fn construct_migration_xcm_messages(migrations: Migrations) -> Vec<(Migrations, Xcm<()>)> {
		// TODO: adjust this as benchmarks for polimec-receiver are written
		const MAX_WEIGHT: Weight = Weight::from_parts(10_000, 0);

		// const MAX_WEIGHT: Weight = Weight::from_parts(100_003_000_000_000, 10_000_196_608);
		let _polimec_receiver_info = T::PolimecReceiverInfo::get();
		// TODO: use the actual pallet index when the fields are not private anymore (https://github.com/paritytech/polkadot-sdk/pull/2231)
		let mut output = Vec::new();

		for migrations_slice in migrations.inner().chunks(MaxMigrationsPerXcm::<T>::get() as usize) {
			let migrations_vec = migrations_slice.to_vec();
			let migrations_item = Migrations::from(migrations_vec);

			let mut encoded_call = vec![51u8, 0];
			encoded_call.extend_from_slice(migrations_item.encode().as_slice());
			let xcm: Xcm<()> = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Native,
					require_weight_at_most: MAX_WEIGHT,
					call: encoded_call.into(),
				},
				// ReportTransactStatus should be appended here after knowing the query_id
			]);

			output.push((migrations_item, xcm));
		}

		// TODO: we probably want to ensure we dont build too many messages to overflow the queue. Which we know from the parameter `T::RequiredMaxCapacity`.
		// the problem is that we don't know the existing messages in the destination queue. So for now we assume all messages will succeed
		output
	}

	pub fn mark_migrations_as_sent(project_migration_origins: ProjectMigrationOriginsOf<T>, query_id: QueryId) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Sent(query_id);
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Sent(query_id);
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Sent(query_id);
							}
						},
					);
				},
			}
		}
	}

	pub fn mark_migrations_as_confirmed(project_migration_origins: ProjectMigrationOriginsOf<T>) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Confirmed;
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Confirmed;
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Confirmed;
							}
						},
					);
				},
			}
		}
	}

	pub fn mark_migrations_as_failed(
		project_migration_origins: ProjectMigrationOriginsOf<T>,
		error: BoundedVec<u8, MaxDispatchErrorLen>,
	) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Failed(error.clone());
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Failed(error.clone());
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Failed(error.clone());
							}
						},
					);
				},
			}
		}
	}
}
