# Polimec

> **Warning**: This project is under HEAVY development.

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
 Version          : polimec-mainnet-2 (polimec-mainnet-0.tx1.au1)
 Metadata         : V14
 Size             : 5.95 MB (6238415 bytes)
 setCode          : 0x0582b7c4d42bb46593ac2788d17c3d083eedfbc9d8ef3fb0c912378189d44f94
 authorizeUpgrade : 0xe8d26589c2c5257c3f52e21ba420eb0c6fd25fa5cee0878bc183ca0256dee9bc
 IPFS             : Qmbi9ymmCdJVJCLsBAmYKWGYgYjGuHJLRdCPj9fvXQ3X9U
 BLAKE2_256       : 0x7ac6016ddf9179bb2d6d0284df4d60323519f20016647ba887057756d131b51e
 Wasm             : runtimes/testnet/target/srtool/release/wbuild/politest-runtime/politest_runtime.compact.wasm

== Compressed
 Version          : polimec-mainnet-2 (polimec-mainnet-0.tx1.au1)
 Metadata         : V14
 Size             : 1.20 MB (1255405 bytes)
 Compression      : 79.88%
 setCode          : 0x84730f2715acaba69361390edf28cc788e0ebdf491380d02764f78f0c702ecb9
 authorizeUpgrade : 0x8dcd2827b4c86be23da13a93a8d63a38ad4952c1450738ed8471982bcb4fc714
 IPFS             : QmPFr7QRFKM5jSuYfBNAg21dfiKEpvMxXMydLDZuW9yLFH
 BLAKE2_256       : 0x7341cc921de52eaea99af5865c3e36562cf49158dcb961daef3c2e06f531ae00
 Wasm             : runtimes/testnet/target/srtool/release/wbuild/politest-runtime/politest_runtime.compact.compressed.wasm
```
- **Utility Scripts**: Check the `scripts` directory for useful scripts. Use [just](https://github.com/casey/just) for executing scripts, e.g., `$ just build-parachain-node`.

```
Available recipes:
  benchmark-pallet-funding  # Benchmark the "Testnet" Runtime
  benchmark-runtime-funding # Benchmark the "Testnet" Runtime
  benchmarks-test
  build-all                 # Build everything
  build-base-runtime        # Build the "Base" Runtime
  build-base-srtool         # Build the "Base" Runtime using srtool
  build-parachain-node      # Build the "Parachain" Node
  build-parachain-runtime   # Build the "Testnet" Runtime
  build-parachain-srtool    # Build the "Testnet" Runtime using srtool
  create-chainspec-base     # Create the "Base" Runtime Chainspec
  default                   # Help information
  docker-build tag="latest" package="polimec-node" # Build the Node Docker Image
  run-node                  # Run the "Standalone" node in --dev mode
  test-runtime-features     # Test the runtimes features
  zombienet path_to_file="scripts/zombienet/native/base-rococo-local.toml" # Use zombienet to spawn rococo + polimec testnet
```


## Contributing

We welcome contributions! Feel free to raise issues or submit pull requests. Your feedback and contributions are valued as we develop Polimec into a robust and versatile software.

