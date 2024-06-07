use cosmwasm_std::{Deps, StdResult};

use crate::{percentage::Percentage, state::CONFIG};

/// Retrieves the protocol fee represented as a percentage.
pub fn get_protocol_fee(deps: Deps) -> StdResult<Percentage> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}
