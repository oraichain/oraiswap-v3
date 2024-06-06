use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use decimal::Decimal;

use crate::math::percentage::Percentage;
use crate::{ContractError, FeeTier};

#[cw_serde]
#[derive(Eq)]
pub struct PoolKey {
    pub token_x: Addr,
    pub token_y: Addr,
    pub fee_tier: FeeTier,
}

impl Default for PoolKey {
    fn default() -> Self {
        Self {
            token_x: Addr::unchecked("token0"),
            token_y: Addr::unchecked("token1"),
            fee_tier: FeeTier {
                fee: Percentage::new(0),
                tick_spacing: 1,
            },
        }
    }
}

impl PoolKey {
    pub fn key(&self) -> Vec<u8> {
        let token_x_bytes = self.token_x.as_bytes();
        let token_y_bytes = self.token_y.as_bytes();
        match token_x_bytes.le(token_y_bytes) {
            true => [token_x_bytes, token_y_bytes].concat(),
            false => [token_y_bytes, token_x_bytes].concat(),
        }
    }

    pub fn new(token_0: Addr, token_1: Addr, fee_tier: FeeTier) -> Result<Self, ContractError> {
        if token_0 == token_1 {
            return Err(ContractError::TokensAreSame);
        }

        if token_0 < token_1 {
            Ok(PoolKey {
                token_x: token_0,
                token_y: token_1,
                fee_tier,
            })
        } else {
            Ok(PoolKey {
                token_x: token_1,
                token_y: token_0,
                fee_tier,
            })
        }
    }
}
