//! Holds the XCM specific configuration that would otherwise be in lib.rs
//!
//! This configuration dictates how the Polimec will communicate with other chains.
//!
//! One of the main uses of Polimec will be to be a benefactor of reserve asset transfers
//! with Statemine as the reserve.
//! At present no derivative tokens are minted on receipt of a
//! `ReserveAssetTransferDeposited` message but that will but the intension will be to support this

use super::{
	AccountId, AssetId as AssetIdPalletAssets, AssetRegistry, Assets, Balance, Balances,
	ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
	Treasury, WeightToFee, XcmpQueue,
};
use core::marker::PhantomData;
use frame_support::{
	match_types, parameter_types,
	traits::{Everything, Get, PalletInfoAccess},
};
use pallet_xcm::XcmPassthrough;
use parachains_common::{
	impls::NonZeroIssuance,
	xcm_config::{DenyReserveTransferToRelayChain, DenyThenTry},
};
use polkadot_parachain::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom, AsPrefixedGeneralIndex,
	ConvertedConcreteAssetId, CurrencyAdapter, EnsureXcmOrigin, FixedWeightBounds,
	FungiblesAdapter, IsConcrete, LocationInverter, NativeAsset, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
	UsingComponents,
};
use xcm_executor::{
	traits::{FilterAssetLocation, JustTry},
	XcmExecutor,
};
use xcm_primitives::{AsAssetMultiLocation, ConvertedRegisteredAssetId};

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Any;
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub AssetsPalletLocation: MultiLocation =
		PalletInstance(<Assets as PalletInfoAccess>::index() as u8).into();
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: u64 = 1_000_000_000;
	pub const MaxInstructions: u32 = 100;
	// ALWAYS ensure that the index in PalletInstance stays up-to-date with
	// Statemint's Assets pallet index
	pub CommonGoodAssetsPalletLocation: MultiLocation =
		MultiLocation::new(1, X2(Parachain(1000), PalletInstance(50)));
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
	pub type CommonGoodAssetsParachain: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: X1(Parachain(1000)) }
	};
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
pub type CurrencyTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// Means for transacting reserved fungible assets.
/// AsAssetMultiLocation uses pallet_asset_registry to convert between AssetId and MultiLocation.
pub type ReservedFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a registered fungible asset matching the given location or name
	// Assets not found in AssetRegistry will not be used
	ConvertedRegisteredAssetId<
		AssetIdPalletAssets,
		Balance,
		AsAssetMultiLocation<AssetIdPalletAssets, AssetRegistry>,
		JustTry,
	>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	NonZeroIssuance<AccountId, Assets>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting assets besides the native currency on this chain.
pub type LocalFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ConvertedConcreteAssetId<
		AssetIdPalletAssets,
		Balance,
		AsPrefixedGeneralIndex<AssetsPalletLocation, AssetIdPalletAssets, JustTry>,
		JustTry,
	>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	NonZeroIssuance<AccountId, Assets>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors =
	(CurrencyTransactor, ReservedFungiblesTransactor, LocalFungiblesTransactor);

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

pub type Barrier = DenyThenTry<
	DenyReserveTransferToRelayChain,
	(
		TakeWeightCredit,
		AllowTopLevelPaidExecutionFrom<Everything>,
		// Parent and its exec plurality get free execution
		AllowUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
		// Assets Common Good parachain gets free execution
		AllowUnpaidExecutionFrom<CommonGoodAssetsParachain>,
		// Expected responses are OK.
		AllowKnownQueryResponses<PolkadotXcm>,
		// Subscriptions for version tracking are OK.
		AllowSubscriptionsFrom<Everything>,
	),
>;

parameter_types! {
	pub StatemineLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(1000)));
}

// MARK: Reserve

/// Asset filter that allows all assets from a certain location.
pub struct AssetsFrom<T>(PhantomData<T>);
impl<T: Get<MultiLocation>> FilterAssetLocation for AssetsFrom<T> {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		let loc = T::get();
		&loc == origin &&
			matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
			if asset_loc.match_and_split(&loc).is_some())
	}
}

pub type Reserves = (NativeAsset, AssetsFrom<StatemineLocation>);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;

	/// How to send an onward XCM message.
	type XcmSender = XcmRouter;

	// How to withdraw and deposit an asset.
	type AssetTransactor = AssetTransactors;

	/// How to get a call origin from a `OriginKind` value.
	type OriginConverter = XcmOriginToTransactDispatchOrigin;

	/// Combinations of (Location, Asset) pairs which we trust as reserves.
	type IsReserve = Reserves;

	/// Combinations of (Location, Asset) pairs which we trust as teleporters.
	type IsTeleporter = AssetsFrom<StatemineLocation>;

	/// Means of inverting a location.
	type LocationInverter = LocationInverter<Ancestry>;

	/// Whether we should execute the given XCM at all.
	type Barrier = Barrier;

	/// The means of determining an XCM message's weight.
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;

	/// The means of purchasing weight credit for XCM execution.
	/// The fees are not taken out of the Balances pallet here.
	/// Balances is only used if fees are dropped without being
	/// used. In that case they are put into the treasury.
	type Trader =
		UsingComponents<WeightToFee<Runtime>, RelayLocation, AccountId, Balances, Treasury>;

	/// What to do when a response of a query is found.
	type ResponseHandler = PolkadotXcm;

	/// The general asset trap - handler for when assets are left in the Holding Register at the
	/// end of execution.
	type AssetTrap = PolkadotXcm;

	/// The handler for when there is an instruction to claim assets.
	type AssetClaims = PolkadotXcm;

	/// How we handle version subscription requests.
	type SubscriptionService = PolkadotXcm;
}

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;

	/// Required origin for sending XCM messages. If successful, it resolves to `MultiLocation`
	/// which exists as an interior location within this chain's XCM context.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;

	/// The type used to actually dispatch an XCM to its destination.
	type XcmRouter = XcmRouter;

	/// Required origin for executing XCM messages, including the teleport functionality. If successful,
	/// then it resolves to `MultiLocation` which exists as an interior location within this chain's XCM
	/// context.
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;

	/// Our XCM filter which messages to be executed using `XcmExecutor` must pass.
	type XcmExecuteFilter = Everything;
	// ^ Enable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing, `Nothing` by default.

	/// Something to execute an XCM message.
	type XcmExecutor = XcmExecutor<XcmConfig>;

	/// Our XCM filter which messages to be teleported using the dedicated extrinsic must pass.
	type XcmTeleportFilter = Everything;

	/// Our XCM filter which messages to be reserve-transferred using the dedicated extrinsic must pass.
	type XcmReserveTransferFilter = Everything;

	/// Means of measuring the weight consumed by an XCM message locally.
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;

	/// Means of inverting a location.
	type LocationInverter = LocationInverter<Ancestry>;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default

	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}