// Copyright (C) Parity Technologies (UK) Ltd.

// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// This library includes code from Substrate, which is licensed
// under both the GNU General Public License version 3 (GPLv3) and the
// Apache License 2.0. You may choose to redistribute and/or modify this
// code under either the terms of the GPLv3 or the Apache 2.0 License,
// whichever suits your needs.

//! The for various partial storage decoders

use super::*;
use frame_support::{
	storage::{migration, unhashed},
	BoundedVec,
};

#[test]
fn test_decode_compact_u32_at() {
	new_test_ext().execute_with(|| {
		let v = parity_scale_codec::Compact(u64::MAX);
		migration::put_storage_value(b"test", b"", &[], v);
		assert_eq!(decode_compact_u32_at(b"test"), None);

		for v in [0, 10, u32::MAX] {
			let compact_v = parity_scale_codec::Compact(v);
			unhashed::put(b"test", &compact_v);
			assert_eq!(decode_compact_u32_at(b"test"), Some(v));
		}

		unhashed::kill(b"test");
		assert_eq!(decode_compact_u32_at(b"test"), None);
	})
}

#[test]
fn len_of_deposit_of() {
	new_test_ext().execute_with(|| {
		for l in [0, 1, 200, 1000] {
			let value: (BoundedVec<u64, _>, u64) =
				((0..l).map(|_| Default::default()).collect::<Vec<_>>().try_into().unwrap(), 3u64);
			DepositOf::<Test>::insert(2, value);
			assert_eq!(Democracy::len_of_deposit_of(2), Some(l));
		}

		DepositOf::<Test>::remove(2);
		assert_eq!(Democracy::len_of_deposit_of(2), None);
	})
}
