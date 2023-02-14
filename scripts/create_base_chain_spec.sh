#!/usr/bin/env bash

usage() {
	echo Usage:
	echo "$1 <srtool compressed runtime path>"
	echo "$2 <para_id>"
	echo "e.g.: ./scripts/create_bridge_hub_polkadot_spec.sh ./target/release/wbuild/bridge-hub-polkadot-runtime/bridge_hub_polkadot_runtime.compact.compressed.wasm 1002"
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

binary="./target/release/polimec-parachain-node"

# Build the chain spec we'll manipulate
$binary build-spec --chain base-polkadot > chain-spec-plain.json

# Convert runtime to hex
od -A n -v -t x1 < "$rt_path" | tr -d ' \n' > rt-hex.txt

# replace the runtime in the spec with the given runtime and set some values to production
cat chain-spec-plain.json | jq --rawfile code rt-hex.txt '.genesis.runtime.system.code = ("0x" + $code)' |
	jq '.name = "Polimec Base"' |
	jq '.id = "polimec-polkadot"' |
	jq '.chainType = "Live"' |

	# FIXME: Add bootNodes
	# jq '.bootNodes = []' |

	jq '.relay_chain = "polkadot"' |
	jq --argjson para_id "$para_id" '.para_id = $para_id' |
	jq --argjson para_id $para_id '.genesis.runtime.parachainInfo.parachainId = $para_id' > edited-chain-spec-plain.json
	
	# FIXME: Check balances
	
	# jq '.genesis.runtime.balances.balances = []'  > edited-chain-spec-plain.json

	# TODO: Check if we have "collatorSelection"
	# FIXME: Check invulnerables
	# jq '.genesis.runtime.collatorSelection.invulnerables = []' |
	
	# FIXME: Check Session Keys
	# jq '.genesis.runtime.session.keys = []' \

# build a raw spec
$binary build-spec --chain edited-chain-spec-plain.json --raw > chain-spec-raw.json

# cp edited-chain-spec-plain.json bridge-hub-polkadot-spec.json
# cp chain-spec-raw.json ./parachains/chain-specs/bridge-hub-polkadot.json
# cp chain-spec-raw.json bridge-hub-polkadot-spec-raw.json

# build genesis data
# $binary export-genesis-state --chain chain-spec-raw.json > bridge-hub-polkadot-genesis-head-data

# build genesis wasm
# $binary export-genesis-wasm --chain chain-spec-raw.json > bridge-hub-polkadot-wasm

# cleanup
rm -f rt-hex.txt
rm -f chain-spec-plain.json
rm -f chain-spec-raw.json
rm -f edited-chain-spec-plain.json