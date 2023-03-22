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

	pub fn do_create(
		project_id: T::ProjectIdentifier,
		issuer: &T::AccountId,
		project: ProjectOf<T>,
	) -> Result<(), DispatchError> {
		let fundraising_target = project.total_allocation_size * project.minimum_price;
		let project_info = ProjectInfo {
			is_frozen: false,
			weighted_average_price: None,
			created_at: <frame_system::Pallet<T>>::block_number(),
			project_status: ProjectStatus::Application,
			evaluation_period_ends: None,
			auction_metadata: None,
			fundraising_target,
		};

		Projects::<T>::insert(project_id, project);
		ProjectsInfo::<T>::insert(project_id, project_info);
		ProjectsIssuers::<T>::insert(project_id, issuer);
		NextProjectId::<T>::mutate(|n| n.saturating_inc());

		Self::deposit_event(Event::<T>::Created { project_id });
		Ok(())
	}

	pub fn do_start_evaluation(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		let evaluation_period_ends =
			<frame_system::Pallet<T>>::block_number() + T::EvaluationDuration::get();

		ProjectsActive::<T>::try_append(project_id)
			.map_err(|()| Error::<T>::TooManyActiveProjects)?;

		let maybe_project_info = ProjectsInfo::<T>::get(project_id);
		let mut project_info = maybe_project_info.ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(!project_info.is_frozen, Error::<T>::ProjectAlreadyFrozen);
		ensure!(
			project_info.project_status == ProjectStatus::Application,
			Error::<T>::ProjectNotInApplicationRound
		);
		project_info.is_frozen = true;
		project_info.project_status = ProjectStatus::EvaluationRound;
		project_info.evaluation_period_ends = Some(evaluation_period_ends);

		ProjectsInfo::<T>::insert(project_id, project_info);

		Self::deposit_event(Event::<T>::EvaluationStarted { project_id });
		Ok(())
	}

	pub fn do_start_auction(project_id: T::ProjectIdentifier) -> Result<(), DispatchError> {
		let current_block_number = <frame_system::Pallet<T>>::block_number();
		let english_ending_block = current_block_number + T::EnglishAuctionDuration::get();
		let candle_ending_block = english_ending_block + T::CandleAuctionDuration::get();
		let community_ending_block = candle_ending_block + T::CommunityRoundDuration::get();

		let auction_metadata = AuctionMetadata {
			starting_block: current_block_number,
			english_ending_block,
			candle_ending_block,
			community_ending_block,
			random_ending_block: None,
		};

		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(
			project_info.project_status == ProjectStatus::EvaluationEnded,
			Error::<T>::ProjectNotInEvaluationRound
		);

		project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
		project_info.auction_metadata = Some(auction_metadata);

		ProjectsInfo::<T>::insert(project_id, project_info);

		Self::deposit_event(Event::<T>::AuctionStarted { project_id, when: current_block_number });
		Ok(())
	}

	pub fn handle_evaluation_end(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
		fundraising_target: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		if now > evaluation_period_ends {
			let initial_balance: BalanceOf<T> = Zero::zero();
			let total_amount_bonded = Bonds::<T>::iter_prefix_values(project_id)
				.fold(initial_balance, |acc, bond| acc.saturating_add(bond));
			// Check if the total amount bonded is greater than the 10% of the fundraising target
			// TODO: PLMC-142. 10% is hardcoded, check if we want to configure it a runtime as explained here:
			// 	https://substrate.stackexchange.com/questions/2784/how-to-get-a-percent-portion-of-a-balance:
			// TODO: PLMC-143. Check if it's safe to use * here
			let evaluation_target = Percent::from_percent(9) * fundraising_target;
			let is_funded = total_amount_bonded > evaluation_target;
			let mut project_info =
				ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
			ensure!(
				project_info.project_status == ProjectStatus::EvaluationRound,
				Error::<T>::ProjectNotInEvaluationRound
			);
			if is_funded {
				project_info.project_status = ProjectStatus::EvaluationEnded;
				Self::deposit_event(Event::<T>::EvaluationEnded { project_id: *project_id });
			// TODO: PLMC-144. Unlock the bonds and clean the storage
			} else {
				project_info.project_status = ProjectStatus::EvaluationFailed;
				Self::deposit_event(Event::<T>::EvaluationFailed { project_id: *project_id });
				// TODO: PLMC-144. Unlock the bonds and clean the storage
				ProjectsActive::<T>::mutate(|projects| {
					projects.retain(|id| id != project_id);
				});
			}

			ProjectsInfo::<T>::insert(project_id, project_info);
		}

		Ok(())
	}

	pub fn handle_auction_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
	) -> Result<(), DispatchError> {
		let project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(
			project_info.project_status == ProjectStatus::EvaluationEnded,
			Error::<T>::ProjectNotInEvaluationEndedRound
		);
		if evaluation_period_ends <= now {
		// TODO: PLMC-145. Unused error, more tests needed
		// 	Here the start_auction is "free", check the Weight
			Self::do_start_auction(*project_id)
		} else {
			Ok(())
		}
	}

	pub fn handle_auction_candle(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) -> Result<(), DispatchError> {
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::English),
			Error::<T>::ProjectNotInEnglishAuctionRound
		);
		if now >= english_ending_block {
			project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
			ProjectsInfo::<T>::insert(project_id, project_info);
			Ok(())
		} else {
			Err(Error::<T>::TooEarlyForCandleAuctionStart)?
		}
	}

	pub fn handle_community_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		candle_ending_block: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) -> Result<(), DispatchError> {
		if now <= candle_ending_block {
			Err(Error::<T>::TooEarlyForCommunityRoundStart)?
		}

		// TODO: PLMC-148 Move fundraising_target to AuctionMetadata
		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		ensure!(
			project_info.project_status == ProjectStatus::AuctionRound(AuctionPhase::Candle),
			Error::<T>::ProjectNotInCandleAuctionRound
		);
		let mut auction_metadata = project_info
			.auction_metadata
			.as_mut()
			.ok_or(Error::<T>::AuctionMetadataNotFound)?;
		let end_block =
			Self::select_random_block(english_ending_block + 1_u8.into(), candle_ending_block);
		project_info.project_status = ProjectStatus::CommunityRound;
		auction_metadata.random_ending_block = Some(end_block);

		project_info.weighted_average_price = Some(Self::calculate_weighted_average_price(
			*project_id,
			end_block,
			project_info.fundraising_target,
		)?);

		ProjectsInfo::<T>::insert(project_id, project_info);
		Ok(())
	}

	pub fn handle_community_end(
		project_id: T::ProjectIdentifier,
		now: T::BlockNumber,
		community_ending_block: T::BlockNumber,
	) -> Result<(), DispatchError> {
		if now <= community_ending_block {
			Err(Error::<T>::TooEarlyForFundingEnd)?
		};

		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
		project_info.project_status = ProjectStatus::FundingEnded;
		ProjectsInfo::<T>::insert(project_id, project_info);

		// TODO: PLMC-149 Check if make sense to set the admin as T::fund_account_id(project_id)
		let issuer =
			ProjectsIssuers::<T>::get(project_id).ok_or(Error::<T>::ProjectIssuerNotFound)?;
		let project = Projects::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let token_information = project.token_information;

		// TODO: PLMC-149 Unused result
		// Create the "Contribution Token" as an asset using the pallet_assets and set its metadata
		let _ = T::Assets::create(project_id, issuer.clone(), false, 1_u32.into());
		// TODO: PLMC-149 Unused result
		let _ = T::Assets::set(
			project_id,
			&issuer,
			token_information.name.into(),
			token_information.symbol.into(),
			token_information.decimals,
		);
		Ok(())
	}

	pub fn handle_fuding_end(
		project_id: &T::ProjectIdentifier,
		_now: T::BlockNumber,
	) -> Result<(), DispatchError> {
		// Project identified by project_id is no longer "active"
		ProjectsActive::<T>::mutate(|active_projects| {
			if let Some(pos) = active_projects.iter().position(|x| x == project_id) {
				active_projects.remove(pos);
			}
		});

		let mut project_info =
			ProjectsInfo::<T>::get(project_id).ok_or(Error::<T>::ProjectInfoNotFound)?;
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
		fundraising_target: BalanceOf<T>,
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
				let buyable_amount = fundraising_target.saturating_sub(bid_amount_sum);
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
