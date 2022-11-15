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

#[frame_support::pallet]
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
	pub type Members<T: Config> =
		StorageDoubleMap<_, Twox64Concat, T::AccountId, Twox64Concat, MemberRole, ()>;

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
		/// Too many members.
		TooManyMembers,
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

			Pallet::<T>::initialize_members(&self.issuers, &MemberRole::Issuer);

			Pallet::<T>::initialize_members(&self.retails, &MemberRole::Retail);

			Pallet::<T>::initialize_members(&self.professionals, &MemberRole::Professional);

			Pallet::<T>::initialize_members(&self.institutionals, &MemberRole::Institutional);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a member `who` to the set.
		///
		/// May only be called from `T::AddOrigin`.
		#[pallet::weight(50_000_000)]
		pub fn add_member(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
			credential: Credential,
		) -> DispatchResult {
			T::AddOrigin::ensure_origin(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::do_add_member(&who, &credential)?;
			Ok(())
		}

		/// Remove a member `who` to the set.
		///
		/// May only be called from `T::RemoveOrigin`.
		#[pallet::weight(50_000_000)]
		pub fn remove_member(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
			credential: Credential,
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
		ensure!(!Self::is_in(who, &role), Error::<T>::AlreadyMember);

		Self::do_add_member_with_role(who, &role)?;
		Ok(())
	}

	fn do_add_member_with_role(who: &T::AccountId, role: &MemberRole) -> Result<(), DispatchError> {
		Members::<T>::insert(who, role, ());
		Self::deposit_event(Event::MemberAdded);
		Ok(())
	}

	fn do_remove_member(who: &T::AccountId, credential: &Credential) -> Result<(), DispatchError> {
		let role = credential.role;

		Self::do_add_member_with_role(who, &role)?;

		Self::deposit_event(Event::MemberRemoved);
		Ok(())
	}
}

impl<T: Config> PolimecMembers<T::AccountId> for Pallet<T> {
	/// Chech if `who` is in the `role` set
	fn is_in(who: &T::AccountId, role: &MemberRole) -> bool {
		<Members<T>>::contains_key(who, role)
	}

	/// Add `who` to the `role` set
	fn add_member(who: &T::AccountId, role: &MemberRole) -> Result<(), DispatchError> {
		Self::do_add_member_with_role(who, role)
	}

	/// Utility function to set `members` during the genesis
	fn initialize_members(members: &[T::AccountId], role: &MemberRole) {
		if !members.is_empty() {
			for member in members {
				assert!(!Self::is_in(member, role), "Members are already initialized!");
			}
			for member in members {
				let _ = Self::do_add_member_with_role(member, role);
			}
		}
	}
}
