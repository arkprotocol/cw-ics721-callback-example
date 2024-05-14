# cw-ics721-callback-example

## About Ark Protocol

Our mission:

> Building an InterChain NFT Hub. Technically this means:
> Transitioning NFT utlities from a local, single-chain to a global and interchain level (like transfers, staking, snapshots, launchpads, marketplace, ...).

Ark Protocol is the main contributor for `cw-ics721` and `cw-nfts`. Recent utilities we have provided:

- ics721
  - interchain transfers (obviously)
  - outgoing and incoming proxies for additional security
  - callbacks
- cw-nfts
  - `cw721-expiration`: NFTs expires after a given duration (e.g. 1 year) - useful for subscriptions and services
  - upcoming major [v0.19 release](https://github.com/CosmWasm/cw-nfts/pull/156)
    - main logic moved to `cw721` package for re-use
    - distinction between `creator` and `minter`
    - NEW `CollectionMedata` in `cw721` package
    - NEW utility: `UpdateNftInfo` and `UpdateCollectionMetadata` msg

- more interchain utilities coming soon:
  - interchain launchpad and marketplace,
  - ics721 v2 (onchain metadata, royalties, single-hop-only transfers, ...)
  - widgets and APIs...

## Demo

This is a full example demoing how cw721 interacts with ics721, incoming, and outgoing proxies.

Demo will show how NFT and its metadata are affected using callbacks:

1. Minting an NFT on Osmosis
2. Transferring NFT to Stargaze
3. Transferring Back to Osmosis

All contracts are deployed on Stargaze and Osmosis testnet. There are deployment scripts described in  [scripts/SETUP.md](./scripts/SETUP.md).

### Minting an NFT on Osmosis

A minted NFT looks like this on Osmosis (source/home chain):

![minted nft](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis01_home.png?raw=true)

### Transferring NFT to Stargaze

After NFT is transferred over to Stargaze, NFT on Osmosis is escrowed, and its metadata data are changed:

![transfer, home chain](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis02_away.png?raw=true)

On the other, an NFT is minted by ICS721 on Stargaze (target/sub chain):

![transfer, sub chain](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis03_transferred.png?raw=true)

Please note, that ics721 transfers PFP as provided by the collection on the home chain. PFPs on both chains are updated, using callbacks!

### Transferring Back to Osmosis

Transferring an NFT back to home chain, leads to NFT being burned on subchain/Stargaze, and callback resetting PFP on home chain:

![backtransfer](https://github.com/arkprotocol/cw-ics721-callback-example/blob/main/public/passport_osmosis01_home.png?raw=true)

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
- 