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

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::{
	ensure,
	pallet_prelude::DispatchResult,
	traits::{ChangeMembers, InitializeMembers},
};
use polimec_traits::{Credential, MemberRole, PolimecMembers};
use sp_runtime::{traits::StaticLookup, DispatchError};

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Required origin for adding a member (though can always be Root).
		type AddOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for removing a member (though can always be Root).
		type RemoveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for adding and removing a member in a single action.
		/// TODO: Not used ATM
		type SwapOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for resetting membership.
		/// TODO: Not used ATM
		type ResetOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Required origin for setting or resetting the prime member.
		/// TODO: Not used ATM
		type PrimeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The receiver of the signal for when the membership has been initialized. This happens
		/// pre-genesis and will usually be the same as `MembershipChanged`. If you need to do
		/// something different on initialization, then you can change this accordingly.
		type MembershipInitialized: InitializeMembers<Self::AccountId>;

		/// The receiver of the signal for when the membership has changed.
		type MembershipChanged: ChangeMembers<Self::AccountId>;

		// Weight information for extrinsics in this pallet.
		// type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Maps member type to members of each type.
	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config> = StorageDoubleMap<_, Twox64Concat, MemberRole, Twox64Concat, T::AccountId, ()>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The given member was added; see the transaction for who.
		MemberAdded,
		/// The given member was removed; see the transaction for who.
		MemberRemoved,
		/// Two members were swapped; see the transaction for who.
		MembersSwapped,
		/// The membership was reset; see the transaction for who the new set is.
		MembersReset,
		/// One of the members' keys changed.
		KeyChanged,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Already a member.
		AlreadyMember,
		/// Not a member.
		NotMember,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub issuers: Vec<T::AccountId>,
		pub retails: Vec<T::AccountId>,
		pub professionals: Vec<T::AccountId>,
		pub institutionals: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				issuers: Default::default(),
				retails: Default::default(),
				professionals: Default::default(),
				institutionals: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			use sp_std::collections::btree_set::BTreeSet;

			let issuers_set: BTreeSet<_> = self.issuers.iter().collect();
			assert_eq!(
				issuers_set.len(),
				self.issuers.len(),
				"Issuers cannot contain duplicate accounts."
			);

			let retails_set: BTreeSet<_> = self.retails.iter().collect();
			assert_eq!(
				retails_set.len(),
				self.retails.len(),
				"Issuers cannot contain duplicate accounts."
			);

			let professionals_set: BTreeSet<_> = self.professionals.iter().collect();
			assert_eq!(
				professionals_set.len(),
				self.professionals.len(),
				"Issuers cannot contain duplicate accounts."
			);

			let institutionals_set: BTreeSet<_> = self.institutionals.iter().collect();
			assert_eq!(
				institutionals_set.len(),
				self.institutionals.len(),
				"Issuers cannot contain duplicate accounts."
			);

			Pallet::<T>::initialize_members(&MemberRole::Issuer, &self.issuers);
			Pallet::<T>::initialize_members(&MemberRole::Retail, &self.retails);
			Pallet::<T>::initialize_members(&MemberRole::Professional, &self.professionals);
			Pallet::<T>::initialize_members(&MemberRole::Institutional, &self.institutionals);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a member `who` to the set.
		///
		/// May only be called from `T::AddOrigin`.
		// TODO: Set a proper weight
		#[pallet::weight(1)]
		pub fn add_member(origin: OriginFor<T>, credential: Credential, who: AccountIdLookupOf<T>) -> DispatchResult {
			T::AddOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::do_add_member(&who, &credential)?;
			Ok(())
		}

		/// Remove a member `who` from the set.
		///
		/// May only be called from `T::RemoveOrigin`.
		// TODO: Set a proper weight
		#[pallet::weight(1)]
		pub fn remove_member(
			origin: OriginFor<T>, credential: Credential, who: AccountIdLookupOf<T>,
		) -> DispatchResult {
			T::RemoveOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::do_remove_member(&who, &credential)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn do_add_member(who: &T::AccountId, credential: &Credential) -> Result<(), DispatchError> {
		// TODO: This is a placeholder, we still dont't know the actual structure of a `Credential`
		let role = credential.role;
		ensure!(!Self::is_in(&role, who), Error::<T>::AlreadyMember);

		Self::do_add_member_with_role(&role, who)?;
		Ok(())
	}

	fn do_add_member_with_role(role: &MemberRole, who: &T::AccountId) -> Result<(), DispatchError> {
		Members::<T>::insert(role, who, ());
		Self::deposit_event(Event::MemberAdded);
		Ok(())
	}

	fn do_remove_member(who: &T::AccountId, credential: &Credential) -> Result<(), DispatchError> {
		// TODO: This is a placeholder, we still dont't know the actual structure of a `Credential`
		let role = credential.role;
		ensure!(Self::is_in(&role, who), Error::<T>::NotMember);

		Self::do_remove_member_with_role(&role, who)?;
		Ok(())
	}

	fn do_remove_member_with_role(role: &MemberRole, who: &T::AccountId) -> Result<(), DispatchError> {
		Members::<T>::remove(role, who);
		Self::deposit_event(Event::MemberRemoved);
		Ok(())
	}
}

use sp_std::{vec, vec::Vec};

impl<T: Config> PolimecMembers<T::AccountId> for Pallet<T> {
	/// Chech if `who` is in the `role` set
	fn is_in(role: &MemberRole, who: &T::AccountId) -> bool {
		<Members<T>>::contains_key(role, who)
	}

	/// Add `who` to the `role` set
	fn add_member(role: &MemberRole, who: &T::AccountId) -> Result<(), DispatchError> {
		Self::do_add_member_with_role(role, who)
	}

	/// Utility function to set a vector of `member` during the genesis
	fn initialize_members(role: &MemberRole, members: &[T::AccountId]) {
		if !members.is_empty() {
			for member in members {
				assert!(!Self::is_in(role, member), "Members are already initialized!");
			}
			for member in members {
				let _ = Self::do_add_member_with_role(role, member);
			}
		}
	}

	fn get_members_of(role: &MemberRole) -> Vec<T::AccountId> {
		<Members<T>>::iter_key_prefix(role).collect()
	}

	fn get_roles_of(who: &T::AccountId) -> Vec<MemberRole> {
		let mut user_roles = vec![];
		for role in MemberRole::iterator() {
			if let Some(()) = Members::<T>::get(role, who) {
				user_roles.push(*role)
			}
		}
		user_roles
	}
}
