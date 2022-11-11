.PHONY: build-polimec-release
build-polimec-release:
	cargo build --release --locked --workspace -p polimec-parachain-node

.PHONY: build-all
build-all:
	cargo build --release
