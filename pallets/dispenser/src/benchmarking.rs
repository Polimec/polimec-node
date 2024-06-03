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
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Dispenser;
use frame_benchmarking::v2::*;
use frame_support::traits::{EnsureOrigin, Get};
use frame_system::RawOrigin;
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt_with_cid};
use sp_runtime::traits::One;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
mod benchmarks {
	use super::*;
	#[benchmark]
	fn dispense() {
		let caller: T::AccountId = whitelisted_caller();
		let did = generate_did_from_account(1);
		assert_eq!(Dispensed::<T>::get(did.clone()), None);
		CurrencyOf::<T>::deposit_creating(&Dispenser::<T>::dispense_account(), T::InitialDispenseAmount::get());

		let jwt = get_mock_jwt_with_cid(caller.clone(), InvestorType::Retail, did.clone(), T::WhitelistedPolicy::get());
		#[extrinsic_call]
		dispense(RawOrigin::Signed(caller.clone()), jwt);

		assert_eq!(Dispensed::<T>::get(did.clone()), Some(()));
		assert_last_event::<T>(
			Event::<T>::Dispensed {
				dispensed_to_did: did.clone(),
				dispensed_to: caller,
				amount: T::InitialDispenseAmount::get(),
			}
			.into(),
		);
	}

	#[benchmark]
	fn set_dispense_amount() -> Result<(), BenchmarkError> {
		let origin = T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_eq!(DispenseAmount::<T>::get(), T::InitialDispenseAmount::get());

		let new_amount = T::InitialDispenseAmount::get() + One::one();
		#[extrinsic_call]
		set_dispense_amount(origin as T::RuntimeOrigin, new_amount);

		assert_eq!(DispenseAmount::<T>::get(), new_amount);
		assert_last_event::<T>(Event::<T>::DispenseAmountChanged(new_amount).into());
		Ok(())
	}

	impl_benchmark_test_suite!(Dispenser, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
