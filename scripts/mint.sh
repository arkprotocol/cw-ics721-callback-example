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
elif [ "$SOURCE_CHAIN" == "osmosis" ]; then
    TARGET_CHAIN="stargaze"
else
    echo "Invalid chain: $SOURCE_CHAIN"
    exit 1
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

echo "============ checking NFT"
# query source chain for NFT
# - cw721 query for nft info
MSG="'{\"all_nft_info\":{\"token_id\": \"$TOKEN_ID\"}}'"
CMD="$CLI query wasm contract-state smart $ADDR_CW721 $MSG --chain-id $CHAIN_ID --node $CHAIN_NODE --output $CLI_OUTPUT"
echo $CMD
OUTPUT=$(eval $CMD)
SOURCE_TOKEN_URI=$(echo $OUTPUT | jq -r ".data.info.token_uri")
SOURCE_OWNER=$(echo $OUTPUT | jq -r ".data.access.owner")
echo "------------------------------------------------------------"
echo "$SOURCE_CHAIN"
echo "- nft contract: $ADDR_CW721"
echo "- NFT #$TOKEN_ID, token uri: $SOURCE_TOKEN_URI, owner: $SOURCE_OWNER (ics721: $ADDR_ICS721)"
echo "------------------------------------------------------------"
