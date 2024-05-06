use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw_cii::ContractInstantiateInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub cw721_base: ContractInstantiateInfo,
    pub ics721_base: ContractInstantiateInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Test {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    CW721 {},
    #[returns(Addr)]
    ICS721 {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {},
}
