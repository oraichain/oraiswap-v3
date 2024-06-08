use crate::math::sqrt_price::get_max_tick;
use crate::math::MAX_TICK;
use crate::{MAX_RESULT_SIZE, TICK_SEARCH_RANGE};
pub const CHUNK_SIZE: i32 = 64;
pub const MAX_TICKMAP_QUERY_SIZE: usize = MAX_RESULT_SIZE / (16 + 64);

pub fn get_max_chunk(tick_spacing: u16) -> u16 {
    let max_tick = get_max_tick(tick_spacing);
    let max_bitmap_index = (max_tick + MAX_TICK) / tick_spacing as i32;
    let max_chunk_index = max_bitmap_index / CHUNK_SIZE;
    max_chunk_index as u16
}

pub fn get_min_chunk(tick_spacing: u16) -> u16 {
    let min_tick = get_max_tick(tick_spacing);
    let min_bitmap_index = (MAX_TICK - min_tick) / tick_spacing as i32;
    let min_chunk_index = min_bitmap_index / CHUNK_SIZE;
    min_chunk_index as u16
}

pub fn tick_to_position(tick: i32, tick_spacing: u16) -> (u16, u8) {
    assert!(
        (-MAX_TICK..=MAX_TICK).contains(&tick),
        "tick not in range of <{}, {}>",
        -MAX_TICK,
        MAX_TICK
    );

    assert_eq!(
        (tick % tick_spacing as i32),
        0,
        "tick not divisible by tick spacing"
    );

    let bitmap_index = (tick + MAX_TICK) / tick_spacing as i32;

    let chunk: u16 = (bitmap_index / CHUNK_SIZE) as u16;
    let bit: u8 = (bitmap_index % CHUNK_SIZE) as u8;

    (chunk, bit)
}
#[allow(dead_code)]
pub fn position_to_tick(chunk: u16, bit: u8, tick_spacing: u16) -> i32 {
    let tick_range_limit = MAX_TICK - MAX_TICK % tick_spacing as i32;
    (chunk as i32 * CHUNK_SIZE * tick_spacing as i32 + bit as i32 * tick_spacing as i32)
        - tick_range_limit
}

pub fn get_bit_at_position(value: u64, position: u8) -> u64 {
    (value >> position) & 1
}

pub fn flip_bit_at_position(value: u64, position: u8) -> u64 {
    value ^ (1 << position)
}

pub fn get_search_limit(tick: i32, tick_spacing: u16, up: bool) -> i32 {
    let index = tick / tick_spacing as i32;

    // limit unscaled
    let limit = if up {
        // search range is limited to 256 at the time ...
        let range_limit = index + TICK_SEARCH_RANGE;
        // ...also ticks for sqrt_prices over 2^64 aren't needed
        let sqrt_price_limit = MAX_TICK / tick_spacing as i32;

        range_limit.min(sqrt_price_limit)
    } else {
        let range_limit = index - TICK_SEARCH_RANGE;
        let sqrt_price_limit = -MAX_TICK / tick_spacing as i32;

        range_limit.max(sqrt_price_limit)
    };

    // scaled by tick_spacing
    limit * tick_spacing as i32
}
