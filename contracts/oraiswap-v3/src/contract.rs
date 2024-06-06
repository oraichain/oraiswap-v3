use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::percentage::Percentage;
use crate::state::{AMM_CONFIG, POOLS};
use crate::{AmmConfig, PoolKey};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
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
    let config = AmmConfig {
        admin: info.sender,
        protocol_fee: msg.protocol_fee,
    };
    AMM_CONFIG.save(deps.storage, &config)?;

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
    let caller = info.sender;
    let pool_key_db = pool_key.key(deps.api)?;
    let mut pool = POOLS.load(deps.storage, &pool_key_db)?;

    if pool.fee_receiver != caller {
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

fn get_protocol_fee(deps: Deps) -> StdResult<Percentage> {
    let config = AMM_CONFIG.load(deps.storage)?;
    Ok(config.protocol_fee)
}
