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

use super::{
	AccountId, AllPalletsWithSystem, AssetId as AssetIdPalletAssets, Balance, Balances, EnsureRoot, ForeignAssets,
	ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, Treasury,
	TreasuryAccount, Vec, WeightToFee, XcmpQueue,
};
use core::marker::PhantomData;
use frame_support::{
	ensure, match_types, parameter_types,
	traits::{ConstU32, Contains, ContainsPair, Everything, Nothing, ProcessMessageError},
	weights::Weight,
};
use pallet_xcm::XcmPassthrough;
use polimec_xcm_executor::{
	polimec_traits::{JustTry, Properties, ShouldExecute},
	XcmExecutor,
};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::traits::MaybeEquivalence;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, CreateMatcher, DenyReserveTransferToRelayChain, DenyThenTry, EnsureXcmOrigin,
	FixedRateOfFungible, FixedWeightBounds, FungibleAdapter, FungiblesAdapter, IsConcrete, MatchXcm,
	MatchedConvertedConcreteId, MintLocation, NoChecking, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation,
	TakeWeightCredit, UsingComponents, WithComputedOrigin,
};

const DOT_ASSET_ID: AssetId = Concrete(RelayLocation::get());
const DOT_PER_SECOND_EXECUTION: u128 = 0_2_000_000_000; // 0.2 DOT per second of execution time
const DOT_PER_MB_PROOF: u128 = 0_2_000_000_000; // 0.0000001 DOT per Megabyte of proof size

const USDT_ASSET_ID: AssetId =
	Concrete(MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)) });
#[allow(unused)]
const USDT_PER_SECOND_EXECUTION: u128 = 1_000_000; // 1 USDT per second of execution time
const USDT_PER_MB_PROOF: u128 = 1_000_000; // 1 USDT per Megabyte of proof size

const USDC_ASSET_ID: AssetId =
	Concrete(MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1337)) });
#[allow(unused)]
const USDC_PER_SECOND_EXECUTION: u128 = 1_000_000; // 1 USDC per second of execution time
const USDC_PER_MB_PROOF: u128 = 1_000_000; // 1 USDC per Megabyte of proof size

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorMultiLocation = (
		GlobalConsensus(Polkadot),
		 Parachain(ParachainInfo::parachain_id().into()),
	).into();
	pub const HereLocation: MultiLocation = MultiLocation::here();
	pub AssetHubLocation: MultiLocation = (Parent, Parachain(1000)).into();
	pub CheckAccount: AccountId = PolkadotXcm::check_account();
	/// The check account that is allowed to mint assets locally. Used for PLMC teleport
	/// checking once enabled.
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);

	pub const DotTraderParams: (AssetId, u128, u128) = (DOT_ASSET_ID, DOT_PER_SECOND_EXECUTION, DOT_PER_MB_PROOF);
	pub const UsdtTraderParams: (AssetId, u128, u128) = (USDT_ASSET_ID, USDT_PER_SECOND_EXECUTION, USDT_PER_MB_PROOF);
	pub const UsdcTraderParams: (AssetId, u128, u128) = (USDC_ASSET_ID, USDC_PER_SECOND_EXECUTION, USDC_PER_MB_PROOF);
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<HereLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// Check teleport accounting once we enable PLMC teleports
	LocalCheckAccount,
>;

// The `AssetIdPalletAssets` ids that are supported by this chain.
// Currently, we only support DOT (0), USDT (1984) and USDC (1337).
match_types! {
	pub type SupportedAssets: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1337)) } |
		MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)) }
	};
}

impl MaybeEquivalence<MultiLocation, AssetIdPalletAssets> for SupportedAssets {
	fn convert(asset: &MultiLocation) -> Option<AssetIdPalletAssets> {
		match asset {
			MultiLocation { parents: 1, interior: Here } => Some(10),
			MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1337)) } =>
				Some(1337),
			MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)) } =>
				Some(1984),
			_ => None,
		}
	}

	fn convert_back(value: &AssetIdPalletAssets) -> Option<MultiLocation> {
		match value {
			10 => Some(MultiLocation { parents: 1, interior: Here }),
			1337 => Some(MultiLocation {
				parents: 1,
				interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1337)),
			}),
			1984 => Some(MultiLocation {
				parents: 1,
				interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)),
			}),
			_ => None,
		}
	}
}

/// Foreign assets adapter for supporting assets from other chains. Currently the only
/// supported assets are DOT, USDT, and USDC.
pub type ForeignAssetsAdapter = FungiblesAdapter<
	// Use this fungibles implementation:
	ForeignAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	MatchedConvertedConcreteId<AssetIdPalletAssets, Balance, SupportedAssets, SupportedAssets, JustTry>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We do not allow teleportation of foreign assets. We only allow the reserve-based
	// transfer of USDT, USDC and DOT.
	NoChecking,
	// The account to use for tracking teleports.
	CheckAccount,
>;

pub struct AssetHubAssetsAsReserve;
impl ContainsPair<MultiAsset, MultiLocation> for AssetHubAssetsAsReserve {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		// location must be the AssetHub parachain
		let asset_hub_loc = AssetHubLocation::get();
		if &asset_hub_loc != origin {
			return false
		}
		match asset.id {
			Concrete(id) => SupportedAssets::contains(&id),
			_ => false,
		}
	}
}
impl Contains<(MultiLocation, Vec<MultiAsset>)> for AssetHubAssetsAsReserve {
	fn contains(item: &(MultiLocation, Vec<MultiAsset>)) -> bool {
		// We allow all signed origins to send back the AssetHub reserve assets.
		let (_, assets) = item;
		assets.iter().all(|asset| match asset.id {
			Concrete(id) => SupportedAssets::contains(&id),
			_ => false,
		})
	}
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
	pub type CommonGoodAssetsParachain: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: X1(Parachain(1000)) }
	};
	pub type ParentOrSiblings: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Parachain(_)) }
	};
}

pub type Barrier = DenyThenTry<
	DenyReserveTransferToRelayChain,
	(
		TakeWeightCredit,
		// Expected responses are OK.
		AllowKnownQueryResponses<PolkadotXcm>,
		// Allow XCMs with some computed origins to pass through.
		WithComputedOrigin<
			(
				// HRMP notifications from relay get free pass
				AllowHrmpNotifications<ParentOrParentsExecutivePlurality>,
				// If the message is one that immediately attemps to pay for execution, then allow it.
				AllowTopLevelPaidExecutionFrom<Everything>,
				// Common Good Assets parachain, parent and its exec plurality get free execution
				AllowExplicitUnpaidExecutionFrom<(CommonGoodAssetsParachain, ParentOrParentsExecutivePlurality)>,
				// Subscriptions for version tracking are OK.
				AllowSubscriptionsFrom<ParentOrSiblings>,
			),
			UniversalLocation,
			ConstU32<8>,
		>,
	),
>;

/// Trusted reserve locations for reserve assets. For now we only trust the AssetHub parachain
/// for the following assets: DOT, USDT, USDC.
pub type Reserves = AssetHubAssetsAsReserve;

/// Means for transacting assets on this chain.
/// FungibleTransactor is a FungibleAdapter that allows for transacting PLMC.
/// ForeignAssetsAdapter is a FungiblesAdapter that allows for transacting foreign assets.
/// Currently we only support DOT, USDT and USDC.
pub type AssetTransactors = (FungibleTransactor, ForeignAssetsAdapter);
pub type TakeRevenueToTreasury =
	cumulus_primitives_utility::XcmFeesTo32ByteAccount<AssetTransactors, AccountId, TreasuryAccount>;
pub struct XcmConfig;
impl polimec_xcm_executor::Config for XcmConfig {
	type Aliasers = ();
	type AssetClaims = PolkadotXcm;
	type AssetExchanger = ();
	type AssetLocker = ();
	// How to withdraw and deposit an asset.
	type AssetTransactor = AssetTransactors;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type CallDispatcher = RuntimeCall;
	type FeeManager = ();
	type HrmpHandler = ();
	/// Locations that we trust to act as reserves for specific assets.
	type IsReserve = Reserves;
	/// Currently we do not support teleportation of PLMC or other assets.
	type IsTeleporter = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	// Do not allow any Transact instructions to be executed on our chain.
	type SafeCallFilter = Nothing;
	type SubscriptionService = PolkadotXcm;
	type Trader = (
		// TODO: weight to fee has to be carefully considered. For now use default
		UsingComponents<WeightToFee, HereLocation, AccountId, Balances, Treasury>,
		FixedRateOfFungible<UsdtTraderParams, TakeRevenueToTreasury>,
		FixedRateOfFungible<DotTraderParams, TakeRevenueToTreasury>,
		FixedRateOfFungible<UsdcTraderParams, TakeRevenueToTreasury>,
	);
	type UniversalAliases = Nothing;
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// Conservative weight values for XCM extrinsics. Should eventually be adjusted by benchmarking.
pub struct XcmWeightInfo;
impl pallet_xcm::WeightInfo for XcmWeightInfo {
	fn send() -> Weight {
		Weight::from_parts(500_000_000, 10000)
	}

	fn teleport_assets() -> Weight {
		Weight::from_parts(100_000_000, 10000)
	}

	fn reserve_transfer_assets() -> Weight {
		Weight::from_parts(100_000_000, 10000)
	}

	fn transfer_assets() -> Weight {
		Weight::from_parts(1_500_000_000, 10000)
	}

	// Disables any custom local execution of XCM messages. Same value as system parachains.
	fn execute() -> Weight {
		Weight::from_parts(18_446_744_073_709_551_000, 0)
	}

	fn force_xcm_version() -> Weight {
		Weight::from_parts(200_000_000, 10000)
	}

	fn force_default_xcm_version() -> Weight {
		Weight::from_parts(200_000_000, 10000)
	}

	fn force_subscribe_version_notify() -> Weight {
		Weight::from_parts(1_000_000_000, 10000)
	}

	fn force_unsubscribe_version_notify() -> Weight {
		Weight::from_parts(1_000_000_000, 10000)
	}

	fn force_suspension() -> Weight {
		Weight::from_parts(200_000_000, 10000)
	}

	fn migrate_supported_version() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}

	fn migrate_version_notifiers() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}

	fn already_notified_target() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}

	fn notify_current_targets() -> Weight {
		Weight::from_parts(1_000_000_000, 20000)
	}

	fn notify_target_migration_fail() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}

	fn migrate_version_notify_targets() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}

	fn migrate_and_notify_old_targets() -> Weight {
		Weight::from_parts(1_000_000_000, 20000)
	}

	fn new_query() -> Weight {
		Weight::from_parts(500_000_000, 10000)
	}

	fn take_response() -> Weight {
		Weight::from_parts(500_000_000, 20000)
	}
}

impl pallet_xcm::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
	type SovereignAccountOf = LocationToAccountId;
	type TrustedLockers = ();
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = XcmWeightInfo;
	type XcmExecuteFilter = Nothing;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	// We only allow reserve based transfers of AssetHub reserve assets back to AssetHub.
	type XcmReserveTransferFilter = AssetHubAssetsAsReserve;
	type XcmRouter = XcmRouter;
	// We do not allow teleportation of PLMC or other assets.
	// TODO: change this once we enable PLMC teleports
	type XcmTeleportFilter = Nothing;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct AllowHrmpNotifications<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>> ShouldExecute for AllowHrmpNotifications<T> {
	fn should_execute<Call>(
		origin: &MultiLocation,
		instructions: &mut [Instruction<Call>],
		max_weight: Weight,
		_weight_credit: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		log::trace!(
			target: "xcm::barriers",
			"AllowHrmpNotifications origin: {:?}, instructions: {:?}, max_weight: {:?}, weight_credit: {:?}",
			origin, instructions, max_weight, _weight_credit,
		);
		ensure!(T::contains(origin), ProcessMessageError::Unsupported);
		instructions.matcher().assert_remaining_insts(1)?.match_next_inst(|inst| match inst {
			HrmpNewChannelOpenRequest { .. } => Ok(()),
			HrmpChannelAccepted { .. } => Ok(()),
			HrmpChannelClosing { .. } => Ok(()),
			_ => Err(ProcessMessageError::Unsupported),
		})?;
		Ok(())
	}
}
