use cosmwasm_std::{Addr, Deps, Env};

use crate::{
    get_max_chunk, get_min_chunk,
    interface::SwapHop,
    msg::{PoolWithPoolKey, QuoteResult},
    percentage::Percentage,
    sqrt_price::{get_max_tick, get_min_tick, SqrtPrice},
    state::{self, CONFIG},
    tick_to_position,
    token_amount::TokenAmount,
    ContractError, FeeTier, LiquidityTick, Pool, PoolKey, Position, PositionTick, Tick, CHUNK_SIZE,
    LIQUIDITY_TICK_LIMIT, POSITION_TICK_LIMIT,
};

use super::{calculate_swap, route, tickmap_slice};

/// Retrieves the protocol fee represented as a percentage.
pub fn get_protocol_fee(deps: Deps) -> Result<Percentage, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}

/// Retrieves information about a single position.
///
/// # Parameters
/// - `owner_id`: An `Addr` identifying the user who owns the position.
/// - `index`: The index of the user position.
///
/// # Errors
/// - Fails if position cannot be found    
pub fn get_position(deps: Deps, owner_id: Addr, index: u32) -> Result<Position, ContractError> {
    state::get_position(deps.storage, &owner_id, index)
}

// /// Retrieves a vector containing all positions held by the user.
// ///
// /// # Parameters
// /// - `owner_id`: An `Addr` identifying the user who owns the positions.
pub fn get_positions(
    deps: Deps,
    owner_id: Addr,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Position>, ContractError> {
    state::get_all_positions(deps.storage, &owner_id, limit, offset)
}

/// Query of whether the fee tier exists.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
pub fn fee_tier_exist(deps: Deps, fee_tier: FeeTier) -> Result<bool, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.fee_tiers.contains(&fee_tier))
}

/// Retrieves information about a pool created on a specified token pair with an associated fee tier.
///
/// # Parameters
/// - `token_0`: The address of the first token.
/// - `token_1`: The address of the second token.
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if there is no pool associated with created key

pub fn get_pool(
    deps: Deps,
    token_0: Addr,
    token_1: Addr,
    fee_tier: FeeTier,
) -> Result<Pool, ContractError> {
    let pool_key = &PoolKey::new(token_0, token_1, fee_tier)?;
    state::get_pool(deps.storage, pool_key)
}

/// Retrieves information about a tick at a specified index.
///
/// # Parameters
/// - `key`: A unique key that identifies the specified pool.
/// - `index`: The tick index in the tickmap.
///
/// # Errors
/// - Fails if tick cannot be found    
pub fn get_tick(deps: Deps, key: PoolKey, index: i32) -> Result<Tick, ContractError> {
    state::get_tick(deps.storage, &key, index)
}

/// Checks if the tick at a specified index is initialized.
///
/// # Parameters
/// - `key`: A unique key that identifies the specified pool.
/// - `index`: The tick index in the tickmap.
pub fn is_tick_initialized(deps: Deps, key: PoolKey, index: i32) -> Result<bool, ContractError> {
    Ok(state::get_bitmap(
        deps.storage,
        index,
        key.fee_tier.tick_spacing,
        &key,
    ))
}

/// Retrieves listed pools
/// - `size`: Amount of pool keys to retrive
/// - `offset`: The offset from which retrive pools.
pub fn get_pools(
    deps: Deps,
    limit: Option<u32>,
    start_after: Option<PoolKey>,
) -> Result<Vec<PoolWithPoolKey>, ContractError> {
    state::get_pools(deps.storage, limit, start_after)
}

/// Retrieves listed pools for provided token pair
/// - `token0`: Address of first token
/// - `token1`: Address of second token
pub fn get_all_pools_for_pair(
    deps: Deps,
    token0: Addr,
    token1: Addr,
) -> Result<Vec<PoolWithPoolKey>, ContractError> {
    let fee_tiers = get_fee_tiers(deps)?;
    let mut pool_key = PoolKey::new(token0, token1, FeeTier::default())?;
    let mut pools = vec![];
    for fee_tier in fee_tiers {
        pool_key.fee_tier = fee_tier;
        if let Ok(pool) = state::get_pool(deps.storage, &pool_key) {
            pools.push(PoolWithPoolKey {
                pool,
                pool_key: pool_key.clone(),
            });
        }
    }
    Ok(pools)
}

/// Retrieves available fee tiers
pub fn get_fee_tiers(deps: Deps) -> Result<Vec<FeeTier>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.fee_tiers)
}

/// Retrieves list of lower and upper ticks of user positions.
///
/// # Parameters
/// - `owner`: An `Addr` identifying the user who owns the position.
/// - `offset`: The offset from the current position index.
pub fn get_position_ticks(
    deps: Deps,
    owner: Addr,
    offset: u32,
) -> Result<Vec<PositionTick>, ContractError> {
    let positions_length = state::get_position_length(deps.storage, &owner);
    let mut ticks = vec![];

    if offset > positions_length {
        return Err(ContractError::InvalidOffset);
    }

    for i in offset..positions_length {
        state::get_position(deps.storage, &owner, i)
            .map(|position| {
                state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)
                    .map(|tick| {
                        ticks.push(PositionTick {
                            index: tick.index,
                            fee_growth_outside_x: tick.fee_growth_outside_x,
                            fee_growth_outside_y: tick.fee_growth_outside_y,
                            seconds_outside: tick.seconds_outside,
                        })
                    })
                    .ok();

                state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)
                    .map(|tick| {
                        ticks.push(PositionTick {
                            index: tick.index,
                            fee_growth_outside_x: tick.fee_growth_outside_x,
                            fee_growth_outside_y: tick.fee_growth_outside_y,
                            seconds_outside: tick.seconds_outside,
                        })
                    })
                    .ok();
            })
            .ok();

        if ticks.len() >= POSITION_TICK_LIMIT {
            break;
        }
    }

    Ok(ticks)
}

/// Retrieves the amount of positions held by the user.
///
/// # Parameters
/// - `owner`: An `Addr` identifying the user who owns the position.
pub fn get_user_position_amount(deps: Deps, owner: Addr) -> Result<u32, ContractError> {
    Ok(state::get_position_length(deps.storage, &owner))
}

/// Retrieves tickmap chunks
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `lower_tick_index`: offset tick index.
/// - `upper_tick_index`: limiting tick index.
/// - `x_to_y`: direction of the query.
pub fn get_tickmap(
    deps: Deps,
    pool_key: PoolKey,
    lower_tick_index: i32,
    upper_tick_index: i32,
    x_to_y: bool,
) -> Result<Vec<(u16, u64)>, ContractError> {
    let tick_spacing = pool_key.fee_tier.tick_spacing;
    let (start_chunk, _) = tick_to_position(lower_tick_index, tick_spacing);
    let (end_chunk, _) = tick_to_position(upper_tick_index, tick_spacing);

    let min_chunk_index = get_min_chunk(tick_spacing).max(start_chunk);
    let max_chunk_index = get_max_chunk(tick_spacing).min(end_chunk);

    let tickmaps = if x_to_y {
        tickmap_slice(
            deps.storage,
            (min_chunk_index..=max_chunk_index).rev(),
            &pool_key,
        )
    } else {
        tickmap_slice(deps.storage, min_chunk_index..=max_chunk_index, &pool_key)
    };

    Ok(tickmaps)
}

/// Retrieves ticks of a specified pool.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `tick_indexes`: Indexes of the tick to be retrieved.
///
/// # Errors
/// - Fails if tick_indexes are too large
/// - Fails if tick is not found
///
pub fn get_liquidity_ticks(
    deps: Deps,
    pool_key: PoolKey,
    tick_indexes: Vec<i32>,
) -> Result<Vec<LiquidityTick>, ContractError> {
    let mut liqudity_ticks: Vec<LiquidityTick> = vec![];

    if tick_indexes.len() > LIQUIDITY_TICK_LIMIT {
        return Err(ContractError::TickLimitReached);
    }

    for index in tick_indexes {
        let tick = LiquidityTick::from(state::get_tick(deps.storage, &pool_key, index)?);

        liqudity_ticks.push(tick);
    }

    Ok(liqudity_ticks)
}

/// Retrieves the amount of liquidity ticks of a specified pool.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool. For poolkeys with tick_spacing equal to 1 the query has to be split into 2 smaller queries
/// - `lower_tick`: index to start counting from(inclusive)
/// - `upper_tick`: index to stop counting after(inclusive)
///
/// # Errors
/// - Fails if lower_tick or upper_tick are invalid
/// - Fails if tick_spacing is invalid
pub fn get_liquidity_ticks_amount(
    deps: Deps,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
) -> Result<u32, ContractError> {
    let tick_spacing = pool_key.fee_tier.tick_spacing;
    if tick_spacing == 0 {
        return Err(ContractError::InvalidTickSpacing);
    };

    if lower_tick % (tick_spacing as i32) != 0 || upper_tick % (tick_spacing as i32) != 0 {
        return Err(ContractError::InvalidTickIndex);
    }

    let max_tick = get_max_tick(tick_spacing);
    let min_tick = get_min_tick(tick_spacing);

    if lower_tick < min_tick || upper_tick > max_tick {
        return Err(ContractError::InvalidTickIndex);
    };

    let (min_chunk_index, min_bit) = tick_to_position(lower_tick, tick_spacing);
    let (max_chunk_index, max_bit) = tick_to_position(upper_tick, tick_spacing);

    let active_bits_in_range = |chunk, min_bit, max_bit| {
        let range: u64 = (chunk >> min_bit) & ((1u64 << (max_bit - min_bit + 1)) - 1);
        range.count_ones()
    };

    let min_chunk = state::get_bitmap_item(deps.storage, min_chunk_index, &pool_key).unwrap_or(0);

    if max_chunk_index == min_chunk_index {
        return Ok(active_bits_in_range(min_chunk, min_bit, max_bit));
    }

    let max_chunk = state::get_bitmap_item(deps.storage, max_chunk_index, &pool_key).unwrap_or(0);

    let mut amount: u32 = 0;
    amount += active_bits_in_range(min_chunk, min_bit, (CHUNK_SIZE - 1) as u8);
    amount += active_bits_in_range(max_chunk, 0, max_bit);

    for i in (min_chunk_index + 1)..max_chunk_index {
        let chunk = state::get_bitmap_item(deps.storage, i, &pool_key).unwrap_or(0);

        amount += chunk.count_ones();
    }

    Ok(amount)
}

/// Simulates the swap without its execution.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `x_to_y`: A boolean specifying the swap direction.
/// - `amount`: The amount of tokens that the user wants to swap.
/// - `by_amount_in`: A boolean specifying whether the user provides the amount to swap or expects the amount out.
/// - `sqrt_price_limit`: A square root of price limit allowing the price to move for the swap to occur.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the price has reached the specified limit.
/// - Fails if the user would receive zero tokens.
/// - Fails if pool does not exist
pub fn quote(
    deps: Deps,
    env: Env,
    pool_key: PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<QuoteResult, ContractError> {
    let calculate_swap_result = calculate_swap(
        deps.storage,
        env.block.time.nanos(),
        &pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    Ok(QuoteResult {
        amount_in: calculate_swap_result.amount_in,
        amount_out: calculate_swap_result.amount_out,
        target_sqrt_price: calculate_swap_result.pool.sqrt_price,
        ticks: calculate_swap_result.ticks,
    })
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
) -> Result<TokenAmount, ContractError> {
    let amount_out = route(deps.storage, env, amount_in, swaps)?;
    Ok(amount_out)
}
