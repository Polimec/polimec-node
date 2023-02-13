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

use frame_support::{pallet_prelude::DispatchError, traits::Get};
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
		// TODO: Validate and check if the preimage is a valid JSON conforming with our needs
		// TODO: Check if we can use serde in a no_std environment

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

		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.is_frozen = true;
			project_info.project_status = ProjectStatus::EvaluationRound;
			project_info.evaluation_period_ends = Some(evaluation_period_ends);
		});

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
		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::English);
			project_info.auction_metadata = Some(auction_metadata);
		});

		Self::deposit_event(Event::<T>::AuctionStarted { project_id, when: current_block_number });
		Ok(())
	}

	pub fn handle_evaluation_end(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
		fundraising_target: BalanceOf<T>,
	) {
		if now >= evaluation_period_ends {
			let initial_balance: BalanceOf<T> = Zero::zero();
			let total_amount_bonded = Bonds::<T>::iter_prefix_values(project_id)
				.fold(initial_balance, |acc, bond| acc.saturating_add(bond));
			// Check if the total amount bonded is greater than the 10% of the fundraising target
			// TODO: 10% is hardcoded, check if we want to configure it a runtime as explained here:
			// https://substrate.stackexchange.com/questions/2784/how-to-get-a-percent-portion-of-a-balance
			// TODO: Check if it's safe to use * here
			let evaluation_target = Percent::from_percent(9) * fundraising_target;
			let is_funded = total_amount_bonded > evaluation_target;
			if is_funded {
				ProjectsInfo::<T>::mutate(project_id, |project_info| {
					project_info.project_status = ProjectStatus::EvaluationEnded;
				});
				Self::deposit_event(Event::<T>::EvaluationEnded { project_id: *project_id });
			// TODO: Unlock the bonds and clean the storage
			} else {
				ProjectsInfo::<T>::mutate(project_id, |project_info| {
					project_info.project_status = ProjectStatus::EvaluationFailed;
				});
				Self::deposit_event(Event::<T>::EvaluationFailed { project_id: *project_id });
				// TODO: Unlock the bonds and clean the storage
				// TODO: Remove the project from the active projects
				ProjectsActive::<T>::mutate(|projects| {
					projects.retain(|id| id != project_id);
				});
			}
		}
	}

	pub fn handle_auction_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		evaluation_period_ends: T::BlockNumber,
	) {
		if evaluation_period_ends + T::EnglishAuctionDuration::get() <= now {
			// TODO: Unused error, more tests needed
			// TODO: Here the start_auction is "free", check the Weight
			let _ = Self::do_start_auction(*project_id);
		}
	}

	pub fn handle_auction_candle(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) {
		if now >= english_ending_block {
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				project_info.project_status = ProjectStatus::AuctionRound(AuctionPhase::Candle);
			});
		}
	}

	pub fn handle_community_start(
		project_id: &T::ProjectIdentifier,
		now: T::BlockNumber,
		candle_ending_block: T::BlockNumber,
		english_ending_block: T::BlockNumber,
	) {
		if now >= candle_ending_block {
			// TODO: Move fundraising_target to AuctionMetadata
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				let mut auction_metadata =
					project_info.auction_metadata.as_mut().expect("Auction must exist");
				let end_block = Self::select_random_block(
					english_ending_block + 1_u8.into(),
					candle_ending_block,
				);
				project_info.project_status = ProjectStatus::CommunityRound;
				auction_metadata.random_ending_block = Some(end_block);
				project_info.weighted_average_price = Some(
					Self::calculate_weighted_average_price(
						*project_id,
						project_info.fundraising_target,
						end_block,
					)
					.expect("placeholder_function"),
				);
			});
		}
	}

	pub fn handle_community_end(
		project_id: T::ProjectIdentifier,
		now: T::BlockNumber,
		community_ending_block: T::BlockNumber,
	) {
		if now >= community_ending_block {
			ProjectsInfo::<T>::mutate(project_id, |project_info| {
				project_info.project_status = ProjectStatus::FundingEnded;
			});
		};

		// TODO: Check if make sense to set the admin as T::fund_account_id(project_id)
		let issuer =
			ProjectsIssuers::<T>::get(project_id).expect("The issuer exists, already tested.");
		let project = Projects::<T>::get(project_id).expect("The project exists, already tested.");
		let token_information = project.token_information;

		// TODO: Unused result
		// Create the "Contribution Token" as an asset using the pallet_assets and set its metadata
		let _ = T::Assets::create(project_id, issuer.clone(), false, 1_u32.into());
		// TODO: Unused result
		let _ = T::Assets::set(
			project_id,
			&issuer,
			token_information.name.into(),
			token_information.symbol.into(),
			token_information.decimals,
		);
	}

	pub fn handle_fuding_end(project_id: &T::ProjectIdentifier, _now: T::BlockNumber) {
		// Project identified by project_id is no longer "active"
		ProjectsActive::<T>::mutate(|active_projects| {
			if let Some(pos) = active_projects.iter().position(|x| x == project_id) {
				active_projects.remove(pos);
			}
		});

		ProjectsInfo::<T>::mutate(project_id, |project_info| {
			project_info.project_status = ProjectStatus::ReadyToLaunch;
		});
	}

	pub fn calculate_weighted_average_price(
		project_id: T::ProjectIdentifier,
		total_allocation_size: BalanceOf<T>,
		end_block: T::BlockNumber,
	) -> Result<BalanceOf<T>, DispatchError> {
		// Get all the bids that were made before the end of the candle
		// FIXME: Update the `status` field of every `Bid` to BeforeCandle or AfterCandle if `bid.when > end_block`
		let mut bids = AuctionsInfo::<T>::get(project_id);
		bids.retain(|bid| bid.when <= end_block);
		// TODO: Unreserve the funds of the bids that were made after the end of the candle
		// TODO: We can do this inside the "on_idle" hook, and change the `status` of the `Bid` to "Unreserved"

		// Calculate the final price
		let mut fundraising_amount = BalanceOf::<T>::zero();
		for (idx, bid) in bids.iter_mut().enumerate() {
			let old_amount = fundraising_amount;
			fundraising_amount.saturating_accrue(bid.amount);
			if fundraising_amount > total_allocation_size {
				bid.amount = total_allocation_size.saturating_sub(old_amount);
				bid.ratio = Some(Perbill::from_rational(bid.amount, total_allocation_size));
				bid.status = BidStatus::NotValid(bid.amount.saturating_sub(old_amount));
				bids.truncate(idx + 1);
				// Important TODO: Refund the rest of the amount to the bidder in the "on_idle" hook
				break
			}
		}

		// TODO: Test more cases
		let mut weighted_average_price = BalanceOf::<T>::zero();
		for mut bid in bids {
			let ratio = Perbill::from_rational(bid.amount, fundraising_amount);
			bid.ratio = Some(ratio);
			let weighted_price = ratio.mul_ceil(bid.price);
			weighted_average_price.saturating_accrue(weighted_price);
		}
		// AuctionsInfo::<T>::set(project_id, bids);
		Ok(weighted_average_price)
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
		// FIXME: This is a hack to convert the FixedU128 to BalanceOf<T>, it doesnt work for all cases
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
