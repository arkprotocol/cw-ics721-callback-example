# cw-ics721-callback-example

## About Ark Protocol

Mission:

> Building an InterChain NFT Hub. Technically this means:
>
> Transitioning NFT utlities from a local, single-chain to a global and interchain level (like transfers, staking, snapshots, launchpads, marketplace, ...).

Ark Protocol is the main contributor for `cw-ics721` and `cw-nfts`. Recent utilities we have provided are:

- [ICS 721](https://github.com/cosmos/ibc/tree/main/spec/app/ics-721-nft-transfer)
  - InterChain transfers
  - Outgoing and incoming proxies for additional security (e.g., whitelisting IBC channels)
  - Optional receive and ack callbacks
- `cw-nfts`
  - [cw721-expiration](https://github.com/CosmWasm/cw-nfts/tree/main/contracts/cw721-expiration): For issuing time-based subscriptions and services
  - Upcoming major [v0.19 release](https://github.com/CosmWasm/cw-nfts/pull/156)
    - Main logic moved to `cw721` package for better re-use
    - Distinction between `creator` and `minter`
    - NEW `CollectionInfo` in `cw721` package
    - NEW utility: `UpdateNftInfo` and `UpdateCollectionInfo` msg

- More InterChain utilities coming soon:
  - InterChain launchpad
  - InterChain marketplace
  - `cw-ics721` v2 (onchain metadata, royalties, single-hop-only transfers, etc.)

## Demo

All contracts are deployed on Stargaze and Osmosis testnet. There are deployment scripts described in  [scripts/SETUP.md](./scripts/SETUP.md).

This is a full example demoing how cw721 interacts with ics721, incoming, and outgoing proxies.

Demo will show how NFT and its metadata are affected using callbacks during InterChain (aka ics721) transfers:

1. Minting an NFT on Osmosis
2. Transferring NFT to Stargaze
3. Transferring Back to Osmosis

## Kudos

Special thanks to Ilo and the GraviDao team. PFPs have been provided by them.

### Minting an NFT on Osmosis

A minted NFT looks like this on Osmosis (source/home chain):

![minted nft](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis01_home.png?raw=true)

### Transferring NFT to Stargaze

After NFT is transferred over to Stargaze, NFT on Osmosis is escrowed, and its metadata data is updated:

![transfer, home chain](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis02_away.png?raw=true)
Note: PFP has changed from `home` to `escrowed` PFP!

On the target/sub chain, an NFT is minted by ICS721 on Stargaze:

![transfer, sub chain](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis03_transferred.png?raw=true)
Note: PFP has changed from `home` to `transferred` PFP!

Please also note, that ics721 transfers NFT data `as-is` (as provided by the collection on the home chain). PFPs on both chains are updated, using callbacks!

### Transferring Back to Osmosis

Transferring an NFT back to home chain, leads to NFT being burned on subchain/Stargaze, and callback resetting PFP on home chain:

![backtransfer](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis01_home.png?raw=true)
Note: PFP has resetted back `home` PFP!

### Scripts for Testing

All scripts are availble in [./scripts](./scripts/):

```sh
# Mint an NFT
./scripts/mint.sh osmosis # output returns NFT id, e.g. "1"

# Transfer and relay NFT #1:
./scripts/transfer.sh osmosis 1 # output returns collection address on target chain, e.g. "stars1qfclvue79dnhmm7n5d3zce6ftza866zxl57se2s47mrfkf3cetgsew2fmj"

# Back transfer of specific collection `stars1qfclvue79dnhmm7n5d3zce6ftza866zxl57se2s47mrfkf3cetgsew2fmj`
./scripts/transfer.sh stargaze 1 stars1qfclvue79dnhmm7n5d3zce6ftza866zxl57se2s47mrfkf3cetgsew2fmj

```

Some explanation here regarding NFT ownership:

- check ownership
  - UI: `https://testnet.arkprotocol.io/collections/CW721_ADDRESS/NFT_ID`
  - CLI: `source ./scripts/osmosis.env;$CLI query wasm contract-state smart $ADDR_CW721 '{"all_nft_info":{"token_id": "35"}}' --chain-id $CHAIN_ID --node $CHAIN_NODE | jq`
- forward transfer:
  - NFT on source chain is escrowed/owned by `$ADDR_ICS721`
  - NFT on target chain is owned by `$WALLET_ARKITE_PASSPORT`

Next, we can test transferring an NFT via a channel that is not whitelisted. For this open [./scripts/transfer.sh](./scripts/transfer.sh) and replace `$CHANNEL_ID` with `$CHANNEL_ID_NOT_WHITELISTED`. Now on transfer an `ack fail` (with `code 5: execution error`) is returned. NFT on source chain is returned back to `$WALLET_ARKITE_PASSPORT`.

### cw-ics721 Specifics

Ark Protocol has implemented enhanced proxies. All contracts have been audited by SCV. We plan to open source our contracts in the near future - after a certain grace period on mainnet chains.
Ark's proxies provides addtional securities, like: whitelisting channels, collections, and code hashes.

This way `cw-ics721` secures transferring via:

- safe WLed channels and excluding unknown, unsecure channels
- avoid fragmentation (e.g. only one channel to each target chain)
- safe cw721 contracts - by WLing address and code hashes
- checks on both sides: outbound and inbound NFTs

For the rare case a chain or contracts gets compromised, Ark proxies will de-WL channels, collections either on outbound or inbound or both sides - on all required chains.
As an additional measure, `cw-ics721` also allows defining a one-time `pauser` address, allowing to halt `cw-ics721` until it is explicitly unpaused again.

### Contracts

Main contract is [./contracts/cw-ics721-arkite-passport](./contracts/cw-ics721-arkite-passport/). It acts as:

- a minter
- forwards incoming NFTs to ICS721 and attaching callbacks
- receive and ack callback handler
- also holds references to:
  - passport and poap collection (both cw721-base)

Callback does:

- receive callback on `arkite-passport` contract:
  - mutates the NFT's metadata (token uri) on target chain
  - mints a POAP NFT
- ack callback on `arkite-passport` contract:
  - mutates NFT on source chain

The workflow for transferring an NFT from Stargaze to Osmosis is:

- user calls `send_nft` to `arkite-passport` contract
- `arkite-passport` transfers NFT to target chain
  - attaches receive and ack callback as part of memo
  - forwards `send_nft` to outgoing proxy
  - outgoing proxy forwards `send_nft` to ics721
  - ics721 sends ibc message / packet to target chain
- relayer picks packet and triggers call on target chain
- ics721 processing
  - transfer nft
    - initially instantiates collection
    - mints nft and transfers to target recipient
  - forwards receive callback to 

## Resources

- cw-ics721 repo: https://github.com/public-awesome/cw-ics721
- ics721-plus repo (private): https://github.com/arkprotocol/ics721-plus
- cw-ics721-proxy repo: https://github.com/arkprotocol/cw-ics721-proxy