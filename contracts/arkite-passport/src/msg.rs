use cosmwasm_schema::{cw_serde, QueryResponses};
use cw_cii::ContractInstantiateInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub cw721_base: ContractInstantiateInfo,
    pub ics721_base: ContractInstantiateInfo,
    pub incoming_proxy: ContractInstantiateInfo,
    pub outgoing_proxy: ContractInstantiateInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Test {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(String)]
    Test {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {},
}
