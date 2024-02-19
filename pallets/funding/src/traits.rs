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

use crate::{AccountIdOf, BalanceOf, BidInfoOf, BidStatus, Config, ContributionInfoOf, EvaluationInfoOf, ProjectId};
use frame_support::{weights::Weight, WeakBoundedVec};
use frame_system::pallet_prelude::BlockNumberFor;
use itertools::Itertools;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_arithmetic::FixedPointNumber;
use sp_core::{ConstU32, Get, MaxEncodedLen};
use sp_runtime::{DispatchError, RuntimeDebug};
use sp_std::{marker::PhantomData, prelude::*};

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
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Default)]
pub struct SettlementParticipants<Evaluations, Bids, Contributions> {
	pub evaluations: Evaluations,
	pub successful_bids: Bids,
	pub contributions: Contributions,
}

pub type SettlementParticipantsOf<T> = SettlementParticipants<
	WeakBoundedVec<EvaluationInfoOf<T>, <T as Config>::MaxEvaluationsPerProject>,
	WeakBoundedVec<BidInfoOf<T>, <T as Config>::MaxBidsPerProject>,
	WeakBoundedVec<ContributionInfoOf<T>, ConstU32<20_000>>,
>;

pub struct ParticipantExtractor<T: Config>(PhantomData<T>);
impl<T: Config> ParticipantExtractor<T> {
	pub fn evaluations(settlement_participants: SettlementParticipantsOf<T>) -> SettlementTarget<T> {
		SettlementTarget::<T>::Evaluations(settlement_participants.evaluations.to_vec())
	}

	pub fn successful_bids(settlement_participants: SettlementParticipantsOf<T>) -> SettlementTarget<T> {
		SettlementTarget::<T>::Bids(
			settlement_participants
				.successful_bids
				.to_vec()
				.into_iter()
				.filter(|b| matches!(b.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)))
				.collect_vec(),
		)
	}

	pub fn contributions(settlement_participants: SettlementParticipantsOf<T>) -> SettlementTarget<T> {
		SettlementTarget::<T>::Contributions(settlement_participants.contributions.to_vec())
	}

	pub fn accounts(settlement_participants: SettlementParticipantsOf<T>) -> SettlementTarget<T> {
		let evaluators = settlement_participants.evaluations.into_iter().map(|e| e.evaluator);
		let bidders = settlement_participants.successful_bids.into_iter().map(|b| b.bidder);
		let contributors = settlement_participants.contributions.into_iter().map(|c| c.contributor);
		let participants = evaluators.chain(bidders).chain(contributors).collect_vec();
		SettlementTarget::<T>::Accounts(participants)
	}
}

/// The current participants that are awaiting a specific settlement to be done by the chain.
#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
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
