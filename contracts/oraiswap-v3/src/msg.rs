use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::{percentage::Percentage, PoolKey};

#[cw_serde]
pub struct InstantiateMsg {
    pub protocol_fee: Percentage,
}

#[cw_serde]
pub enum ExecuteMsg {
    WithdrawProtocolFee { pool_key: PoolKey },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Percentage)]
    ProtocolFee {},
}
