#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg,
    IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg, IbcPacketReceiveMsg,
    IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Never, Reply, Response, StdResult,
    SubMsg,
};
use cw2::set_contract_version;
use ics721::ContractError;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    INSTANTIATE_CW721_REPLY_ID,
};

const CONTRACT_NAME: &str = "crates.io:arkite-passport";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut sub_msgs: Vec<SubMsg<Empty>> = Vec::new();
    sub_msgs.push(SubMsg::reply_on_success(
        msg.cw721_base.into_wasm_msg(env.clone().contract.address),
        INSTANTIATE_CW721_REPLY_ID,
    ));
    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_submessages(sub_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default().add_attribute("method", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    todo!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    todo!()
}
