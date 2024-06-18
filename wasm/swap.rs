use crate::clamm::compute_swap_step;
use crate::sqrt_price::{get_max_tick, get_min_tick, SqrtPrice};
use crate::token_amount::TokenAmount;
use crate::{
    CalculateSwapResult, FeeTier, Tickmap, UpdatePoolTick, MAX_SQRT_PRICE, MAX_TICK_CROSS,
    MIN_SQRT_PRICE,
};
use crate::{LiquidityTick, Pool};
use decimal::Decimal;
use serde_wasm_bindgen::from_value;
use traceable_result::TrackableResult;
use traceable_result::*;
use wasm_bindgen::prelude::*;

type LiquidityTicks = Vec<LiquidityTick>;

#[wasm_bindgen(js_name = simulateSwap)]
#[allow(non_snake_case)]
pub fn simulate_swap(
    tickmap: Tickmap,
    fee_tier: FeeTier,
    mut pool: Pool,
    ticks: JsValue,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> TrackableResult<CalculateSwapResult> {
    let ticks: LiquidityTicks = from_value(ticks).unwrap();
    if amount.is_zero() {
        return Err(err!("Amount is zero"));
    }

    if x_to_y {
        if pool.sqrt_price <= sqrt_price_limit || sqrt_price_limit > SqrtPrice::new(MAX_SQRT_PRICE)
        {
            return Err(err!("Wrong limit"));
        }
    } else if pool.sqrt_price >= sqrt_price_limit
        || sqrt_price_limit < SqrtPrice::new(MIN_SQRT_PRICE)
    {
        return Err(err!("Wrong limit"));
    }

    let tick_limit = if x_to_y {
        get_min_tick(fee_tier.tick_spacing as u16)
    } else {
        get_max_tick(fee_tier.tick_spacing as u16)
    };

    let start_sqrt_price = pool.sqrt_price;

    let mut global_insufficient_liquidity = false;
    let mut state_outdated = false;
    let mut max_ticks_crossed = false;

    let mut crossed_ticks: Vec<LiquidityTick> = vec![];
    let mut remaining_amount = amount;
    let mut total_amount_in = TokenAmount(0);
    let mut total_amount_out = TokenAmount(0);
    let mut total_fee_amount = TokenAmount(0);

    while !remaining_amount.is_zero() {
        let closer_limit = tickmap.get_closer_limit(
            sqrt_price_limit,
            x_to_y,
            pool.current_tick_index as i32,
            fee_tier.tick_spacing as u16,
        );
        let (swap_limit, limiting_tick) = if let Ok(closer_limit) = closer_limit {
            closer_limit
        } else {
            global_insufficient_liquidity = true;
            break;
        };

        let result = compute_swap_step(
            pool.sqrt_price,
            swap_limit,
            pool.liquidity,
            remaining_amount,
            by_amount_in,
            fee_tier.fee,
        )?;

        // make remaining amount smaller
        if by_amount_in {
            remaining_amount -= result.amount_in + result.fee_amount;
        } else {
            remaining_amount -= result.amount_out;
        }

        total_fee_amount += result.fee_amount;

        pool.sqrt_price = result.next_sqrt_price;

        total_amount_in += result.amount_in + result.fee_amount;
        total_amount_out += result.amount_out;

        // Fail if price would go over swap limit
        if pool.sqrt_price == sqrt_price_limit && !remaining_amount.is_zero() {
            global_insufficient_liquidity = true;
            break;
        }

        let mut tick_update = {
            if let Some((tick_index, is_initialized)) = limiting_tick {
                if is_initialized {
                    let tick = ticks.iter().find(|t| t.index as i32 == tick_index);

                    match tick {
                        Some(tick) => UpdatePoolTick::TickInitialized(*tick),
                        None => {
                            state_outdated = true;
                            break;
                        }
                    }
                } else {
                    UpdatePoolTick::TickUninitialized(tick_index)
                }
            } else {
                UpdatePoolTick::NoTick
            }
        };

        let tick_update_return = pool.update_tick(
            result,
            swap_limit,
            &mut tick_update,
            remaining_amount,
            by_amount_in,
            x_to_y,
            pool.last_timestamp,
            fee_tier,
        );
        let (amount_to_add, amount_after_tick_update, has_crossed) =
            if let Ok(tick_update_return) = tick_update_return {
                tick_update_return
            } else {
                state_outdated = true;
                break;
            };

        remaining_amount = amount_after_tick_update;
        total_amount_in += amount_to_add;

        if let UpdatePoolTick::TickInitialized(tick) = tick_update {
            if has_crossed {
                crossed_ticks.push(tick);
                if crossed_ticks.len() > MAX_TICK_CROSS as usize {
                    max_ticks_crossed = true;
                    break;
                }
            }
        }

        let reached_tick_limit = match x_to_y {
            true => pool.current_tick_index <= tick_limit,
            false => pool.current_tick_index >= tick_limit,
        };

        if reached_tick_limit {
            global_insufficient_liquidity = true;
            break;
        }
    }

    Ok(CalculateSwapResult {
        amount_in: total_amount_in,
        amount_out: total_amount_out,
        start_sqrt_price,
        target_sqrt_price: pool.sqrt_price,
        fee: total_fee_amount,
        crossed_ticks,
        global_insufficient_liquidity,
        state_outdated,
        max_ticks_crossed,
    })
}
