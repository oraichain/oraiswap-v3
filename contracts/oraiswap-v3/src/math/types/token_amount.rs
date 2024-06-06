use cosmwasm_schema::cw_serde;
use decimal::*;

use crate::ContractError;

use super::sqrt_price::SqrtPrice;

#[decimal(0)]
#[cw_serde]
#[derive(Default, Eq, Copy, PartialOrd)]
pub struct TokenAmount(pub u128);

impl TokenAmount {
    pub fn from_big_sqrt_price(value: U256) -> Result<TokenAmount, ContractError> {
        let result: u128 = value
            .checked_div(SqrtPrice::one())
            .ok_or_else(|| ContractError::Div)?
            .try_into()
            .map_err(|_| ContractError::Cast)?;

        Ok(TokenAmount(result))
    }

    pub fn from_big_sqrt_price_up(value: U256) -> Result<TokenAmount, ContractError> {
        let result: u128 = value
            .checked_add(SqrtPrice::almost_one())
            .ok_or_else(|| ContractError::Add)?
            .checked_div(SqrtPrice::one())
            .ok_or_else(|| ContractError::Div)?
            .try_into()
            .map_err(|_| ContractError::Cast)?;
        Ok(TokenAmount(result))
    }
}
