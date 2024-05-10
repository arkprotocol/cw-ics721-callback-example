use anyhow::Result;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Api, CanonicalAddr, DepsMut, Empty, Env, GovMsg,
    IbcTimeout, MemoryStorage, Reply, Response, Storage, Timestamp,
};
use cw721_base::{
    msg::{AllNftInfoResponse, InstantiateMsg as Cw721InstantiateMsg, NumTokensResponse},
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, Ownership,
};
use cw_cii::{Admin, ContractInstantiateInfo};
use cw_multi_test::{
    addons::MockApiBech32, AddressGenerator, App, AppBuilder, AppResponse, BankKeeper, Contract,
    ContractWrapper, DistributionKeeper, Executor, FailingModule, IbcAcceptingModule, Router,
    StakeKeeper, StargateFailing, WasmKeeper,
};
use ics721::{ClassId, ContractError as Ics721ContractError, NonFungibleTokenPacketData, TokenId};
use ics721_types::{
    ibc_types::IbcOutgoingMsg,
    types::{Ics721AckCallbackMsg, Ics721ReceiveCallbackMsg, Ics721Status},
};
use sha2::{digest::Update, Digest, Sha256};

use crate::{
    error::ContractError,
    execute,
    msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, QueryMsg},
};

use ics721::msg::{InstantiateMsg as Ics721InstantiateMsg, MigrateMsg as Ics721MigrateMsg};

const ARKITE_WALLET: &str = "arkite";
const NFT_OWNER_WALLET: &str = "nft_owner";
const OTHER_CHAIN_WALLET: &str = "other_chain";
const BECH32_PREFIX_HRP: &str = "ark";
const WHITELISTED_CHANNEL: &str = "channel";
const COUNTERPARTY_CONTRACT: &str = "counterparty_contract";
const DEFAULT_TOKEN_URI: &str = "ipfs://interchain.passport";
const ESCROWED_TOKEN_URI: &str = "ipfs://interchain.escrowed";
const TRANSFERRED_TOKEN_URI: &str = "ipfs://interchain.transferred";

type MockRouter = Router<
    BankKeeper,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
    IbcAcceptingModule,
    FailingModule<GovMsg, Empty, Empty>,
    StargateFailing,
>;

type MockApp = App<
    BankKeeper,
    MockApiBech32,
    MemoryStorage,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
    IbcAcceptingModule,
>;

#[derive(Default)]
pub struct MockAddressGenerator;

impl AddressGenerator for MockAddressGenerator {
    fn contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        code_id: u64,
        instance_id: u64,
    ) -> Result<Addr> {
        let canonical_addr = Self::instantiate_address(code_id, instance_id);
        Ok(Addr::unchecked(api.addr_humanize(&canonical_addr)?))
    }

    fn predictable_contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        _instance_id: u64,
        checksum: &[u8],
        creator: &CanonicalAddr,
        salt: &[u8],
    ) -> Result<Addr> {
        let canonical_addr = instantiate2_address(checksum, creator, salt)?;
        Ok(Addr::unchecked(api.addr_humanize(&canonical_addr)?))
    }
}

impl MockAddressGenerator {
    // non-predictable contract address generator, see `BuildContractAddressClassic`
    // implementation in wasmd: https://github.com/CosmWasm/wasmd/blob/main/x/wasm/keeper/addresses.go#L35-L42
    fn instantiate_address(code_id: u64, instance_id: u64) -> CanonicalAddr {
        let mut key = Vec::<u8>::new();
        key.extend_from_slice(b"wasm\0");
        key.extend_from_slice(&code_id.to_be_bytes());
        key.extend_from_slice(&instance_id.to_be_bytes());
        let module = Sha256::digest("module".as_bytes());
        Sha256::new()
            .chain(module)
            .chain(key)
            .finalize()
            .to_vec()
            .into()
    }
}

struct Test {
    app: MockApp,
    creator: Addr,
    nft_owner: Addr,
    other_chain_wallet: Addr,
    addr_arkite_contract: Addr,
    addr_poap_contract: Addr,
    addr_cw721_contract: Addr,
    addr_ics721_contract: Addr,
}

fn no_init(_router: &mut MockRouter, _api: &dyn Api, _storage: &mut dyn Storage) {}

impl Test {
    /// Test setup with optional pauser and proxy contracts.
    fn new() -> Self {
        let mut app = AppBuilder::new()
            .with_wasm::<WasmKeeper<Empty, Empty>>(
                WasmKeeper::new().with_address_generator(MockAddressGenerator),
            )
            .with_ibc(IbcAcceptingModule::default())
            .with_api(MockApiBech32::new(BECH32_PREFIX_HRP))
            .build(no_init);
        let code_id_arkite_passport = app.store_code(arkite_passport_contract());
        let code_id_cw721 = app.store_code(cw721_base_contract());
        let code_id_ics721 = app.store_code(ics721_contract());

        let creator = app.api().addr_make(ARKITE_WALLET);
        let addr_arkite_contract = app
            .instantiate_contract(
                code_id_arkite_passport,
                creator.clone(),
                &InstantiateMsg {
                    default_token_uri: DEFAULT_TOKEN_URI.to_string(),
                    escrowed_token_uri: ESCROWED_TOKEN_URI.to_string(),
                    transferred_token_uri: TRANSFERRED_TOKEN_URI.to_string(),
                    cw721_poap: ContractInstantiateInfo {
                        admin: Some(Admin::Instantiator {}),
                        msg: to_json_binary(&Cw721InstantiateMsg::<
                            DefaultOptionalCollectionExtensionMsg,
                        > {
                            name: "poap".to_string(),
                            symbol: "poap".to_string(),
                            collection_info_extension: None,
                            minter: None,  // none = sender/arkite is minter
                            creator: None, // none = sender/arkite is creator
                            withdraw_address: None,
                        })
                        .unwrap(),
                        code_id: code_id_cw721,
                        label: "arkite passport".to_string(),
                    },
                    cw721_base: ContractInstantiateInfo {
                        admin: Some(Admin::Instantiator {}),
                        msg: to_json_binary(&Cw721InstantiateMsg::<
                            DefaultOptionalCollectionExtensionMsg,
                        > {
                            name: "name".to_string(),
                            symbol: "symbol".to_string(),
                            collection_info_extension: None,
                            minter: None,  // none = sender/arkite is minter
                            creator: None, // none = sender/arkite is creator
                            withdraw_address: None,
                        })
                        .unwrap(),
                        code_id: code_id_cw721,
                        label: "arkite passport".to_string(),
                    },
                    ics721_base: ContractInstantiateInfo {
                        admin: Some(Admin::Address {
                            addr: creator.to_string(),
                        }),
                        msg: to_json_binary(&Ics721InstantiateMsg {
                            cw721_base_code_id: code_id_cw721,
                            incoming_proxy: None,
                            outgoing_proxy: None,
                            pauser: Some(creator.to_string()),
                            cw721_admin: None,
                        })
                        .unwrap(),
                        code_id: code_id_ics721,
                        label: "arkite passport".to_string(),
                    },
                },
                &[],
                "cw721-base",
                None,
            )
            .unwrap();

        let addr_poap_contract = app
            .wrap()
            .query_wasm_smart(addr_arkite_contract.clone(), &QueryMsg::Poap {})
            .unwrap();

        let addr_cw721_contract = app
            .wrap()
            .query_wasm_smart(addr_arkite_contract.clone(), &QueryMsg::CW721 {})
            .unwrap();

        let addr_ics721_contract: Addr = app
            .wrap()
            .query_wasm_smart(addr_arkite_contract.clone(), &QueryMsg::ICS721 {})
            .unwrap();

        let code_id_outgoing_proxy = app.store_code(outgoing_proxy_contract());
        let addr_outgoing_proxy_contract = app
            .instantiate_contract(
                code_id_outgoing_proxy,
                creator.clone(),
                &cw_ics721_outgoing_proxy_rate_limit::msg::InstantiateMsg {
                    rate_limit: cw_ics721_outgoing_proxy_rate_limit::Rate::PerBlock(10),
                    origin: Some(addr_ics721_contract.to_string()),
                },
                &[],
                "cw721-base",
                None,
            )
            .unwrap();

        let code_id_incoming_proxy = app.store_code(incoming_proxy_contract());
        let addr_incoming_proxy_contract = app
            .instantiate_contract(
                code_id_incoming_proxy,
                creator.clone(),
                &cw_ics721_incoming_proxy_base::msg::InstantiateMsg {
                    origin: Some(addr_ics721_contract.to_string()),
                    channels: Some(vec![WHITELISTED_CHANNEL.to_string()]),
                },
                &[],
                "cw721-base",
                None,
            )
            .unwrap();
        app.migrate_contract(
            creator.clone(),
            addr_ics721_contract.clone(),
            &Ics721MigrateMsg::WithUpdate {
                cw721_base_code_id: None,
                incoming_proxy: Some(addr_incoming_proxy_contract.to_string()),
                outgoing_proxy: Some(addr_outgoing_proxy_contract.to_string()),
                pauser: None,
                cw721_admin: None,
            },
            code_id_ics721,
        )
        .unwrap();

        let nft_owner = app.api().addr_make(NFT_OWNER_WALLET);
        let other_chain_wallet = app.api().addr_make(OTHER_CHAIN_WALLET);

        let mut test = Self {
            app,
            creator,
            nft_owner,
            other_chain_wallet,
            addr_arkite_contract,
            addr_poap_contract,
            addr_cw721_contract,
            addr_ics721_contract,
        };
        test.execute_counter_party_contract(COUNTERPARTY_CONTRACT.to_string())
            .unwrap();
        test
    }

    fn query_default_token_uri(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_arkite_contract.clone(),
                &QueryMsg::DefaultTokenUri {},
            )
            .unwrap()
    }

    fn query_escrowed_token_uri(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_arkite_contract.clone(),
                &QueryMsg::EscrowedTokenUri {},
            )
            .unwrap()
    }

    fn query_transferred_token_uri(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_arkite_contract.clone(),
                &QueryMsg::TransferredTokenUri {},
            )
            .unwrap()
    }

    fn query_poap(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(self.addr_arkite_contract.clone(), &QueryMsg::Poap {})
            .unwrap()
    }

    fn query_cw721(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(self.addr_arkite_contract.clone(), &QueryMsg::CW721 {})
            .unwrap()
    }

    fn query_ics721(&mut self) -> Addr {
        self.app
            .wrap()
            .query_wasm_smart(self.addr_arkite_contract.clone(), &QueryMsg::ICS721 {})
            .unwrap()
    }

    fn query_counter_party_contract(&mut self) -> String {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_arkite_contract.clone(),
                &QueryMsg::CounterPartyContract {},
            )
            .unwrap()
    }

    fn query_cw721_num_tokens(&mut self, cw721: Addr) -> NumTokensResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                cw721,
                &cw721_base::msg::QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::NumTokens {},
            )
            .unwrap()
    }

    fn query_cw721_minter_ownership(&mut self) -> Ownership<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_cw721_contract.clone(),
                &cw721_base::msg::QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::GetMinterOwnership {},
            )
            .unwrap()
    }

    fn query_cw721_creator_ownership(&mut self) -> Ownership<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_cw721_contract.clone(),
                &cw721_base::msg::QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::GetCreatorOwnership {},
            )
            .unwrap()
    }

    fn query_cw721_all_nft_info(
        &mut self,
        cw721: Addr,
        token_id: String,
    ) -> AllNftInfoResponse<DefaultOptionalNftExtension> {
        self.app
            .wrap()
            .query_wasm_smart(
                cw721,
                &cw721_base::msg::QueryMsg::<
                    DefaultOptionalNftExtension,
                    DefaultOptionalCollectionExtension,
                    Empty,
                >::AllNftInfo {
                    token_id,
                    include_expired: None,
                },
            )
            .unwrap()
    }

    fn execute_passport_mint(&mut self, sender: Addr) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            sender,
            self.addr_arkite_contract.clone(),
            &ExecuteMsg::Mint {},
            &[],
        )
    }

    fn execute_counter_party_contract(
        &mut self,
        addr: String,
    ) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            self.creator.clone(),
            self.addr_arkite_contract.clone(),
            &ExecuteMsg::CounterPartyContract { addr },
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn execute_ack_callback(
        &mut self,
        ics721: Addr,
        class_id: ClassId,
        status: Ics721Status,
        msg: CallbackMsg,
        token_id: String,
        receiver: String,
        sender: String,
    ) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            ics721,
            self.addr_arkite_contract.clone(),
            &ExecuteMsg::Ics721AckCallback(Ics721AckCallbackMsg {
                status,
                nft_contract: self.addr_cw721_contract.to_string(),
                msg: to_json_binary(&msg).unwrap(),
                original_packet: NonFungibleTokenPacketData {
                    class_id,
                    token_ids: vec![TokenId::new(token_id)],
                    receiver,
                    sender,
                    class_data: None,
                    class_uri: None,
                    memo: None,
                    token_data: None,
                    token_uris: None,
                },
            }),
            &[],
        )
    }

    fn execute_receive_callback(
        &mut self,
        ics721: Addr,
        class_id: ClassId,
        msg: CallbackMsg,
        token_id: String,
        receiver: String,
        sender: String,
    ) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            ics721,
            self.addr_arkite_contract.clone(),
            &ExecuteMsg::Ics721ReceiveCallback(Ics721ReceiveCallbackMsg {
                msg: to_json_binary(&msg).unwrap(),
                nft_contract: self.addr_cw721_contract.to_string(), // pretend this is the escrowed cw721 contract
                original_packet: NonFungibleTokenPacketData {
                    class_id,
                    token_ids: vec![TokenId::new(token_id)],
                    receiver,
                    sender,
                    class_data: None,
                    class_uri: None,
                    memo: None,
                    token_data: None,
                    token_uris: None,
                },
            }),
            &[],
        )
    }

    fn execute_cw721_send_nft(
        &mut self,
        token_id: String,
        receiver: String,
        channel_id: String,
    ) -> Result<AppResponse, anyhow::Error> {
        let ibc_outgoing_msg = IbcOutgoingMsg {
            receiver,
            channel_id,
            timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(0)),
            memo: None,
        };

        self.app.execute_contract(
            self.nft_owner.clone(),
            self.addr_cw721_contract.clone(),
            &cw721_base::msg::ExecuteMsg::<
                DefaultOptionalNftExtensionMsg,
                DefaultOptionalCollectionExtensionMsg,
                Empty,
            >::SendNft {
                contract: self.addr_arkite_contract.to_string(),
                token_id,
                msg: to_json_binary(&ibc_outgoing_msg).unwrap(),
            },
            &[],
        )
    }
}

fn arkite_passport_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute::execute, execute::instantiate, execute::query)
        .with_reply(execute::reply);
    Box::new(contract)
}

fn cw721_base_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw721_base::entry::execute,
        cw721_base::entry::instantiate,
        cw721_base::entry::query,
    );
    Box::new(contract)
}

fn ics721_contract() -> Box<dyn Contract<Empty>> {
    // need to wrap method in function for testing
    fn ibc_reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, Ics721ContractError> {
        ics721_base::reply(deps, env, reply)
    }

    let contract = ContractWrapper::new(
        ics721_base::execute,
        ics721_base::instantiate,
        ics721_base::query,
    )
    .with_migrate(ics721_base::migrate)
    .with_reply(ibc_reply);
    Box::new(contract)
}

fn incoming_proxy_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_ics721_incoming_proxy_base::contract::execute,
        cw_ics721_incoming_proxy_base::contract::instantiate,
        cw_ics721_incoming_proxy_base::contract::query,
    );
    Box::new(contract)
}

fn outgoing_proxy_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_ics721_outgoing_proxy_rate_limit::contract::execute,
        cw_ics721_outgoing_proxy_rate_limit::contract::instantiate,
        cw_ics721_outgoing_proxy_rate_limit::contract::query,
    )
    .with_reply(cw_ics721_outgoing_proxy_rate_limit::contract::reply);
    Box::new(contract)
}

#[test]
fn test_instantiate() {
    let mut test = Test::new();

    // check stores are properly initialized
    let poap = test.query_poap();
    assert_eq!(poap, test.addr_poap_contract);
    let cw721 = test.query_cw721();
    assert_eq!(cw721, test.addr_cw721_contract);
    let ics721 = test.query_ics721();
    assert_eq!(ics721, test.addr_ics721_contract);
    let supply = test
        .query_cw721_num_tokens(test.addr_cw721_contract.clone())
        .count;
    assert_eq!(supply, 0);
    let default_token_uri = test.query_default_token_uri();
    assert_eq!(default_token_uri, DEFAULT_TOKEN_URI.to_string());
    let escrowed_token_uri = test.query_escrowed_token_uri();
    assert_eq!(escrowed_token_uri, ESCROWED_TOKEN_URI.to_string());
    let transferred_token_uri = test.query_transferred_token_uri();
    assert_eq!(transferred_token_uri, TRANSFERRED_TOKEN_URI.to_string());

    // cw721: check minter is arkite contract and creator is arkite wallet
    let minter_owner_ship = test.query_cw721_minter_ownership();
    assert_eq!(
        minter_owner_ship.owner,
        Some(test.addr_arkite_contract.clone())
    );
    let creator_owner_ship = test.query_cw721_creator_ownership();
    assert_eq!(
        creator_owner_ship.owner,
        Some(test.addr_arkite_contract.clone())
    );
}

#[test]
fn test_execute_counter_party_contract() {
    let mut test = Test::new();

    // mint and send nft
    test.execute_counter_party_contract(COUNTERPARTY_CONTRACT.to_string())
        .unwrap();

    // assert results
    // - nft owned by ics721
    let counter_party_contract = test.query_counter_party_contract();
    assert_eq!(counter_party_contract, COUNTERPARTY_CONTRACT.to_string());
}

#[test]
fn test_mint() {
    let mut test = Test::new();

    test.execute_passport_mint(test.nft_owner.clone()).unwrap();

    // assert results
    let supply = test
        .query_cw721_num_tokens(test.addr_cw721_contract.clone())
        .count;
    assert_eq!(supply, 1);

    let all_nft_info =
        test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
    assert_eq!(all_nft_info.access.owner, test.nft_owner);
    assert_eq!(
        all_nft_info.info.token_uri,
        Some(DEFAULT_TOKEN_URI.to_string())
    );
}

#[test]
fn test_send_nft() {
    let mut test = Test::new();

    // mint and send nft
    test.execute_passport_mint(test.nft_owner.clone()).unwrap();
    test.execute_cw721_send_nft(
        "0".to_string(),
        "receiver".to_string(),
        WHITELISTED_CHANNEL.to_string(),
    )
    .unwrap();

    // assert results
    // - nft owned by ics721
    let all_nft_info =
        test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
    assert_eq!(all_nft_info.access.owner, test.addr_ics721_contract);
}

#[test]
fn test_receive_callback() {
    // assert unauthorized
    {
        let mut test = Test::new();
        // process unauhtorized receive
        let err: ContractError = test
            .execute_receive_callback(
                test.addr_cw721_contract.clone(), // unauthorized
                ClassId::new("some/class/id"),
                CallbackMsg {
                    sender: test.other_chain_wallet.to_string(),
                    token_id: "1".to_string(),
                },
                "1".to_string(),
                test.nft_owner.to_string(),
                test.other_chain_wallet.to_string(),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::UnauthorizedCallback {});
    }
    // assert receive ok
    {
        let mut test = Test::new();
        // pretend nft has been transferred
        test.execute_passport_mint(test.nft_owner.clone()).unwrap();
        // assert nft info and token uri
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.nft_owner);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(DEFAULT_TOKEN_URI.to_string())
        );
        // assert no poaps yet minted
        let supply = test
            .query_cw721_num_tokens(test.addr_poap_contract.clone())
            .count;
        assert_eq!(supply, 0);

        // process receive
        test.execute_receive_callback(
            test.addr_ics721_contract.clone(),
            ClassId::new("some/class/id"),
            CallbackMsg {
                sender: test.other_chain_wallet.to_string(),
                token_id: "0".to_string(),
            },
            "0".to_string(),
            test.nft_owner.to_string(),
            test.other_chain_wallet.to_string(),
        )
        .unwrap();
        // assert token uri has changed
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.nft_owner);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(TRANSFERRED_TOKEN_URI.to_string())
        );
        // assert one poap minted
        let supply = test
            .query_cw721_num_tokens(test.addr_poap_contract.clone())
            .count;
        assert_eq!(supply, 1);

        // process receive again for testing back transfer
        test.execute_receive_callback(
            test.addr_ics721_contract.clone(),
            ClassId::new(test.addr_cw721_contract.to_string()),
            CallbackMsg {
                sender: test.other_chain_wallet.to_string(),
                token_id: "0".to_string(),
            },
            "0".to_string(),
            test.nft_owner.to_string(),
            test.other_chain_wallet.to_string(),
        )
        .unwrap();
        // assert token uri has changed
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.nft_owner);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(DEFAULT_TOKEN_URI.to_string())
        );
        // assert one poap minted
        let supply = test
            .query_cw721_num_tokens(test.addr_poap_contract.clone())
            .count;
        assert_eq!(supply, 2);
    }
}

#[test]
fn test_ack_callback() {
    // assert unauthorized
    {
        let mut test = Test::new();
        // process unauhtorized ack
        let err: ContractError = test
            .execute_ack_callback(
                test.addr_cw721_contract.clone(), // unauthorized
                ClassId::new("some/class/id"),
                Ics721Status::Success,
                CallbackMsg {
                    sender: test.nft_owner.to_string(),
                    token_id: "0".to_string(),
                },
                "0".to_string(),
                test.nft_owner.to_string(),
                test.nft_owner.to_string(),
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::UnauthorizedCallback {});
    }
    // assert ack success
    {
        let mut test = Test::new();
        // pretend nft has been escrowed by ics721
        test.execute_passport_mint(test.addr_ics721_contract.clone())
            .unwrap();
        // assert nft info and token uri
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.addr_ics721_contract);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(DEFAULT_TOKEN_URI.to_string())
        );

        // process ack
        test.execute_ack_callback(
            test.addr_ics721_contract.clone(),
            ClassId::new("some/class/id"),
            Ics721Status::Success,
            CallbackMsg {
                sender: test.nft_owner.to_string(),
                token_id: "0".to_string(),
            },
            "0".to_string(),
            test.nft_owner.to_string(),
            test.nft_owner.to_string(),
        )
        .unwrap();
        // assert token uri has changed
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.addr_ics721_contract);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(ESCROWED_TOKEN_URI.to_string())
        );

        // process ack again
        test.execute_ack_callback(
            test.addr_ics721_contract.clone(),
            ClassId::new(test.addr_cw721_contract.to_string()),
            Ics721Status::Success,
            CallbackMsg {
                sender: test.nft_owner.to_string(),
                token_id: "0".to_string(),
            },
            "0".to_string(),
            test.nft_owner.to_string(),
            test.nft_owner.to_string(),
        )
        .unwrap();
        // assert token uri has changed
        let all_nft_info =
            test.query_cw721_all_nft_info(test.addr_cw721_contract.clone(), "0".to_string());
        assert_eq!(all_nft_info.access.owner, test.addr_ics721_contract);
        assert_eq!(
            all_nft_info.info.token_uri,
            Some(DEFAULT_TOKEN_URI.to_string())
        );
    }
    // assert ack fail
    {
        let mut test = Test::new();

        // process ack
        test.execute_ack_callback(
            test.addr_ics721_contract.clone(),
            ClassId::new("some/class/id"),
            Ics721Status::Failed("some reason".to_string()),
            CallbackMsg {
                sender: test.nft_owner.to_string(),
                token_id: "0".to_string(),
            },
            "0".to_string(),
            test.nft_owner.to_string(),
            test.nft_owner.to_string(),
        )
        .unwrap();
    }
}
