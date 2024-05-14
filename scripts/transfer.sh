#!/bin/bash
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
# Description: Upload contracts

# check user provided chain and token_id
if [[ -z "$1" || -z "$2" ]]; then
    echo "Usage:"
    echo "$0 [stargaze|osmosis] [token_id]"
    echo "$0 [stargaze|osmosis] [token_id] [ADDR_CW721]"
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

TOKEN_ID=$2

if [[ -z "$3" ]]; then
    echo "using default cw721: $ADDR_CW721"
    SOURCE_NFT_CONTRACT=$ADDR_CW721
else
    SOURCE_NFT_CONTRACT=$3
fi

echo "============ Transferring NFT"
# create ICS721 message with:
# - recipient of NFT on target chain (Osmosis)
# - source channel: Stargaze channel referencing ICS721 contracts/ports on both chains
# - timeout: expiration in case relayer doesnt relay on time
RECIPIENT=$(
    source $SCRIPT_DIR/$TARGET_CHAIN.env
    echo $WALLET_ARKITE_PASSPORT
)
TIMESTAMP=$(date -d "+5 minutes" +%s%N) # time in nano seconds, other options: "+1 day"
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
CMD="$CLI tx wasm execute '$SOURCE_NFT_CONTRACT' '$EXECUTE_MSG' \
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
sleep 20

echo "============ relaying packets"
CMD="hermes --config ./relayer/hermes/config.toml clear packets --chain $CHAIN_ID --channel $CHANNEL_ID --port wasm.$ADDR_ICS721"
echo $CMD
eval $CMD

echo "============ checking NFTs"
# query source chain for NFT
# - cw721 query for nft info
MSG="'{\"all_nft_info\":{\"token_id\": \"$TOKEN_ID\"}}'"
CMD="$CLI query wasm contract-state smart $SOURCE_NFT_CONTRACT $MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
echo $CMD
sleep 20
OUTPUT=$(eval $CMD)
ERROR_CODE=${PIPESTATUS[0]}
echo "------------------------------------------------------------"
echo "$SOURCE_CHAIN"
if [ $ERROR_CODE -ne 0 ]; then
    echo "Failed to upload $CONTRACT to $ENV: $TX_HASH"
    echo "- NFT #$TOKEN_ID got burned"
    BURNED=true
else
    SOURCE_TOKEN_URI=$(echo $OUTPUT | jq -r ".data.info.extension.image")
    SOURCE_OWNER=$(echo $OUTPUT | jq -r ".data.access.owner")
    echo "- nft contract: $SOURCE_NFT_CONTRACT"
    echo "- NFT #$TOKEN_ID, token uri: $SOURCE_TOKEN_URI, owner: $SOURCE_OWNER (ics721: $ADDR_ICS721)"
fi
echo "------------------------------------------------------------"

# query target chain for NFT
source $SCRIPT_DIR/$TARGET_CHAIN.env
# - check if burned
if [ "$BURNED" = true ]; then
    TARGET_NFT_CONTRACT=$ADDR_CW721
else
    # - ics721 query for nft contract
    QUERY_MSG="'{\"nft_contract\":{\"class_id\":\"wasm.$ADDR_ICS721/$CHANNEL_ID/$SOURCE_NFT_CONTRACT\"}}'"
    QUERY_CMD="$CLI query wasm contract-state smart $ADDR_ICS721 $QUERY_MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
    QUERY_OUTPUT=$(eval $QUERY_CMD)
    TARGET_NFT_CONTRACT=$(echo $QUERY_OUTPUT | jq -r ".data")
fi
# - cw721 query for nft info
CMD="$CLI query wasm contract-state smart $TARGET_NFT_CONTRACT $MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
echo $CMD
OUTPUT=$(eval $CMD)
TARGET_TOKEN_URI=$(echo $OUTPUT | jq -r ".data.info.extension.image")
TARGET_OWNER=$(echo $OUTPUT | jq -r ".data.access.owner")
echo "------------------------------------------------------------"
echo "$TARGET_CHAIN"
echo "- nft contract: $TARGET_NFT_CONTRACT"
echo "- NFT #$TOKEN_ID, token uri: $TARGET_TOKEN_URI, owner: $TARGET_OWNER (ics721: $ADDR_ICS721)"
echo "------------------------------------------------------------"
