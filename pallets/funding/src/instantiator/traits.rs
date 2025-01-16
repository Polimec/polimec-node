use super::{Config, UserToPLMCBalance, Vec};
use crate::Balance;

pub trait Deposits<T: Config> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>>;
}
pub trait Accounts {
	type Account;

	fn accounts(&self) -> Vec<Self::Account>;
}

pub enum MergeOperation {
	Add,
	Subtract,
}
pub trait AccountMerge:
	Clone + Accounts<Account: Clone + PartialEq> + Sized + IntoIterator<Item = Self::Inner> + FromIterator<Self::Inner>
{
	/// The inner type of the Vec implementing this Trait.
	type Inner: PartialEq + Clone;

	fn get_account(inner: Self::Inner) -> Self::Account;

	/// Merge accounts in the list based on the operation. Only the first receiving account will be used for the merged map.
	fn merge_accounts(&self, ops: MergeOperation) -> Self;

	/// Subtract amount of the matching accounts in the other list from the current list.
	/// If the account is not present in the current list, it is ignored.
	fn subtract_accounts(&self, other_list: Self) -> Self {
		let current_accounts = self.accounts();
		let filtered_list: Self =
			other_list.into_iter().filter(|x| current_accounts.contains(&Self::get_account(x.clone()))).collect();
		let combined: Self = self.clone().into_iter().chain(filtered_list).collect();
		combined.merge_accounts(MergeOperation::Subtract)
	}

	fn sum_accounts(&self, other_list: Self) -> Self {
		let self_iter = self.clone().into_iter();
		let other_iter = other_list.into_iter();

		let combined: Self = self_iter.chain(other_iter).collect();
		combined.merge_accounts(MergeOperation::Add)
	}
}

pub trait Total {
	fn total(&self) -> Balance;
}

pub trait Totals {
	type AssetId;
	fn totals(&self) -> Vec<(Self::AssetId, Balance)>;
}

pub trait Conversions {
	type AccountId;
	type AssetId;
	fn to_account_asset_map(&self) -> Vec<(Self::AccountId, Self::AssetId)>;
}
