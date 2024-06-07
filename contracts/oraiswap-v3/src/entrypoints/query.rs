use cosmwasm_std::{Deps, Env, StdResult};
use decimal::Decimal;

use crate::{
    interface::SwapHop, percentage::Percentage, sqrt_price::SqrtPrice, state::CONFIG,
    token_amount::TokenAmount, MAX_SQRT_PRICE, MIN_SQRT_PRICE,
};

use super::calculate_swap;

/// Retrieves the protocol fee represented as a percentage.
pub fn get_protocol_fee(deps: Deps) -> StdResult<Percentage> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}

/// Simulates multiple swaps without its execution.
///
/// # Parameters
/// - `amount_in`: The amount of tokens that the user wants to swap.
/// - `swaps`: A vector containing all parameters needed to identify separate swap steps.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the user would receive zero tokens.
/// - Fails if pool does not exist
pub fn quote_route(
    deps: Deps,
    env: Env,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> StdResult<TokenAmount> {
    let mut next_swap_amount = amount_in;

    for swap_hop in swaps.iter() {
        let SwapHop { pool_key, x_to_y } = swap_hop;

        let sqrt_price_limit = if *x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        let res = calculate_swap(
            deps.storage,
            env.block.time.nanos(),
            pool_key.clone(),
            *x_to_y,
            next_swap_amount,
            true,
            sqrt_price_limit,
        )
        .unwrap();

        next_swap_amount = res.amount_out;
    }

    Ok(next_swap_amount)
}
