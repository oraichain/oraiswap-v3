use crate::{math::types::percentage::Percentage, ContractError};
use cosmwasm_schema::cw_serde;
use decimal::*;

#[cw_serde]
pub struct FeeTier {
    pub fee: Percentage,
    pub tick_spacing: u16,
}

impl Default for FeeTier {
    fn default() -> Self {
        Self {
            fee: Percentage::new(0),
            tick_spacing: 1,
        }
    }
}

impl FeeTier {
    pub fn new(fee: Percentage, tick_spacing: u16) -> Result<Self, ContractError> {
        if tick_spacing == 0 || tick_spacing > 100 {
            return Err(ContractError::InvalidTickSpacing);
        }

        if fee > Percentage::from_integer(1) {
            return Err(ContractError::InvalidFee);
        }

        Ok(Self { fee, tick_spacing })
    }
}
