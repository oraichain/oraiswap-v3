use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::math::types::percentage::Percentage;

#[cw_serde]
pub struct AmmConfig {
    pub admin: Addr,
    pub protocol_fee: Percentage,
}

impl Default for AmmConfig {
    fn default() -> Self {
        Self {
            admin: Addr::unchecked(""),
            protocol_fee: Default::default(),
        }
    }
}
