#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

use pallet_funding::{self as funding};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

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
			project_id: <T as funding::Config>::ProjectIdParameter,
			amount: <T as funding::Config>::Balance,
		) -> DispatchResult {
			let retail_user = ensure_signed(origin)?;
			let project_id: <T as funding::Config>::ProjectIdentifier = project_id.into();
			// Check project is in the community round
			let project_info = funding::Pallet::<T>::project_info(project_id)
				.ok_or(Error::<T>::ProjectNotFound)?;
			ensure!(
				project_info.project_status == funding::ProjectStatus::CommunityRound,
				"Project is not in the community round"
			);

			// Calculate how much funding was done already
			let project_contributions: <T as funding::Config>::Balance =
				funding::Contributions::<T>::iter_prefix_values(project_id).flatten().fold(
					0u64.into(),
					|total_tokens_bought, contribution| {
						total_tokens_bought + contribution.contribution_amount
					},
				);

			ensure!(
				project_contributions >= 500_000_0_000_000_000u64.into(),
				"Project did not achieve at least 500k USDT funding"
			);

			// Buy tokens with the default multiplier
			<funding::Pallet<T>>::do_contribute(retail_user, project_id, amount, None)?;

			Ok(())
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		ProjectNotFound,
	}
}
