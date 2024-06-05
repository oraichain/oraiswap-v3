#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Decimal256};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw721::{AllNftInfoResponse, Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse};
use ruint::Uint;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::interface::{AssetInfo, Cw721BaseExecuteMsg, NftExtentions};
use crate::libraries::{
    add_delta, fixed_point_64, get_delta_amount_0_unsigned, get_delta_amount_1_unsigned,
    get_sqrt_price_at_tick, get_tick_at_sqrt_price, swap_math, tick_math, MulDiv, SwapStep,
    MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64, U128,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
