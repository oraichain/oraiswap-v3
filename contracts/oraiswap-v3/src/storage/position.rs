use super::{Pool, PoolKey, Tick};
use crate::{
    math::{
        clamm::*,
        types::{
            fee_growth::{calculate_fee_growth_inside, FeeGrowth},
            liquidity::Liquidity,
            sqrt_price::SqrtPrice,
            token_amount::TokenAmount,
        },
    },
    ContractError,
};
use cosmwasm_schema::cw_serde;
use decimal::*;

#[cw_serde]
#[derive(Default, Eq)]
pub struct Position {
    pub pool_key: PoolKey,
    pub liquidity: Liquidity,
    pub lower_tick_index: i32,
    pub upper_tick_index: i32,
    pub fee_growth_inside_x: FeeGrowth,
    pub fee_growth_inside_y: FeeGrowth,
    pub last_block_number: u64,
    pub tokens_owed_x: TokenAmount,
    pub tokens_owed_y: TokenAmount,
}

impl Position {
    #[allow(clippy::too_many_arguments)]
    pub fn modify(
        &mut self,
        pool: &mut Pool,
        upper_tick: &mut Tick,
        lower_tick: &mut Tick,
        liquidity_delta: Liquidity,
        add: bool,
        current_timestamp: u64,
        tick_spacing: u16,
    ) -> Result<(TokenAmount, TokenAmount), ContractError> {
        pool.last_timestamp = current_timestamp;

        // calculate dynamically limit allows easy modification
        let max_liquidity_per_tick = calculate_max_liquidity_per_tick(tick_spacing);

        // update initialized tick
        lower_tick.update(liquidity_delta, max_liquidity_per_tick, false, add)?;

        upper_tick.update(liquidity_delta, max_liquidity_per_tick, true, add)?;

        // update fee inside position
        let (fee_growth_inside_x, fee_growth_inside_y) = calculate_fee_growth_inside(
            lower_tick.index,
            lower_tick.fee_growth_outside_x,
            lower_tick.fee_growth_outside_y,
            upper_tick.index,
            upper_tick.fee_growth_outside_x,
            upper_tick.fee_growth_outside_y,
            pool.current_tick_index,
            pool.fee_growth_global_x,
            pool.fee_growth_global_y,
        );

        self.update(
            add,
            liquidity_delta,
            fee_growth_inside_x,
            fee_growth_inside_y,
        )?;

        // calculate tokens amounts and update pool liquidity
        pool.update_liquidity(liquidity_delta, add, upper_tick.index, lower_tick.index)
    }

    pub fn update(
        &mut self,
        sign: bool,
        liquidity_delta: Liquidity,
        fee_growth_inside_x: FeeGrowth,
        fee_growth_inside_y: FeeGrowth,
    ) -> Result<(), ContractError> {
        if liquidity_delta.is_zero() && self.liquidity.is_zero() {
            return Err(ContractError::EmptyPositionPokes);
        }

        // calculate accumulated fee
        let tokens_owed_x = (fee_growth_inside_x
            .unchecked_sub(self.fee_growth_inside_x)
            .to_fee(self.liquidity))?;
        let tokens_owed_y = (fee_growth_inside_y
            .unchecked_sub(self.fee_growth_inside_y)
            .to_fee(self.liquidity))?;

        self.liquidity = (self.calculate_new_liquidity(sign, liquidity_delta))?;
        self.fee_growth_inside_x = fee_growth_inside_x;
        self.fee_growth_inside_y = fee_growth_inside_y;

        self.tokens_owed_x += tokens_owed_x;
        self.tokens_owed_y += tokens_owed_y;
        Ok(())
    }

    fn calculate_new_liquidity(
        &mut self,
        sign: bool,
        liquidity_delta: Liquidity,
    ) -> Result<Liquidity, ContractError> {
        // validate in decrease liquidity case
        if !sign && { self.liquidity } < liquidity_delta {
            return Err(ContractError::InsufficientLiquidity);
        }

        match sign {
            true => self
                .liquidity
                .checked_add(liquidity_delta)
                .map_err(|_| ContractError::PositionAddLiquidityOverflow),
            false => self
                .liquidity
                .checked_sub(liquidity_delta)
                .map_err(|_| ContractError::PositionRemoveLiquidityUnderflow),
        }
    }

    pub fn claim_fee(
        &mut self,
        pool: &mut Pool,
        upper_tick: &mut Tick,
        lower_tick: &mut Tick,
        current_timestamp: u64,
    ) -> Result<(TokenAmount, TokenAmount), ContractError> {
        self.modify(
            pool,
            upper_tick,
            lower_tick,
            Liquidity::new(0),
            true,
            current_timestamp,
            self.pool_key.fee_tier.tick_spacing,
        )?;

        let tokens_owed_x = self.tokens_owed_x;
        let tokens_owed_y = self.tokens_owed_y;

        self.tokens_owed_x = TokenAmount(0);
        self.tokens_owed_y = TokenAmount(0);

        Ok((tokens_owed_x, tokens_owed_y))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        pool: &mut Pool,
        pool_key: PoolKey,
        lower_tick: &mut Tick,
        upper_tick: &mut Tick,
        current_timestamp: u64,
        liquidity_delta: Liquidity,
        slippage_limit_lower: SqrtPrice,
        slippage_limit_upper: SqrtPrice,
        block_number: u64,
        tick_spacing: u16,
    ) -> Result<(Self, TokenAmount, TokenAmount), ContractError> {
        if pool.sqrt_price < slippage_limit_lower || pool.sqrt_price > slippage_limit_upper {
            return Err(ContractError::PriceLimitReached);
        }

        // init position
        let mut position = Position {
            pool_key,
            liquidity: Liquidity::new(0),
            lower_tick_index: lower_tick.index,
            upper_tick_index: upper_tick.index,
            fee_growth_inside_x: FeeGrowth::new(0),
            fee_growth_inside_y: FeeGrowth::new(0),
            last_block_number: block_number,
            tokens_owed_x: TokenAmount::new(0),
            tokens_owed_y: TokenAmount::new(0),
        };

        let (required_x, required_y) = (position.modify(
            pool,
            upper_tick,
            lower_tick,
            liquidity_delta,
            true,
            current_timestamp,
            tick_spacing,
        ))
        .unwrap();

        Ok((position, required_x, required_y))
    }

    pub fn remove(
        &mut self,
        pool: &mut Pool,
        current_timestamp: u64,
        lower_tick: &mut Tick,
        upper_tick: &mut Tick,
        tick_spacing: u16,
    ) -> Result<(TokenAmount, TokenAmount, bool, bool), ContractError> {
        let liquidity_delta = self.liquidity;
        let (mut amount_x, mut amount_y) = self.modify(
            pool,
            upper_tick,
            lower_tick,
            liquidity_delta,
            false,
            current_timestamp,
            tick_spacing,
        )?;

        amount_x += self.tokens_owed_x;
        amount_y += self.tokens_owed_y;

        let deinitialize_lower_tick = lower_tick.liquidity_gross.is_zero();
        let deinitialize_upper_tick = upper_tick.liquidity_gross.is_zero();

        Ok((
            amount_x,
            amount_y,
            deinitialize_lower_tick,
            deinitialize_upper_tick,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_new_liquidity() {
        // negative liquidity error
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(1),
                ..Default::default()
            };
            let sign: bool = false;
            let liquidity_delta = Liquidity::from_integer(2);

            let result = position.calculate_new_liquidity(sign, liquidity_delta);

            assert!(result.is_err());
        }
        // adding liquidity
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(2),
                ..Default::default()
            };
            let sign: bool = true;
            let liquidity_delta = Liquidity::from_integer(2);

            let new_liquidity = position
                .calculate_new_liquidity(sign, liquidity_delta)
                .unwrap();

            assert_eq!(new_liquidity, Liquidity::from_integer(4));
        }
        // subtracting liquidity
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(2),
                ..Default::default()
            };
            let sign: bool = false;
            let liquidity_delta = Liquidity::from_integer(2);

            let new_liquidity = position
                .calculate_new_liquidity(sign, liquidity_delta)
                .unwrap();

            assert_eq!(new_liquidity, Liquidity::from_integer(0));
        }
    }

    #[test]
    fn test_update() {
        // Disable empty position pokes error
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(0),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(0);
            let fee_growth_inside_x = FeeGrowth::from_integer(1);
            let fee_growth_inside_y = FeeGrowth::from_integer(1);

            let result = position.update(
                sign,
                liquidity_delta,
                fee_growth_inside_x,
                fee_growth_inside_y,
            );

            assert!(result.is_err());
        }
        // zero liquidity fee shouldn't change
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(0),
                fee_growth_inside_x: FeeGrowth::from_integer(4),
                fee_growth_inside_y: FeeGrowth::from_integer(4),
                tokens_owed_x: TokenAmount(100),
                tokens_owed_y: TokenAmount(100),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(1);
            let fee_growth_inside_x = FeeGrowth::from_integer(5);
            let fee_growth_inside_y = FeeGrowth::from_integer(5);

            position
                .update(
                    sign,
                    liquidity_delta,
                    fee_growth_inside_x,
                    fee_growth_inside_y,
                )
                .unwrap();

            assert_eq!({ position.liquidity }, Liquidity::from_integer(1));
            assert_eq!({ position.fee_growth_inside_x }, FeeGrowth::from_integer(5));
            assert_eq!({ position.fee_growth_inside_y }, FeeGrowth::from_integer(5));
            assert_eq!({ position.tokens_owed_x }, TokenAmount(100));
            assert_eq!({ position.tokens_owed_y }, TokenAmount(100));
        }
        // fee should change
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(1),
                fee_growth_inside_x: FeeGrowth::from_integer(4),
                fee_growth_inside_y: FeeGrowth::from_integer(4),
                tokens_owed_x: TokenAmount(100),
                tokens_owed_y: TokenAmount(100),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(1);
            let fee_growth_inside_x = FeeGrowth::from_integer(5);
            let fee_growth_inside_y = FeeGrowth::from_integer(5);

            position
                .update(
                    sign,
                    liquidity_delta,
                    fee_growth_inside_x,
                    fee_growth_inside_y,
                )
                .unwrap();

            assert_eq!({ position.liquidity }, Liquidity::from_integer(2));
            assert_eq!({ position.fee_growth_inside_x }, FeeGrowth::from_integer(5));
            assert_eq!({ position.fee_growth_inside_y }, FeeGrowth::from_integer(5));
            assert_eq!({ position.tokens_owed_x }, TokenAmount(101));
            assert_eq!({ position.tokens_owed_y }, TokenAmount(101));
        }
        // previous fee_growth_inside close to max and current close to 0
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(1),
                fee_growth_inside_x: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(10),
                fee_growth_inside_y: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(10),
                tokens_owed_x: TokenAmount(100),
                tokens_owed_y: TokenAmount(100),
                ..Default::default()
            };
            let sign = true;
            let liquidity_delta = Liquidity::from_integer(1);
            let fee_growth_inside_x = FeeGrowth::from_integer(10);
            let fee_growth_inside_y = FeeGrowth::from_integer(10);

            position
                .update(
                    sign,
                    liquidity_delta,
                    fee_growth_inside_x,
                    fee_growth_inside_y,
                )
                .unwrap();

            assert_eq!({ position.liquidity }, Liquidity::from_integer(2));
            assert_eq!(
                { position.fee_growth_inside_x },
                FeeGrowth::from_integer(10)
            );
            assert_eq!(
                { position.fee_growth_inside_y },
                FeeGrowth::from_integer(10)
            );
            assert_eq!({ position.tokens_owed_x }, TokenAmount(120));
            assert_eq!({ position.tokens_owed_y }, TokenAmount(120));
        }
    }

    #[test]
    fn test_modify() {
        // owed tokens after overflow
        {
            let mut position = Position {
                liquidity: Liquidity::from_integer(123),
                fee_growth_inside_x: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(1234),
                fee_growth_inside_y: FeeGrowth::new(u128::MAX) - FeeGrowth::from_integer(1234),
                tokens_owed_x: TokenAmount(0),
                tokens_owed_y: TokenAmount(0),
                ..Default::default()
            };
            let mut pool = Pool {
                current_tick_index: 0,
                fee_growth_global_x: FeeGrowth::from_integer(20),
                fee_growth_global_y: FeeGrowth::from_integer(20),
                ..Default::default()
            };
            let mut upper_tick = Tick {
                index: -10,
                fee_growth_outside_x: FeeGrowth::from_integer(15),
                fee_growth_outside_y: FeeGrowth::from_integer(15),
                liquidity_gross: Liquidity::from_integer(123),
                ..Default::default()
            };
            let mut lower_tick = Tick {
                index: -20,
                fee_growth_outside_x: FeeGrowth::from_integer(20),
                fee_growth_outside_y: FeeGrowth::from_integer(20),
                liquidity_gross: Liquidity::from_integer(123),
                ..Default::default()
            };
            let liquidity_delta = Liquidity::new(0);
            let add = true;
            let current_timestamp: u64 = 1234567890;

            position
                .modify(
                    &mut pool,
                    &mut upper_tick,
                    &mut lower_tick,
                    liquidity_delta,
                    add,
                    current_timestamp,
                    1,
                )
                .unwrap();

            assert_eq!({ position.tokens_owed_x }, TokenAmount(151167));
        }
    }
}
