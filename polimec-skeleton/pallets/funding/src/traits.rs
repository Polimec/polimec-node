use crate::{BalanceOf, Config};
use frame_support::pallet_prelude::DispatchResult;
use frame_support::traits::tokens::fungible;
use frame_support::weights::Weight;
use sp_arithmetic::FixedPointNumber;
use sp_runtime::DispatchError;

pub trait BondingRequirementCalculation<T: Config> {
	fn calculate_bonding_requirement(&self, ticket_size: BalanceOf<T>) -> Result<BalanceOf<T>, ()>;
}

pub trait ProvideStatemintPrice {
	type AssetId;
	type Price: FixedPointNumber;
	fn get_price(asset_id: Self::AssetId) -> Option<Self::Price>;
}

pub trait DoRemainingOperation {
	fn is_done(&self) -> bool;

	fn do_one_operation<T: crate::Config>(&mut self, project_id: T::ProjectIdentifier)
		-> Result<Weight, DispatchError>;
}

/// A vesting schedule over a currency. This allows a particular currency to have vesting limits
/// applied to it.
pub trait ReleaseSchedule<AccountId> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type Moment;

	/// The currency that this schedule applies to.
	type Currency: fungible::InspectHold<AccountId>
		+ fungible::MutateHold<AccountId>
		+ fungible::BalancedHold<AccountId>;

	/// Get the amount that is currently being vested and cannot be transferred out of this account.
	/// Returns `None` if the account has no vesting schedule.
	fn vesting_balance(who: &AccountId) -> Option<<Self::Currency as fungible::Inspect<AccountId>>::Balance>;

	/// Adds a release schedule to a given account.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// Is a no-op if the amount to be vested is zero.
	///
	/// NOTE: This doesn't alter the free balance of the account.
	fn set_release_schedule(
		who: &AccountId, locked: <Self::Currency as fungible::Inspect<AccountId>>::Balance,
		per_block: <Self::Currency as fungible::Inspect<AccountId>>::Balance, starting_block: Self::Moment,
	) -> DispatchResult;

	// /// Checks if `add_vesting_schedule` would work against `who`.
	// fn can_add_vesting_schedule(
	// 	who: &AccountId, locked: <Self::Currency as Currency<AccountId>>::Balance,
	// 	per_block: <Self::Currency as Currency<AccountId>>::Balance, starting_block: Self::Moment,
	// ) -> DispatchResult;

	// /// Remove a vesting schedule for a given account.
	// ///
	// /// NOTE: This doesn't alter the free balance of the account.
	// fn remove_vesting_schedule(who: &AccountId, schedule_index: u32) -> DispatchResult;
}
