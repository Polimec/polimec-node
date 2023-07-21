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

//! Benchmarking setup for pallet-asset-registry

use super::*;

#[allow(unused)]
use crate::Pallet as AssetRegistry;
use frame_benchmarking::benchmarks;
use frame_support::{assert_ok, traits::fungibles::Inspect};
use frame_system::RawOrigin;
use xcm::opaque::latest::{
	Junction::{GeneralIndex, PalletInstance, Parachain},
	Junctions, MultiLocation,
};

pub const LOCAL_ASSET_ID: u32 = 10;

benchmarks! {
	where_clause {
		where
			T::Assets: Inspect<<T as frame_system::Config>::AccountId, AssetId = u32>,
	}

	register_reserve_asset {
		let asset_multi_location = MultiLocation {
			parents: 1,
			interior: Junctions::X3(Parachain(Default::default()), PalletInstance(Default::default()), GeneralIndex(Default::default()))
		};

	}: _(RawOrigin::Root, LOCAL_ASSET_ID, asset_multi_location.clone())
	verify {
		let read_asset_multi_location = AssetRegistry::<T>::asset_id_multilocation(LOCAL_ASSET_ID)
			.expect("error reading AssetIdMultiLocation");
		assert_eq!(read_asset_multi_location, asset_multi_location);
	}

	unregister_reserve_asset {
		let asset_multi_location = MultiLocation {
			parents: 1,
			interior: Junctions::X3(Parachain(Default::default()), PalletInstance(Default::default()), GeneralIndex(Default::default()))
		};

		assert_ok!(AssetRegistry::<T>::register_reserve_asset(RawOrigin::Root.into(), LOCAL_ASSET_ID, asset_multi_location.clone()));
		let read_asset_multi_location = AssetRegistry::<T>::asset_id_multilocation(LOCAL_ASSET_ID)
			.expect("error reading AssetIdMultiLocation");
		assert_eq!(read_asset_multi_location, asset_multi_location);

	}: _(RawOrigin::Root, LOCAL_ASSET_ID)
	verify {
		assert_eq!(AssetRegistry::<T>::asset_id_multilocation(LOCAL_ASSET_ID), None);
	}

	impl_benchmark_test_suite!(AssetRegistry, crate::mock::new_test_ext(), crate::mock::Test);
}
