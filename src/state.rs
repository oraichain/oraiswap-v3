use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Int256, Uint128, Uint256};
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
    pub fee_growth_global_0_x128: Uint256,
    pub fee_growth_global_1_x128: Uint256,
}

#[cw_serde]
pub struct SwapState {
    pub amount_specified_remaining: Int256,
    pub amount_calculated: Int256,
    pub sqrt_price_x96: Uint256,
    pub tick: i32,
    pub fee_growth_global_x128: Uint256,
    pub liquidity: u128,
}

#[cw_serde]
pub struct StepComputations {
    pub sqrt_price_start_x96: Uint256,
    pub tick_next: i32,
    pub initialized: bool,
    pub sqrt_price_next_x96: Uint256,
    pub amount_in: Uint256,
    pub amount_out: Uint256,
    pub fee_amount: Uint256,
}

#[cw_serde]
pub struct TickInfo {
    // the total position liquidity that references this tick
    pub liquidity_gross: u128,
    // amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left),
    pub liquidity_net: i128,
    // fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    // only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub fee_growth_outside_0_x128: Uint256,
    pub fee_growth_outside_1_x128: Uint256,
    // // the cumulative tick value on the other side of the tick
    // pub tick_cumulative_outside: i32,
    // // the seconds per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    // // only has relative meaning, not absolute — the value depends on when the tick is initialized
    // pub seconds_per_liquidity_outside_x128: Uint256,
    // // the seconds spent on the other side of the tick (relative to the current tick)
    // // only has relative meaning, not absolute — the value depends on when the tick is initialized
    // pub seconds_outside: u32,
}

#[cw_serde]
pub struct Position {
    // the amount of liquidity owned by this position
    pub liquidity: u128,
    // fee growth per unit of liquidity as of the last update to liquidity or fees owed
    pub fee_growth_inside_0_last_x128: Uint256,
    pub fee_growth_inside_1_last_x128: Uint256,
    // the fees owed to the position owner in token0/token1
    pub tokens_owned_0: Uint128,
    pub tokens_owned_1: Uint128,
}

#[cw_serde]
pub struct CurrentState {
    pub liquidity: u128,
    // the current price
    pub sqrt_price_x96: Uint256,
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
