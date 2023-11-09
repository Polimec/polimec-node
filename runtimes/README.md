In this folder, there are 3 (+1) runtimes

1. "Standalone" runtime.
2. "Testnet" runtime.
3. "Base" runtime.

+ The "Standalone" runtime is the runtime that is used for the standalone node. It is the same as the "Testnet" runtime, but it works as a standalone chain. Useful for testing.

+ The "Testnet" runtime is the runtime that is used for the testnet node.

+ The "Base" runtime is the runtime that is used for the base node. It is basically the ["Substrate Cumulus Parachain Template"](https://github.com/substrate-developer-hub/substrate-parachain-template). It includes the following extra pallets:
    + `pallet_sudo`
    + `parachain_staking` by KILT
        + Useful as a starter parachain for the Polkadot/Kusama auction, then will be updated to the "Testnet" runtime.

+ The "Common" runtime contains the common code that is used by all the other runtimes.