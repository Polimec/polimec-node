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
pub trait AccountMerge: Accounts + Sized {
	/// The inner type of the Vec implementing this Trait.
	type Inner;
	/// Merge accounts in the list based on the operation.
	fn merge_accounts(&self, ops: MergeOperation) -> Self;
	/// Subtract amount of the matching accounts in the other list from the current list.
	/// If the account is not present in the current list, it is ignored.
	fn subtract_accounts(&self, other_list: Self) -> Self;

	fn sum_accounts(&self, other_list: Self) -> Self;
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
