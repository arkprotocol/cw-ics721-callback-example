use cosmwasm_schema::write_api;

use cw_ics721_arkite_passport::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg,
        migrate: MigrateMsg,
    }
}
