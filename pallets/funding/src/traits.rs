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

use crate::{AccountIdOf, BalanceOf, BidInfoOf, Config, ContributionInfoOf, EvaluationInfoOf, ProjectId};
use frame_support::weights::Weight;
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::DispatchError;
use sp_std::prelude::*;

pub trait BondingRequirementCalculation {
	fn calculate_bonding_requirement<T: Config>(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait VestingDurationCalculation {
	fn calculate_vesting_duration<T: Config>(&self) -> BlockNumberFor<T>;
}

pub trait ProvideAssetPrice {
	type AssetId;
	type Price: FixedPointNumber;
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;
}

pub trait SettlementOperations<T: Config> {
	fn has_remaining_operations(&self) -> bool;

	fn do_one_operation(
		&mut self,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)>;

	fn execute_with_given_weight(
		&mut self,
		weight: Weight,
		project_id: ProjectId,
		target: &mut SettlementTarget<T>,
	) -> Result<Weight, (Weight, DispatchError)>;
}

/// The original participants of a project that need some settlements (i.e extrinsics) to be done by the chain.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SettlementParticipants<T: Config> {
	evaluations: Vec<EvaluationInfoOf<T>>,
	bids: Vec<BidInfoOf<T>>,
	contributions: Vec<ContributionInfoOf<T>>,
}
impl<T: Config> SettlementParticipants<T> {
	pub fn evaluations(&self) -> SettlementTarget<T> {
		SettlementTarget::Evaluations(self.evaluations)
	}

	pub fn bids(&self) -> SettlementTarget<T> {
		SettlementTarget::Bids(self.bids)
	}

	pub fn contributions(&self) -> SettlementTarget<T> {
		SettlementTarget::Contributions(self.contributions)
	}

	pub fn accounts(&self) -> SettlementTarget<T> {
		let evaluators = self.evaluations.into_iter().map(|e| e.evaluator);
		let bidders = self.bids.into_iter().map(|b| b.bidder);
		let contributors = self.contributions.into_iter().map(|c| c.contributor);
		let participants = evaluators.chain(bidders).chain(contributors).collect_vec();
		SettlementTarget::Accounts(participants)
	}
}

/// The current participants that are awaiting a specific settlement to be done by the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettlementTarget<T: Config> {
	Empty,
	Accounts(Vec<AccountIdOf<T>>),
	Evaluations(Vec<EvaluationInfoOf<T>>),
	Bids(Vec<BidInfoOf<T>>),
	Contributions(Vec<ContributionInfoOf<T>>),
}
impl<T: Config> SettlementTarget<T> {
	pub fn is_empty(&self) -> bool {
		match self {
			Self::Empty => true,
			Self::Accounts(accounts) => accounts.is_empty(),
			Self::Evaluations(evaluations) => evaluations.is_empty(),
			Self::Bids(bids) => bids.is_empty(),
			Self::Contributions(contributions) => contributions.is_empty(),
		}
	}
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub trait SetPrices {
	fn set_prices();
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
impl SetPrices for () {
	fn set_prices() {}
}
