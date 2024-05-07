use cosmwasm_std::Addr;
use cw721_base::msg::NftExtensionMsg;
use cw_storage_plus::Item;

pub const NFT_EXTENSION_MSG: Item<NftExtensionMsg> = Item::new("nft_extension");
pub const ADDR_CW721: Item<Addr> = Item::new("addr_cw721");
pub const ADDR_ICS721: Item<Addr> = Item::new("addr_ics721");
pub const SUPPLY: Item<u64> = Item::new("supply");
