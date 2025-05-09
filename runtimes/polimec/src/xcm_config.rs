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
	AccountId, AllPalletsWithSystem, Balance, Balances, ContributionTokens, EnsureRoot, ForeignAssets,
	HereToForeignAsset, ParachainInfo, ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
	TreasuryAccount, Vec, WeightToFee,
};
use core::marker::PhantomData;
use cumulus_primitives_core::ParaId;
use frame_support::{
	ensure,
	pallet_prelude::*,
	parameter_types,
	traits::{
		tokens::ConversionToAssetBalance, ConstU32, Contains, ContainsPair, Disabled, Everything, Nothing,
		ProcessMessageError,
	},
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use pallet_xcm::XcmPassthrough;
use parachains_common::xcm_config::{AllSiblingSystemParachains, ParentRelayOrSiblingParachains};
use polimec_common::assets::AcceptedFundingAsset;
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use sp_runtime::traits::{TryConvertInto, Zero};
use xcm::v5::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DenyReserveTransferToRelayChain, DenyThenTry, DescribeAllTerminal, DescribeFamily,
	EnsureXcmOrigin, FixedWeightBounds, FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter,
	GlobalConsensusParachainConvertsFor, HashedDescription, IsConcrete, MatchedConvertedConcreteId, MintLocation,
	NoChecking, ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, StartsWith,
	StartsWithExplicitGlobalConsensus, TakeRevenue, TakeWeightCredit, TrailingSetTopicAsId, WithComputedOrigin,
	WithLatestLocationConverter,
};
use xcm_executor::{
	traits::{Properties, ShouldExecute, WeightTrader},
	AssetsInHolding, XcmExecutor,
};
parameter_types! {
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub UniversalLocation: InteriorLocation =
		[GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
	pub const HereLocation: Location = Location::here();
	pub AssetHubLocation: Location = (Parent, Parachain(1000)).into();
	pub UniversalLocationNetworkId: NetworkId = UniversalLocation::get().global_consensus().unwrap();

	pub CheckAccount: AccountId = PolkadotXcm::check_account();
	/// The check account that is allowed to mint assets locally. Used for PLMC teleport
	/// checking once enabled.
	pub LocalCheckAccount: (AccountId, MintLocation) = (CheckAccount::get(), MintLocation::Local);

	pub ForeignAssetsPalletIndex: u8 = <ForeignAssets as PalletInfoAccess>::index() as u8;
	pub ForeignAssetsPalletLocation: Location = PalletInstance(ForeignAssetsPalletIndex::get()).into();

	pub ContributionTokensPalletIndex: u8 = <ContributionTokens as PalletInfoAccess>::index() as u8;
	pub ContributionTokensPalletLocation: Location = PalletInstance(ContributionTokensPalletIndex::get()).into();
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
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
	// Different global consensus parachain sovereign account.
	// (Used for over-bridge transfers and reserve processing)
	GlobalConsensusParachainConvertsFor<UniversalLocation, AccountId>,
);

/// Means for transacting assets on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<HereLocation>,
	// Do a simple punn to convert an AccountId32 Location into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// Check teleport accounting once we enable PLMC teleports
	LocalCheckAccount,
>;

pub type ForeignAssetsConvertedConcreteId = assets_common::ForeignAssetsConvertedConcreteId<
	(
		// Ignore `TrustBackedAssets` explicitly
		StartsWith<ForeignAssetsPalletLocation>,
		// Ignore assets that start explicitly with our `GlobalConsensus(NetworkId)`, means:
		// - foreign assets from our consensus should be: `Location {parents: 1, X*(Parachain(xyz),
		//   ..)}`
		// - foreign assets outside our consensus with the same `GlobalConsensus(NetworkId)` won't
		//   be accepted here
		StartsWithExplicitGlobalConsensus<UniversalLocationNetworkId>,
	),
	Balance,
	xcm::v4::Location,
>;

/// `AssetId`/`Balance` converter for `ContributionTokens`.
pub type ContributionTokensConvertedConcreteId =
	assets_common::TrustBackedAssetsConvertedConcreteId<ContributionTokensPalletLocation, Balance>;

// The `AssetIdPalletAssets` ids that are supported by this chain.
// Currently, we only support DOT (10), USDT (1984) and USDC (1337).
pub struct SupportedAssets;
impl frame_support::traits::Contains<Location> for SupportedAssets {
	fn contains(l: &Location) -> bool {
		let funding_assets = AcceptedFundingAsset::all_ids();
		l.clone().try_into().ok().is_some_and(|v4_location| funding_assets.contains(&v4_location))
	}
}

/// Foreign assets adapter for supporting assets from other chains. Currently the only
/// supported assets are DOT, USDT, and USDC.
pub type ForeignAssetsAdapter = FungiblesAdapter<
	// Use this fungibles implementation:
	ForeignAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	MatchedConvertedConcreteId<
		xcm::v4::Location,
		Balance,
		SupportedAssets,
		WithLatestLocationConverter<xcm::v4::Location>,
		TryConvertInto,
	>,
	// Convert an XCM Location into a local account id:
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
impl ContainsPair<Asset, Location> for AssetHubAssetsAsReserve {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		// The `origin` location must be Polkadot Asset Hub
		let asset_hub_loc = AssetHubLocation::get();
		if &asset_hub_loc != origin {
			return false;
		}
		SupportedAssets::contains(&asset.id.0)
	}
}
impl Contains<(Location, Vec<Asset>)> for AssetHubAssetsAsReserve {
	fn contains(item: &(Location, Vec<Asset>)) -> bool {
		// We allow all signed origins to send back the AssetHub reserve assets.
		let (_, assets) = item;
		assets.iter().all(|asset| SupportedAssets::contains(&asset.id.0))
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

pub struct ParentOrParentsPlurality;
impl Contains<Location> for ParentOrParentsPlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
	}
}

/// Allows messages starting with DescendOrigin(AccountId32) from AssetHub
/// followed immediately by BuyExecution.
pub struct AllowPaidDescendFromAssetHub;
impl ShouldExecute for AllowPaidDescendFromAssetHub {
	fn should_execute<RuntimeCall>(
		origin: &Location,
		instructions: &mut [Instruction<RuntimeCall>],
		_max_weight: Weight,
		_properties: &mut Properties,
	) -> Result<(), ProcessMessageError> {
		log::trace!(target: "xcm::barrier", "AllowPaidDescendFromAssetHub checking origin: {:?}, instructions: {:?}", origin, instructions);

		// 1. Check the origin is Asset Hub
		let expected_origin = AssetHubLocation::get().into_versioned();
		let origin_location = origin.clone().into_versioned();
		if origin_location != expected_origin {
			log::trace!(target: "xcm::barrier", "AllowPaidDescendFromAssetHub: Origin mismatch");
			return Err(ProcessMessageError::Unsupported); // Doesn't match, fail this barrier path
		}

		// 2. Check the first two instructions
		if instructions.len() < 2 {
			log::trace!(target: "xcm::barrier", "AllowPaidDescendFromAssetHub: Not enough instructions");
			return Err(ProcessMessageError::Unsupported);
		}

		// Note: Maybe restrict to X1(AccountId32 { id: SPECIFIC_ACCOUNT })?
		let first_instr_matches = matches!(
			instructions[0],
			Instruction::DescendOrigin(Junctions::X1(ref arc_val))
			if matches!(arc_val.as_ref(), &[Junction::AccountId32 { .. }])
		);
		let second_instr_matches = matches!(instructions[1], Instruction::BuyExecution { .. });

		if first_instr_matches && second_instr_matches {
			log::trace!(target: "xcm::barrier", "AllowPaidDescendFromAssetHub: Pattern matched, allowing.");
			Ok(())
		} else {
			log::trace!(target: "xcm::barrier", "AllowPaidDescendFromAssetHub: Instruction pattern mismatch");
			Err(ProcessMessageError::Unsupported)
		}
	}
}

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			TakeWeightCredit,
			// Allow DescendOrigin(AccountId32) from Asset Hub
			AllowPaidDescendFromAssetHub,
			// Expected responses are OK.
			AllowKnownQueryResponses<PolkadotXcm>,
			// Allow XCMs with some computed origins to pass through.
			WithComputedOrigin<
				(
					// If the message is one that immediately attemps to pay for execution, then allow it.
					AllowTopLevelPaidExecutionFrom<Everything>,
					// System parachains, parent and its exec plurality get free execution
					AllowExplicitUnpaidExecutionFrom<(AllSiblingSystemParachains, ParentOrParentsPlurality)>,
					// Subscriptions for version tracking are OK.
					AllowSubscriptionsFrom<ParentRelayOrSiblingParachains>,
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

/// Trusted reserve locations for reserve assets. For now we only trust the AssetHub parachain
/// for the following assets: DOT, USDT and USDC.
pub type Reserves = AssetHubAssetsAsReserve;

pub struct HereToHub;
impl ContainsPair<Asset, Location> for HereToHub {
	fn contains(asset: &Asset, location: &Location) -> bool {
		// We only allow teleportation of PLMC to the AssetHub parachain.
		asset.id.0 == Location::here() && location == &AssetHubLocation::get()
	}
}

pub struct TeleportFilter;
impl Contains<(Location, Vec<Asset>)> for TeleportFilter {
	fn contains(item: &(Location, Vec<Asset>)) -> bool {
		// We only allow teleportation of PLMC, but anyone can do it
		let (_loc, assets) = item;
		assets.iter().all(|asset| asset.id.0 == Location::here())
	}
}

/// Means for transacting assets on this chain.
/// FungibleTransactor is a FungibleAdapter that allows for transacting PLMC.
/// ForeignAssetsAdapter is a FungiblesAdapter that allows for transacting foreign assets.
/// Currently we only support DOT, USDT and USDC.
pub type AssetTransactors = (FungibleTransactor, ForeignAssetsAdapter);

pub type TakeRevenueToTreasury =
	cumulus_primitives_utility::XcmFeesTo32ByteAccount<AssetTransactors, AccountId, TreasuryAccount>;

// TODO: once we open more channels and get more XCM's we should consider adding a fee.
pub type PriceForParentDelivery = NoPriceForMessageDelivery<()>;
pub type PriceForSiblingParachainDelivery = NoPriceForMessageDelivery<ParaId>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
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
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type HrmpNewChannelOpenRequestHandler = ();
	/// Locations that we trust to act as reserves for specific assets.
	type IsReserve = Reserves;
	type IsTeleporter = HereToHub;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type ResponseHandler = PolkadotXcm;
	type RuntimeCall = RuntimeCall;
	// We allow any Transact instructions to be executed on our chain.
	type SafeCallFilter = Everything;
	type SubscriptionService = PolkadotXcm;
	type Trader = (AssetTrader<TakeRevenueToTreasury>,);
	type TransactionalProcessor = FrameTransactionalProcessor;
	type UniversalAliases = Nothing;
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmEventEmitter = PolkadotXcm;
	type XcmRecorder = PolkadotXcm;
	type XcmSender = XcmRouter;
}

/// Converts a local signed origin into an XCM `Location`.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalSignedOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	super::XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type AdminOrigin = EnsureRoot<AccountId>;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type AuthorizedAliasConsideration = Disabled;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	// Any local signed origin can send XCM messages.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalSignedOriginToLocation>;
	type SovereignAccountOf = LocationToAccountId;
	type TrustedLockers = ();
	type UniversalLocation = UniversalLocation;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	// We only allow reserve based transfers of Asset Hub reserve assets back to Asset Hub.
	type XcmReserveTransferFilter = AssetHubAssetsAsReserve;
	type XcmRouter = XcmRouter;
	// We allow teleportation of PLMC to Polkadot Asset Hub.
	type XcmTeleportFilter = TeleportFilter;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::migration::v5::V5Config for Runtime {
	// This must be the same as the `ChannelInfo` from the `Config`:
	type ChannelList = ParachainSystem;
}

/// Can be used to buy weight in exchange for an accepted asset.
/// Only one asset can be used to buy weight at a time.
pub struct AssetTrader<Payee: TakeRevenue> {
	weight_bought: Weight,
	asset_spent: Option<Asset>,
	phantom: PhantomData<Payee>,
}
impl<Payee: TakeRevenue> WeightTrader for AssetTrader<Payee> {
	fn new() -> Self {
		Self { weight_bought: Weight::zero(), asset_spent: None, phantom: PhantomData }
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: AssetsInHolding,
		context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		log::trace!(target: "xcm::weight", "AssetsTrader::buy_weight weight: {:?}, payment: {:?}, context: {:?}", weight, payment, context);
		let native_amount = WeightToFee::weight_to_fee(&weight);
		let acceptable_assets = AcceptedFundingAsset::all_ids_and_plmc();

		// We know the executor always sends just one asset to pay for weight, even if the struct supports multiple.
		let payment_fun = payment.fungible.clone();
		let (asset_id, asset_amount) = payment_fun.first_key_value().ok_or(XcmError::FeesNotMet)?;

		let asset_id_v4: polimec_common::Location =
			asset_id.0.clone().try_into().map_err(|_| XcmError::UnhandledXcmVersion)?;

		ensure!(acceptable_assets.contains(&asset_id_v4), XcmError::FeesNotMet);

		// If the trader was used already in this xcm execution, make sure we continue trading with the same asset
		let old_amount = if let Some(asset) = &self.asset_spent {
			ensure!(asset.id == *asset_id, XcmError::FeesNotMet);
			if let Fungibility::Fungible(amount) = asset.fun {
				amount
			} else {
				return Err(XcmError::FeesNotMet)
			}
		} else {
			Zero::zero()
		};

		let required_asset_amount =
			HereToForeignAsset::to_asset_balance(native_amount, asset_id_v4).map_err(|_| XcmError::FeesNotMet)?;
		ensure!(*asset_amount >= required_asset_amount, XcmError::FeesNotMet);

		let required = (AssetId(asset_id.0.clone()), required_asset_amount).into();
		let unused = payment.checked_sub(required).map_err(|_| XcmError::FeesNotMet)?;

		self.weight_bought = self.weight_bought.saturating_add(weight);
		self.asset_spent =
			Some(Asset { id: asset_id.clone(), fun: Fungibility::Fungible(old_amount + required_asset_amount) });

		Ok(unused)
	}

	fn refund_weight(&mut self, weight: Weight, context: &XcmContext) -> Option<Asset> {
		log::trace!(target: "xcm::weight", "AssetsTrader::refund_weight weight: {:?}, context: {:?}, available weight: {:?}, available amount: {:?}", weight, context, self.weight_bought, self.asset_spent);
		let weight_refunded = weight.min(self.weight_bought);
		self.weight_bought -= weight_refunded;

		let native_amount = WeightToFee::weight_to_fee(&weight_refunded);
		let asset_id = self.asset_spent.clone()?.id;
		let asset_id_v4 = asset_id.0.clone().try_into().ok()?;
		let asset_amount = HereToForeignAsset::to_asset_balance(native_amount, asset_id_v4).ok()?;
		log::trace!(target: "xcm::weight", "AssetTrader::refund_weight amount to refund: {:?}", asset_amount);

		if let Fungibility::Fungible(amount) = self.asset_spent.clone()?.fun {
			self.asset_spent =
				Some(Asset { id: asset_id.clone(), fun: Fungibility::Fungible(amount.saturating_sub(asset_amount)) });
		} else {
			log::trace!(target: "xcm::weight", "AssetTrader::refund_weight unexpected non-fungible asset found. Bug somewhere");
			return None;
		}

		if asset_amount > 0 {
			Some((asset_id, asset_amount).into())
		} else {
			None
		}
	}
}
impl<Payee: TakeRevenue> Drop for AssetTrader<Payee> {
	fn drop(&mut self) {
		if let Some(asset) = &self.asset_spent {
			log::trace!(target: "xcm::weight", "AssetTrader::drop asset: {:?}", asset);
			Payee::take_revenue(asset.clone());
		}
	}
}
