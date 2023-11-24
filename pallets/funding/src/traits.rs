use crate::{BalanceOf, Config};
use frame_support::weights::Weight;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::DispatchError;

pub trait BondingRequirementCalculation {
	fn calculate_bonding_requirement<T: Config>(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait VestingDurationCalculation {
	fn calculate_vesting_duration<T: Config>(&self) -> <T as frame_system::Config>::BlockNumber;
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
