use cosmwasm_schema::cw_serde;
use decimal::*;

#[decimal(12)]
#[cw_serde]
#[derive(Default, Eq, Copy)]
pub struct FixedPoint(pub u128);
