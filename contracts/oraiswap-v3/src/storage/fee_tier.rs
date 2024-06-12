use crate::{math::types::percentage::Percentage, ContractError};
use cosmwasm_schema::cw_serde;
use decimal::*;

#[cw_serde]
#[derive(Eq, Copy, Default)]
pub struct FeeTier {
    pub fee: Percentage,
    pub tick_spacing: u16,
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

    pub fn key(&self) -> Vec<u8> {
        let mut key = self.fee.0.to_be_bytes().to_vec();
        key.extend_from_slice(&self.tick_spacing.to_be_bytes());
        key
    }
}
