use crate::error::ContractError;
use crate::liquidity::Liquidity;
use crate::percentage::Percentage;
use crate::sqrt_price::SqrtPrice;
use crate::state::{add_position, get_tick, update_tick, CONFIG, POOLS};
use crate::token_amount::TokenAmount;
use crate::{PoolKey, Position};

use cosmwasm_std::{to_binary, Addr, DepsMut, Env, MessageInfo, Response, WasmMsg};
use cw20::Cw20ExecuteMsg;

use super::{calculate_swap, create_tick};

/// Allows an fee receiver to withdraw collected fees.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized receiver.
///
/// # External contracts
/// - PSP22
pub fn withdraw_protocol_fee(
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

/// Allows an admin to adjust the protocol fee.
///
/// # Parameters
/// - `protocol_fee`: The expected fee represented as a percentage.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user.
pub fn change_protocol_fee(
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

/// Allows admin to change current fee receiver.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `fee_receiver`: An `AccountId` identifying the user authorized to claim fees.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized user.
pub fn change_fee_receiver(
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

/// Opens a position.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `lower_tick`: The index of the lower tick for opening the position.
/// - `upper_tick`: The index of the upper tick for opening the position.
/// - `liquidity_delta`: The desired liquidity provided by the user in the specified range.
/// - `slippage_limit_lower`: The price limit for downward movement to execute the position creation.
/// - `slippage_limit_upper`: The price limit for upward movement to execute the position creation.
///
/// # Events
/// - On successful transfer, emits a `Create Position` event for the newly opened position.
///
/// # Errors
/// - Fails if the user attempts to open a position with zero liquidity.
/// - Fails if the user attempts to create a position with invalid tick indexes or tick spacing.
/// - Fails if the price has reached the slippage limit.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if pool does not exist
///
/// # External contracts
/// - PSP22
pub fn create_position(
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

/// Performs a single swap based on the provided parameters.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
/// - `x_to_y`: A boolean specifying the swap direction.
/// - `amount`: TokenAmount that the user wants to swap.
/// - `by_amount_in`: A boolean specifying whether the user provides the amount to swap or expects the amount out.
/// - `sqrt_price_limit`: A square root of price limit allowing the price to move for the swap to occur.
///
/// # Events
/// - On a successful swap, emits a `Swap` event for the freshly made swap.
/// - On a successful swap, emits a `Cross Tick` event for every single tick crossed.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the price has reached the specified price limit (or price associated with specified square root of price).
/// - Fails if the user would receive zero tokens.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if there is insufficient liquidity in pool
/// - Fails if pool does not exist
///
/// # External contracts
/// - PSP22
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

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "swap")
        .add_attribute("amount_out", calculate_swap_result.amount_out.to_string()))
}
