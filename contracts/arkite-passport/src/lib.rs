pub mod error;
pub mod execute;
pub mod msg;
pub mod query;
pub mod state;

/// Submessage reply ID used for instantiating cw721 contracts.
pub(crate) const INSTANTIATE_CW721_REPLY_ID: u64 = 0;
/// Submessage reply ID used for instantiating the proxy contract.
pub(crate) const INSTANTIATE_INCOMING_PROXY_REPLY_ID: u64 = 1;
/// Submessage reply ID used for instantiating the proxy contract.
pub(crate) const INSTANTIATE_OUTGOING_PROXY_REPLY_ID: u64 = 2;

#[cfg(test)]
pub mod testing;
