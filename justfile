build-all:
	cargo build --release

build-base-runtime:
	cargo build --release -p polimec-base-runtime

build-testnet-runtime:
	cargo build --release -p polimec-testnet-runtime

build-standalone-runtime:
	cargo build --release -p polimec-standalone-runtime

build-parachain-node:
	cargo build --release -p polimec-parachain-node

build-standalone-node:
	cargo build --release -p polimec-standalone-node

build-base-srtool:
	srtool build --root -p polimec-base-runtime --runtime-dir runtimes/base

build-testnet-srtool:
	srtool build --root -p polimec-parachain-runtime --runtime-dir runtimes/testnet

build-standalone-srtool:
	srtool build --root -p polimec-standalone-runtime --runtime-dir runtimes/standalone

test-runtime-features:
	cargo test --features runtime-benchmarks -- --nocapture

benchmark-runtime-funding:
	cargo run --features runtime-benchmarks --release -p polimec-standalone-node benchmark pallet \
		--chain=dev \
		--steps=50 \
		--repeat=20 \
		--pallet=pallet_funding \
		--extrinsic '*' \
		--execution=wasm \
		--wasm-execution=compiled \
		--heap-pages=4096 \
		--output=runtimes/testnet/src/weights/pallet_funding.rs

benchmark-pallet-funding:
	cargo run --features runtime-benchmarks --release -p polimec-standalone-node benchmark pallet \
		--chain=dev \
		--steps=50 \
		--repeat=20 \
		--pallet=pallet_funding \
		--extrinsic '*' \
		--execution=wasm \
		--heap-pages=4096 \
		--output=pallets/funding/src/weights.rs \
		--template=./.maintain/frame-weight-template.hbs

docker-build-standalone:
	./scripts/build_image.sh latest ./Containerfile polimec-standalone-node

run-node:
	cargo run --release -p polimec-standalone-node -- --dev