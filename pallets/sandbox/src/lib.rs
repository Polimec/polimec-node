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

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

use pallet_funding::{self as funding, ProjectId};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use funding::AcceptedFundingAsset;
	use pallet_funding::MultiplierOf;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + funding::Config {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Buy tokens for a project in the community round if it achieved at least 500k USDT funding
		#[pallet::weight(0)]
		pub fn buy_if_popular(
			origin: OriginFor<T>,
			project_id: ProjectId,
			amount: <T as funding::Config>::Balance,
			asset_id: AcceptedFundingAsset,
			did: polimec_common::credentials::DID,
			investor_type: polimec_common::credentials::InvestorType
		) -> DispatchResultWithPostInfo {
			let retail_user = ensure_signed(origin)?;
			let project_id: ProjectId = project_id;
			// Check project is in the community round
			let project_info = funding::Pallet::<T>::project_details(project_id).ok_or(Error::<T>::ProjectNotFound)?;
			ensure!(
				project_info.status == funding::ProjectStatus::CommunityRound,
				"Project is not in the community round"
			);

			// Calculate how much funding was done already
			let project_contributions: <T as funding::Config>::Balance =
				funding::Contributions::<T>::iter_prefix_values((project_id,))
					.fold(0u64.into(), |total_tokens_bought, contribution| {
						total_tokens_bought + contribution.funding_asset_amount
					});

			ensure!(
				project_contributions >= 500_000_0_000_000_000u64.into(),
				"Project did not achieve at least 500k USDT funding"
			);

			let multiplier: MultiplierOf<T> = 1u8.try_into().map_err(|_| Error::<T>::ProjectNotFound)?;
			// Buy tokens with the default multiplier
			<funding::Pallet<T>>::do_community_contribute(&retail_user, project_id, amount, multiplier, asset_id, did, investor_type)
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		ProjectNotFound,
	}
}
