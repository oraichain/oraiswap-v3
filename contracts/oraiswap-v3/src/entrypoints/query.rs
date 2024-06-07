use cosmwasm_std::{Addr, Deps};

use crate::{
    percentage::Percentage,
    state::{self, CONFIG},
    ContractError, Position,
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
