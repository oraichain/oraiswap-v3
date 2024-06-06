#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::liquidity::Liquidity;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::sqrt_price::SqrtPrice;
use crate::state::{add_position, add_tick, flip_bitmap, get_tick, update_tick, CONFIG, POOLS};
use crate::{check_tick, Config, PoolKey, Position, Tick};

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
