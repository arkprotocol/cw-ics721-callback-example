#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: set counter party contracts for osmosis and stargaze

echo "================================================================"
echo "setting counter party contracts"
for SOURCE_CHAIN in "osmosis" "stargaze"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$SOURCE_CHAIN.env"
    source $SCRIPT_DIR/$SOURCE_CHAIN.env

    if [ "$SOURCE_CHAIN" == "stargaze" ]; then
        TARGET_CHAIN="osmosis"
    else
        TARGET_CHAIN="stargaze"
    fi

    MSG="'{\"counter_party_contract\": { \"addr\": \"$(
        source $SCRIPT_DIR/$TARGET_CHAIN.env
        echo $ADDR_ARKITE_PASSPORT
    )\"}}'"
    CMD="$CLI tx wasm execute $ADDR_ARKITE_PASSPORT $MSG --from $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --chain-id $CHAIN_ID --node $CHAIN_NODE --yes"
    echo $CMD
    OUTPUT=$(eval $CMD)
    EXIT_CODE=$?
    if [ $EXIT_CODE != 0 ]; then
        exit "$EXIT_CODE"
    fi

    TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
    echo "TX_HASH: $TX_HASH"
    sleep 10
    CMD="$CLI query wasm contract-state smart $ADDR_ARKITE_PASSPORT '{\"counter_party_contract\":{}}' --chain-id $CHAIN_ID --node $CHAIN_NODE --output json"
    echo $CMD
    eval $CMD

done
