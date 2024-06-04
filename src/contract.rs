#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, Decimal256};
use cosmwasm_std::{
    from_slice, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Storage, Uint128, Uint256, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use cw721::{AllNftInfoResponse, Cw721ExecuteMsg, Cw721QueryMsg, Cw721ReceiveMsg, NftInfoResponse};
use ruint::Uint;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::interface::{AssetInfo, Cw721BaseExecuteMsg, NftExtentions};
use crate::libraries::{
    fixed_point_64, get_sqrt_price_at_tick, get_tick_at_sqrt_price, swap_math, tick_math, MulDiv,
    SwapStep, MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64, U128,
};
use crate::msg::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    Config, CurrentState, FeeGrowthGlobal, NftInfo, Position, StepComputations, SwapState,
    TickInfo, CONFIG, CURRENT_STATE, FEE_GROWTH_GROBAL, LIST_INITIALIZED_TICKS, NFT_INFO,
    POSITIONS, TICKS,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let tick_spacing = msg.tick_spacing;
    let num_ticks = (u32::try_from(tick_math::MAX_TICK).unwrap() / tick_spacing) * 2 + 1;
    let max_liquidity_per_tick =
        Uint128::try_from(Decimal256::from_ratio(Uint128::MAX, num_ticks).to_uint_floor()).unwrap();

    CONFIG.save(
        deps.storage,
        &Config {
            factory: info.sender,
            token_0: msg.token_0,
            token_1: msg.token_1,
            fee: msg.fee,
            tick_spacing,
            max_liquidity_per_tick,
        },
    )?;

    NFT_INFO.save(
        deps.storage,
        &NftInfo {
            nft_address: msg.nft_address,
            last_id: 0,
        },
    )?;

    LIST_INITIALIZED_TICKS.save(deps.storage, &vec![])?;

    FEE_GROWTH_GROBAL.save(
        deps.storage,
        &FeeGrowthGlobal {
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
        },
    )?;
    Ok(Response::default())
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
            lp_amount,
        } => execute_mint(
            deps, info, env, recipient, tick_lower, tick_upper, lp_amount,
        ),
        ExecuteMsg::ReceiveNft(msg) => receive_cw721(deps, env, info, msg),
        ExecuteMsg::Collect { token_ids } => execute_collect(deps, token_ids),
        ExecuteMsg::Swap {
            recipient,
            is_base_input,
            zero_for_one,
            amount_specified,
            sqrt_price_limit_x64,
        } => execute_swap(
            deps,
            env,
            info,
            recipient,
            zero_for_one,
            is_base_input,
            amount_specified,
            sqrt_price_limit_x64,
        ),
    }
}

fn execute_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: Addr,
    zero_for_one: bool,
    is_base_input: bool,
    amount_specified: u64,
    sqrt_price_limit_x64: u128,
) -> Result<Response, ContractError> {
    if amount_specified == 0 {
        return Err(ContractError::ZeroAmount {});
    }

    let mut current_state = CURRENT_STATE.load(deps.storage)?;

    if zero_for_one {
        if sqrt_price_limit_x64 >= current_state.sqrt_price_x64
            && sqrt_price_limit_x64 > MIN_SQRT_PRICE_X64
        {
            return Err(ContractError::InvalidPriceLimit {});
        }
    } else if sqrt_price_limit_x64 <= current_state.sqrt_price_x64
        && sqrt_price_limit_x64 < MAX_SQRT_PRICE_X64
    {
        return Err(ContractError::InvalidPriceLimit {});
    }

    let cache_liquidity = current_state.liquidity;

    let mut fee_growth_global = FEE_GROWTH_GROBAL.load(deps.storage).unwrap();
    let fee_growth_global_x64 = if zero_for_one {
        fee_growth_global.fee_growth_global_0_x64
    } else {
        fee_growth_global.fee_growth_global_1_x64
    };
    let mut state = SwapState {
        amount_specified_remaining: amount_specified,
        amount_calculated: 0,
        sqrt_price_x64: current_state.sqrt_price_x64,
        tick: current_state.tick,
        fee_growth_global_x64,
        liquidity: current_state.liquidity,
    };
    let list_initialized_ticks = LIST_INITIALIZED_TICKS.load(deps.storage)?;
    let config = CONFIG.load(deps.storage).unwrap();

    while state.amount_specified_remaining != 0 && state.sqrt_price_x64 != sqrt_price_limit_x64 {
        let mut step = StepComputations {
            sqrt_price_start_x64: 0,
            tick_next: 0,
            initialized: false,
            sqrt_price_next_x64: 0,
            amount_in: 0,
            amount_out: 0,
            fee_amount: 0,
        };
        step.sqrt_price_start_x64 = state.sqrt_price_x64;
        // let mut compressed = state.tick / i32::try_from(config.tick_spacing).unwrap();
        // if state.tick < 0 && state.tick % i32::try_from(config.tick_spacing).unwrap() != 0 {
        //     compressed = compressed - 1;
        // }
        if zero_for_one {
            let pos = list_initialized_ticks.binary_search(&current_state.tick);
            step.tick_next = match pos {
                Ok(index) => {
                    if index >= 1 {
                        step.initialized = true;
                        list_initialized_ticks[index - 1]
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    return Err(ContractError::InvalidPriceLimit {});
                }
            }
        } else {
            let pos = list_initialized_ticks.binary_search(&current_state.tick);
            step.tick_next = match pos {
                Ok(index) => {
                    if index > 0 && index + 1 <= list_initialized_ticks.len() {
                        step.initialized = true;
                        list_initialized_ticks[index - 1]
                    } else {
                        break;
                    }
                }
                Err(_) => {
                    return Err(ContractError::InvalidPriceLimit {});
                }
            }
        }

        // Bound tick next
        if step.tick_next < tick_math::MIN_TICK {
            step.tick_next = tick_math::MIN_TICK;
        } else if step.tick_next > tick_math::MAX_TICK {
            step.tick_next = tick_math::MAX_TICK;
        }
        step.sqrt_price_next_x64 = tick_math::get_sqrt_price_at_tick(step.tick_next)?;
        let precheck = if zero_for_one {
            step.sqrt_price_next_x64 < sqrt_price_limit_x64
        } else {
            step.sqrt_price_next_x64 > sqrt_price_limit_x64
        };

        let new_sqrt_ratio_target_x64;
        if precheck {
            new_sqrt_ratio_target_x64 = sqrt_price_limit_x64;
        } else {
            new_sqrt_ratio_target_x64 = step.sqrt_price_next_x64;
        }

        let SwapStep {
            sqrt_price_next_x64,
            amount_in,
            amount_out,
            fee_amount,
        } = swap_math::compute_swap_step(
            state.sqrt_price_x64,
            new_sqrt_ratio_target_x64,
            state.liquidity,
            state.amount_specified_remaining,
            config.fee.into(),
            is_base_input,
            zero_for_one,
        );

        state.sqrt_price_x64 = sqrt_price_next_x64;
        step.amount_in = amount_in;
        step.amount_out = amount_out;
        step.fee_amount = fee_amount;

        if is_base_input {
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(step.amount_in + step.fee_amount)
                .unwrap();

            state.amount_calculated = state
                .amount_calculated
                .checked_sub(step.amount_out)
                .unwrap();
        } else {
            state.amount_specified_remaining += step.amount_out;
            state.amount_calculated += step.amount_in + step.fee_amount;
        }

        if state.liquidity > 0u128 {
            let fee_growth_global_x64_delta = U128::from(step.fee_amount)
                .mul_div_floor(U128::from(fixed_point_64::Q64), U128::from(state.liquidity))
                .unwrap()
                .as_u128();

            state.fee_growth_global_x64 += state
                .fee_growth_global_x64
                .checked_add(fee_growth_global_x64_delta)
                .unwrap();
        }

        let mut ticks = TICKS.load(deps.storage, step.tick_next).unwrap();

        if state.sqrt_price_x64 == step.sqrt_price_next_x64 {
            if step.initialized {
                let var2;
                let var3;
                if zero_for_one {
                    var2 = state.fee_growth_global_x64;
                    var3 = fee_growth_global.fee_growth_global_0_x64;
                } else {
                    var2 = fee_growth_global.fee_growth_global_1_x64;
                    var3 = state.fee_growth_global_x64
                }
                ticks = cross(ticks.clone(), var2, var3);
                let liquidity_net;
                if zero_for_one {
                    liquidity_net = ticks.liquidity_net * (-1);
                } else {
                    liquidity_net = ticks.liquidity_net;
                }
                state.liquidity = if liquidity_net < 0 {
                    state.liquidity - u128::try_from(liquidity_net.abs()).unwrap()
                } else {
                    state.liquidity + u128::try_from(liquidity_net).unwrap()
                };
                state.tick = if zero_for_one {
                    step.tick_next - 1
                } else {
                    step.tick_next
                }
            }
        } else if state.sqrt_price_x64 != step.sqrt_price_start_x64 {
            state.tick = get_tick_at_sqrt_price(state.sqrt_price_x64)?;
        }

        TICKS.save(deps.storage, step.tick_next, &ticks)?;
    }

    // Update sqrtPriceX96 and tick
    if state.tick != current_state.tick {
        current_state.sqrt_price_x64 = state.sqrt_price_x64;
        current_state.tick = state.tick;
    } else {
        current_state.sqrt_price_x64 = state.sqrt_price_x64;
    }

    // Update liquidity
    if cache_liquidity != state.liquidity {
        current_state.liquidity = state.liquidity;
    }

    if zero_for_one {
        fee_growth_global.fee_growth_global_0_x64 = state.fee_growth_global_x64;
    } else {
        fee_growth_global.fee_growth_global_1_x64 = state.fee_growth_global_x64
    }

    CURRENT_STATE.save(deps.storage, &current_state)?;
    FEE_GROWTH_GROBAL.save(deps.storage, &fee_growth_global)?;
    // Set amount0 and amount1
    // zero for one | exact input |
    //    true      |    true     | amount 0 = specified - remaining (> 0)
    //              |             | amount 1 = calculated            (< 0)
    //    false     |    false    | amount 0 = specified - remaining (< 0)
    //              |             | amount 1 = calculated            (> 0)
    //    false     |    true     | amount 0 = calculated            (< 0)
    //              |             | amount 1 = specified - remaining (> 0)
    //    true      |    false    | amount 0 = calculated            (> 0)
    //              |             | amount 1 = specified - remaining (< 0)

    let (amount0, amount1) = if zero_for_one == is_base_input {
        (
            amount_specified - state.amount_specified_remaining,
            state.amount_calculated,
        )
    } else {
        (
            state.amount_calculated,
            amount_specified - state.amount_specified_remaining,
        )
    };

    let mut messages: Vec<CosmosMsg> = vec![];
    if zero_for_one {
        let amount1_format = Uint128::from(amount1);
        let amount0_format = Uint128::from(amount0);
        match config.token_0 {
            AssetInfo::NativeToken { denom } => {
                if !(info.funds.contains(&Coin {
                    denom: denom.clone(),
                    amount: amount0_format,
                })) {
                    return Err(ContractError::InvalidFunds {});
                }
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: amount1_format,
                    })?,
                    funds: vec![],
                }));
            }
        }
        match config.token_1 {
            AssetInfo::NativeToken { denom } => {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: amount1_format,
                    }],
                }));
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount: amount1_format,
                    })?,
                    funds: vec![],
                }));
            }
        }
    } else {
        let amount0_format = Uint128::from(amount0);
        let amount1_format = Uint128::from(amount1);
        match config.token_1 {
            AssetInfo::NativeToken { denom } => {
                if !(info.funds.contains(&Coin {
                    denom: denom.clone(),
                    amount: amount1_format,
                })) {
                    return Err(ContractError::InvalidFunds {});
                }
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                        owner: info.sender.to_string(),
                        recipient: env.contract.address.to_string(),
                        amount: amount1_format,
                    })?,
                    funds: vec![],
                }));
            }
        }
        match config.token_0 {
            AssetInfo::NativeToken { denom } => {
                messages.push(CosmosMsg::Bank(BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: vec![Coin {
                        denom,
                        amount: amount0_format,
                    }],
                }));
            }
            AssetInfo::Token { contract_addr } => {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount: amount0_format,
                    })?,
                    funds: vec![],
                }));
            }
        }
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("amount1", &amount1.to_string()),
            ("amount0", &amount0.to_string()),
            ("recipient", &recipient.to_string()),
        ])
        .add_messages(messages))
}

fn cross(
    mut cur_tick_info: TickInfo,
    fee_growth_global_0_x64: u128,
    fee_growth_global_1_x64: u128,
) -> TickInfo {
    cur_tick_info.fee_growth_outside_0_x64 =
        fee_growth_global_0_x64 - cur_tick_info.fee_growth_outside_0_x64;
    cur_tick_info.fee_growth_outside_1_x64 =
        fee_growth_global_1_x64 - cur_tick_info.fee_growth_outside_1_x64;
    cur_tick_info
}

//     /// Transitions to the current tick as needed by price movement, returning the amount of liquidity
//     /// added (subtracted) when tick is crossed from left to right (right to left)
//     pub fn cross(
//         &mut self,
//         fee_growth_global_0_x64: u128,
//         fee_growth_global_1_x64: u128,
//         reward_infos: &[RewardInfo; REWARD_NUM],
//     ) -> i128 {
//         self.fee_growth_outside_0_x64 = fee_growth_global_0_x64
//             .checked_sub(self.fee_growth_outside_0_x64)
//             .unwrap();
//         self.fee_growth_outside_1_x64 = fee_growth_global_1_x64
//             .checked_sub(self.fee_growth_outside_1_x64)
//             .unwrap();

//         for i in 0..REWARD_NUM {
//             if !reward_infos[i].initialized() {
//                 continue;
//             }

//             self.reward_growths_outside_x64[i] = reward_infos[i]
//                 .reward_growth_global_x64
//                 .checked_sub(self.reward_growths_outside_x64[i])
//                 .unwrap();
//         }

//         self.liquidity_net
//     }

fn update_tick_and_list_initialized_tick(
    current_tick: i32,
    tick: i32,
    tick_info: Option<TickInfo>,
    mut list_initialized_ticks: Vec<i32>,
    fee_growth_global: FeeGrowthGlobal,
    liquidity_delta: i128,
    upper: bool,
) -> (TickInfo, Vec<i32>, bool) {
    let mut remove_tick_info = false;
    let mut new_tick_info = match tick_info {
        Some(tick_info) => tick_info,
        None => {
            // ADD TICK TO LIST HERE
            // list_initialzed_ticks.push(tick_lower);
            // list_initialzed_ticks.sort();

            let pos = list_initialized_ticks
                .binary_search(&tick)
                .unwrap_or_else(|e| e);
            list_initialized_ticks.insert(pos, tick);

            if tick <= current_tick {
                TickInfo {
                    liquidity_gross: 0,
                    liquidity_net: 0,
                    fee_growth_outside_0_x64: fee_growth_global.fee_growth_global_0_x64,
                    fee_growth_outside_1_x64: fee_growth_global.fee_growth_global_1_x64,
                }
            } else {
                TickInfo {
                    liquidity_gross: 0,
                    liquidity_net: 0,
                    fee_growth_outside_0_x64: 0,
                    fee_growth_outside_1_x64: 0,
                }
            }
        }
    };

    if liquidity_delta != 0 {
        new_tick_info.liquidity_gross =
            crate::libraries::add_delta(new_tick_info.liquidity_gross, liquidity_delta).unwrap();

        if upper {
            new_tick_info.liquidity_net -= liquidity_delta;
        } else {
            new_tick_info.liquidity_net += liquidity_delta;
        }

        if (new_tick_info.liquidity_gross == 0)
            == (u128::try_from(new_tick_info.liquidity_net.abs()).unwrap() == 0)
        {
            let pos = list_initialized_ticks.binary_search(&tick).unwrap();
            list_initialized_ticks.remove(pos);
            remove_tick_info = true;
        }
    }

    (new_tick_info, list_initialized_ticks, remove_tick_info)
}

// fn modiy_position(
//     storage: &mut dyn Storage,
//     tick_lower: i32,
//     tick_upper: i32,
//     liquidity_delta: i128,
//     position: Option<Position>,
// ) -> Result<(Position, Uint128, Uint128), ContractError> {
//     if !(tick_lower < tick_upper
//         && tick_lower >= tick_math::MIN_TICK
//         && tick_upper <= tick_math::MAX_TICK)
//     {
//         return Err(ContractError::InvalidTicks {});
//     }

//     let mut current_state = match CURRENT_STATE.may_load(storage)? {
//         Some(current_state) => current_state,
//         None => {
//             let tick_lower_sqrt_price_x64 = get_sqrt_price_at_tick(tick_lower).unwrap();
//             let tick_upper_sqrt_price_x64 = get_sqrt_price_at_tick(tick_upper).unwrap();
//             let sqrt_price_x64 = (tick_lower_sqrt_price_x64 + tick_upper_sqrt_price_x64) / 2;
//             CurrentState {
//                 liquidity: 0,
//                 sqrt_price_x64: sqrt_price_x64,
//                 tick: get_tick_at_sqrt_price(sqrt_price_x64).unwrap(),
//             }
//         }
//     };
//     let fee_growth_global = FEE_GROWTH_GROBAL.load(storage)?;

//     // UPDATE TICK HERE
//     let list_initialized_ticks = LIST_INITIALIZED_TICKS.load(storage)?;
//     let (tick_lower_info, list_initialized_ticks, remove_lower) =
//         update_tick_and_list_initialized_tick(
//             current_state.tick,
//             tick_lower,
//             TICKS.may_load(storage, tick_lower)?,
//             list_initialized_ticks,
//             fee_growth_global.clone(),
//             liquidity_delta,
//             false,
//         );

//     let (tick_upper_info, list_initialized_ticks, remove_upper) =
//         update_tick_and_list_initialized_tick(
//             current_state.tick,
//             tick_upper,
//             TICKS.may_load(storage, tick_upper)?,
//             list_initialized_ticks,
//             fee_growth_global.clone(),
//             liquidity_delta,
//             true,
//         );

//     LIST_INITIALIZED_TICKS.save(storage, &list_initialized_ticks)?;
//     if remove_lower {
//         TICKS.remove(storage, tick_lower)
//     } else {
//         TICKS.save(storage, tick_lower, &tick_lower_info)?
//     }

//     if remove_upper {
//         TICKS.remove(storage, tick_upper)
//     } else {
//         TICKS.save(storage, tick_upper, &tick_upper_info)?
//     }

//     // UPDATE POSITION HERE
//     let (fee_growth_inside_0_x128, fee_growth_inside_1_x128) = get_fee_growth_inside(
//         FeeGrowthOutside {
//             fee_growth_outside0_x128: Uint::<256, 4>::from_be_bytes(
//                 tick_lower_info.fee_growth_outside_0_x64.to_be_bytes(),
//             ),
//             fee_growth_outside1_x128: Uint::<256, 4>::from_be_bytes(
//                 tick_lower_info.fee_growth_outside_1_x64.to_be_bytes(),
//             ),
//         },
//         FeeGrowthOutside {
//             fee_growth_outside0_x128: Uint::<256, 4>::from_be_bytes(
//                 tick_upper_info.fee_growth_outside_0_x64.to_be_bytes(),
//             ),
//             fee_growth_outside1_x128: Uint::<256, 4>::from_be_bytes(
//                 tick_upper_info.fee_growth_outside_1_x64.to_be_bytes(),
//             ),
//         },
//         tick_lower,
//         tick_upper,
//         current_state.tick,
//         Uint::<256, 4>::from_be_bytes(fee_growth_global.fee_growth_global_0_x64.to_be_bytes()),
//         Uint::<256, 4>::from_be_bytes(fee_growth_global.fee_growth_global_1_x64.to_be_bytes()),
//     );

//     let new_position = match position {
//         Some(mut position) => {
//             let (token_owned_0, token_owned_1) = get_tokens_owed(
//                 Uint::<256, 4>::from_be_bytes(position.fee_growth_inside_0_last_x64.to_be_bytes()),
//                 Uint::<256, 4>::from_be_bytes(position.fee_growth_inside_1_last_x64.to_be_bytes()),
//                 position.liquidity,
//                 fee_growth_inside_0_x128,
//                 fee_growth_inside_1_x128,
//             );

//             position.liquidity = add_delta(position.liquidity, liquidity_delta).unwrap();
//             position.fee_growth_inside_0_last_x64 =
//                 Uint256::from_be_bytes(fee_growth_inside_0_x128.to_be_bytes());
//             position.fee_growth_inside_1_last_x64 =
//                 Uint256::from_be_bytes(fee_growth_inside_1_x128.to_be_bytes());
//             position.tokens_owned_0 = Uint128::from(u128::from_be_bytes(
//                 <[u8; 16]>::try_from(&token_owned_0.to_be_bytes::<32>()[16..32]).unwrap(),
//             ));
//             position.tokens_owned_1 = Uint128::from(u128::from_be_bytes(
//                 <[u8; 16]>::try_from(&token_owned_1.to_be_bytes::<32>()[16..32]).unwrap(),
//             ));
//             position
//         }
//         None => Position {
//             liquidity: add_delta(0, liquidity_delta).unwrap(),
//             fee_growth_inside_0_last_x64: Uint256::from_be_bytes(
//                 fee_growth_inside_0_x128.to_be_bytes(),
//             ),
//             fee_growth_inside_1_last_x64: Uint256::from_be_bytes(
//                 fee_growth_inside_1_x128.to_be_bytes(),
//             ),
//             tokens_owned_0: Uint128::zero(),
//             tokens_owned_1: Uint128::zero(),
//         },
//     };

//     // GET amount_0 and amount_1 from liquidity_delta. update current_state.liquidity if needed

//     let (mut amount_0_uint128, mut amount_1_uint128) = (Uint128::zero(), Uint128::zero());

//     if liquidity_delta != 0 {
//         let (amount_0, amount_1) = if current_state.tick < tick_lower {
//             (
//                 get_amount_0_delta(
//                     get_sqrt_price_at_tick(tick_lower).unwrap(),
//                     get_sqrt_price_at_tick(tick_upper).unwrap(),
//                     u128::try_from(liquidity_delta.abs()).unwrap(),
//                     false,
//                 )
//                 .unwrap(),
//                 Uint::<256, 4>::ZERO,
//             )
//         } else if current_state.tick < tick_upper {
//             current_state.liquidity = add_delta(current_state.liquidity, liquidity_delta).unwrap();
//             CURRENT_STATE.save(storage, &current_state)?;
//             (
//                 get_amount_0_delta(
//                     Uint::<256, 4>::from_be_bytes(current_state.sqrt_price_x64.to_be_bytes()),
//                     get_sqrt_price_at_tick(tick_upper).unwrap(),
//                     u128::try_from(liquidity_delta.abs()).unwrap(),
//                     false,
//                 )
//                 .unwrap(),
//                 get_amount_1_delta(
//                     get_sqrt_price_at_tick(tick_lower).unwrap(),
//                     Uint::<256, 4>::from_be_bytes(current_state.sqrt_price_x64.to_be_bytes()),
//                     u128::try_from(liquidity_delta.abs()).unwrap(),
//                     false,
//                 )
//                 .unwrap(),
//             )
//         } else {
//             (
//                 Uint::<256, 4>::ZERO,
//                 get_amount_1_delta(
//                     get_sqrt_price_at_tick(tick_lower).unwrap(),
//                     get_sqrt_price_at_tick(tick_upper).unwrap(),
//                     u128::try_from(liquidity_delta.abs()).unwrap(),
//                     false,
//                 )
//                 .unwrap(),
//             )
//         };
//         amount_0_uint128 = Uint128::from(u128::from_be_bytes(
//             <[u8; 16]>::try_from(&amount_0.to_be_bytes::<32>()[16..32]).unwrap(),
//         ));
//         amount_1_uint128 = Uint128::from(u128::from_be_bytes(
//             <[u8; 16]>::try_from(&amount_1.to_be_bytes::<32>()[16..32]).unwrap(),
//         ));
//     }

//     Ok((new_position, amount_0_uint128, amount_1_uint128))
// }

fn execute_mint(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    recipient: Addr,
    tick_lower: i32,
    tick_upper: i32,
    lp_amount: i128,
) -> Result<Response, ContractError> {
    Ok(Response::default())

    //     let config = CONFIG.load(deps.storage)?;
    //     let mut nft_info = NFT_INFO.load(deps.storage)?;
    //     let mut messages: Vec<WasmMsg> = vec![];

    //     if !(tick_lower < tick_upper
    //         && tick_lower >= tick_math::MIN_TICK
    //         && tick_upper <= tick_math::MAX_TICK)
    //     {
    //         return Err(ContractError::InvalidTicks {});
    //     }

    //     if lp_amount == 0 {
    //         return Err(ContractError::ZeroAmount {});
    //     }

    //     let (new_position, amount_0, amount_1) =
    //         modiy_position(deps.storage, tick_lower, tick_upper, lp_amount, None)?;

    //     let mut nft_name = "".to_string();

    //     if amount_0 > Uint128::zero() {
    //         match config.token_0 {
    //             AssetInfo::NativeToken { denom } => {
    //                 if !(info.funds.contains(&Coin {
    //                     denom: denom.clone(),
    //                     amount: amount_0,
    //                 })) {
    //                     return Err(ContractError::InvalidFunds {});
    //                 }
    //                 nft_name.push_str(&denom);
    //                 nft_name.push('_');
    //             }
    //             AssetInfo::Token { contract_addr } => {
    //                 messages.push(WasmMsg::Execute {
    //                     contract_addr: contract_addr.to_string(),
    //                     msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
    //                         owner: info.sender.to_string(),
    //                         recipient: env.contract.address.to_string(),
    //                         amount: amount_0,
    //                     })?,
    //                     funds: vec![],
    //                 });
    //                 nft_name.push_str(contract_addr.as_ref());
    //                 nft_name.push('_');
    //             }
    //         }
    //     }

    //     if amount_1 > Uint128::zero() {
    //         match config.token_1 {
    //             AssetInfo::NativeToken { denom } => {
    //                 if !(info.funds.contains(&Coin {
    //                     denom: denom.clone(),
    //                     amount: amount_1,
    //                 })) {
    //                     return Err(ContractError::InvalidFunds {});
    //                 }
    //                 nft_name.push_str(&denom);
    //                 nft_name.push('_');
    //             }
    //             AssetInfo::Token { contract_addr } => {
    //                 messages.push(WasmMsg::Execute {
    //                     contract_addr: contract_addr.to_string(),
    //                     msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
    //                         owner: info.sender.to_string(),
    //                         recipient: env.contract.address.to_string(),
    //                         amount: amount_1,
    //                     })?,
    //                     funds: vec![],
    //                 });
    //                 nft_name.push_str(contract_addr.as_ref());
    //                 nft_name.push('_');
    //             }
    //         }
    //     }

    //     let nft_id = nft_info.last_id + 1;
    //     nft_name.push_str(&nft_id.to_string());

    //     messages.push(WasmMsg::Execute {
    //         contract_addr: nft_info.nft_address.to_string(),
    //         msg: to_binary(&Cw721BaseExecuteMsg::<NftExtentions>::Mint {
    //             token_id: nft_name.clone(),
    //             owner: recipient.to_string(),
    //             token_uri: None,
    //             extension: NftExtentions {
    //                 pool: env.contract.address.clone(),
    //                 tick_lower,
    //                 tick_upper,
    //             },
    //         })?,
    //         funds: vec![],
    //     });

    //     nft_info.last_id = nft_id;
    //     NFT_INFO.save(deps.storage, &nft_info)?;
    //     POSITIONS.save(deps.storage, nft_name.clone(), &new_position)?;

    //     Ok(Response::new()
    //         .add_attributes(vec![
    //             ("action", "mint"),
    //             ("pool", env.contract.address.as_ref()),
    //             ("tick_lower", &tick_lower.to_string()),
    //             ("tick_upper", &tick_upper.to_string()),
    //             ("token_id", &nft_name),
    //         ])
    //         .add_messages(messages))
}

fn receive_cw721(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw721_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    Ok(Response::default())
    //     match from_json(&cw721_msg.msg)? {
    //         Cw721HookMsg::Burn {} => {
    //             let nft_info = NFT_INFO.load(deps.storage)?;
    //             let nft_info_resposne: NftInfoResponse<NftExtentions> = deps.querier.query_wasm_smart(
    //                 nft_info.nft_address.clone(),
    //                 &Cw721QueryMsg::NftInfo {
    //                     token_id: cw721_msg.token_id.clone(),
    //                 },
    //             )?;
    //             let nft_extension = nft_info_resposne.extension;

    //             if info.sender != nft_info.nft_address || nft_extension.pool != env.contract.address {
    //                 return Err(ContractError::InvalidFunds {});
    //             }
    //             let position = POSITIONS.load(deps.storage, cw721_msg.token_id.clone())?;
    //             let (new_position, amount_0, amount_1) = modiy_position(
    //                 deps.storage,
    //                 nft_extension.tick_lower,
    //                 nft_extension.tick_upper,
    //                 -i128::try_from(position.liquidity).unwrap(),
    //                 Some(position),
    //             )?;

    //             let total_amount_0 = amount_0 + new_position.tokens_owned_0;
    //             let total_amount_1 = amount_1 + new_position.tokens_owned_1;

    //             let config = CONFIG.load(deps.storage)?;

    //             let mut messages: Vec<CosmosMsg> = vec![];

    //             if total_amount_0 > Uint128::zero() {
    //                 match config.token_0 {
    //                     AssetInfo::NativeToken { denom } => {
    //                         messages.push(CosmosMsg::Bank(BankMsg::Send {
    //                             to_address: cw721_msg.sender.clone(),
    //                             amount: vec![Coin {
    //                                 denom,
    //                                 amount: total_amount_0,
    //                             }],
    //                         }));
    //                     }
    //                     AssetInfo::Token { contract_addr } => {
    //                         messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //                             contract_addr: contract_addr.to_string(),
    //                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                                 recipient: cw721_msg.sender.clone(),
    //                                 amount: total_amount_0,
    //                             })?,
    //                             funds: vec![],
    //                         }));
    //                     }
    //                 }
    //             }

    //             if total_amount_1 > Uint128::zero() {
    //                 match config.token_1 {
    //                     AssetInfo::NativeToken { denom } => {
    //                         messages.push(CosmosMsg::Bank(BankMsg::Send {
    //                             to_address: cw721_msg.sender,
    //                             amount: vec![Coin {
    //                                 denom,
    //                                 amount: total_amount_1,
    //                             }],
    //                         }));
    //                     }
    //                     AssetInfo::Token { contract_addr } => {
    //                         messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //                             contract_addr: contract_addr.to_string(),
    //                             msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                                 recipient: cw721_msg.sender,
    //                                 amount: total_amount_1,
    //                             })?,
    //                             funds: vec![],
    //                         }));
    //                     }
    //                 }
    //             }

    //             messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //                 contract_addr: nft_info.nft_address.to_string(),
    //                 msg: to_binary(&Cw721ExecuteMsg::Burn {
    //                     token_id: cw721_msg.token_id.clone(),
    //                 })?,
    //                 funds: vec![],
    //             }));

    //             POSITIONS.remove(deps.storage, cw721_msg.token_id.clone());

    //             Ok(Response::new()
    //                 .add_attributes(vec![
    //                     ("action", "burn"),
    //                     ("pool", env.contract.address.as_ref()),
    //                     ("tick_lower", &nft_extension.tick_lower.to_string()),
    //                     ("tick_upper", &nft_extension.tick_upper.to_string()),
    //                     ("token_id", &cw721_msg.token_id),
    //                 ])
    //                 .add_messages(messages))
    //         }
    //     }
}

fn execute_collect(deps: DepsMut, token_ids: Vec<String>) -> Result<Response, ContractError> {
    Ok(Response::default())
    //     let config = CONFIG.load(deps.storage)?;
    //     let nft_info = NFT_INFO.load(deps.storage)?;

    //     let mut messages: Vec<CosmosMsg> = vec![];

    //     for token_id in token_ids {
    //         let all_nft_info_res: AllNftInfoResponse<NftExtentions> = deps.querier.query_wasm_smart(
    //             nft_info.nft_address.clone(),
    //             &Cw721QueryMsg::AllNftInfo {
    //                 token_id: token_id.clone(),
    //                 include_expired: None,
    //             },
    //         )?;

    //         let nft_info_extenstion = all_nft_info_res.info.extension;
    //         let position = POSITIONS.may_load(deps.storage, token_id.clone())?;

    //         let (mut new_position, _, _) = modiy_position(
    //             deps.storage,
    //             nft_info_extenstion.tick_lower,
    //             nft_info_extenstion.tick_upper,
    //             0,
    //             position,
    //         )?;
    //         let owner = all_nft_info_res.access.owner;
    //         match config.token_0.clone() {
    //             AssetInfo::NativeToken { denom } => {
    //                 messages.push(CosmosMsg::Bank(BankMsg::Send {
    //                     to_address: owner.clone(),
    //                     amount: vec![Coin {
    //                         denom,
    //                         amount: new_position.tokens_owned_0,
    //                     }],
    //                 }));
    //             }
    //             AssetInfo::Token { contract_addr } => {
    //                 messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //                     contract_addr: contract_addr.to_string(),
    //                     msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                         recipient: owner.clone(),
    //                         amount: new_position.tokens_owned_0,
    //                     })?,
    //                     funds: vec![],
    //                 }));
    //             }
    //         }
    //         match config.token_1.clone() {
    //             AssetInfo::NativeToken { denom } => {
    //                 messages.push(CosmosMsg::Bank(BankMsg::Send {
    //                     to_address: owner,
    //                     amount: vec![Coin {
    //                         denom,
    //                         amount: new_position.tokens_owned_1,
    //                     }],
    //                 }));
    //             }
    //             AssetInfo::Token { contract_addr } => {
    //                 messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
    //                     contract_addr: contract_addr.to_string(),
    //                     msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                         recipient: owner,
    //                         amount: new_position.tokens_owned_1,
    //                     })?,
    //                     funds: vec![],
    //                 }));
    //             }
    //         }
    //         new_position.tokens_owned_0 = Uint128::zero();
    //         new_position.tokens_owned_1 = Uint128::zero();
    //         POSITIONS.save(deps.storage, token_id, &new_position)?;
    //     }
    //     Ok(Response::new()
    //         .add_attributes(vec![("action", "collect")])
    //         .add_messages(messages))
}

// fn execute_swap(
//     deps: DepsMut,
//     recipient: Addr,
//     zero_for_one: bool,
//     amount_specified: Int256,
//     sqrt_price_limit_x64: Uint256,
// ) -> Result<Response, ContractError> {
//     if amount_specified == Int256::zero() {
//         return Err(ContractError::InvalidSqrtPriceLimitX96 {});
//     }

//     let current_state = CURRENT_STATE.load(deps.storage).unwrap();
//     if zero_for_one {
//         let min_sqrt_ratio_in_uint256 = Uint256::from_str(&MIN_SQRT_RATIO.to_string()).unwrap();
//         if !(sqrt_price_limit_x64 < current_state.sqrt_price_x64
//             && sqrt_price_limit_x64 > min_sqrt_ratio_in_uint256)
//         {
//             return Err(ContractError::InvalidSqrtPrice {});
//         }
//     } else {
//         let max_sqrt_ratio_in_uint256 = Uint256::from_str(&MAX_SQRT_RATIO.to_string()).unwrap();
//         if !(sqrt_price_limit_x64 > current_state.sqrt_price_x64
//             && sqrt_price_limit_x64 < max_sqrt_ratio_in_uint256)
//         {
//             return Err(ContractError::InvalidSqrtPrice {});
//         }
//     }

//     let exact_input = amount_specified > Int256::zero();

//     let fee_growth_global = FEE_GROWTH_GROBAL.load(deps.storage).unwrap();

//     let fee_growth_global_x64 = if zero_for_one {
//         fee_growth_global.fee_growth_global_0_x64
//     } else {
//         fee_growth_global.fee_growth_global_1_x64
//     };

//     let state = SwapState {
//         amount_specified_remaining: amount_specified,
//         amount_calculated: Int256::zero(),
//         sqrt_price_x64: current_state.sqrt_price_x64,
//         tick: current_state.tick,
//         fee_growth_global_x64,
//         liquidity: current_state.liquidity,
//     };

//     while state.amount_specified_remaining != Int256::zero()
//         && state.sqrt_price_x64 != sqrt_price_limit_x64
//     {
//         let mut step = StepComputations {
//             sqrt_price_start_x64: Uint256::zero(),
//             tick_next: 0,
//             initialized: false,
//             sqrt_price_next_x64: Uint256::zero(),
//             amount_in: Uint256::zero(),
//             amount_out: Uint256::zero(),
//             fee_amount: Uint256::zero(),
//         };
//         step.sqrt_price_start_x64 = state.sqrt_price_x64;
//     }

//     Ok(Response::new())
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

// #[cfg(test)]
// mod tests {
//     use std::str::FromStr;

//     use crate::contract::get_tick_at_sqrt_ratio;
//     use cosmwasm_std::Decimal256;
//     use cosmwasm_std::Int128;
//     use cosmwasm_std::Uint128;
//     use cosmwasm_std::Uint256;
//     use ruint::Uint;
//     use utils::MAX_TICK;
//     use utils::MIN_SQRT_RATIO;
//     use utils::MIN_TICK;

//     #[test]
//     fn test_1() {
//         // test individual values for correct results
//         let x = Uint256::from_u128(4295128739u128);
//         let y = Uint256::from_be_bytes(Uint::<256, 4>::from(4295128739u128).to_be_bytes());
//         let tick = get_tick_at_sqrt_ratio(Uint::<256, 4>::from_be_bytes(x.to_be_bytes())).unwrap();

//         assert_eq!(x, y, "incorrect");
//         assert_eq!(tick, MIN_TICK, "incorrect");
//     }

//     #[test]
//     fn test_2() {
//         // test individual values for correct results

//         let tick_spacing = 600;
//         let num_ticks = (u32::try_from(MAX_TICK).unwrap() / tick_spacing) * 2 + 1;
//         assert_eq!(num_ticks, 2957, "incorrect");
//     }

//     #[test]
//     fn test_3() {
//         // test individual values for correct results

//         let x: Int128 = Int128::try_from(Uint128::from(50u128)).unwrap();
//         let y: i128 = -i128::try_from(50u128).unwrap();
//         assert_eq!(x, Int128::from(50), "incorrect");
//         assert_eq!(y, -50, "incorrect");
//     }

//     #[test]
//     fn test_4() {
//         // test individual values for correct results

//         let x: u128 = u128::try_from(Uint128::from(50u128)).unwrap();
//         assert_eq!(x, 50_u128, "incorrect");
//     }
//     #[test]
//     fn test_5() {
//         let x = Uint::<256, 4>::from(5000_u128);
//         let y = Uint128::from(u128::from_be_bytes(
//             <[u8; 16]>::try_from(&x.to_be_bytes::<32>()[16..32]).unwrap(),
//         ));

//         assert_eq!(y, Uint128::from(5000_u128), "incorrect");
//     }
//     #[test]
//     fn test_6() {
//         let x = MIN_SQRT_RATIO.to_string();
//         let y = Uint256::from_str(&x).unwrap();
//         println!("{}", x);
//         println!("{}", y);
//     }
//     #[test]
//     fn test_7() {
//         let mut vec = vec![10, 20, 30, 40, 50];
//         let a = 25;
//         vec.sort();
//         let pos = vec.binary_search_by(|x| x.cmp(&a).then(std::cmp::Ordering::Greater));
//         match pos {
//             Ok(index) => println!("Found element exactly matching {}: {}", a, vec[index]),
//             Err(index) => {
//                 if index < vec.len() {
//                     println!("Smallest element greater than {}: {}", a, vec[index]);
//                 } else {
//                     println!("No element greater than {}", a);
//                 }
//             }
//         }
//     }
//     #[test]
//     fn test_8() {
//         let vec = vec![10, 20, 30, 40, 50];
//         let a = 25;
//         let sorted_vec = vec;
//         let pos = sorted_vec.binary_search(&a);
//         match pos {
//             Ok(index) => {
//                 if index + 1 < sorted_vec.len() {
//                     println!(
//                         "Smallest element greater than {}: {}",
//                         index,
//                         sorted_vec[index - 1]
//                     );
//                 } else {
//                     println!("No element greater than {}", a);
//                 }
//             }
//             Err(index) => {
//                 if index < sorted_vec.len() {
//                     println!("Smallest element greater than {}: {}", a, sorted_vec[index]);
//                 } else {
//                     println!("No element greater than {}", a);
//                 }
//             }
//         }
//     }
//     #[test]
//     fn test_9() {
//         let vec = vec![30, 40];
//         let a = 25;
//         let sorted_vec = vec;
//         let pos = sorted_vec.binary_search(&a);
//         match pos {
//             Ok(index) => {
//                 if index > 0 {
//                     println!(
//                         "Largest element smaller than {}: {}",
//                         a,
//                         sorted_vec[index - 1]
//                     );
//                 } else {
//                     println!("No element smaller than {}", a);
//                 }
//             }
//             Err(index) => {
//                 if index > 0 {
//                     println!(
//                         "Largest element smaller than {}: {}",
//                         a,
//                         sorted_vec[index - 1]
//                     );
//                 } else {
//                     println!("No element smaller than {}", a);
//                 }
//             }
//         }
//     }

//     #[test]
//     fn test_10() {
//         let tick_spacing = 200;
//         let num_ticks = (u32::try_from(MAX_TICK).unwrap() / tick_spacing) * 2 + 1;
//         let x = Uint128::try_from(Decimal256::from_ratio(Uint128::MAX, num_ticks).to_uint_floor())
//             .unwrap();
//         println!("{}", x);
//     }
// }
