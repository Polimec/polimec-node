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

pub mod constants {
	use frame_support::{
		parameter_types,
		weights::{constants, Weight},
	};

	parameter_types! {
		/// Importing a block with 0 Extrinsics.
		pub const BlockExecutionWeight: Weight =
			Weight::from_parts(constants::WEIGHT_REF_TIME_PER_NANOS.saturating_mul(5_000_000), 0);
	}

	#[cfg(test)]
	mod test_weights {
		use frame_support::weights::constants;

		/// Checks that the weight exists and is sane.
		// NOTE: If this test fails but you are sure that the generated values are fine,
		// you can delete it.
		#[test]
		fn sane() {
			let w = super::constants::BlockExecutionWeight::get();

			// At least 100 µs.
			assert!(
				w.ref_time() >= 100u64 * constants::WEIGHT_REF_TIME_PER_MICROS,
				"Weight should be at least 100 µs."
			);
			// At most 50 ms.
			assert!(w.ref_time() <= 50u64 * constants::WEIGHT_REF_TIME_PER_MILLIS, "Weight should be at most 50 ms.");
		}
	}
}
