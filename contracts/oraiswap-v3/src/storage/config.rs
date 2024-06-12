use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use crate::{math::types::percentage::Percentage, FeeTier};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub fee_tiers: Vec<FeeTier>,
    pub protocol_fee: Percentage,
}
