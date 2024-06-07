use cosmwasm_std::{Addr, Deps};

use crate::{
    percentage::Percentage,
    state::{self, get_bitmap, get_position_length, CONFIG, MAX_LIMIT},
    ContractError, FeeTier, LiquidityTick, Pool, PoolKey, Position, PositionTick, Tick,
};

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
    Ok(get_bitmap(
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
    offset: Option<u32>,
) -> Result<Vec<PoolKey>, ContractError> {
    state::get_all_pool_keys(deps.storage, limit, offset)
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
    let positions_length = get_position_length(deps.storage, &owner);
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

        if ticks.len() >= MAX_LIMIT as usize {
            break;
        }
    }

    Ok(ticks)
}

// /// Retrieves the amount of positions held by the user.
// ///
// /// # Parameters
// /// - `owner`: An `Addr` identifying the user who owns the position.
// pub fn get_user_position_amount(deps: Deps, owner: Addr) -> Result<u32, ContractError> {}

// /// Retrieves tickmap chunks
// ///
// /// # Parameters
// /// - `pool_key`: A unique key that identifies the specified pool.
// /// - `start_tick_index`: offset tick index.
// /// - `end_tick_index`: limiting tick index.
// /// - `x_to_y`: direction of the query.
// pub fn get_tickmap(
//     deps: Deps,
//     pool_key: PoolKey,
//     start_tick_index: i32,
//     end_tick_index: i32,
//     x_to_y: bool,
// ) -> Result<Vec<(u16, u64)>, ContractError> {
// }

// /// Retrieves ticks of a specified pool.
// ///
// /// # Parameters
// /// - `pool_key`: A unique key that identifies the specified pool.
// /// - `tick_indexes`: Indexes of the tick to be retrieved.
// ///
// /// # Errors
// /// - Fails if tick_indexes are too large
// /// - Fails if tick is not found
// ///
// pub fn get_liquidity_ticks(
//     deps: Deps,
//     pool_key: PoolKey,
//     tick_indexes: Vec<i32>,
// ) -> Result<Vec<LiquidityTick>, ContractError> {
// }
// /// Retrieves the amount of liquidity ticks of a specified pool.
// ///
// /// # Parameters
// /// - `pool_key`: A unique key that identifies the specified pool. For poolkeys with tick_spacing equal to 1 the query has to be split into 2 smaller queries
// /// - `lower_tick`: index to start counting from(inclusive)
// /// - `upper_tick`: index to stop counting after(inclusive)
// ///
// /// # Errors
// /// - Fails if lower_tick or upper_tick are invalid
// /// - Fails if tick_spacing is invalid
// pub fn get_liquidity_ticks_amount(
//     deps: Deps,
//     pool_key: PoolKey,
//     lower_tick: i32,
//     upper_tick: i32,
// ) -> Result<u32, ContractError> {
// }
