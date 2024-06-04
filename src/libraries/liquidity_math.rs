use super::big_num::U128;
use super::big_num::U256;
use super::fixed_point_64;
use super::full_math::MulDiv;
use super::tick_math;
use super::unsafe_math::UnsafeMathTrait;
use crate::error::ContractError;
// use anchor_lang::prelude::*;

/// Add a signed liquidity delta to liquidity and revert if it overflows or underflows
///
/// # Arguments
///
/// * `x` - The liquidity (L) before change
/// * `y` - The delta (ΔL) by which liquidity should be changed
///
pub fn add_delta(x: u128, y: i128) -> Result<u128, ContractError> {
    let z: u128;
    if y < 0 {
        z = x - u128::try_from(-y).unwrap();
        if x <= z {
            return Err(ContractError::LiquiditySubValueErr);
        }
    } else {
        z = x + u128::try_from(y).unwrap();
        if z < x {
            return Err(ContractError::LiquidityAddValueErr);
        }
    }

    Ok(z)
}

/// Computes the amount of liquidity received for a given amount of token_0 and price range
/// Calculates ΔL = Δx (√P_upper x √P_lower)/(√P_upper - √P_lower)
pub fn get_liquidity_from_amount_0(
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    amount_0: u64,
) -> u128 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };
    let intermediate = U128::from(sqrt_ratio_a_x64)
        .mul_div_floor(
            U128::from(sqrt_ratio_b_x64),
            U128::from(fixed_point_64::Q64),
        )
        .unwrap();

    U128::from(amount_0)
        .mul_div_floor(
            intermediate,
            U128::from(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
        )
        .unwrap()
        .as_u128()
}

/// Computes the amount of liquidity received for a given amount of token_1 and price range
/// Calculates ΔL = Δy / (√P_upper - √P_lower)
pub fn get_liquidity_from_amount_1(
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    amount_1: u64,
) -> u128 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    U128::from(amount_1)
        .mul_div_floor(
            U128::from(fixed_point_64::Q64),
            U128::from(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
        )
        .unwrap()
        .as_u128()
}

/// Computes the maximum amount of liquidity received for a given amount of token_0, token_1, the current
/// pool prices and the prices at the tick boundaries
pub fn get_liquidity_from_amounts(
    sqrt_ratio_x64: u128,
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    amount_0: u64,
    amount_1: u64,
) -> u128 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    if sqrt_ratio_x64 <= sqrt_ratio_a_x64 {
        // If P ≤ P_lower, only token_0 liquidity is active
        get_liquidity_from_amount_0(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount_0)
    } else if sqrt_ratio_x64 < sqrt_ratio_b_x64 {
        // If P_lower < P < P_upper, active liquidity is the minimum of the liquidity provided
        // by token_0 and token_1
        u128::min(
            get_liquidity_from_amount_0(sqrt_ratio_x64, sqrt_ratio_b_x64, amount_0),
            get_liquidity_from_amount_1(sqrt_ratio_a_x64, sqrt_ratio_x64, amount_1),
        )
    } else {
        // If P ≥ P_upper, only token_1 liquidity is active
        get_liquidity_from_amount_1(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount_1)
    }
}

/// Computes the maximum amount of liquidity received for a given amount of token_0, token_1, the current
/// pool prices and the prices at the tick boundaries
pub fn get_liquidity_from_single_amount_0(
    sqrt_ratio_x64: u128,
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    amount_0: u64,
) -> u128 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    if sqrt_ratio_x64 <= sqrt_ratio_a_x64 {
        // If P ≤ P_lower, only token_0 liquidity is active
        get_liquidity_from_amount_0(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount_0)
    } else if sqrt_ratio_x64 < sqrt_ratio_b_x64 {
        // If P_lower < P < P_upper, active liquidity is the minimum of the liquidity provided
        // by token_0 and token_1
        get_liquidity_from_amount_0(sqrt_ratio_x64, sqrt_ratio_b_x64, amount_0)
    } else {
        // If P ≥ P_upper, only token_1 liquidity is active
        0
    }
}

/// Computes the maximum amount of liquidity received for a given amount of token_0, token_1, the current
/// pool prices and the prices at the tick boundaries
pub fn get_liquidity_from_single_amount_1(
    sqrt_ratio_x64: u128,
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    amount_1: u64,
) -> u128 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    if sqrt_ratio_x64 <= sqrt_ratio_a_x64 {
        // If P ≤ P_lower, only token_0 liquidity is active
        0
    } else if sqrt_ratio_x64 < sqrt_ratio_b_x64 {
        // If P_lower < P < P_upper, active liquidity is the minimum of the liquidity provided
        // by token_0 and token_1
        get_liquidity_from_amount_1(sqrt_ratio_a_x64, sqrt_ratio_x64, amount_1)
    } else {
        // If P ≥ P_upper, only token_1 liquidity is active
        get_liquidity_from_amount_1(sqrt_ratio_a_x64, sqrt_ratio_b_x64, amount_1)
    }
}

/// Gets the delta amount_0 for given liquidity and price range
///
/// # Formula
///
/// * `Δx = L * (1 / √P_lower - 1 / √P_upper)`
/// * i.e. `L * (√P_upper - √P_lower) / (√P_upper * √P_lower)`
pub fn get_delta_amount_0_unsigned(
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    liquidity: u128,
    round_up: bool,
) -> u64 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    let numerator_1 = U256::from(liquidity) << fixed_point_64::RESOLUTION;
    let numerator_2 = U256::from(sqrt_ratio_b_x64 - sqrt_ratio_a_x64);

    assert!(sqrt_ratio_a_x64 > 0);

    if round_up {
        U256::div_rounding_up(
            numerator_1
                .mul_div_ceil(numerator_2, U256::from(sqrt_ratio_b_x64))
                .unwrap(),
            U256::from(sqrt_ratio_a_x64),
        )
        .as_u64()
    } else {
        (numerator_1
            .mul_div_floor(numerator_2, U256::from(sqrt_ratio_b_x64))
            .unwrap()
            / U256::from(sqrt_ratio_a_x64))
        .as_u64()
    }
}

/// Gets the delta amount_1 for given liquidity and price range
/// * `Δy = L (√P_upper - √P_lower)`
pub fn get_delta_amount_1_unsigned(
    mut sqrt_ratio_a_x64: u128,
    mut sqrt_ratio_b_x64: u128,
    liquidity: u128,
    round_up: bool,
) -> u64 {
    // sqrt_ratio_a_x64 should hold the smaller value
    if sqrt_ratio_a_x64 > sqrt_ratio_b_x64 {
        std::mem::swap(&mut sqrt_ratio_a_x64, &mut sqrt_ratio_b_x64);
    };

    if round_up {
        U256::from(liquidity).mul_div_ceil(
            U256::from(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
            U256::from(fixed_point_64::Q64),
        )
    } else {
        U256::from(liquidity).mul_div_floor(
            U256::from(sqrt_ratio_b_x64 - sqrt_ratio_a_x64),
            U256::from(fixed_point_64::Q64),
        )
    }
    .unwrap()
    .as_u64()
}

/// Helper function to get signed delta amount_0 for given liquidity and price range
pub fn get_delta_amount_0_signed(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    liquidity: i128,
) -> u64 {
    if liquidity < 0 {
        get_delta_amount_0_unsigned(
            sqrt_ratio_a_x64,
            sqrt_ratio_b_x64,
            u128::try_from(-liquidity).unwrap(),
            false,
        )
    } else {
        get_delta_amount_0_unsigned(
            sqrt_ratio_a_x64,
            sqrt_ratio_b_x64,
            u128::try_from(liquidity).unwrap(),
            true,
        )
    }
}

/// Helper function to get signed delta amount_1 for given liquidity and price range
pub fn get_delta_amount_1_signed(
    sqrt_ratio_a_x64: u128,
    sqrt_ratio_b_x64: u128,
    liquidity: i128,
) -> u64 {
    if liquidity < 0 {
        get_delta_amount_1_unsigned(
            sqrt_ratio_a_x64,
            sqrt_ratio_b_x64,
            u128::try_from(-liquidity).unwrap(),
            false,
        )
    } else {
        get_delta_amount_1_unsigned(
            sqrt_ratio_a_x64,
            sqrt_ratio_b_x64,
            u128::try_from(liquidity).unwrap(),
            true,
        )
    }
}

pub fn get_delta_amounts_signed(
    tick_current: i32,
    sqrt_price_x64_current: u128,
    tick_lower: i32,
    tick_upper: i32,
    liquidity_delta: i128,
) -> Result<(u64, u64), ContractError> {
    let mut amount_0 = 0;
    let mut amount_1 = 0;
    if tick_current < tick_lower {
        amount_0 = get_delta_amount_0_signed(
            tick_math::get_sqrt_price_at_tick(tick_lower)?,
            tick_math::get_sqrt_price_at_tick(tick_upper)?,
            liquidity_delta,
        );
    } else if tick_current < tick_upper {
        amount_0 = get_delta_amount_0_signed(
            sqrt_price_x64_current,
            tick_math::get_sqrt_price_at_tick(tick_upper)?,
            liquidity_delta,
        );
        amount_1 = get_delta_amount_1_signed(
            tick_math::get_sqrt_price_at_tick(tick_lower)?,
            sqrt_price_x64_current,
            liquidity_delta,
        );
    } else {
        amount_1 = get_delta_amount_1_signed(
            tick_math::get_sqrt_price_at_tick(tick_lower)?,
            tick_math::get_sqrt_price_at_tick(tick_upper)?,
            liquidity_delta,
        );
    }
    Ok((amount_0, amount_1))
}

#[cfg(test)]
mod liquidity_math_test {
    use super::*;
    mod get_amounts_delta_signed {
        use crate::libraries::liquidity_math;

        use super::*;

        #[test]
        fn get_amounts_delta_signed_test() {
            let current_tick = -1860;
            let current_price = tick_math::get_sqrt_price_at_tick(current_tick).unwrap();
            let (amount0, amount1) =
                get_delta_amounts_signed(current_tick, current_price, -6960, 4080, 100000).unwrap();
            println!("amount0:{}, amount1:{}", amount0, amount1)
        }

        #[test]
        fn get_liquidity_after_delta_added_test() {
            let liquidity_net = -1000i128;
            let mut liquidity = 1200u128;
            liquidity = liquidity_math::add_delta(liquidity, liquidity_net).unwrap();
            assert!(liquidity == 200u128);
        }
    }
}
