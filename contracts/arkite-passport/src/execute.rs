#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use cw721_base::{DefaultOptionalCollectionExtensionMsg, DefaultOptionalNftExtensionMsg};
use cw_utils::parse_reply_instantiate_data;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{ADDR_CW721, ADDR_ICS721, NFT_EXTENSION_MSG, SUPPLY},
    INSTANTIATE_CW721_REPLY_ID, INSTANTIATE_ICS721_REPLY_ID, MINT_NFT_REPLY_ID,
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
    SUPPLY.save(deps.storage, &0)?;
    NFT_EXTENSION_MSG.save(deps.storage, &msg.nft_extension)?;
    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_submessages(sub_msgs))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let response = Response::default();
    match msg {
        ExecuteMsg::Mint {} => {
            let cw721 = ADDR_CW721.load(deps.storage)?;
            let token_id = SUPPLY.load(deps.storage)?;
            let nft_extension_msg = NFT_EXTENSION_MSG.load(deps.storage)?;
            let mint_msg = WasmMsg::Execute {
                contract_addr: cw721.to_string(),
                msg: to_json_binary(&cw721_base::msg::ExecuteMsg::<
                    DefaultOptionalNftExtensionMsg,
                    DefaultOptionalCollectionExtensionMsg,
                    Empty,
                >::Mint {
                    token_id: token_id.to_string(),
                    owner: info.sender.to_string(),
                    token_uri: None,
                    extension: Some(nft_extension_msg),
                })?,
                funds: vec![],
            };
            let sub_msg = SubMsg::reply_always(mint_msg, MINT_NFT_REPLY_ID);
            Ok(response
                .add_attribute("method", "execute_mint")
                .add_submessage(sub_msg))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CW721 {} => to_json_binary(&ADDR_CW721.load(deps.storage)?),
        QueryMsg::ICS721 {} => to_json_binary(&ADDR_ICS721.load(deps.storage)?),
        QueryMsg::NftExtensionMsg {} => to_json_binary(&NFT_EXTENSION_MSG.load(deps.storage)?),
        QueryMsg::Supply {} => to_json_binary(&SUPPLY.load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    todo!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    let response = Response::default()
        .add_attribute("method", "reply")
        .add_attribute("reply_id", reply.id.to_string());
    match reply.id {
        INSTANTIATE_CW721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let cw721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_CW721.save(deps.storage, &cw721)?;
            Ok(response.add_attribute("cw721", cw721))
        }
        INSTANTIATE_ICS721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let ics721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_ICS721.save(deps.storage, &ics721)?;
            Ok(response.add_attribute("ics721", ics721))
        }
        MINT_NFT_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => {
                let token_id = SUPPLY.load(deps.storage)?;
                SUPPLY.save(deps.storage, &(token_id + 1))?;
                Ok(response.add_attribute("token_id", token_id.to_string()))
            }
            SubMsgResult::Err(error) => {
                let token_id = SUPPLY.load(deps.storage)?;
                Err(ContractError::MintFailed { error, token_id })
            }
        },
        _ => Err(ContractError::UnrecognisedReplyId {}),
    }
}
