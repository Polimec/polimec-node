use crate::{BalanceOf, Config, ProjectId};
use frame_support::weights::Weight;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::DispatchError;

pub trait BondingRequirementCalculation {
	fn calculate_bonding_requirement<T: Config>(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait VestingDurationCalculation {
	fn calculate_vesting_duration<T: Config>(&self) -> BlockNumberFor<T>;
}

pub trait ProvideStatemintPrice {
	type AssetId;
	type Price: FixedPointNumber;
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;
}

pub trait DoRemainingOperation<T: Config> {
	fn has_remaining_operations(&self) -> bool;

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError>;
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
pub trait SetPrices {
	fn set_prices();
}

#[cfg(any(feature = "runtime-benchmarks", feature = "std"))]
impl SetPrices for () {
	fn set_prices() {}
}
