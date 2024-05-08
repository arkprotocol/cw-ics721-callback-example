use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const DEFAULT_TOKEN_URI: Item<String> = Item::new("token_uri");
pub const ESCROWED_TOKEN_URI: Item<String> = Item::new("escrowed_token_uri");
pub const TRANSFERRED_TOKEN_URI: Item<String> = Item::new("transferred_token_uri");
pub const ADDR_CW721: Item<Addr> = Item::new("addr_cw721");
pub const ADDR_ICS721: Item<Addr> = Item::new("addr_ics721");
pub const ADDR_POAP: Item<Addr> = Item::new("addr_poap");
pub const COUNTERPARTY_CONTRACT: Item<String> = Item::new("counterpart_contract");
