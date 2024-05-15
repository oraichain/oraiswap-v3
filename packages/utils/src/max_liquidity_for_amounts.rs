use super::u256_to_big_uint;
use alloy_primitives::U256;
use num_bigint::BigUint;

/// Returns an imprecise maximum amount of liquidity received for a given amount of token 0.
/// This function is available to accommodate LiquidityAmounts#getLiquidityForAmount0 in the v3
/// periphery, which could be more precise by at least 32 bits by dividing by Q64 instead of Q96 in
/// the intermediate step, and shifting the subtracted ratio left by 32 bits. This imprecise
/// calculation will likely be replaced in a future v3 router contract.
///
/// ## Arguments
///
/// * `sqrt_ratio_a_x96`: The price at the lower boundary
/// * `sqrt_ratio_b_x96`: The price at the upper boundary
/// * `amount0`: The token0 amount
///
/// returns: liquidity for amount0, imprecise
pub fn max_liquidity_for_amount0_imprecise(
    mut sqrt_ratio_a_x96: U256,
    mut sqrt_ratio_b_x96: U256,
    amount0: U256,
) -> BigUint {
    if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
        (sqrt_ratio_a_x96, sqrt_ratio_b_x96) = (sqrt_ratio_b_x96, sqrt_ratio_a_x96);
    }
    let sqrt_ratio_a_x96 = u256_to_big_uint(sqrt_ratio_a_x96);
    let sqrt_ratio_b_x96 = u256_to_big_uint(sqrt_ratio_b_x96);

    let intermediate = (&sqrt_ratio_a_x96 * &sqrt_ratio_b_x96) >> 96;
    u256_to_big_uint(amount0) * intermediate / (sqrt_ratio_b_x96 - sqrt_ratio_a_x96)
}

/// Returns a precise maximum amount of liquidity received for a given amount of token 0 by dividing
/// by Q64 instead of Q96 in the intermediate step, and shifting the subtracted ratio left by 32
/// bits.
///
/// ## Arguments
///
/// * `sqrt_ratio_a_x96`: The price at the lower boundary
/// * `sqrt_ratio_b_x96`: The price at the upper boundary
/// * `amount0`: The token0 amount
///
/// returns: liquidity for amount0, precise
pub fn max_liquidity_for_amount0_precise(
    mut sqrt_ratio_a_x96: U256,
    mut sqrt_ratio_b_x96: U256,
    amount0: U256,
) -> BigUint {
    if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
        (sqrt_ratio_a_x96, sqrt_ratio_b_x96) = (sqrt_ratio_b_x96, sqrt_ratio_a_x96);
    }
    let sqrt_ratio_a_x96 = u256_to_big_uint(sqrt_ratio_a_x96);
    let sqrt_ratio_b_x96 = u256_to_big_uint(sqrt_ratio_b_x96);

    let numerator = u256_to_big_uint(amount0) * &sqrt_ratio_a_x96 * &sqrt_ratio_b_x96;
    let denominator = (sqrt_ratio_b_x96 - sqrt_ratio_a_x96) << 96;

    numerator / denominator
}

/// Computes the maximum amount of liquidity received for a given amount of token1
///
/// ## Arguments
///
/// * `sqrt_ratio_a_x96`: The price at the lower boundary
/// * `sqrt_ratio_b_x96`: The price at the upper boundary
/// * `amount1`: The token1 amount
///
/// returns: liquidity for amount1
pub fn max_liquidity_for_amount1(
    mut sqrt_ratio_a_x96: U256,
    mut sqrt_ratio_b_x96: U256,
    amount1: U256,
) -> BigUint {
    if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
        (sqrt_ratio_a_x96, sqrt_ratio_b_x96) = (sqrt_ratio_b_x96, sqrt_ratio_a_x96);
    }
    let sqrt_ratio_a_x96 = u256_to_big_uint(sqrt_ratio_a_x96);
    let sqrt_ratio_b_x96 = u256_to_big_uint(sqrt_ratio_b_x96);

    (u256_to_big_uint(amount1) << 96) / (sqrt_ratio_b_x96 - sqrt_ratio_a_x96)
}

/// Computes the maximum amount of liquidity received for a given amount of token0, token1,
/// and the prices at the tick boundaries.
///
/// ## Arguments
///
/// * `sqrt_ratio_current_x96`: The current price
/// * `sqrt_ratio_a_x96`: The price at the lower boundary
/// * `sqrt_ratio_b_x96`: The price at the upper boundary
/// * `amount0`: The token0 amount
/// * `amount1`: The token1 amount
/// * `use_full_precision`: if false, liquidity will be maximized according to what the router can
///   calculate, not what core can theoretically support
///
/// returns: maximum liquidity for the given amounts
pub fn max_liquidity_for_amounts(
    sqrt_ratio_current_x96: U256,
    mut sqrt_ratio_a_x96: U256,
    mut sqrt_ratio_b_x96: U256,
    amount0: U256,
    amount1: U256,
    use_full_precision: bool,
) -> BigUint {
    if sqrt_ratio_a_x96 > sqrt_ratio_b_x96 {
        (sqrt_ratio_a_x96, sqrt_ratio_b_x96) = (sqrt_ratio_b_x96, sqrt_ratio_a_x96);
    }

    if sqrt_ratio_current_x96 <= sqrt_ratio_a_x96 {
        if use_full_precision {
            max_liquidity_for_amount0_precise(sqrt_ratio_a_x96, sqrt_ratio_b_x96, amount0)
        } else {
            max_liquidity_for_amount0_imprecise(sqrt_ratio_a_x96, sqrt_ratio_b_x96, amount0)
        }
    } else if sqrt_ratio_current_x96 < sqrt_ratio_b_x96 {
        let liquidity0 = if use_full_precision {
            max_liquidity_for_amount0_precise(sqrt_ratio_current_x96, sqrt_ratio_b_x96, amount0)
        } else {
            max_liquidity_for_amount0_imprecise(sqrt_ratio_current_x96, sqrt_ratio_b_x96, amount0)
        };
        let liquidity1 =
            max_liquidity_for_amount1(sqrt_ratio_a_x96, sqrt_ratio_current_x96, amount1);

        if liquidity0 < liquidity1 {
            liquidity0
        } else {
            liquidity1
        }
    } else {
        max_liquidity_for_amount1(sqrt_ratio_a_x96, sqrt_ratio_b_x96, amount1)
    }
}
