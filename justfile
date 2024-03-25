# Help information
default:
    @just --list

# Build the "Base" Runtime using srtool
build-polimec-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build" --no-wasm-std

build-rolimec-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build,fast-mode" --no-wasm-std

# Build the "Testnet" Runtime using srtool

# Test the runtimes features
test-runtime-features:
    cargo test --features runtime-benchmarks -p politest-runtime

# Run the integration tests
test-integration:
    cargo test -p integration-tests

dry-run-benchmarks:
    cargo run --features runtime-benchmarks --release -p polimec-node benchmark pallet \
        --chain=politest-local \
        --steps=2 \
        --repeat=1 \
        --pallet="*" \
        --extrinsic=* \
        --wasm-execution=compiled \
        --heap-pages=4096

# src: https://github.com/polkadot-fellows/runtimes/blob/48ccfae6141d2924f579d81e8b1877efd208693f/system-parachains/asset-hubs/asset-hub-polkadot/src/weights/cumulus_pallet_xcmp_queue.rs
# Benchmark a specific pallet on the "Polimec" Runtime
# TODO: Adjust the `--chain` flag to match the chain you are benchmarking
benchmark-runtime chain="polimec-local" pallet="pallet-elections-phragmen" features="runtime-benchmarks":
    cargo run --features {{ features }} --profile production -p polimec-node benchmark pallet \
      --chain={{ chain }} \
      --steps=50 \
      --repeat=20 \
      --pallet={{ pallet }} \
      --extrinsic=* \
      --wasm-execution=compiled \
      --heap-pages=4096 \
      --output=runtimes/polimec/src/weights/{{ replace(pallet, "-", "_") }}.rs

# src: https://github.com/paritytech/polkadot-sdk/blob/bc2e5e1fe26e2c2c8ee766ff9fe7be7e212a0c62/substrate/frame/nfts/src/weights.rs
# Run the Runtime benchmarks for a specific pallet
# TODO: Adjust the `--chain` flag to match the chain you are benchmarking
benchmark-pallet chain="polimec-local"  pallet="pallet-elections-phragmen" features="runtime-benchmarks":
    cargo run --features {{ features }} --profile production -p polimec-node benchmark pallet \
      --chain={{ chain }} \
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
docker-build tag="latest" package="polimec-node":
    ./scripts/build_image.sh {{ tag }} ./Dockerfile {{ package }}

# Create the "Base" Runtime Chainspec
create-chainspec-base:
    ./scripts/create_base_chain_spec.sh ./runtimes/base/target/srtool/release/wbuild/polimec-runtime/polimec_runtime.compact.compressed.wasm 2105

# Use zombienet to spawn rococo + polimec testnet
zombienet path_to_file="scripts/zombienet/native/base-rococo-local.toml":
    zombienet spawn {{ path_to_file }}
