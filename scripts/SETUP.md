# Setup

For the Arkite Passport example these smart contracts have been deployed on Stargaze and Osmosis testnet:

- [cw_ics721_arkite_passport.wasm v0.1.1](https://github.com/arkprotocol/cw-ics721-callback-example/releases/tag/v0.1.1): checksum 532d590ba580dd7881b77551ea864818dc309c52e3002b8f5a13e11057fe4d6e
- [ics721_base.wasm](https://github.com/arkprotocol/ark-cw-ics721/tree/instantiate_with_creator): checksum 47c9da625237df793ed5cdbe91a284d03515421abb48612621554db53567e419
- [cw721_base.wasm](https://github.com/arkprotocol/cw-nfts/tree/collection-info): checksum 312f82835cbbce2cd2b51354d48616c18870845c1fd19f7132c8b373cef23eb6
- [cw_ics721_incoming_proxy_base.wasm](https://github.com/arkprotocol/cw-ics721-proxy/releases/tag/v0.1.1): checksum 32dd48b27688b4f7783bccd506006b6c0754db79deccca92a9db13d8e4aa7355
- [cw_ics721_outgoing_proxy_rate_limit.wasm](https://github.com/arkprotocol/cw-ics721-proxy/releases/tag/v0.1.1): checksum aae9d990fa4bd4b2d25692c0d6496b52865c64fdb2df3911deb093060871a72f

For ease-of-use binaries are provided in [scripts](./scripts) folder. Repos can be found here:

- [cw-ics721-arkite-passport](https://github.com/arkprotocol/cw-ics721-callback-example)
- [cw-ics721](https://github.com/public-awesome/cw-ics721)
- [cw-ics721-proxy](https://github.com/arkprotocol/cw-ics721-proxy)

A test wallet is needed. Restore wallet for Osmosis and Stargaze using this [passport.mnemonic](./passport.mnemonic).

## Setup Scripts

All scripts provided here uses [stargaze.env](./stargaze.env) and [osmosis.env](./osmosis.env).

Smart contracts have been deployed on Osmosis and Stargaze testnet. All code ids can be found in above env files.

For testing, there is no need to re-deploy contracts, except:

- new smart contract versions are needed for testing
- IBC channels have been expired

### Upload Contracts

```sh
# uploads all wasm binaries provided in scripts folders
# NOTE: script also updates code ids in env files!
./scripts/upload-contracts.sh
```

### Arkite Passport contract

The `arkite-passport` contract must be instantiated first:

```sh
./scripts/setup-contracts.sh
```

IMPORTANT: Manually update port in `chains.packet_filter` in `config.toml`!

This contract also instantiates 3 additional contracts. These env variables are then updated:

- `ADDR_ARKITE_PASSPORT`: address for `arkite-passport`, contract also acts as minter and creator for below collection contracts.
- `ADDR_CW721`: `Passport` collection used for interchain transfers.
- `ADDR_POAP`: `POAP` collection ([Proof of Attendance Protocol](https://poap.xyz)).
- `ADDR_ICS721`: `ics721` contract used by `arkite-passport`

For each `Passport NFT` a user successfully transfers, it also receives an `InterChain POAP NFT` on target chain.

NOTE: for receive callbacks, counter party contracts will be set later below!

### Relayer

Setup relayer:

- install [Hermes](https://hermes.informal.systems/)
- create relayer wallet keys `osmosis_ark_relayer` and `stargaze_ark_relayer` (as defined in [relayer/hermes/config.toml](../relayer/hermes/config.toml))

```sh
# restore keys, using same mnemonic from passport
source ./scripts/osmosis.env
hermes --config ./relayer/hermes/config.toml keys add --key-name osmosis_ark_relayer --chain $CHAIN_ID --mnemonic-file ./scripts/passport.mnemonic
rly --home ./relayer/cosmos keys restore osmosistestnet default "$(cat ./scripts/passport.mnemonic)"
source ./scripts/stargaze.env
hermes --config ./relayer/hermes/config.toml keys add --key-name stargaze_ark_relayer --chain $CHAIN_ID --mnemonic-file ./scripts/passport.mnemonic
rly --home ./relayer/cosmos keys restore stargazetestnet default "$(cat ./scripts/passport.mnemonic)"

# start cosmos relayer
rly --home ./relayer/cosmos start --time-threshold 4h # auto update client every 4 hours

# create IBC client, connection, and channel for ics721
# NOTE: there is no script for this rn, so channel-id must be manually updated in env, config.toml and config.yaml files
hermes --config ./relayer/hermes/config.toml --json create channel --a-chain $(source ./scripts/osmosis.env;echo $CHAIN_ID) --a-port wasm.$(source ./scripts/osmosis.env;echo $ADDR_ICS721) --b-port wasm.$(source ./scripts/stargaze.env;echo $ADDR_ICS721) --b-chain $(source ./scripts/stargaze.env;echo $CHAIN_ID) --new-client-connection --channel-version ics721-1 --yes
# alternative create channel with existing connection (NOTE: client MUST be active):
hermes --config ./relayer/hermes/config.toml --json create channel --a-chain $(source ./scripts/osmosis.env;echo $CHAIN_ID) --a-port wasm.$(source ./scripts/osmosis.env;echo $ADDR_ICS721) --b-port wasm.$(source ./scripts/stargaze.env;echo $ADDR_ICS721) --a-connection $(source ./scripts/osmosis.env;echo $CONNECTION_ID) --channel-version ics721-1 --yes

# Hermes final output looks like this:
# {
#     "result": {
#         "a_side": {
#             "channel_id": "channel-7832",
#             "client_id": "07-tendermint-3491",
#             "connection_id": "connection-3058",
#             "port_id": "wasm.osmo1wcrz58307g3yczh0swjkgcqhg0zuksheq6j5j5497qhswj02ul3q98kqc4",
#             "version": "ics721-1"
#         },
#         "b_side": {
#             "channel_id": "channel-919",
#             "client_id": "07-tendermint-895",
#             "connection_id": "connection-867",
#             "port_id": "wasm.stars1hlpk0cjfyep3ffsrrgkjnk9u7jqvwag8nprf8nvl5jem3etwkklq9kselc",
#             "version": "ics721-1"
#         },
#         "connection_delay": {
#             "nanos": 0,
#             "secs": 0
#         },
#         "ordering": "Unordered"
#     },
#     "status": "success"
# }
#
# Manually update CHANNEL_ID in config.toml, config.yaml, stargaze.env, and osmosis.env based on above output!

```

IMPORTANT: Manually update:

- CHANNEL_ID in stargaze.env and osmosis.env based on output Hermes results!
- config.toml (Hermes) and config.yaml (Cosmos rly)!

### Proxy Contracts and Migrate ICS721

`cw-ics721` allows to set 2 optional proxies:

- `cw-ics721-incoming-proxy-base`: it holds a list of whitelisted channels, on NFT inbounds only these channels WLed for ics721
- `cw-ics721-outgoing-proxy-rate-limit`: rate limits number of NFTs being able to be transferred per block by ics721 on outbound

NOTE: Ark Protocol manages its own, extended outgoing proxy. Here it optionally WLs channels, code ids and collections.
More upcoming features are in the pipe, like:

- `single-hop`: allowing NFTs to only transfer to one chain, transferring to another chain, requires NFT be backtransfer first to home chain
- `backtransfer`: always allow backtransfer without requiring being WLed
- `collection-fee`: allowing to define fees on a collection level
- ...

```sh
# script does 2 things:
# - instantiate proxies with WL channel and rate limit of 1 NFT per block
# - migrate ics721, set proxies and sets arkite address as pauser and wallet as cw721 admin
./scripts/setup-proxies.sh
```

Finally, counter party contracts need to be set:

```sh
./scripts/setup-counter-party-contracts.sh
# output:
# > ...
# > {"data":"stars1f2fg4jdkfj78y43w8l4w4x0vnmyd80m6seq67reyfty0g42s24jq4r0nyu"}
# > ...
# > {"data":"osmo1sg29sdvmkjc7cdlawv8c85khmk55q5lfkkfgc0qpcxgrspere97szgwg9w"}
```

### Getting Started: Mint and InterChain Transfer an NFT between Osmosis and Stargaze

ICS721 controls collection contracts on other chains (than home chain where NFT originates). In latest `cw721-base v0.19` release
creator is authorised to update `NftInfo` and extension (aka NFT metadata).

In this `arkite-passport` example, `passport` and `poap` collections are created, where `arkite-passport` contract is minter and creator.
On transfer ics721 also creates a `passport` collection (aka `passport voucher`) on target chain. Here creator and minter is ics721.
Hence `arkite-passport` wont be able to update NFTs on target chain.

Admin to the rescue - on initial transfer, voucher collection is created.

```sh
# mint NFT via Arkite Passport contract as minter for cw721 contract:
./scripts/mint.sh stargaze
# output:
# ...
# > Minted NFT #0
# > ============ checking NFT
# > starsd query wasm contract-state smart stars12u499ljeegts85hqx937rpwccuhw48272ke7n7kvkhfznu0ky7mqgz2gv9 '{"all_nft_info":{"token_id": "0"}}' --chain-id elgafar-1 --node https://rpc.elgafar-1.stargaze-apis.com:443 --output json
# > ------------------------------------------------------------
# > stargaze
# > - nft contract: stars12u499ljeegts85hqx937rpwccuhw48272ke7n7kvkhfznu0ky7mqgz2gv9
# > - NFT #0, token uri: ipfs://passport/default, owner: stars1qk0hwv23h2kdsewt92apk62f2v40fla8z8qlth (ics721: stars14uelnppq5vsc3dfp8k3ll68cqrdpcf4nrhns9j0v6jnc6k9hj94skccdmh)

# interchain transfer NFT and relay
./scripts/transfer.sh stargaze 0
# output:
# > ============ Transferring NFT
# > ...
# > ============ relaying packets
# > ...
# > 2024-05-10T12:48:31.763936Z  INFO ThreadId(01) relay_recv_packet_and_timeout_messages{src_chain=elgafar-1 src_port=wasm.stars14uelnppq5vsc3dfp8k3ll68cqrdpcf4nrhns9j0v6jnc6k9hj94skccdmh # > src_channel=channel-923 dst_chain=osmo-test-5}:relay{odata=packet-recv ->Destination @1-10430320; len=1}: [Sync->osmo-test-5] result events:
        UpdateClient(UpdateClient { Attributes { client_id: 07-tendermint-3495, client_type: ClientType(07-tendermint), consensus_height: 1-10430321 } }) at height 5-7445842
# >         WriteAcknowledgement(WriteAcknowledgement { packet: seq:1, path:channel-923/wasm.stars14uelnppq5vsc3dfp8k3ll68cqrdpcf4nrhns9j0v6jnc6k9hj94skccdmh->channel-7836/wasm.# > osmo1sq5x7mag5mxdkmsv2kw6j7gu3u9m68x4kcdfpwyzlgjergxxjaks7rkc8m, toh:no timeout, tos:2024-05-10T13:48:11.443847Z), ack: [ 123, 34, 114, 101, 115, 117, 108, 116, 34, 58, 34, 77, 81, 61, 61, 34, 125 ] }) at height 5-7445842
...
# > ============ checking NFTs
# > ...
# > stargaze
# > - nft contract: stars12u499ljeegts85hqx937rpwccuhw48272ke7n7kvkhfznu0ky7mqgz2gv9
# > - NFT #0, token uri: ipfs://passport/escrowed, owner: stars14uelnppq5vsc3dfp8k3ll68cqrdpcf4nrhns9j0v6jnc6k9hj94skccdmh (ics721: stars14uelnppq5vsc3dfp8k3ll68cqrdpcf4nrhns9j0v6jnc6k9hj94skccdmh)
# > ------------------------------------------------------------
# > ...
# > osmosis
# > - nft contract: osmo189smaj36x85w2ldfvtgc3m2w5fygte2wpk4qp3ttsgneyvmvadesad8dvc
# > - NFT #0, token uri: ipfs://passport/default, owner: osmo1qk0hwv23h2kdsewt92apk62f2v40fla87qyjk5 (ics721: osmo1sq5x7mag5mxdkmsv2kw6j7gu3u9m68x4kcdfpwyzlgjergxxjaks7rkc8m)
# > ------------------------------------------------------------

# Now let's do a back transfer:
./scripts/transfer.sh osmosis 0 osmo189smaj36x85w2ldfvtgc3m2w5fygte2wpk4qp3ttsgneyvmvadesad8dvc
# output:
# ...
# ============ checking NFTs
# osmosisd query wasm contract-state smart osmo189smaj36x85w2ldfvtgc3m2w5fygte2wpk4qp3ttsgneyvmvadesad8dvc '{"all_nft_info":{"token_id": "0"}}' --chain-id osmo-test-5 --node https://rpc.osmo.test.yieldpay.finance:443 --output json
# Error: rpc error: code = Unknown desc = type: cw721::state::NftInfo<core::option::Option<cw721::state::NftExtension>>; key: [00, 06, 74, 6F, 6B, 65, 6E, 73, 30] not found: query wasm contract failed: unknown request
# ------------------------------------------------------------
# osmosis
# Failed to upload  to : A0095A925FD114565F531CAC6CA33447DB24BE54E4515B52D470B46999712A55
# - NFT #0 got burned
# ------------------------------------------------------------
# starsd query wasm contract-state smart stars12u499ljeegts85hqx937rpwccuhw48272ke7n7kvkhfznu0ky7mqgz2gv9 '{"all_nft_info":{"token_id": "0"}}' --chain-id elgafar-1 --node https://rpc.elgafar-1.stargaze-apis.com:443 --output json
# ------------------------------------------------------------
# stargaze
# - nft contract: stars12u499ljeegts85hqx937rpwccuhw48272ke7n7kvkhfznu0ky7mqgz2gv9
# - NFT #0, token uri: https://github.com/arkprotocol/cw-ics721-callback-example/raw/main/public/passport_stargaze01_home.png, owner: stars1qk0hwv23h2kdsewt92apk62f2v40fla8z8qlth (ics721: stars1k6jl2wcgl8uh5h9yp4usxxz0r6pha6myuwvx8mm5l8tqqzpc2xtsp7lgl4)
# - POAP NFT #1
# ------------------------------------------------------------

# also try on other chain:
./scripts/mint.sh osmosis
./scripts/transfer.sh osmosis 0
```

Some notes here:

- if hermes logs `ack: [ 123, 34, 114, 101, 115, 117, 108, 116, 34, 58, 34, 77, 81, 61, 61, 34, 125 ]` (=`{"result":"MQ=="}` ), then relaying was succcessful
- `token_uri` on source and target chain are updated during transfer
  - on initial transfer (e.g. Osmosis -> Stargaze):
    - in case on target chain is unchanged (=`ipfs://passport/default`)
    - reason: there is no counter party contract defined yet for receive callback
    - solution: run `setup-counter-party-contracts.sh`
    - once proxies are set, transfers triggers callback to counter party contract
      - `token_uri` on source chain is: `ipfs://passport/escrowed`
      - `token_uri` on target chain is: `ipfs://passport/transferred`
  - on back transfer (e.g. Stargaze -> Omosis):
    - NFT is burned on Stargaze
    - `token_uri` on Osmosis is set back: `ipfs://passport/default`
