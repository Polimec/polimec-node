# Polimec Parachain <!-- omit in toc -->

> **Warning** Under HEAVY development

## How to run it (Parachain Mode using Zombienet)

### Requirements

- [Rust](https://rustup.rs/)
- [Zombienet](https://github.com/paritytech/zombienet), install guide
  [here](https://paritytech.github.io/zombienet/install.html)

### Step 1: Compile the relay chain and add it to $PATH

- Clone the [Polkadot Repository](https://github.com/paritytech/polkadot)
- Checkout the `release-v0.9.39` branch
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
$ ./target/release/polimec-standalone-node --dev
```

or

```
$ cargo run --release -- --dev
```

You can use [srtool](https://github.com/paritytech/srtool) to compile the
runtime and generate the WASM blob.

```
== Compact
 Version          : polimec-node-1 (polimec-node-0.tx1.au1)
 Metadata         : V14
 Size             : 3.04 MB (3192701 bytes)
 setCode          : 0x58fe8e8e8a2f18ada059ec4be28e9ae9e587c9d9030d131fd1490642430e210d
 authorizeUpgrade : 0x652e9ff841af94b66059ea5e824b707b143afd133059f3e668445ceef0d0adde
 IPFS             : QmV9JDMFT96ir1mWVXbTYsNRTGffzEpN7NtDBVsoCMDYhe
 BLAKE2_256       : 0x5a3305ff9dd3e1cee8686e6f2deacf8e4e44d8b09f8f37be91bdeb01b6a75d5f
 Wasm             : runtimes/testnet//target/srtool/production/wbuild/polimec-parachain-runtime/polimec_parachain_runtime.compact.wasm

== Compressed
 Version          : polimec-node-1 (polimec-node-0.tx1.au1)
 Metadata         : V14
 Size             : 761.35 KB (779620 bytes)
 Compression      : 75.59%
 setCode          : 0x552b913183ee59beca9bc1181dfeee1df8d7d0f3957d26a5dc0a0b3be51aeb22
 authorizeUpgrade : 0x17a8fb876f0762636da732a5675f8b2e8e45b2360e1706dc74e74e8efa0a43e1
 IPFS             : QmRSgzykaEhC5ADgtqVPHn1sUn1XXN2CmdDEopL4gw9LDe
 BLAKE2_256       : 0xe0b45ff9c7afaff573df61f5424f392ca1bb1e397d091424f5bf7b5b16fde16b
 Wasm             : runtimes/testnet//target/srtool/production/wbuild/polimec-parachain-runtime/polimec_parachain_runtime.compact.compressed.wasm
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
  build-standalone-node     # Build the "Standalone" Node
  build-standalone-runtime  # Build the "Standalone" Runtime
  create-chainspec-base     # Create the "Base" Runtime Chainspec
  default                   # Help information
  docker-build tag="latest" package="polimec-parachain-node" # Build the Node Docker Image
  run-node                  # Run the "Standalone" node in --dev mode
  test-runtime-features     # Test the runtimes features
  zombienet path_to_file="scripts/zombienet/native/base-rococo-local.toml" # Use zombienet to spawn rococo + polimec testnet
```
