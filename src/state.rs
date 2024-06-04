use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

use crate::interface::AssetInfo;

pub const CONFIG: Item<Config> = Item::new("config");
pub const NFT_INFO: Item<NftInfo> = Item::new("nft_info");
pub const FEE_GROWTH_GROBAL: Item<FeeGrowthGlobal> = Item::new("fee_growth_global");
pub const TICKS: Map<i32, TickInfo> = Map::new("ticks");
pub const LIST_INITIALIZED_TICKS: Item<Vec<i32>> = Item::new("list_initialzed_ticks");
pub const POSITIONS: Map<String, Position> = Map::new("positions");
pub const CURRENT_STATE: Item<CurrentState> = Item::new("current_state");

#[cw_serde]
pub struct Config {
    pub factory: Addr,
    pub token_0: AssetInfo,
    pub token_1: AssetInfo,
    pub fee: u16,
    pub tick_spacing: u32,
    pub max_liquidity_per_tick: Uint128,
}

#[cw_serde]
pub struct NftInfo {
    pub nft_address: Addr,
    pub last_id: u64,
}

#[cw_serde]
pub struct FeeGrowthGlobal {
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
}

#[cw_serde]
pub struct SwapState {
    // the amount remaining to be swapped in/out of the input/output asset
    pub amount_specified_remaining: u64,
    // the amount already swapped out/in of the output/input asset
    pub amount_calculated: u64,
    // current sqrt(price)
    pub sqrt_price_x64: u128,
    // the tick associated with the current price
    pub tick: i32,
    // the global fee growth of the input token
    pub fee_growth_global_x64: u128,

    // // the global fee of the input token
    // pub fee_amount: u64,
    // // amount of input token paid as protocol fee
    // pub protocol_fee: u64,
    // // amount of input token paid as fund fee
    // pub fund_fee: u64,

    // the current liquidity in range
    pub liquidity: u128,
}

#[cw_serde]
pub struct StepComputations {
    // the price at the beginning of the step
    pub sqrt_price_start_x64: u128,
    // the next tick to swap to from the current tick in the swap direction
    pub tick_next: i32,
    // whether tick_next is initialized or not
    pub initialized: bool,
    // sqrt(price) for the next tick (1/0)
    pub sqrt_price_next_x64: u128,
    // how much is being swapped in in this step
    pub amount_in: u64,
    // how much is being swapped out
    pub amount_out: u64,
    // how much fee is being paid in
    pub fee_amount: u64,
}

#[cw_serde]
pub struct TickInfo {
    // the total position liquidity that references this tick
    pub liquidity_gross: u128,
    // amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left),
    pub liquidity_net: i128,
    // fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    // only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    // // the cumulative tick value on the other side of the tick
    // pub tick_cumulative_outside: i32,
    // // the seconds per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    // // only has relative meaning, not absolute — the value depends on when the tick is initialized
    // pub seconds_per_liquidity_outside_x64: u128,
    // // the seconds spent on the other side of the tick (relative to the current tick)
    // // only has relative meaning, not absolute — the value depends on when the tick is initialized
    // pub seconds_outside: u32,
}

#[cw_serde]
pub struct Position {
    // the amount of liquidity owned by this position
    pub liquidity: u128,
    // fee growth per unit of liquidity as of the last update to liquidity or fees owed
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    // the fees owed to the position owner in token0/token1
    pub tokens_owned_0: u64,
    pub tokens_owned_1: u64,
}

#[cw_serde]
pub struct CurrentState {
    pub liquidity: u128,
    // the current price
    pub sqrt_price_x64: u128,
    // the current tick
    pub tick: i32,
    // the most-recently updated index of the observations array
    // pub observation_index: u16,
    // // the current maximum number of observations that are being stored
    // pub observation_cardinality: u16,
    // // the next maximum number of observations to store, triggered in observations.write
    // pub observation_cardinality_next: u16,
    // // the current protocol fee as a percentage of the swap fee taken on withdrawal
    // // represented as an integer denominator (1/x)%
    // pub fee_protocol: u8,
}
