#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: instantiate proxies and migrate ics721 with proxies

for ENV in "osmosis" "stargaze"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$ENV.env"
    source $SCRIPT_DIR/$ENV.env

    echo "============ instantiating incoming proxy"
    MSG="'{\"origin\": \"$ADDR_ICS721\", \"channels\": [\"$CHANNEL_ID\"]}'"
    CMD="$CLI tx wasm instantiate $CODE_ID_INCOMING_PROXY "$MSG" --from $WALLET_ARKITE_PASSPORT --label 'ICS721 Incoming Proxy (powered by Ark)' --admin $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
    echo "executing cmd: $CMD" >&2
    OUTPUT=$(eval $CMD)
    EXIT_CODE=$?
    if [ $EXIT_CODE != 0 ]; then
        exit "$EXIT_CODE"
    fi

    TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
    echo "TX_HASH: $TX_HASH"
    sleep 10
    QUERY_OUTPUT=$($CLI q tx $TX_HASH --chain-id $CHAIN_ID --node $CHAIN_NODE --output json)

    echo "============ Updating ADDR_INCOMING_PROXY in config"
    ADDR_INCOMING_PROXY=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "instantiate") | .attributes[] | select(.key =="_contract_address")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_INCOMING_PROXY" ]; then
        echo "Failed to get ADDR_INCOMING_PROXY from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_INCOMING_PROXY=$(echo "$ADDR_INCOMING_PROXY" | sed 's/"//g')
    echo "ADDR_INCOMING_PROXY: $ADDR_INCOMING_PROXY"
    CMD="sed -i \"s@export ADDR_INCOMING_PROXY.*@export ADDR_INCOMING_PROXY=\\\"$ADDR_INCOMING_PROXY\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

    echo "============ instantiating outgoing proxy"
    MSG="'{\"origin\": \"$ADDR_ICS721\", \"rate_limit\": {\"per_block\": 1000}}'"
    CMD="$CLI tx wasm instantiate $CODE_ID_OUTGOING_PROXY "$MSG" --from $WALLET_ARKITE_PASSPORT --label 'ICS721 Outgoing Proxy (powered by Ark)' --admin $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
    echo "executing cmd: $CMD" >&2
    OUTPUT=$(eval $CMD)
    EXIT_CODE=$?
    if [ $EXIT_CODE != 0 ]; then
        exit "$EXIT_CODE"
    fi

    TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
    echo "TX_HASH: $TX_HASH"
    sleep 10
    QUERY_OUTPUT=$($CLI q tx $TX_HASH --chain-id $CHAIN_ID --node $CHAIN_NODE --output json)

    echo "============ Updating ADDR_OUTGOING_PROXY in config"
    ADDR_OUTGOING_PROXY=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "instantiate") | .attributes[] | select(.key =="_contract_address")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_OUTGOING_PROXY" ]; then
        echo "Failed to get ADDR_OUTGOING_PROXY from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_OUTGOING_PROXY=$(echo "$ADDR_OUTGOING_PROXY" | sed 's/"//g')
    echo "ADDR_OUTGOING_PROXY: $ADDR_OUTGOING_PROXY"
    CMD="sed -i \"s@export ADDR_OUTGOING_PROXY.*@export ADDR_OUTGOING_PROXY=\\\"$ADDR_OUTGOING_PROXY\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

    echo "============ migrating ics721 with proxies"
    MSG="'{\"with_update\":{
\"incoming_proxy\": \"$ADDR_INCOMING_PROXY\",
\"outgoing_proxy\": \"$ADDR_OUTGOING_PROXY\",
\"cw721_base_code_id\": $CODE_ID_CW721,
\"pauser\": \"$WALLET_ARKITE_PASSPORT\",
\"cw721_admin\": \"$WALLET_ARKITE_PASSPORT\"
}}'"
    CMD="$CLI tx wasm migrate $ADDR_ICS721 $CODE_ID_ICS721 "$MSG" --from $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
    echo "executing cmd: $CMD" >&2
    OUTPUT=$(eval $CMD)
    EXIT_CODE=$?
    if [ $EXIT_CODE != 0 ]; then
        exit "$EXIT_CODE"
    fi

    TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
    echo "TX_HASH: $TX_HASH"

done
