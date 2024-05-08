#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: Upload contracts

# check user provided chain
if [ -z "$1" ]; then
    echo "Usage: $0 [stargaze|osmosis]"
    exit 1
fi
SOURCE_CHAIN=$1
echo "reading $SCRIPT_DIR/$SOURCE_CHAIN.env"
source $SCRIPT_DIR/$SOURCE_CHAIN.env

# set target chain
if [ "$SOURCE_CHAIN" == "stargaze" ]; then
    TARGET_CHAIN="osmosis"
else
    TARGET_CHAIN="stargaze"
fi

echo "============ Minting NFT"
MSG="'{\"mint\": {}}'"
CMD="$CLI tx wasm execute $ADDR_ARKITE_PASSPORT "$MSG" --from $WALLET_ARKITE_PASSPORT --gas $CLI_GAS --gas-prices $CLI_GAS_PRICES --gas-adjustment $CLI_GAS_ADJUSTMENT -b $CLI_BROADCAST_MODE --output $CLI_OUTPUT --yes --node $CHAIN_NODE --chain-id $CHAIN_ID"
echo $CMD
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

TOKEN_ID=$(echo $QUERY_OUTPUT | jq '.events[] | select(.type == "wasm") | .attributes[] | select(.key =="token_id")' | jq -r '.value') &>/dev/null
if [ -z "$TOKEN_ID" ]; then
    echo "Failed to get TOKEN_ID from tx $TX_HASH"
    exit 1
fi
echo "Minted NFT #$TOKEN_ID" >&2

echo "============ Transferring NFT"
# create ICS721 message with:
# - recipient of NFT on target chain (Osmosis)
# - source channel: Stargaze channel referencing ICS721 contracts/ports on both chains
# - timeout: expiration in case relayer doesnt relay on time
RECIPIENT=$(
    source $SCRIPT_DIR/$TARGET_CHAIN.env
    echo $WALLET_ARKITE_PASSPORT
)
TIMESTAMP=$(date -d "+60 minutes" +%s%N) # time in nano seconds, other options: "+1 day"
printf -v RAW_MSG '{
"receiver": "%s",
"channel_id": "%s",
"timeout": { "timestamp": "%s" } }' \
    "$RECIPIENT" \
    "$CHANNEL_ID" \
    "$TIMESTAMP"

# Base64 encode msg
MSG=$(echo "$RAW_MSG" | base64 | xargs | sed 's/ //g') # xargs concats multiple lines into one (with spaces), sed removes spaces

# send nft msg for $TOKEN_ID
printf -v EXECUTE_MSG '{"send_nft": {
"contract": "%s",
"token_id": "%s",
"msg": "%s"}}' \
    "$ADDR_ARKITE_PASSPORT" \
    "$TOKEN_ID" \
    "$MSG"

# execute transfer
CMD="$CLI tx wasm execute '$ADDR_CW721' '$EXECUTE_MSG' \
--from "$WALLET_ARKITE_PASSPORT" \
--gas-prices "$CLI_GAS_PRICES" \
--gas "$CLI_GAS" \
--gas-adjustment "$CLI_GAS_ADJUSTMENT" \
-b "$CLI_BROADCAST_MODE" \
--chain-id $CHAIN_ID --node $CHAIN_NODE \
--yes"
echo $CMD
echo "executing cmd: $CMD" >&2
OUTPUT=$(eval $CMD)
EXIT_CODE=$?
if [ $EXIT_CODE != 0 ]; then
    exit "$EXIT_CODE"
fi

TX_HASH=$(echo $OUTPUT | jq -r ".txhash")
echo "TX_HASH: $TX_HASH"
sleep 10

echo "============ relaying packets"
CMD="hermes --config ./relayer/hermes/config.toml clear packets --chain $CHAIN_ID --channel $CHANNEL_ID --port wasm.$ADDR_ICS721"
eval $CMD

echo "============ checking NFTs"
MSG="'{\"all_nft_info\":{\"token_id\": \"4\"}}'"
CMD="$CLI query wasm contract-state smart $ADDR_CW721 $MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
OUTPUT=$(eval $CMD)
SOURCE_TOKEN_URI=$(echo $OUTPUT | jq -r ".data.info.token_uri")
SOURCE_OWNER=$(echo $OUTPUT | jq -r ".data.access.owner")
echo "$SOURCE_CHAIN: NFT #$TOKEN_ID, token uri: $SOURCE_TOKEN_URI, owner: $SOURCE_OWNER (ics721: $ADDR_ICS721)"

source $SCRIPT_DIR/$TARGET_CHAIN.env
CMD="$CLI query wasm contract-state smart $ADDR_CW721 $MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
OUTPUT=$(eval $CMD)
TARGET_TOKEN_URI=$(echo $OUTPUT | jq -r ".data.info.token_uri")
TARGET_OWNER=$(echo $OUTPUT | jq -r ".data.access.owner")
echo "$TARGET_CHAIN: NFT #$TOKEN_ID, token uri: $TARGET_TOKEN_URI, owner: $TARGET_OWNER (ics721: $ADDR_ICS721)"
