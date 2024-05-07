#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response,
    StdResult, Storage, SubMsg, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use cw721_base::{
    receiver::Cw721ReceiveMsg, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtensionMsg,
};
use cw_utils::parse_reply_instantiate_data;
use ics721_types::{
    ibc_types::IbcOutgoingMsg,
    types::{Ics721Callbacks, Ics721Memo},
};

use crate::{
    error::ContractError,
    msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{ADDR_CW721, ADDR_ICS721, ADDR_POAP, COUNTERPARTY_CONTRACT, NFT_EXTENSION_MSG, SUPPLY},
    INSTANTIATE_CW721_REPLY_ID, INSTANTIATE_ICS721_REPLY_ID, INSTANTIATE_POAP_REPLY_ID,
    MINT_NFT_REPLY_ID,
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
        msg.cw721_poap.into_wasm_msg(env.clone().contract.address),
        INSTANTIATE_POAP_REPLY_ID,
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {} => execute_mint(deps, info),
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, msg),
        ExecuteMsg::CounterPartyContract { addr } => execute_counterparty_contract(deps, addr),
    }
}

fn execute_counterparty_contract(deps: DepsMut, addr: String) -> Result<Response, ContractError> {
    COUNTERPARTY_CONTRACT.save(deps.storage, &addr)?;
    Ok(Response::default()
        .add_attribute("method", "execute_counterparty_contract")
        .add_attribute("counterparty_contract", addr))
}

fn execute_mint(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
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
    Ok(Response::default()
        .add_attribute("method", "execute_mint")
        .add_submessage(sub_msg))
}

fn execute_receive_nft(
    deps: DepsMut,
    env: Env,
    msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let ics721 = ADDR_ICS721.load(deps.storage)?;
    // query whether there is an outgoing proxy defined by ics721
    let outgoing_proxy_or_ics721 = match deps
        .querier
        .query_wasm_smart(ics721.clone(), &ics721::msg::QueryMsg::OutgoingProxy {})?
    {
        Some(outgoing_proxy) => outgoing_proxy,
        None => ics721,
    };
    let mut ibc_msg: IbcOutgoingMsg = from_json(&msg.msg)?;
    let memo = create_memo(deps.storage, env, msg.sender, msg.token_id.clone())?;
    ibc_msg.memo = Some(Binary::to_base64(&to_json_binary(&memo)?));
    // forward nft to ics721 or outgoing proxy
    let cw721 = ADDR_CW721.load(deps.storage)?;
    let send_msg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_json_binary(&cw721_base::msg::ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::SendNft {
            contract: outgoing_proxy_or_ics721.to_string(),
            token_id: msg.token_id,
            msg: to_json_binary(&ibc_msg)?,
        })?,
        funds: vec![],
    };
    Ok(Response::default()
        .add_message(send_msg)
        .add_attribute("method", "execute_receive_nft")
        .add_attribute("receiver", ibc_msg.receiver.clone())
        .add_attribute("channel_id", ibc_msg.channel_id.clone()))
}

fn create_memo(
    storage: &dyn Storage,
    env: Env,
    sender: String,
    token_id: String,
) -> Result<Ics721Memo, ContractError> {
    let callback_msg = CallbackMsg { sender, token_id };
    let mut callbacks = Ics721Callbacks {
        ack_callback_data: Some(to_json_binary(&callback_msg)?),
        ack_callback_addr: Some(env.contract.address.to_string()),
        receive_callback_data: None,
        receive_callback_addr: None,
    };
    if let Some(counterparty_contract) = COUNTERPARTY_CONTRACT.may_load(storage)? {
        callbacks.receive_callback_data = Some(to_json_binary(&callback_msg)?);
        callbacks.receive_callback_addr = Some(counterparty_contract); // here we need to set contract addr, since receiver is NFT receiver
    }
    Ok(Ics721Memo {
        callbacks: Some(callbacks),
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CounterPartyContract {} => {
            to_json_binary(&COUNTERPARTY_CONTRACT.load(deps.storage)?)
        }
        QueryMsg::Poap {} => to_json_binary(&ADDR_POAP.load(deps.storage)?),
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
        INSTANTIATE_POAP_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let poap = deps.api.addr_validate(&res.contract_address)?;
            ADDR_POAP.save(deps.storage, &poap)?;
            Ok(response.add_attribute("cw721", poap))
        }
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
