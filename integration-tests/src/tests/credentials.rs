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

use frame_support::assert_ok;
use polimec_common::credentials::InvestorType;
use polimec_common_test_utils::get_test_jwt;
use polimec_parachain_runtime::PolimecFunding;
use tests::defaults::*;
use crate::*;

#[test]
fn jwt_verify_retail() {
	let jwt = get_test_jwt(PolimecAccountId::from(BUYER_1), InvestorType::Retail);
	Polimec::execute_with(|| {
		assert_ok!(PolimecFunding::verify(PolimecOrigin::signed(BUYER_1.into()), jwt));
	});
}