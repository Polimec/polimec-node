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
use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_project_decision(project_id: T::ProjectIdentifier, decision: FundingOutcomeDecision) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

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

    pub fn do_decide_project_outcome(
		issuer: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		decision: FundingOutcomeDecision,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity checks *
		ensure!(project_details.issuer == issuer, Error::<T>::NotAllowed);
		ensure!(project_details.status == ProjectStatus::AwaitingProjectDecision, Error::<T>::NotAllowed);

		// * Update storage *
		Self::remove_from_update_store(&project_id)?;
		Self::add_to_update_store(now + 1u32.into(), (&project_id, UpdateType::ProjectDecision(decision)));

		Self::deposit_event(Event::ProjectOutcomeDecided { project_id, decision });

		Ok(())
	}
}