# Polimec Parachain <!-- omit in toc -->

> **Warning** Under HEAVY development

## How to run it (Parachain Mode using Zombienet)

### Requirements

- [Rust](https://rustup.rs/)
- [Zombienet](https://github.com/paritytech/zombienet), install guide
  [here](https://paritytech.github.io/zombienet/install.html)

### Step 1: Compile the relay chain and add it to $PATH

- Clone the [Polkadot Repository](https://github.com/paritytech/polkadot)
- Checkout the `release-v1.0.0` branch
- Compile it using `cargo b -r -p polkadot`
- Add the binary to your $PATH, e.g.
  `cp target/release/polkadot ~/.local/bin/polkadot`

### Step 2: Compile Polimec and add it to $PATH

- Clone this repository
- Compile it using `cargo b -r -p polimec-parachain-node`
- Add the binary to your $PATH, e.g.
  `cp target/release/polimec-parachain-node ~/.local/bin/polimec`

### Step 3: Run the network using Zombienet

- Use the `zombienet` command to run the network, e.g.
  `zombienet spawn scripts/local_parachain.toml -p native`

## How to run it (Standalone Mode)

```
$ cargo build --release
```

```
$ cargo run --release -- --dev
```

You can use [srtool](https://github.com/paritytech/srtool) to compile the
runtime and generate the WASM blob.

```
== Compact
 Version          : polimec-mainnet-2 (polimec-mainnet-0.tx1.au1)
 Metadata         : V14
 Size             : 5.95 MB (6238415 bytes)
 setCode          : 0x0582b7c4d42bb46593ac2788d17c3d083eedfbc9d8ef3fb0c912378189d44f94
 authorizeUpgrade : 0xe8d26589c2c5257c3f52e21ba420eb0c6fd25fa5cee0878bc183ca0256dee9bc
 IPFS             : Qmbi9ymmCdJVJCLsBAmYKWGYgYjGuHJLRdCPj9fvXQ3X9U
 BLAKE2_256       : 0x7ac6016ddf9179bb2d6d0284df4d60323519f20016647ba887057756d131b51e
 Wasm             : runtimes/testnet/target/srtool/release/wbuild/polimec-parachain-runtime/polimec_parachain_runtime.compact.wasm

== Compressed
 Version          : polimec-mainnet-2 (polimec-mainnet-0.tx1.au1)
 Metadata         : V14
 Size             : 1.20 MB (1255405 bytes)
 Compression      : 79.88%
 setCode          : 0x84730f2715acaba69361390edf28cc788e0ebdf491380d02764f78f0c702ecb9
 authorizeUpgrade : 0x8dcd2827b4c86be23da13a93a8d63a38ad4952c1450738ed8471982bcb4fc714
 IPFS             : QmPFr7QRFKM5jSuYfBNAg21dfiKEpvMxXMydLDZuW9yLFH
 BLAKE2_256       : 0x7341cc921de52eaea99af5865c3e36562cf49158dcb961daef3c2e06f531ae00
 Wasm             : runtimes/testnet/target/srtool/release/wbuild/polimec-parachain-runtime/polimec_parachain_runtime.compact.compressed.wasm
```

- A collection of useful scripts are available in the `scripts` folder, there is
  also a `justfile` to launch the scripts using
  [just](https://github.com/casey/just), e.g. `$ just build-parachain-node`

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
  docker-build tag="latest" package="polimec-parachain-node" # Build the Node Docker Image
  run-node                  # Run the "Standalone" node in --dev mode
  test-runtime-features     # Test the runtimes features
  zombienet path_to_file="scripts/zombienet/native/base-rococo-local.toml" # Use zombienet to spawn rococo + polimec testnet
```
