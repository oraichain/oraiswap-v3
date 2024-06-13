use cosmwasm_schema::cw_serde;
use decimal::*;

#[decimal(6)]
#[cw_serde]
#[derive(Default, Eq, Copy, PartialOrd)]
pub struct Liquidity(#[schemars(with = "String")] pub u128);
