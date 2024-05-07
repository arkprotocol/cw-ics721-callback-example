use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw721_base::msg::NftExtensionMsg;
use cw_cii::ContractInstantiateInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub nft_extension: NftExtensionMsg,
    pub cw721_base: ContractInstantiateInfo,
    pub ics721_base: ContractInstantiateInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(u64)]
    Supply {},
    #[returns(Addr)]
    CW721 {},
    #[returns(Addr)]
    ICS721 {},
    #[returns(NftExtensionMsg)]
    NftExtensionMsg {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {},
}
