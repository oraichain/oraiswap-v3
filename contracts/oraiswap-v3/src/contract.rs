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
            pool_key,
            x_to_y,
            amount,
            by_amount_in,
            sqrt_price_limit,
        ),
        ExecuteMsg::TransferPosition { index, receiver } => {
            transfer_position(deps, env, info, index, receiver)
        }
        ExecuteMsg::ClaimFee { index } => claim_fee(deps, env, info, index),
        ExecuteMsg::RemovePosition { index } => remove_pos(deps, env, info, index),
        ExecuteMsg::CreatePool {
            token_0,
            token_1,
            fee_tier,
            init_sqrt_price,
            init_tick,
        } => create_pool(
            deps,
            env,
            info,
            token_0,
            token_1,
            fee_tier,
            init_sqrt_price,
            init_tick,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProtocolFee {} => to_binary(&get_protocol_fee(deps)?),
        QueryMsg::QuoteRoute { amount_in, swaps } => {
            to_binary(&quote_route(deps, env, amount_in, swaps)?)
        }
    }
}

// fn route(
//     &mut self,
//     is_swap: bool,
//     amount_in: TokenAmount,
//     swaps: Vec<SwapHop>,
// ) -> Result<TokenAmount, InvariantError> {
//     let mut next_swap_amount = amount_in;

//     for swap in swaps.iter() {
//         let SwapHop { pool_key, x_to_y } = *swap;

//         let sqrt_price_limit = if x_to_y {
//             SqrtPrice::new(MIN_SQRT_PRICE)
//         } else {
//             SqrtPrice::new(MAX_SQRT_PRICE)
//         };

//         let result = if is_swap {
//             self.swap(pool_key, x_to_y, next_swap_amount, true, sqrt_price_limit)
//         } else {
//             self.calculate_swap(pool_key, x_to_y, next_swap_amount, true, sqrt_price_limit)
//         }?;

//         next_swap_amount = result.amount_out;
//     }

//     Ok(next_swap_amount)
// }

// fn swap_route(
//     deps: Deps,
//     amount_in: TokenAmount,
//     expected_amount_out: TokenAmount,
//     slippage: Percentage,
//     swaps: Vec<SwapHop>,
// ) -> Result<(), InvariantError> {
//     let amount_out = self.route(true, amount_in, swaps)?;

//     let min_amount_out = calculate_min_amount_out(expected_amount_out, slippage);

//     if amount_out < min_amount_out {
//         return Err(InvariantError::AmountUnderMinimumAmountOut);
//     }

//     Ok(())
// }
