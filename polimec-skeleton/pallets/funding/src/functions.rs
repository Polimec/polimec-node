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

use super::*;

use crate::traits::{BondingRequirementCalculation, ProvideStatemintPrice};
use frame_support::traits::fungible::InspectHold;
use frame_support::traits::fungibles::Inspect;
use frame_support::traits::tokens::{Precision, Preservation};
use frame_support::{
	ensure,
	pallet_prelude::DispatchError,
	traits::{
		fungible::MutateHold as FungibleMutateHold,
		fungibles::{metadata::Mutate as MetadataMutate, Create, Mutate as FungiblesMutate},
		Get,
	},
};
use sp_arithmetic::Perbill;

use sp_arithmetic::traits::{CheckedSub, Zero};
use sp_runtime::Percent;
use sp_std::prelude::*;

use itertools::Itertools;

pub const US_DOLLAR: u128 = 1_0_000_000_000;
pub const US_CENT: u128 = 0_0_100_000_000;

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
	pub fn do_create(issuer: AccountIdOf<T>, initial_metadata: ProjectMetadataOf<T>) -> Result<(), DispatchError> {
		// TODO: Probably the issuers don't want to sell all of their tokens. Is there some logic for this?
		// 	also even if an issuer wants to sell all their tokens, they could target a lower amount than that to consider it a success
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
			};
		}

		// * Calculate new variables *
		let fundraising_target = initial_metadata
			.minimum_price
			.checked_mul_int(initial_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let project_details = ProjectDetails {
			issuer: issuer.clone(),
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair::new(Some(<frame_system::Pallet<T>>::block_number()), None),
				evaluation: BlockNumberPair::new(None, None),
				auction_initialize_period: BlockNumberPair::new(None, None),
				english_auction: BlockNumberPair::new(None, None),
				random_candle_ending: None,
				candle_auction: BlockNumberPair::new(None, None),
				community: BlockNumberPair::new(None, None),
				remainder: BlockNumberPair::new(None, None),
			},
			remaining_contribution_tokens: initial_metadata.total_allocation_size,
			funding_amount_reached: BalanceOf::<T>::zero(),
			cleanup: ProjectCleanup::NotReady,
		};

		let project_metadata = initial_metadata;

		// * Update storage *
		ProjectsMetadata::<T>::insert(project_id, project_metadata.clone());
		ProjectsDetails::<T>::insert(project_id, project_details);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());
		if let Some(metadata) = project_metadata.offchain_information_hash {
			Images::<T>::insert(metadata, issuer);
		}

		// * Emit events *
		Self::deposit_event(Event::<T>::Created { project_id });

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
		ensure!(
			project_details.status == ProjectStatus::Application,
			Error::<T>::ProjectNotInApplicationRound
		);
		ensure!(!project_details.is_frozen, Error::<T>::ProjectAlreadyFrozen);
		ensure!(
			project_metadata.offchain_information_hash.is_some(),
			Error::<T>::MetadataNotProvided
		);

		// * Calculate new variables *
		let evaluation_end_block = now + T::EvaluationDuration::get();
		project_details
			.phase_transition_points
			.application
			.update(None, Some(now));
		project_details
			.phase_transition_points
			.evaluation
			.update(Some(now + 1u32.into()), Some(evaluation_end_block));
		project_details.is_frozen = true;
		project_details.status = ProjectStatus::EvaluationRound;

		// * Update storage *
		// TODO: Should we make it possible to end an application, and schedule for a later point the evaluation?
		// 	Or should we just make it so that the evaluation starts immediately after the application ends?
		ProjectsDetails::<T>::insert(project_id, project_details);
		Self::add_to_update_store(
			evaluation_end_block + 1u32.into(),
			(&project_id, UpdateType::EvaluationEnd),
		);

		// * Emit events *
		Self::deposit_event(Event::<T>::EvaluationStarted { project_id });

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
		let evaluation_end_block = project_details
			.phase_transition_points
			.evaluation
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target_usd = project_details.fundraising_target;
		let current_plmc_price =
			T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::EvaluationRound,
			Error::<T>::ProjectNotInEvaluationRound
		);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// * Calculate new variables *
		let initial_balance: BalanceOf<T> = 0u32.into();
		let total_amount_bonded =
			Evaluations::<T>::iter_prefix(project_id).fold(initial_balance, |total, (_evaluator, bonds)| {
				let user_total_plmc_bond = bonds
					.iter()
					.fold(total, |acc, bond| acc.saturating_add(bond.original_plmc_bond));
				total.saturating_add(user_total_plmc_bond)
			});
		// TODO: PLMC-142. 10% is hardcoded, check if we want to configure it a runtime as explained here:
		// 	https://substrate.stackexchange.com/questions/2784/how-to-get-a-percent-portion-of-a-balance:
		// TODO: PLMC-143. Check if it's safe to use * here
		let evaluation_target_usd = Perbill::from_percent(10) * fundraising_target_usd;
		let evaluation_target_plmc = current_plmc_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(evaluation_target_usd)
			.ok_or(Error::<T>::BadMath)?;

		let auction_initialize_period_start_block = now + 1u32.into();
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block.clone() + T::AuctionInitializePeriodDuration::get();

		// Check which logic path to follow
		let is_funded = total_amount_bonded >= evaluation_target_plmc;

		// * Branch in possible project paths *
		// Successful path
		if is_funded {
			// * Update storage *
			project_details
				.phase_transition_points
				.auction_initialize_period
				.update(
					Some(auction_initialize_period_start_block.clone()),
					Some(auction_initialize_period_end_block),
				);
			project_details.status = ProjectStatus::AuctionInitializePeriod;
			ProjectsDetails::<T>::insert(project_id, project_details);
			Self::add_to_update_store(
				auction_initialize_period_end_block + 1u32.into(),
				(&project_id, UpdateType::EnglishAuctionStart),
			);

			// * Emit events *
			Self::deposit_event(Event::<T>::AuctionInitializePeriod {
				project_id,
				start_block: auction_initialize_period_start_block,
				end_block: auction_initialize_period_end_block,
			});
		// TODO: PLMC-144. Unlock the bonds and clean the storage

		// Unsuccessful path
		} else {
			// * Update storage *
			project_details.status = ProjectStatus::EvaluationFailed;
			project_details.cleanup = ProjectCleanup::Ready(ProjectFinalizer::Failure(Default::default()));
			ProjectsDetails::<T>::insert(project_id, project_details);

			// * Emit events *
			Self::deposit_event(Event::<T>::EvaluationFailed { project_id });
			// TODO: PLMC-144. Unlock the bonds and clean the storage
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
		ensure!(
			now >= auction_initialize_period_start_block,
			Error::<T>::TooEarlyForEnglishAuctionStart
		);
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
			.update(Some(english_start_block), Some(english_end_block.clone()));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::English);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// If this function was called inside the period, then it was called by the extrinsic and we need to
		// remove the scheduled automatic transition
		if now <= auction_initialize_period_end_block {
			Self::remove_from_update_store(&project_id)?;
		}
		// Schedule for automatic transition to candle auction round
		Self::add_to_update_store(
			english_end_block + 1u32.into(),
			(&project_id, UpdateType::CandleAuctionStart),
		);

		// * Emit events *
		Self::deposit_event(Event::<T>::EnglishAuctionStarted { project_id, when: now });

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
		let english_end_block = project_details
			.phase_transition_points
			.english_auction
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

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
		project_details
			.phase_transition_points
			.candle_auction
			.update(Some(candle_start_block), Some(candle_end_block.clone()));
		project_details.status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic check by on_initialize. Success depending on enough funding reached
		Self::add_to_update_store(
			candle_end_block + 1u32.into(),
			(&project_id, UpdateType::CommunityFundingStart),
		);

		// * Emit events *
		Self::deposit_event(Event::<T>::CandleAuctionStarted { project_id, when: now });

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
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_candle_start_block = project_details
			.phase_transition_points
			.candle_auction
			.start()
			.ok_or(Error::<T>::FieldIsNone)?;
		let auction_candle_end_block = project_details
			.phase_transition_points
			.candle_auction
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(
			now > auction_candle_end_block,
			Error::<T>::TooEarlyForCommunityRoundStart
		);
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
			Self::calculate_weighted_average_price(project_id, end_block, project_details.fundraising_target);
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
				Self::deposit_event(Event::<T>::AuctionFailed { project_id });

				Ok(())
			}
			e @ Err(_) => e,
			Ok(()) => {
				// Get info again after updating it with new price.
				project_details.phase_transition_points.random_candle_ending = Some(end_block);
				project_details
					.phase_transition_points
					.community
					.update(Some(community_start_block), Some(community_end_block.clone()));
				project_details.status = ProjectStatus::CommunityRound;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Self::add_to_update_store(
					community_end_block + 1u32.into(),
					(&project_id, UpdateType::RemainderFundingStart),
				);

				// * Emit events *
				Self::deposit_event(Event::<T>::CommunityFundingStarted { project_id });

				Ok(())
			}
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
		let community_end_block = project_details
			.phase_transition_points
			.community
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(
			project_details.status == ProjectStatus::CommunityRound,
			Error::<T>::ProjectNotInCommunityRound
		);

		// * Calculate new variables *
		let remainder_start_block = now + 1u32.into();
		let remainder_end_block = now + T::RemainderFundingDuration::get();

		// * Update Storage *
		project_details
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block.clone()));
		project_details.status = ProjectStatus::RemainderRound;
		ProjectsDetails::<T>::insert(project_id, project_details);
		// Schedule for automatic transition by `on_initialize`
		Self::add_to_update_store(remainder_end_block + 1u32.into(), (&project_id, UpdateType::FundingEnd));

		// * Emit events *
		Self::deposit_event(Event::<T>::RemainderFundingStarted { project_id });

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
	/// TODO: unsuccessful funding unimplemented
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
		let now = <frame_system::Pallet<T>>::block_number();
		// TODO: PLMC-149 Check if make sense to set the admin as T::fund_account_id(project_id)
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let token_information = project_metadata.token_information;
		let remaining_cts = project_details.remaining_contribution_tokens;
		let remainder_end_block = project_details.phase_transition_points.remainder.end();

		// * Validity checks *
		if let Some(end_block) = remainder_end_block {
			ensure!(now > end_block, Error::<T>::TooEarlyForFundingEnd);
		} else {
			ensure!(
				remaining_cts == 0u32.into() || project_details.status == ProjectStatus::FundingFailed,
				Error::<T>::TooEarlyForFundingEnd
			);
		}

		// * Calculate new variables *
		let funding_target = project_metadata
			.minimum_price
			.checked_mul_int(project_metadata.total_allocation_size)
			.ok_or(Error::<T>::BadMath)?;
		let funding_reached = project_details.funding_amount_reached;
		let funding_is_successful =
			!(project_details.status == ProjectStatus::FundingFailed || funding_reached < funding_target);

		if funding_is_successful {
			project_details.status = ProjectStatus::FundingSuccessful;
			project_details.cleanup = ProjectCleanup::Ready(ProjectFinalizer::Success(Default::default()));

			// * Update Storage *
			ProjectsDetails::<T>::insert(project_id, project_details.clone());
			T::ContributionTokenCurrency::create(project_id, project_details.issuer.clone(), false, 1_u32.into())
				.map_err(|_| Error::<T>::AssetCreationFailed)?;
			T::ContributionTokenCurrency::set(
				project_id,
				&project_details.issuer,
				token_information.name.into(),
				token_information.symbol.into(),
				token_information.decimals,
			)
			.map_err(|_| Error::<T>::AssetMetadataUpdateFailed)?;

			// * Emit events *
			let success_reason = match remaining_cts {
				x if x == 0u32.into() => SuccessReason::SoldOut,
				_ => SuccessReason::ReachedTarget,
			};
			Self::deposit_event(Event::<T>::FundingEnded {
				project_id,
				outcome: FundingOutcome::Success(success_reason),
			});
			Ok(())
		} else {
			project_details.status = ProjectStatus::FundingFailed;
			project_details.cleanup = ProjectCleanup::Ready(ProjectFinalizer::Failure(Default::default()));

			// * Update Storage *
			ProjectsDetails::<T>::insert(project_id, project_details.clone());

			// * Emit events *
			let failure_reason = FailureReason::TargetNotReached;
			Self::deposit_event(Event::<T>::FundingEnded {
				project_id,
				outcome: FundingOutcome::Failure(failure_reason),
			});
			Ok(())
		}
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
	/// TODO: Discuss this function with Leonardo
	///
	/// # Next step
	/// WIP
	pub fn do_ready_to_launch(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful,
			Error::<T>::ProjectNotInFundingEndedRound
		);

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
		issuer: AccountIdOf<T>, project_id: T::ProjectIdentifier, project_metadata_hash: T::Hash,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(project_details.issuer == issuer, Error::<T>::NotAllowed);
		ensure!(!project_details.is_frozen, Error::<T>::Frozen);
		ensure!(
			!Images::<T>::contains_key(project_metadata_hash),
			Error::<T>::MetadataAlreadyExists
		);

		// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );

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
		evaluator: AccountIdOf<T>, project_id: T::ProjectIdentifier, usd_amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_id = Self::next_evaluation_id();
		let mut caller_existing_evaluations = Evaluations::<T>::get(project_id, evaluator.clone());
		let plmc_usd_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		let early_evaluation_reward_threshold_usd =
			T::EarlyEvaluationThreshold::get() * project_details.fundraising_target;
		let all_existing_evaluations = Evaluations::<T>::iter_prefix(project_id);

		// * Validity Checks *
		ensure!(
			evaluator.clone() != project_details.issuer,
			Error::<T>::ContributionToThemselves
		);
		ensure!(
			project_details.status == ProjectStatus::EvaluationRound,
			Error::<T>::EvaluationNotStarted
		);

		// * Calculate new variables *
		let plmc_bond = plmc_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_amount)
			.ok_or(Error::<T>::BadMath)?;

		let previous_total_evaluation_bonded_usd = all_existing_evaluations
			.map(|(evaluator, evaluations)| {
				evaluations.iter().fold(BalanceOf::<T>::zero(), |acc, evaluation| {
					acc.saturating_add(evaluation.early_usd_amount)
						.saturating_add(evaluation.late_usd_amount)
				})
			})
			.fold(BalanceOf::<T>::zero(), |acc, evaluation| acc.saturating_add(evaluation));

		let remaining_bond_to_reach_threshold = early_evaluation_reward_threshold_usd
			.checked_sub(&previous_total_evaluation_bonded_usd)
			.unwrap_or(BalanceOf::<T>::zero());

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
		// TODO: PLMC-144. Unlock the PLMC when it's the right time

		match caller_existing_evaluations.try_push(new_evaluation.clone()) {
			Ok(_) => {
				T::NativeCurrency::hold(&LockType::Evaluation(project_id), &evaluator, plmc_bond)
					.map_err(|_| Error::<T>::InsufficientBalance)?;
			}
			Err(_) => {
				// Evaluations are stored in descending order. If the evaluation vector for the user is full, we drop the lowest/last bond
				let lowest_evaluation = caller_existing_evaluations.swap_remove(caller_existing_evaluations.len() - 1);

				ensure!(
					lowest_evaluation.original_plmc_bond < plmc_bond,
					Error::<T>::EvaluationBondTooLow
				);

				T::NativeCurrency::release(
					&LockType::Evaluation(project_id),
					&lowest_evaluation.evaluator,
					lowest_evaluation.original_plmc_bond,
					Precision::Exact,
				)
				.map_err(|_| Error::<T>::InsufficientBalance)?;

				T::NativeCurrency::hold(&LockType::Evaluation(project_id), &evaluator, plmc_bond)
					.map_err(|_| Error::<T>::InsufficientBalance)?;

				// This should never fail since we just removed an element from the vector
				caller_existing_evaluations
					.try_push(new_evaluation)
					.map_err(|_| Error::<T>::ImpossibleState)?;
			}
		};

		caller_existing_evaluations.sort_by_key(|bond| Reverse(bond.original_plmc_bond));

		Evaluations::<T>::set(project_id, evaluator.clone(), caller_existing_evaluations);
		NextEvaluationId::<T>::set(evaluation_id.saturating_add(One::one()));

		// * Emit events *
		Self::deposit_event(Event::<T>::FundsBonded {
			project_id,
			amount: plmc_bond,
			bonder: evaluator,
		});

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
		bidder: AccountIdOf<T>, project_id: T::ProjectIdentifier, ct_amount: BalanceOf<T>, ct_usd_price: T::Price,
		multiplier: Option<MultiplierOf<T>>, funding_asset: AcceptedFundingAsset,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let bid_id = Self::next_bid_id();
		let mut existing_bids = Bids::<T>::get(project_id, bidder.clone());

		let ticket_size = ct_usd_price.checked_mul_int(ct_amount).ok_or(Error::<T>::BadMath)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(funding_asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;
		let multiplier = multiplier.unwrap_or_default();

		// * Validity checks *
		ensure!(
			bidder.clone() != project_details.issuer,
			Error::<T>::ContributionToThemselves
		);
		ensure!(
			matches!(project_details.status, ProjectStatus::AuctionRound(_)),
			Error::<T>::AuctionNotStarted
		);
		ensure!(ct_usd_price >= project_metadata.minimum_price, Error::<T>::BidTooLow);
		if let Some(minimum_ticket_size) = project_metadata.ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(ticket_size >= minimum_ticket_size, Error::<T>::BidTooLow);
		};
		if let Some(maximum_ticket_size) = project_metadata.ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(ticket_size <= maximum_ticket_size, Error::<T>::BidTooLow);
		};
		ensure!(
			funding_asset == project_metadata.participation_currencies,
			Error::<T>::FundingAssetNotAccepted
		);

		// * Calculate new variables *
		let (plmc_vesting_period, ct_vesting_period) =
			Self::calculate_vesting_periods(bidder.clone(), multiplier, ct_amount, ct_usd_price)
				.map_err(|_| Error::<T>::BadMath)?;
		let required_plmc_bond = plmc_vesting_period.amount;
		let required_funding_asset_transfer = funding_asset_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.saturating_mul_int(ticket_size);
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
			funding_asset_amount_locked: required_funding_asset_transfer,
			multiplier,
			plmc_bond: required_plmc_bond,
			funded: false,
			plmc_vesting_period,
			ct_vesting_period,
			when: now,
			funds_released: false,
		};

		// * Update storage *
		match existing_bids.try_push(new_bid.clone()) {
			Ok(_) => {
				Self::try_plmc_participation_lock(&bidder, project_id, required_plmc_bond)?;
				Self::try_funding_asset_hold(&bidder, project_id, required_funding_asset_transfer, asset_id)?;

				// TODO: PLMC-159. Send an XCM message to Statemint/e to transfer a `bid.market_cap` amount of USDC (or the Currency specified by the issuer) to the PalletId Account
				// Alternative TODO: PLMC-159. The user should have the specified currency (e.g: USDC) already on Polimec
			}
			Err(_) => {
				// Since the bids are sorted by price, and in this branch the Vec is full, the last element is the lowest bid
				let lowest_plmc_bond = existing_bids
					.iter()
					.last()
					.ok_or(Error::<T>::ImpossibleState)?
					.plmc_bond;

				ensure!(new_bid.plmc_bond > lowest_plmc_bond, Error::<T>::BidTooLow);

				Self::release_last_funding_item_in_vec(
					&bidder,
					project_id,
					asset_id,
					&mut existing_bids,
					|x| x.plmc_bond,
					|x| x.funding_asset_amount_locked,
				)?;

				Self::try_plmc_participation_lock(&bidder, project_id, required_plmc_bond)?;

				Self::try_funding_asset_hold(&bidder, project_id, required_funding_asset_transfer, asset_id)?;

				// This should never fail, since we just removed an element from the Vec
				existing_bids
					.try_push(new_bid)
					.map_err(|_| Error::<T>::ImpossibleState)?;
			}
		};

		existing_bids.sort_by(|a, b| b.cmp(a));

		Bids::<T>::set(project_id, bidder, existing_bids);
		NextBidId::<T>::set(bid_id.saturating_add(One::one()));

		Self::deposit_event(Event::<T>::Bid {
			project_id,
			amount: ct_amount,
			price: ct_usd_price,
			multiplier,
		});

		Ok(())
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
		contributor: AccountIdOf<T>, project_id: T::ProjectIdentifier, token_amount: BalanceOf<T>,
		multiplier: Option<MultiplierOf<T>>, asset: AcceptedFundingAsset,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let contribution_id = Self::next_contribution_id();
		let mut existing_contributions = Contributions::<T>::get(project_id, contributor.clone());

		let ct_usd_price = project_details
			.weighted_average_price
			.ok_or(Error::<T>::AuctionNotStarted)?;
		let mut ticket_size = ct_usd_price.checked_mul_int(token_amount).ok_or(Error::<T>::BadMath)?;
		let funding_asset_usd_price =
			T::PriceProvider::get_price(asset.to_statemint_id()).ok_or(Error::<T>::PriceNotFound)?;
		// Default should normally be multiplier of 1
		let multiplier = multiplier.unwrap_or_default();

		// * Validity checks *
		ensure!(
			contributor.clone() != project_details.issuer,
			Error::<T>::ContributionToThemselves
		);
		ensure!(
			project_details.status == ProjectStatus::CommunityRound
				|| project_details.status == ProjectStatus::RemainderRound,
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
		ensure!(
			project_metadata.participation_currencies == asset,
			Error::<T>::FundingAssetNotAccepted
		);

		// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Retail, &contributor),
		// 	Error::<T>::NotAuthorized
		// );

		// * Calculate variables *
		let buyable_tokens = if project_details.remaining_contribution_tokens > token_amount {
			token_amount
		} else {
			let remaining_amount = project_details.remaining_contribution_tokens;
			ticket_size = ct_usd_price
				.checked_mul_int(remaining_amount)
				.ok_or(Error::<T>::BadMath)?;
			remaining_amount
		};
		let (plmc_vesting_period, ct_vesting_period) =
			Self::calculate_vesting_periods(contributor.clone(), multiplier.clone(), buyable_tokens, ct_usd_price)
				.map_err(|_| Error::<T>::BadMath)?;
		let required_plmc_bond = plmc_vesting_period.amount;
		let required_funding_asset_transfer = funding_asset_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.saturating_mul_int(ticket_size);
		let asset_id = asset.to_statemint_id();
		let remaining_cts_after_purchase = project_details
			.remaining_contribution_tokens
			.saturating_sub(buyable_tokens);

		let new_contribution = ContributionInfoOf::<T> {
			id: contribution_id,
			project_id,
			contributor: contributor.clone(),
			ct_amount: ct_vesting_period.amount,
			usd_contribution_amount: ticket_size,
			funding_asset: asset,
			funding_asset_amount: required_funding_asset_transfer,
			plmc_bond: required_plmc_bond,
			plmc_vesting_period,
			ct_vesting_period,
			funds_released: false,
		};

		// * Update storage *
		// Try adding the new contribution to the system
		match existing_contributions.try_push(new_contribution.clone()) {
			Ok(_) => {
				Self::try_plmc_participation_lock(&contributor, project_id, required_plmc_bond)?;
				Self::try_funding_asset_hold(&contributor, project_id, required_funding_asset_transfer, asset_id)?;
			}
			Err(_) => {
				// The contributions are sorted by highest PLMC bond. If the contribution vector for the user is full, we drop the lowest/last item
				let lowest_plmc_bond = existing_contributions
					.iter()
					.last()
					.ok_or(Error::<T>::ImpossibleState)?
					.plmc_bond;

				ensure!(
					new_contribution.plmc_bond > lowest_plmc_bond,
					Error::<T>::ContributionTooLow
				);

				Self::release_last_funding_item_in_vec(
					&contributor,
					project_id,
					asset_id,
					&mut existing_contributions,
					|x| x.plmc_bond,
					|x| x.funding_asset_amount,
				)?;

				Self::try_plmc_participation_lock(&contributor, project_id, required_plmc_bond)?;

				Self::try_funding_asset_hold(&contributor, project_id, required_funding_asset_transfer, asset_id)?;

				// This should never fail, since we just removed an item from the vector
				existing_contributions
					.try_push(new_contribution)
					.map_err(|_| Error::<T>::ImpossibleState)?;
			}
		}

		existing_contributions.sort_by_key(|contribution| Reverse(contribution.plmc_bond));

		Contributions::<T>::set(project_id, contributor.clone(), existing_contributions);
		NextContributionId::<T>::set(contribution_id.saturating_add(One::one()));
		ProjectsDetails::<T>::mutate(project_id, |maybe_project| {
			if let Some(project) = maybe_project {
				project.remaining_contribution_tokens = remaining_cts_after_purchase;
				project.funding_amount_reached = project.funding_amount_reached.saturating_add(ticket_size);
			}
		});



		// If no CTs remain, end the funding phase
		if remaining_cts_after_purchase == 0u32.into() {
			Self::remove_from_update_store(&project_id)?;
			Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::FundingEnd));
		}

		// * Emit events *
		Self::deposit_event(Event::<T>::Contribution {
			project_id,
			contributor,
			amount: token_amount,
			multiplier,
		});

		Ok(())
	}

	/// Unbond some plmc from a successful bid, after a step in the vesting period has passed.
	///
	/// # Arguments
	/// * bid: The bid to unbond from
	///
	/// # Storage access
	/// * [`Bids`] - Check if its time to unbond some plmc based on the bid vesting period, and update the bid after unbonding.
	/// * [`BiddingBonds`] - Update the bid with the new vesting period struct, reflecting this withdrawal
	/// * [`T::NativeCurrency`] - Unreserve the unbonded amount
	pub fn do_vested_plmc_bid_unbond_for(
		releaser: AccountIdOf<T>, project_id: T::ProjectIdentifier, bidder: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let bids = Bids::<T>::get(project_id, &bidder);
		let now = <frame_system::Pallet<T>>::block_number();
		let mut new_bids = vec![];

		for mut bid in bids {
			let mut plmc_vesting = bid.plmc_vesting_period;

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if plmc_vesting.next_withdrawal > now {
				continue;
			}

			// * Calculate variables *
			let mut unbond_amount: BalanceOf<T> = 0u32.into();

			// update vesting period until the next withdrawal is in the future
			while let Ok(amount) = plmc_vesting.calculate_next_withdrawal() {
				unbond_amount = unbond_amount.saturating_add(amount);
				if plmc_vesting.next_withdrawal > now {
					break;
				}
			}
			bid.plmc_vesting_period = plmc_vesting;

			// * Update storage *
			// TODO: check that the full amount was unreserved
			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&bid.bidder,
				unbond_amount,
				Precision::Exact,
			)?;
			new_bids.push(bid.clone());

			// * Emit events *
			Self::deposit_event(Event::<T>::BondReleased {
				project_id: bid.project_id,
				amount: unbond_amount,
				bonder: bid.bidder,
				releaser: releaser.clone(),
			});
		}

		// Should never return error since we are using the same amount of bids that were there before.
		let new_bids: BoundedVec<BidInfoOf<T>, T::MaxBidsPerUser> =
			new_bids.try_into().map_err(|_| Error::<T>::TooManyBids)?;

		// Update the AuctionInfo with the new bids vector
		Bids::<T>::insert(project_id, &bidder, new_bids);

		Ok(())
	}

	/// Mint contribution tokens after a step in the vesting period for a successful bid.
	///
	/// # Arguments
	/// * bidder: The account who made bids
	/// * project_id: The project the bids where made for
	///
	/// # Storage access
	///
	/// * `AuctionsInfo` - Check if its time to mint some tokens based on the bid vesting period, and update the bid after minting.
	/// * `T::ContributionTokenCurrency` - Mint the tokens to the bidder
	pub fn do_vested_contribution_token_bid_mint_for(
		releaser: AccountIdOf<T>, project_id: T::ProjectIdentifier, bidder: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let bids = Bids::<T>::get(project_id, &bidder);
		let mut new_bids = vec![];
		let now = <frame_system::Pallet<T>>::block_number();
		for mut bid in bids {
			let mut ct_vesting = bid.ct_vesting_period;
			let mut mint_amount: BalanceOf<T> = 0u32.into();

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if ct_vesting.next_withdrawal > now {
				continue;
			}

			// * Calculate variables *
			// Update vesting period until the next withdrawal is in the future
			while let Ok(amount) = ct_vesting.calculate_next_withdrawal() {
				mint_amount = mint_amount.saturating_add(amount);
				if ct_vesting.next_withdrawal > now {
					break;
				}
			}
			bid.ct_vesting_period = ct_vesting;

			// * Update storage *
			// TODO: Should we mint here, or should the full mint happen to the treasury and then do transfers from there?
			// Mint the funds for the user
			T::ContributionTokenCurrency::mint_into(bid.project_id, &bid.bidder, mint_amount)?;
			new_bids.push(bid);

			// * Emit events *
			Self::deposit_event(Event::<T>::ContributionTokenMinted {
				caller: releaser.clone(),
				project_id,
				contributor: bidder.clone(),
				amount: mint_amount,
			})
		}
		// Update the bids with the new vesting period struct
		let new_bids: BoundedVec<BidInfoOf<T>, T::MaxBidsPerUser> =
			new_bids.try_into().map_err(|_| Error::<T>::TooManyBids)?;
		Bids::<T>::insert(project_id, &bidder, new_bids);

		Ok(())
	}

	/// Unbond some plmc from a contribution, after a step in the vesting period has passed.
	///
	/// # Arguments
	/// * bid: The bid to unbond from
	///
	/// # Storage access
	/// * [`Bids`] - Check if its time to unbond some plmc based on the bid vesting period, and update the bid after unbonding.
	/// * [`BiddingBonds`] - Update the bid with the new vesting period struct, reflecting this withdrawal
	/// * [`T::NativeCurrency`] - Unreserve the unbonded amount
	pub fn do_vested_plmc_purchase_unbond_for(
		releaser: AccountIdOf<T>, project_id: T::ProjectIdentifier, claimer: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let contributions = Contributions::<T>::get(project_id, &claimer);
		let now = <frame_system::Pallet<T>>::block_number();
		let mut updated_contributions = vec![];

		// * Validity checks *
		// TODO: PLMC-133. Check the right credential status
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful,
			Error::<T>::CannotClaimYet
		);
		// TODO: PLMC-160. Check the flow of the final_price if the final price discovery during the Auction Round fails

		for mut contribution in contributions {
			let mut plmc_vesting = contribution.plmc_vesting_period;
			let mut unbond_amount: BalanceOf<T> = 0u32.into();

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if plmc_vesting.next_withdrawal > now {
				continue;
			}

			// * Calculate variables *
			// Update vesting period until the next withdrawal is in the future
			while let Ok(amount) = plmc_vesting.calculate_next_withdrawal() {
				unbond_amount = unbond_amount.saturating_add(amount);
				if plmc_vesting.next_withdrawal > now {
					break;
				}
			}
			contribution.plmc_vesting_period = plmc_vesting;

			// * Update storage *
			// TODO: Should we mint here, or should the full mint happen to the treasury and then do transfers from there?
			// Unreserve the funds for the user
			T::NativeCurrency::release(
				&LockType::Participation(project_id),
				&claimer,
				unbond_amount,
				Precision::Exact,
			)?;
			updated_contributions.push(contribution);

			// * Emit events *
			Self::deposit_event(Event::BondReleased {
				project_id,
				amount: unbond_amount,
				bonder: claimer.clone(),
				releaser: releaser.clone(),
			})
		}

		// * Update storage *
		// TODO: PLMC-147. For now only the participants of the Community Round can claim their tokens
		// 	Obviously also the participants of the Auction Round should be able to claim their tokens
		// In theory this should never fail, since we insert the same number of contributions as before
		let updated_contributions: BoundedVec<ContributionInfoOf<T>, T::MaxContributionsPerUser> =
			updated_contributions
				.try_into()
				.map_err(|_| Error::<T>::TooManyContributions)?;
		Contributions::<T>::insert(project_id, &claimer, updated_contributions);

		Ok(())
	}

	/// Mint contribution tokens after a step in the vesting period for a contribution.
	///
	/// # Arguments
	/// * claimer: The account who made the contribution
	/// * project_id: The project the contribution was made for
	///
	/// # Storage access
	/// * [`ProjectsDetails`] - Check that the funding period ended
	/// * [`Contributions`] - Check if its time to mint some tokens based on the contributions vesting periods, and update the contribution after minting.
	/// * [`T::ContributionTokenCurrency`] - Mint the tokens to the claimer
	pub fn do_vested_contribution_token_purchase_mint_for(
		releaser: AccountIdOf<T>, project_id: T::ProjectIdentifier, claimer: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let contributions = Contributions::<T>::get(project_id, &claimer);
		let now = <frame_system::Pallet<T>>::block_number();
		let mut updated_contributions = vec![];

		// * Validity checks *
		// TODO: PLMC-133. Check the right credential status
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );
		ensure!(
			project_details.status == ProjectStatus::FundingSuccessful,
			Error::<T>::CannotClaimYet
		);
		// TODO: PLMC-160. Check the flow of the final_price if the final price discovery during the Auction Round fails

		for mut contribution in contributions {
			let mut ct_vesting = contribution.ct_vesting_period;
			let mut mint_amount: BalanceOf<T> = 0u32.into();

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if ct_vesting.next_withdrawal > now {
				continue;
			}

			// * Calculate variables *
			// Update vesting period until the next withdrawal is in the future
			while let Ok(amount) = ct_vesting.calculate_next_withdrawal() {
				mint_amount = mint_amount.saturating_add(amount);
				if ct_vesting.next_withdrawal > now {
					break;
				}
			}
			contribution.ct_vesting_period = ct_vesting;

			// * Update storage *
			// TODO: Should we mint here, or should the full mint happen to the treasury and then do transfers from there?
			// Mint the funds for the user
			T::ContributionTokenCurrency::mint_into(project_id, &claimer, mint_amount)?;
			updated_contributions.push(contribution);

			// * Emit events *
			Self::deposit_event(Event::ContributionTokenMinted {
				caller: releaser.clone(),
				project_id,
				contributor: claimer.clone(),
				amount: mint_amount,
			})
		}

		// * Update storage *
		// TODO: PLMC-147. For now only the participants of the Community Round can claim their tokens
		// 	Obviously also the participants of the Auction Round should be able to claim their tokens
		// In theory this should never fail, since we insert the same number of contributions as before
		let updated_contributions: BoundedVec<ContributionInfoOf<T>, T::MaxContributionsPerUser> =
			updated_contributions
				.try_into()
				.map_err(|_| Error::<T>::TooManyContributions)?;
		Contributions::<T>::insert(project_id, &claimer, updated_contributions);

		Ok(())
	}

	pub fn do_evaluation_unbond_for(
		releaser: AccountIdOf<T>, project_id: T::ProjectIdentifier, evaluator: AccountIdOf<T>,
		evaluation_id: T::StorageItemId,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let mut user_evaluations = Evaluations::<T>::get(project_id, evaluator.clone());
		let evaluation_position = user_evaluations
			.iter()
			.position(|evaluation| evaluation.id == evaluation_id)
			.ok_or(Error::<T>::EvaluationNotFound)?;
		let released_evaluation = user_evaluations.swap_remove(evaluation_position);

		// * Validity checks *
		ensure!(
			released_evaluation.rewarded_or_slashed == true
				&& matches!(
					project_details.status,
					ProjectStatus::EvaluationFailed | ProjectStatus::FundingFailed | ProjectStatus::FundingSuccessful
				),
			Error::<T>::NotAllowed
		);

		// * Update Storage *
		T::NativeCurrency::release(
			&LockType::Evaluation(project_id),
			&evaluator,
			released_evaluation.current_plmc_bond,
			Precision::Exact,
		)?;
		Evaluations::<T>::set(project_id, evaluator.clone(), user_evaluations);

		// * Emit events *
		Self::deposit_event(Event::<T>::BondReleased {
			project_id,
			amount: released_evaluation.current_plmc_bond,
			bonder: evaluator,
			releaser,
		});

		Ok(())
	}

	pub fn do_evaluation_reward_or_slash(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, evaluator: AccountIdOf<T>,
		evaluation_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_release_bid_funds_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, bidder: AccountIdOf<T>, bid_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_bid_unbond_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, bidder: AccountIdOf<T>, bid_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_release_contribution_funds_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, contributor: AccountIdOf<T>,
		contribution_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_contribution_unbond_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, contributor: AccountIdOf<T>,
		contribution_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_payout_contribution_funds_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, contributor: AccountIdOf<T>,
		contribution_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
		Ok(())
	}

	pub fn do_payout_bid_funds_for(
		caller: AccountIdOf<T>, project_id: T::ProjectIdentifier, bidder: AccountIdOf<T>, bid_id: StorageItemIdOf<T>,
	) -> Result<(), DispatchError> {
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
		while ProjectsToUpdate::<T>::try_append(block_number, store).is_err() {
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

	/// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_periods(
		_caller: AccountIdOf<T>, multiplier: MultiplierOf<T>, token_amount: BalanceOf<T>, token_price: T::Price,
	) -> Result<
		(
			Vesting<T::BlockNumber, BalanceOf<T>>,
			Vesting<T::BlockNumber, BalanceOf<T>>,
		),
		DispatchError,
	> {
		let plmc_start: T::BlockNumber = 0u32.into();
		let ct_start: T::BlockNumber = (T::MaxProjectsToUpdatePerBlock::get() * 7).into();
		// TODO: Calculate real vesting periods based on multiplier and caller type
		// FIXME: if divide fails, we probably dont want to assume the multiplier is one
		let ticket_size = token_price.checked_mul_int(token_amount).ok_or(Error::<T>::BadMath)?;
		let usd_bonding_amount = multiplier
			.calculate_bonding_requirement(ticket_size)
			.map_err(|_| Error::<T>::BadMath)?;
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		let plmc_bonding_amount = plmc_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_bonding_amount)
			.ok_or(Error::<T>::BadMath)?;
		Ok((
			Vesting {
				amount: plmc_bonding_amount,
				start: plmc_start,
				end: plmc_start,
				step: 0u32.into(),
				next_withdrawal: 0u32.into(),
			},
			Vesting {
				amount: token_amount,
				start: ct_start,
				end: ct_start,
				step: 0u32.into(),
				next_withdrawal: 0u32.into(),
			},
		))
	}

	/// Calculates the price (in USD) of contribution tokens for the Community and Remainder Rounds
	pub fn calculate_weighted_average_price(
		project_id: T::ProjectIdentifier, end_block: T::BlockNumber, total_allocation_size: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// Get all the bids that were made before the end of the candle
		let mut bids = Bids::<T>::iter_prefix(project_id)
			.flat_map(|(_bidder, bids)| bids)
			.collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_token_amount_sum = BalanceOf::<T>::zero();
		// temp variable to store the total value of the bids (i.e price * amount)
		let mut bid_usd_value_sum = BalanceOf::<T>::zero();
		let project_account = Self::fund_account_id(project_id);
		let plmc_price = T::PriceProvider::get_price(PLMC_STATEMINT_ID).ok_or(Error::<T>::PLMCPriceNotAvailable)?;
		// sort bids by price
		bids.sort();
		// accept only bids that were made before `end_block` i.e end of candle auction
		let bids: Result<Vec<_>, DispatchError> = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					bid.status = BidStatus::Rejected(RejectionReason::AfterCandleEnd);
					// TODO: PLMC-147. Unlock funds. We can do this inside the "on_idle" hook, and change the `status` of the `Bid` to "Unreserved"
					return Ok(bid);
				}
				let buyable_amount = total_allocation_size.saturating_sub(bid_token_amount_sum);
				if buyable_amount == 0_u32.into() {
					bid.status = BidStatus::Rejected(RejectionReason::NoTokensLeft);
				} else if bid.original_ct_amount <= buyable_amount {
					let maybe_ticket_size = bid.original_ct_usd_price.checked_mul_int(bid.original_ct_amount);
					if let Some(ticket_size) = maybe_ticket_size {
						bid_token_amount_sum.saturating_accrue(bid.original_ct_amount);
						bid_usd_value_sum.saturating_accrue(ticket_size);
						bid.status = BidStatus::Accepted;
					} else {
						bid.status = BidStatus::Rejected(RejectionReason::BadMath);
						return Ok(bid);
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
							bid.funding_asset_amount_locked
								.saturating_sub(funding_asset_amount_needed),
							Preservation::Preserve,
						)?;

						let usd_bond_needed = bid
							.multiplier
							.calculate_bonding_requirement(ticket_size)
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

						return Ok(bid);
					}

					// TODO: PLMC-147. Refund remaining amount
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
			// TODO: PLMC-150. collecting due to previous mut borrow, find a way to not collect and borrow bid on filter_map
			.iter()
			.filter_map(|bid| match bid.status {
				BidStatus::Accepted => {
					let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(
						bid.original_ct_usd_price.saturating_mul_int(bid.original_ct_amount), bid_usd_value_sum
					);
					let weighted_price = bid.original_ct_usd_price * bid_weight;
					Some(weighted_price)
				},

				BidStatus::PartiallyAccepted(amount, _) => {
					let bid_weight = <T::Price as FixedPointNumber>::saturating_from_rational(
						bid.original_ct_usd_price.saturating_mul_int(amount), bid_usd_value_sum
					);
					Some(bid.original_ct_usd_price.saturating_mul(bid_weight))
				},

				_ => None,
			})
			.reduce(|a, b| a.saturating_add(b))
			.ok_or(Error::<T>::NoBidsFound)?;

		let mut final_total_funding_reached_by_bids = BalanceOf::<T>::zero();
		// Update the bid in the storage
		for bid in bids.into_iter() {
			Bids::<T>::mutate(project_id, bid.bidder.clone(), |bids| -> Result<(), DispatchError> {
				let bid_index = bids
					.clone()
					.into_iter()
					.position(|b| b.id == bid.id)
					.ok_or(Error::<T>::ImpossibleState)?;
				let mut final_bid = bid;

				if final_bid.final_ct_usd_price > weighted_token_price {
					final_bid.final_ct_usd_price = weighted_token_price;
					let new_ticket_size = weighted_token_price
						.checked_mul_int(final_bid.final_ct_amount)
						.ok_or(Error::<T>::BadMath)?;

					let funding_asset_price = T::PriceProvider::get_price(final_bid.funding_asset.to_statemint_id())
						.ok_or(Error::<T>::PriceNotFound)?;
					let funding_asset_amount_needed = funding_asset_price
						.reciprocal()
						.ok_or(Error::<T>::BadMath)?
						.checked_mul_int(new_ticket_size)
						.ok_or(Error::<T>::BadMath)?;

					let try_transfer = T::FundingCurrency::transfer(
						final_bid.funding_asset.to_statemint_id(),
						&project_account,
						&final_bid.bidder,
						final_bid
							.funding_asset_amount_locked
							.saturating_sub(funding_asset_amount_needed),
						Preservation::Preserve,
					);
					if let Err(e) = try_transfer {
						Self::deposit_event(Event::<T>::TransferError { error: e });
					}

					final_bid.funding_asset_amount_locked = funding_asset_amount_needed;

					let usd_bond_needed = final_bid
						.multiplier
						.calculate_bonding_requirement(new_ticket_size)
						.map_err(|_| Error::<T>::BadMath)?;
					let plmc_bond_needed = plmc_price
						.reciprocal()
						.ok_or(Error::<T>::BadMath)?
						.checked_mul_int(usd_bond_needed)
						.ok_or(Error::<T>::BadMath)?;

					let try_release = T::NativeCurrency::release(
						&LockType::Participation(project_id),
						&final_bid.bidder,
						final_bid.plmc_bond.saturating_sub(plmc_bond_needed),
						Precision::Exact,
					);
					if let Err(e) = try_release {
						Self::deposit_event(Event::<T>::TransferError { error: e });
					}

					final_bid.plmc_bond = plmc_bond_needed;
				}
				let final_ticket_size = final_bid.final_ct_usd_price
					.checked_mul_int(final_bid.final_ct_amount)
					.ok_or(Error::<T>::BadMath)?;
				final_total_funding_reached_by_bids += final_ticket_size;
				bids[bid_index] = final_bid;
				Ok(())
			})?;
		}

		// Update storage
		ProjectsDetails::<T>::mutate(project_id, |maybe_info| -> Result<(), DispatchError> {
			if let Some(info) = maybe_info {
				info.weighted_average_price = Some(weighted_token_price);
				info.remaining_contribution_tokens =
					info.remaining_contribution_tokens.saturating_sub(bid_token_amount_sum);
				info.funding_amount_reached = info.funding_amount_reached.saturating_add(final_total_funding_reached_by_bids);
				Ok(())
			} else {
				Err(Error::<T>::ProjectNotFound.into())
			}
		})?;

		Ok(())
	}

	pub fn select_random_block(
		candle_starting_block: T::BlockNumber, candle_ending_block: T::BlockNumber,
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
		contribution_amount: BalanceOf<T>, weighted_average_price: BalanceOf<T>,
	) -> FixedU128 {
		FixedU128::saturating_from_rational(contribution_amount, weighted_average_price)
	}

	pub fn add_decimals_to_number(number: BalanceOf<T>, decimals: u8) -> BalanceOf<T> {
		let zeroes: BalanceOf<T> = BalanceOf::<T>::from(10u64).saturating_pow(decimals.into());
		number.saturating_mul(zeroes)
	}

	pub fn try_plmc_participation_lock(
		who: &T::AccountId, project_id: T::ProjectIdentifier, amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// Check if the user has already locked tokens in the evaluation period
		let evaluation_bonded = <T as Config>::NativeCurrency::balance_on_hold(&LockType::Evaluation(project_id), who);

		let new_amount_to_lock = amount.saturating_sub(evaluation_bonded);
		let evaluation_bonded_to_change_lock = amount.saturating_sub(new_amount_to_lock);

		T::NativeCurrency::release(
			&LockType::Evaluation(project_id),
			who,
			evaluation_bonded_to_change_lock,
			Precision::Exact,
		)
		.map_err(|_| Error::<T>::ImpossibleState)?;

		T::NativeCurrency::hold(&LockType::Participation(project_id), who, amount)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId, project_id: T::ProjectIdentifier, amount: BalanceOf<T>, asset_id: AssetIdOf<T>,
	) -> Result<(), DispatchError> {
		let fund_account = Self::fund_account_id(project_id);

		T::FundingCurrency::transfer(asset_id, &who, &fund_account, amount, Preservation::Expendable)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn release_last_funding_item_in_vec<I, M>(
		who: &T::AccountId, project_id: T::ProjectIdentifier, asset_id: AssetIdOf<T>, vec: &mut BoundedVec<I, M>,
		plmc_getter: impl Fn(&I) -> BalanceOf<T>, funding_asset_getter: impl Fn(&I) -> BalanceOf<T>,
	) -> Result<(), DispatchError> {
		let fund_account = Self::fund_account_id(project_id);
		let last_item = vec.swap_remove(vec.len() - 1);
		let plmc_amount = plmc_getter(&last_item);
		let funding_asset_amount = funding_asset_getter(&last_item);

		T::NativeCurrency::release(
			&LockType::Participation(project_id),
			&who,
			plmc_amount,
			Precision::Exact,
		)?;

		T::FundingCurrency::transfer(
			asset_id,
			&fund_account,
			&who,
			funding_asset_amount,
			Preservation::Expendable,
		)?;

		Ok(())
	}

	pub fn calculate_fees(project_id: T::ProjectIdentifier) -> Result<BalanceOf<T>, DispatchError> {
		let funding_reached = ProjectsDetails::<T>::get(project_id)
			.ok_or(Error::<T>::ProjectNotFound)?
			.funding_amount_reached;
		let mut remaining_for_fee = funding_reached;

		Ok(T::FeeBrackets::get()
			.into_iter()
			.map(|(fee, limit)| {
				let try_operation = remaining_for_fee.checked_sub(&limit);
				if let Some(remaining_amount) = try_operation {
					remaining_for_fee = remaining_amount;
					fee * limit
				} else {
					let temp = remaining_for_fee;
					remaining_for_fee = BalanceOf::<T>::zero();
					fee * temp
				}
			})
			.fold(BalanceOf::<T>::zero(), |acc, fee| acc.saturating_add(fee)))
	}

	pub fn get_evaluator_ct_rewards(
		project_id: T::ProjectIdentifier,
	) -> Result<Vec<(T::AccountId, T::Balance)>, DispatchError> {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let evaluation_usd_amounts = Evaluations::<T>::iter_prefix(project_id)
			.map(|(evaluator, evaluations)| {
				(
					evaluator,
					evaluations.into_iter().fold(
						(BalanceOf::<T>::zero(), BalanceOf::<T>::zero()),
						|acc, evaluation| {
							(
								acc.0.saturating_add(evaluation.early_usd_amount),
								acc.1.saturating_add(evaluation.late_usd_amount),
							)
						},
					),
				)
			})
			.collect::<Vec<_>>();
		let ct_price = project_details
			.weighted_average_price
			.ok_or(Error::<T>::ImpossibleState)?;
		let target_funding = project_details.fundraising_target;
		let funding_reached = project_details.funding_amount_reached;

		// This is the "Y" variable from the knowledge hub
		let percentage_of_target_funding = Perbill::from_rational(funding_reached, target_funding);

		let fees = Self::calculate_fees(project_id)?;
		let evaluator_fees = percentage_of_target_funding * (Perbill::from_percent(30) * fees);

		let early_evaluator_rewards = Perbill::from_percent(20) * evaluator_fees;
		let all_evaluator_rewards = Perbill::from_percent(80) * evaluator_fees;

		let early_evaluator_total_locked = evaluation_usd_amounts
			.iter()
			.fold(BalanceOf::<T>::zero(), |acc, (_, (early, _))| {
				acc.saturating_add(*early)
			});
		let late_evaluator_total_locked = evaluation_usd_amounts
			.iter()
			.fold(BalanceOf::<T>::zero(), |acc, (_, (_, late))| acc.saturating_add(*late));
		let all_evaluator_total_locked = early_evaluator_total_locked.saturating_add(late_evaluator_total_locked);

		let evaluator_usd_rewards = evaluation_usd_amounts
			.into_iter()
			.map(|(evaluator, (early, late))| {
				let early_evaluator_weight = Perbill::from_rational(early, early_evaluator_total_locked);
				let all_evaluator_weight = Perbill::from_rational(early + late, all_evaluator_total_locked);

				let early_reward = early_evaluator_weight * early_evaluator_rewards;
				let all_reward = all_evaluator_weight * all_evaluator_rewards;

				(evaluator, early_reward.saturating_add(all_reward))
			})
			.collect::<Vec<_>>();
		let ct_price_reciprocal = ct_price.reciprocal().ok_or(Error::<T>::BadMath)?;

		evaluator_usd_rewards
			.iter()
			.map(|(evaluator, usd_reward)| {
				if let Some(reward) = ct_price_reciprocal.checked_mul_int(*usd_reward) {
					Ok((evaluator.clone(), reward))
				} else {
					Err(Error::<T>::BadMath.into())
				}
			})
			.collect()
	}
}
