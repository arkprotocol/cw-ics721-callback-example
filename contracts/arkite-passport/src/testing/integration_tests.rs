use anyhow::Result;
use cosmwasm_std::{
    instantiate2_address, to_json_binary, Addr, Api, CanonicalAddr, DepsMut, Empty, Env, GovMsg,
    MemoryStorage, Reply, Response, Storage,
};
use cw721_base::{
    msg::{AllNftInfoResponse, InstantiateMsg as Cw721InstantiateMsg, NftExtensionMsg},
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, NftExtension, Ownership,
};
use cw_cii::{Admin, ContractInstantiateInfo};
use cw_multi_test::{
    addons::MockApiBech32, AddressGenerator, App, AppBuilder, AppResponse, BankKeeper, Contract,
    ContractWrapper, DistributionKeeper, Executor, FailingModule, IbcAcceptingModule, Router,
    StakeKeeper, StargateFailing, WasmKeeper,
};
use ics721::ContractError;
use sha2::{digest::Update, Digest, Sha256};

use crate::{
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use ics721::msg::{InstantiateMsg as Ics721InstantiateMsg, MigrateMsg as Ics721MigrateMsg};

const ARKITE_WALLET: &str = "arkite";
const NFT_OWNER_WALLET: &str = "nft_owner";
const BECH32_PREFIX_HRP: &str = "ark";
const WHITELISTED_CHANNEL: &str = "channel";

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
    code_id_cw721: u64,
    code_id_ics721: u64,
    addr_arkite_contract: Addr,
    addr_cw721_contract: Addr,
    addr_ics721_contract: Addr,
    addr_outgoing_proxy_contract: Addr,
    addr_incoming_proxy_contract: Addr,
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
                    nft_extension: Test::default_nft_extension_msg(),
                    cw721_base: ContractInstantiateInfo {
                        admin: Some(Admin::Instantiator {}),
                        msg: to_json_binary(&Cw721InstantiateMsg::<
                            DefaultOptionalCollectionExtensionMsg,
                        > {
                            name: "name".to_string(),
                            symbol: "symbol".to_string(),
                            collection_info_extension: None,
                            minter: None,
                            creator: Some(creator.to_string()),
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
                    origin: None,
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
                    origin: None,
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
        Self {
            app,
            creator,
            nft_owner,
            code_id_cw721,
            code_id_ics721,
            addr_arkite_contract,
            addr_cw721_contract,
            addr_ics721_contract,
            addr_outgoing_proxy_contract,
            addr_incoming_proxy_contract,
        }
    }

    fn default_nft_extension_msg() -> NftExtensionMsg {
        NftExtensionMsg {
            image: Some(Some("ipfs://interchain.passport".to_string())),
            image_data: None,
            external_url: None,
            description: None,
            name: None,
            attributes: None,
            background_color: None,
            animation_url: None,
            youtube_url: None,
        }
    }

    fn query_supply(&mut self) -> u64 {
        self.app
            .wrap()
            .query_wasm_smart(self.addr_arkite_contract.clone(), &QueryMsg::Supply {})
            .unwrap()
    }

    fn query_nft_extension_msg(&mut self) -> NftExtensionMsg {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_arkite_contract.clone(),
                &QueryMsg::NftExtensionMsg {},
            )
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
        token_id: String,
    ) -> AllNftInfoResponse<DefaultOptionalNftExtension> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.addr_cw721_contract.clone(),
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

    fn execute_cw721_mint(&mut self) -> Result<AppResponse, anyhow::Error> {
        self.app.execute_contract(
            self.nft_owner.clone(),
            self.addr_arkite_contract.clone(),
            &ExecuteMsg::Mint {},
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
    fn ibc_reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
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
    );
    Box::new(contract)
}

#[test]
fn test_instantiate() {
    let mut test = Test::new();

    // check stores are properly initialized
    let cw721 = test.query_cw721();
    assert_eq!(cw721, test.addr_cw721_contract);
    let ics721 = test.query_ics721();
    assert_eq!(ics721, test.addr_ics721_contract);
    let supply = test.query_supply();
    assert_eq!(supply, 0);
    let nft_extension_msg = test.query_nft_extension_msg();
    assert_eq!(nft_extension_msg, Test::default_nft_extension_msg());

    // cw721: check minter is arkite contract and creator is arkite wallet
    let minter_owner_ship = test.query_cw721_minter_ownership();
    assert_eq!(
        minter_owner_ship.owner,
        Some(test.addr_arkite_contract.clone())
    );
    let creator_owner_ship = test.query_cw721_creator_ownership();
    assert_eq!(creator_owner_ship.owner, Some(test.creator.clone()));
}

#[test]
fn test_mint() {
    let mut test = Test::new();

    test.execute_cw721_mint().unwrap();

    // assert results
    let supply = test.query_supply();
    assert_eq!(supply, 1);

    let all_nft_info = test.query_cw721_all_nft_info("0".to_string());
    assert_eq!(all_nft_info.access.owner, test.nft_owner);
    assert_eq!(
        all_nft_info.info.extension,
        Some(NftExtension {
            image: Some("ipfs://interchain.passport".to_string()),
            image_data: None,
            external_url: None,
            description: None,
            name: None,
            attributes: None,
            background_color: None,
            animation_url: None,
            youtube_url: None,
        })
    );
}
