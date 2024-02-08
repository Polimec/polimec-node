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

use xcm::v3::{
	opaque::Instruction,
	prelude::{XcmError, XcmResult},
};

pub trait HrmpHandler {
	fn handle_channel_open_request(message: Instruction) -> XcmResult;
	fn handle_channel_accepted(message: Instruction) -> XcmResult;
}

impl HrmpHandler for () {
	fn handle_channel_open_request(_message: Instruction) -> XcmResult {
		Err(XcmError::NoDeal)
	}

	fn handle_channel_accepted(_message: Instruction) -> XcmResult {
		Err(XcmError::NoDeal)
	}
}
