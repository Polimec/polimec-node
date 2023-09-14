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

#[benchmarks(
	where
	T: Config + frame_system::Config<RuntimeEvent = <T as Config>::RuntimeEvent>,
    <T as frame_system::Config>::AccountId: Into<<<T as frame_system::Config>::RuntimeOrigin as OriginTrait>::AccountId> + sp_std::fmt::Debug,
)]
mod benches {
	use super::*;

	#[benchmark]
	fn vest() -> Result<(), BenchmarkError> {
		let caller = whitelisted_caller();

		let reason: ReasonOf<T> = LockType::Participation(0);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), reason);

		Ok(())
	}
	// Implements a test for each benchmark. Execute with:
	// `cargo test -p pallet-linear-release --features runtime-benchmarks`.
	impl_benchmark_test_suite!(PalletLinearRelease, crate::mock::new_test_ext(), crate::mock::Test);
}
