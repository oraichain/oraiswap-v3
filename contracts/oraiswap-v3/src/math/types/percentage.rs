use cosmwasm_schema::cw_serde;
use decimal::*;

#[decimal(12)]
#[cw_serde]
#[derive(Default, Copy, Eq, PartialOrd, scale::Decode, scale::Encode)]
#[cfg_attr(
    feature = "std",
    derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
)]
pub struct Percentage(pub u64);
