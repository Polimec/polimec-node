use crate::{BalanceOf, Config};
use sp_arithmetic::FixedPointNumber;

pub trait BondingRequirementCalculation<T: Config> {
	fn calculate_bonding_requirement(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait ProvideStatemintPrice {
	type AssetId;
	type Price: FixedPointNumber;
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;
}
