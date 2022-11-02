# Polimec Parachain  <!-- omit in toc -->

TODO

## How to run it (Parachain Mode)

### Phase 1: Clone and build the relay chain node

`$ git clone --depth 1 --branch release-v0.9.29 https://github.com/paritytech/polkadot.git`

`$ cd polkadot`

`$ cargo build --release`

### Phase 2: Generate a chain spec

`$ ./target/release/polkadot build-spec --chain rococo-local --disable-default-bootnode --raw > rococo-local-cfde.json`

### Phase 3: Validators
Start the first validator using the `alice` account (on terminal T1)

`$ ./target/release/polkadot --chain rococo-local-cfde.json --alice --tmp`

Start the second validator using the `bob` account (on terminal T2) 

`$ ./target/release/polkadot --chain rococo-local-cfde.json --bob --tmp --port 30334`

### Phase 4: Prepare the parachain

Export genesis state

`$ ./target/release/polimec-parachain-node export-genesis-state > genesis-state`

Export genesis wasm

`$ ./target/release/polimec-parachain-node export-genesis-wasm > genesis-wasm`

### Phase 5: Register the parachain on polkadot.js

TODO

### Phase 6: Collators

Start the first collator using the `alice` account (on terminal T3)

`$ ./target/release/polimec-parachain-node --collator --alice --force-authoring --tmp --port 40335 --ws-port 9946 -- --execution wasm --chain ../polkadot/rococo-local-cfde.json --port 30335`

Start the second collator using the `bob` account (on terminal T4)

`$ ./target/release/polimec-parachain-node --collator --bob --force-authoring --tmp --port 40336 --ws-port 9947 -- --execution wasm --chain ../polkadot/rococo-local-cfde.json --port 30336`

Start a parachain full node (on terminal T5)

`$ ./target/release/polimec-parachain-node --tmp --port 40337 --ws-port 9948 -- --execution wasm --chain ../polkadot/rococo-local-cfde.json --port 30337`


## How to run it (Standalone Mode)

`$ cargo build --release`

`$ ./target/release/polimec-standalone-node --dev`

or 

`$ cargo run --release -- --dev`
