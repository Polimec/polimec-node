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

use super::{AccountId, ParachainInfo, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin};
use frame_support::{match_types, parameter_types};
use xcm::latest::prelude::*;
use xcm_builder::{
	AllowUnpaidExecutionFrom, FixedWeightBounds, LocationInverter, ParentAsSuperuser,
	ParentIsPreset, SovereignSignedViaLocation,
};

parameter_types! {
	pub const RococoLocation: MultiLocation = MultiLocation::parent();
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<ParentIsPreset<AccountId>, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
);

match_types! {
	pub type JustTheParent: impl Contains<MultiLocation> = { MultiLocation { parents:1, interior: Here } };
}

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: u64 = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = (); // sending XCM not supported
	type AssetTransactor = (); // balances not supported
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = (); // balances not supported
	type IsTeleporter = (); // balances not supported
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = AllowUnpaidExecutionFrom<JustTheParent>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>; // balances not supported
	type Trader = (); // balances not supported
	type ResponseHandler = (); // Don't handle responses for now.
	type AssetTrap = (); // don't trap for now
	type AssetClaims = (); // don't claim for now
	type SubscriptionService = (); // don't handle subscriptions for now
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = xcm_executor::XcmExecutor<XcmConfig>;
}
