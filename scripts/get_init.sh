#!/bin/bash

../target/release/polimec-node build-spec --chain base-rococo-local --raw --disable-default-bootnode > chain-spec.json

../target/release/polimec-node export-genesis-wasm --chain chain-spec.json > base-wasm

../target/release/polimec-node export-genesis-state --chain chain-spec.json > base-state