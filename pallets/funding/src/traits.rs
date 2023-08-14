use crate::{BalanceOf, Config};
use frame_support::weights::Weight;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::DispatchError;

pub trait BondingRequirementCalculation<T: Config> {
	fn calculate_bonding_requirement(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait VestingDurationCalculation<T: Config> {
	fn calculate_vesting_duration(&self) -> Result<T::BlockNumber, ()>;
}

pub trait ProvideStatemintPrice {
	type AssetId;
	type Price: FixedPointNumber;
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;
}

pub trait DoRemainingOperation {
	fn has_remaining_operations(&self) -> bool;

	fn do_one_operation<T: crate::Config>(&mut self, project_id: T::ProjectIdentifier)
		-> Result<Weight, DispatchError>;
}
