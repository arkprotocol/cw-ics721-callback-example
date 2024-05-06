#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, MessageInfo, Never, Reply,
    Response, StdResult, SubMsg,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{ADDR_CW721_BASE, ADDR_ICS721},
    INSTANTIATE_CW721_REPLY_ID, INSTANTIATE_ICS721_REPLY_ID,
};

const CONTRACT_NAME: &str = "crates.io:arkite-passport";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut sub_msgs: Vec<SubMsg<Empty>> = Vec::new();
    sub_msgs.push(SubMsg::reply_on_success(
        msg.cw721_base.into_wasm_msg(env.clone().contract.address),
        INSTANTIATE_CW721_REPLY_ID,
    ));
    sub_msgs.push(SubMsg::reply_on_success(
        msg.ics721_base.into_wasm_msg(env.clone().contract.address),
        INSTANTIATE_ICS721_REPLY_ID,
    ));
    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_submessages(sub_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    let response = Response::default().add_attribute("method", "reply");
    match reply.id {
        INSTANTIATE_CW721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let cw721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_CW721_BASE.save(deps.storage, &cw721)?;
            Ok(response.add_attribute("cw721", cw721))
        }
        INSTANTIATE_ICS721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let ics721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_ICS721.save(deps.storage, &ics721)?;
            Ok(response.add_attribute("ics721", ics721))
        }
        _ => Err(ContractError::UnrecognisedReplyId {}),
    }
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CW721 {} => to_json_binary(&ADDR_CW721_BASE.load(deps.storage)?),
        QueryMsg::ICS721 {} => to_json_binary(&ADDR_ICS721.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    todo!()
}
