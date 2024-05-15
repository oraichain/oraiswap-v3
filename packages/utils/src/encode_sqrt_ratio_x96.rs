use super::big_int_to_u256;
use alloy_primitives::U256;
use num_bigint::BigInt;
use uniswap_sdk_core::utils::sqrt::sqrt;

/// Returns the sqrt ratio as a Q64.96 corresponding to a given ratio of amount1 and amount0
///
/// ## Arguments
///
/// * `amount1`: The numerator amount i.e., the amount of token1
/// * `amount0`: The denominator amount i.e., the amount of token0
///
/// returns: U256 The sqrt ratio as a Q64.96
pub fn encode_sqrt_ratio_x96(amount1: impl Into<BigInt>, amount0: impl Into<BigInt>) -> U256 {
    let numerator: BigInt = amount1.into() << 192;
    let denominator = amount0.into();
    big_int_to_u256(sqrt(&(numerator / denominator)).unwrap())
}
