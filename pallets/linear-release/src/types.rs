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

use super::*;

/// Struct to encode the vesting schedule of an individual account.
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct VestingInfo<Balance, BlockNumber> {
	/// Locked amount at genesis.
	pub locked: Balance,
	/// Amount that gets unlocked every block after `starting_block`.
	pub per_block: Balance,
	/// Starting block for unlocking(vesting).
	pub starting_block: BlockNumber,
}

impl<Balance, BlockNumber> VestingInfo<Balance, BlockNumber>
where
	Balance: AtLeast32BitUnsigned + Copy,
	BlockNumber: AtLeast32BitUnsigned + Copy + Bounded,
{
	/// Instantiate a new `VestingInfo`.
	pub const fn new(
		locked: Balance,
		per_block: Balance,
		starting_block: BlockNumber,
	) -> VestingInfo<Balance, BlockNumber> {
		VestingInfo { locked, per_block, starting_block }
	}

	/// Validate parameters for `VestingInfo`. Note that this does not check
	/// against `MinVestedTransfer`.
	pub fn is_valid(&self) -> bool {
		!self.locked.is_zero() && !self.raw_per_block().is_zero()
	}

	/// Locked amount at schedule creation.
	pub const fn locked(&self) -> Balance {
		self.locked
	}

	/// Amount that gets unlocked every block after `starting_block`. Corrects for `per_block` of 0.
	/// We don't let `per_block` be less than 1, or else the vesting will never end.
	/// This should be used whenever accessing `per_block` unless explicitly checking for 0 values.
	pub fn per_block(&self) -> Balance {
		self.per_block.max(One::one())
	}

	/// Get the unmodified `per_block`. Generally should not be used, but is useful for
	/// validating `per_block`.
	pub(crate) const fn raw_per_block(&self) -> Balance {
		self.per_block
	}

	/// Starting block for unlocking(vesting).
	pub const fn starting_block(&self) -> BlockNumber {
		self.starting_block
	}

	/// Amount locked at block `n`.
	pub fn locked_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self, now: BlockNumber) -> Balance {
		// Number of blocks that count toward vesting;
		// saturating to 0 when n < starting_block.
		let time_range = now.saturating_sub(self.starting_block);
		let vested_block_count = BlockNumberToBalance::convert(time_range);
		// Return amount that is still locked in vesting.
		vested_block_count
			.checked_mul(&self.per_block()) // `per_block` accessor guarantees at least 1.
			.map(|to_unlock| self.locked.saturating_sub(to_unlock))
			.unwrap_or_else(Zero::zero())
	}

	/// Amount to be released at block `n`.
	pub fn releaseble_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self, now: BlockNumber) -> Balance {
		// Number of blocks that count toward vesting;
		// saturating to 0 when n < starting_block.
		let time_range = now.saturating_sub(self.starting_block);
		let vested_block_count = BlockNumberToBalance::convert(time_range);
		// TODO: Find a way to improve this.
		// Return amount that is releasable in vesting.
		let res = vested_block_count.checked_mul(&self.per_block()).unwrap_or_else(Zero::zero());
		res.min(self.locked)
	}

	/// Block number at which the schedule ends (as type `Balance`).
	pub fn ending_block_as_balance<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self) -> Balance {
		let starting_block = BlockNumberToBalance::convert(self.starting_block);
		let duration = if self.per_block() >= self.locked {
			// If `per_block` is bigger than `locked`, the schedule will end
			// the block after starting.
			One::one()
		} else {
			self.locked / self.per_block() +
				if (self.locked % self.per_block()).is_zero() {
					Zero::zero()
				} else {
					// `per_block` does not perfectly divide `locked`, so we need an extra block to
					// unlock some amount less than `per_block`.
					One::one()
				}
		};

		starting_block.saturating_add(duration)
	}
}
