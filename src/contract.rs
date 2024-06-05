use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

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
    match msg {
        ExecuteMsg::Mint {
            recipient,
            tick_lower,
            tick_upper,
            liquidity,
        } => unimplemented!(),
        ExecuteMsg::Collect {
            recipient,
            tick_lower,
            tick_upper,
            amount_0_requested,
            amount_1_requested,
        } => unimplemented!(),
        ExecuteMsg::Burn {
            tick_lower,
            tick_upper,
            liquidity,
        } => unimplemented!(),
        ExecuteMsg::Swap {
            recipient,
            zero_for_one,
            amount_in,
            sqrt_price_limit_x64,
        } => unimplemented!(),
        ExecuteMsg::CollectProtocol {
            recipient,
            amount_0_requested,
            amount_1_requested,
        } => unimplemented!(),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PoolConfig {} => unimplemented!(),
        QueryMsg::Fee {} => unimplemented!(),
        QueryMsg::Slot0 {} => unimplemented!(),
        QueryMsg::FeeGrowthGlobalX64 {} => unimplemented!(),
        QueryMsg::ProtocolFee {} => unimplemented!(),
        QueryMsg::Ticks { tick } => unimplemented!(),
        QueryMsg::Position { key } => unimplemented!(),
        QueryMsg::Liquidity {} => unimplemented!(),
        QueryMsg::TickBitmap { word_position } => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {}
