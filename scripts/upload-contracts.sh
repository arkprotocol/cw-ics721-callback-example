#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: Upload contracts

for ENV in "stargaze" "osmosis"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$ENV.env"
    source $SCRIPT_DIR/$ENV.env
    for FILE in $SCRIPT_DIR/*.wasm; do
        echo "================================================================"
        CONTRACT=$(basename $FILE .wasm)
        # ask whether to upload the contract, othersie skip
        read -p "Upload $CONTRACT to $ENV? (y/n): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            continue
        fi
        echo "============ Uploading $CONTRACT"
        CMD="$CLI tx wasm store $FILE --from $WALLET_ARKITE_PASSPORT --chain-id $CHAIN_ID --node $CHAIN_NODE --gas $CLI_GAS --gas-prices $CLI_GAS_PRICES --gas-adjustment $CLI_GAS_ADJUSTMENT --broadcast-mode $CLI_BROADCAST_MODE --yes"
        echo $CMD
        TX_HASH=$($CMD | jq -r ".txhash")
        ERROR_CODE=${PIPESTATUS[0]}
        if [ $ERROR_CODE -ne 0 ]; then
            echo "Failed to upload $CONTRACT to $ENV: $TX_HASH"
            exit $ERROR_CODE
        fi
        echo "============ Waiting for tx $TX_HASH to be included in a block"
        sleep 10
        # get code id from output
        QUERY_OUTPUT=$($CLI q tx $TX_HASH --chain-id $CHAIN_ID --node $CHAIN_NODE --output json)
        CODE_ID=$(echo $QUERY_OUTPUT | jq '.logs[0].events[] | select(.type == "store_code") | .attributes[] | select(.key =="code_id")' | jq -r '.value') &>/dev/null
        # if CODE_ID is empty, query in data.events
        if [ -z "$CODE_ID" ] || [ "$CODE_ID" = null ]; then
            echo "CODE_ID is empty, trying to get it from data.events" >&2
            CODE_ID=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "store_code") | .attributes[] | select(.key =="code_id")' | jq -r '.value') &>/dev/null
        fi
        if [ -z "$CODE_ID" ] || [ "$CODE_ID" = null ]; then # injective hides the code id in the events
            echo "CODE_ID is empty, trying to get it from logs.attributes" >&2
            CODE_ID=$(echo "$QUERY_OUTPUT" | jq -r '.logs[0].events[] | select(.type == "cosmwasm.wasm.v1.EventCodeStored").attributes[] | select(.key == "code_id").value') &>/dev/null
            # remove the extra quotes around it ""1234""
            CODE_ID=$(echo "$CODE_ID" | sed 's/"//g')
        fi
        if [ -z "$CODE_ID" ]; then
            echo "Failed to get code id from tx $TX_HASH"
            exit 1
        fi
        # update code id in config
        echo "============ Updating $CONTRACT code id to $CODE_ID"
        echo sed -i "s/\".*$CONTRACT/\"$CODE_ID\" # $CONTRACT/" $SCRIPT_DIR/$ENV.env
        sed -i "s/\".*$CONTRACT/\"$CODE_ID\" # $CONTRACT/" $SCRIPT_DIR/$ENV.env
    done
done
