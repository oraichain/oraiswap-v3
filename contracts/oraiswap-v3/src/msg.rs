use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[allow(unused_imports)]
use crate::{
    interface::SwapHop, liquidity::Liquidity, percentage::Percentage, sqrt_price::SqrtPrice,
    token_amount::TokenAmount, FeeTier, Pool, PoolKey, Position,
};

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
    QuoteRoute {
        amount_in: TokenAmount,
        swaps: Vec<SwapHop>,
    },
    TransferPosition {
        index: u32,
        receiver: String,
    },
    ClaimFee {
        index: u32,
    },
    RemovePosition {
        index: u32,
    },
    CreatePool {
        token_0: Addr,
        token_1: Addr,
        fee_tier: FeeTier,
        init_sqrt_price: SqrtPrice,
        init_tick: i32,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Percentage)]
    ProtocolFee {},

    #[returns(Position)]
    Position { owner_id: Addr, index: u32 },

    #[returns(Vec<Position>)]
    Positions {
        owner_id: Addr,
        limit: Option<u32>,
        offset: Option<u32>,
    },

    #[returns(bool)]
    FeeTierExist { fee_tier: FeeTier },

    #[returns(Pool)]
    Pool {
        token_0: Addr,
        token_1: Addr,
        fee_tier: FeeTier,
    },
}
