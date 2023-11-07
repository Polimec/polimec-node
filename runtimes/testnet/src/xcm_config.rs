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

//! Holds the XCM specific configuration that would otherwise be in lib.rs
//!
//! This configuration dictates how the Penpal chain will communicate with other chains.
//!
//! One of the main uses of the penpal chain will be to be a benefactor of reserve asset transfers
//! with statemine as the reserve. At present no derivative tokens are minted on receipt of a
//! ReserveAssetTransferDeposited message but that will but the intension will be to support this soon.
use core::marker::PhantomData;
use frame_support::{
	ensure, match_types, parameter_types,
	traits::{
		fungibles::{self, Balanced, Credit},
		ConstU32, Contains, ContainsPair, Everything, Get, Nothing, ProcessMessageError,
	},
	weights::Weight,
};
use pallet_asset_tx_payment::HandleCredit;
use pallet_xcm::XcmPassthrough;
use parachains_common::xcm_config::{DenyReserveTransferToRelayChain, DenyThenTry};
use polimec_xcm_executor::XcmExecutor;
use polkadot_parachain::primitives::Sibling;
use polkadot_runtime_common::impls::ToAuthor;
use sp_runtime::traits::Zero;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, AsPrefixedGeneralIndex, ConvertedConcreteId, CreateMatcher, CurrencyAdapter,
	EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds, FungiblesAdapter, IsConcrete, LocalMint, MatchXcm,
	NativeAsset, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
	WithComputedOrigin,
};
use xcm_executor::traits::{Convert, Error, JustTry, MatchesFungibles, ShouldExecute};

use super::{
	AccountId, AllPalletsWithSystem, AssetId as AssetIdPalletAssets, Balance, Balances, EnsureRoot, ParachainInfo,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, StatemintAssets, WeightToFee,
	XcmpQueue,
};
// TODO: Check these numbers
const DOT_ASSET_ID: AssetId = Concrete(RelayLocation::get());
const DOT_PER_SECOND_EXECUTION: u128 = 0_0_000_001_000; // 0.0000001 DOT per second of execution time
const DOT_PER_MB_PROOF: u128 = 0_0_000_001_000; // 0.0000001 DOT per Megabyte of proof size

const USDT_ASSET_ID: AssetId =
	Concrete(MultiLocation { parents: 1, interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)) });
// const USDT_PER_SECOND_EXECUTION: u128 = 0_0_000_001_000; // 0.0000001 USDT per second of execution time

parameter_types! {
	pub const RelayLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorMultiLocation = (
		GlobalConsensus(Polkadot),
		 Parachain(ParachainInfo::parachain_id().into()),
	).into();
	pub const HereLocation: MultiLocation = MultiLocation::here();

	pub const DotTraderParams: (AssetId, u128, u128) = (DOT_ASSET_ID, DOT_PER_SECOND_EXECUTION, DOT_PER_MB_PROOF);
	pub const UsdtTraderParams: (AssetId, u128, u128) = (USDT_ASSET_ID, DOT_PER_SECOND_EXECUTION, DOT_PER_MB_PROOF);
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
	IsConcrete<HereLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// Means for transacting assets besides the native currency on this chain.
pub type StatemintFungiblesTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	StatemintAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ConvertedConcreteId<
		AssetIdPalletAssets,
		Balance,
		AsPrefixedGeneralIndex<CommonGoodAssetsPalletLocation, AssetIdPalletAssets, JustTry>,
		JustTry,
	>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<NonZeroIssuance<AccountId, StatemintAssets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

// asset id "0" is reserved for relay chain native token in the pallet assets
pub struct NativeToFungible;
impl Convert<MultiLocation, AssetIdPalletAssets> for NativeToFungible {
	fn convert(asset: MultiLocation) -> Result<AssetIdPalletAssets, MultiLocation> {
		match asset {
			MultiLocation { parents: 1, interior: Here } =>
				Ok(pallet_funding::types::AcceptedFundingAsset::DOT.to_statemint_id()),
			_ => Err(asset),
		}
	}

	fn reverse(value: AssetIdPalletAssets) -> Result<MultiLocation, AssetIdPalletAssets> {
		if value == 0u32 {
			Ok(MultiLocation { parents: 1, interior: Here })
		} else {
			Err(value)
		}
	}
}

// We need the matcher to return either `AssetNotFound` or `Unimplemented` so that the tuple impl of
// TransactAsset will move on to the next struct.
pub struct NonBlockingConvertedConcreteId<AssetId, Balance, ConvertAssetId, ConvertOther>(
	PhantomData<(AssetId, Balance, ConvertAssetId, ConvertOther)>,
);
impl<
		AssetId: Clone,
		Balance: Clone,
		ConvertAssetId: Convert<MultiLocation, AssetId>,
		ConvertBalance: Convert<u128, Balance>,
	> MatchesFungibles<AssetId, Balance>
	for NonBlockingConvertedConcreteId<AssetId, Balance, ConvertAssetId, ConvertBalance>
{
	fn matches_fungibles(a: &MultiAsset) -> Result<(AssetId, Balance), Error> {
		ConvertedConcreteId::<AssetId, Balance, ConvertAssetId, ConvertBalance>::matches_fungibles(a)
			.map_err(|_| Error::AssetNotHandled)
	}
}

pub type StatemintDotTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	StatemintAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	NonBlockingConvertedConcreteId<AssetIdPalletAssets, Balance, NativeToFungible, JustTry>,
	// Convert an XCM MultiLocation into a local account id:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<NonZeroIssuance<AccountId, StatemintAssets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (StatemintDotTransactor, CurrencyTransactor, StatemintFungiblesTransactor);

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
				AllowSubscriptionsFrom<Everything>,
			),
			UniversalLocation,
			ConstU32<8>,
		>,
	),
>;

/// Type alias to conveniently refer to `frame_system`'s `Config::AccountId`.
pub type AccountIdOf<R> = <R as frame_system::Config>::AccountId;

/// Asset filter that allows all assets from a certain location.
pub struct AssetsFrom<T>(PhantomData<T>);

pub struct StatemintAssetsFilter;
impl ContainsPair<MultiAsset, MultiLocation> for StatemintAssetsFilter {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		// location must be the statemint parachain
		let loc = MultiLocation::new(1, X1(Parachain(1000)));
		// asset must be either a fungible asset from `pallet_assets` or the native token of the relay chain
		&loc == origin &&
			match asset {
				MultiAsset {
					id:
						Concrete(MultiLocation {
							parents: 1,
							interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(_)),
						}),
					..
				} => true,
				MultiAsset { id: Concrete(MultiLocation { parents: 1, interior: Here }), .. } => true,

				_ => false,
			}
	}
}

impl<T: Get<MultiLocation>> ContainsPair<MultiAsset, MultiLocation> for AssetsFrom<T> {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		let loc = T::get();
		&loc == origin &&
			matches!(asset, MultiAsset { id: AssetId::Concrete(asset_loc), fun: Fungible(_a) }
			if asset_loc.match_and_split(&loc).is_some())
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
pub struct AssetsToBlockAuthor<R, I>(PhantomData<(R, I)>);
impl<R, I> HandleCredit<AccountIdOf<R>, pallet_assets::Pallet<R, I>> for AssetsToBlockAuthor<R, I>
where
	I: 'static,
	R: pallet_authorship::Config + pallet_assets::Config<I>,
	AccountIdOf<R>: From<polkadot_primitives::AccountId> + Into<polkadot_primitives::AccountId>,
{
	fn handle_credit(credit: Credit<AccountIdOf<R>, pallet_assets::Pallet<R, I>>) {
		if let Some(author) = pallet_authorship::Pallet::<R>::author() {
			// In case of error: Will drop the result triggering the `OnDrop` of the imbalance.
			let _ = pallet_assets::Pallet::<R, I>::resolve(&author, credit);
		}
	}
}

pub trait Reserve {
	/// Returns assets reserve location.
	fn reserve(&self) -> Option<MultiLocation>;
}

// Takes the chain part of a MultiAsset
impl Reserve for MultiAsset {
	fn reserve(&self) -> Option<MultiLocation> {
		if let AssetId::Concrete(location) = self.id {
			let first_interior = location.first_interior();
			let parents = location.parent_count();
			match (parents, first_interior) {
				(0, Some(Parachain(id))) => Some(MultiLocation::new(0, X1(Parachain(*id)))),
				(1, Some(Parachain(id))) => Some(MultiLocation::new(1, X1(Parachain(*id)))),
				(1, _) => Some(MultiLocation::parent()),
				_ => None,
			}
		} else {
			None
		}
	}
}

/// A `FilterAssetLocation` implementation. Filters multi native assets whose
/// reserve is same with `origin`.
pub struct MultiNativeAsset;
impl ContainsPair<MultiAsset, MultiLocation> for MultiNativeAsset {
	fn contains(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref reserve) = asset.reserve() {
			if reserve == origin {
				return true
			}
		}
		false
	}
}

parameter_types! {
	pub CommonGoodAssetsLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(1000)));
	// ALWAYS ensure that the index in PalletInstance stays up-to-date with
	// Statemint's Assets pallet index
	pub CommonGoodAssetsPalletLocation: MultiLocation =
		MultiLocation::new(1, X2(Parachain(1000), PalletInstance(50)));
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

pub type Reserves = (NativeAsset, StatemintAssetsFilter);

pub struct XcmConfig;

impl polimec_xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetExchanger = ();
	type AssetLocker = ();
	// How to withdraw and deposit an asset.
	type AssetTransactor = AssetTransactors;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type CallDispatcher = RuntimeCall;
	type FeeManager = ();
	type HrmpHandler = pallet_funding::xcm_executor_impl::HrmpHandler<Runtime>;
	type IsReserve = Reserves;
	type IsTeleporter = NativeAsset;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	// TODO: Restrict this to a subset of allowed `RuntimeCall`.
	type SafeCallFilter = Everything;
	type SubscriptionService = PolkadotXcm;
	type Trader = (
		// TODO: weight to fee has to be carefully considered. For now use default
		UsingComponents<WeightToFee, HereLocation, AccountId, Balances, ToAuthor<Runtime>>,
		FixedRateOfFungible<UsdtTraderParams, ()>,
		FixedRateOfFungible<DotTraderParams, ()>,
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

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
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
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
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
	// TODO: change back to `Nothing` once we add the xcm functionalities into a pallet
	type XcmExecuteFilter = Everything;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
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

pub struct AllowHrmpNotifications<T>(PhantomData<T>);
impl<T: Contains<MultiLocation>> ShouldExecute for AllowHrmpNotifications<T> {
	fn should_execute<Call>(
		origin: &MultiLocation,
		instructions: &mut [Instruction<Call>],
		max_weight: Weight,
		_weight_credit: &mut Weight,
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
