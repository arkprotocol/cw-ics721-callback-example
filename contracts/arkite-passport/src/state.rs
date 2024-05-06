use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const ADDR_CW721_BASE: Item<Addr> = Item::new("addr_cw721_base");
