use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw721_base::{msg::NftExtensionMsg, receiver::Cw721ReceiveMsg};
use cw_cii::ContractInstantiateInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub nft_extension: NftExtensionMsg,
    pub cw721_base: ContractInstantiateInfo,
    pub ics721_base: ContractInstantiateInfo,
    pub cw721_poap: ContractInstantiateInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {},
    ReceiveNft(Cw721ReceiveMsg),
    CounterPartyContract { addr: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    Supply {},
    #[returns(Addr)]
    Poap {},
    #[returns(Addr)]
    CW721 {},
    #[returns(Addr)]
    ICS721 {},
    #[returns(NftExtensionMsg)]
    NftExtensionMsg {},
    #[returns(String)]
    CounterPartyContract {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {},
}

#[cw_serde]
pub struct CallbackMsg {
    pub token_id: String,
    /// NFT owner on source chain, on ack this sender also receives a POAP
    pub sender: String,
}
