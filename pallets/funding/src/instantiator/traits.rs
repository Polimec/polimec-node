use super::{Config, UserToPLMCBalance};
use crate::Balance;
use alloc::vec::Vec;

/// The trait for existential deposits in the system.
pub trait Deposits<T: Config> {
	fn existential_deposits(&self) -> Vec<UserToPLMCBalance<T>>;
}

/// The trait for accounts that can be iterated over and merged.
pub trait Accounts {
	type Account;

	fn accounts(&self) -> Vec<Self::Account>;
}

/// A type of operation for merging accounts.
pub enum MergeOperation {
	Add,
	Subtract,
}

/// A trait for merging accounts, allowing for operations like addition and subtraction of balances.
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

/// A trait for total balances, providing a method to retrieve the total balance.
pub trait Total {
	fn total(&self) -> Balance;
}

/// A trait for retrieving totals of assets, providing a method to get a vector of asset IDs and their corresponding balances.
pub trait Totals {
	type AssetId;
	fn totals(&self) -> Vec<(Self::AssetId, Balance)>;
}

/// A trait for converting data structures into a vector of tuples containing account IDs and asset IDs.
pub trait Conversions {
	type AccountId;
	type AssetId;
	fn to_account_asset_map(&self) -> Vec<(Self::AccountId, Self::AssetId)>;
}
