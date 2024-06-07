#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::CONFIG;
use crate::{entrypoints::*, Config};

use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

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
            &pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        ),
        ExecuteMsg::QuoteRoute { amount_in, swaps } => {
            quote_route(deps, env, info, amount_in, swaps)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProtocolFee {} => to_binary(&get_protocol_fee(deps)?),
        QueryMsg::Position { owner_id, index } => to_binary(&get_position(deps, owner_id, index)?),
        QueryMsg::Positions {
            owner_id,
            limit,
            offset,
        } => to_binary(&get_positions(deps, owner_id, limit, offset)?),
    }
}
