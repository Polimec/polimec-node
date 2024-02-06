# Help information
default:
    @just --list

# Build the "Base" Runtime using srtool
build-base-srtool:
    srtool build --root -p polimec-base-runtime --runtime-dir runtimes/base --build-opts="--features=on-chain-release-build"

# Build the "Testnet" Runtime using srtool

# Test the runtimes features
test-runtime-features:
    cargo test --features runtime-benchmarks -p polimec-parachain-runtime

# Run the integration tests
test-integration:
    cargo test -p integration-tests

# Run the Runtime benchmarks
# src: https://github.com/polkadot-fellows/runtimes/blob/48ccfae6141d2924f579d81e8b1877efd208693f/system-parachains/asset-hubs/asset-hub-polkadot/src/weights/cumulus_pallet_xcmp_queue.rs
benchmark-runtime pallet="pallet-linear-release" features="runtime-benchmarks":
    cargo run --features {{ features }} --release -p polimec-parachain-node benchmark pallet \
      --chain=polimec-rococo-local \
      --steps=50 \
      --repeat=20 \
      --pallet={{ pallet }} \
      --extrinsic=* \
      --wasm-execution=compiled \
      --heap-pages=4096 \
      --output=runtimes/base/src/weights/{{ replace(pallet, "-", "_") }}.rs

# Benchmark the "Testnet" Runtime
# src: https://github.com/paritytech/polkadot-sdk/blob/bc2e5e1fe26e2c2c8ee766ff9fe7be7e212a0c62/substrate/frame/nfts/src/weights.rs
benchmark-pallet pallet="pallet-linear-release" features="runtime-benchmarks":
    cargo run --features {{ features }} --release -p polimec-parachain-node benchmark pallet \
      --chain=polimec-rococo-local \
      --steps=50 \
      --repeat=20 \
      --pallet={{ pallet }}  \
      --no-storage-info \
      --no-median-slopes \
      --no-min-squares \
      --extrinsic '*' \
      --wasm-execution=compiled \
      --heap-pages=4096 \
      --output=pallets/{{ replace(pallet, "pallet-", "") }}/src/weights.rs \
      --template=./.maintain/frame-weight-template.hbs

# Build the Node Docker Image
docker-build tag="latest" package="polimec-parachain-node":
    ./scripts/build_image.sh {{ tag }} ./Dockerfile {{ package }}

# Create the "Base" Runtime Chainspec
create-chainspec-base:
    ./scripts/create_base_chain_spec.sh ./runtimes/base/target/srtool/release/wbuild/polimec-base-runtime/polimec_base_runtime.compact.compressed.wasm 2105

# Use zombienet to spawn rococo + polimec testnet
zombienet path_to_file="scripts/zombienet/native/base-rococo-local.toml":
    zombienet spawn {{ path_to_file }}
