use cosmwasm_std::{Addr, Deps};

use crate::{
    percentage::Percentage,
    state::{self, CONFIG},
    ContractError, FeeTier, Pool, PoolKey, Position,
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
