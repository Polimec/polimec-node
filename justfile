# Help information
default:
  @just --list

# Build everything
build-all:
	cargo build --release

# Build the "Base" Runtime
build-base-runtime:
	cargo build --release -p polimec-base-runtime

# Build the "Testnet" Runtime
build-parachain-runtime:
	cargo build --release -p polimec-parachain-runtime

# Build the "Standalone" Runtime
build-standalone-runtime:
	cargo build --release -p polimec-standalone-runtime

# Build the "Parachain" Node
build-parachain-node:
	cargo build --release -p polimec-parachain-node

# Build the "Standalone" Node
build-standalone-node:
	cargo build --release -p polimec-standalone-node

# Build the "Base" Runtime using srtool
build-base-srtool:
	srtool build --root -p polimec-base-runtime --runtime-dir runtimes/base

# Build the "Testnet" Runtime using srtool
build-parachain-srtool:
	srtool build --root -p polimec-parachain-runtime --runtime-dir runtimes/testnet

# Test the runtimes features
test-runtime-features:
	cargo test --features runtime-benchmarks

# Benchmark the "Testnet" Runtime
benchmark-runtime-funding:
	cargo run --features runtime-benchmarks --release -p polimec-parachain-node benchmark pallet \
		--chain=polimec-rococo-local \
		--steps=50 \
		--repeat=20 \
		--pallet=pallet_funding \
		--extrinsic '*' \
		--execution=wasm \
		--wasm-execution=compiled \
		--heap-pages=4096 \
		--output=runtimes/testnet/src/weights/pallet_funding.rs

# Benchmark the "Testnet" Runtime
benchmark-pallet-funding:
	cargo run --features runtime-benchmarks,fast-gov --release -p polimec-parachain-node benchmark pallet \
		--chain=polimec-rococo-local \
		--steps=50 \
		--repeat=20 \
		--pallet=pallet_funding \
		--extrinsic '*' \
		--execution=wasm \
		--heap-pages=4096 \
		--output=pallets/funding/src/weights.rs \
		--template=./.maintain/frame-weight-template.hbs

benchmarks-test:
	cargo run --features runtime-benchmarks,fast-gov -p polimec-parachain-node benchmark pallet \
		--chain=polimec-rococo-local \
		--pallet="*" \
		--extrinsic="*"

# Build the Node Docker Image
docker-build tag = "latest" package= "polimec-parachain-node":
	./scripts/build_image.sh {{tag}} ./Dockerfile {{package}}

# Run the "Standalone" node in --dev mode
run-node:
	cargo run --release -p polimec-standalone-node -- --dev

# Create the "Base" Runtime Chainspec
create-chainspec-base:
	./scripts/create_base_chain_spec.sh ./runtimes/base/target/srtool/release/wbuild/polimec-base-runtime/polimec_base_runtime.compact.compressed.wasm 2105

# Use zombienet to spawn rococo + polimec testnet
zombienet path_to_file = "scripts/zombienet/native/base-rococo-local.toml":
	zombienet spawn {{path_to_file}}