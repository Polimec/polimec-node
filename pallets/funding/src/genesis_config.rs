use frame_support::pallet_macros::*;
use frame_support::pallet_macros::pallet_section;

#[cfg(all(feature = "testing-node", feature = "std"))]
#[pallet_section]
mod genesis_config {
    use crate::instantiator::async_features::create_multiple_projects_at;
    use crate::instantiator::TestProjectParams;
    use sp_std::marker::PhantomData;
    use frame_support::dispatch::{Decode, Encode};
    use frame_support::pallet_prelude::BuildGenesisConfig;
    use frame_support::Parameter;
    use frame_system::pallet_prelude::BlockNumberFor;
    use sp_runtime::traits::Member;
    use crate::{BalanceOf, instantiator, pallet};

    #[pallet::genesis_config]
    #[derive(Clone, PartialEq, Eq, Debug, Encode, Decode)]
    pub struct GenesisConfig<T: Config>
        where
            T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
            <T as Config>::AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>>
            + OnIdle<BlockNumberFor<T>>
            + OnInitialize<BlockNumberFor<T>>
            + Sync
            + Send
            + 'static,
            <T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
            <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
            <T as Config>::ProjectIdentifier: Send + Sync,
            <T as Config>::Balance: Send + Sync,
            <T as Config>::Price: Send + Sync,
            <T as Config>::StringLimit: Send + Sync,
            <T as Config>::Multiplier: Send + Sync,
    {
        pub starting_projects: Vec<TestProjectParams<T>>,
        pub phantom: PhantomData<T>,
    }

    impl<T: Config> Default for GenesisConfig<T>
        where
            T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
            <T as Config>::AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>>
            + OnIdle<BlockNumberFor<T>>
            + OnInitialize<BlockNumberFor<T>>
            + Sync
            + Send
            + 'static,
            <T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
            <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
            <T as Config>::ProjectIdentifier: Send + Sync,
            <T as Config>::Balance: Send + Sync,
            <T as Config>::Price: Send + Sync,
            <T as Config>::StringLimit: Send + Sync,
            <T as Config>::Multiplier: Send + Sync,
    {
        fn default() -> Self {
            Self { starting_projects: vec![], phantom: PhantomData }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
        where
            T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
            <T as Config>::AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>>
            + OnIdle<BlockNumberFor<T>>
            + OnInitialize<BlockNumberFor<T>>
            + Sync
            + Send
            + 'static,
            <T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
            <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
            <T as Config>::ProjectIdentifier: Send + Sync,
            <T as Config>::Balance: Send + Sync,
            <T as Config>::Price: Send + Sync,
            <T as Config>::StringLimit: Send + Sync,
            <T as Config>::Multiplier: Send + Sync,
    {
        fn build(&self) {
            {
                type GenesisInstantiator<T> =
                instantiator::Instantiator<T, <T as Config>::AllPalletsWithoutSystem, <T as Config>::RuntimeEvent>;
                let mut inst = GenesisInstantiator::<T>::new(None);
                <T as Config>::SetPrices::set_prices();
                let current_block = <frame_system::Pallet<T>>::block_number();
                create_multiple_projects_at(inst, self.starting_projects.clone());
            }
        }
    }
    impl<T: Config> GenesisConfig<T>
        where
            T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
            <T as Config>::AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>>
            + OnIdle<BlockNumberFor<T>>
            + OnInitialize<BlockNumberFor<T>>
            + Sync
            + Send
            + 'static,
            <T as Config>::RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member,
            <T as pallet_balances::Config>::Balance: Into<BalanceOf<T>>,
            <T as Config>::ProjectIdentifier: Send + Sync,
            <T as Config>::Balance: Send + Sync,
            <T as Config>::Price: Send + Sync,
            <T as Config>::StringLimit: Send + Sync,
            <T as Config>::Multiplier: Send + Sync,
    {
        /// Direct implementation of `GenesisBuild::build_storage`.
        ///
        /// Kept in order not to break dependency.
        pub fn build(&self) {
            <Self as BuildGenesisConfig>::build(self)
        }
    }
}

#[cfg(not(all(feature = "testing-node", feature = "std")))]
#[pallet_section]
mod genesis_config {}