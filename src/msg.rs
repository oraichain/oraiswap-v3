use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::{FeeGrowthGlobalX64, PoolConfig, Position, ProtocolFee, Slot0, TickInfo};

#[cw_serde]
pub struct InstantiateMsg {
    token_0: String,
    token_1: String,
    fee: u8,
    tick_spacing: u16,
}

#[cw_serde]
pub enum ExecuteMsg {
    Mint {
        recipient: Addr,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    },
    Collect {
        recipient: Addr,
        tick_lower: i32,
        tick_upper: i32,
        amount_0_requested: u128,
        amount_1_requested: u128,
    },
    Burn {
        tick_lower: i32,
        tick_upper: i32,
        liquidity: u128,
    },
    Swap {
        recipient: Addr,
        zero_for_one: bool,
        amount_in: u128,
        sqrt_price_limit_x64: u128,
    },
    CollectProtocol {
        recipient: Addr,
        amount_0_requested: u128,
        amount_1_requested: u128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PoolConfig)]
    PoolConfig {},

    #[returns(u8)]
    Fee {},

    #[returns(Slot0)]
    Slot0 {},

    #[returns(FeeGrowthGlobalX64)]
    FeeGrowthGlobalX64 {},

    #[returns(ProtocolFee)]
    ProtocolFee {},

    #[returns(u128)]
    Liquidity {},

    #[returns(TickInfo)]
    Ticks {
        tick: i32,
    },

    #[returns(u128)]
    TickBitmap {
        word_position: i16
    },

    #[returns(Position)]
    Position {
        key: String,
    }
}

