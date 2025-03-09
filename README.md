# Polimec

Polimec is a blockchain platform built on Substrate, designed for robustness and scalability. This README provides guidelines for setting up and running Polimec as a Parachain using Zombienet.

## Table of Contents
1. [Requirements](#requirements)
2. [Installation Guide](#installation-guide)
   - [Setting up the Relay Chain](#setting-up-the-relay-chain)
   - [Setting up Polimec](#setting-up-polimec)
   - [Running the Network](#running-the-network)
3. [Additional Resources](#additional-resources)
4. [Contributing](#contributing)

## Requirements

- [Rust Programming Language](https://rustup.rs/) - Ensure you have the version defined in the `rust-toolchain.toml`.
- [Zombienet](https://github.com/paritytech/zombienet) - For network simulation and testing. Installation guide can be found [here](https://paritytech.github.io/zombienet/install.html).

## Installation Guide

### Setting up the Relay Chain

1. **Clone the Polkadot Repository**:
   `git clone https://github.com/paritytech/polkadot`
2. **Checkout the specific branch**:
   `git checkout release-v1.0.0`
3. **Compile the source**:
   `cargo build --release --package polkadot`
4. **Add the Polkadot binary to your PATH**:
   `cp target/release/polkadot ~/.local/bin/polkadot`

### Setting up Polimec

1. **Clone the Polimec Repository**:
   `git clone <Polimec Repository URL>`
2. **Compile the source**:
   `cargo build --release --package polimec-node`
3. **Add the Polimec node binary to your PATH**:
   `cp target/release/polimec-node ~/.local/bin/polimec`

### Running the Network

1. **Launch the network with Zombienet**:
   `zombienet spawn scripts/zombienet/native/local-testnet.toml`
2. A Polimec node is now reachable at https://polkadot.js.org/apps/?rpc=ws://127.0.0.1:8080#/explorer

## Additional Resources

- **Compilation of the Runtime**: Use [srtool](https://github.com/paritytech/srtool) for compiling the runtime and generating the WASM blob.

```
== Compact
 Version          : polimec-mainnet-1000000 (polimec-mainnet-0.tx7.au1)
 Metadata         : V14
 Size             : 6.09 MB (6388233 bytes)
 setCode          : 0x51c78d58adc2b79d41ec2bcca074a17685bedadaf9c4e5a8d6c2426055262192
 authorizeUpgrade : 0x6bb01a720ef423759bc6541243cccffd0ec35dee69116e540906c23e2d61593f
 IPFS             : QmRvqT7DfSFNYszTC9dH99Fdqta5XNEfnd4tXxM9vaW69y
 BLAKE2_256       : 0x78d8e5ac7bcaf6ad2f38701928f2c387a32dbc453ab58c000f88e48fa2dadd4c
 Wasm             : runtimes/polimec/target/srtool/production/wbuild/polimec-runtime/polimec_runtime.compact.wasm

== Compressed
 Version          : polimec-mainnet-1000000 (polimec-mainnet-0.tx7.au1)
 Metadata         : V14
 Size             : 1.52 MB (1592474 bytes)
 Compression      : 75.08%
 setCode          : 0x7c0c21c12ca5722f9f0a916a9ef96be22d56452149b69c8a51344f5ecf651075
 authorizeUpgrade : 0x31a4614dcda36f15a84899147df88f31d128b4d6204b32b7510d7a5cf0e8bf84
 IPFS             : Qme6jU3X5nWWpsgFFLFnhabCuFEphwvzJ5g91s8bQ3Ab3b
 BLAKE2_256       : 0x084190d81d4d8e2a17842ce8caebab8fc6051069ab0c06ca94bd8b4984d52dd7
 Wasm             : runtimes/polimec/target/srtool/production/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm
```
- **Utility Scripts**: Check the `scripts` directory for useful scripts. Use [just](https://github.com/casey/just) for executing scripts, e.g., `$ just build-parachain-node`.

```
Available recipes:
    benchmark-extrinsics pallet="pallet-funding" extrinsics="*"
    benchmark-pallet chain="polimec-paseo-local" pallet="pallet-dispenser" # Run the Runtime benchmarks for a specific pallet
    benchmark-runtime
    build-polimec-paseo-srtool
    build-polimec-polkadot-srtool                    # Build the "Base" Runtime using srtool
    create-chainspec-base                            # Create the "Base" Runtime Chainspec
    default                                          # Help information
    dev path_to_file="scripts/zombienet/polimec-paseo-local.toml" # Use zombienet to spawn rococo + polimec testnet
    docker-build tag="latest" package="polimec-node" # Build the Node Docker Image
    dry-run-benchmarks mode="fast-mode" pallet="*" extrinsic="*"
    test-integration                                 # Run the integration tests
    test-runtime-features runtime="polimec-runtime"  # Test the runtimes features
```


## Contributing

We welcome contributions! Feel free to raise issues or submit pull requests. Your feedback and contributions are valued as we develop Polimec into a robust and versatile software.
