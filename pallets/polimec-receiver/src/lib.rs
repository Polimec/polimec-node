#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::TypeInfo, RuntimeDebug};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::MigrationInfo;
	use frame_support::{
		pallet_prelude::*,
		traits::{tokens::Balance, Currency, ExistenceRequirement::KeepAlive, VestingSchedule},
	};
	use frame_system::pallet_prelude::*;
	use polkadot_parachain::primitives::{Id as ParaId, Sibling};
	use polkadot_runtime_parachains::origin::{ensure_parachain, Origin as ParachainOrigin};
	use sp_runtime::traits::{AccountIdConversion, Convert};
	use sp_std::prelude::*;

	type MomentOf<T> = <<T as Config>::Vesting as VestingSchedule<<T as frame_system::Config>::AccountId>>::Moment;

	#[pallet::config]
	pub trait Config: frame_system::Config
	where
		// Used for converting a polimec account into a local account for Contribution Token migrations
		Self::AccountId: From<[u8; 32]>,
	{
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type PolimecParaId: Get<ParaId>;
		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<ParachainOrigin, <Self as Config>::RuntimeOrigin>>;
		type Vesting: VestingSchedule<Self::AccountId, Currency = Self::Balances>;
		type Balances: Currency<Self::AccountId, Balance = Self::Balance>;
		type Balance: Balance + From<u128> + MaybeSerializeDeserialize;
		type GenesisMoment: Get<MomentOf<Self>>;
		type MigrationInfoToPerBlockBalance: Convert<MigrationInfo, Self::Balance>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		T::AccountId: From<[u8; 32]>,
	{
		MigrationsExecutedForUser { user: T::AccountId, migrations: Vec<MigrationInfo> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NoneValue,
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> where T::AccountId: From<[u8; 32]> {}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<[u8; 32]>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0))]
		pub fn migrate_for_user(
			origin: OriginFor<T>,
			user: [u8; 32],
			migrations: Vec<MigrationInfo>,
		) -> DispatchResult {
			let para_id: ParaId = ensure_parachain(<T as Config>::RuntimeOrigin::from(origin))?;
			let user: T::AccountId = user.into();
			let polimec_id = T::PolimecParaId::get();
			let polimec_soverign_account = Sibling(polimec_id).into_account_truncating();

			ensure!(para_id == T::PolimecParaId::get(), "Only Polimec Parachain can call migrations");

			for migration in migrations.clone() {
				T::Balances::transfer(
					&polimec_soverign_account,
					&user,
					migration.contribution_token_amount.into(),
					KeepAlive,
				)?;
				T::Vesting::add_vesting_schedule(
					&user,
					migration.contribution_token_amount.into(),
					T::MigrationInfoToPerBlockBalance::convert(migration),
					T::GenesisMoment::get(),
				)?;
			}

			Self::deposit_event(Event::MigrationsExecutedForUser { user, migrations });

			Ok(())
		}
	}
}

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct MigrationInfo {
	pub contribution_token_amount: u128,
	pub vesting_time: u64,
}

