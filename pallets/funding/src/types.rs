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

//! Types for Funding pallet.

use crate::traits::BondingRequirementCalculation;
pub use config::*;
pub use extrinsic::*;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::BlockNumberFor;
pub use inner::*;
use parachains_common::DAYS;
use polimec_common::USD_DECIMALS;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use serde::{Deserialize, Serialize};
use sp_arithmetic::{traits::Saturating, FixedPointNumber, FixedU128, Percent};
use sp_runtime::traits::{Convert, One};
use sp_std::{cmp::Eq, prelude::*};
pub use storage::*;

use crate::{traits::VestingDurationCalculation, Config};

use sp_runtime::traits::Zero;

pub mod config {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use crate::Balance;

	#[derive(
		Clone,
		Encode,
		Decode,
		Eq,
		PartialEq,
		TypeInfo,
		MaxEncodedLen,
		Copy,
		Ord,
		PartialOrd,
		RuntimeDebug,
		Serialize,
		Deserialize,
	)]
	pub struct Multiplier(u8);

	impl Multiplier {
		pub const fn force_new(x: u8) -> Self {
			Self(x)
		}
	}

	impl BondingRequirementCalculation for Multiplier {
		fn calculate_usd_bonding_requirement<T: Config>(&self, usd_ticket_size: Balance) -> Option<Balance> {
			let balance_multiplier = Balance::from(self.0);
			usd_ticket_size.checked_div(balance_multiplier)
		}
	}

	impl VestingDurationCalculation for Multiplier {
		fn calculate_vesting_duration<T: Config>(&self) -> BlockNumberFor<T> {
			// gradient "m" of the linear curve function y = m*x + b where x is the multiplier and y is the number of weeks
			const GRADIENT: FixedU128 = FixedU128::from_rational(2167u128, 1000u128);
			// negative constant (because we cannot have negative values, so we take the negative and do "-b" instead of "+b") "b" of the linear curve function y = m*x + b
			const NEG_CONSTANT: FixedU128 = FixedU128::from_rational(2167u128, 1000u128);

			let multiplier_as_fixed = FixedU128::saturating_from_integer(self.0);
			let weeks = GRADIENT.saturating_mul(multiplier_as_fixed).saturating_sub(NEG_CONSTANT);

			T::DaysToBlocks::convert(weeks * FixedU128::from_u32(7u32)).max(One::one())
		}
	}

	impl Default for Multiplier {
		fn default() -> Self {
			Self(1u8)
		}
	}

	impl TryFrom<u8> for Multiplier {
		type Error = &'static str;

		fn try_from(x: u8) -> Result<Self, Self::Error> {
			// The minimum and maximum values are chosen to be 1 and 25 respectively, as defined in the Knowledge Hub.
			const MIN_VALID: u8 = 1;
			const MAX_VALID: u8 = 25;

			if (MIN_VALID..=MAX_VALID).contains(&x) {
				Ok(Self(x))
			} else {
				Err("u8 outside the allowed multiplier range")
			}
		}
	}

	impl From<Multiplier> for u8 {
		fn from(val: Multiplier) -> Self {
			val.0
		}
	}

	/// Enum used to identify PLMC holds.
	/// It implements Serialize and Deserialize to hold a fungible in the Genesis Configuration.
	#[derive(
		Encode,
		Decode,
		Copy,
		Clone,
		PartialEq,
		Eq,
		RuntimeDebug,
		MaxEncodedLen,
		TypeInfo,
		Ord,
		PartialOrd,
		Serialize,
		Deserialize,
	)]

	pub struct DaysToBlocks;
	impl Convert<FixedU128, u64> for DaysToBlocks {
		fn convert(a: FixedU128) -> u64 {
			let one_day_in_blocks = DAYS;
			a.saturating_mul_int(one_day_in_blocks as u64)
		}
	}
	impl Convert<FixedU128, u32> for DaysToBlocks {
		fn convert(a: FixedU128) -> u32 {
			let one_day_in_blocks = DAYS;
			a.saturating_mul_int(one_day_in_blocks)
		}
	}

	pub const RETAIL_MAX_MULTIPLIER: u8 = 5u8;
	pub const PROFESSIONAL_MAX_MULTIPLIER: u8 = 10u8;
	pub const INSTITUTIONAL_MAX_MULTIPLIER: u8 = 25u8;
}

pub mod storage {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use crate::Balance;

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
	pub struct ProjectMetadata<BoundedString, Price: FixedPointNumber, AccountId, Cid> {
		/// Token Metadata
		pub token_information: CurrencyMetadata<BoundedString>,
		/// Mainnet Token Max Supply
		pub mainnet_token_max_supply: Balance,
		/// Total allocation of Contribution Tokens available for the Funding Round.
		pub total_allocation_size: Balance,
		/// Percentage of the total allocation of Contribution Tokens available for the Auction Round
		pub auction_round_allocation_percentage: Percent,
		/// The minimum price per token in USD, decimal-aware. See [`calculate_decimals_aware_price()`](crate::traits::ProvideAssetPrice::calculate_decimals_aware_price) for more information.
		pub minimum_price: Price,
		/// Maximum and minimum ticket sizes for auction round
		pub bidding_ticket_sizes: BiddingTicketSizes<Price>,
		/// Maximum and minimum ticket sizes for community/remainder rounds
		pub contributing_ticket_sizes: ContributingTicketSizes<Price>,
		/// Participation currencies (e.g stablecoin, DOT, KSM)
		/// e.g. https://github.com/paritytech/substrate/blob/427fd09bcb193c1e79dec85b1e207c718b686c35/frame/uniques/src/types.rs#L110
		/// For now is easier to handle the case where only just one Currency is accepted
		pub participation_currencies:
			BoundedVec<AcceptedFundingAsset, ConstU32<{ AcceptedFundingAsset::VARIANT_COUNT as u32 }>>,
		pub funding_destination_account: AccountId,
		/// Additional metadata
		pub policy_ipfs_cid: Option<Cid>,
	}

	impl<BoundedString, Price: FixedPointNumber, AccountId, Cid> ProjectMetadata<BoundedString, Price, AccountId, Cid> {
		/// Validate issuer metadata for the following checks:
		/// - Minimum price is not zero
		/// - Minimum bidding ticket sizes are higher than 5k USD
		/// - Specified participation currencies are unique
		pub fn is_valid(&self) -> Result<(), MetadataError> {
			if self.minimum_price == Price::zero() {
				return Err(MetadataError::PriceTooLow);
			}
			let usd_unit = sp_arithmetic::traits::checked_pow(10u128, USD_DECIMALS as usize)
				.ok_or(MetadataError::BadTokenomics)?;

			let min_bidder_bound_usd: Balance = usd_unit.checked_mul(5000u128).ok_or(MetadataError::BadTokenomics)?;
			self.bidding_ticket_sizes.is_valid(vec![
				InvestorTypeUSDBounds::Professional((min_bidder_bound_usd, None).into()),
				InvestorTypeUSDBounds::Institutional((min_bidder_bound_usd, None).into()),
			])?;

			let min_contributor_bound_usd: Balance = usd_unit.checked_mul(1u128).ok_or(MetadataError::BadTokenomics)?;
			self.contributing_ticket_sizes.is_valid(vec![
				InvestorTypeUSDBounds::Institutional((min_contributor_bound_usd, None).into()),
				InvestorTypeUSDBounds::Professional((min_contributor_bound_usd, None).into()),
				InvestorTypeUSDBounds::Retail((min_contributor_bound_usd, None).into()),
			])?;

			if self.total_allocation_size == 0u64.into() ||
				self.total_allocation_size > self.mainnet_token_max_supply ||
				self.total_allocation_size < 10u128.saturating_pow(self.token_information.decimals as u32)
			{
				return Err(MetadataError::AllocationSizeError);
			}

			if self.auction_round_allocation_percentage <= Percent::from_percent(0) {
				return Err(MetadataError::AuctionRoundPercentageError);
			}

			let mut deduped = self.participation_currencies.clone().to_vec();
			deduped.sort();
			deduped.dedup();
			if deduped.len() != self.participation_currencies.len() {
				return Err(MetadataError::ParticipationCurrenciesError);
			}

			let target_funding = self.minimum_price.saturating_mul_int(self.total_allocation_size);
			if target_funding < (1000u64 * 10u64.saturating_pow(USD_DECIMALS.into())).into() {
				return Err(MetadataError::FundingTargetTooLow);
			}
			if target_funding > (1_000_000_000u64 * 10u64.saturating_pow(USD_DECIMALS.into())).into() {
				return Err(MetadataError::FundingTargetTooHigh);
			}

			if self.token_information.decimals < 6 || self.token_information.decimals > 18 {
				return Err(MetadataError::BadDecimals);
			}

			let abs_diff: u32 = self.token_information.decimals.abs_diff(USD_DECIMALS).into();
			let abs_diff_unit = 10u128.checked_pow(abs_diff).ok_or(MetadataError::BadDecimals)?;
			let abs_diff_fixed = Price::checked_from_rational(abs_diff_unit, 1).ok_or(MetadataError::BadDecimals)?;
			let original_price = if USD_DECIMALS > self.token_information.decimals {
				self.minimum_price.checked_div(&abs_diff_fixed)
			} else {
				self.minimum_price.checked_mul(&abs_diff_fixed)
			}
			.ok_or(MetadataError::BadDecimals)?;

			let min_price = Price::checked_from_rational(1, 100_000).ok_or(MetadataError::BadTokenomics)?;
			let max_price = Price::checked_from_rational(1000, 1).ok_or(MetadataError::BadTokenomics)?;
			if original_price < min_price || original_price > max_price {
				return Err(MetadataError::BadTokenomics);
			}

			Ok(())
		}
	}

	pub struct Bound {
		pub lower: Balance,
		pub upper: Option<Balance>,
	}

	impl From<(Balance, Option<Balance>)> for Bound {
		fn from(value: (Balance, Option<Balance>)) -> Self {
			Self { lower: value.0, upper: value.1 }
		}
	}

	pub enum InvestorTypeUSDBounds {
		Retail(Bound),
		Professional(Bound),
		Institutional(Bound),
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum MigrationType {
		Offchain,
		Pallet(PalletMigrationInfo),
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct PalletMigrationInfo {
		/// ParaId of project
		pub parachain_id: ParaId,
		/// HRMP Channel status
		pub hrmp_channel_status: HRMPChannelStatus,
		/// Migration readiness check
		pub migration_readiness_check: Option<PalletMigrationReadinessCheck>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct ProjectDetails<AccountId, Did, BlockNumber, Price: FixedPointNumber, EvaluationRoundInfo> {
		pub issuer_account: AccountId,
		pub issuer_did: Did,
		/// Whether the project is frozen, so no `metadata` changes are allowed.
		pub is_frozen: bool,
		/// The price in USD per token decided after the Auction Round
		pub weighted_average_price: Option<Price>,
		/// The current status of the project
		pub status: ProjectStatus<BlockNumber>,
		/// When the different project phases start and end
		pub round_duration: BlockNumberPair<BlockNumber>,
		/// Fundraising target amount in USD (6 decimals)
		pub fundraising_target_usd: Balance,
		/// The amount of Contribution Tokens that have not yet been sold
		pub remaining_contribution_tokens: Balance,
		/// Funding reached amount in USD (6 decimals)
		pub funding_amount_reached_usd: Balance,
		/// Information about the total amount bonded, and the outcome in regards to reward/slash/nothing
		pub evaluation_round_info: EvaluationRoundInfo,
		/// If the auction was oversubscribed, how much USD was raised across all winning bids
		pub usd_bid_on_oversubscription: Option<Balance>,
		/// When the Funding Round ends
		pub funding_end_block: Option<BlockNumber>,
		pub migration_type: Option<MigrationType>,
	}
	/// Tells on_initialize what to do with the project
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum UpdateType {
		EvaluationEnd,
		AuctionOpeningStart,
		AuctionClosingStart,
		AuctionClosingEnd,
		CommunityFundingStart,
		RemainderFundingStart,
		FundingEnd,
		ProjectDecision(FundingOutcomeDecision),
		StartSettlement,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Ord, PartialOrd)]
	pub struct EvaluationInfo<Id, Did, ProjectId, AccountId, BlockNumber> {
		pub id: Id,
		pub did: Did,
		pub project_id: ProjectId,
		pub evaluator: AccountId,
		pub original_plmc_bond: Balance,
		// An evaluation bond can be converted to participation bond
		pub current_plmc_bond: Balance,
		pub early_usd_amount: Balance,
		pub late_usd_amount: Balance,
		pub when: BlockNumber,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct BidInfo<ProjectId, Did, Price: FixedPointNumber, AccountId, BlockNumber> {
		pub id: u32,
		pub project_id: ProjectId,
		pub bidder: AccountId,
		pub did: Did,
		pub status: BidStatus,
		#[codec(compact)]
		pub original_ct_amount: Balance,
		pub original_ct_usd_price: Price,
		pub funding_asset: AcceptedFundingAsset,
		pub funding_asset_amount_locked: Balance,
		pub mode: ParticipationMode,
		pub plmc_bond: Balance,
		pub when: BlockNumber,
	}
	impl<ProjectId, Did, Price: FixedPointNumber, AccountId, BlockNumber>
		BidInfo<ProjectId, Did, Price, AccountId, BlockNumber>
	{
		pub fn final_ct_amount(&self) -> Balance {
			match self.status {
				BidStatus::Accepted => self.original_ct_amount,
				BidStatus::PartiallyAccepted(amount) => amount,
				_ => Zero::zero(),
			}
		}
	}

	impl<ProjectId: Eq, Did: Eq, Price: FixedPointNumber, AccountId: Eq, BlockNumber: Eq + Ord> Ord
		for BidInfo<ProjectId, Did, Price, AccountId, BlockNumber>
	{
		fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
			match self.original_ct_usd_price.cmp(&other.original_ct_usd_price) {
				sp_std::cmp::Ordering::Equal => Ord::cmp(&other.id, &self.id),
				other => other,
			}
		}
	}

	impl<ProjectId: Eq, Did: Eq, Price: FixedPointNumber, AccountId: Eq, BlockNumber: Eq + Ord> PartialOrd
		for BidInfo<ProjectId, Did, Price, AccountId, BlockNumber>
	{
		fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
			Some(self.cmp(other))
		}
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct ContributionInfo<Id, Did, ProjectId, AccountId, BlockNumber> {
		pub id: Id,
		pub did: Did,
		pub project_id: ProjectId,
		pub contributor: AccountId,
		pub ct_amount: Balance,
		pub usd_contribution_amount: Balance,
		pub mode: ParticipationMode,
		pub funding_asset: AcceptedFundingAsset,
		pub funding_asset_amount: Balance,
		pub plmc_bond: Balance,
		pub when: BlockNumber,
	}

	/// Represents a bucket that holds a specific amount of tokens at a given price.
	/// Each bucket has a unique ID, an amount of tokens left, a current price, an initial price,
	/// and constants to define price and amount increments for the next buckets.
	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct Bucket<Price> {
		/// The amount of tokens left in this bucket.
		pub amount_left: Balance,
		/// The current price of tokens in this bucket.
		pub current_price: Price,
		/// The initial price of tokens in the bucket.
		pub initial_price: Price,
		/// Defines the price increment for each subsequent bucket.
		pub delta_price: Price,
		/// Defines the amount increment for each subsequent bucket.
		pub delta_amount: Balance,
	}

	impl<Price: FixedPointNumber> Bucket<Price> {
		/// Creates a new bucket with the given parameters.
		pub const fn new(
			amount_left: Balance,
			initial_price: Price,
			delta_price: Price,
			delta_amount: Balance,
		) -> Self {
			Self { amount_left, current_price: initial_price, initial_price, delta_price, delta_amount }
		}

		/// Update the bucket
		pub fn update(&mut self, removed_amount: Balance) {
			self.amount_left.saturating_reduce(removed_amount);
			if self.amount_left.is_zero() {
				self.next();
			}
		}

		/// Updates the bucket to represent the next one in the sequence. This involves:
		/// - resetting the amount left,
		/// - recalculating the price based on the current price and the price increments defined by the `delta_price`.
		fn next(&mut self) {
			self.amount_left = self.delta_amount;
			self.current_price = self.current_price.saturating_add(self.delta_price);
		}

		pub fn calculate_wap(self, mut total_amount: Balance) -> Price {
			// First bucket is not empty so wap is the same as the initial price
			if self.current_price == self.initial_price {
				return self.current_price;
			}
			let mut amount: Balance = self.delta_amount.saturating_sub(self.amount_left);
			let mut price: Price = self.current_price;
			let mut bucket_sizes: Vec<(Balance, Price)> = Vec::new();
			while price > self.initial_price && total_amount > Balance::zero() {
				total_amount.saturating_reduce(amount);
				bucket_sizes.push((price.saturating_mul_int(amount), price));
				price = price.saturating_sub(self.delta_price);
				amount = self.delta_amount;
			}

			if total_amount > Balance::zero() {
				bucket_sizes.push((self.initial_price.saturating_mul_int(total_amount), self.initial_price));
			}

			let sum = bucket_sizes.iter().map(|x| x.0).fold(Balance::zero(), |acc, x| acc.saturating_add(x));

			bucket_sizes
				.into_iter()
				.map(|x| <Price as FixedPointNumber>::saturating_from_rational(x.0, sum).saturating_mul(x.1))
				.fold(Price::zero(), |acc: Price, p: Price| acc.saturating_add(p))
		}
	}
}

pub mod inner {
	#[allow(clippy::wildcard_imports)]
	use super::*;
	use crate::Balance;
	use variant_count::VariantCount;
	use xcm::v4::QueryId;

	pub enum MetadataError {
		/// The minimum price per token is too low.
		PriceTooLow,
		/// The ticket sizes are not valid.
		TicketSizeError,
		/// The participation currencies are not unique.
		ParticipationCurrenciesError,
		/// The allocation size is invalid. Either zero or higher than the max supply.
		AllocationSizeError,
		/// The auction round percentage cannot be zero.
		AuctionRoundPercentageError,
		/// The funding target has to be higher than 1000 USD.
		FundingTargetTooLow,
		/// The funding target has to be lower than 1bn USD.
		FundingTargetTooHigh,
		/// The project's metadata hash is not provided while starting the evaluation round.
		CidNotProvided,
		/// The ct decimals specified for the CT is outside the 4 to 20 range.
		BadDecimals,
		// The combination of decimals and price of this project is not representable within our 6 decimals USD system,
		// and integer space of 128 bits.
		BadTokenomics,
	}

	#[derive(
		Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct CurrencyMetadata<BoundedString> {
		/// The user-friendly name of this asset. Limited in length by `StringLimit`.
		pub name: BoundedString,
		/// The ticker symbol for this asset. Limited in length by `StringLimit`.
		pub symbol: BoundedString,
		/// The number of decimals this asset uses to represent one unit.
		pub decimals: u8,
	}

	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct TicketSize {
		pub usd_minimum_per_participation: Balance,
		pub usd_maximum_per_did: Option<Balance>,
	}
	impl TicketSize {
		pub fn new(usd_minimum_per_participation: Balance, usd_maximum_per_did: Option<Balance>) -> Self {
			Self { usd_minimum_per_participation, usd_maximum_per_did }
		}

		pub fn usd_ticket_above_minimum_per_participation(&self, usd_amount: Balance) -> bool {
			usd_amount >= self.usd_minimum_per_participation
		}

		pub fn usd_ticket_below_maximum_per_did(&self, usd_amount: Balance) -> bool {
			match self.usd_maximum_per_did {
				Some(max) => usd_amount <= max,
				None => true,
			}
		}

		pub fn check_valid(&self, bound: Bound) -> bool {
			if let (min, Some(max)) = (self.usd_minimum_per_participation, self.usd_maximum_per_did) {
				if min > max {
					return false
				}
			}

			if self.usd_minimum_per_participation < bound.lower {
				return false;
			}

			if let Some(upper_bound) = bound.upper {
				let Some(max_usd) = self.usd_maximum_per_did else { return false };
				if max_usd > upper_bound {
					return false;
				}
			}
			true
		}
	}

	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct BiddingTicketSizes<Price: FixedPointNumber> {
		pub professional: TicketSize,
		pub institutional: TicketSize,
		pub phantom: PhantomData<(Price, Balance)>,
	}
	impl<Price: FixedPointNumber> BiddingTicketSizes<Price> {
		pub fn is_valid(&self, usd_bounds: Vec<InvestorTypeUSDBounds>) -> Result<(), MetadataError> {
			for bound in usd_bounds {
				match bound {
					InvestorTypeUSDBounds::Professional(bound) =>
						if !self.professional.check_valid(bound) {
							return Err(MetadataError::TicketSizeError);
						},
					InvestorTypeUSDBounds::Institutional(bound) =>
						if !self.institutional.check_valid(bound) {
							return Err(MetadataError::TicketSizeError);
						},
					_ => {},
				}
			}
			Ok(())
		}
	}

	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize,
	)]
	pub struct ContributingTicketSizes<Price: FixedPointNumber> {
		pub retail: TicketSize,
		pub professional: TicketSize,
		pub institutional: TicketSize,
		pub phantom: PhantomData<(Price, Balance)>,
	}
	impl<Price: FixedPointNumber> ContributingTicketSizes<Price> {
		pub fn is_valid(&self, usd_bounds: Vec<InvestorTypeUSDBounds>) -> Result<(), MetadataError> {
			for bound in usd_bounds {
				match bound {
					InvestorTypeUSDBounds::Professional(bound) =>
						if !self.professional.check_valid(bound) {
							return Err(MetadataError::TicketSizeError);
						},
					InvestorTypeUSDBounds::Institutional(bound) =>
						if !self.institutional.check_valid(bound) {
							return Err(MetadataError::TicketSizeError);
						},
					InvestorTypeUSDBounds::Retail(bound) =>
						if !self.retail.check_valid(bound) {
							return Err(MetadataError::TicketSizeError);
						},
				}
			}
			Ok(())
		}
	}

	#[derive(
		VariantCount,
		Clone,
		Copy,
		Encode,
		Decode,
		Eq,
		PartialEq,
		PartialOrd,
		Ord,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		Serialize,
		Deserialize,
	)]
	pub enum AcceptedFundingAsset {
		#[codec(index = 0)]
		USDT,
		#[codec(index = 1)]
		USDC,
		#[codec(index = 2)]
		DOT,
	}
	impl AcceptedFundingAsset {
		pub const fn id(&self) -> u32 {
			match self {
				AcceptedFundingAsset::USDT => 1984,
				AcceptedFundingAsset::DOT => 10,
				AcceptedFundingAsset::USDC => 1337,
			}
		}
	}

	#[derive(
		Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize,
	)]
	pub enum ProjectStatus<BlockNumber> {
		#[default]
		Application,
		EvaluationRound,
		AuctionRound,
		CommunityRound(BlockNumber),
		FundingFailed,
		FundingSuccessful,
		SettlementStarted(FundingOutcome),
		SettlementFinished(FundingOutcome),
		CTMigrationStarted,
		CTMigrationFinished,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
	pub enum FundingOutcome {
		Success,
		Failure,
	}

	#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct BlockNumberPair<BlockNumber> {
		pub start: Option<BlockNumber>,
		pub end: Option<BlockNumber>,
	}

	impl<BlockNumber: Copy + sp_std::cmp::PartialOrd> BlockNumberPair<BlockNumber> {
		pub const fn new(start: Option<BlockNumber>, end: Option<BlockNumber>) -> Self {
			Self { start, end }
		}

		pub const fn start(&self) -> Option<BlockNumber> {
			self.start
		}

		pub const fn end(&self) -> Option<BlockNumber> {
			self.end
		}

		pub fn started(&self, at: BlockNumber) -> bool {
			self.start.map_or(true, |start| start <= at)
		}

		pub fn ended(&self, at: BlockNumber) -> bool {
			self.end.map_or(true, |end| end <= at)
		}
	}

	#[derive(Default, Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum BidStatus {
		/// The bid is not yet accepted or rejected
		#[default]
		YetUnknown,
		/// The bid is accepted
		Accepted,
		/// The bid is rejected because the ct tokens ran out
		Rejected,
		/// The bid is partially accepted as there were not enough tokens to fill the full bid
		PartiallyAccepted(Balance),
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct VestingInfo<BlockNumber> {
		pub total_amount: Balance,
		pub amount_per_block: Balance,
		pub duration: BlockNumber,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct EvaluationRoundInfo {
		pub total_bonded_usd: Balance,
		pub total_bonded_plmc: Balance,
		pub evaluators_outcome: Option<EvaluatorsOutcome>,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum EvaluatorsOutcome {
		Rewarded(RewardInfo),
		Slashed,
	}

	#[derive(Default, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct RewardInfo {
		// Total "Early Evaluators" rewards amount in Contribution Tokens
		pub early_evaluator_reward_pot: Balance,
		// Total "Normal Evaluators" rewards amount in Contribution Tokens
		pub normal_evaluator_reward_pot: Balance,
		pub early_evaluator_total_bonded_usd: Balance,
		pub normal_evaluator_total_bonded_usd: Balance,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum FundingOutcomeDecision {
		AcceptFunding,
		RejectFunding,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct PalletMigrationReadinessCheck {
		pub holding_check: (QueryId, CheckOutcome),
		pub pallet_check: (QueryId, CheckOutcome),
	}

	impl PalletMigrationReadinessCheck {
		pub fn is_ready(&self) -> bool {
			self.holding_check.1 == CheckOutcome::Passed(None) &&
				matches!(self.pallet_check.1, CheckOutcome::Passed(Some(_)))
		}
	}

	pub type PalletIndex = u8;
	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum CheckOutcome {
		AwaitingResponse,
		Passed(Option<PalletIndex>),
		Failed,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct HRMPChannelStatus {
		pub project_to_polimec: ChannelStatus,
		pub polimec_to_project: ChannelStatus,
	}

	#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub enum ChannelStatus {
		/// hrmp channel is closed.
		Closed,
		/// hrmp channel is open.
		Open,
		/// request for a hrmp channel was sent to the relay. Waiting for response.
		AwaitingAcceptance,
	}

	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct ProjectMigrationOrigins<ProjectId, MigrationOrigins> {
		pub project_id: ProjectId,
		pub migration_origins: MigrationOrigins,
	}

	#[derive(
		Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen, Serialize, Deserialize,
	)]
	pub enum ParticipationMode {
		/// One Token Model. User only needs funding assets, and pays a fee to bond treasury PLMC.
		OTM,
		/// Classic model. User needs to bond PLMC based on a multiplier, and pays no extra fee.
		Classic(u8),
	}
	impl ParticipationMode {
		pub fn multiplier(&self) -> u8 {
			match self {
				// OTM multiplier is fixed at 5
				ParticipationMode::OTM => 5u8,
				ParticipationMode::Classic(multiplier) => *multiplier,
			}
		}
	}
}

pub mod extrinsic {
	use crate::{
		AcceptedFundingAsset, AccountIdOf, Balance, Config, ParticipationMode, PriceOf, ProjectDetailsOf, ProjectId,
		TicketSize,
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use polimec_common::credentials::{Cid, Did, InvestorType};

	pub struct DoBidParams<T: Config> {
		pub bidder: AccountIdOf<T>,
		pub project_id: ProjectId,
		pub ct_amount: Balance,
		pub mode: ParticipationMode,
		pub funding_asset: AcceptedFundingAsset,
		pub did: Did,
		pub investor_type: InvestorType,
		pub whitelisted_policy: Cid,
	}

	pub struct DoPerformBidParams<T: Config> {
		pub bidder: AccountIdOf<T>,
		pub project_id: ProjectId,
		pub ct_amount: Balance,
		pub ct_usd_price: PriceOf<T>,
		pub mode: ParticipationMode,
		pub funding_asset: AcceptedFundingAsset,
		pub bid_id: u32,
		pub now: BlockNumberFor<T>,
		pub did: Did,
		pub metadata_ticket_size_bounds: TicketSize,
		pub total_bids_by_bidder: u32,
		pub total_bids_for_project: u32,
	}

	pub struct DoContributeParams<T: Config> {
		pub contributor: AccountIdOf<T>,
		pub project_id: ProjectId,
		pub ct_amount: Balance,
		pub mode: ParticipationMode,
		pub funding_asset: AcceptedFundingAsset,
		pub did: Did,
		pub investor_type: InvestorType,
		pub whitelisted_policy: Cid,
	}

	pub struct DoPerformContributionParams<'a, T: Config> {
		pub contributor: AccountIdOf<T>,
		pub project_id: ProjectId,
		pub project_details: &'a mut ProjectDetailsOf<T>,
		pub buyable_tokens: Balance,
		pub mode: ParticipationMode,
		pub funding_asset: AcceptedFundingAsset,
		pub investor_type: InvestorType,
		pub did: Did,
		pub whitelisted_policy: Cid,
	}

	pub struct BidRefund<T: Config> {
		pub final_ct_usd_price: PriceOf<T>,
		pub final_ct_amount: Balance,
		pub refunded_plmc: Balance,
		pub refunded_funding_asset_amount: Balance,
	}
}
