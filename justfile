build-all:
	cargo build --release

build-base-runtime:
	cargo build --release -p polimec-base-runtime

build-testnet-runtime:
	cargo build --release -p polimec-testnet-runtime

build-standalone-runtime:
	cargo build --release -p polimec-standalone-runtime

build-node:
	cargo build --release -p polimec-parachain-node

build-base-srtool:
	srtool build --root -p polimec-base-runtime --runtime-dir runtimes/base

build-testnet-srtool:
	srtool build --root -p polimec-testnet-runtime --runtime-dir runtimes/testnet

build-standalone-srtool:
	srtool build --root -p polimec-standalone-runtime --runtime-dir runtimes/standalone

test-runtime-features:
	cargo test --features runtime-benchmarks -- --nocapture