#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems

set -e

echo "*** Initializing Relay Chain"

echo "*** Exporting RoCoCo Chainspec"

../../polkadot/target/release/polkadot build-spec \
    --chain rococo-local \
    --disable-default-bootnode \
    --raw \
    > rococo-local-cfde.json

sleep 1

echo "*** Launching RoCoCo Validator #1"

../../polkadot/target/release/polkadot \
    --alice \
    --tmp \
    --chain ./rococo-local-cfde.json \
    > polka_alice.log 2>&1 &

sleep 3

echo "*** Launching RoCoCo Validator #2"

../../polkadot/target/release/polkadot \
    --bob \
    --tmp \
    --port 30336 \
    --chain ./rococo-local-cfde.json \
    > polka_bob.log 2>&1 &

sleep 3

