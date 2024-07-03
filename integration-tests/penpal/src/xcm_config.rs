// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Holds the XCM specific configuration that would otherwise be in lib.rs
//!
//! This configuration dictates how the Penpal chain will communicate with other chains.
//!
//! One of the main uses of the penpal chain will be to be a benefactor of reserve asset transfers
//! with Asset Hub as the reserve. At present no derivative tokens are minted on receipt of a
//! `ReserveAssetTransferDeposited` message but that will but the intension will be to support this
//! soon.
use super::{
	AccountId, AllPalletsWithSystem, AssetId as AssetIdPalletAssets, Assets, Balance, Balances, ForeignAssets,
	ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee,
	XcmpQueue,
};
use core::marker::PhantomData;
use frame_support::{
	match_types, parameter_types,
	traits::{
		fungibles::{self, Balanced, Credit},
		ConstU32, Contains, ContainsPair, Everything, Get, Nothing,
	},
	weights::Weight,
};
use frame_system::EnsureRoot;
use pallet_asset_tx_payment::HandleCredit;
use pallet_assets::Instance1;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_common::impls::ToAuthor;
use snowbridge_rococo_common::EthereumNetwork;
use sp_runtime::traits::Zero;
use xcm::latest::prelude::*;
#[allow(deprecated)]
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AsPrefixedGeneralIndex, ConvertedConcreteId, CurrencyAdapter, EnsureXcmOrigin,
	FixedWeightBounds, FrameTransactionalProcessor, FungiblesAdapter, IsConcrete, LocalMint, NativeAsset, NoChecking,
	ParentAsSuperuser, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, StartsWith, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WithComputedOrigin, WithUniqueTopic,
};
use xcm_executor::{traits::JustTry, XcmExecutor};

parameter_types! {
	pub const RelayLocation: Location = Location::parent();
	pub const HereLocation: Location = Location::here();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorMultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
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
#[allow(deprecated)]
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<HereLocation>,
	// Do a simple punn to convert an AccountId32 Location into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type FungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	(
		ConvertedConcreteId<
			AssetIdPalletAssets,
			Balance,
			AsPrefixedGeneralIndex<AssetsPalletLocation, AssetIdPalletAssets, JustTry>,
			JustTry,
		>,
		ConvertedConcreteId<
			AssetIdPalletAssets,
			Balance,
			AsPrefixedGeneralIndex<SystemAssetHubAssetsPalletLocation, AssetIdPalletAssets, JustTry>,
			JustTry,
		>,
	),
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<NonZeroIssuance<AccountId, Assets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// `AssetId/Balance` converter for `TrustBackedAssets`
pub type ForeignAssetsConvertedConcreteId =
	assets_common::ForeignAssetsConvertedConcreteId<StartsWith<RelayLocation>, Balance>;

/// Means for transacting foreign assets from different global consensus.
pub type ForeignFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	ForeignAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ForeignAssetsConvertedConcreteId,
	// Convert an XCM Location into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We dont need to check teleports here.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (CurrencyTransactor, ForeignFungiblesTransactor, FungiblesTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
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
	pub type ParentOrParentsExecutivePlurality: impl Contains<Location> = {
		Location { parents: 1, interior: Here } |
		Location { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
	pub type CommonGoodAssetsParachain: impl Contains<Location> = {
		Location { parents: 1, interior: X1(Parachain(1000)) }
	};
	pub type Polimec: impl Contains<Location> = {
		Location { parents: 1, interior: X1(Parachain(3344)) }
	};
}

pub type Barrier = TrailingSetTopicAsId<(
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Allow XCMs with some computed origins to pass through.
	WithComputedOrigin<
		(
			// If the message is one that immediately attempts to pay for execution, then
			// allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// System Assets parachain, parent and its exec plurality get free
			// execution
			AllowExplicitUnpaidExecutionFrom<(CommonGoodAssetsParachain, ParentOrParentsExecutivePlurality, Polimec)>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<Everything>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
)>;

/// Type alias to conveniently refer to `frame_system`'s `Config::AccountId`.
pub type AccountIdOf<R> = <R as frame_system::Config>::AccountId;

/// Asset filter that allows all assets from a certain location matching asset id.
pub struct AssetPrefixFrom<Prefix, Origin>(PhantomData<(Prefix, Origin)>);
impl<Prefix, Origin> ContainsPair<MultiAsset, Location> for AssetPrefixFrom<Prefix, Origin>
where
	Prefix: Get<Location>,
	Origin: Get<Location>,
{
	fn contains(asset: &MultiAsset, origin: &Location) -> bool {
		let loc = Origin::get();
		&loc == origin &&
			matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
			if asset_loc.starts_with(&Prefix::get()))
	}
}

type AssetsFrom<T> = AssetPrefixFrom<T, T>;

/// Asset filter that allows native/relay asset if coming from a certain location.
pub struct NativeAssetFrom<T>(PhantomData<T>);
impl<T: Get<Location>> ContainsPair<MultiAsset, Location> for NativeAssetFrom<T> {
	fn contains(asset: &MultiAsset, origin: &Location) -> bool {
		let loc = T::get();
		&loc == origin &&
			matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
			if *asset_loc == Location::from(Parent))
	}
}

/// Allow checking in assets that have issuance > 0.
pub struct NonZeroIssuance<AccountId, Assets>(PhantomData<(AccountId, Assets)>);
impl<AccountId, Assets> Contains<<Assets as fungibles::Inspect<AccountId>>::AssetId>
	for NonZeroIssuance<AccountId, Assets>
where
	Assets: fungibles::Inspect<AccountId>,
{
	fn contains(id: &<Assets as fungibles::Inspect<AccountId>>::AssetId) -> bool {
		!Assets::total_issuance(id.clone()).is_zero()
	}
}

/// A `HandleCredit` implementation that naively transfers the fees to the block author.
/// Will drop and burn the assets in case the transfer fails.
pub struct AssetsToBlockAuthor<R>(PhantomData<R>);
impl<R> HandleCredit<AccountIdOf<R>, pallet_assets::Pallet<R, Instance1>> for AssetsToBlockAuthor<R>
where
	R: pallet_authorship::Config + pallet_assets::Config<Instance1>,
	AccountIdOf<R>: From<polkadot_primitives::AccountId> + Into<polkadot_primitives::AccountId>,
{
	fn handle_credit(credit: Credit<AccountIdOf<R>, pallet_assets::Pallet<R, Instance1>>) {
		if let Some(author) = pallet_authorship::Pallet::<R>::author() {
			// In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
			let _ = pallet_assets::Pallet::<R, Instance1>::resolve(&author, credit);
		}
	}
}

// This asset can be added to AH as ForeignAsset and teleported between Penpal and AH
pub const TELEPORTABLE_ASSET_ID: u32 = 2;
parameter_types! {
	/// The location that this chain recognizes as the Relay network's Asset Hub.
	pub SystemAssetHubLocation: Location = Location::new(1, X1(Parachain(1000)));
	// ALWAYS ensure that the index in PalletInstance stays up-to-date with
	// the Relay Chain's Asset Hub's Assets pallet index
	pub SystemAssetHubAssetsPalletLocation: Location =
		Location::new(1, X2(Parachain(1000), PalletInstance(50)));
	pub AssetsPalletLocation: Location =
		Location::new(0, X1(PalletInstance(50)));
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub LocalTeleportableToAssetHub: Location = Location::new(
		0,
		X2(PalletInstance(50), GeneralIndex(TELEPORTABLE_ASSET_ID.into()))
	);
	pub EthereumLocation: Location = Location::new(2, X1(GlobalConsensus(EthereumNetwork::get())));
}

/// Accepts asset with ID `AssetLocation` and is coming from `Origin` chain.
pub struct AssetFromChain<AssetLocation, Origin>(PhantomData<(AssetLocation, Origin)>);
impl<AssetLocation: Get<Location>, Origin: Get<Location>> ContainsPair<MultiAsset, Location>
	for AssetFromChain<AssetLocation, Origin>
{
	fn contains(asset: &MultiAsset, origin: &Location) -> bool {
		log::trace!(target: "xcm::contains", "AssetFromChain asset: {:?}, origin: {:?}", asset, origin);
		*origin == Origin::get() && matches!(asset.id, Concrete(id) if id == AssetLocation::get())
	}
}

pub type Reserves = (
	NativeAsset,
	AssetsFrom<SystemAssetHubLocation>,
	NativeAssetFrom<SystemAssetHubLocation>,
	AssetPrefixFrom<EthereumLocation, SystemAssetHubLocation>,
);

pub type TrustedTeleporters = AssetFromChain<LocalTeleportableToAssetHub, SystemAssetHubLocation>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Aliasers = Nothing;
	type AssetClaims = PolkadotXcm;
	type AssetExchanger = ();
	type AssetLocker = ();
	// How to withdraw and deposit an asset.
	type AssetTransactor = AssetTransactors;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type CallDispatcher = RuntimeCall;
	type FeeManager = ();
	type IsReserve = Reserves;
	// no teleport trust established with other chains
	type IsTeleporter = TrustedTeleporters;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	type SafeCallFilter = Everything;
	type SubscriptionService = PolkadotXcm;
	type Trader = UsingComponents<WeightToFee, RelayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type UniversalAliases = Nothing;
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
)>;

impl pallet_xcm::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
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
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type SovereignAccountOf = LocationToAccountId;
	type TrustedLockers = ();
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

/// Simple conversion of `u32` into an `AssetId` for use in benchmarking.
pub struct XcmBenchmarkHelper;
#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<Location> for XcmBenchmarkHelper {
	fn create_asset_id_parameter(id: u32) -> Location {
		Location { parents: 1, interior: X1(Parachain(id)) }
	}
}
