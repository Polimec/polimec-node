# Help information
default:
    @just --list

# Build the "Base" Runtime using srtool
build-polimec-polkadot-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build" --no-wasm-std

build-polimec-paseo-srtool:
    srtool build --root -p polimec-runtime --profile production --runtime-dir runtimes/polimec --build-opts="--features=on-chain-release-build,fast-mode" --no-wasm-std

# Test the runtimes features
test-runtime-features runtime="polimec-runtime":
    cargo test --features runtime-benchmarks -p {{ runtime }}

# Run the integration tests
test-integration:
    cargo test -p integration-tests

dry-run-benchmarks mode="fast-mode" pallet="*" extrinsic="*" :
    #!/bin/bash
    # Set the internal field separator for splitting the runtime variable
    IFS=','
    # Read the runtime variable into an array
    read -ra modes <<< "{{mode}}"

    # Build the project with each mode
    for mode in "${modes[@]}"; do \
        echo -e "\033[34mBuilding runtime with mode: \033[92m$mode\033[34m\033[0m"
        cargo build --features runtime-benchmarks,$mode --release

        echo -e "\033[34mRunning benchmarks"

        ./target/release/polimec-node benchmark pallet \
            --chain=polimec-paseo-local \
            --steps=2 \
            --repeat=1 \
            --pallet={{ pallet }} \
            --extrinsic={{ extrinsic }} \
            --wasm-execution=compiled \
            --heap-pages=4096
    done

benchmark-runtime:
    #!/bin/bash
    steps=${4:-50}
    repeat=${5:-20}

    weightsDir=./runtimes/polimec/src/weights
    chainSpec="polimec-paseo-local"
    benchmarkCommand="./target/production/polimec-node benchmark pallet"


    cargo run --features runtime-benchmarks --profile=production -p polimec-node benchmark pallet
    # Load all pallet names in an array.
    pallets=($(
      ${benchmarkCommand} --list --chain=${chainSpec}  |\
        tail -n+2 |\
        cut -d',' -f1 |\
        sort |\
        uniq
    ))

    echo "[+] Benchmarking ${#pallets[@]} pallets"

    for pallet in ${pallets[@]}
    do
      output_pallet=$(echo $pallet | tr '-' '_')
      echo $output_pallet
        ${benchmarkCommand} \
            --chain=${chainSpec} \
            --wasm-execution=compiled \
            --pallet=$pallet  \
            --extrinsic='*' \
            --steps=$steps  \
            --repeat=$repeat \
        --output=$weightsDir/$output_pallet.rs

    done

# src: https://github.com/paritytech/polkadot-sdk/blob/bc2e5e1fe26e2c2c8ee766ff9fe7be7e212a0c62/substrate/frame/nfts/src/weights.rs
# Run the Runtime benchmarks for a specific pallet
benchmark-pallet chain="polimec-paseo-local"  pallet="pallet-dispenser":
    cargo run --features runtime-benchmarks --profile=production -p polimec-node benchmark pallet \
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


benchmark-extrinsics pallet="pallet-funding" extrinsics="*" :
    cargo run --features runtime-benchmarks --profile=production -p polimec-node benchmark pallet \
      --chain=polimec-paseo-local \
      --steps=10 \
      --repeat=5 \
      --pallet={{ pallet }}  \
      --no-storage-info \
      --no-median-slopes \
      --no-min-squares \
      --extrinsic={{ extrinsics }} \
      --wasm-execution=compiled \
      --heap-pages=4096 \
      --output=benchmarked-extrinsics.rs \
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
