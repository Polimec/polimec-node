//! Benchmarking setup for pallet-template
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Faucet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use frame_support::traits::{EnsureOrigin, Get};
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
		assert_last_event::<T>(Event::<T>::Claimed { claimer_did: did.clone(), claimer: caller, amount: T::InitialClaimAmount::get() }.into() );
	}

	#[benchmark]
	fn set_claiming_amount() -> Result<(), BenchmarkError> {
		let origin = T::AdminOrigin::try_successful_origin()
			.map_err(|_| BenchmarkError::Weightless)?;
		assert_eq!(ClaimAmount::<T>::get(), T::InitialClaimAmount::get());

		let new_amount = T::InitialClaimAmount::get() + One::one();
		#[extrinsic_call]
		set_claiming_amount(origin as T::RuntimeOrigin, new_amount);

		assert_eq!(ClaimAmount::<T>::get(), new_amount);
		assert_last_event::<T>(Event::<T>::ClaimAmountChanged(new_amount).into() );
		Ok(())
	}

	impl_benchmark_test_suite!(Faucet, crate::mock::ExtBuilder::default().build(), crate::mock::Test);
}
