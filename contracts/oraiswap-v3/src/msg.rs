use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::{interface::SwapHop, liquidity::Liquidity, percentage::Percentage, sqrt_price::SqrtPrice, token_amount::TokenAmount, PoolKey};

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
    Swap {
        pool_key: PoolKey,
        x_to_y: bool,
        amount: TokenAmount,
        by_amount_in: bool,
        sqrt_price_limit: SqrtPrice,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Percentage)]
    ProtocolFee {},

    #[returns(TokenAmount)]
    QuoteRoute {
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    }
}
