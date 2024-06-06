#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use decimal::{CheckedOps, Decimal};

use crate::error::ContractError;
use crate::interface::CalculateSwapResult;
use crate::liquidity::Liquidity;
use crate::token_amount::TokenAmount;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::sqrt_price::{get_max_tick, get_min_tick, SqrtPrice};
use crate::state::{add_position, add_tick, flip_bitmap, get_closer_limit, get_tick, update_tick, CONFIG, POOLS};
use crate::{check_tick, compute_swap_step, Config, PoolKey, Position, Tick, UpdatePoolTick, MAX_SQRT_PRICE, MIN_SQRT_PRICE};

use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:oraiswap_v3";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        admin: info.sender,
        protocol_fee: msg.protocol_fee,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::WithdrawProtocolFee { pool_key } => withdraw_protocol_fee(deps, info, pool_key),
        ExecuteMsg::ChangeProtocolFee { protocol_fee } => {
            change_protocol_fee(deps, info, protocol_fee)
        }
        ExecuteMsg::ChangeFeeReceiver {
            pool_key,
            fee_receiver,
        } => change_fee_receiver(deps, info, pool_key, fee_receiver),
        ExecuteMsg::CreatePosition {
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
        } => create_position(
            deps,
            env,
            info,
            pool_key,
            lower_tick,
            upper_tick,
            liquidity_delta,
            slippage_limit_lower,
            slippage_limit_upper,
        ),
        ExecuteMsg::Swap {
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        } => swap(
            deps,
            env,
            info,
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProtocolFee {} => to_binary(&get_protocol_fee(deps)?),
    }
}

fn withdraw_protocol_fee(
    deps: DepsMut,
    info: MessageInfo,
    pool_key: PoolKey,
) -> Result<Response, ContractError> {
    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    if pool.fee_receiver != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let (fee_protocol_token_x, fee_protocol_token_y) = pool.withdraw_protocol_fee();
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    let mut msgs = vec![];

    msgs.push(WasmMsg::Execute {
        contract_addr: pool_key.token_x.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: pool.fee_receiver.to_string(),
            amount: fee_protocol_token_x.into(),
        })?,
        funds: vec![],
    });
    msgs.push(WasmMsg::Execute {
        contract_addr: pool_key.token_y.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: pool.fee_receiver.to_string(),
            amount: fee_protocol_token_y.into(),
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_protocol_fee"))
}

fn change_protocol_fee(
    deps: DepsMut,
    info: MessageInfo,
    protocol_fee: Percentage,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.protocol_fee = protocol_fee;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "change_protocol_fee"))
}

fn get_protocol_fee(deps: Deps) -> StdResult<Percentage> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}

fn change_fee_receiver(
    deps: DepsMut,
    info: MessageInfo,
    pool_key: PoolKey,
    fee_receiver: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let pool_key_db = pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;
    pool.fee_receiver = fee_receiver;
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    Ok(Response::new().add_attribute("action", "change_fee_receiver"))
}

fn create_tick(
    store: &mut dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    index: i32,
) -> Result<Tick, ContractError> {
    check_tick(index, pool_key.fee_tier.tick_spacing)?;
    let pool_key_db = pool_key.key();
    let pool = POOLS.load(store, &pool_key_db)?;

    let tick = Tick::create(index, &pool, current_timestamp);
    add_tick(store, pool_key, index, &tick)?;
    flip_bitmap(store, true, index, pool_key.fee_tier.tick_spacing, pool_key)?;

    Ok(tick)
}

fn create_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    lower_tick: i32,
    upper_tick: i32,
    liquidity_delta: Liquidity,
    slippage_limit_lower: SqrtPrice,
    slippage_limit_upper: SqrtPrice,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.nanos();
    let current_block_number = env.block.height;

    // liquidity delta = 0 => return
    if liquidity_delta.is_zero() {
        return Err(ContractError::InsufficientLiquidity {});
    }

    if lower_tick == upper_tick {
        return Err(ContractError::InvalidTickIndex {});
    }
    let pool_key_db = pool_key.key();
    let mut pool = POOLS
        .load(deps.storage, &pool_key_db)
        .map_err(|_| ContractError::PoolNotFound {})?;

    let mut lower_tick = match get_tick(deps.storage, &pool_key, lower_tick) {
        Ok(tick) => tick,
        _ => create_tick(deps.storage, current_timestamp, &pool_key, lower_tick)?,
    };

    let mut upper_tick = match get_tick(deps.storage, &pool_key, upper_tick) {
        Ok(tick) => tick,
        _ => create_tick(deps.storage, current_timestamp, &pool_key, upper_tick)?,
    };

    let (position, x, y) = Position::create(
        &mut pool,
        pool_key.clone(),
        &mut lower_tick,
        &mut upper_tick,
        current_timestamp,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        current_block_number,
        pool_key.fee_tier.tick_spacing,
    )?;

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    add_position(deps.storage, &info.sender, &position)?;

    update_tick(deps.storage, &pool_key, lower_tick.index, &lower_tick)?;
    update_tick(deps.storage, &pool_key, upper_tick.index, &upper_tick)?;

    let mut msgs = vec![];

    msgs.push(WasmMsg::Execute {
        contract_addr: pool_key.token_x.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: x.into(),
        })?,
        funds: vec![],
    });
    msgs.push(WasmMsg::Execute {
        contract_addr: pool_key.token_y.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: y.into(),
        })?,
        funds: vec![],
    });

    Ok(Response::new().add_messages(msgs).add_attributes(vec![
        ("action", "create_position"),
        ("sender", info.sender.as_str()),
        ("lower_tick", &lower_tick.index.to_string()),
        ("upper_tick", &upper_tick.index.to_string()),
        ("sqrt_price", &pool.sqrt_price.to_string()),
    ]))
}

pub fn calculate_swap(
    store: &dyn Storage,
    current_timestamp: u64,
    pool_key: PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::AmountIsZero {});
    }

    let mut ticks: Vec<Tick> = vec![];

    let mut pool = POOLS.load(store, &pool_key.key())?;

    if x_to_y {
        if pool.sqrt_price <= sqrt_price_limit
            || sqrt_price_limit > SqrtPrice::new(MAX_SQRT_PRICE)
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
            &pool_key,
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
                .map_err(|_| ContractError::SubtractionError)?;
        } else {
            remaining_amount = remaining_amount
                .checked_sub(result.amount_out)
                .map_err(|_| ContractError::SubtractionError)?;
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
                    let tick = get_tick(store, &pool_key, tick_index)?;
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

pub fn swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_key: PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.nanos();

    let calculate_swap_result = calculate_swap(
        deps.storage,
        current_timestamp,
        pool_key.clone(),
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    let mut crossed_tick_indexes: Vec<i32> = vec![];

    for tick in calculate_swap_result.ticks.iter() {
        update_tick(deps.storage, &pool_key, tick.index, tick)?;
        crossed_tick_indexes.push(tick.index);
    }

    POOLS.save(deps.storage, &pool_key.key(), &calculate_swap_result.pool)?;

    let mut msgs = vec![];

    if x_to_y {
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount: calculate_swap_result.amount_in.into(),
            })?,
            funds: vec![],
        });
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: calculate_swap_result.amount_out.into(),
            })?,
            funds: vec![],
        });
    } else {
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount: calculate_swap_result.amount_in.into(),
            })?,
            funds: vec![],
        });
        msgs.push(WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: calculate_swap_result.amount_out.into(),
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new().add_messages(msgs).add_attribute("action", "swap"))
}
