use cosmwasm_std::{to_binary, Addr, DepsMut, Env, MessageInfo, Storage, WasmMsg};
use cw20::Cw20ExecuteMsg;
use decimal::{CheckedOps, Decimal};

use crate::{
    check_tick, compute_swap_step,
    interface::{CalculateSwapResult, SwapHop},
    sqrt_price::{get_max_tick, get_min_tick, SqrtPrice},
    state::{
        self, add_tick, flip_bitmap, get_closer_limit, get_tick, update_tick, CONFIG, MAX_LIMIT,
        POOLS,
    },
    token_amount::TokenAmount,
    ContractError, PoolKey, Tick, UpdatePoolTick, MAX_SQRT_PRICE, MIN_SQRT_PRICE,
};

pub fn create_tick(
    store: &mut dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    index: i32,
) -> Result<Tick, ContractError> {
    check_tick(index, pool_key.fee_tier.tick_spacing)?;
    let pool = state::get_pool(store, &pool_key)?;

    let tick = Tick::create(index, &pool, current_timestamp);
    add_tick(store, pool_key, index, &tick)?;
    flip_bitmap(store, true, index, pool_key.fee_tier.tick_spacing, pool_key)?;

    Ok(tick)
}

pub fn calculate_swap(
    store: &dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::AmountIsZero {});
    }

    let mut ticks: Vec<Tick> = vec![];
    let mut pool = state::get_pool(store, &pool_key)?;

    if x_to_y {
        if pool.sqrt_price <= sqrt_price_limit || sqrt_price_limit > SqrtPrice::new(MAX_SQRT_PRICE)
        {
            return Err(ContractError::WrongLimit {});
        }
    } else if pool.sqrt_price >= sqrt_price_limit
        || sqrt_price_limit < SqrtPrice::new(MIN_SQRT_PRICE)
    {
        return Err(ContractError::WrongLimit {});
    }

    let tick_limit = if x_to_y {
        get_min_tick(pool_key.fee_tier.tick_spacing)
    } else {
        get_max_tick(pool_key.fee_tier.tick_spacing)
    };

    let mut remaining_amount = amount;

    let mut total_amount_in = TokenAmount::new(0);
    let mut total_amount_out = TokenAmount::new(0);

    let event_start_sqrt_price = pool.sqrt_price;
    let mut event_fee_amount = TokenAmount::new(0);

    while !remaining_amount.is_zero() {
        let (swap_limit, limiting_tick) = get_closer_limit(
            store,
            sqrt_price_limit,
            x_to_y,
            pool.current_tick_index,
            pool_key.fee_tier.tick_spacing,
            pool_key,
        )?;

        let result = compute_swap_step(
            pool.sqrt_price,
            swap_limit,
            pool.liquidity,
            remaining_amount,
            by_amount_in,
            pool_key.fee_tier.fee,
        )?;

        // make remaining amount smaller
        if by_amount_in {
            remaining_amount = remaining_amount
                .checked_sub(result.amount_in + result.fee_amount)
                .map_err(|_| ContractError::Sub)?;
        } else {
            remaining_amount = remaining_amount
                .checked_sub(result.amount_out)
                .map_err(|_| ContractError::Sub)?;
        }

        pool.add_fee(result.fee_amount, x_to_y, CONFIG.load(store)?.protocol_fee)?;
        event_fee_amount += result.fee_amount;

        pool.sqrt_price = result.next_sqrt_price;

        total_amount_in += result.amount_in + result.fee_amount;
        total_amount_out += result.amount_out;

        // Fail if price would go over swap limit
        if pool.sqrt_price == sqrt_price_limit && !remaining_amount.is_zero() {
            return Err(ContractError::PriceLimitReached {});
        }

        let mut tick_update = {
            if let Some((tick_index, is_initialized)) = limiting_tick {
                if is_initialized {
                    let tick = get_tick(store, pool_key, tick_index)?;
                    UpdatePoolTick::TickInitialized(tick)
                } else {
                    UpdatePoolTick::TickUninitialized(tick_index)
                }
            } else {
                UpdatePoolTick::NoTick
            }
        };

        let (amount_to_add, amount_after_tick_update, has_crossed) = pool.update_tick(
            result,
            swap_limit,
            &mut tick_update,
            remaining_amount,
            by_amount_in,
            x_to_y,
            current_timestamp,
            CONFIG.load(store)?.protocol_fee,
            pool_key.fee_tier,
        )?;

        remaining_amount = amount_after_tick_update;
        total_amount_in += amount_to_add;

        if let UpdatePoolTick::TickInitialized(tick) = tick_update {
            if has_crossed {
                ticks.push(tick)
            }
        }

        let reached_tick_limit = match x_to_y {
            true => pool.current_tick_index <= tick_limit,
            false => pool.current_tick_index >= tick_limit,
        };

        if reached_tick_limit {
            return Err(ContractError::TickLimitReached {});
        }
    }
    if total_amount_out.is_zero() {
        return Err(ContractError::NoGainSwap {});
    }

    Ok(CalculateSwapResult {
        amount_in: total_amount_in,
        amount_out: total_amount_out,
        start_sqrt_price: event_start_sqrt_price,
        target_sqrt_price: pool.sqrt_price,
        fee: event_fee_amount,
        pool,
        ticks,
    })
}

pub fn swap_internal(
    store: &mut dyn Storage,
    msgs: &mut Vec<WasmMsg>,
    sender: &Addr,
    contract_address: &Addr,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    let calculate_swap_result = calculate_swap(
        store,
        current_timestamp,
        pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    let mut crossed_tick_indexes: Vec<i32> = vec![];

    for tick in calculate_swap_result.ticks.iter() {
        update_tick(store, &pool_key, tick.index, tick)?;
        crossed_tick_indexes.push(tick.index);
    }

    POOLS.save(store, &pool_key.key(), &calculate_swap_result.pool)?;

    if x_to_y {
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: sender.to_string(),
                recipient: contract_address.to_string(),
                amount: calculate_swap_result.amount_in.into(),
            })?,
            funds: vec![],
        });
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: calculate_swap_result.amount_out.into(),
            })?,
            funds: vec![],
        });
    } else {
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: sender.to_string(),
                recipient: contract_address.to_string(),
                amount: calculate_swap_result.amount_in.into(),
            })?,
            funds: vec![],
        });
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: calculate_swap_result.amount_out.into(),
            })?,
            funds: vec![],
        });
    }

    Ok(calculate_swap_result)
}

pub fn route(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msgs: &mut Vec<WasmMsg>,
    is_swap: bool,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let mut next_swap_amount = amount_in;

    let current_timestamp = env.block.time.nanos();

    for swap_hop in swaps {
        let SwapHop { pool_key, x_to_y } = swap_hop;

        let sqrt_price_limit = if x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        let res = if is_swap {
            swap_internal(
                deps.storage,
                msgs,
                &info.sender,
                &env.contract.address,
                current_timestamp,
                &pool_key,
                x_to_y,
                next_swap_amount,
                true,
                sqrt_price_limit,
            )
        } else {
            calculate_swap(
                deps.storage,
                current_timestamp,
                &pool_key,
                x_to_y,
                next_swap_amount,
                true,
                sqrt_price_limit,
            )
        }?;

        next_swap_amount = res.amount_out;
    }

    Ok(next_swap_amount)
}

pub fn tickmap_slice(
    store: &dyn Storage,
    range: impl Iterator<Item = u16>,
    pool_key: &PoolKey,
) -> Vec<(u16, u64)> {
    let mut tickmap_slice: Vec<(u16, u64)> = vec![];

    for chunk_index in range {
        if let Ok(chunk) = state::get_bitmap_item(store, chunk_index, pool_key) {
            tickmap_slice.push((chunk_index, chunk));

            if tickmap_slice.len() == MAX_LIMIT as usize {
                return tickmap_slice;
            }
        }
    }

    tickmap_slice
}
