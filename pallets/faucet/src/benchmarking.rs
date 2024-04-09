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
use crate::Pallet as Faucet;
use frame_benchmarking::v2::*;
use frame_support::traits::{EnsureOrigin, Get};
use frame_system::RawOrigin;
use polimec_common_test_utils::{generate_did_from_account, get_mock_jwt};
use sp_runtime::traits::One;

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn claim() {
		let caller: T::AccountId = whitelisted_caller();
		let did = generate_did_from_account(1);
		assert_eq!(Claims::<T>::get(did.clone()), None);

		let jwt = get_mock_jwt(caller.clone(), InvestorType::Retail, did.clone());
		#[extrinsic_call]
		claim(RawOrigin::Signed(caller.clone()), jwt);

		assert_eq!(Claims::<T>::get(did.clone()), Some(()));
		assert_last_event::<T>(
			Event::<T>::Claimed { claimer_did: did.clone(), claimer: caller, amount: T::InitialClaimAmount::get() }
				.into(),
		);
	}

	#[benchmark]
	fn set_claiming_amount() -> Result<(), BenchmarkError> {
		let origin = T::AdminOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_eq!(ClaimAmount::<T>::get(), T::InitialClaimAmount::get());

		let new_amount = T::InitialClaimAmount::get() + One::one();
		#[extrinsic_call]
		set_claiming_amount(origin as T::RuntimeOrigin, new_amount);

		assert_eq!(ClaimAmount::<T>::get(), new_amount);
		assert_last_event::<T>(Event::<T>::ClaimAmountChanged(new_amount).into());
		Ok(())
	}

	impl_benchmark_test_suite!(Faucet, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
