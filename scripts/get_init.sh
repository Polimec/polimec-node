#!/bin/bash

../target/release/polimec-parachain-node build-spec --chain base-rococo-local --raw --disable-default-bootnode > chain-spec.json

../target/release/polimec-parachain-node export-genesis-wasm --chain chain-spec.json > base-wasm

../target/release/polimec-parachain-node export-genesis-state --chain chain-spec.json > base-state