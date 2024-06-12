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
        // first 8 bytes is fee, next 2 bytes is tick_spacing
        let mut out = Vec::with_capacity(10);
        out.extend_from_slice(&self.fee.0.to_be_bytes());
        out.extend_from_slice(&self.tick_spacing.to_be_bytes());
        out
    }

    pub fn from_bytes(raw_key: &[u8]) -> Result<Self, ContractError> {
        // first 8 bytes is key, next 2 bytes is tick_spacing
        if raw_key.len() != 10 {
            return Err(ContractError::InvalidSize);
        }

        let fee = u64::from_be_bytes(raw_key[0..8].try_into().unwrap());
        let tick_spacing = u16::from_be_bytes(raw_key[8..10].try_into().unwrap());

        Ok(Self {
            fee: Percentage::new(fee),
            tick_spacing,
        })
    }
}
