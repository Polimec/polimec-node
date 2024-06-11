use super::*;
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
	pub expected_state: ProjectStatus,
	pub metadata: ProjectMetadataOf<T>,
	pub issuer: AccountIdOf<T>,
	pub evaluations: Vec<UserToUSDBalance<T>>,
	pub bids: Vec<BidParams<T>>,
	pub community_contributions: Vec<ContributionParams<T>>,
	pub remainder_contributions: Vec<ContributionParams<T>>,
}

#[cfg(feature = "std")]
pub type OptionalExternalities = Option<RefCell<sp_io::TestExternalities>>;

#[cfg(not(feature = "std"))]
pub type OptionalExternalities = Option<()>;

pub struct Instantiator<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct UserToUSDBalance<T: Config> {
	pub account: AccountIdOf<T>,
	pub usd_amount: BalanceOf<T>,
}
impl<T: Config> UserToUSDBalance<T> {
	pub fn new(account: AccountIdOf<T>, usd_amount: BalanceOf<T>) -> Self {
		Self { account, usd_amount }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToUSDBalance<T> {
	fn from((account, usd_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToUSDBalance::<T>::new(account, usd_amount)
	}
}
impl<T: Config> Accounts for Vec<UserToUSDBalance<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToUSDBalance { account, usd_amount: _ } in self {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> AccountMerge for Vec<UserToUSDBalance<T>> {
	type Inner = UserToUSDBalance<T>;

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToUSDBalance { account, usd_amount } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut BalanceOf<T>| {
					*e = match ops {
						MergeOperation::Add => e.saturating_add(*usd_amount),
						MergeOperation::Subtract => e.saturating_sub(*usd_amount),
					}
				})
				.or_insert(*usd_amount);
		}
		btree.into_iter().map(|(account, usd_amount)| UserToUSDBalance::new(account, usd_amount)).collect()
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserToForeignAssets<T: Config> {
	pub account: AccountIdOf<T>,
	pub asset_amount: BalanceOf<T>,
	pub asset_id: AssetIdOf<T>,
}
impl<T: Config> UserToForeignAssets<T> {
	pub fn new(account: AccountIdOf<T>, asset_amount: BalanceOf<T>, asset_id: AssetIdOf<T>) -> Self {
		Self { account, asset_amount, asset_id }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)> for UserToForeignAssets<T> {
	fn from((account, asset_amount, asset_id): (AccountIdOf<T>, BalanceOf<T>, AssetIdOf<T>)) -> Self {
		UserToForeignAssets::<T>::new(account, asset_amount, asset_id)
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for UserToForeignAssets<T> {
	fn from((account, asset_amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		UserToForeignAssets::<T>::new(account, asset_amount, AcceptedFundingAsset::USDT.to_assethub_id())
	}
}
impl<T: Config> Accounts for Vec<UserToForeignAssets<T>> {
	type Account = AccountIdOf<T>;

	fn accounts(&self) -> Vec<Self::Account> {
		let mut btree = BTreeSet::new();
		for UserToForeignAssets { account, .. } in self.iter() {
			btree.insert(account.clone());
		}
		btree.into_iter().collect_vec()
	}
}
impl<T: Config> AccountMerge for Vec<UserToForeignAssets<T>> {
	type Inner = UserToForeignAssets<T>;

	fn merge_accounts(&self, ops: MergeOperation) -> Self {
		let mut btree = BTreeMap::new();
		for UserToForeignAssets { account, asset_amount, asset_id } in self.iter() {
			btree
				.entry(account.clone())
				.and_modify(|e: &mut (BalanceOf<T>, u32)| {
					e.0 = match ops {
						MergeOperation::Add => e.0.saturating_add(*asset_amount),
						MergeOperation::Subtract => e.0.saturating_sub(*asset_amount),
					}
				})
				.or_insert((*asset_amount, *asset_id));
		}
		btree.into_iter().map(|(account, info)| UserToForeignAssets::new(account, info.0, info.1)).collect()
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct BidParams<T: Config> {
	pub bidder: AccountIdOf<T>,
	pub amount: BalanceOf<T>,
	pub multiplier: MultiplierOf<T>,
	pub asset: AcceptedFundingAsset,
}
impl<T: Config> BidParams<T> {
	pub fn new(bidder: AccountIdOf<T>, amount: BalanceOf<T>, multiplier: u8, asset: AcceptedFundingAsset) -> Self {
		Self { bidder, amount, multiplier: multiplier.try_into().map_err(|_| ()).unwrap(), asset }
	}

	pub fn new_with_defaults(bidder: AccountIdOf<T>, amount: BalanceOf<T>) -> Self {
		Self {
			bidder,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for BidParams<T> {
	fn from((bidder, amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		Self {
			bidder,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, u8)> for BidParams<T> {
	fn from((bidder, amount, multiplier): (AccountIdOf<T>, BalanceOf<T>, u8)) -> Self {
		Self {
			bidder,
			amount,
			multiplier: multiplier.try_into().unwrap_or_else(|_| panic!("Failed to create multiplier")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, u8, AcceptedFundingAsset)> for BidParams<T> {
	fn from((bidder, amount, multiplier, asset): (AccountIdOf<T>, BalanceOf<T>, u8, AcceptedFundingAsset)) -> Self {
		Self {
			bidder,
			amount,
			multiplier: multiplier.try_into().unwrap_or_else(|_| panic!("Failed to create multiplier")),
			asset,
		}
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

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, bound(serialize = ""), bound(deserialize = ""))]
pub struct ContributionParams<T: Config> {
	pub contributor: AccountIdOf<T>,
	pub amount: BalanceOf<T>,
	pub multiplier: MultiplierOf<T>,
	pub asset: AcceptedFundingAsset,
}
impl<T: Config> ContributionParams<T> {
	pub fn new(contributor: AccountIdOf<T>, amount: BalanceOf<T>, multiplier: u8, asset: AcceptedFundingAsset) -> Self {
		Self { contributor, amount, multiplier: multiplier.try_into().map_err(|_| ()).unwrap(), asset }
	}

	pub fn new_with_defaults(contributor: AccountIdOf<T>, amount: BalanceOf<T>) -> Self {
		Self {
			contributor,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>)> for ContributionParams<T> {
	fn from((contributor, amount): (AccountIdOf<T>, BalanceOf<T>)) -> Self {
		Self {
			contributor,
			amount,
			multiplier: 1u8.try_into().unwrap_or_else(|_| panic!("multiplier could not be created from 1u8")),
			asset: AcceptedFundingAsset::USDT,
		}
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)> for ContributionParams<T> {
	fn from((contributor, amount, multiplier): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>)) -> Self {
		Self { contributor, amount, multiplier, asset: AcceptedFundingAsset::USDT }
	}
}
impl<T: Config> From<(AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset)> for ContributionParams<T> {
	fn from(
		(contributor, amount, multiplier, asset): (AccountIdOf<T>, BalanceOf<T>, MultiplierOf<T>, AcceptedFundingAsset),
	) -> Self {
		Self { contributor, amount, multiplier, asset }
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BidInfoFilter<T: Config> {
	pub id: Option<u32>,
	pub project_id: Option<ProjectId>,
	pub bidder: Option<AccountIdOf<T>>,
	pub status: Option<BidStatus<BalanceOf<T>>>,
	pub original_ct_amount: Option<BalanceOf<T>>,
	pub original_ct_usd_price: Option<PriceOf<T>>,
	pub final_ct_amount: Option<BalanceOf<T>>,
	pub final_ct_usd_price: Option<PriceOf<T>>,
	pub funding_asset: Option<AcceptedFundingAsset>,
	pub funding_asset_amount_locked: Option<BalanceOf<T>>,
	pub multiplier: Option<MultiplierOf<T>>,
	pub plmc_bond: Option<BalanceOf<T>>,
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
		if self.final_ct_amount.is_some() && self.final_ct_amount.unwrap() != bid.final_ct_amount {
			return false;
		}
		if self.final_ct_usd_price.is_some() && self.final_ct_usd_price.unwrap() != bid.final_ct_usd_price {
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
		if self.multiplier.is_some() && self.multiplier.unwrap() != bid.multiplier {
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
			final_ct_amount: None,
			final_ct_usd_price: None,
			funding_asset: None,
			funding_asset_amount_locked: None,
			multiplier: None,
			plmc_bond: None,
			when: None,
		}
	}
}
