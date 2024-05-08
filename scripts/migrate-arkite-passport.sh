#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: instantiate proxies and migrate ics721 with proxies

for ENV in "osmosis" "stargaze"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$ENV.env"
    source $SCRIPT_DIR/$ENV.env

    echo "============ migrating ics721 with proxies"
    MSG="'{\"with_update\":{}}'"
    CMD="$CLI tx wasm migrate $ADDR_ARKITE_PASSPORT $CODE_ID_ARKITE_PASSPORT "$MSG" --from $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
    echo "executing cmd: $CMD" >&2
    OUTPUT=$(eval $CMD)
    EXIT_CODE=$?
    if [ $EXIT_CODE != 0 ]; then
        exit "$EXIT_CODE"
    fi

    TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
    echo "TX_HASH: $TX_HASH"

done
