use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/*
    mapping(int24 => Tick.Info) public override ticks; x
    mapping(int16 => uint256) public override tickBitmap; x
    mapping(bytes32 => Position.Info) public override positions; x
    Oracle.Observation[65535] public override observations; x
*/
#[cw_serde]
pub struct PoolConfig {
    // Token_0 address < Token_1 address
    pub token_0: Addr,
    pub token_1: Addr,

    /// The minimum number of ticks between initialized ticks
    pub tick_spacing: u16,
    // Max liquidity per tick
    pub max_liquidity_per_tick: u128,
    // The amounts in and out of swap token_0 and token_1 ???
    // pub swap_in_amount_token_0: u128,
    // pub swap_out_amount_token_1: u128,
    // pub swap_in_amount_token_1: u128,
    // pub swap_out_amount_token_0: u128,
}

#[cw_serde]
pub struct FeeGrowthGlobalX64 {
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
}

#[cw_serde]
pub struct ProtocolFee {
    pub token_0: u128,
    pub token_1: u128,
}

#[cw_serde]
pub struct Slot0 {
    pub sqrt_price_x64: u128,
    pub tick: i32,
    pub observation_index: u16,
    pub observation_cardinality: u16,
    pub observation_cardinality_next: u16,
    pub fee_protocol: u8,
    pub unlocked: bool,
}

/*
struct Info {
        uint128 liquidityGross; x
        int128 liquidityNet; x
        uint256 feeGrowthOutside0X128; x
        uint256 feeGrowthOutside1X128; x
        int56 tickCumulativeOutside; x
        uint160 secondsPerLiquidityOutsideX128; x reward_growths_outside_x64
        uint32 secondsOutside; x
        bool initialized; x
    }
*/
#[cw_serde]
pub struct TickInfo {
    /// Amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left)
    pub liquidity_net: i128,
    /// The total position liquidity that references this tick
    pub liquidity_gross: u128,

    /// Fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    /// only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,

    pub tick_cumulative_outside: i64,

    // ?
    pub seconds_out_side: u32,

    // Reward growth per unit of liquidity like fee, array of Q64.64
    pub reward_growths_outside_x64: u128,

    pub initialized: bool,
}

/*
     struct Info {
        uint128 liquidity; x
        uint256 feeGrowthInside0LastX128; x
        uint256 feeGrowthInside1LastX128; x
        uint128 tokensOwed0; x
        uint128 tokensOwed1; x
    }
*/
#[cw_serde]
pub struct Position {
    pub liquidity: u128,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    pub tokens_owed_0: u128,
    pub tokens_owed_1: u128,
}

/*
    struct Observation {
        uint32 blockTimestamp; x
        int56 tickCumulative; x
        uint160 secondsPerLiquidityCumulativeX128; x
*/
#[cw_serde]
pub struct Observation {
    pub block_timestamp: u32,
    pub tick_cumulative: u128,
    pub seconds_per_liquidity_cumulative_x128: u128,
}

pub const LIQUIDITY: Item<u128> = Item::new("liquidity");
pub const FEE_GROWTH_GLOBAL_X64: Item<FeeGrowthGlobalX64> = Item::new("fee_growth_global_x64");
pub const PROTOCOL_FEE: Item<ProtocolFee> = Item::new("protocol_fee");
pub const SLOT_0: Item<Slot0> = Item::new("slot_0");
pub const POOL_CONFIG: Item<PoolConfig> = Item::new("pool_config");
pub const TICKS: Map<i64, TickInfo> = Map::new("ticks");
pub const TICK_BITMAP: Map<i16, u128> = Map::new("tick_bitmap");
pub const POSITIONS: Map<&str, Position> = Map::new("positions");
pub const OBSERVATIONS: Map<u16, Observation> = Map::new("observations");
