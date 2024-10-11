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

// If you feel like getting in touch with us, you can do so at info@polimec.org

//! Test environment for Funding pallet.

use super::*;
use crate as pallet_funding;
use crate::runtime_api::{
	ExtrinsicHelpers, Leaderboards, ProjectInformation, ProjectParticipationIds, UserInformation,
};
use core::ops::RangeInclusive;
use frame_support::{
	construct_runtime, derive_impl,
	pallet_prelude::Weight,
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU32, ConstU64, Everything, OriginTrait, WithdrawReasons},
	PalletId,
};
use frame_system as system;
use frame_system::{EnsureRoot, RawOrigin as SystemRawOrigin};
use polimec_common::{credentials::EnsureInvestor, DummyXcmSender, ProvideAssetPrice, USD_UNIT};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_arithmetic::{Perbill, Percent};
use sp_core::{ConstU8, H256};
use sp_runtime::{
	traits::{BlakeTwo256, ConvertBack, ConvertInto, Get, IdentityLookup, TryConvert},
	BuildStorage, Perquintill,
};
use sp_std::collections::btree_map::BTreeMap;
use std::cell::RefCell;
use system::EnsureSigned;
use xcm::v4::PalletInfo as XcmPalletInfo;
use xcm_builder::{EnsureXcmOrigin, FixedWeightBounds, ParentIsPreset, SiblingParachainConvertsVia};
use xcm_executor::traits::XcmAssetTransfers;

pub const PLMC: Balance = 10u128.pow(PLMC_DECIMALS as u32);
pub const MILLI_PLMC: Balance = PLMC / 10u128.pow(3);
pub const MICRO_PLMC: Balance = PLMC / 10u128.pow(6);
pub const EXISTENTIAL_DEPOSIT: Balance = 10 * MILLI_PLMC;
pub type Block = frame_system::mocking::MockBlock<TestRuntime>;
pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Identifier = u32;
pub type Price = FixedU128;
pub type ContributionTokensInstance = pallet_assets::Instance1;
pub type ForeignAssetsInstance = pallet_assets::Instance2;
pub type AssetId = u32;

pub const fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * 15 * MICRO_PLMC + (bytes as Balance) * 6 * MICRO_PLMC
}

pub const fn free_deposit() -> Balance {
	0 * MICRO_PLMC
}

pub struct SignedToAccountIndex<RuntimeOrigin, AccountId, Network>(PhantomData<(RuntimeOrigin, AccountId, Network)>);

impl<RuntimeOrigin: OriginTrait + Clone, AccountId: Into<u64>, Network: Get<Option<NetworkId>>>
	TryConvert<RuntimeOrigin, Location> for SignedToAccountIndex<RuntimeOrigin, AccountId, Network>
where
	RuntimeOrigin::PalletsOrigin:
		From<SystemRawOrigin<AccountId>> + TryInto<SystemRawOrigin<AccountId>, Error = RuntimeOrigin::PalletsOrigin>,
{
	fn try_convert(o: RuntimeOrigin) -> Result<Location, RuntimeOrigin> {
		o.try_with_caller(|caller| match caller.try_into() {
			Ok(SystemRawOrigin::Signed(who)) =>
				Ok(Junction::AccountIndex64 { network: Network::get(), index: Into::<u64>::into(who) }.into()),
			Ok(other) => Err(other.into()),
			Err(other) => Err(other),
		})
	}
}
pub type LocalOriginToLocation = SignedToAccountIndex<RuntimeOrigin, AccountId, RelayNetwork>;
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
);

parameter_types! {
	pub UniversalLocation: InteriorLocation = (
		GlobalConsensus(Polkadot),
		 Parachain(3344u32),
	).into();
	pub const RelayNetwork: Option<NetworkId> = None;
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;

	pub const HereLocation: Location = Location::here();
}

pub struct MockPrepared;
impl PreparedMessage for MockPrepared {
	fn weight_of(&self) -> Weight {
		Weight::zero()
	}
}

pub struct MockXcmExecutor;
impl XcmAssetTransfers for MockXcmExecutor {
	type AssetTransactor = ();
	type IsReserve = ();
	type IsTeleporter = ();
}

impl ExecuteXcm<RuntimeCall> for MockXcmExecutor {
	type Prepared = MockPrepared;

	fn prepare(_message: Xcm<RuntimeCall>) -> core::result::Result<Self::Prepared, Xcm<RuntimeCall>> {
		Ok(MockPrepared)
	}

	fn execute(
		_origin: impl Into<Location>,
		_pre: Self::Prepared,
		_id: &mut XcmHash,
		_weight_credit: Weight,
	) -> Outcome {
		Outcome::Complete { used: Weight::zero() }
	}

	fn charge_fees(_location: impl Into<Location>, _fees: Assets) -> XcmResult {
		Ok(())
	}
}

impl pallet_xcm::Config for TestRuntime {
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
	type XcmExecutor = MockXcmExecutor;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = DummyXcmSender;
	type XcmTeleportFilter = Everything;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
}

parameter_types! {
	pub const AssetDeposit: Balance = PLMC; // 1 UNIT deposit to create asset
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const ZeroAssetAccountDeposit: Balance = free_deposit();
	pub const AssetsStringLimit: u32 = 50;
	// https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
	pub const MetadataDepositBase: Balance = free_deposit();
	pub const MetadataDepositPerByte: Balance = free_deposit();
	pub const ApprovalDeposit: Balance = EXISTENTIAL_DEPOSIT;

}
impl pallet_assets::Config<ContributionTokensInstance> for TestRuntime {
	type ApprovalDeposit = ApprovalDeposit;
	type AssetAccountDeposit = ZeroAssetAccountDeposit;
	type AssetDeposit = AssetDeposit;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type CallbackHandle = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Currency = Balances;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type RemoveItemsLimit = ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
}

impl pallet_assets::Config<ForeignAssetsInstance> for TestRuntime {
	type ApprovalDeposit = ApprovalDeposit;
	type AssetAccountDeposit = AssetAccountDeposit;
	type AssetDeposit = AssetDeposit;
	type AssetId = AssetId;
	type AssetIdParameter = parity_scale_codec::Compact<AssetId>;
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type CallbackHandle = ();
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type Currency = Balances;
	type Extra = ();
	type ForceOrigin = EnsureRoot<AccountId>;
	type Freezer = ();
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type RemoveItemsLimit = ConstU32<1000>;
	type RuntimeEvent = RuntimeEvent;
	type StringLimit = AssetsStringLimit;
	type WeightInfo = ();
}
parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for TestRuntime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type Block = Block;
	type BlockLength = ();
	type BlockWeights = ();
	type DbWeight = ();
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Lookup = IdentityLookup<Self::AccountId>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type Nonce = u64;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type PalletInfo = PalletInfo;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type SS58Prefix = ConstU16<42>;
	type SystemWeightInfo = ();
	type Version = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}
impl pallet_balances::Config for TestRuntime {
	type AccountStore = System;
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<1024>;
	type MaxLocks = frame_support::traits::ConstU32<1024>;
	type MaxReserves = frame_support::traits::ConstU32<1024>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
}

impl pallet_timestamp::Config for TestRuntime {
	type MinimumPeriod = ConstU64<5>;
	type Moment = u64;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

pub const HOURS: BlockNumber = 300u64;

// REMARK: In the production configuration we use DAYS instead of HOURS.
// We need all durations to use different times to catch bugs in the tests.
parameter_types! {
	pub const EvaluationRoundDuration: BlockNumber = 10u64;
	pub const AuctionRoundDuration: BlockNumber = 15u64;
	pub const CommunityRoundDuration: BlockNumber = 18u64;
	pub const RemainderRoundDuration: BlockNumber = 6u64;

	pub const FundingPalletId: PalletId = PalletId(*b"plmc-fun");
	pub FeeBrackets: Vec<(Percent, Balance)> = vec![
		(Percent::from_percent(10), 1_000_000 * USD_UNIT),
		(Percent::from_percent(8), 4_000_000 * USD_UNIT),
		(Percent::from_percent(6), u128::MAX), // Making it max signifies the last bracket
	];
	pub EarlyEvaluationThreshold: Percent = Percent::from_percent(10);
	pub EvaluatorSlash: Percent = Percent::from_percent(20);
	pub BlockchainOperationTreasuryAccount: AccountId = AccountId::from(696969u32);
	pub ProxyBondingTreasuryAccount: AccountId = AccountId::from(555u32);
	pub ContributionTreasury: AccountId = AccountId::from(4204204206u32);
	pub FundingSuccessThreshold: Perquintill = Perquintill::from_percent(33);
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub PolimecReceiverInfo: XcmPalletInfo = XcmPalletInfo::new(
		51, "PolimecReceiver".into(), "polimec_receiver".into(), 0, 1, 0
	).unwrap();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub BenchmarkReason: RuntimeHoldReason = RuntimeHoldReason::PolimecFunding(crate::HoldReason::Participation);
}
impl pallet_linear_release::Config for TestRuntime {
	type Balance = Balance;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkReason = BenchmarkReason;
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type MinVestedTransfer = MinVestedTransfer;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	type WeightInfo = ();

	const MAX_VESTING_SCHEDULES: u32 = 100;
}

parameter_types! {
	pub MaxMessageSizeThresholds: RangeInclusive<u32> = 50000..=102_400;
	pub MaxCapacityThresholds: RangeInclusive<u32> = 8..=1000;
	pub RequiredMaxCapacity: u32 = 8;
	pub RequiredMaxMessageSize: u32 = 102_400;
	pub VerifierPublicKey: [u8; 32] = [
		32, 118, 30, 171, 58, 212, 197, 27, 146, 122, 255, 243, 34, 245, 90, 244, 221, 37, 253,
		195, 18, 202, 111, 55, 39, 48, 123, 17, 101, 78, 215, 94,
	];
	pub MinUsdPerEvaluation: Balance = 100 * USD_UNIT;
}

pub struct DummyConverter;
impl sp_runtime::traits::Convert<AccountId, [u8; 32]> for DummyConverter {
	fn convert(a: AccountId) -> [u8; 32] {
		let mut account: [u8; 32] = [0u8; 32];
		account[0..8].copy_from_slice(a.to_le_bytes().as_slice());
		account
	}
}
impl ConvertBack<AccountId, [u8; 32]> for DummyConverter {
	fn convert_back(bytes: [u8; 32]) -> AccountId {
		let account: [u8; 8] = bytes[0..7].try_into().unwrap();
		u64::from_le_bytes(account)
	}
}
thread_local! {
	pub static PRICE_MAP: RefCell<BTreeMap<AssetId, FixedU128>> = RefCell::new(BTreeMap::from_iter(vec![
		(AcceptedFundingAsset::DOT.id(), FixedU128::from_float(69f64)), // DOT
		(AcceptedFundingAsset::USDC.id(), FixedU128::from_float(0.97f64)), // USDC
		(AcceptedFundingAsset::USDT.id(), FixedU128::from_float(1.0f64)), // USDT
		(PLMC_FOREIGN_ID, FixedU128::from_float(8.4f64)), // PLMC
	]));
}
pub struct ConstPriceProvider;
impl ProvideAssetPrice for ConstPriceProvider {
	type AssetId = AssetId;
	type Price = Price;

	fn get_price(asset_id: AssetId) -> Option<Price> {
		PRICE_MAP.with(|price_map| price_map.borrow().get(&asset_id).cloned())
	}
}

impl ConstPriceProvider {
	pub fn set_price(asset_id: AssetId, price: Price) {
		PRICE_MAP.with(|price_map| {
			price_map.borrow_mut().insert(asset_id, price);
		});
	}
}
impl Config for TestRuntime {
	type AccountId32Conversion = DummyConverter;
	type AllPalletsWithoutSystem = (Balances, ContributionTokens, ForeignAssets, PolimecFunding, LinearRelease);
	type AuctionRoundDuration = AuctionRoundDuration;
	type BlockNumber = BlockNumber;
	type BlockchainOperationTreasury = BlockchainOperationTreasuryAccount;
	type CommunityRoundDuration = CommunityRoundDuration;
	type ContributionTokenCurrency = ContributionTokens;
	type ContributionTreasury = ContributionTreasury;
	type DaysToBlocks = DaysToBlocks;
	type EvaluationRoundDuration = EvaluationRoundDuration;
	type EvaluationSuccessThreshold = EarlyEvaluationThreshold;
	type EvaluatorSlash = EvaluatorSlash;
	type FeeBrackets = FeeBrackets;
	type FundingCurrency = ForeignAssets;
	type FundingSuccessThreshold = FundingSuccessThreshold;
	type InvestorOrigin = EnsureInvestor<TestRuntime>;
	type MaxBidsPerProject = ConstU32<512>;
	type MaxBidsPerUser = ConstU32<25>;
	type MaxCapacityThresholds = MaxCapacityThresholds;
	type MaxContributionsPerUser = ConstU32<25>;
	type MaxEvaluationsPerProject = ConstU32<512>;
	type MaxEvaluationsPerUser = ConstU32<4>;
	type MaxMessageSizeThresholds = MaxMessageSizeThresholds;
	type MinUsdPerEvaluation = MinUsdPerEvaluation;
	type Multiplier = Multiplier;
	type NativeCurrency = Balances;
	type OnSlash = ();
	type PalletId = FundingPalletId;
	type Price = FixedU128;
	type PriceProvider = ConstPriceProvider;
	type RemainderRoundDuration = RemainderRoundDuration;
	type RequiredMaxCapacity = RequiredMaxCapacity;
	type RequiredMaxMessageSize = RequiredMaxMessageSize;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeOrigin = RuntimeOrigin;
	#[cfg(feature = "runtime-benchmarks")]
	type SetPrices = ();
	type StringLimit = ConstU32<64>;
	type VerifierPublicKey = VerifierPublicKey;
	type WeightInfo = weights::SubstrateWeight<TestRuntime>;
}

parameter_types! {
	// Means a USD Ticket fee of 1.5%, since the FeePercentage is applied on the PLMC bond with multiplier 5.
	pub FeePercentage: Perbill = Perbill::from_rational(75u32, 100u32);
	pub const FeeRecipient: AccountId = 80085;
	pub const RootId: PalletId = PalletId(*b"treasury");
}
impl pallet_proxy_bonding::Config for TestRuntime {
	type BondingToken = Balances;
	type BondingTokenDecimals = ConstU8<PLMC_DECIMALS>;
	type BondingTokenId = ConstU32<PLMC_FOREIGN_ID>;
	type FeePercentage = FeePercentage;
	type FeeRecipient = FeeRecipient;
	type FeeToken = ForeignAssets;
	type Id = PalletId;
	type PriceProvider = ConstPriceProvider;
	type RootId = RootId;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Treasury = ProxyBondingTreasuryAccount;
	type UsdDecimals = ConstU8<USD_DECIMALS>;
}

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum TestRuntime
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		Balances: pallet_balances,
		LinearRelease: pallet_linear_release,
		ContributionTokens: pallet_assets::<Instance1>::{Pallet, Call, Storage, Event<T>},
		ForeignAssets: pallet_assets::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>},
		PolkadotXcm: pallet_xcm,
		PolimecFunding: pallet_funding::{Pallet, Call, Storage, Event<T>, HoldReason}  = 52,
		ProxyBonding: pallet_proxy_bonding,
	}
);

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();
	let ed = <TestRuntime as pallet_balances::Config>::ExistentialDeposit::get();
	RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: vec![
				(<TestRuntime as Config>::PalletId::get().into_account_truncating(), ed),
				(<TestRuntime as Config>::ContributionTreasury::get(), ed),
				(<TestRuntime as Config>::BlockchainOperationTreasury::get(), ed),
				// Treasury account needs PLMC for the One Token Model participations
				(ProxyBondingTreasuryAccount::get(), 1_000_000 * PLMC),
				(FeeRecipient::get(), ed),
			],
		},
		foreign_assets: ForeignAssetsConfig {
			assets: vec![
				(
					AcceptedFundingAsset::USDT.id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					// asset is sufficient, i.e. participants can hold only this asset to participate with OTM
					true,
					70_000,
				),
				(
					AcceptedFundingAsset::USDC.id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					true,
					70_000,
				),
				(
					AcceptedFundingAsset::DOT.id(),
					<TestRuntime as Config>::PalletId::get().into_account_truncating(),
					true,
					100_000_000,
				),
			],
			metadata: vec![
				(AcceptedFundingAsset::USDT.id(), "USDT".as_bytes().to_vec(), "USDT".as_bytes().to_vec(), 6),
				(AcceptedFundingAsset::USDC.id(), "USDC".as_bytes().to_vec(), "USDC".as_bytes().to_vec(), 6),
				(AcceptedFundingAsset::DOT.id(), "DOT".as_bytes().to_vec(), "DOT".as_bytes().to_vec(), 10),
			],
			accounts: vec![],
		},
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	// In order to emit events the block number must be more than 0
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

sp_api::mock_impl_runtime_apis! {
	impl Leaderboards<Block, TestRuntime> for TestRuntime {
		fn top_evaluations(project_id: ProjectId, amount: u32) -> Vec<EvaluationInfoOf<TestRuntime>> {
			PolimecFunding::top_evaluations(project_id, amount)
		}

		fn top_bids(project_id: ProjectId, amount: u32) -> Vec<BidInfoOf<TestRuntime>> {
			PolimecFunding::top_bids(project_id, amount)
		}

		fn top_contributions(project_id: ProjectId, amount: u32) -> Vec<ContributionInfoOf<TestRuntime>> {
			PolimecFunding::top_contributions(project_id, amount)
		}

		fn top_projects_by_usd_raised(amount: u32) -> Vec<(ProjectId, ProjectMetadataOf<TestRuntime>, ProjectDetailsOf<TestRuntime>)> {
			PolimecFunding::top_projects_by_usd_raised(amount)
		}

		fn top_projects_by_usd_target_percent_reached(amount: u32) -> Vec<(ProjectId, ProjectMetadataOf<TestRuntime>, ProjectDetailsOf<TestRuntime>)> {
			PolimecFunding::top_projects_by_usd_target_percent_reached(amount)
		}
	}

	impl UserInformation<Block, TestRuntime> for TestRuntime {
		fn contribution_tokens(account: AccountId) -> Vec<(ProjectId, Balance)> {
			PolimecFunding::contribution_tokens(account)
		}

		fn all_project_participations_by_did(project_id: ProjectId, did: Did) -> Vec<ProjectParticipationIds<TestRuntime>> {
			PolimecFunding::all_project_participations_by_did(project_id, did)
		}
	}

	impl ProjectInformation<Block, TestRuntime> for TestRuntime {
		fn usd_target_percent_reached(project_id: ProjectId) -> FixedU128 {
			PolimecFunding::usd_target_percent_reached(project_id)
		}

		fn projects_by_did(did: Did) -> Vec<ProjectId> {
			PolimecFunding::projects_by_did(did)
		}
	}

	impl ExtrinsicHelpers<Block, TestRuntime> for TestRuntime {
		fn funding_asset_to_ct_amount(project_id: ProjectId, asset: AcceptedFundingAsset, asset_amount: Balance) -> Balance {
			PolimecFunding::funding_asset_to_ct_amount(project_id, asset, asset_amount)
		}
		fn get_next_vesting_schedule_merge_candidates(account: AccountId, hold_reason: RuntimeHoldReason, end_max_delta: Balance) -> Option<(u32, u32)> {
			PolimecFunding::get_next_vesting_schedule_merge_candidates(account, hold_reason, end_max_delta)
		}

		fn calculate_otm_fee(funding_asset: AcceptedFundingAsset, funding_asset_amount: Balance) -> Option<Balance> {
			PolimecFunding::calculate_otm_fee(funding_asset, funding_asset_amount)
		}
		fn get_funding_asset_min_max_amounts(project_id: ProjectId, did: Did, funding_asset: AcceptedFundingAsset, investor_type: InvestorType) -> Option<(Balance, Balance)> {
			PolimecFunding::get_funding_asset_min_max_amounts(project_id, did, funding_asset, investor_type)
		}


	}
}
