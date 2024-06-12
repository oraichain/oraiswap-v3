use crate::{ContractError, FeeTier};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
#[derive(Default, Eq)]
pub struct PoolKey {
    pub token_x: String,
    pub token_y: String,
    pub fee_tier: FeeTier,
}

impl PoolKey {
    pub fn key(&self) -> Vec<u8> {
        // sort by asc then append fee_tier at the end to create unique key
        [
            self.token_x.as_bytes(),
            self.token_y.as_bytes(),
            &self.fee_tier.key(),
        ]
        .concat()
    }

    pub fn new(token_0: Addr, token_1: Addr, fee_tier: FeeTier) -> Result<Self, ContractError> {
        if token_0 == token_1 {
            return Err(ContractError::TokensAreSame);
        }

        if token_0 < token_1 {
            Ok(PoolKey {
                token_x: token_0.to_string(),
                token_y: token_1.to_string(),
                fee_tier,
            })
        } else {
            Ok(PoolKey {
                token_x: token_1.to_string(),
                token_y: token_0.to_string(),
                fee_tier,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use decimal::Decimal;

    use crate::{percentage::Percentage, FeeTier, PoolKey};

    #[test]
    fn test_key() {
        let token_x = Addr::unchecked("token_0");
        let token_y = Addr::unchecked("token_1");
        let fee_tier = FeeTier {
            fee: Percentage::new(10),
            tick_spacing: 1,
        };

        let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();

        assert_eq!(
            pool_key.key(),
            [b"token_0", b"token_1", fee_tier.key().as_slice()].concat()
        );
    }
}
