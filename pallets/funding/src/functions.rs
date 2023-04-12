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

use crate::ProjectStatus::EvaluationRound;
use frame_support::{ensure, pallet_prelude::DispatchError, traits::Get};
use sp_arithmetic::{traits::Zero, Perbill};
use sp_runtime::{DispatchResult, Percent};
use sp_std::prelude::*;

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
	/// * `Projects` - Inserting the main project information. 1 to 1 with the `project` argument.
	/// * `ProjectsInfo` - Inserting the project information. constructed from the `project` argument.
	/// * `ProjectsIssuers` - Inserting the issuer of the project. Mapping of the two parameters `project_id` and `issuer`.
	/// * `NextProjectId` - Getting the next usable id, and updating it for the next project.
	///
	/// # Success path
	/// The `project` argument is valid. A ProjectInfo struct is constructed, and the storage is updated
	/// with the new structs and mappings to reflect the new project creation
	///
	/// # Next step
	/// The issuer will call an extrinsic to start the evaluation round of the project.
	/// `do_evaluation_start` will be executed.
	pub fn do_create(issuer: T::AccountId, project: ProjectOf<T>) -> Result<(), DispatchError> {
		// TODO: Probably the issuers don't want to sell all of their tokens. Is there some logic for this?
		// 	also even if an issuer wants to sell all their tokens, they could target a lower amount than that to consider it a success
		// * Get variables *
		let fundraising_target = project.total_allocation_size * project.minimum_price;
		let project_id = NextProjectId::<T>::get();

		// * Validity checks *
		ensure!(Images::<T>::contains_key(project.metadata), Error::<T>::NoImageFound);
		if let Err(error) = project.validity_check() {
			return match error {
				ValidityError::PriceTooLow => Err(Error::<T>::PriceTooLow.into()),
				ValidityError::ParticipantsSizeError =>
					Err(Error::<T>::ParticipantsSizeError.into()),
				ValidityError::TicketSizeError => Err(Error::<T>::TicketSizeError.into()),
			}
		}

		// * Calculate new variables *
		let project_info = ProjectInfo {
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			project_status: ProjectStatus::Application,
			phase_transition_points: PhaseTransitionPoints {
				application: BlockNumberPair::new(
					Some(<frame_system::Pallet<T>>::block_number()),
					None,
				),
				evaluation: BlockNumberPair::new(None, None),
				auction_initialize_period: BlockNumberPair::new(None, None),
				english_auction: BlockNumberPair::new(None, None),
				random_candle_ending: None,
				candle_auction: BlockNumberPair::new(None, None),
				community: BlockNumberPair::new(None, None),
				remainder: BlockNumberPair::new(None, None),
			},
		};

		// * Update storage *
		Projects::<T>::insert(project_id, project);
		ProjectsInfo::<T>::insert(project_id, project_info);
		ProjectsIssuers::<T>::insert(project_id, issuer);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());

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
	/// * `ProjectsInfo` - Checking and updating the round status, transition points and freezing the project.
	/// * `ProjectsToUpdate` - Scheduling the project for automatic transition by on_initialize later on.
	///
	/// # Success path
	/// The project information is found, its round status was in Application round, and It's not yet frozen.
	/// The pertinent project info is updated on the storage, and the project is scheduled for automatic transition by on_initialize.
	///
	/// # Next step
	/// Users will pond PLMC for this project, and when the time comes, the project will be transitioned
	/// to the next round by `on_initialize` using `do_evaluation_end`
	pub fn do_evaluation_start(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(
			project_info.project_status == ProjectStatus::Application,
			Error::<T>::ProjectNotInApplicationRound
		);
		ensure!(!project_info.is_frozen, Error::<T>::ProjectAlreadyFrozen);

		// * Calculate new variables *
		let evaluation_end_block = now + T::EvaluationDuration::get();

		// * Update storage *
		// TODO: Should we make it possible to end an application, and schedule for a later point the evaluation?
		// 	Or should we just make it so that the evaluation starts immediately after the application ends?
		project_info.phase_transition_points.application.update(None, Some(now));
		project_info
			.phase_transition_points
			.evaluation
			.update(Some(now + 1u32.into()), Some(evaluation_end_block));
		project_info.is_frozen = true;
		project_info.project_status = EvaluationRound;
		ProjectsInfo::<T>::insert(project_id, project_info);
		Self::add_to_update_store(evaluation_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

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
	/// * `ProjectsInfo` - Checking the round status and transition points for validity, and updating
	/// the round status and transition points in case of success or failure of the evaluation.
	/// * `EvaluationBonds` - Checking that the threshold for PLMC bonded was reached, to decide
	/// whether the project failed or succeeded.
	///
	/// # Success path
	/// The project information is found, its round status was in Evaluation round,
	/// the current block is after the defined end block of the evaluation round.
	///
	/// 2 Possible paths.
	///
	/// * Project achieves its evaluation goal - 10% of the target funding was reached through bonding,
	/// so the project is transitioned to the `AuctionInitializePeriod` round. The project information
	/// is updated with the new transition points and round status.
	///
	/// * Project doesn't reach the evaluation goal - less than 10% of the target funding was reached
	/// through bonding, so the project is transitioned to the `EvaluationFailed` round. The project
	/// information is updated with the new rounds status and it is scheduled for automatic unbonding.
	///
	/// # Next step
	/// * Bonding achieved - The issuer calls an extrinsic within the set period to initialize the
	/// auction round. `do_english_auction` is called
	///
	/// * Bonding failed - `on_idle` at some point checks for failed evaluation projects, and
	/// unbonds the evaluators funds.
	pub fn do_evaluation_end(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_end_block = project_info
			.phase_transition_points
			.evaluation
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target = project_info.fundraising_target;

		// * Validity checks *
		ensure!(
			project_info.project_status == EvaluationRound,
			Error::<T>::ProjectNotInEvaluationRound
		);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// * Calculate new variables *
		let initial_balance: BalanceOf<T> = 0u32.into();
		let total_amount_bonded = EvaluationBonds::<T>::iter_prefix(project_id)
			.fold(initial_balance, |acc, (_, bond)| acc.saturating_add(bond.amount));
		// Check if the total amount bonded is greater than the 10% of the fundraising target
		// TODO: PLMC-142. 10% is hardcoded, check if we want to configure it a runtime as explained here:
		// 	https://substrate.stackexchange.com/questions/2784/how-to-get-a-percent-portion-of-a-balance:
		// TODO: PLMC-143. Check if it's safe to use * here
		let evaluation_target = Percent::from_percent(10) * fundraising_target;
		let auction_initialize_period_start_block = now + 1u32.into();
		let auction_initialize_period_end_block =
			auction_initialize_period_start_block + T::AuctionInitializePeriodDuration::get();
		// Check which logic path to follow
		let is_funded = total_amount_bonded >= evaluation_target;

		// * Branch in possible project paths *
		// Successful path
		if is_funded {
			// * Update storage *
			project_info.phase_transition_points.auction_initialize_period.update(
				Some(auction_initialize_period_start_block),
				Some(auction_initialize_period_end_block),
			);
			project_info.project_status = ProjectStatus::AuctionInitializePeriod;
			ProjectsInfo::<T>::insert(project_id, project_info);

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
			project_info.project_status = ProjectStatus::EvaluationFailed;
			ProjectsInfo::<T>::insert(project_id, project_info);
			// Schedule project for processing in on_initialize
			Self::add_to_update_store(now + 1u32.into(), &project_id)
				.expect("Always returns Ok; qed");

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
	/// * `ProjectsInfo` - Get the project information, and check if the project is in the correct
	/// round, and the current block is between the defined start and end blocks of the initialize period.
	/// Update the project information with the new round status and transition points in case of success.
	///
	/// # Success Path
	/// The validity checks pass, and the project is transitioned to the English Auction round.
	/// The project is scheduled to be transitioned automatically by `on_initialize` at the end of the
	/// english auction round.
	///
	/// # Next step
	/// Professional and Institutional users set bids for the project using the `bid` extrinsic.
	/// Later on, `on_initialize` transitions the project into the candle auction round, by calling
	/// `do_candle_auction`.
	pub fn do_english_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_initialize_period_start_block = project_info
			.phase_transition_points
			.auction_initialize_period
			.start()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;
		let auction_initialize_period_end_block = project_info
			.phase_transition_points
			.auction_initialize_period
			.end()
			.ok_or(Error::<T>::EvaluationPeriodNotEnded)?;

		// * Validity checks *
		ensure!(
			now >= auction_initialize_period_start_block,
			Error::<T>::TooEarlyForEnglishAuctionStart
		);
		ensure!(
			now <= auction_initialize_period_end_block,
			Error::<T>::TooLateForEnglishAuctionStart
		);
		ensure!(
			project_info.project_status == ProjectStatus::AuctionInitializePeriod,
			Error::<T>::ProjectNotInAuctionInitializePeriodRound
		);

		// * Calculate new variables *
		let english_start_block = now + 1u32.into();
		let english_end_block = now + T::EnglishAuctionDuration::get();

		// * Update Storage *
		project_info
			.phase_transition_points
			.english_auction
			.update(Some(english_start_block), Some(english_end_block));
		project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
		ProjectsInfo::<T>::insert(project_id, project_info);
		// Schedule for automatic transition to candle auction round
		Self::add_to_update_store(english_end_block + 1u32.into(), &project_id)
			.expect("Always return Ok; qed");

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
	/// * `ProjectsInfo` - Get the project information, and check if the project is in the correct
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
	/// by calling `do_community_funding`.
	pub fn do_candle_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let english_end_block = project_info
			.phase_transition_points
			.english_auction
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > english_end_block, Error::<T>::TooEarlyForCandleAuctionStart);
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English),
			Error::<T>::ProjectNotInEnglishAuctionRound
		);

		// * Calculate new variables *
		let candle_start_block = now + 1u32.into();
		let candle_end_block = now + T::CandleAuctionDuration::get();

		// * Update Storage *
		project_info
			.phase_transition_points
			.candle_auction
			.update(Some(candle_start_block), Some(candle_end_block));
		project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
		ProjectsInfo::<T>::insert(project_id, project_info);
		// Schedule for automatic check by on_initialize. Success depending on enough funding reached
		Self::add_to_update_store(candle_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		// * Emit events *
		Self::deposit_event(Event::<T>::CandleAuctionStarted { project_id, when: now });

		Ok(())
	}

	/// Called automatically by on_initialize
	/// Starts the community round for a project.
	/// Retail users now buy tokens instead of bid on them. The price of the tokens are calculated
	/// based on the available bids, using the function `calculate_weighted_average_price`.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * `ProjectsInfo` - Get the project information, and check if the project is in the correct
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
	/// Later on, `on_initialize` ends the community round by calling `do_remainder_funding` and
	/// starts the remainder round, where anyone can buy at that price point.
	pub fn do_community_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let auction_candle_start_block = project_info
			.phase_transition_points
			.candle_auction
			.start()
			.ok_or(Error::<T>::FieldIsNone)?;
		let auction_candle_end_block = project_info
			.phase_transition_points
			.candle_auction
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > auction_candle_end_block, Error::<T>::TooEarlyForCommunityRoundStart);
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle),
			Error::<T>::ProjectNotInCandleAuctionRound
		);

		// * Calculate new variables *
		let end_block =
			Self::select_random_block(auction_candle_start_block, auction_candle_end_block);
		let community_start_block = now + 1u32.into();
		let community_end_block = now + T::CommunityFundingDuration::get();

		// * Update Storage *
		project_info.phase_transition_points.random_candle_ending = Some(end_block);
		project_info
			.phase_transition_points
			.community
			.update(Some(community_start_block), Some(community_end_block));
		project_info.project_status = ProjectStatus::CommunityRound;
		project_info.weighted_average_price = Some(Self::calculate_weighted_average_price(
			project_id,
			end_block,
			project_info.fundraising_target,
		)?);
		ProjectsInfo::<T>::insert(project_id, project_info);
		// Schedule for automatic transition by `on_initialize`
		Self::add_to_update_store(community_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		// * Emit events *
		Self::deposit_event(Event::<T>::CommunityFundingStarted { project_id });
		Ok(())
	}

	/// Called automatically by on_initialize
	/// Starts the remainder round for a project.
	/// Anyone can now buy tokens, until they are all sold out, or the time is reached.
	///
	/// # Arguments
	/// * `project_id` - The project identifier
	///
	/// # Storage access
	/// * `ProjectsInfo` - Get the project information, and check if the project is in the correct
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
	/// `do_end_funding`
	pub fn do_remainder_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block = project_info
			.phase_transition_points
			.community
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// * Validity checks *
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(
			project_info.project_status == ProjectStatus::CommunityRound,
			Error::<T>::ProjectNotInCommunityRound
		);

		// * Calculate new variables *
		let remainder_start_block = now + 1u32.into();
		let remainder_end_block = now + T::RemainderFundingDuration::get();

		// * Update Storage *
		project_info
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block));
		project_info.project_status = ProjectStatus::RemainderRound;
		ProjectsInfo::<T>::insert(project_id, project_info);
		// Schedule for automatic transition by `on_initialize`
		Self::add_to_update_store(remainder_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

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
	/// * `ProjectsInfo` - Get the project information, and check if the project is in the correct
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
	/// If successful, users can claim their contribution tokens and bonded plmc when the time is right.
	/// If unsuccessful, users can claim their bonded plmc and unlock their reserved bidding currency immediately
	pub fn do_end_funding(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let remainder_end_block = project_info
			.phase_transition_points
			.remainder
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;
		// TODO: PLMC-149 Check if make sense to set the admin as T::fund_account_id(project_id)
		let issuer =
			ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectIssuerNotFound)?;
		let project = Projects::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let token_information = project.token_information;

		// * Validity checks *
		ensure!(now > remainder_end_block, Error::<T>::TooEarlyForFundingEnd);
		ensure!(
			project_info.project_status == ProjectStatus::RemainderRound,
			Error::<T>::ProjectNotInRemainderRound
		);

		// * Calculate new variables *
		project_info.project_status = ProjectStatus::FundingEnded;
		ProjectsInfo::<T>::insert(project_id, project_info);

		// * Update Storage *
		// Create the "Contribution Token" as an asset using the pallet_assets and set its metadata
		T::Assets::create(project_id.clone(), issuer.clone(), false, 1_u32.into())
			.map_err(|_| Error::<T>::AssetCreationFailed)?;
		// Update the CT metadata
		T::Assets::set(
			project_id.clone(),
			&issuer,
			token_information.name.into(),
			token_information.symbol.into(),
			token_information.decimals,
		)
		.map_err(|_| Error::<T>::AssetMetadataUpdateFailed)?;

		// * Emit events *
		Self::deposit_event(Event::FundingEnded { project_id: project_id.clone() });
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
	/// * `ProjectsInfo` - Check that the funding round ended, and update the status to ReadyToLaunch
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
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		ensure!(
			project_info.project_status == ProjectStatus::FundingEnded,
			Error::<T>::ProjectNotInFundingEndedRound
		);

		// Update project Info
		project_info.project_status = ProjectStatus::ReadyToLaunch;
		ProjectsInfo::<T>::insert(project_id, project_info);

		Ok(())
	}
}

// Extrinsic functions (except round transitions)
impl<T: Config> Pallet<T> {
	/// Store an image's hash on chain, to be retrievable with IPFS
	///
	/// # Arguments
	/// * `preimage` - The image bytes
	/// * `issuer` - The project issuer account
	///
	/// # Storage access
	/// * `Images` - Insert the hash of the image as key and issuer as value
	pub fn do_note_bytes(
		preimage: BoundedVec<u8, T::PreImageLimit>,
		issuer: &T::AccountId,
	) -> Result<(), DispatchError> {
		// * Get Variables *

		// * Validity checks *
		// TODO: PLMC-141. Validate and check if the preimage is a valid JSON conforming with our needs.
		// 	also check if we can use serde in a no_std environment

		// * Calculate new variables *
		let hash = T::Hashing::hash(&preimage);

		// * Update Storage *
		Images::<T>::insert(hash, issuer);

		// * Emit events *
		Self::deposit_event(Event::Noted { hash });

		Ok(())
	}

	/// Change the metadata of a project to the hash of the image that was previously stored with the `note_bytes` extrinsic
	///
	/// # Arguments
	/// * `issuer` - The project issuer account
	/// * `project_id` - The project identifier
	/// * `project_metadata_hash` - The hash of the image that contains the metadata
	///
	/// # Storage access
	/// * `ProjectsIssuers` - Check that the issuer is the owner of the project
	/// * `Images` - Check that the image exists
	/// * `ProjectsInfo` - Check that the project is not frozen
	/// * `Projects` - Update the metadata hash
	pub fn do_edit_metadata(issuer: T::AccountId, project_id: T::ProjectIdentifier, project_metadata_hash: T::Hash) -> Result<(), DispatchError> {
		// * Get variables *

		// * Validity checks *
		ensure!(ProjectsIssuers::<T>::get(project_id) == Some(issuer), Error::<T>::NotAllowed);
		ensure!(Images::<T>::contains_key(project_metadata_hash), Error::<T>::NoImageFound);
		ensure!(
				!ProjectsInfo::<T>::get(project_id)
					.ok_or(Error::<T>::ProjectInfoNotFound)?
					.is_frozen,
				Error::<T>::Frozen
			);
		// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );

		// * Calculate new variables *

		// * Update Storage *
		Projects::<T>::try_mutate(project_id, |maybe_project| -> DispatchResult {
			let project = maybe_project.as_mut().ok_or(Error::<T>::ProjectIssuerNotFound)?;
			project.metadata = project_metadata_hash;
			Self::deposit_event(Event::MetadataEdited { project_id });
			Ok(())
		})

		// * Emit events *

	}

	/// Bond PLMC for a project in the evaluation stage
	///
	/// # Arguments
	/// * `evaluator` - The account to which the PLMC will be bonded
	/// * `project_id` - The project to bond to
	/// * `amount` - The amount of PLMC to bond
	///
	/// # Storage access
	/// * `ProjectsIssuers` - Check that the evaluator is not the project issuer
	/// * `ProjectsInfo` - Check that the project is in the evaluation stage
	/// * `EvaluationBonds` - Update the storage with the evaluators bond, by either increasing an existing
	/// one, or appending a new bond
	pub fn do_evaluation_bond(
		evaluator: T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_issuer =
			ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectIssuerNotFound)?;
		let project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// * Validity checks *
		// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );
		ensure!(evaluator != project_issuer, Error::<T>::ContributionToThemselves);
		ensure!(
			project_info.project_status == ProjectStatus::EvaluationRound,
			Error::<T>::EvaluationNotStarted
		);

		// * Calculate new variables *

		// * Update Storage *
		// TODO: PLMC-144. Unlock the PLMC when it's the right time
		EvaluationBonds::<T>::try_mutate(project_id, evaluator.clone(), |maybe_bond| {
			match maybe_bond {
				Some(bond) => {
					// If the user has already bonded, add the new amount to the old one
					bond.amount += amount;
					T::Currency::reserve_named(&BondType::Evaluation, &evaluator, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
				None => {
					// If the user has not bonded yet, create a new bond
					*maybe_bond = Some(EvaluationBond {
						project: project_id,
						account: evaluator.clone(),
						amount,
						when: <frame_system::Pallet<T>>::block_number(),
					});

					// Reserve the required PLMC
					T::Currency::reserve_named(&BondType::Evaluation, &evaluator, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
			}
			Self::deposit_event(Event::<T>::FundsBonded {
				project_id,
				amount,
				bonder: evaluator.clone(),
			});
			Result::<(), Error<T>>::Ok(())
		})?;

		// * Emit events *

		Ok(())
	}

	/// Unbond the PLMC of an evaluator for a project that failed the evaluation stage
	///
	/// # Arguments
	/// * `bond` - The bond struct containing the information about the funds to unbond
	/// * `releaser` - The account that is releasing the funds, which will be shown in the event emitted
	///
	/// # Storage access
	/// * `ProjectsInfo` - Check that the project is in the evaluation failed stage
	/// * `EvaluationBonds` - Remove the bond from storage
	pub fn do_failed_evaluation_unbond_for(
		bond: EvaluationBond<
			T::ProjectIdentifier,
			T::AccountId,
			T::CurrencyBalance,
			T::BlockNumber,
		>,
		releaser: T::AccountId,
	) -> Result<(), DispatchError> {
		let project_info =
			ProjectsInfo::<T>::get(bond.project.clone()).ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(
			project_info.project_status == ProjectStatus::EvaluationFailed,
			Error::<T>::EvaluationNotFailed
		);
		T::Currency::unreserve_named(&BondType::Evaluation, &bond.account, bond.amount.clone());
		EvaluationBonds::<T>::remove(bond.project.clone(), bond.account.clone());

		Self::deposit_event(Event::<T>::BondReleased {
			project_id: bond.project,
			amount: bond.amount,
			bonder: bond.account,
			releaser,
		});

		Ok(())
	}

	/// Bid for a project in the bidding stage
	///
	/// # Arguments
	/// * `bidder` - The account that is bidding
	/// * `project_id` - The project to bid for
	/// * `amount` - The amount of tokens that the bidder wants to buy
	/// * `price` - The price per token that the bidder is willing to pay for
	/// * `multiplier` - Used for calculating how much PLMC needs to be bonded to spend this much money (in USD)
	///
	/// # Storage access
	/// * `ProjectsIssuers` - Check that the bidder is not the project issuer
	/// * `ProjectsInfo` - Check that the project is in the bidding stage
	/// * `BiddingBonds` - Update the storage with the bidder's PLMC bond for that bid
	/// * `AuctionsInfo` - Check previous bids by that user, and update the storage with the new bid
	pub fn do_bid(
		bidder: T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
		price: BalanceOf<T>,
		multiplier: Option<u32>,
	) -> Result<(), DispatchError> {
		// * Get variables *
		let project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let project_issuer =
			ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectIssuerNotFound)?;
		let project = Projects::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let project_ticket_size = amount.saturating_mul(price);
		let now = <frame_system::Pallet<T>>::block_number();
		let multiplier = multiplier.unwrap_or(1_u32);

		// * Validity checks *
		ensure!(bidder != project_issuer, Error::<T>::ContributionToThemselves);
		ensure!(
			matches!(project_info.project_status, ProjectStatus::AuctionRound(_)),
			Error::<T>::AuctionNotStarted
		);
		ensure!(price >= project.minimum_price, Error::<T>::BidTooLow);
		if let Some(minimum_ticket_size) = project.ticket_size.minimum {
			// Make sure the bid amount is greater than the minimum specified by the issuer
			ensure!(project_ticket_size >= minimum_ticket_size, Error::<T>::BidTooLow);
		};
		if let Some(maximum_ticket_size) = project.ticket_size.maximum {
			// Make sure the bid amount is less than the maximum specified by the issuer
			ensure!(project_ticket_size <= maximum_ticket_size, Error::<T>::BidTooLow);
		};

		// * Calculate new variables *
		let mut required_plmc_bond =
			amount.checked_div(&multiplier.into()).ok_or(Error::<T>::BadMath)?;
		let mut bonded_plmc;
		let (plmc_vesting_period, ct_vesting_period) =
			Self::calculate_vesting_periods(bidder.clone(), multiplier, amount.clone());
		let bid = BidInfo::new(
			project_id.clone(),
			amount,
			price,
			now,
			bidder.clone(),
			plmc_vesting_period,
			ct_vesting_period,
		);
		// Check how much PLMC is already bonded for this project
		if let Some(bond) = BiddingBonds::<T>::get(project_id.clone(), bidder.clone()) {
			bonded_plmc = bond.amount;
		} else {
			bonded_plmc = Zero::zero();
		}
		let mut user_bids = AuctionsInfo::<T>::get(project_id, bidder.clone()).unwrap_or_default();
		// Check how much of the project-bonded PLMC is already in use by a bid
		for bid in user_bids.iter() {
			bonded_plmc.saturating_sub(bid.plmc_vesting_period.amount);
		}
		required_plmc_bond.saturating_sub(bonded_plmc);

		// * Update storage *
		// Try bonding the required PLMC for this bid
		Self::bond_bidding(bidder.clone(), project_id.clone(), required_plmc_bond)?;
		// Try adding the new bid to the system
		match user_bids.try_push(bid.clone()) {
			Ok(_) => {
				// Reserve the new bid
				T::BiddingCurrency::reserve(&bidder, bid.ticket_size)?;
				// TODO: PLMC-159. Send an XCM message to Statemint/e to transfer a `bid.market_cap` amount of USDC (or the Currency specified by the issuer) to the PalletId Account
				// Alternative TODO: PLMC-159. The user should have the specified currency (e.g: USDC) already on Polimec
				user_bids.sort_by_key(|bid| Reverse(bid.price));
				AuctionsInfo::<T>::set(project_id, bidder.clone(), Some(user_bids));
				Self::deposit_event(Event::<T>::Bid { project_id, amount, price, multiplier });
			},
			Err(_) => {
				// Since the bids are sorted by price, and in this branch the Vec is full, the last element is the lowest bid
				let lowest_bid_index: usize = (T::MaximumBidsPerUser::get() - 1)
					.try_into()
					.map_err(|_| Error::<T>::BadMath)?;
				let lowest_bid = user_bids.swap_remove(lowest_bid_index);
				ensure!(bid > lowest_bid, Error::<T>::BidTooLow);
				// Unreserve the lowest bid first
				T::BiddingCurrency::unreserve(&lowest_bid.bidder, lowest_bid.ticket_size);
				// Reserve the new bid
				T::BiddingCurrency::reserve(&bidder, bid.ticket_size)?;
				// Add the new bid to the AuctionsInfo, this should never fail since we just removed an element
				user_bids
					.try_push(bid)
					.expect("We removed an element, so there is always space");
				user_bids.sort_by_key(|bid| Reverse(bid.price));
				AuctionsInfo::<T>::set(project_id, bidder.clone(), Some(user_bids));
				// TODO: PLMC-159. Send an XCM message to Statemine to transfer amount * multiplier USDT to the PalletId Account
				Self::deposit_event(Event::<T>::Bid { project_id, amount, price, multiplier });
			},
		};

		// * Emit events *

		Ok(())
	}

	/// Buy tokens in the Community Round at the price set in the Bidding Round
	///
	/// # Arguments
	/// * contributor: The account that is buying the tokens
	/// * project_id: The identifier of the project
	/// * amount: The amount of tokens to buy
	///
	/// # Storage access
	/// * `ProjectsIssuers` - Check that the issuer is not a contributor
	/// * `ProjectsInfo` - Check that the project is in the Community Round, and the amount is big
	/// enough to buy at least 1 token
	/// * `Contributions` - Update storage with the new contribution
	/// * `T::Currency` - Update the balance of the contributor and the project pot
	pub fn do_contribute(contributor: T::AccountId, project_id: T::ProjectIdentifier, amount: BalanceOf<T>, multiplier: Option<u32>) -> Result<(), DispatchError> {
		// TODO: PLMC-103? Add the "Retail before, Institutional and Professionals after, if there are still tokens" logic

		// * Get variables *
		let project_issuer =
			ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectIssuerNotFound)?;
		let project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let multiplier = multiplier.unwrap_or(1u32);

		// * Validity checks *
		ensure!(contributor != project_issuer, Error::<T>::ContributionToThemselves);
		ensure!(
			project_info.project_status == ProjectStatus::CommunityRound,
			Error::<T>::AuctionNotStarted
		);
		ensure!(
			amount >=
				project_info
					.weighted_average_price
					.expect("This value exists in Community Round"),
			Error::<T>::BidTooLow
		);
		// TODO: PLMC-133. Replace this when this PR is merged: https://github.com/KILTprotocol/kilt-node/pull/448
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Retail, &contributor),
		// 	Error::<T>::NotAuthorized
		// );

		// * Calculate variables *
		let weighted_average_price = project_info
			.weighted_average_price
			.ok_or(Error::<T>::FieldIsNone)?;
		let fund_account = Self::fund_account_id(project_id);
		// TODO: PLMC-159. Use USDC on Statemint/e (via XCM) instead of PLMC
		// TODO: PLMC-157. Check the logic
		// TODO: PLMC-157. Check if we need to use T::Currency::resolve_creating(...)
		let mut required_plmc_bond =
			amount.checked_div(&multiplier.into()).ok_or(Error::<T>::BadMath)?;
		let ct_amount = amount.checked_div(&weighted_average_price).ok_or(Error::<T>::BadMath)?;
		let mut bonded_plmc;
		let (plmc_vesting, ct_vesting) =
			Self::calculate_vesting_periods(contributor.clone(), multiplier, ct_amount);
		let contribution = ContributionInfo {
			contribution_amount: amount.clone(),
			plmc_vesting,
			ct_vesting,
		};
		// Check how much PLMC is already bonded for this project
		if let Some(bond) = ContributingBonds::<T>::get(project_id.clone(), contributor.clone()) {
			bonded_plmc = bond.amount;
		} else {
			bonded_plmc = Zero::zero();
		}
		let mut user_contributions = Contributions::<T>::get(project_id, contributor.clone()).unwrap_or_default();
		// Check how much of the project-bonded PLMC is already in use by a contribution
		for contribution in user_contributions.iter() {
			bonded_plmc.saturating_sub(contribution.plmc_vesting.amount);
		}
		required_plmc_bond.saturating_sub(bonded_plmc);

		// * Update storage *
		// Try bonding the required PLMC for this contribution
		Self::bond_contributing(contributor.clone(), project_id.clone(), required_plmc_bond)?;

		// Try adding the new contribution to the system
		match user_contributions.try_push(contribution.clone()) {
			Ok(_) => {
				// TODO: PLMC-159. Send an XCM message to Statemint/e to transfer a `bid.market_cap` amount of USDC (or the Currency specified by the issuer) to the PalletId Account
				// Alternative TODO: PLMC-159. The user should have the specified currency (e.g: USDC) already on Polimec
				user_contributions.sort_by_key(|contribution| Reverse(contribution.plmc_vesting.amount));
				Contributions::<T>::set(project_id, contributor.clone(), Some(user_contributions));
			},
			Err(_) => {
				// The contributions are sorted by highest PLMC bond. If the contribution vector for the user is full, we drop the lowest/last item
				let lowest_contribution_index: usize = (T::MaxContributionsPerUser::get() - 1)
					.try_into()
					.map_err(|_| Error::<T>::BadMath)?;
				let lowest_contribution = user_contributions.swap_remove(lowest_contribution_index);
				ensure!(contribution.plmc_vesting.amount > lowest_contribution.plmc_vesting.amount, Error::<T>::ContributionTooLow);
				// Return contribution funds
				T::Currency::transfer(
					&fund_account,
					&contributor,
					lowest_contribution.contribution_amount,
					// TODO: PLMC-157. Take the ExistenceRequirement as parameter (?)
					frame_support::traits::ExistenceRequirement::KeepAlive,
				)?;
				// Add the new bid to the AuctionsInfo, this should never fail since we just removed an element
				user_contributions
					.try_push(contribution)
					.expect("We removed an element, so there is always space");
				user_contributions.sort_by_key(|contribution| Reverse(contribution.plmc_vesting.amount));
				Contributions::<T>::set(project_id, contributor.clone(), Some(user_contributions));
				// TODO: PLMC-159. Send an XCM message to Statemine to transfer amount * multiplier USDT to the PalletId Account
			},
		};

		// Transfer funds from contributor to fund account
		T::Currency::transfer(
			&contributor,
			&fund_account,
			amount,
			// TODO: PLMC-157. Take the ExistenceRequirement as parameter (?)
			frame_support::traits::ExistenceRequirement::KeepAlive,
		)?;

		// * Emit events *
		Self::deposit_event(Event::<T>::Contribution { project_id, contributor: contributor.clone(), amount, multiplier });

		Ok(())
	}

	/// Unbond some plmc from a successful bid, after a step in the vesting period has passed.
	///
	/// # Arguments
	/// * bid: The bid to unbond from
	///
	/// # Storage access
	/// * `AuctionsInfo` - Check if its time to unbond some plmc based on the bid vesting period, and update the bid after unbonding.
	/// * `BiddingBonds` - Update the bid with the new vesting period struct, reflecting this withdrawal
	/// * `T::Currency` - Unreserve the unbonded amount
	pub fn do_vested_plmc_bid_unbond_for(bidder: T::AccountId, project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let bids = AuctionsInfo::<T>::get(project_id, &bidder).ok_or(Error::<T>::BidNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let mut new_bids = vec![];
		// let mut new_bids: BoundedVec<BidInfoOf<T>, T::MaximumBidsPerUser> = vec![].into();
		for mut bid in bids {
			let mut plmc_vesting = bid.plmc_vesting_period;

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if plmc_vesting.next_withdrawal > now {
				continue
			}

			// * Calculate variables *
			let mut unbond_amount: BalanceOf<T> = 0u32.into();
			// update vesting period until the next withdrawal is in the future
			while let Ok(amount) = plmc_vesting.calculate_next_withdrawal() {
				unbond_amount = unbond_amount.saturating_add(amount);
				if plmc_vesting.next_withdrawal > now {
					break
				}
			}
			bid.plmc_vesting_period = plmc_vesting;

			// * Update storage *
			// TODO: check that the full amount was unreserved
			T::Currency::unreserve_named(&BondType::Bidding, &bid.bidder, unbond_amount);
			// Update the new vector that will go in AuctionInfo with the updated vesting period struct
			new_bids.push(bid.clone());
			// Update the BiddingBonds map with the reduced amount for that project-user
			let mut bond = BiddingBonds::<T>::get(bid.project.clone(), bid.bidder.clone())
				.ok_or(Error::<T>::FieldIsNone)?;
			bond.amount = bond.amount.saturating_sub(unbond_amount);
			// TODO: maybe the BiddingBonds map is redundant, since we can iterate over the Bids vec and calculate it ourselves
			BiddingBonds::<T>::insert(bid.project.clone(), bid.bidder.clone(), bond);

			// * Emit events *
		}

		// Should never return error since we are using the same amount of bids that were there before.
		let new_bids: BoundedVec<BidInfoOf<T>, T::MaximumBidsPerUser> = new_bids.try_into().map_err(|_| Error::<T>::TooManyBids)?;

		// Update the AuctionInfo with the new bids vector
		AuctionsInfo::<T>::insert(project_id, &bidder, new_bids);

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
	/// * `T::Currency` - Mint the tokens to the bidder
	pub fn do_vested_contribution_token_bid_mint_for(bidder: T::AccountId, project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let bids = AuctionsInfo::<T>::get(project_id, &bidder).ok_or(Error::<T>::BidNotFound)?;
		let mut new_bids = vec![];
		let now = <frame_system::Pallet<T>>::block_number();
		for mut bid in bids {
			let mut ct_vesting = bid.ct_vesting_period;
			let mut mint_amount: BalanceOf<T> = 0u32.into();

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if ct_vesting.next_withdrawal > now {
				continue
			}

			// * Calculate variables *
			// Update vesting period until the next withdrawal is in the future
			while let Ok(amount) = ct_vesting.calculate_next_withdrawal() {
				mint_amount = mint_amount.saturating_add(amount);
				if ct_vesting.next_withdrawal > now {
					break
				}
			}
			bid.ct_vesting_period = ct_vesting;

			// * Update storage *
			// TODO: Should we mint here, or should the full mint happen to the treasury and then do transfers from there?
			// Mint the funds for the user
			T::Assets::mint_into(bid.project, &bid.bidder, mint_amount)?;
			new_bids.push(bid);
			// * Emit events *
		}
		// Update the bids with the new vesting period struct
		let new_bids: BoundedVec<BidInfoOf<T>, T::MaximumBidsPerUser> = new_bids.try_into().map_err(|_| Error::<T>::TooManyBids)?;
		AuctionsInfo::<T>::insert(project_id, &bidder, new_bids);


		Ok(())
	}

	// TODO: implement vesting on contributions
	/// Mint contribution tokens after a step in the vesting period for a contribution.
	///
	/// # Arguments
	/// * claimer: The account who made the contribution
	/// * project_id: The project the contribution was made for
	///
	/// # Storage access
	///
	pub fn do_vested_contribution_token_contribution_mint_for(claimer: T::AccountId, project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// * Get variables *
		let project_info = ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let contributions = Contributions::<T>::get(project_id, &claimer).ok_or(Error::<T>::BidNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		// let weighted_average_price = project_info
		// 	.weighted_average_price
		// 	.expect("Final price is set after the Funding Round");
		let mut updated_contributions = vec![];

		// * Validity checks *
		// TODO: PLMC-133. Check the right credential status
		// ensure!(
		// 	T::HandleMembers::is_in(&MemberRole::Issuer, &issuer),
		// 	Error::<T>::NotAuthorized
		// );
		ensure!(
				project_info.project_status == ProjectStatus::FundingEnded,
				Error::<T>::CannotClaimYet
			);
		// TODO: PLMC-160. Check the flow of the final_price if the final price discovery during the Auction Round fails

		for mut contribution in contributions {
			let mut ct_vesting = contribution.ct_vesting;
			let mut mint_amount: BalanceOf<T> = 0u32.into();

			// * Validity checks *
			// check that it is not too early to withdraw the next amount
			if ct_vesting.next_withdrawal > now {
				continue
			}

			// * Calculate variables *
			// Update vesting period until the next withdrawal is in the future
			while let Ok(amount) = ct_vesting.calculate_next_withdrawal() {
				mint_amount = mint_amount.saturating_add(amount);
				if ct_vesting.next_withdrawal > now {
					break
				}
			}
			contribution.ct_vesting = ct_vesting;

			// * Update storage *
			// TODO: Should we mint here, or should the full mint happen to the treasury and then do transfers from there?
			// Mint the funds for the user
			T::Assets::mint_into(project_id, &claimer, mint_amount)?;
			updated_contributions.push(contribution);
			// * Emit events *
		}

		// * Update storage *
		// TODO: PLMC-147. For now only the participants of the Community Round can claim their tokens
		// 	Obviously also the participants of the Auction Round should be able to claim their tokens
		// In theory this should never fail, since we insert the same number of contributions as before
		let updated_contributions: BoundedVec<ContributionInfoOf<T>, T::MaxContributionsPerUser> = updated_contributions.try_into().map_err(|_| Error::<T>::TooManyContributions)?;
		Contributions::<T>::insert(project_id, &claimer, updated_contributions);

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
	pub fn fund_account_id(index: T::ProjectIdentifier) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(index)
	}

	pub fn bond_bidding(
		caller: T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		let now = <frame_system::Pallet<T>>::block_number();
		let project_info = ProjectsInfo::<T>::get(project_id)
			.ok_or(Error::<T>::ProjectInfoNotFound)
			.unwrap();

		if let Some(bidding_end_block) = project_info.phase_transition_points.candle_auction.end() {
			ensure!(now < bidding_end_block, Error::<T>::TooLateForBidBonding);
		}

		BiddingBonds::<T>::try_mutate(project_id, caller.clone(), |maybe_bond| {
			match maybe_bond {
				Some(bond) => {
					// If the user has already bonded, add the new amount to the old one
					bond.amount += amount;
					T::Currency::reserve_named(&BondType::Bidding, &caller, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
				None => {
					// If the user has not bonded yet, create a new bond
					*maybe_bond = Some(BiddingBond {
						project: project_id,
						account: caller.clone(),
						amount,
						when: <frame_system::Pallet<T>>::block_number(),
					});

					// Reserve the required PLMC
					T::Currency::reserve_named(&BondType::Bidding, &caller, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
			}
			Self::deposit_event(Event::<T>::FundsBonded {
				project_id,
				amount,
				bonder: caller.clone(),
			});
			Result::<(), Error<T>>::Ok(())
		})?;

		Ok(())
	}

	pub fn bond_contributing(
		caller: T::AccountId,
		project_id: T::ProjectIdentifier,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		let now = <frame_system::Pallet<T>>::block_number();
		let project_info = ProjectsInfo::<T>::get(project_id)
			.ok_or(Error::<T>::ProjectInfoNotFound)
			.unwrap();

		if let Some(remainder_end_block) = project_info.phase_transition_points.remainder.end() {
			ensure!(now < remainder_end_block, Error::<T>::TooLateForContributingBonding);
		}

		ContributingBonds::<T>::try_mutate(project_id, caller.clone(), |maybe_bond| {
			match maybe_bond {
				Some(bond) => {
					// If the user has already bonded, add the new amount to the old one
					bond.amount += amount;
					T::Currency::reserve_named(&BondType::Contributing, &caller, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
				None => {
					// If the user has not bonded yet, create a new bond
					*maybe_bond = Some(ContributingBond {
						project: project_id,
						account: caller.clone(),
						amount,
					});

					// Reserve the required PLMC
					T::Currency::reserve_named(&BondType::Contributing, &caller, amount)
						.map_err(|_| Error::<T>::InsufficientBalance)?;
				},
			}
			Self::deposit_event(Event::<T>::FundsBonded {
				project_id,
				amount,
				bonder: caller.clone(),
			});
			Result::<(), Error<T>>::Ok(())
		})?;

		Ok(())
	}

	/// Adds a project to the ProjectsToUpdate storage, so it can be updated at some later point in time.
	///
	/// * `block_number` - the minimum block number at which the project should be updated.
	/// * `project_id` - the id of the project to be updated.
	pub fn add_to_update_store(
		block_number: T::BlockNumber,
		project_id: &T::ProjectIdentifier,
	) -> Result<(), DispatchError> {
		// Try to get the project into the earliest possible block to update.
		// There is a limit for how many projects can update each block, so we need to make sure we don't exceed that limit
		let mut block_number = block_number;
		while ProjectsToUpdate::<T>::try_append(block_number, project_id).is_err() {
			block_number += 1u32.into();
		}
		Ok(())
	}

	pub fn calculate_vesting_periods(
		_caller: T::AccountId,
		_multiplier: u32,
		ct_amount: BalanceOf<T>,
	) -> (Vesting<T::BlockNumber, BalanceOf<T>>, Vesting<T::BlockNumber, BalanceOf<T>>) {
		let plmc_start: T::BlockNumber = 0u32.into();
		let ct_start: T::BlockNumber = (parachains_common::DAYS * 7).into();
		// TODO: Calculate real vesting periods based on multiplier and caller type
		let plmc_amount = ct_amount;
		(
			Vesting {
				amount: plmc_amount,
				start: plmc_start.into(),
				end: plmc_start.into(),
				step: 0u32.into(),
				next_withdrawal: 0u32.into(),
			},
			Vesting {
				amount: ct_amount,
				start: ct_start.into(),
				end: ct_start.into(),
				step: 0u32.into(),
				next_withdrawal: 0u32.into(),
			},
		)
	}

	/// Calculates the price of contribution tokens for the Community and Remainder Rounds
	///
	/// # Arguments
	///
	/// * `project_id` - Id used to retrieve the project information from storage
	/// * `end_block` - Block where the candle auction ended, which will make bids after it invalid
	/// * `fundraising_target` - Amount of tokens that the project wants to raise
	pub fn calculate_weighted_average_price(
		project_id: T::ProjectIdentifier,
		end_block: T::BlockNumber,
		total_allocation_size: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		// Get all the bids that were made before the end of the candle
		let mut bids = AuctionsInfo::<T>::iter_values().flatten().collect::<Vec<_>>();
		// temp variable to store the sum of the bids
		let mut bid_amount_sum = BalanceOf::<T>::zero();
		// temp variable to store the total value of the bids (i.e price * amount)
		let mut bid_value_sum = BalanceOf::<T>::zero();

		// sort bids by price
		bids.sort();
		// accept only bids that were made before `end_block` i.e end of candle auction
		let bids = bids
			.into_iter()
			.map(|mut bid| {
				if bid.when > end_block {
					bid.status = BidStatus::Rejected(RejectionReason::AfterCandleEnd);
					// TODO: PLMC-147. Unlock funds. We can do this inside the "on_idle" hook, and change the `status` of the `Bid` to "Unreserved"
					return bid
				}
				let buyable_amount = total_allocation_size.saturating_sub(bid_amount_sum);
				if buyable_amount == 0_u32.into() {
					bid.status = BidStatus::Rejected(RejectionReason::NoTokensLeft);
				} else if bid.amount <= buyable_amount {
					bid_amount_sum.saturating_accrue(bid.amount);
					bid_value_sum.saturating_accrue(bid.amount * bid.price);
					bid.status = BidStatus::Accepted;
				} else {
					bid_amount_sum.saturating_accrue(buyable_amount);
					bid_value_sum.saturating_accrue(buyable_amount * bid.price);
					bid.status =
						BidStatus::PartiallyAccepted(buyable_amount, RejectionReason::NoTokensLeft)
					// TODO: PLMC-147. Refund remaining amount
				}
				bid
			})
			.collect::<Vec<BidInfoOf<T>>>();

		// Calculate the weighted price of the token for the next funding rounds, using winning bids.
		// for example: if there are 3 winning bids,
		// A: 10K tokens @ USD15 per token = 150K USD value
		// B: 20K tokens @ USD20 per token = 400K USD value
		// C: 20K tokens @ USD10 per token = 200K USD value,

		// then the weight for each bid is:
		// A: 150K / (150K + 400K + 200K) = 0.20
		// B: 400K / (150K + 400K + 200K) = 0.53
		// C: 200K / (150K + 400K + 200K) = 0.26

		// then multiply each weight by the price of the token to get the weighted price per bid
		// A: 0.20 * 15 = 3
		// B: 0.53 * 20 = 10.6
		// C: 0.26 * 10 = 2.6

		// lastly, sum all the weighted prices to get the final weighted price for the next funding round
		// 3 + 10.6 + 2.6 = 16.2
		let weighted_token_price = bids
			// TODO: PLMC-150. collecting due to previous mut borrow, find a way to not collect and borrow bid on filter_map
			.into_iter()
			.filter_map(|bid| match bid.status {
				BidStatus::Accepted =>
					Some(Perbill::from_rational(bid.amount * bid.price, bid_value_sum) * bid.price),
				BidStatus::PartiallyAccepted(amount, _) =>
					Some(Perbill::from_rational(amount * bid.price, bid_value_sum) * bid.price),
				_ => None,
			})
			.reduce(|a, b| a.saturating_add(b))
			.ok_or(Error::<T>::NoBidsFound)?;

		Ok(weighted_token_price)
	}

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
}
