use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw721_base::receiver::Cw721ReceiveMsg;
use cw_cii::ContractInstantiateInfo;
use ics721_types::types::{Ics721AckCallbackMsg, Ics721ReceiveCallbackMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub default_token_uri: String,
    pub escrowed_token_uri: String,
    pub transferred_token_uri: String,
    pub cw721_base: ContractInstantiateInfo,
    pub ics721_base: ContractInstantiateInfo,
    pub cw721_poap: ContractInstantiateInfo,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {},
    ReceiveNft(Cw721ReceiveMsg),
    CounterPartyContract {
        addr: String,
    },
    /// Ack callback on source chain
    Ics721AckCallback(Ics721AckCallbackMsg),
    /// Receive callback on target chain, NOTE: if this fails, the transfer will fail and NFT is reverted back to the sender
    Ics721ReceiveCallback(Ics721ReceiveCallbackMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Addr)]
    Poap {},
    #[returns(Addr)]
    CW721 {},
    #[returns(Addr)]
    ICS721 {},
    #[returns(String)]
    DefaultTokenUri {},
    #[returns(String)]
    EscrowedTokenUri {},
    #[returns(String)]
    TransferredTokenUri {},
    #[returns(String)]
    CounterPartyContract {},
}

#[cw_serde]
pub enum MigrateMsg {
    WithUpdate {
        default_token_uri: Option<String>,
        escrowed_token_uri: Option<String>,
        transferred_token_uri: Option<String>,
    },
}

#[cw_serde]
pub struct CallbackMsg {
    pub token_id: String,
    /// NFT owner on source chain, on ack this sender also receives a POAP
    pub sender: String,
}
