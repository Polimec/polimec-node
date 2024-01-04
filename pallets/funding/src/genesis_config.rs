use frame_support::pallet_macros::pallet_section;

#[pallet_section]
mod genesis_config {
	use crate::{
		instantiator::{Instantiator, TestProjectParams},
		pallet, BalanceOf,
	};
	use frame_support::{
		dispatch::{Decode, Encode},
		pallet_prelude::BuildGenesisConfig,
		Parameter,
	};
	use frame_system::pallet_prelude::BlockNumberFor;
	use sp_runtime::traits::Member;
	use sp_std::marker::PhantomData;

	#[pallet::genesis_config]
	#[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
	pub struct GenesisConfig<T: Config>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as Config>::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		pub starting_projects: Vec<TestProjectParams<T>>,
		pub phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as Config>::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		fn default() -> Self {
			Self { starting_projects: vec![], phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as Config>::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		fn build(&self) {
			{
				type GenesisInstantiator<T> =
					Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;
				let mut inst = GenesisInstantiator::<T>::new(None);
				<T as Config>::SetPrices::set_prices();
				let current_block = <frame_system::Pallet<T>>::block_number();
				instantiator::async_features::create_multiple_projects_at(inst, self.starting_projects.clone());
			}
		}
	}
	impl<T: Config> GenesisConfig<T>
	where
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		<T as Config>::AllPalletsWithoutSystem:
			OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		<T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
		<T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
	{
		/// Direct implementation of `GenesisBuild::build_storage`.
		///
		/// Kept in order not to break dependency.
		pub fn build(&self) {
			<Self as BuildGenesisConfig>::build(self)
		}
	}
}

// #[cfg(not(all(feature = "testing-node", feature = "std")))]
// #[pallet_section]
// mod genesis_config {
// 	#[pallet::genesis_config]
// 	pub struct GenesisConfig<T: Config> {
//         phantom: PhantomData<T>,
//     }
//
// 	impl<T: Config> Default for GenesisConfig<T> {
// 		fn default() -> Self {
// 			Self {phantom: PhantomData}
// 		}
// 	}
//
//     #[pallet::genesis_build]
//     impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
//         fn build(&self) {}
//     }
//
// }
