#!/usr/bin/env bash

usage() {
	echo Usage:
	echo "$1 <srtool compressed runtime path>"
	echo "$2 <para_id>"
	echo "e.g.: ./scripts/create_bridge_hub_polkadot_spec.sh ./target/release/wbuild/base-polkadot-runtime/bridge_hub_polkadot_runtime.compact.compressed.wasm 1002"
	exit 1
}

if [ -z "$1" ]; then
	usage
fi

if [ -z "$2" ]; then
	usage
fi

set -e

rt_path=$1
para_id=$2

echo "Generating chain spec for runtime: $rt_path and para_id: $para_id"

binary="./target/release/polimec-node"

# Build the chain spec we'll manipulate
$binary build-spec --chain base-polkadot > chain-spec-plain.json

# Convert runtime to hex
od -A n -v -t x1 < "$rt_path" | tr -d ' \n' > rt-hex.txt

# TODO: This works only using jq from Git, otherwise it will generate a wrong spec using scientific notation for numbers
# replace the runtime in the spec with the given runtime and set some values to production
cat chain-spec-plain.json | jq --rawfile code rt-hex.txt '.genesis.runtime.system.code = ("0x" + $code)' |
	jq '.name = "Polimec Base"' |
	jq '.id = "polimec-polkadot"' |
	jq '.chainType = "Live"' |
	jq '.relay_chain = "polkadot"' |
	jq --argjson para_id "$para_id" '.para_id = $para_id' |
	jq --argjson para_id $para_id '.genesis.runtime.parachainInfo.parachainId = $para_id' > edited-chain-spec-plain.json
	
	# FIXME: Add bootNodes
	# jq '.bootNodes = []' |
	
	# FIXME: Check balances	
	# jq '.genesis.runtime.balances.balances = []'  > edited-chain-spec-plain.json

	# TODO: Check if we have "collatorSelection"
	# FIXME: Check invulnerables
	# jq '.genesis.runtime.collatorSelection.invulnerables = []' |
	
	# FIXME: Check Session Keys
	# jq '.genesis.runtime.session.keys = []' \

# build a raw spec
$binary build-spec --chain edited-chain-spec-plain.json --raw > chain-spec-raw.json

cp edited-chain-spec-plain.json base-polkadot-spec.json
cp chain-spec-raw.json ./chain-specs/base-polkadot.json
cp chain-spec-raw.json base-polkadot-spec-raw.json

# build genesis data
$binary export-genesis-state --chain chain-spec-raw.json > base-polkadot-genesis-head-data

# build genesis wasm
$binary export-genesis-wasm --chain chain-spec-raw.json > base-polkadot-wasm

# cleanup
rm -f rt-hex.txt
rm -f chain-spec-plain.json
rm -f chain-spec-raw.json
rm -f edited-chain-spec-plain.json