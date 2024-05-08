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

    #[error("Failed to mint NFT: {error}")]
    MintFailed { error: String },

    #[error("Unauthorized callback. Only ICS721 can call back.")]
    UnauthorizedCallback {},
}
