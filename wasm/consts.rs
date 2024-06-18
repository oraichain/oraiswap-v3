use crate::types::sqrt_price::get_max_tick;
use js_sys::BigInt;
use wasm_bindgen::prelude::*;

pub const MAX_TICK: i32 = 221_818;
pub const MIN_TICK: i32 = -MAX_TICK;

pub const MAX_SQRT_PRICE: u128 = 65535383934512647000000000000;
pub const MIN_SQRT_PRICE: u128 = 15258932000000000000;

pub const TICK_SEARCH_RANGE: i32 = 256;
pub const CHUNK_SIZE: i32 = 64;

pub const MAX_TICK_CROSS: i32 = 173;

pub const MAX_RESULT_SIZE: usize = 16 * 1024 * 8;
pub const MAX_TICKMAP_QUERY_SIZE: usize = MAX_RESULT_SIZE / (16 + 64);

pub const LIQUIDITY_TICK_LIMIT: usize = MAX_RESULT_SIZE / (32 + 128 + 8);

pub const MAX_POOL_KEYS_RETURNED: u16 = 220;

pub const MAX_POOL_PAIRS_RETURNED: usize =
    MAX_RESULT_SIZE / (128 + 128 + 32 + 128 + 128 + 128 + 128 + 64 + 64 + 32 + 64 + 16);

#[wasm_bindgen(js_name = getGlobalMaxSqrtPrice)]
#[allow(non_snake_case)]
pub fn get_global_max_sqrt_price() -> BigInt {
    BigInt::from(MAX_SQRT_PRICE)
}

#[wasm_bindgen(js_name = getGlobalMinSqrtPrice)]
#[allow(non_snake_case)]
pub fn get_global_min_sqrt_price() -> BigInt {
    BigInt::from(MIN_SQRT_PRICE)
}

#[wasm_bindgen(js_name = getTickSearchRange)]
#[allow(non_snake_case)]
pub fn get_tick_search_range() -> i32 {
    TICK_SEARCH_RANGE
}

#[wasm_bindgen(js_name = getMaxChunk)]
#[allow(non_snake_case)]
pub fn get_max_chunk(tick_spacing: u16) -> u16 {
    let max_tick = get_max_tick(tick_spacing);
    let max_bitmap_index = (max_tick + MAX_TICK) / tick_spacing as i32;
    let max_chunk_index = max_bitmap_index / CHUNK_SIZE;
    max_chunk_index as u16
}

#[wasm_bindgen(js_name = getChunkSize)]
#[allow(non_snake_case)]
pub fn get_chunk_size() -> i32 {
    CHUNK_SIZE
}

#[wasm_bindgen(js_name = getMaxTickCross)]
#[allow(non_snake_case)]
pub fn get_max_tick_cross() -> i32 {
    MAX_TICK_CROSS
}

#[wasm_bindgen(js_name = getMaxTickmapQuerySize)]
#[allow(non_snake_case)]
pub fn get_max_tickmap_query_size() -> u32 {
    MAX_TICKMAP_QUERY_SIZE as u32
}

#[wasm_bindgen(js_name = getLiquidityTicksLimit)]
#[allow(non_snake_case)]
pub fn get_liquidity_ticks_limit() -> u32 {
    LIQUIDITY_TICK_LIMIT as u32
}

#[wasm_bindgen(js_name = getMaxPoolKeysReturned)]
#[allow(non_snake_case)]
pub fn get_max_pool_keys_returned() -> u16 {
    MAX_POOL_KEYS_RETURNED
}

#[wasm_bindgen(js_name = getMaxPoolPairsReturned)]
#[allow(non_snake_case)]
pub fn get_max_pool_pairs_returned() -> u32 {
    MAX_POOL_PAIRS_RETURNED as u32
}
