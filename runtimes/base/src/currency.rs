use crate::Balance;

/// One PLMC
pub const PLMC: Balance = 10u128.pow(10);
/// 0.001 PLMC
pub const MILLI_PLMC: Balance = 10u128.pow(7);
/// 0.000_001 PLMC
pub const MICRO_PLMC: Balance = 10u128.pow(4);

pub const EXISTENTIAL_DEPOSIT: Balance = MILLI_PLMC;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * 20 * PLMC + (bytes as Balance) * 100 * MICRO_PLMC) / 100
}