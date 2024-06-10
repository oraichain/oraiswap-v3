use crate::error::ContractError;
use crate::interface::{CalculateSwapResult, SwapHop};
use crate::liquidity::Liquidity;
use crate::percentage::Percentage;
use crate::sqrt_price::SqrtPrice;
use crate::state::{self, CONFIG, POOLS};
use crate::token_amount::TokenAmount;
use crate::{calculate_min_amount_out, check_tick, FeeTier, Pool, PoolKey, Position};

use super::{create_tick, remove_tick_and_flip_bitmap, route, swap_internal};
use cosmwasm_std::{attr, to_binary, Addr, DepsMut, Env, MessageInfo, Response, WasmMsg};
use cw20::Cw20ExecuteMsg;
use decimal::Decimal;

/// Allows an fee receiver to withdraw collected fees.
///
/// # Parameters
/// - `pool_key`: A unique key that identifies the specified pool.
///
/// # Errors
/// - Reverts the call when the caller is an unauthorized receiver.
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
    println!("{:?} {:?}", fee_protocol_token_x, fee_protocol_token_y);
    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: pool.fee_receiver.to_string(),
                amount: fee_protocol_token_x.into(),
            })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: pool.fee_receiver.to_string(),
                amount: fee_protocol_token_y.into(),
            })?,
            funds: vec![],
        },
    ];

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
/// - `fee_receiver`: An `Addr` identifying the user authorized to claim fees.
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
#[allow(clippy::too_many_arguments)]
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

    let mut lower_tick = match state::get_tick(deps.storage, &pool_key, lower_tick) {
        Ok(tick) => tick,
        _ => create_tick(deps.storage, current_timestamp, &pool_key, lower_tick)?,
    };

    let mut upper_tick = match state::get_tick(deps.storage, &pool_key, upper_tick) {
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

    state::add_position(deps.storage, &info.sender, &position)?;

    state::update_tick(deps.storage, &pool_key, lower_tick.index, &lower_tick)?;
    state::update_tick(deps.storage, &pool_key, upper_tick.index, &upper_tick)?;

    let msgs = vec![
        WasmMsg::Execute {
            contract_addr: pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount: x.into(),
            })?,
            funds: vec![],
        },
        WasmMsg::Execute {
            contract_addr: pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: info.sender.to_string(),
                recipient: env.contract.address.to_string(),
                amount: y.into(),
            })?,
            funds: vec![],
        },
    ];

    let event_attributes = vec![
        attr("action", "create_position"),
        attr("address", info.sender.as_str()),
        attr("liquidity", liquidity_delta.to_string()),
        attr("lower_tick", lower_tick.index.to_string()),
        attr("upper_tick", upper_tick.index.to_string()),
        attr("current_sqrt_price", pool.sqrt_price.to_string()),
    ];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
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
/// - Fails if pool does not
#[allow(clippy::too_many_arguments)]
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
    let mut msgs = vec![];

    let CalculateSwapResult {
        amount_in,
        amount_out,
        ..
    } = swap_internal(
        deps.storage,
        &mut msgs,
        &info.sender,
        &env.contract.address,
        env.block.time.nanos(),
        &pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "swap")
        .add_attribute("amount_in", amount_in.to_string())
        .add_attribute("amount_out", amount_out.to_string()))
}

/// Performs atomic swap involving several pools based on the provided parameters.
///
/// # Parameters
/// - `amount_in`: The amount of tokens that the user wants to swap.
/// - `expected_amount_out`: The amount of tokens that the user wants to receive as a result of the swaps.
/// - `slippage`: The max acceptable percentage difference between the expected and actual amount of output tokens in a trade, not considering square root of target price as in the case of a swap.
/// - `swaps`: A vector containing all parameters needed to identify separate swap steps.
///
/// # Events
/// - On every successful swap, emits a `Swap` event for the freshly made swap.
/// - On every successful swap, emits a `Cross Tick` event for every single tick crossed.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the user would receive zero tokens.
/// - Fails if the allowance is insufficient or the user balance transfer fails.
/// - Fails if the minimum amount out after a single swap is insufficient to perform the next swap to achieve the expected amount out.
/// - Fails if pool does not exist
///
/// # External contracts
pub fn swap_route(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_in: TokenAmount,
    expected_amount_out: TokenAmount,
    slippage: Percentage,
    swaps: Vec<SwapHop>,
) -> Result<Response, ContractError> {
    let mut msgs = vec![];
    let amount_out = route(deps, env, info, &mut msgs, true, amount_in, swaps)?;

    let min_amount_out = calculate_min_amount_out(expected_amount_out, slippage);

    if amount_out < min_amount_out {
        return Err(ContractError::AmountUnderMinimumAmountOut);
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "swap_route")
        .add_attribute("amount_out", amount_out.to_string()))
}

/// Transfers a position between users.
///
/// # Parameters
/// - `index`: The index of the user position to transfer.
/// - `receiver`: An `AccountId` identifying the user who will own the position.
pub fn transfer_position(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    index: u32,
    receiver: String,
) -> Result<Response, ContractError> {
    let caller = info.sender;

    let position = state::get_position(deps.storage, &caller, index)?;

    state::remove_position(deps.storage, &caller, index)?;

    let receiver_addr = deps.api.addr_validate(&receiver)?;
    state::add_position(deps.storage, &receiver_addr, &position)?;

    Ok(Response::new().add_attribute("action", "transfer_position"))
}

/// Allows an authorized user (owner of the position) to claim collected fees.
///
/// # Parameters
/// - `index`: The index of the user position from which fees will be claimed.
///
/// # Errors
/// - Fails if the position cannot be found.
pub fn claim_fee(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let caller = info.sender;
    let current_timestamp = env.block.time.nanos();

    let mut position = state::get_position(deps.storage, &caller, index)?;

    let mut lower_tick =
        state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let mut upper_tick =
        state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;
    let pool_key_db = position.pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    let (x, y) = position.claim_fee(
        &mut pool,
        &mut upper_tick,
        &mut lower_tick,
        current_timestamp,
    )?;

    state::update_position(deps.storage, &caller, index, &position)?;
    POOLS.save(deps.storage, &pool_key_db, &pool)?;
    state::update_tick(
        deps.storage,
        &position.pool_key,
        upper_tick.index,
        &upper_tick,
    )?;
    state::update_tick(
        deps.storage,
        &position.pool_key,
        lower_tick.index,
        &lower_tick,
    )?;

    let mut msgs = vec![];

    if !x.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: position.pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: caller.to_string(),
                amount: x.into(),
            })?,
            funds: vec![],
        });
    }

    if !y.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: position.pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: caller.to_string(),
                amount: y.into(),
            })?,
            funds: vec![],
        });
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim_fee")
        .add_attribute("amount_x", x.to_string())
        .add_attribute("amount_y", y.to_string()))
}

/// Removes a position. Sends tokens associated with specified position to the owner.
///
/// # Parameters
/// - `index`: The index of the user position to be removed.
///
/// # Events
/// - Emits a `Remove Position` event upon success.
///
/// # Errors
/// - Fails if Position cannot be found
pub fn remove_position(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.nanos();

    let mut position = state::get_position(deps.storage, &info.sender, index)?;
    let withdrawed_liquidity = position.liquidity;

    let mut lower_tick =
        state::get_tick(deps.storage, &position.pool_key, position.lower_tick_index)?;
    let mut upper_tick =
        state::get_tick(deps.storage, &position.pool_key, position.upper_tick_index)?;

    let pool_key_db = position.pool_key.key();
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    let (amount_x, amount_y, deinitialize_lower_tick, deinitialize_upper_tick) = position.remove(
        &mut pool,
        current_timestamp,
        &mut lower_tick,
        &mut upper_tick,
        position.pool_key.fee_tier.tick_spacing,
    )?;

    POOLS.save(deps.storage, &pool_key_db, &pool)?;

    if deinitialize_lower_tick {
        remove_tick_and_flip_bitmap(deps.storage, &position.pool_key, &lower_tick)?;
    } else {
        state::update_tick(
            deps.storage,
            &position.pool_key,
            position.lower_tick_index,
            &lower_tick,
        )?;
    }

    if deinitialize_upper_tick {
        remove_tick_and_flip_bitmap(deps.storage, &position.pool_key, &upper_tick)?;
    } else {
        state::update_tick(
            deps.storage,
            &position.pool_key,
            position.upper_tick_index,
            &upper_tick,
        )?;
    }
    let position = state::remove_position(deps.storage, &info.sender, index)?;

    let mut msgs = vec![];

    if !amount_x.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: position.pool_key.token_x.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount_x.into(),
            })?,
            funds: vec![],
        });
    }

    if !amount_y.is_zero() {
        msgs.push(WasmMsg::Execute {
            contract_addr: position.pool_key.token_y.to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: amount_y.into(),
            })?,
            funds: vec![],
        });
    }

    let event_attributes = vec![
        attr("action", "remove_position"),
        attr("address", info.sender.as_str()),
        attr("liquidity", withdrawed_liquidity.to_string()),
        attr("lower_tick", lower_tick.index.to_string()),
        attr("upper_tick", upper_tick.index.to_string()),
        attr("current_sqrt_price", pool.sqrt_price.to_string()),
    ];

    Ok(Response::new()
        .add_messages(msgs)
        .add_attributes(event_attributes))
}

/// Allows a user to create a custom pool on a specified token pair and fee tier.
/// The contract specifies the order of tokens as x and y, the lower token address assigned as token x.
/// The choice is deterministic.
///
/// # Parameters
/// - `token_0`: The address of the first token.
/// - `token_1`: The address of the second token.
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
/// - `init_sqrt_price`: The square root of the price for the initial pool related to `init_tick`.
/// - `init_tick`: The initial tick at which the pool will be created.
///
/// # Errors
/// - Fails if the specified fee tier cannot be found.
/// - Fails if the user attempts to create a pool for the same tokens.
/// - Fails if Pool with same tokens and fee tier already exist.
/// - Fails if the init tick is not divisible by the tick spacing.
/// - Fails if the init sqrt price is not related to the init tick.
#[allow(clippy::too_many_arguments)]
pub fn create_pool(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    token_0: Addr,
    token_1: Addr,
    fee_tier: FeeTier,
    init_sqrt_price: SqrtPrice,
    init_tick: i32,
) -> Result<Response, ContractError> {
    let current_timestamp = env.block.time.nanos();

    let config = CONFIG.load(deps.storage)?;

    if !config.fee_tiers.contains(&fee_tier) {
        return Err(ContractError::FeeTierNotFound);
    }

    check_tick(init_tick, fee_tier.tick_spacing).map_err(|_| ContractError::InvalidInitTick)?;

    let pool_key =
        PoolKey::new(token_0, token_1, fee_tier).map_err(|_| ContractError::InvalidPoolKey)?;

    if POOLS.may_load(deps.storage, &pool_key.key())?.is_some() {
        return Err(ContractError::PoolAlreadyExist);
    };

    let config = CONFIG.load(deps.storage)?;

    let pool = Pool::create(
        init_sqrt_price,
        init_tick,
        current_timestamp,
        fee_tier.tick_spacing,
        config.admin,
    )
    .map_err(|_| ContractError::CreatePoolError)?;

    POOLS.save(deps.storage, &pool_key.key(), &pool)?;

    Ok(Response::new().add_attribute("action", "create_pool"))
}

/// Simulates multiple swaps without its execution.
///
/// # Parameters
/// - `amount_in`: The amount of tokens that the user wants to swap.
/// - `swaps`: A vector containing all parameters needed to identify separate swap steps.
///
/// # Errors
/// - Fails if the user attempts to perform a swap with zero amounts.
/// - Fails if the user would receive zero tokens.
/// - Fails if pool does not exist
pub fn quote_route(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<Response, ContractError> {
    let mut msgs = vec![];

    let amount_out = route(deps, env, info, &mut msgs, false, amount_in, swaps)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "quote_route")
        .add_attribute("amount_out", amount_out.to_string()))
}

/// Allows admin to add a custom fee tier.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if an unauthorized user attempts to create a fee tier.
/// - Fails if the tick spacing is invalid.
/// - Fails if the fee tier already exists.
/// - Fails if fee is invalid
pub fn add_fee_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    fee_tier: FeeTier,
) -> Result<Response, ContractError> {
    let caller = info.sender;

    let mut config = CONFIG.load(deps.storage)?;

    if fee_tier.tick_spacing == 0 || fee_tier.tick_spacing > 100 {
        return Err(ContractError::InvalidTickSpacing);
    }

    if fee_tier.fee >= Percentage::new(1000000000000) {
        // 100% -> fee invalid
        return Err(ContractError::InvalidFee);
    }

    if caller != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    config.fee_tiers.push(fee_tier);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "add_fee_tier"))
}

/// Removes an existing fee tier.
///
/// # Parameters
/// - `fee_tier`: A struct identifying the pool fee and tick spacing.
///
/// # Errors
/// - Fails if an unauthorized user attempts to remove a fee tier.
/// - Fails if fee tier does not exist
pub fn remove_fee_tier(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    fee_tier: FeeTier,
) -> Result<Response, ContractError> {
    let caller = info.sender;

    let mut config = CONFIG.load(deps.storage)?;

    if caller != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(pos) = config.fee_tiers.iter().position(|x| *x == fee_tier) {
        config.fee_tiers.remove(pos);
    } else {
        return Err(ContractError::FeeTierNotFound);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "remove_fee_tier"))
}
