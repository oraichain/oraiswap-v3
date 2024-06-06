use cw_storage_plus::{Item, Map};

use crate::{AmmConfig, Pool};

// TODO: get state from storage
pub const AMM_CONFIG: Item<AmmConfig> = Item::new("amm_config");

pub const POOLS: Map<&[u8], Pool> = Map::new("pools");
