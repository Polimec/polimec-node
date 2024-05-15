# Help information
default:
    @just --list

# Build the "Base" Runtime using srtool
build-polimec-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build" --no-wasm-std

build-rolimec-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build,fast-mode" --no-wasm-std

# Build the "Testnet" Runtime using srtool
build-politest-srtool:
    srtool build --root -p politest-runtime --profile production --runtime-dir runtimes/politest --build-opts="--features=on-chain-release-build,fast-mode" --no-wasm-std

# Test the runtimes features
test-runtime-features runtime="polimec-runtime":
    cargo test --features runtime-benchmarks -p {{ runtime }}

# Run the integration tests
test-integration:
    cargo test -p integration-tests

dry-run-benchmarks mode="fast-mode" runtime="politest,polimec" pallet="*" extrinsic="*" :
    #!/bin/bash
    # Set the internal field separator for splitting the runtime variable
    IFS=','
    # Read the runtime variable into an array
    read -ra runtimes <<< "{{runtime}}"
    read -ra modes <<< "{{mode}}"

    # Build the project with each mode
    for mode in "${modes[@]}"; do \
        echo -e "\033[34mBuilding runtime with mode: \033[92m$mode\033[34m\033[0m"
        cargo build --features runtime-benchmarks,$mode --release
        # Loop over each runtime and run the benchmark
        for runtime in "${runtimes[@]}"; do \
            echo -e "\033[34mRunning benchmarks for runtime: \033[92m$runtime\033[34m\033[0m"

            ./target/release/polimec-node benchmark pallet \
                --chain=${runtime}-local \
                --steps=2 \
                --repeat=1 \
                --pallet={{ pallet }} \
                --extrinsic={{ extrinsic }} \
                --wasm-execution=compiled \
                --heap-pages=4096
        done
    done

# src: https://github.com/polkadot-fellows/runtimes/blob/48ccfae6141d2924f579d81e8b1877efd208693f/system-parachains/asset-hubs/asset-hub-polkadot/src/weights/cumulus_pallet_xcmp_queue.rs
# Benchmark a specific pallet on the "Polimec" Runtime
# Use mode="production" to generate production weights.
benchmark-runtime chain="polimec-local" pallet="pallet-elections-phragmen" mode="release":
    cargo run --features runtime-benchmarks --profile {{mode}} -p polimec-node benchmark pallet \
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
benchmark-pallet chain="politest-local"  pallet="pallet-dispenser":
    cargo run --features runtime-benchmarks --release -p polimec-node benchmark pallet \
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
