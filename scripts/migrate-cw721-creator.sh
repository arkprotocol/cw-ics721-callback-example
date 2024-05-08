#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: migrates cw721 contract with ADDR_ARKITE_PASSPORT as new creator

# check user provided chain and token_id
if [[ -z "$1" || -z "$2" ]]; then
    echo "Usage:"
    echo "$0 [stargaze|osmosis] [ADDR_CW721]"
    exit 1
fi

# set target chain
SOURCE_CHAIN=$1
if [ "$SOURCE_CHAIN" == "stargaze" ]; then
    TARGET_CHAIN="osmosis"
elif [ "$SOURCE_CHAIN" == "osmosis" ]; then
    TARGET_CHAIN="stargaze"
else
    echo "Invalid chain: $SOURCE_CHAIN"
    exit 1
fi

echo "reading $SCRIPT_DIR/$SOURCE_CHAIN.env"
source $SCRIPT_DIR/$SOURCE_CHAIN.env

ADDR_CW721=$2

echo "============ migrating $ADDR_CW721 with creator $ADDR_ARKITE_PASSPORT"
MSG="'{\"with_update\":{\"creator\": \"$ADDR_ARKITE_PASSPORT\"}}'"
CMD="$CLI tx wasm migrate $ADDR_CW721 $CODE_ID_CW721 "$MSG" --from $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
echo "executing cmd: $CMD" >&2
OUTPUT=$(eval $CMD)
EXIT_CODE=$?
if [ $EXIT_CODE != 0 ]; then
    exit "$EXIT_CODE"
fi

TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
echo "TX_HASH: $TX_HASH"
sleep 10
$CLI query wasm contract-state smart $ADDR_CW721 '{"get_creator_ownership":{}}' --chain-id $CHAIN_ID --node $CHAIN_NODE --output json | jq
