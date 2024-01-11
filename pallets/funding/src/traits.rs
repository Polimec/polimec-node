use crate::{AccountIdOf, BalanceOf, Config, ProjectId};
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

pub trait CleanerOperations<T: Config> {
	fn has_remaining_operations(&self) -> bool;

	fn do_one_operation(&mut self, project_id: ProjectId) -> Result<Weight, DispatchError>;

	fn do_operations_with_weight_limit(
		&mut self,
		project_id: ProjectId,
		weight_limit: Weight,
	) -> Result<Weight, DispatchError> {
		let mut weight_consumed = Weight::zero();
		while self.has_remaining_operations() && weight_consumed < weight_limit {
			weight_consumed += self.do_one_operation(project_id)?;
		}
		Ok(weight_consumed)
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
