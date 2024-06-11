pub mod error;
pub mod execute;
pub mod msg;
pub mod state;

pub(crate) const INSTANTIATE_CW721_REPLY_ID: u64 = 0;
pub(crate) const INSTANTIATE_POAP_REPLY_ID: u64 = 1;
pub(crate) const INSTANTIATE_ICS721_REPLY_ID: u64 = 2;
pub(crate) const MINT_NFT_REPLY_ID: u64 = 3;
pub(crate) const UPDATE_NFT_REPLY_ID: u64 = 4;

#[cfg(test)]
pub mod testing;
