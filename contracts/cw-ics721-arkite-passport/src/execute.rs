use std::vec;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply,
    Response, StdResult, Storage, SubMsg, SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use cw721_base::{
    msg::{NftExtensionMsg, NftInfoResponse, NumTokensResponse},
    receiver::Cw721ReceiveMsg,
    state::Trait,
    DefaultOptionalCollectionExtensionMsg, DefaultOptionalNftExtension,
    DefaultOptionalNftExtensionMsg,
};
use cw_utils::parse_reply_instantiate_data;
use ics721_types::{
    ibc_types::IbcOutgoingMsg,
    types::{
        Ics721AckCallbackMsg, Ics721Callbacks, Ics721Memo, Ics721ReceiveCallbackMsg, Ics721Status,
    },
};

use crate::{
    error::ContractError,
    msg::{CallbackData, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    state::{
        ADDR_CW721, ADDR_ICS721, ADDR_POAP, COUNTERPARTY_CONTRACT, DEFAULT_TOKEN_URI,
        ESCROWED_TOKEN_URI, TRANSFERRED_TOKEN_URI,
    },
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
    let sub_msgs: Vec<SubMsg<Empty>> = vec![
        SubMsg::reply_on_success(
            msg.cw721_base.into_wasm_msg(env.clone().contract.address),
            INSTANTIATE_CW721_REPLY_ID,
        ),
        SubMsg::reply_on_success(
            msg.cw721_poap.into_wasm_msg(env.clone().contract.address),
            INSTANTIATE_POAP_REPLY_ID,
        ),
        SubMsg::reply_on_success(
            msg.ics721_base.into_wasm_msg(env.clone().contract.address),
            INSTANTIATE_ICS721_REPLY_ID,
        ),
    ];
    DEFAULT_TOKEN_URI.save(deps.storage, &msg.default_token_uri)?;
    ESCROWED_TOKEN_URI.save(deps.storage, &msg.escrowed_token_uri)?;
    TRANSFERRED_TOKEN_URI.save(deps.storage, &msg.transferred_token_uri)?;
    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("addr_arkite_passport", env.contract.address.to_string())
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
        ExecuteMsg::Mint {} => execute_mint(deps, info.sender.to_string()),
        ExecuteMsg::ReceiveNft(msg) => execute_receive_nft(deps, env, info, msg),
        ExecuteMsg::CounterPartyContract { addr } => execute_counterparty_contract(deps, addr),
        ExecuteMsg::Ics721AckCallback(msg) => execute_ack_callback(deps, env, info, msg),
        ExecuteMsg::Ics721ReceiveCallback(msg) => execute_receive_callback(deps, env, info, msg),
    }
}

fn execute_counterparty_contract(deps: DepsMut, addr: String) -> Result<Response, ContractError> {
    COUNTERPARTY_CONTRACT.save(deps.storage, &addr)?;
    Ok(Response::default()
        .add_attribute("method", "execute_counterparty_contract")
        .add_attribute("counterparty_contract", addr))
}

fn execute_mint(deps: DepsMut, owner: String) -> Result<Response, ContractError> {
    let cw721 = ADDR_CW721.load(deps.storage)?;
    let sub_msg = create_mint_msg(deps, cw721, owner)?;
    Ok(Response::default()
        .add_attribute("method", "execute_mint")
        .add_submessage(sub_msg))
}

fn create_mint_msg(deps: DepsMut, cw721: Addr, owner: String) -> Result<SubMsg, ContractError> {
    let num_tokens: NumTokensResponse = deps.querier.query_wasm_smart(
        cw721.clone(),
        &cw721_base::msg::QueryMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::NumTokens {},
    )?;

    let default_token_uri = DEFAULT_TOKEN_URI.load(deps.storage)?;
    let escrowed_token_uri = ESCROWED_TOKEN_URI.load(deps.storage)?;
    let transferred_token_uri = TRANSFERRED_TOKEN_URI.load(deps.storage)?;
    let trait_token_uri = Trait {
        display_type: None,
        trait_type: "token_uri".to_string(),
        value: default_token_uri.clone(),
    };
    let trait_default_uri = Trait {
        display_type: None,
        trait_type: "default_uri".to_string(),
        value: default_token_uri.clone(),
    };
    let trait_escrowed_uri = Trait {
        display_type: None,
        trait_type: "escrowed_uri".to_string(),
        value: escrowed_token_uri.clone(),
    };
    let trait_transferred_uri = Trait {
        display_type: None,
        trait_type: "transferred_uri".to_string(),
        value: transferred_token_uri.clone(),
    };
    let extension = Some(NftExtensionMsg {
        image: Some(Some(default_token_uri.clone())),
        attributes: Some(Some(vec![
            trait_token_uri,
            trait_default_uri,
            trait_escrowed_uri,
            trait_transferred_uri,
        ])),
        ..Default::default()
    });
    let mint_msg = WasmMsg::Execute {
        contract_addr: cw721.to_string(),
        msg: to_json_binary(&cw721_base::msg::ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::Mint {
            token_id: num_tokens.count.to_string(),
            owner,
            token_uri: Some(default_token_uri.clone()),
            extension,
        })?,
        funds: vec![],
    };
    let sub_msg = SubMsg::reply_always(mint_msg, MINT_NFT_REPLY_ID);
    Ok(sub_msg)
}

fn execute_receive_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
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
    let cw721 = info.sender;
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
        .add_attribute("cw721", cw721)
        .add_attribute("receiver", ibc_msg.receiver.clone())
        .add_attribute("channel_id", ibc_msg.channel_id.clone()))
}

fn create_memo(
    storage: &dyn Storage,
    env: Env,
    sender: String,
    token_id: String,
) -> Result<Ics721Memo, ContractError> {
    let default_token_uri = DEFAULT_TOKEN_URI.load(storage)?;
    let escrowed_token_uri = ESCROWED_TOKEN_URI.load(storage)?;
    let transferred_token_uri = TRANSFERRED_TOKEN_URI.load(storage)?;
    let callback_data = CallbackData {
        sender,
        token_id,
        default_token_uri,
        escrowed_token_uri,
        transferred_token_uri,
    };
    let mut callbacks = Ics721Callbacks {
        ack_callback_data: Some(to_json_binary(&callback_data)?),
        ack_callback_addr: Some(env.contract.address.to_string()),
        receive_callback_data: None,
        receive_callback_addr: None,
    };
    if let Some(counterparty_contract) = COUNTERPARTY_CONTRACT.may_load(storage)? {
        callbacks.receive_callback_data = Some(to_json_binary(&callback_data)?);
        callbacks.receive_callback_addr = Some(counterparty_contract); // here we need to set contract addr, since receiver is NFT receiver
    }
    Ok(Ics721Memo {
        callbacks: Some(callbacks),
    })
}

fn execute_receive_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Ics721ReceiveCallbackMsg,
) -> Result<Response, ContractError> {
    // only ics721 can execute callback
    let ics721 = ADDR_ICS721.load(deps.storage)?;
    if info.sender != ics721 {
        return Err(ContractError::UnauthorizedCallback {});
    }

    // receive callback does two things:
    // 1. change token uri
    // 2. mints a poap to the receiver

    // ========= 1. change token uri
    let callback_data: CallbackData = from_json(msg.msg)?;
    let (update_nft_info, old_token_uri, new_token_uri) = create_update_nft_info_msg(
        deps.as_ref(),
        msg.nft_contract,
        callback_data.clone(),
        false,
    )?;
    // ========= 2. mint poap
    let poap = ADDR_POAP.load(deps.storage)?;
    let mint_poap = create_mint_msg(deps, poap, msg.original_packet.receiver)?;

    Ok(Response::default()
        .add_attribute("method", "execute_receive_callback")
        .add_attribute("token_id", callback_data.token_id)
        .add_attribute("sender", callback_data.sender)
        .add_message(update_nft_info)
        .add_submessage(mint_poap)
        .add_attribute("new_token_uri", new_token_uri)
        .add_attribute("old_token_uri", old_token_uri.clone())
        .add_attribute("default_token_uri", callback_data.default_token_uri.clone())
        .add_attribute(
            "escrowed_token_uri",
            callback_data.escrowed_token_uri.clone(),
        )
        .add_attribute(
            "transferred_token_uri",
            callback_data.transferred_token_uri.clone(),
        ))
}

/// Updates NftInfo with new token uri on both, source (ack) and target (receive) chain.
/// This is executed as a message (not sub message) allowing global TX to succeed and not to roll back, for 2 reasons:
/// - back transfer/on ack: NFT is burned and may error and this is fine
/// - on initial transfer: ics721 is creator of voucher collection on target chain. So this contract cant update NFT Info.
///
/// In future releases of ics721 this may change, allowing to pass creator of voucher collection to ics721.
fn create_update_nft_info_msg(
    deps: Deps,
    cw721: String,
    callback_data: CallbackData,
    use_escrowed_uri: bool,
) -> Result<(WasmMsg, String, String), ContractError> {
    // check if token uri is unchanged ( holds default token uri)
    let nft_info: NftInfoResponse<DefaultOptionalNftExtension> = deps.querier.query_wasm_smart(
        cw721.clone(),
        &cw721_base::msg::QueryMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::NftInfo {
            token_id: callback_data.token_id.clone(),
        },
    )?;
    // currently ics721 does not store onchain metadata, so source for URIs are:
    // - forward transfer: URIs in callback
    // - back transfer: URIs in nft extension/onchain metadata
    let (default_token_uri, escrowed_token_uri, transferred_token_uri) =
        if let Some(extension) = nft_info.extension {
            if let Some(attributes) = extension.attributes {
                let default_token_uri = attributes
                    .clone()
                    .into_iter()
                    .find(|attribute| attribute.trait_type == "default_uri")
                    .map(|a| a.value)
                    .unwrap_or(callback_data.default_token_uri.clone());
                let escrowed_token_uri = attributes
                    .clone()
                    .into_iter()
                    .find(|attribute| attribute.trait_type == "escrowed_uri")
                    .map(|a| a.value)
                    .unwrap_or(callback_data.escrowed_token_uri.clone());
                let transferred_token_uri = attributes
                    .into_iter()
                    .find(|attribute| attribute.trait_type == "transferred_uri")
                    .map(|a| a.value)
                    .unwrap_or(callback_data.transferred_token_uri.clone());
                (default_token_uri, escrowed_token_uri, transferred_token_uri)
            } else {
                (
                    callback_data.default_token_uri.clone(),
                    callback_data.escrowed_token_uri.clone(),
                    callback_data.transferred_token_uri.clone(),
                )
            }
        } else {
            (
                callback_data.default_token_uri.clone(),
                callback_data.escrowed_token_uri.clone(),
                callback_data.transferred_token_uri.clone(),
            )
        };
    let current_token_uri = nft_info.token_uri.unwrap(); // safe to unwrap, since it is set in mint
    let new_token_uri = if current_token_uri == default_token_uri {
        if use_escrowed_uri {
            escrowed_token_uri.clone()
        } else {
            transferred_token_uri.clone()
        }
    } else {
        default_token_uri.clone()
    };
    let trait_token_uri = Trait {
        display_type: None,
        trait_type: "token_uri".to_string(),
        value: new_token_uri.clone(),
    };
    let trait_default_uri = Trait {
        display_type: None,
        trait_type: "default_uri".to_string(),
        value: default_token_uri.clone(),
    };
    let trait_escrowed_uri = Trait {
        display_type: None,
        trait_type: "escrowed_uri".to_string(),
        value: escrowed_token_uri.clone(),
    };
    let trait_transferred_uri = Trait {
        display_type: None,
        trait_type: "transferred_uri".to_string(),
        value: transferred_token_uri.clone(),
    };
    let extension = Some(NftExtensionMsg {
        image: Some(Some(new_token_uri.clone())),
        attributes: Some(Some(vec![
            trait_token_uri,
            trait_default_uri,
            trait_escrowed_uri,
            trait_transferred_uri,
        ])),
        ..Default::default()
    });
    // - set new token uri
    let update_nft_info: WasmMsg = WasmMsg::Execute {
        contract_addr: cw721,
        msg: to_json_binary(&cw721_base::msg::ExecuteMsg::<
            DefaultOptionalNftExtensionMsg,
            DefaultOptionalCollectionExtensionMsg,
            Empty,
        >::UpdateNftInfo {
            token_id: callback_data.token_id.clone(),
            token_uri: Some(Some(new_token_uri.clone())),
            extension,
        })?,
        funds: vec![],
    };
    Ok((update_nft_info, current_token_uri, new_token_uri))
}

fn execute_ack_callback(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: Ics721AckCallbackMsg,
) -> Result<Response, ContractError> {
    // only ics721 can execute callback
    let ics721 = ADDR_ICS721.load(deps.storage)?;
    if info.sender != ics721 {
        return Err(ContractError::UnauthorizedCallback {});
    }

    let callback_data: CallbackData = from_json(&msg.msg)?;

    let res = Response::default()
        .add_attribute("method", "execute_ack_callback")
        .add_attribute("default_token_uri", callback_data.default_token_uri.clone())
        .add_attribute(
            "escrowed_token_uri",
            callback_data.escrowed_token_uri.clone(),
        )
        .add_attribute(
            "transferred_token_uri",
            callback_data.transferred_token_uri.clone(),
        )
        .add_attribute("token_id", callback_data.token_id.clone())
        .add_attribute("sender", callback_data.sender.clone());
    match msg.status {
        Ics721Status::Success => {
            let (update_nft_info, old_token_uri, new_token_uri) = create_update_nft_info_msg(
                deps.as_ref(),
                msg.nft_contract,
                callback_data.clone(),
                true,
            )?;
            Ok(res
                .add_message(update_nft_info)
                .add_attribute("old_token_uri", old_token_uri)
                .add_attribute("new_token_uri", new_token_uri))
        }
        Ics721Status::Failed(error) => Ok(res.add_attribute("error", error)),
    }
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
        QueryMsg::DefaultTokenUri {} => to_json_binary(&DEFAULT_TOKEN_URI.load(deps.storage)?),
        QueryMsg::EscrowedTokenUri {} => to_json_binary(&ESCROWED_TOKEN_URI.load(deps.storage)?),
        QueryMsg::TransferredTokenUri {} => {
            to_json_binary(&TRANSFERRED_TOKEN_URI.load(deps.storage)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let response: Response<Empty> = Response::default()
        .add_attribute("method", "migrate")
        .add_attribute("contract_name", CONTRACT_NAME)
        .add_attribute("contract_version", CONTRACT_VERSION);
    match msg {
        MigrateMsg::WithUpdate {
            default_token_uri,
            escrowed_token_uri,
            transferred_token_uri,
        } => {
            let response = if let Some(token_uri) = default_token_uri {
                DEFAULT_TOKEN_URI.save(deps.storage, &token_uri)?;
                response.add_attribute("default_token_uri", token_uri)
            } else {
                response
            };
            let response = if let Some(token_uri) = escrowed_token_uri {
                ESCROWED_TOKEN_URI.save(deps.storage, &token_uri)?;
                response.add_attribute("escrowed_token_uri", token_uri)
            } else {
                response
            };
            let response = if let Some(token_uri) = transferred_token_uri {
                TRANSFERRED_TOKEN_URI.save(deps.storage, &token_uri)?;
                response.add_attribute("transferred_token_uri", token_uri)
            } else {
                response
            };
            Ok(response)
        }
    }
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
            Ok(response.add_attribute("addr_poap", poap))
        }
        INSTANTIATE_CW721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let cw721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_CW721.save(deps.storage, &cw721)?;
            Ok(response.add_attribute("addr_cw721", cw721))
        }
        INSTANTIATE_ICS721_REPLY_ID => {
            let res = parse_reply_instantiate_data(reply)?;
            let ics721 = deps.api.addr_validate(&res.contract_address)?;
            ADDR_ICS721.save(deps.storage, &ics721)?;
            Ok(response.add_attribute("addr_ics721", ics721))
        }
        MINT_NFT_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(response),
            SubMsgResult::Err(error) => Err(ContractError::MintFailed { error }),
        },
        _ => Err(ContractError::UnrecognisedReplyId {}),
    }
}
