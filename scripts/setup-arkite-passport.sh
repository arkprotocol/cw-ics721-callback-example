#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: instantiate arkite passport contract, which includes instantiation of cw721, poap, and ics721

for ENV in "osmosis" "stargaze"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$ENV.env"
    source $SCRIPT_DIR/$ENV.env

    CW721_MSG_RAW="{\"name\": \"Arkite InterChain Passport collection\", \"symbol\": \"passport\"}"
    # Base64 encode msg
    CW721_MSG=$(echo "$CW721_MSG_RAW" | base64 | xargs | sed 's/ //g') # xargs concats multiple lines into one (with spaces), sed removes spaces

    POAP_MSG_RAW="{\"name\": \"Arkite POAP collection\", \"symbol\": \"poap\"}"
    POAP_MSG=$(echo "$POAP_MSG_RAW" | base64 | xargs | sed 's/ //g')

    # cw721_admin needs to be set, this way creator can be transitioned from ics721 to any wallet
    ICS721_MSG_RAW="{\"cw721_base_code_id\": $CODE_ID_CW721, \"cw721_admin\": \"$WALLET_ARKITE_PASSPORT\", \"pauser\": \"$WALLET_ARKITE_PASSPORT\"}"
    ICS721_MSG=$(echo "$ICS721_MSG_RAW" | base64 | xargs | sed 's/ //g')

    MSG="'{\"default_token_uri\": \"$DEFAULT_TOKEN_URI\", \"escrowed_token_uri\": \"$ESCROWED_TOKEN_URI\", \"transferred_token_uri\": \"$TRANSFERRED_TOKEN_URI\", \"cw721_base\": {\"code_id\": $CODE_ID_CW721, \"admin\": {\"address\": {\"addr\": \"$WALLET_ARKITE_PASSPORT\"}}, \"label\": \"Arkite InterChain Passport collection\", \"msg\": \"$CW721_MSG\"}, \"cw721_poap\": {\"code_id\": $CODE_ID_CW721, \"admin\": {\"address\": {\"addr\": \"$WALLET_ARKITE_PASSPORT\"}}, \"label\": \"Arkite InterChain Passport collection\", \"msg\": \"$POAP_MSG\"},\"ics721_base\": {\"code_id\": $CODE_ID_ICS721, \"admin\": {\"address\": {\"addr\": \"$WALLET_ARKITE_PASSPORT\"}}, \"label\": \"Arkite InterChain Passport collection\", \"msg\": \"$ICS721_MSG\"}}'"
    CMD="$CLI tx wasm instantiate $CODE_ID_ARKITE_PASSPORT "$MSG" --from $WALLET_ARKITE_PASSPORT --label 'Arkite InterChain Passport' --admin $WALLET_ARKITE_PASSPORT --gas-prices $CLI_GAS_PRICES --gas $CLI_GAS --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --yes --node $CHAIN_NODE --chain-id $CHAIN_ID --output $CLI_OUTPUT"
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

    echo "============ Updating ADDR_ARKITE_PASSPORT in config"
    ADDR_ARKITE_PASSPORT=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "wasm") | .attributes[] | select(.key =="addr_arkite_passport")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_ARKITE_PASSPORT" ]; then
        echo "Failed to get ADDR_ARKITE_PASSPORT from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_ARKITE_PASSPORT=$(echo "$ADDR_ARKITE_PASSPORT" | sed 's/"//g')
    echo "ADDR_ARKITE_PASSPORT: $ADDR_ARKITE_PASSPORT"
    CMD="sed -i \"s@export ADDR_ARKITE_PASSPORT.*@export ADDR_ARKITE_PASSPORT=\\\"$ADDR_ARKITE_PASSPORT\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

    ADDR_CW721=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "wasm") | .attributes[] | select(.key =="addr_cw721")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_CW721" ]; then
        echo "Failed to get ADDR_CW721 from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_CW721=$(echo "$ADDR_CW721" | sed 's/"//g')
    echo "ADDR_CW721: $ADDR_CW721"
    CMD="sed -i \"s@export ADDR_CW721.*@export ADDR_CW721=\\\"$ADDR_CW721\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

    ADDR_POAP=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "wasm") | .attributes[] | select(.key =="addr_poap")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_POAP" ]; then
        echo "Failed to get ADDR_POAP from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_POAP=$(echo "$ADDR_POAP" | sed 's/"//g')
    echo "ADDR_POAP: $ADDR_POAP"
    CMD="sed -i \"s@export ADDR_POAP.*@export ADDR_POAP=\\\"$ADDR_POAP\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

    ADDR_ICS721=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "wasm") | .attributes[] | select(.key =="addr_ics721")' | jq -r '.value') &>/dev/null
    if [ -z "$ADDR_ICS721" ]; then
        echo "Failed to get ADDR_ICS721 from tx $TX_HASH"
        exit 1
    fi
    # remove the extra quotes around it ""1234""
    ADDR_ICS721=$(echo "$ADDR_ICS721" | sed 's/"//g')
    echo "ADDR_ICS721: $ADDR_ICS721"
    CMD="sed -i \"s@export ADDR_ICS721.*@export ADDR_ICS721=\\\"$ADDR_ICS721\\\"@\" $SCRIPT_DIR/$ENV.env"
    echo $CMD
    eval $CMD

done

echo "================================================================"
echo "setting counter party contracts"
for SOURCE_CHAIN in "osmosis" "stargaze"; do
    echo "================================================================"
    echo "reading $SCRIPT_DIR/$SOURCE_CHAIN.env"
    source $SCRIPT_DIR/$SOURCE_CHAIN.env

    SOURCE_CHAIN=$1
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

done

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
