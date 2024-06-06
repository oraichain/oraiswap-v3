use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::{liquidity::Liquidity, percentage::Percentage, sqrt_price::SqrtPrice, PoolKey};

#[cw_serde]
pub struct InstantiateMsg {
    pub protocol_fee: Percentage,
}

#[cw_serde]
pub enum ExecuteMsg {
    WithdrawProtocolFee {
        pool_key: PoolKey,
    },
    ChangeProtocolFee {
        protocol_fee: Percentage,
    },
    ChangeFeeReceiver {
        pool_key: PoolKey,
        fee_receiver: Addr,
    },
    CreatePosition {
        pool_key: PoolKey,
        lower_tick: i32,
        upper_tick: i32,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Percentage)]
    ProtocolFee {},
}
