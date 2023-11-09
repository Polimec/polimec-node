In this folder, there are 3 (+1) runtimes

1. "Testnet" runtime.
2. "Base" runtime.

+ The "Testnet" runtime is the runtime that is used for the testnet node.

+ The "Base" runtime is the runtime that is used for the base node. It is basically the ["Substrate Cumulus Parachain Template"](https://github.com/substrate-developer-hub/substrate-parachain-template) but it includes the following extra pallets:
    + `pallet_sudo`
    + `parachain_staking` by KILT
        + Useful as a starter parachain for the Polkadot/Kusama auction, then will be updated to the "Testnet" runtime.

+ The "Common" runtime contains the common code that is used by all the other runtimes.