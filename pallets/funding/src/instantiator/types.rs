#[allow(clippy::wildcard_imports)]
use super::*;
use crate::ParticipationMode;
use alloc::{boxed::Box, vec::Vec};
use frame_support::{Deserialize, Serialize};
use polimec_common::assets::AcceptedFundingAsset;

pub type RuntimeOriginOf<T> = <T as frame_system::Config>::RuntimeOrigin;
pub struct BoxToFunction(pub Box<dyn FnOnce()>);
impl Default for BoxToFunction {
	fn default() -> Self {
		BoxToFunction(Box::new(|| ()))
	}
}

#[cfg(feature = "std")]
pub type OptionalExternalities = Option<RefCell<sp_io::TestExternalities>>;

#[cfg(not(feature = "std"))]
pub type OptionalExternalities = Option<()>;

pub struct Instantiator<
	T: Config + cumulus_pallet_parachain_system::Config,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
> {
	pub ext: OptionalExternalities,
	pub nonce: RefCell<u64>,
	pub _marker: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
}

impl<T: Config> Deposits<T> for Vec<AccountIdOf<T>> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>> {
		self.iter()
			.map(|x| UserToPLMCBalance::new(x.clone(), NativeCurrencyOf::<T>::minimum_balance()))
			.collect::<Vec<_>>()
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct UserToPLMCBalance<T: Config> {
	pub account: AccountIdOf<T>,
	pub plmc_amount: BalanceOf<T>,
}
impl<T: Config> UserToPLMCBalance<T> {
	pub fn new(account: AccountIdOf<T>, plmc_amount: BalanceOf<T>) -> Self {
		Self { account, plmc_amount }
	}
}
impl<T: Config> Accounts for Vec<UserToPLMCBalance<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToPLMCBalance { account, plmc_amount: _ } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToPLMCBalance<T> {
	fn from((account, plmc_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToPLMCBalance::<T>::new(account, plmc_amount)
	}
}
impl<T: Config> AccountMerge for Vec<UserToPLMCBalance<T>> {
	type Inner = UserToPLMCBalance<T>;

	fn get_account(inner: Self::Inner) -> Self::Account {
		inner.account
	}

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToPLMCBalance { account, plmc_amount } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut BalanceOf<T>| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*plmc_amount),
						MergeOperation::Subtract => e.saturating_sub(*plmc_amount),
					}
				})
				.or_insert(*plmc_amount);
		}
		btree.into_iter().map(|(account, plmc_amount)| UserToPLMCBalance::new(account, plmc_amount)).collect()
	}
}

impl<T: Config> Total for Vec<UserToPLMCBalance<T>> {
	fn total(&self) -> BalanceOf<T> {
		self.iter().map(|x| x.plmc_amount).sum()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct EvaluationParams<T: Config> {
	pub account: AccountIdOf<T>,
	pub usd_amount: BalanceOf<T>,
	pub receiving_account: Junction,
}
impl<T: Config> EvaluationParams<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: BalanceOf<T>, receiving_account: Junction) -> Self {
		EvaluationParams::<T> { account, usd_amount, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, Junction)> for EvaluationParams<T> {
	fn from((account, usd_amount, receiving_account): (AccountIdOf<T>, BalanceOf<T>, Junction)) -> Self {
		EvaluationParams::<T>::new(account, usd_amount, receiving_account)
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for EvaluationParams<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		let receiving_account = Junction::AccountId32 {
			network: Some(NetworkId::Polkadot),
			id: T::AccountId32Conversion::convert(account.clone()),
		};
		EvaluationParams::<T>::new(account, usd_amount, receiving_account)
	}
}
impl<T: Config> Accounts for Vec<EvaluationParams<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for EvaluationParams { account, usd_amount: _, receiving_account: _ } in self {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, PartialEq)]
pub struct UserToUSDAmount<T: Config> {
	pub account: AccountIdOf<T>,
	pub usd_amount: BalanceOf<T>,
}
impl<T: Config> UserToUSDAmount<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: BalanceOf<T>) -> Self {
		Self { account, usd_amount }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToUSDAmount<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToUSDAmount::<T>::new(account, usd_amount)
	}
}
impl<T: Config> Accounts for Vec<UserToUSDAmount<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToUSDAmount { account, usd_amount: _ } in self {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> AccountMerge for Vec<UserToUSDAmount<T>> {
	type Inner = UserToUSDAmount<T>;

	fn get_account(inner: Self::Inner) -> Self::Account {
		inner.account
	}

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToUSDAmount { account, usd_amount } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|stored_usd_amount: &mut BalanceOf<T>| match ops {
					MergeOperation::Add => {
						*stored_usd_amount = stored_usd_amount.saturating_add(*usd_amount);
					},
					MergeOperation::Subtract => {
						*stored_usd_amount = stored_usd_amount.saturating_sub(*usd_amount);
					},
				})
				.or_insert(*usd_amount);
		}
		btree.into_iter().map(|(account, usd_amount)| UserToUSDAmount::new(account, usd_amount)).collect()
	}
}
impl<T: Config> Total for Vec<UserToUSDAmount<T>> {
	fn total(&self) -> BalanceOf<T> {
		self.iter().map(|x| x.usd_amount).sum()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserToFundingAsset<T: Config> {
	pub account: AccountIdOf<T>,
	pub asset_amount: BalanceOf<T>,
	pub asset_id: AssetIdOf<T>,
}
impl<T: Config> UserToFundingAsset<T> {
	pub fn new(account: AccountIdOf<T>, asset_amount: BalanceOf<T>, asset_id: AssetIdOf<T>) -> Self {
		Self { account, asset_amount, asset_id }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)> for UserToFundingAsset<T> {
	fn from((account, asset_amount, asset_id): (AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)) -> Self {
		UserToFundingAsset::<T>::new(account, asset_amount, asset_id)
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToFundingAsset<T> {
	fn from((account, asset_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToFundingAsset::<T>::new(account, asset_amount, AcceptedFundingAsset::USDT.id())
	}
}
impl<T: Config> Accounts for Vec<UserToFundingAsset<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToFundingAsset { account, .. } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> AccountMerge for Vec<UserToFundingAsset<T>> {
	type Inner = UserToFundingAsset<T>;

	fn get_account(inner: Self::Inner) -> Self::Account {
		inner.account
	}

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToFundingAsset { account, asset_amount, asset_id } in self.iter() {
			btree
				.entry((account.clone(), asset_id))
				.and_modify(|e: &mut BalanceOf<T>| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*asset_amount),
						MergeOperation::Subtract => e.saturating_sub(*asset_amount),
					}
				})
				.or_insert(*asset_amount);
		}
		btree
			.into_iter()
			.map(|((account, asset_id), asset_amount)| UserToFundingAsset::new(account, asset_amount, asset_id.clone()))
			.collect()
	}

	fn subtract_accounts(&self, other_list: Self) -> Self {
		let current_accounts = self.accounts();
		let filtered_list = other_list.into_iter().filter(|x| current_accounts.contains(&x.account)).collect_vec();
		let mut new_list = self.clone();
		new_list.extend(filtered_list);
		new_list.merge_accounts(MergeOperation::Subtract)
	}

	fn sum_accounts(&self, mut other_list: Self) -> Self {
		let mut output = self.clone();
		output.append(&mut other_list);
		output.merge_accounts(MergeOperation::Add)
	}
}
impl<T: Config> Totals for Vec<UserToFundingAsset<T>> {
	type AssetId = AssetIdOf<T>;

	fn totals(&self) -> Vec<(Self::AssetId, BalanceOf<T>)> {
		let mut btree = BTreeMap::new();
		for UserToFundingAsset { account: _, asset_amount, asset_id } in self.iter() {
			btree
				.entry(asset_id.clone())
				.and_modify(|e: &mut BalanceOf<T>| *e = e.saturating_add(*asset_amount))
				.or_insert(*asset_amount);
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> Conversions for Vec<UserToFundingAsset<T>> {
	type AccountId = AccountIdOf<T>;
	type AssetId = AssetIdOf<T>;

	fn to_account_asset_map(&self) -> Vec<(Self::AccountId, Self::AssetId)> {
		let mut btree = BTreeSet::new();
		for UserToFundingAsset { account, asset_id, .. } in self.iter() {
			btree.insert((account.clone(), asset_id.clone()));
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct BidParams<T: Config> {
	pub bidder: AccountIdOf<T>,
	pub investor_type: InvestorType,
	pub amount: BalanceOf<T>,
	pub mode: ParticipationMode,
	pub asset: AcceptedFundingAsset,
	pub receiving_account: Junction,
}
impl<T: Config> BidParams<T> {
	pub fn new(
		bidder: AccountIdOf<T>,
		investor_type: InvestorType,
		amount: BalanceOf<T>,
		mode: ParticipationMode,
		asset: AcceptedFundingAsset,
		receiving_account: Junction,
	) -> Self {
		Self { bidder, investor_type, amount, mode, asset, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for BidParams<T> {
	fn from((bidder, amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		Self {
			bidder: bidder.clone(),
			investor_type: InvestorType::Retail,
			amount,
			mode: ParticipationMode::Classic(1u8),
			asset: AcceptedFundingAsset::USDT,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(bidder.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>)> for BidParams<T> {
	fn from((bidder, investor_type, amount): (AccountIdOf<T>, InvestorType, BalanceOf<T>)) -> Self {
		Self {
			bidder: bidder.clone(),
			investor_type,
			amount,
			mode: ParticipationMode::Classic(1u8),
			asset: AcceptedFundingAsset::USDT,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(bidder.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>, ParticipationMode)> for BidParams<T> {
	fn from(
		(bidder, investor_type, amount, mode): (AccountIdOf<T>, InvestorType, BalanceOf<T>, ParticipationMode),
	) -> Self {
		Self {
			bidder: bidder.clone(),
			investor_type,
			amount,
			mode,
			asset: AcceptedFundingAsset::USDT,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(bidder.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>, AcceptedFundingAsset)> for BidParams<T> {
	fn from(
		(bidder, investor_type, amount, asset): (AccountIdOf<T>, InvestorType, BalanceOf<T>, AcceptedFundingAsset),
	) -> Self {
		Self {
			bidder: bidder.clone(),
			investor_type,
			amount,
			mode: ParticipationMode::Classic(1u8),
			asset,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(bidder.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>, ParticipationMode, AcceptedFundingAsset)>
	for BidParams<T>
{
	fn from(
		(bidder, investor_type, amount, mode, asset): (
			AccountIdOf<T>,
			InvestorType,
			BalanceOf<T>,
			ParticipationMode,
			AcceptedFundingAsset,
		),
	) -> Self {
		Self {
			bidder: bidder.clone(),
			investor_type,
			amount,
			mode,
			asset,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(bidder.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>, AcceptedFundingAsset, Junction)> for BidParams<T> {
	fn from(
		(bidder, investor_type, amount, asset, receiving_account): (
			AccountIdOf<T>,
			InvestorType,
			BalanceOf<T>,
			AcceptedFundingAsset,
			Junction,
		),
	) -> Self {
		Self { bidder, investor_type, amount, mode: ParticipationMode::Classic(1u8), asset, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, InvestorType, BalanceOf<T>, ParticipationMode, AcceptedFundingAsset, Junction)>
	for BidParams<T>
{
	fn from(
		(bidder, investor_type, amount, mode, asset, receiving_account): (
			AccountIdOf<T>,
			InvestorType,
			BalanceOf<T>,
			ParticipationMode,
			AcceptedFundingAsset,
			Junction,
		),
	) -> Self {
		Self { bidder, investor_type, amount, mode, asset, receiving_account }
	}
}
impl<T: Config> From<BidParams<T>> for (AccountIdOf<T>, AssetIdOf<T>) {
	fn from(bid: BidParams<T>) -> (AccountIdOf<T>, AssetIdOf<T>) {
		(bid.bidder, bid.asset.id())
	}
}

impl<T: Config> Accounts for Vec<BidParams<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for BidParams { bidder, .. } in self {
			btree.insert(bidder.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> Conversions for Vec<BidParams<T>> {
	type AccountId = AccountIdOf<T>;
	type AssetId = AssetIdOf<T>;

	fn to_account_asset_map(&self) -> Vec<(Self::AccountId, Self::AssetId)> {
		let mut btree = BTreeSet::new();
		for BidParams { bidder, asset, .. } in self.iter() {
			btree.insert((bidder.clone(), asset.id()));
		}
		btree.into_iter().collect_vec()
	}
}
