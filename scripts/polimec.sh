#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems

set -e

echo "*** Initializing Polimec Parachain"

echo "*** Exporting Polimec's Genesis State"

../target/release/polimec-parachain-node export-genesis-state > genesis-state

sleep 1

echo "*** Exporting Polimec's Genesis WASM"

../target/release/polimec-parachain-node export-genesis-wasm > genesis-wasm

sleep 1

echo "*** Launching Polimec Instance #1 (Callator 1)"

../target/release/polimec-parachain-node \
    --collator \
    --alice \
    --force-authoring \
    --tmp \
    --port 40335 \
    --ws-port 9946 \
    -- \
    --execution wasm \
    --chain ./rococo-local-cfde.json \
    --port 30339 \
    > polimec_alice.log 2>&1 &

sleep 2

echo "*** Launching Polimec Instance #2 (Callator 2)"

../target/release/polimec-parachain-node \
    --collator \
    --bob \
    --force-authoring \
    --tmp \
    --port 40336 \
    --ws-port 9947 \
    -- \
    --execution wasm \
    --chain ./rococo-local-cfde.json \
    --port 30340 \
    > polimec_bob.log 2>&1 &

sleep 2

echo "*** Launching Polimec Instance #3 (Full Node)"

../target/release/polimec-parachain-node \
    --tmp \
    --port 40337 \
    --ws-port 9948 \
    --rpc-cors all \
    -- \
    --execution wasm \
    --chain ./rococo-local-cfde.json \
    --port 30341 \
    > polimec_fullnode.log 2>&1 &
