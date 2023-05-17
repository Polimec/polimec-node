use crate::{BalanceOf, Config};

pub trait BondingRequirementCalculation<T: Config> {
	fn calculate_bonding_requirement(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}
