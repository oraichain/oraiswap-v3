use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

use super::{tick_math, REWARD_NUM};

pub const TICK_ARRAY_SIZE_USIZE: usize = 60;
pub const TICK_ARRAY_SIZE: i32 = 60;

pub struct TickArrayState {
    pub pool_id: Addr,
    pub start_tick_index: i32,
    pub ticks: [TickState; TICK_ARRAY_SIZE_USIZE],
    pub initialized_tick_count: u8,
    // Unused bytes for future upgrades.
    pub padding: [u8; 115],
}

impl TickArrayState {
    /// Input an arbitrary tick_index, output the start_index of the tick_array it sits on
    pub fn get_array_start_index(tick_index: i32, tick_spacing: u16) -> i32 {
        let ticks_in_array = TickArrayState::tick_count(tick_spacing);
        let mut start = tick_index / ticks_in_array;
        if tick_index < 0 && tick_index % ticks_in_array != 0 {
            start = start - 1
        }
        start * ticks_in_array
    }

    pub fn check_is_valid_start_index(tick_index: i32, tick_spacing: u16) -> bool {
        if TickState::check_is_out_of_boundary(tick_index) {
            if tick_index > tick_math::MAX_TICK {
                return false;
            }
            let min_start_index =
                TickArrayState::get_array_start_index(tick_math::MIN_TICK, tick_spacing);
            return tick_index == min_start_index;
        }
        tick_index % TickArrayState::tick_count(tick_spacing) == 0
    }

    pub fn tick_count(tick_spacing: u16) -> i32 {
        TICK_ARRAY_SIZE * i32::from(tick_spacing)
    }
}

#[cw_serde]
pub struct TickState {
    pub tick: i32,
    /// Amount of net liquidity added (subtracted) when tick is crossed from left to right (right to left)
    pub liquidity_net: i128,
    /// The total position liquidity that references this tick
    pub liquidity_gross: u128,

    /// Fee growth per unit of liquidity on the _other_ side of this tick (relative to the current tick)
    /// only has relative meaning, not absolute — the value depends on when the tick is initialized
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,

    // Reward growth per unit of liquidity like fee, array of Q64.64
    pub reward_growths_outside_x64: [u128; REWARD_NUM],
    // Unused bytes for future upgrades.
    pub padding: [u32; 13],
}

impl TickState {
    pub fn check_is_out_of_boundary(tick: i32) -> bool {
        tick < tick_math::MIN_TICK || tick > tick_math::MAX_TICK
    }
}
