//! ## Bit Math Library in Rust
//!
//! This module is a Rust port of the Solidity [BitMath library](https://github.com/uniswap/v3-core/blob/main/contracts/libraries/BitMath.sol).

use alloy_primitives::U256;

pub fn most_significant_bit(x: U256) -> u8 {
    if x.is_zero() {
        panic!("ZERO")
    }
    255 - x.leading_zeros() as u8
}

pub fn least_significant_bit(x: U256) -> u8 {
    if x.is_zero() {
        panic!("ZERO")
    }
    x.trailing_zeros() as u8
}
