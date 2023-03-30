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
use sp_arithmetic::Perbill;
use sp_runtime::Percent;
use sp_std::prelude::*;

impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: T::ProjectIdentifier) -> T::AccountId {
		T::PalletId::get().into_sub_account_truncating(index)
	}
	/// Store an image on chain.
	pub fn note_bytes(
		preimage: BoundedVec<u8, T::PreImageLimit>,
		issuer: &T::AccountId,
	) -> Result<(), DispatchError> {
		// TODO: PLMC-141. Validate and check if the preimage is a valid JSON conforming with our needs.
		// 	also check if we can use serde in a no_std environment

		let hash = T::Hashing::hash(&preimage);
		Images::<T>::insert(hash, issuer);

		Self::deposit_event(Event::Noted { hash });

		Ok(())
	}

	// called by user extrinsic
	pub fn do_create(
		project_id: T::ProjectIdentifier,
		issuer: &T::AccountId,
		project: ProjectOf<T>,
	) -> Result<(), DispatchError> {
		// TODO: Probably the issuers don't want to sell all of their tokens. Is there some logic for this?
		// 	also even if an issuer wants to sell all their tokens, they could target a lower amount than that to consider it a success
		let fundraising_target = project.total_allocation_size * project.minimum_price;
		let project_info = ProjectInfo {
			is_frozen: false,
			weighted_average_price: None,
			fundraising_target,
			project_status: ProjectStatus::Application,
			// phase_transition_points: PhaseTransitionPoints {
			// 	application_start_block: <frame_system::Pallet<T>>::block_number(),
			// 	application_end_block: None,
			//
			// 	evaluation_start_block: None,
			// 	evaluation_end_block: None,
			//
			// 	auction_initialize_period_start_block: None,
			// 	auction_initialize_period_end_block: None,
			//
			// 	english_auction_start_block: None,
			// 	english_auction_end_block: None,
			//
			// 	candle_auction_start_block: None,
			// 	candle_auction_end_block: None,
			//
			// 	random_ending_block: None,
			//
			// 	community_start_block: None,
			// 	community_end_block: None,
			//
			// 	remainder_start_block: None,
			// 	remainder_end_block: None,
			// },
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

		// validity checks need to be done before function is called
		Projects::<T>::insert(project_id, project);
		ProjectsInfo::<T>::insert(project_id, project_info);
		ProjectsIssuers::<T>::insert(project_id, issuer);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());

		Self::deposit_event(Event::<T>::Created { project_id });
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
		loop {
			if let Ok(()) = ProjectsToUpdate::<T>::try_append(block_number, project_id) {
				break
			} else {
				block_number += 1u32.into();
			}
		}
		Ok(())
	}

	// Called by user extrinsic
	pub fn do_evaluation_start(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// Do checks
		ensure!(
			project_info.project_status == ProjectStatus::Application,
			Error::<T>::ProjectNotInApplicationRound
		);
		ensure!(!project_info.is_frozen, Error::<T>::ProjectAlreadyFrozen);

		// Calculate transition points
		let mut evaluation_end_block = now + T::EvaluationDuration::get();

		// Update project info
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

		// Add to update store
		Self::add_to_update_store(evaluation_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		// Emit events
		Self::deposit_event(Event::<T>::EvaluationStarted { project_id });

		Ok(())
	}

	// Called automatically by on_initialize
	pub fn do_evaluation_end(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let evaluation_end_block = project_info
			.phase_transition_points
			.evaluation
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;
		let fundraising_target = project_info.fundraising_target;

		// Do checks
		ensure!(
			project_info.project_status == EvaluationRound,
			Error::<T>::ProjectNotInEvaluationRound
		);
		ensure!(now > evaluation_end_block, Error::<T>::EvaluationPeriodNotEnded);

		// Check which logic path to follow
		let initial_balance: BalanceOf<T> = Zero::zero();
		let total_amount_bonded = Bonds::<T>::iter_prefix_values(project_id)
			.fold(initial_balance, |acc, bond| acc.saturating_add(bond));

		// Check if the total amount bonded is greater than the 10% of the fundraising target
		// TODO: PLMC-142. 10% is hardcoded, check if we want to configure it a runtime as explained here:
		// 	https://substrate.stackexchange.com/questions/2784/how-to-get-a-percent-portion-of-a-balance:
		// TODO: PLMC-143. Check if it's safe to use * here
		let evaluation_target = Percent::from_percent(10) * fundraising_target;
		let is_funded = total_amount_bonded >= evaluation_target;

		// Successful path
		if is_funded {
			// Calculate transition points
			let mut auction_initialize_period_start_block = now + 1u32.into();
			let mut auction_initialize_period_end_block =
				auction_initialize_period_start_block + T::AuctionInitializePeriodDuration::get();

			// Update project info
			project_info.phase_transition_points.auction_initialize_period.update(
				Some(auction_initialize_period_start_block),
				Some(auction_initialize_period_end_block),
			);
			project_info.project_status = ProjectStatus::AuctionInitializePeriod;
			ProjectsInfo::<T>::insert(project_id, project_info);

			// Emit events
			Self::deposit_event(Event::<T>::AuctionInitializePeriod {
				project_id: *project_id,
				start_block: auction_initialize_period_start_block,
				end_block: auction_initialize_period_end_block,
			});
		// TODO: PLMC-144. Unlock the bonds and clean the storage

		// Unsuccessful path
		} else {
			// Update project info
			project_info.project_status = ProjectStatus::EvaluationFailed;
			ProjectsInfo::<T>::insert(project_id, project_info);

			// Emit events
			Self::deposit_event(Event::<T>::EvaluationFailed { project_id: *project_id });

			// Add to update store
			Self::add_to_update_store(now + 1u32.into(), &project_id)
				.expect("Always returns Ok; qed");

			// TODO: PLMC-144. Unlock the bonds and clean the storage
		}

		Ok(())
	}

	// called by user extrinsic
	pub fn do_english_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
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

		// Do checks
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

		// Calculate transition points
		let english_start_block = now + 1u32.into();
		let english_end_block = now + T::EnglishAuctionDuration::get();

		// Update project info
		project_info
			.phase_transition_points
			.english_auction
			.update(Some(english_start_block), Some(english_end_block));
		project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
		ProjectsInfo::<T>::insert(project_id, project_info);

		// Add to update store
		Self::add_to_update_store(english_end_block + 1u32.into(), &project_id)
			.expect("Always return Ok; qed");

		// Emit events
		Self::deposit_event(Event::<T>::AuctionStarted { project_id, when: now });
		Ok(())
	}

	// called automatically by on_initialize
	pub fn do_candle_auction(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let english_end_block = project_info
			.phase_transition_points
			.english_auction
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// Do checks
		ensure!(now > english_end_block, Error::<T>::TooEarlyForCandleAuctionStart);
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English),
			Error::<T>::ProjectNotInEnglishAuctionRound
		);

		// Calculate transition points
		let candle_start_block = now + 1u32.into();
		let candle_end_block = now + T::CandleAuctionDuration::get();

		// Update project info
		project_info
			.phase_transition_points
			.candle_auction
			.update(Some(candle_start_block), Some(candle_end_block));
		project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
		ProjectsInfo::<T>::insert(project_id, project_info);

		// Add to update store
		Self::add_to_update_store(candle_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		Ok(())
	}

	// called automatically by on_initialize
	pub fn do_community_funding(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
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

		// Do checks
		ensure!(now > auction_candle_end_block, Error::<T>::TooEarlyForCommunityRoundStart);
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle),
			Error::<T>::ProjectNotInCandleAuctionRound
		);

		// Calculate transition points
		let end_block =
			Self::select_random_block(auction_candle_start_block, auction_candle_end_block);
		let community_start_block = now + 1u32.into();
		let community_end_block = now + T::CommunityFundingDuration::get();

		// Update project info
		project_info.phase_transition_points.random_candle_ending = Some(end_block);
		project_info
			.phase_transition_points
			.community
			.update(Some(community_start_block), Some(community_end_block));
		project_info.project_status = ProjectStatus::CommunityRound;
		project_info.weighted_average_price = Some(Self::calculate_weighted_average_price(
			*project_id,
			end_block,
			project_info.fundraising_target,
		)?);
		ProjectsInfo::<T>::insert(project_id, project_info);

		// Add to update store
		Self::add_to_update_store(community_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		Ok(())
	}

	// called automatically by on_initialize
	pub fn do_remainder_funding(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();
		let community_end_block = project_info
			.phase_transition_points
			.community
			.end()
			.ok_or(Error::<T>::FieldIsNone)?;

		// Do checks
		ensure!(now > community_end_block, Error::<T>::TooEarlyForRemainderRoundStart);
		ensure!(
			project_info.project_status == ProjectStatus::CommunityRound,
			Error::<T>::ProjectNotInCommunityRound
		);

		// Calculate transition points
		let remainder_start_block = now + 1u32.into();
		let remainder_end_block = now + T::RemainderFundingDuration::get();

		// Update project info
		project_info
			.phase_transition_points
			.remainder
			.update(Some(remainder_start_block), Some(remainder_end_block));
		project_info.project_status = ProjectStatus::RemainderRound;
		ProjectsInfo::<T>::insert(project_id, project_info);

		// Add to update store
		Self::add_to_update_store(remainder_end_block + 1u32.into(), &project_id)
			.expect("Always returns Ok; qed");

		Ok(())
	}

	// called automatically by on_initialize
	pub fn do_end_funding(project_id: &T::ProjectIdentifier) -> Result<(), DispatchError> {
		// Get variables
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

		// Do checks
		ensure!(now > remainder_end_block, Error::<T>::TooEarlyForFundingEnd);
		ensure!(
			project_info.project_status == ProjectStatus::RemainderRound,
			Error::<T>::ProjectNotInRemainderRound
		);

		// Update project info
		project_info.project_status = ProjectStatus::FundingEnded;
		ProjectsInfo::<T>::insert(project_id, project_info);

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

		Ok(())
	}

	// called manually by user extrinsic
	pub fn do_ready_to_launch(
		project_id: &T::ProjectIdentifier,
		_now: T::BlockNumber,
	) -> Result<(), DispatchError> {
		// Get variables
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;

		// Update project Info
		project_info.project_status = ProjectStatus::ReadyToLaunch;
		ProjectsInfo::<T>::insert(project_id, project_info);

		Ok(())
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
		let mut bids = AuctionsInfo::<T>::get(project_id);
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
	pub fn do_claim_contribution_tokens(
		project_id: T::ProjectIdentifier,
		claimer: T::AccountId,
		contribution_amount: BalanceOf<T>,
		weighted_average_price: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		let fixed_amount =
			Self::calculate_claimable_tokens(contribution_amount, weighted_average_price);
		// FIXME: This is a hack to convert the FixedU128 to BalanceOf<T>, it doesnt work
		// FIXME: The pallet_assets::mint_into function expects a BalanceOf<T>, we need to convert the FixedU128 to BalanceOf<T> keeping the precision
		let amount = fixed_amount.saturating_mul_int(BalanceOf::<T>::one());
		T::Assets::mint_into(project_id, &claimer, amount)?;
		Ok(())
	}

	// This functiion is kept separate from the `do_claim_contribution_tokens` for easier testing the logic
	#[inline(always)]
	pub fn calculate_claimable_tokens(
		contribution_amount: BalanceOf<T>,
		weighted_average_price: BalanceOf<T>,
	) -> FixedU128 {
		FixedU128::saturating_from_rational(contribution_amount, weighted_average_price)
	}
}
