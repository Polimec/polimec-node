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

use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::MutateHold as FungibleMutateHold,
		fungibles::{metadata::Mutate as MetadataMutate, Create, Inspect, Mutate as FungiblesMutate},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
};
use sp_arithmetic::{
	traits::{CheckedDiv, CheckedSub, Zero},
	Percent, Perquintill,
};
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;

use polimec_traits::ReleaseSchedule;

use crate::traits::{BondingRequirementCalculation, ProvideStatemintPrice, VestingDurationCalculation};

use super::*;

// Round transition functions
impl<T: Config> Pallet<T> {
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
	/// * [`ProjectsIssuers`] - Inserting the issuer of the project. Mapping of the two parameters `project_id` and `issuer`.
	/// * [`NextProjectId`] - Getting the next usable id, and updating it for the next project.
	///
	/// # Success path
	/// The `project` argument is valid. A ProjectInfo struct is constructed, and the storage is updated
	/// with the new structs and mappings to reflect the new project creation
	///
	/// # Next step
	/// The issuer will call an extrinsic to start the evaluation round of the project.
	/// [`do_evaluation_start`](Self::do_evaluation_start) will be executed.
	pub fn do_create(issuer: &AccountIdOf<T>, initial_metadata: ProjectMetadataOf<T>) -> Result<(), DispatchError> {
		// * Get variables *
		let project_id = Self::next_project_id();

		// * Validity checks *
		if let Some(metadata) = initial_metadata.offchain_information_hash {
			ensure!(!Images::<T>::contains_key(metadata), Error::<T>::MetadataAlreadyExists);
		}

		if let Err(error) = initial_metadata.validity_check() {
			return match error {
				ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
				ValidityError::ParticipantsSizeError => Err(Error::<T>::ParticipantsSizeError.into()),
				ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
			}
		}

		// * Calculate new variables *
		let fundraising_target = initial_metadata
			.minimum_price
			.checked_mul_int(initial_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let bucket_delta_amount = Percent::from_percent(10) * initial_metadata.total_allocation_size;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).ok_or(Error::<T>::BadMath)?;
		let bucket_delta_price: <T as Config>::Price =
			initial_metadata.minimum_price.saturating_mul(ten_percent_in_price);
		let now = <frame_system::Pallet<T>>::block_number();
		let project_details = ProjectDetails {
			issuer: issuer.clone(),
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints::new(now),
			remaining_contribution_tokens: initial_metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<T>::zero(),
			cleanup: Cleaner::NotReady,
			evaluation_round_info: EvaluationRoundInfoOf::<T> {
				total_bonded_usd: Zero::zero(),
				total_bonded_plmc: Zero::zero(),
				evaluators_outcome: EvaluatorsOutcome::Unchanged,
			},
			funding_end_block: None,
		};
		let bucket: BucketOf<T> = Bucket::new(
			initial_metadata.total_allocation_size,
			initial_metadata.minimum_price,
			bucket_delta_price,
			bucket_delta_amount,
		);

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, &initial_metadata);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Buckets::<T>::insert(project_id, bucket);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());
		if let Some(metadata) = initial_metadata.offchain_information_hash {
			Images::<T>::insert(metadata, issuer);
		}

		// * Emit events *
		Self::deposit_event(Event::Created { project_id });

		Ok(())
	}

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
	pub fn do_evaluation_start(caller: AccountIdOf<T>, project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer == caller, Error::<T>::NotAllowed);
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
		Self::add_to_update_store(evaluation_end_block + 1u32.into(), (&project_id, UpdateType::EvaluationEnd));

		// * Emit events *
		Self::deposit_event(Event::EvaluationStarted { project_id });

		Ok(())
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
	pub fn do_evaluation_end(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_end_block =
			project_details.phase_transition_points.evaluation.end().ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target_usd = project_details.fundraising_target;
		let current_plmc_price =
			T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::ProjectNotInEvaluationRound);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// * Calculate new variables *
		let initial_balance: BalanceOf<T> = 0u32.into();
		let total_amount_bonded = Evaluations::<T>::iter_prefix((project_id,))
			.fold(initial_balance, |total, (_evaluator, bond)| total.saturating_add(bond.original_plmc_bond));

		let evaluation_target_usd = <T as Config>::EvaluationSuccessThreshold::get() * fundraising_target_usd;
		let evaluation_target_plmc = current_plmc_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(evaluation_target_usd)
			.ok_or(Error::<T>::BadMath)?;

		let auction_initialize_period_start_block = now + 1u32.into();
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block + T::AuctionInitializePeriodDuration::get();

		// Check which logic path to follow
		let is_funded = total_amount_bonded >= evaluation_target_plmc;

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
			Self::add_to_update_store(
				auction_initialize_period_end_block + 1u32.into(),
				(&project_id, UpdateType::EnglishAuctionStart),
			);

			// * Emit events *
			Self::deposit_event(Event::AuctionInitializePeriod {
				project_id,
				start_block: auction_initialize_period_start_block,
				end_block: auction_initialize_period_end_block,
			});

		// Unsuccessful path
		} else {
			// * Update storage *
			project_details.status = ProjectStatus::EvaluationFailed;
			project_details.cleanup = Cleaner::Failure(CleanerState::Initialized(PhantomData::<Failure>));
			ProjectsDetails::<T>::insert(project_id, project_details);

			// * Emit events *
			Self::deposit_event(Event::EvaluationFailed { project_id });
		}

		Ok(())
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
	pub fn do_english_auction(caller: AccountIdOf<T>, project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_initialize_period_start_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.start()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;
		let auction_initialize_period_end_block = project_details
			.phase_transition_points
			.auction_initialize_period
			.end()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;

		// * Validity checks *
		ensure!(
			caller == project_details.issuer || caller == T::PalletId::get().into_account_truncating(),
			Error::<T>::NotAllowed
		);
		ensure!(now >= auction_initialize_period_start_block, Error::<T>::TooEarlyForEnglishAuctionStart);
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

		// If this function was called inside the period, then it was called by the extrinsic and we need to
		// remove the scheduled automatic transition
		if now <= auction_initialize_period_end_block {
			Self::remove_from_update_store(&project_id)?;
		}
		// Schedule for automatic transition to candle auction round
		Self::add_to_update_store(english_end_block + 1u32.into(), (&project_id, UpdateType::CandleAuctionStart));

		// * Emit events *
		Self::deposit_event(Event::EnglishAuctionStarted { project_id, when: now });

		Ok(())
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
	pub fn do_candle_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
		Self::add_to_update_store(candle_end_block + 1u32.into(), (&project_id, UpdateType::CommunityFundingStart));

		// * Emit events *
		Self::deposit_event(Event::CandleAuctionStarted { project_id, when: now });

		Ok(())
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
	pub fn do_community_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
		let calculation_result =
			Self::calculate_weighted_average_price(project_id, end_block, project_metadata.total_allocation_size);
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		match calculation_result {
			Err(pallet_error) if pallet_error == Error::<T>::NoBidsFound.into() => {
				project_details.status = ProjectStatus::FundingFailed;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Self::add_to_update_store(
					<frame_system::Pallet<T>>::block_number() + 1u32.into(),
					(&project_id, UpdateType::FundingEnd),
				);

				// * Emit events *
				Self::deposit_event(Event::AuctionFailed { project_id });

				Ok(())
			},
			e @ Err(_) => e,
			Ok(()) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_candle_ending = Some(end_block);
				project_details
					.phase_transition_points
					.community
					.update(Some(community_start_block), Some(community_end_block));
				project_details.status = ProjectStatus::CommunityRound;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Self::add_to_update_store(
					community_end_block + 1u32.into(),
					(&project_id, UpdateType::RemainderFundingStart),
				);

				// * Emit events *
				Self::deposit_event(Event::CommunityFundingStarted { project_id });

				Ok(())
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
	pub fn do_remainder_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block =
			project_details.phase_transition_points.community.end().ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(project_details.status == ProjectStatus::CommunityRound, Error::<T>::ProjectNotInCommunityRound);

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
		Self::add_to_update_store(remainder_end_block + 1u32.into(), (&project_id, UpdateType::FundingEnd));

		// * Emit events *
		Self::deposit_event(Event::RemainderFundingStarted { project_id });

		Ok(())
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
	pub fn do_end_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let remainder_end_block = project_details.phase_transition_points.remainder.end();
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(
			remaining_cts == 0u32.into() ||
				project_details.status == ProjectStatus::FundingFailed ||
				matches!(remainder_end_block, Some(end_block) if now > end_block),
			Error::<T>::TooEarlyForFundingEnd
		);

		// * Calculate new variables *
		let funding_target = project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let funding_reached = project_details.funding_amount_reached;
		let funding_ratio = Perquintill::from_rational(funding_reached, funding_target);

		// * Update Storage *
		if funding_ratio <= Perquintill::from_percent(33u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			Self::make_project_funding_fail(project_id, project_details, FailureReason::TargetNotReached, 1u32.into())
		} else if funding_ratio <= Perquintill::from_percent(75u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Slashed;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			Self::add_to_update_store(
				now + T::ManualAcceptanceDuration::get() + 1u32.into(),
				(&project_id, UpdateType::ProjectDecision(FundingOutcomeDecision::AcceptFunding)),
			);
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(())
		} else if funding_ratio < Perquintill::from_percent(90u64) {
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Unchanged;
			project_details.status = ProjectStatus::AwaitingProjectDecision;
			ProjectsDetails::<T>::insert(project_id, project_details);
			Ok(())
		} else {
			let reward_info = Self::generate_evaluator_rewards_info(project_id)?;
			project_details.evaluation_round_info.evaluators_outcome = EvaluatorsOutcome::Rewarded(reward_info);
			Self::make_project_funding_successful(
				project_id,
				project_details,
				SuccessReason::ReachedTarget,
				T::SuccessToSettlementTime::get(),
			)
		}
	}

	pub fn do_project_decision(project_id: T::ProjectIdentifier, decision: FundingOutcomeDecision) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Update storage *
		match decision {
			FundingOutcomeDecision::AcceptFunding => {
				Self::make_project_funding_successful(
					project_id,
					project_details,
					SuccessReason::ProjectDecision,
					T::SuccessToSettlementTime::get(),
				)?;
			},
			FundingOutcomeDecision::RejectFunding => {
				Self::make_project_funding_fail(
					project_id,
					project_details,
					FailureReason::ProjectDecision,
					T::SuccessToSettlementTime::get(),
				)?;
			},
		}

		Ok(())
	}

	pub fn do_start_settlement(project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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

		if project_details.status == ProjectStatus::FundingSuccessful {
			T::ContributionTokenCurrency::create(project_id, project_details.issuer.clone(), false, 1_u32.into())?;
			T::ContributionTokenCurrency::set(
				project_id,
				&project_details.issuer,
				token_information.name.into(),
				token_information.symbol.into(),
				token_information.decimals,
			)?;
		}

		Ok(())
	}

	/// Called manually by a user extrinsic
	/// Marks the project as ready to launch on mainnet, which will in the future start the logic
	/// to burn the contribution tokens and mint the real tokens the project's chain
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Check that the funding round ended, and update the status to ReadyToLaunch
	///
	/// # Success Path
	/// For now it will always succeed as long as the project exists. This functions is a WIP.
	///
	///
	/// # Next step
	/// WIP
	pub fn do_ready_to_launch(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::ProjectNotInFundingEndedRound);

		// Update project Info
		project_details.status = ProjectStatus::ReadyToLaunch;
		ProjectsDetails::<T>::insert(project_id, project_details);

		Ok(())
	}
}

// Extrinsic functions (except round transitions)
impl<T: Config> Pallet<T> {
	/// Change the metadata hash of a project
	///
	/// # Arguments
	/// * `issuer` - The project issuer account
	/// * `project_id` - The project identifier
	/// * `project_metadata_hash` - The hash of the image that contains the metadata
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the issuer is the owner of the project
	/// * [`Images`] - Check that the image exists
	/// * [`ProjectsDetails`] - Check that the project is not frozen
	/// * [`ProjectsMetadata`] - Update the metadata hash
	pub fn do_edit_metadata(
		issuer: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		project_metadata_hash: T::Hash,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer == issuer, Error::<T>::NotAllowed);
		ensure!(!project_details.is_frozen, Error::<T>::Frozen);
		ensure!(!Images::<T>::contains_key(project_metadata_hash), Error::<T>::MetadataAlreadyExists);

		// * Calculate new variables *

		// * Update Storage *
		project_metadata.offchain_information_hash = Some(project_metadata_hash);
		ProjectsMetadata::<T>::insert(project_id, project_metadata);

		// * Emit events *
		Self::deposit_event(Event::MetadataEdited { project_id });

		Ok(())
	}

	// Note: usd_amount needs to have the same amount of decimals as PLMC,, so when multiplied by the plmc-usd price, it gives us the PLMC amount with the decimals we wanted.
	pub fn do_evaluate(
		evaluator: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		usd_amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = Self::next_evaluation_id();
		let caller_existing_evaluations: Vec<(u32, EvaluationInfoOf<T>)> =
			Evaluations::<T>::iter_prefix((project_id, evaluator)).collect();
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target;
		let evaluation_round_info = &mut project_details.evaluation_round_info;

		// * Validity Checks *
		ensure!(evaluator.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(project_details.status == ProjectStatus::EvaluationRound, Error::<T>::EvaluationNotStarted);

		// * Calculate new variables *
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
			rewarded_or_slashed: false,
		};

		// * Update Storage *
		if caller_existing_evaluations.len() < T::MaxEvaluationsPerUser::get() as usize {
			T::NativeCurrency::hold(&LockType::Evaluation(project_id), evaluator, plmc_bond)?;
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
				&LockType::Evaluation(project_id),
				&lowest_evaluation.evaluator,
				lowest_evaluation.original_plmc_bond,
				Precision::Exact,
			)?;

			T::NativeCurrency::hold(&LockType::Evaluation(project_id), evaluator, plmc_bond)?;

			Evaluations::<T>::remove((project_id, evaluator, low_id));
		}

		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), new_evaluation);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));
		evaluation_round_info.total_bonded_usd += usd_amount;
		evaluation_round_info.total_bonded_plmc += plmc_bond;
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::FundsBonded { project_id, amount: plmc_bond, bonder: evaluator.clone() });

		Ok(())
	}

	/// Bid for a project in the bidding stage
	///
	/// # Arguments
	/// * `bidder` - The account that is bidding
	/// * `project_id` - The project to bid for
	/// * `amount` - The amount of tokens that the bidder wants to buy
	/// * `price` - The price in USD per token that the bidder is willing to pay for
	/// * `multiplier` - Used for calculating how much PLMC needs to be bonded to spend this much money (in USD)
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the bidder is not the project issuer
	/// * [`ProjectsDetails`] - Check that the project is in the bidding stage
	/// * [`BiddingBonds`] - Update the storage with the bidder's PLMC bond for that bid
	/// * [`Bids`] - Check previous bids by that user, and update the storage with the new bid
	pub fn do_bid(
		bidder: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		ct_amount: BalanceOf<T>,
		_ct_usd_price: T::Price,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;

		// * Validity checks *
		ensure!(bidder.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(matches!(project_details.status, ProjectStatus::AuctionRound(_)), Error::<T>::AuctionNotStarted);
		ensure!(funding_asset == project_metadata.participation_currencies, Error::<T>::FundingAssetNotAccepted);

		// Fetch current bucket details and other required info
		let mut current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		if current_bucket.amount_left > ct_amount {
			// There are enough tokens left to bid
			let bid_id = Self::next_bid_id();
			Self::perform_do_bid(
				bidder,
				project_id,
				ct_amount,
				current_bucket.current_price,
				multiplier,
				funding_asset,
				project_metadata.ticket_size,
				bid_id,
				now,
			)?;
			// Update the tokens left in the bucket
			Buckets::<T>::mutate(project_id, |maybe_bucket| {
				if let Some(bucket) = maybe_bucket {
					bucket.amount_left.saturating_reduce(ct_amount);
					Ok(())
				} else {
					Err(Error::<T>::ProjectNotFound)
				}
			})?;
		} else {
			// Tokens in current bucket are not enough, multiple bids may be needed
			let bid_id = Self::next_bid_id();
			let bid = Self::perform_do_bid(
				bidder,
				project_id,
				current_bucket.amount_left,
				current_bucket.current_price,
				multiplier,
				funding_asset,
				project_metadata.ticket_size,
				bid_id,
				now,
			)?;

			// Move to the next bucket
			Buckets::<T>::mutate(project_id, |maybe_bucket| {
				if let Some(bucket) = maybe_bucket {
					bucket.next();
					Ok(())
				} else {
					Err(Error::<T>::ProjectNotFound)
				}
			})?;

			let mut remaining_amount = ct_amount.saturating_sub(bid.original_ct_amount);
			current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;

			// While there's still a remaining amount to bid for
			while !remaining_amount.is_zero() {
				let bid_id = Self::next_bid_id();
				let bid = Self::perform_do_bid(
					bidder,
					project_id,
					remaining_amount,
					current_bucket.current_price,
					multiplier,
					funding_asset,
					project_metadata.ticket_size,
					bid_id,
					now,
				)?;

				remaining_amount = remaining_amount.saturating_sub(bid.original_ct_amount);

				// If the remaining amount exceeds what's left in the current bucket, move to the next bucket
				if remaining_amount > current_bucket.amount_left {
					Buckets::<T>::mutate(project_id, |maybe_bucket| {
						if let Some(bucket) = maybe_bucket {
							bucket.next();
							Ok(())
						} else {
							Err(Error::<T>::ProjectNotFound)
						}
					})?;
					current_bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
				}
			}
		}
		Ok(())
	}

	fn perform_do_bid(
		bidder: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		ct_amount: BalanceOf<T>,
		ct_usd_price: T::Price,
		multiplier: MultiplierOf<T>,
		funding_asset: AcceptedFundingAsset,
		project_ticket_size: TicketSize<BalanceOf<T>>,
		bid_id: u32,
		now: BlockNumberOf<T>,
	) -> Result<BidInfoOf<T>, DispatchError> {
		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(funding_asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PriceNotFound)?;
		let existing_bids = Bids::<T>::iter_prefix_values((project_id, bidder)).collect::<Vec<_>>();

		if let Some(minimum_ticket_size) = project_ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(ticket_size >= minimum_ticket_size, Error::<T>::BidTooLow);
		};
		if let Some(maximum_ticket_size) = project_ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(ticket_size <= maximum_ticket_size, Error::<T>::BidTooLow);
		};
		// * Calculate new variables *
		let plmc_bond =
			Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price).map_err(|_| Error::<T>::BadMath)?;

		let funding_asset_amount_locked =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = funding_asset.to_statemint_id();

		let new_bid = BidInfoOf::<T> {
			id: bid_id,
			project_id,
			bidder: bidder.clone(),
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
			funded: false,
			when: now,
			funds_released: false,
			ct_minted: false,
		};

		// * Update storage *
		if existing_bids.len() >= T::MaxBidsPerUser::get() as usize {
			let lowest_bid =
				existing_bids.iter().min_by_key(|bid| &bid.plmc_bond).ok_or(Error::<T>::ImpossibleState)?;

			ensure!(new_bid.plmc_bond > lowest_bid.plmc_bond, Error::<T>::BidTooLow);

			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&lowest_bid.bidder,
				lowest_bid.plmc_bond,
				Precision::Exact,
			)?;
			T::FundingCurrency::transfer(
				asset_id,
				&Self::fund_account_id(project_id),
				&lowest_bid.bidder,
				lowest_bid.funding_asset_amount_locked,
				Preservation::Expendable,
			)?;
			Bids::<T>::remove((project_id, &lowest_bid.bidder, lowest_bid.id));
		}

		Self::try_plmc_participation_lock(bidder, project_id, plmc_bond)?;
		Self::try_funding_asset_hold(bidder, project_id, funding_asset_amount_locked, asset_id)?;

		Bids::<T>::insert((project_id, bidder, bid_id), &new_bid);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));

		Self::deposit_event(Event::Bid { project_id, amount: ct_amount, price: ct_usd_price, multiplier });

		Ok(new_bid)
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * token_amount: The amount of contribution tokens to buy
	/// * multiplier: Decides how much PLMC bonding is required for buying that amount of tokens
	///
	/// # Storage access
	/// * [`ProjectsIssuers`] - Check that the issuer is not a contributor
	/// * [`ProjectsDetails`] - Check that the project is in the Community Round, and the amount is big
	/// enough to buy at least 1 token
	/// * [`Contributions`] - Update storage with the new contribution
	/// * [`T::NativeCurrency`] - Update the balance of the contributor and the project pot
	pub fn do_contribute(
		contributor: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		token_amount: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		asset: AcceptedFundingAsset,
	) -> DispatchResultWithPostInfo {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let contribution_id = Self::next_contribution_id();
		let existing_contributions =
			Contributions::<T>::iter_prefix_values((project_id, contributor)).collect::<Vec<_>>();

		let ct_usd_price = project_details.weighted_average_price.ok_or(Error::<T>::AuctionNotStarted)?;
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PriceNotFound)?;
		let mut ticket_size = ct_usd_price.checked_mul_int(token_amount).ok_or(Error::<T>::BadMath)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;

		// * Validity checks *
		ensure!(contributor.clone() != project_details.issuer, Error::<T>::ContributionToThemselves);
		ensure!(
			project_details.status == ProjectStatus::CommunityRound ||
				project_details.status == ProjectStatus::RemainderRound,
			Error::<T>::AuctionNotStarted
		);

		if let Some(minimum_ticket_size) = project_metadata.ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(ticket_size >= minimum_ticket_size, Error::<T>::ContributionTooLow);
		};
		if let Some(maximum_ticket_size) = project_metadata.ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(ticket_size <= maximum_ticket_size, Error::<T>::ContributionTooHigh);
		};
		ensure!(project_metadata.participation_currencies == asset, Error::<T>::FundingAssetNotAccepted);

		// * Calculate variables *
		let buyable_tokens = if project_details.remaining_contribution_tokens > token_amount {
			token_amount
		} else {
			let remaining_amount = project_details.remaining_contribution_tokens;
			ticket_size = ct_usd_price.checked_mul_int(remaining_amount).ok_or(Error::<T>::BadMath)?;
			remaining_amount
		};
		let plmc_bond = Self::calculate_plmc_bond(ticket_size, multiplier, plmc_usd_price)?;
		let funding_asset_amount =
			funding_asset_usd_price.reciprocal().ok_or(Error::<T>::BadMath)?.saturating_mul_int(ticket_size);
		let asset_id = asset.to_statemint_id();
		let remaining_cts_after_purchase = project_details.remaining_contribution_tokens.saturating_sub(buyable_tokens);

		let new_contribution = ContributionInfoOf::<T> {
			id: contribution_id,
			project_id,
			contributor: contributor.clone(),
			ct_amount: token_amount,
			usd_contribution_amount: ticket_size,
			multiplier,
			funding_asset: asset,
			funding_asset_amount,
			plmc_bond,
			plmc_vesting_info: None,
			funds_released: false,
			ct_minted: false,
		};

		// * Update storage *
		// Try adding the new contribution to the system
		if existing_contributions.len() < T::MaxContributionsPerUser::get() as usize {
			Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
			Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;
		} else {
			let lowest_contribution = existing_contributions
				.iter()
				.min_by_key(|contribution| contribution.plmc_bond)
				.ok_or(Error::<T>::ImpossibleState)?;

			ensure!(new_contribution.plmc_bond > lowest_contribution.plmc_bond, Error::<T>::ContributionTooLow);

			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&lowest_contribution.contributor,
				lowest_contribution.plmc_bond,
				Precision::Exact,
			)?;
			T::FundingCurrency::transfer(
				asset_id,
				&Self::fund_account_id(project_id),
				&lowest_contribution.contributor,
				lowest_contribution.funding_asset_amount,
				Preservation::Expendable,
			)?;
			Contributions::<T>::remove((project_id, &lowest_contribution.contributor, &lowest_contribution.id));

			Self::try_plmc_participation_lock(contributor, project_id, plmc_bond)?;
			Self::try_funding_asset_hold(contributor, project_id, funding_asset_amount, asset_id)?;

			project_details.remaining_contribution_tokens =
				project_details.remaining_contribution_tokens.saturating_add(lowest_contribution.ct_amount);
			project_details.funding_amount_reached =
				project_details.funding_amount_reached.saturating_sub(lowest_contribution.usd_contribution_amount);
		}

		Contributions::<T>::insert((project_id, contributor, contribution_id), &new_contribution);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));

		project_details.remaining_contribution_tokens =
			project_details.remaining_contribution_tokens.saturating_sub(new_contribution.ct_amount);
		project_details.funding_amount_reached =
			project_details.funding_amount_reached.saturating_add(new_contribution.usd_contribution_amount);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// If no CTs remain, end the funding phase
		if remaining_cts_after_purchase == 0u32.into() {
			Self::remove_from_update_store(&project_id)?;
			Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::FundingEnd));
		}

		// * Emit events *
		Self::deposit_event(Event::Contribution {
			project_id,
			contributor: contributor.clone(),
			amount: token_amount,
			multiplier,
		});

		Ok(Pays::No.into())
	}

	pub fn do_decide_project_outcome(
		issuer: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		decision: FundingOutcomeDecision,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer == issuer, Error::<T>::NotAllowed);
		ensure!(project_details.status == ProjectStatus::AwaitingProjectDecision, Error::<T>::NotAllowed);

		// * Update storage *
		Self::remove_from_update_store(&project_id)?;
		Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::ProjectDecision(decision)));

		Ok(())
	}

	pub fn do_bid_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = bid.final_ct_amount;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!bid.ct_minted, Error::<T>::NotAllowed);
		ensure!(matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)), Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		bid.ct_minted = true;

		// * Update storage *
		T::ContributionTokenCurrency::mint_into(project_id, &bid.bidder, ct_amount)?;
		Bids::<T>::insert((project_id, bidder, bid_id), &bid);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id: bid.project_id,
			claimer: bidder.clone(),
			amount: ct_amount,
		});

		Ok(())
	}

	pub fn do_contribution_ct_mint_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> DispatchResult {
		// * Get variables *
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let ct_amount = contribution.ct_amount;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(!contribution.ct_minted, Error::<T>::NotAllowed);
		ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

		// * Calculate variables *
		contribution.ct_minted = true;

		// * Update storage *
		T::ContributionTokenCurrency::mint_into(project_id, &contribution.contributor, ct_amount)?;
		Contributions::<T>::insert((project_id, contributor, contribution_id), contribution);

		// * Emit events *
		Self::deposit_event(Event::ContributionTokenMinted {
			releaser: releaser.clone(),
			project_id,
			claimer: contributor.clone(),
			amount: ct_amount,
		});

		Ok(())
	}

	pub fn do_evaluation_unbond_for(
		releaser: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let released_evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			(project_details.evaluation_round_info.evaluators_outcome == EvaluatorsOutcomeOf::<T>::Unchanged ||
				released_evaluation.rewarded_or_slashed) &&
				matches!(
					project_details.status,
					ProjectStatus::EvaluationFailed | ProjectStatus::FundingFailed | ProjectStatus::FundingSuccessful
				),
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(
			&LockType::Evaluation(project_id),
			evaluator,
			released_evaluation.current_plmc_bond,
			Precision::Exact,
		)?;
		Evaluations::<T>::remove((project_id, evaluator, evaluation_id));

		// * Emit events *
		Self::deposit_event(Event::BondReleased {
			project_id,
			amount: released_evaluation.current_plmc_bond,
			bonder: evaluator.clone(),
			releaser: releaser.clone(),
		});

		Ok(())
	}

	pub fn do_evaluation_reward_payout_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let reward_info =
			if let EvaluatorsOutcome::Rewarded(info) = project_details.evaluation_round_info.evaluators_outcome {
				info
			} else {
				return Err(Error::<T>::NotAllowed.into())
			};
		let mut evaluation =
			Evaluations::<T>::get((project_id, evaluator, evaluation_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			!evaluation.rewarded_or_slashed && matches!(project_details.status, ProjectStatus::FundingSuccessful),
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
		T::ContributionTokenCurrency::mint_into(project_id, &evaluation.evaluator, total_reward_amount)?;
		evaluation.rewarded_or_slashed = true;
		Evaluations::<T>::insert((project_id, evaluator, evaluation_id), evaluation);

		// * Emit events *
		Self::deposit_event(Event::EvaluationRewarded {
			project_id,
			evaluator: evaluator.clone(),
			id: evaluation_id,
			amount: total_reward_amount,
			caller: caller.clone(),
		});

		Ok(())
	}

	pub fn do_evaluation_slash_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		evaluator: &AccountIdOf<T>,
		evaluation_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::TreasuryAccount::get();

		let mut user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, evaluator));
		let mut evaluation =
			user_evaluations.find(|evaluation| evaluation.id == evaluation_id).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			!evaluation.rewarded_or_slashed &&
				matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		// * Update storage *
		evaluation.rewarded_or_slashed = true;

		T::NativeCurrency::transfer_on_hold(
			&LockType::Evaluation(project_id),
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

	pub fn do_start_bid_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
			LockType::Participation(project_id),
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

	pub fn do_start_contribution_vesting_schedule_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
			LockType::Participation(project_id),
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

	pub fn do_vest_plmc_for(
		caller: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(matches!(project_details.status, ProjectStatus::FundingSuccessful), Error::<T>::NotAllowed);

		// * Update storage *
		let vested_amount = T::Vesting::vest(participant.clone(), LockType::Participation(project_id))?;

		// * Emit events *
		Self::deposit_event(Event::ParticipantPlmcVested { project_id, participant, amount: vested_amount, caller });

		Ok(())
	}

	pub fn do_release_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
			payout_asset.to_statemint_id(),
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

	pub fn do_bid_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingFailed &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)) &&
				bid.funds_released,
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(&LockType::Participation(project_id), bidder, bid.plmc_bond, Precision::Exact)?;
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

	pub fn do_release_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
			payout_asset.to_statemint_id(),
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

	pub fn do_contribution_unbond_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let bid = Contributions::<T>::get((project_id, contributor, contribution_id))
			.ok_or(Error::<T>::EvaluationNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingFailed, Error::<T>::NotAllowed);

		// * Update Storage *
		T::NativeCurrency::release(&LockType::Participation(project_id), contributor, bid.plmc_bond, Precision::Exact)?;
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

	pub fn do_payout_bid_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		bidder: &AccountIdOf<T>,
		bid_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let mut bid = Bids::<T>::get((project_id, bidder, bid_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful &&
				matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)),
			Error::<T>::NotAllowed
		);

		// * Calculate variables *
		let issuer = project_details.issuer;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = bid.funding_asset_amount_locked;
		let payout_asset = bid.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
			&project_pot,
			&issuer,
			payout_amount,
			Preservation::Expendable,
		)?;
		bid.funds_released = true;
		Bids::<T>::insert((project_id, bidder, bid_id), bid);

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

	pub fn do_payout_contribution_funds_for(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		contributor: &AccountIdOf<T>,
		contribution_id: u32,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let mut contribution =
			Contributions::<T>::get((project_id, contributor, contribution_id)).ok_or(Error::<T>::BidNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);

		// * Calculate variables *
		let issuer = project_details.issuer;
		let project_pot = Self::fund_account_id(project_id);
		let payout_amount = contribution.funding_asset_amount;
		let payout_asset = contribution.funding_asset;

		// * Update storage *
		T::FundingCurrency::transfer(
			payout_asset.to_statemint_id(),
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
}

// Helper functions
impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: T::ProjectIdentifier) -> AccountIdOf<T> {
		T::PalletId::get().into_sub_account_truncating(index)
	}

	/// Adds a project to the ProjectsToUpdate storage, so it can be updated at some later point in time.
	pub fn add_to_update_store(block_number: T::BlockNumber, store: (&T::ProjectIdentifier, UpdateType)) {
		// Try to get the project into the earliest possible block to update.
		// There is a limit for how many projects can update each block, so we need to make sure we don't exceed that limit
		let mut block_number = block_number;
		while ProjectsToUpdate::<T>::try_append(block_number, store.clone()).is_err() {
			// TODO: Should we end the loop if we iterated over too many blocks?
			block_number += 1u32.into();
		}
	}

	pub fn remove_from_update_store(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		let (block_position, project_index) = ProjectsToUpdate::<T>::iter()
			.find_map(|(block, project_vec)| {
				let project_index = project_vec.iter().position(|(id, _update_type)| id == project_id)?;
				Some((block, project_index))
			})
			.ok_or(Error::<T>::ProjectNotInUpdateStore)?;

		ProjectsToUpdate::<T>::mutate(block_position, |project_vec| {
			project_vec.remove(project_index);
		});

		Ok(())
	}

	pub fn calculate_plmc_bond(
		ticket_size: BalanceOf<T>,
		multiplier: MultiplierOf<T>,
		plmc_price: PriceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let usd_bond = multiplier.calculate_bonding_requirement::<T>(ticket_size).map_err(|_| Error::<T>::BadMath)?;
		plmc_price.reciprocal().ok_or(Error::<T>::BadMath)?.checked_mul_int(usd_bond).ok_or(Error::<T>::BadMath.into())
	}

	/// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_info(
		_caller: &AccountIdOf<T>,
		multiplier: MultiplierOf<T>,
		bonded_amount: BalanceOf<T>,
	) -> Result<VestingInfo<T::BlockNumber, BalanceOf<T>>, DispatchError> {
		// TODO: duration should depend on `_multiplier` and `_caller` credential
		let duration: T::BlockNumber = multiplier.calculate_vesting_duration::<T>();
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
		project_id: T::ProjectIdentifier,
		end_block: T::BlockNumber,
		total_allocation_size: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// Get all the bids that were made before the end of the candle
		let mut bids = Bids::<T>::iter_prefix_values((project_id,)).collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = Zero::zero();
		// temp variable to store the total value of the bids (i.e price * amount = Ticket size)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();
		let project_account = Self::fund_account_id(project_id);
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		// sort bids by price, and equal prices sorted by block number
		bids.sort_by(|a, b| b.cmp(a));
		// accept only bids that were made before `end_block` i.e end of candle auction
		let bids: Result<Vec<_>, DispatchError> = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					bid.status = BidStatus::Rejected(RejectionReason::AfterCandleEnd);
					bid.final_ct_amount = 0_u32.into();
					bid.final_ct_usd_price = PriceOf::<T>::zero();

					T::FundingCurrency::transfer(
						bid.funding_asset.to_statemint_id(),
						&project_account,
						&bid.bidder,
						bid.funding_asset_amount_locked,
						Preservation::Preserve,
					)?;
					T::NativeCurrency::release(
						&LockType::Participation(project_id),
						&bid.bidder,
						bid.plmc_bond,
						Precision::Exact,
					)?;
					bid.funding_asset_amount_locked = BalanceOf::<T>::zero();
					bid.plmc_bond = BalanceOf::<T>::zero();

					return Ok(bid)
				}
				let buyable_amount = total_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount.is_zero() {
					dbg!("buyable amount is zero");
					bid.status = BidStatus::Rejected(RejectionReason::NoTokensLeft);
					bid.final_ct_amount = Zero::zero();
					bid.final_ct_usd_price = Zero::zero();

					T::FundingCurrency::transfer(
						bid.funding_asset.to_statemint_id(),
						&project_account,
						&bid.bidder,
						bid.funding_asset_amount_locked,
						Preservation::Preserve,
					)?;
					T::NativeCurrency::release(
						&LockType::Participation(project_id),
						&bid.bidder,
						bid.plmc_bond,
						Precision::Exact,
					)?;
					bid.funding_asset_amount_locked = Zero::zero();
					bid.plmc_bond = Zero::zero();
					return Ok(bid)
				} else if bid.original_ct_amount <= buyable_amount {
					let maybe_ticket_size = bid.original_ct_usd_price.checked_mul_int(bid.original_ct_amount);
					if let Some(ticket_size) = maybe_ticket_size {
						bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
						bid_usd_value_sum.saturating_accrue(ticket_size);
						bid.status = BidStatus::Accepted;
					} else {
						bid.status = BidStatus::Rejected(RejectionReason::BadMath);

						bid.final_ct_amount = Zero::zero();
						bid.final_ct_usd_price = Zero::zero();

						T::FundingCurrency::transfer(
							bid.funding_asset.to_statemint_id(),
							&project_account,
							&bid.bidder,
							bid.funding_asset_amount_locked,
							Preservation::Preserve,
						)?;
						T::NativeCurrency::release(
							&LockType::Participation(project_id),
							&bid.bidder,
							bid.plmc_bond,
							Precision::Exact,
						)?;
						bid.funding_asset_amount_locked = Zero::zero();
						bid.plmc_bond = Zero::zero();
						return Ok(bid)
					}
				} else {
					let maybe_ticket_size = bid.original_ct_usd_price.checked_mul_int(buyable_amount);
					if let Some(ticket_size) = maybe_ticket_size {
						bid_usd_value_sum.saturating_accrue(ticket_size);
						bid_token_amount_sum.saturating_accrue(buyable_amount);
						bid.status = BidStatus::PartiallyAccepted(buyable_amount, RejectionReason::NoTokensLeft);
						bid.final_ct_amount = buyable_amount;

						let funding_asset_price = T::PriceProvider::get_price(bid.funding_asset.to_statemint_id())
							.ok_or(Error::<T>::PriceNotFound)?;
						let funding_asset_amount_needed = funding_asset_price
							.reciprocal()
							.ok_or(Error::<T>::BadMath)?
							.checked_mul_int(ticket_size)
							.ok_or(Error::<T>::BadMath)?;
						T::FundingCurrency::transfer(
							bid.funding_asset.to_statemint_id(),
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
							&LockType::Participation(project_id),
							&bid.bidder,
							bid.plmc_bond.saturating_sub(plmc_bond_needed),
							Precision::Exact,
						)?;

						bid.funding_asset_amount_locked = funding_asset_amount_needed;
						bid.plmc_bond = plmc_bond_needed;
					} else {
						bid.status = BidStatus::Rejected(RejectionReason::BadMath);
						bid.final_ct_amount = Zero::zero();
						bid.final_ct_usd_price = Zero::zero();

						T::FundingCurrency::transfer(
							bid.funding_asset.to_statemint_id(),
							&project_account,
							&bid.bidder,
							bid.funding_asset_amount_locked,
							Preservation::Preserve,
						)?;
						T::NativeCurrency::release(
							&LockType::Participation(project_id),
							&bid.bidder,
							bid.plmc_bond,
							Precision::Exact,
						)?;
						bid.funding_asset_amount_locked = Zero::zero();
						bid.plmc_bond = Zero::zero();

						return Ok(bid)
					}
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
		let weighted_token_price: PriceOf<T> = bids
			.iter()
			.filter_map(|bid| match bid.status {
				BidStatus::Accepted => {
					let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(
						bid.original_ct_usd_price.saturating_mul_int(bid.original_ct_amount),
						bid_usd_value_sum,
					);
					let weighted_price = bid.original_ct_usd_price * bid_weight;
					Some(weighted_price)
				},

				BidStatus::PartiallyAccepted(amount, _) => {
					let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(
						bid.original_ct_usd_price.saturating_mul_int(amount),
						bid_usd_value_sum,
					);
					Some(bid.original_ct_usd_price.saturating_mul(bid_weight))
				},

				_ => None,
			})
			.reduce(|a, b| a.saturating_add(b))
			.ok_or(Error::<T>::NoBidsFound)?;

		let mut final_total_funding_reached_by_bids = BalanceOf::<T>::zero();
		// Update the bid in the storage
		for mut bid in bids.into_iter() {
			if bid.final_ct_usd_price > weighted_token_price {
				bid.final_ct_usd_price = weighted_token_price;
				let new_ticket_size =
					weighted_token_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;

				let funding_asset_price = T::PriceProvider::get_price(bid.funding_asset.to_statemint_id())
					.ok_or(Error::<T>::PriceNotFound)?;
				let funding_asset_amount_needed = funding_asset_price
					.reciprocal()
					.ok_or(Error::<T>::BadMath)?
					.checked_mul_int(new_ticket_size)
					.ok_or(Error::<T>::BadMath)?;

				let try_transfer = T::FundingCurrency::transfer(
					bid.funding_asset.to_statemint_id(),
					&project_account,
					&bid.bidder,
					bid.funding_asset_amount_locked.saturating_sub(funding_asset_amount_needed),
					Preservation::Preserve,
				);
				if let Err(e) = try_transfer {
					Self::deposit_event(Event::TransferError { error: e });
				}

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

				let try_release = T::NativeCurrency::release(
					&LockType::Participation(project_id),
					&bid.bidder,
					bid.plmc_bond.saturating_sub(plmc_bond_needed),
					Precision::Exact,
				);
				if let Err(e) = try_release {
					Self::deposit_event(Event::TransferError { error: e });
				}

				bid.plmc_bond = plmc_bond_needed;
			}
			let final_ticket_size =
				bid.final_ct_usd_price.checked_mul_int(bid.final_ct_amount).ok_or(Error::<T>::BadMath)?;
			final_total_funding_reached_by_bids += final_ticket_size;
			Bids::<T>::insert((project_id, &bid.bidder, &bid.id), &bid);
		}

		// Update storage
		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> Result<(), DispatchError> {
			if let Some(info) = maybe_info {
				info.weighted_average_price = Some(weighted_token_price);
				info.remaining_contribution_tokens =
					info.remaining_contribution_tokens.saturating_sub(bid_token_amount_sum);
				info.funding_amount_reached =
					info.funding_amount_reached.saturating_add(final_total_funding_reached_by_bids);
				Ok(())
			} else {
				Err(Error::<T>::ProjectNotFound.into())
			}
		})?;

		Ok(())
	}

	// fn reject_bid(status){

	// }

	pub fn select_random_block(
		candle_starting_block: T::BlockNumber,
		candle_ending_block: T::BlockNumber,
	) -> T::BlockNumber {
		let nonce = Self::get_and_increment_nonce();
		let (random_value, _known_since) = T::Randomness::random(&nonce);
		let random_block = <T::BlockNumber>::decode(&mut random_value.as_ref())
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
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// Check if the user has already locked tokens in the evaluation period
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, who));

		let mut to_convert = amount;
		for mut evaluation in user_evaluations {
			if to_convert == Zero::zero() {
				break
			}
			let slash_deposit = <T as Config>::EvaluatorSlash::get() * evaluation.original_plmc_bond;
			let available_to_convert = evaluation.current_plmc_bond.saturating_sub(slash_deposit);
			let converted = to_convert.min(available_to_convert);
			evaluation.current_plmc_bond = evaluation.current_plmc_bond.saturating_sub(converted);
			Evaluations::<T>::insert((project_id, who, evaluation.id), evaluation);
			T::NativeCurrency::release(&LockType::Evaluation(project_id), who, converted, Precision::Exact)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&LockType::Participation(project_id), who, converted)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&LockType::Participation(project_id), who, to_convert)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
		asset_id: AssetIdOf<T>,
	) -> Result<(), DispatchError> {
		let fund_account = Self::fund_account_id(project_id);

		T::FundingCurrency::transfer(asset_id, who, &fund_account, amount, Preservation::Expendable)?;

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
		if let Some(remaining_amount) = remaining_for_fee.checked_sub(&limit) {
			*remaining_for_fee = remaining_amount;
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
	pub fn generate_evaluator_rewards_info(
		project_id: <T as Config>::ProjectIdentifier,
	) -> Result<RewardInfoOf<T>, DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let evaluations = Evaluations::<T>::iter_prefix((project_id,)).collect::<Vec<_>>();

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached;
		let fundraising_target = project_details.fundraising_target;
		let total_issuer_fees = Self::calculate_fees(funding_amount_reached);

		// Calculate the number of tokens sold for the project.
		let token_sold = project_metadata
			.total_allocation_size
			.checked_sub(&project_details.remaining_contribution_tokens)
			// Ensure safety by providing a default in case of unexpected situations.
			.unwrap_or(project_metadata.total_allocation_size);
		let total_fee_allocation = total_issuer_fees * token_sold;

		// Calculate the percentage of target funding based on available documentation.
		let percentage_of_target_funding = Perquintill::from_rational(funding_amount_reached, fundraising_target);

		// Calculate rewards.
		let evaluator_rewards = percentage_of_target_funding * Perquintill::from_percent(30) * total_fee_allocation;
		// Placeholder allocations (intended for future use).
		let _liquidity_pool = Perquintill::from_percent(50) * total_fee_allocation;
		let _long_term_holder_bonus = _liquidity_pool.saturating_sub(evaluator_rewards);

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

		Ok(reward_info)
	}

	pub fn make_project_funding_successful(
		project_id: T::ProjectIdentifier,
		mut project_details: ProjectDetailsOf<T>,
		reason: SuccessReason,
		settlement_delta: T::BlockNumber,
	) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingSuccessful;
		ProjectsDetails::<T>::insert(project_id, project_details);

		Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement));

		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Success(reason) });

		Ok(())
	}

	pub fn make_project_funding_fail(
		project_id: T::ProjectIdentifier,
		mut project_details: ProjectDetailsOf<T>,
		reason: FailureReason,
		settlement_delta: T::BlockNumber,
	) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		project_details.status = ProjectStatus::FundingFailed;
		ProjectsDetails::<T>::insert(project_id, project_details);

		Self::add_to_update_store(now + settlement_delta, (&project_id, UpdateType::StartSettlement));
		Self::deposit_event(Event::FundingEnded { project_id, outcome: FundingOutcome::Failure(reason) });
		Ok(())
	}
}
