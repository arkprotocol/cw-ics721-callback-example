use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("unrecognised reply ID")]
    UnrecognisedReplyId {},
}
