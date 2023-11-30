#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{dispatch::TypeInfo, RuntimeDebug};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{tokens::Balance, Currency, ExistenceRequirement::KeepAlive, VestingSchedule},
	};
	use frame_system::pallet_prelude::*;
	use polimec_traits::migration_types::{Migration, MigrationInfo, MigrationOrigin, Migrations, ParticipationType};
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
		type MaxMigrations: Get<u128>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn something)]
	// pub type ExecutedMigrations<T> = StorageNMap<_, BoundedVec<MigrationOrigin, T::MaxMigrations>>;
	pub type ExecutedMigrations<T> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, [u8; 32]>,
			NMapKey<Blake2_128Concat, ParticipationType>,
			NMapKey<Blake2_128Concat, u32>,
		),
		bool,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config>
	where
		T::AccountId: From<[u8; 32]>,
	{
		/// A Migration executed sucessfully
		MigrationExecuted { migration: Migration },
		/// A Migration was found which wa already executed, and was skipped.
		DuplicatedMigrationSkipped { migration: Migration },
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
		pub fn execute_migrations(origin: OriginFor<T>, migrations: Migrations) -> DispatchResult {
			let para_id: ParaId = ensure_parachain(<T as Config>::RuntimeOrigin::from(origin))?;
			let polimec_id = T::PolimecParaId::get();
			let polimec_soverign_account = Sibling(polimec_id).into_account_truncating();

			ensure!(para_id == T::PolimecParaId::get(), "Only Polimec Parachain can call migrations");
			for migration @ Migration {
				origin: MigrationOrigin { user, id, participation_type },
				info: MigrationInfo { contribution_token_amount, .. },
			} in migrations.clone().inner()
			{
				let already_executed = ExecutedMigrations::<T>::get((user.clone(), participation_type, id));
				if already_executed {
					Self::deposit_event(Event::DuplicatedMigrationSkipped { migration });
					continue
				}
				T::Balances::transfer(
					&polimec_soverign_account,
					&user.into(),
					contribution_token_amount.into(),
					KeepAlive,
				)?;
				T::Vesting::add_vesting_schedule(
					&user.into(),
					contribution_token_amount.into(),
					T::MigrationInfoToPerBlockBalance::convert(migration.info.clone()),
					T::GenesisMoment::get(),
				)?;
				ExecutedMigrations::<T>::insert((user.clone(), participation_type, id), true);
				Self::deposit_event(Event::MigrationExecuted { migration });
			}

			Ok(())
		}
	}
}
