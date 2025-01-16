#[allow(clippy::wildcard_imports)]
use super::*;
use crate::ParticipationMode;
use frame_support::{Deserialize, Serialize};

pub type RuntimeOriginOf<T> = <T as frame_system::Config>::RuntimeOrigin;
pub struct BoxToFunction(pub Box<dyn FnOnce()>);
impl Default for BoxToFunction {
	fn default() -> Self {
		BoxToFunction(Box::new(|| ()))
	}
}

#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct TestProjectParams<T: Config> {
	pub expected_state: ProjectStatus<BlockNumberFor<T>>,
	pub metadata: ProjectMetadataOf<T>,
	pub issuer: AccountIdOf<T>,
	pub evaluations: Vec<EvaluationParams<T>>,
	pub bids: Vec<BidParams<T>>,
	pub community_contributions: Vec<ContributionParams<T>>,
	pub remainder_contributions: Vec<ContributionParams<T>>,
}

#[cfg(feature = "std")]
pub type OptionalExternalities = Option<RefCell<sp_io::TestExternalities>>;

#[cfg(not(feature = "std"))]
pub type OptionalExternalities = Option<()>;

pub struct Instantiator<
	T: Config + pallet_balances::Config<Balance = Balance>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
> {
	pub ext: OptionalExternalities,
	pub nonce: RefCell<u64>,
	pub _marker: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
}

impl<T: Config + pallet_balances::Config> Deposits<T> for Vec<AccountIdOf<T>> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>> {
		self.iter()
			.map(|x| UserToPLMCBalance::new(x.clone(), <T as pallet_balances::Config>::ExistentialDeposit::get()))
			.collect::<Vec<_>>()
	}
}

#[derive(Clone, PartialEq, Debug)]
pub struct UserToPLMCBalance<T: Config> {
	pub account: AccountIdOf<T>,
	pub plmc_amount: Balance,
}
impl<T: Config> UserToPLMCBalance<T> {
	pub fn new(account: AccountIdOf<T>, plmc_amount: Balance) -> Self {
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
impl<T: Config> From<(AccountIdOf<T>, Balance)> for UserToPLMCBalance<T> {
	fn from((account, plmc_amount): (AccountIdOf<T>, Balance)) -> Self {
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
				.and_modify(|e: &mut Balance| {
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
	fn total(&self) -> Balance {
		self.iter().map(|x| x.plmc_amount).sum()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct EvaluationParams<T: Config> {
	pub account: AccountIdOf<T>,
	pub usd_amount: Balance,
	pub receiving_account: Junction,
}
impl<T: Config> EvaluationParams<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: Balance, receiving_account: Junction) -> Self {
		EvaluationParams::<T> { account, usd_amount, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, Junction)> for EvaluationParams<T> {
	fn from((account, usd_amount, receiving_account): (AccountIdOf<T>, Balance, Junction)) -> Self {
		EvaluationParams::<T>::new(account, usd_amount, receiving_account)
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance)> for EvaluationParams<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, Balance)) -> Self {
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
	pub usd_amount: Balance,
}
impl<T: Config> UserToUSDAmount<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: Balance) -> Self {
		Self { account, usd_amount }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance)> for UserToUSDAmount<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, Balance)) -> Self {
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
				.and_modify(|stored_usd_amount: &mut Balance| match ops {
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
	fn total(&self) -> Balance {
		self.iter().map(|x| x.usd_amount).sum()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserToFundingAsset<T: Config> {
	pub account: AccountIdOf<T>,
	pub asset_amount: Balance,
	pub asset_id: AssetIdOf<T>,
}
impl<T: Config> UserToFundingAsset<T> {
	pub fn new(account: AccountIdOf<T>, asset_amount: Balance, asset_id: AssetIdOf<T>) -> Self {
		Self { account, asset_amount, asset_id }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, AssetIdOf<T>)> for UserToFundingAsset<T> {
	fn from((account, asset_amount, asset_id): (AccountIdOf<T>, Balance, AssetIdOf<T>)) -> Self {
		UserToFundingAsset::<T>::new(account, asset_amount, asset_id)
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance)> for UserToFundingAsset<T> {
	fn from((account, asset_amount): (AccountIdOf<T>, Balance)) -> Self {
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
				.and_modify(|e: &mut Balance| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*asset_amount),
						MergeOperation::Subtract => e.saturating_sub(*asset_amount),
					}
				})
				.or_insert(*asset_amount);
		}
		btree
			.into_iter()
			.map(|((account, asset_id), asset_amount)| UserToFundingAsset::new(account, asset_amount, *asset_id))
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

	fn totals(&self) -> Vec<(Self::AssetId, Balance)> {
		let mut btree = BTreeMap::new();
		for UserToFundingAsset { account: _, asset_amount, asset_id } in self.iter() {
			btree
				.entry(*asset_id)
				.and_modify(|e: &mut Balance| *e = e.saturating_add(*asset_amount))
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
			btree.insert((account.clone(), *asset_id));
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct BidParams<T: Config> {
	pub bidder: AccountIdOf<T>,
	pub amount: Balance,
	pub mode: ParticipationMode,
	pub asset: AcceptedFundingAsset,
	pub receiving_account: Junction,
}
impl<T: Config> BidParams<T> {
	pub fn new(
		bidder: AccountIdOf<T>,
		amount: Balance,
		mode: ParticipationMode,
		asset: AcceptedFundingAsset,
		receiving_account: Junction,
	) -> Self {
		Self { bidder, amount, mode, asset, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance)> for BidParams<T> {
	fn from((bidder, amount): (AccountIdOf<T>, Balance)) -> Self {
		Self {
			bidder: bidder.clone(),
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
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode)> for BidParams<T> {
	fn from((bidder, amount, mode): (AccountIdOf<T>, Balance, ParticipationMode)) -> Self {
		Self {
			bidder: bidder.clone(),
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
impl<T: Config> From<(AccountIdOf<T>, Balance, AcceptedFundingAsset)> for BidParams<T> {
	fn from((bidder, amount, asset): (AccountIdOf<T>, Balance, AcceptedFundingAsset)) -> Self {
		Self {
			bidder: bidder.clone(),
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
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset)> for BidParams<T> {
	fn from((bidder, amount, mode, asset): (AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset)) -> Self {
		Self {
			bidder: bidder.clone(),
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
impl<T: Config> From<(AccountIdOf<T>, Balance, AcceptedFundingAsset, Junction)> for BidParams<T> {
	fn from(
		(bidder, amount, asset, receiving_account): (AccountIdOf<T>, Balance, AcceptedFundingAsset, Junction),
	) -> Self {
		Self { bidder, amount, mode: ParticipationMode::Classic(1u8), asset, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset, Junction)> for BidParams<T> {
	fn from(
		(bidder, amount, mode, asset, receiving_account): (
			AccountIdOf<T>,
			Balance,
			ParticipationMode,
			AcceptedFundingAsset,
			Junction,
		),
	) -> Self {
		Self { bidder, amount, mode, asset, receiving_account }
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct ContributionParams<T: Config> {
	pub contributor: AccountIdOf<T>,
	pub amount: Balance,
	pub mode: ParticipationMode,
	pub asset: AcceptedFundingAsset,
	pub receiving_account: Junction,
}
impl<T: Config> ContributionParams<T> {
	pub fn new(
		contributor: AccountIdOf<T>,
		amount: Balance,
		mode: ParticipationMode,
		asset: AcceptedFundingAsset,
		receiving_account: Junction,
	) -> Self {
		Self { contributor, amount, mode, asset, receiving_account }
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance)> for ContributionParams<T> {
	fn from((contributor, amount): (AccountIdOf<T>, Balance)) -> Self {
		Self {
			contributor: contributor.clone(),
			amount,
			mode: ParticipationMode::Classic(1u8),
			asset: AcceptedFundingAsset::USDT,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(contributor.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode)> for ContributionParams<T> {
	fn from((contributor, amount, mode): (AccountIdOf<T>, Balance, ParticipationMode)) -> Self {
		Self {
			contributor: contributor.clone(),
			amount,
			mode,
			asset: AcceptedFundingAsset::USDT,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(contributor.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset)> for ContributionParams<T> {
	fn from(
		(contributor, amount, mode, asset): (AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset),
	) -> Self {
		Self {
			contributor: contributor.clone(),
			amount,
			mode,
			asset,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(contributor.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, AcceptedFundingAsset)> for ContributionParams<T> {
	fn from((contributor, amount, asset): (AccountIdOf<T>, Balance, AcceptedFundingAsset)) -> Self {
		Self {
			contributor: contributor.clone(),
			amount,
			mode: ParticipationMode::Classic(1u8),
			asset,
			receiving_account: Junction::AccountId32 {
				network: Some(NetworkId::Polkadot),
				id: T::AccountId32Conversion::convert(contributor.clone()),
			},
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, Balance, ParticipationMode, AcceptedFundingAsset, Junction)>
	for ContributionParams<T>
{
	fn from(
		(contributor, amount, mode, asset, receiving_account): (
			AccountIdOf<T>,
			Balance,
			ParticipationMode,
			AcceptedFundingAsset,
			Junction,
		),
	) -> Self {
		Self { contributor, amount, mode, asset, receiving_account }
	}
}
impl<T: Config> Accounts for Vec<ContributionParams<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for ContributionParams { contributor, .. } in self.iter() {
			btree.insert(contributor.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> Conversions for Vec<ContributionParams<T>> {
	type AccountId = AccountIdOf<T>;
	type AssetId = AssetIdOf<T>;

	fn to_account_asset_map(&self) -> Vec<(Self::AccountId, Self::AssetId)> {
		let mut btree = BTreeSet::new();
		for ContributionParams { contributor, asset, .. } in self.iter() {
			btree.insert((contributor.clone(), asset.id()));
		}
		btree.into_iter().collect_vec()
	}
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<T: Config> {
	pub id: Option<u32>,
	pub project_id: Option<ProjectId>,
	pub bidder: Option<AccountIdOf<T>>,
	pub status: Option<BidStatus>,
	pub original_ct_amount: Option<Balance>,
	pub original_ct_usd_price: Option<PriceOf<T>>,
	pub funding_asset: Option<AcceptedFundingAsset>,
	pub funding_asset_amount_locked: Option<Balance>,
	pub mode: Option<ParticipationMode>,
	pub plmc_bond: Option<Balance>,
	pub when: Option<BlockNumberFor<T>>,
}
impl<T: Config> BidInfoFilter<T> {
	pub(crate) fn matches_bid(&self, bid: &BidInfoOf<T>) -> bool {
		if self.id.is_some() && self.id.unwrap() != bid.id {
			return false;
		}
		if self.project_id.is_some() && self.project_id.unwrap() != bid.project_id {
			return false;
		}
		if self.bidder.is_some() && self.bidder.clone().unwrap() != bid.bidder.clone() {
			return false;
		}
		if self.status.is_some() && self.status.as_ref().unwrap() != &bid.status {
			return false;
		}
		if self.original_ct_amount.is_some() && self.original_ct_amount.unwrap() != bid.original_ct_amount {
			return false;
		}
		if self.original_ct_usd_price.is_some() && self.original_ct_usd_price.unwrap() != bid.original_ct_usd_price {
			return false;
		}
		if self.funding_asset.is_some() && self.funding_asset.unwrap() != bid.funding_asset {
			return false;
		}
		if self.funding_asset_amount_locked.is_some() &&
			self.funding_asset_amount_locked.unwrap() != bid.funding_asset_amount_locked
		{
			return false;
		}
		if self.mode.is_some() && self.mode.unwrap() != bid.mode {
			return false;
		}
		if self.plmc_bond.is_some() && self.plmc_bond.unwrap() != bid.plmc_bond {
			return false;
		}
		if self.when.is_some() && self.when.unwrap() != bid.when {
			return false;
		}

		true
	}
}
impl<T: Config> Default for BidInfoFilter<T> {
	fn default() -> Self {
		BidInfoFilter::<T> {
			id: None,
			project_id: None,
			bidder: None,
			status: None,
			original_ct_amount: None,
			original_ct_usd_price: None,
			funding_asset: None,
			funding_asset_amount_locked: None,
			mode: None,
			plmc_bond: None,
			when: None,
		}
	}
}
