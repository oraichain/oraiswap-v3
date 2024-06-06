use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::math::types::percentage::Percentage;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub protocol_fee: Percentage,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            admin: Addr::unchecked(""),
            protocol_fee: Default::default(),
        }
    }
}
