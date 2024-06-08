use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::{math::types::percentage::Percentage, FeeTier};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_tiers: Vec<FeeTier>,
    pub protocol_fee: Percentage,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fee_tiers: vec![],
            admin: Addr::unchecked(""),
            protocol_fee: Default::default(),
        }
    }
}
