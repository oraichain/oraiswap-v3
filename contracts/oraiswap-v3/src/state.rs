use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::AmmConfig;

// TODO: get state from storage
pub const AMM_CONFIG: Item<AmmConfig> = Item::new("amm_config");
