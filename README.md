# cw-ics721-callback-example

## About Ark Protocol

Ark's (technical) goal:
> Transitioning local/single-chain NFT utlities to a global/interchain level (like transfers, staking, snapshots, launchpads, marketplace, ...).

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

- more interchain utilities come soon: like interchain launchpad and marketplace, widgets and APIs...

## Intro

This is a full example demoes how cw721 interacts with ics721 and its outgoing proxies.
For the interchain transfer optional receive and ack callbacks are attached:

- receive callback mutates the NFT's metadata (token uri) on target chain
- ack callback mutates NFT on source chain AND creates a POAP NFT

The following contracts are setup:

- `arkite-passport` contract as an entry point for handling transfers and callbacks
- a `Passport` collection using `cw721-base`
- a `POAP` collection using `cw721-base`
- ics721 contracts: `ics721-base`, `cw-ics721-outgoing-proxy-base`, and `cw-ics721-incoming-proxy-base`

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