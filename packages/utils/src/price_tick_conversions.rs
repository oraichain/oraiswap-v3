//! ## Price and tick conversions
//! Utility functions for converting between [`i32`] ticks and SDK Core [`Price`] prices.

use crate::{
    encode_sqrt_ratio_x96, get_sqrt_ratio_at_tick, get_tick_at_sqrt_ratio, u256_to_big_uint, Q192,
};
use anyhow::Result;
use uniswap_sdk_core::prelude::*;

/// Returns a price object corresponding to the input tick and the base/quote token.
/// Inputs must be tokens because the address order is used to interpret the price represented by
/// the tick.
///
/// ## Arguments
///
/// * `base_token`: the base token of the price
/// * `quote_token`: the quote token of the price
/// * `tick`: the tick for which to return the price
pub fn tick_to_price(
    base_token: Token,
    quote_token: Token,
    tick: i32,
) -> Result<Price<Token, Token>> {
    let sqrt_ratio_x96 = get_sqrt_ratio_at_tick(tick)?;
    let ratio_x192 = u256_to_big_uint(sqrt_ratio_x96).pow(2);
    let q192 = u256_to_big_uint(Q192);
    Ok(if base_token.sorts_before(&quote_token)? {
        Price::new(base_token, quote_token, q192, ratio_x192)
    } else {
        Price::new(base_token, quote_token, ratio_x192, q192)
    })
}

/// Returns the first tick for which the given price is greater than or equal to the tick price
///
/// ## Arguments
///
/// * `price`: for which to return the closest tick that represents a price less than or equal to
/// the input price, i.e. the price of the returned tick is less than or equal to the input price
pub fn price_to_closest_tick(price: &Price<Token, Token>) -> Result<i32> {
    let sorted = price.base_currency.sorts_before(&price.quote_currency)?;
    let sqrt_ratio_x96 = if sorted {
        encode_sqrt_ratio_x96(price.numerator(), price.denominator())
    } else {
        encode_sqrt_ratio_x96(price.denominator(), price.numerator())
    };
    let tick = get_tick_at_sqrt_ratio(sqrt_ratio_x96)?;
    let next_tick_price = tick_to_price(
        price.base_currency.clone(),
        price.quote_currency.clone(),
        tick + 1,
    )?;
    Ok(if sorted {
        if price >= &next_tick_price {
            tick + 1
        } else {
            tick
        }
    } else if price <= &next_tick_price {
        tick + 1
    } else {
        tick
    })
}
