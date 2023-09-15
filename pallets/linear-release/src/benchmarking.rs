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

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use pallet::Pallet as PalletLinearRelease;

use frame_benchmarking::v2::*;
use frame_support::traits::OriginTrait;
use frame_system::RawOrigin;
use pallet_funding::LockType;

const SEED: u32 = 0;

fn add_holds<T: Config<Reason = LockType<u32>>>(who: &T::AccountId, n: u8) {
	for id in 0..n {
		let locked = 256u32;
		let reason: ReasonOf<T> = LockType::Participation(0);
		let _ = T::Currency::hold(&reason, who, locked.into());
	}
}

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent>,
    <T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
)]
mod benches {
	use super::*;

	// Implements a test for each benchmark. Execute with:
	// `cargo test -p pallet-linear-release --features runtime-benchmarks`.
	impl_benchmark_test_suite!(
		PalletLinearRelease,
		crate::mock::ExtBuilder::default().existential_deposit(256).build(),
		crate::mock::Test
	);

	#[benchmark]
	fn vest() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();

		let reason = LockType::Participation(0);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), reason);

		Ok(())
	}

	#[benchmark]
	fn vest_all() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()));

		Ok(())
	}

	#[benchmark]
	fn vest_all_other() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let test_dest: T::AccountId = account("test_dest", 0, SEED);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), test_dest.clone());

		Ok(())
	}
}
