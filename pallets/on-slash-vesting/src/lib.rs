// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

extern crate alloc;
use alloc::vec::Vec;
use frame_support::{
	sp_runtime::{traits::Convert, FixedPointNumber, FixedU128},
	traits::{Currency, OriginTrait},
};
use pallet_vesting::Vesting;
use sp_runtime::traits::BlockNumberProvider;

pub trait OnSlash<AccountId, Balance: Clone> {
	fn on_slash(account: &AccountId, amount: Balance);
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId, Balance: Clone> OnSlash<AccountId, Balance> for Tuple {
	fn on_slash(account: &AccountId, amount: Balance) {
		for_tuples!( #( Tuple::on_slash(account, amount.clone()); )* );
	}
}

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
impl<T> OnSlash<AccountIdOf<T>, u128> for pallet_vesting::Pallet<T>
where
	T: pallet_vesting::Config,
	T::Currency: Currency<AccountIdOf<T>, Balance = u128>,
{
	fn on_slash(account: &AccountIdOf<T>, slashed_amount: u128) {
		let Some(vesting_schedules) = <Vesting<T>>::get(account) else { return };
		let vesting_schedules = vesting_schedules.to_vec();
		let mut new_vesting_schedules = Vec::new();
		let now = T::BlockNumberProvider::current_block_number();
		for schedule in vesting_schedules {
			let total_locked = schedule.locked_at::<T::BlockNumberToBalance>(now).saturating_sub(slashed_amount);
			let start_block: u128 = T::BlockNumberToBalance::convert(now);
			let end_block: u128 = schedule.ending_block_as_balance::<T::BlockNumberToBalance>();
			let duration = end_block.saturating_sub(start_block);
			let per_block = FixedU128::from_rational(total_locked, duration).saturating_mul_int(1u128);
			let new_schedule = pallet_vesting::VestingInfo::new(total_locked, per_block, now);
			if new_schedule.is_valid() {
				new_vesting_schedules.push(new_schedule);
			}
		}
		let Ok(new_vesting_schedules) = new_vesting_schedules.try_into() else {
			log::error!("Failed to convert new vesting schedules into BoundedVec");
			return
		};
		<Vesting<T>>::set(account, Some(new_vesting_schedules));
		let vest_result = <pallet_vesting::Pallet<T>>::vest(T::RuntimeOrigin::signed(account.clone()));
		debug_assert!(vest_result.is_ok());
	}
}
