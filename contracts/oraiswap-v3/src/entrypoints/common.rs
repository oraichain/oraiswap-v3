use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, Env, MessageInfo, StdResult, Storage,
    Timestamp, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use decimal::{CheckedOps, Decimal};
use oraiswap::asset::{Asset, AssetInfo};

use crate::{
    check_tick, compute_swap_step,
    interface::{CalculateSwapResult, SwapHop},
    sqrt_price::{get_max_tick, get_min_tick, SqrtPrice},
    state::{self, CONFIG, POOLS},
    token_amount::TokenAmount,
    ContractError, PoolKey, Tick, UpdatePoolTick, MAX_SQRT_PRICE, MAX_TICKMAP_QUERY_SIZE,
    MIN_SQRT_PRICE,
};

pub trait TimeStampExt {
    fn millis(&self) -> u64;
}

impl TimeStampExt for Timestamp {
    fn millis(&self) -> u64 {
        self.nanos() / 1_000_000
    }
}

pub trait TokenTransfer {
    fn transfer(&self, msgs: &mut Vec<CosmosMsg>, info: &MessageInfo) -> StdResult<()>;
    fn transfer_from(
        &self,
        msgs: &mut Vec<CosmosMsg>,
        info: &MessageInfo,
        recipient: String,
    ) -> StdResult<()>;
}

impl TokenTransfer for Asset {
    fn transfer(&self, msgs: &mut Vec<CosmosMsg>, info: &MessageInfo) -> StdResult<()> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: info.sender.to_string(),
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                AssetInfo::NativeToken { denom } => msgs.push(
                    BankMsg::Send {
                        to_address: info.sender.to_string(),
                        amount: vec![Coin {
                            amount: self.amount,
                            denom: denom.to_string(),
                        }],
                    }
                    .into(),
                ),
            }
        }
        Ok(())
    }

    fn transfer_from(
        &self,
        msgs: &mut Vec<CosmosMsg>,
        info: &MessageInfo,
        recipient: String,
    ) -> StdResult<()> {
        if !self.amount.is_zero() {
            match &self.info {
                AssetInfo::Token { contract_addr } => {
                    msgs.push(
                        WasmMsg::Execute {
                            contract_addr: contract_addr.to_string(),
                            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                                owner: info.sender.to_string(),
                                recipient,
                                amount: self.amount,
                            })?,
                            funds: vec![],
                        }
                        .into(),
                    );
                }
                _ => self.assert_sent_native_token_balance(info)?,
            }
        }

        Ok(())
    }
}

pub fn create_tick(
    store: &mut dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    index: i32,
) -> Result<Tick, ContractError> {
    check_tick(index, pool_key.fee_tier.tick_spacing)?;
    let pool = state::get_pool(store, pool_key)?;

    let tick = Tick::create(index, &pool, current_timestamp);
    state::add_tick(store, pool_key, index, &tick)?;
    state::flip_bitmap(store, true, index, pool_key.fee_tier.tick_spacing, pool_key)?;

    Ok(tick)
}

pub fn calculate_swap(
    store: &dyn Storage,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    if amount.is_zero() {
        return Err(ContractError::AmountIsZero {});
    }

    let mut ticks: Vec<Tick> = vec![];
    let mut pool = state::get_pool(store, pool_key)?;

    if x_to_y {
        if pool.sqrt_price <= sqrt_price_limit || sqrt_price_limit > SqrtPrice::new(MAX_SQRT_PRICE)
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
        let (swap_limit, limiting_tick) = state::get_closer_limit(
            store,
            sqrt_price_limit,
            x_to_y,
            pool.current_tick_index,
            pool_key.fee_tier.tick_spacing,
            pool_key,
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
                .map_err(|_| ContractError::Sub)?;
        } else {
            remaining_amount = remaining_amount
                .checked_sub(result.amount_out)
                .map_err(|_| ContractError::Sub)?;
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
                    let tick = state::get_tick(store, pool_key, tick_index)?;
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

#[allow(clippy::too_many_arguments)]
pub fn swap_internal(
    store: &mut dyn Storage,
    api: &dyn Api,
    info: &MessageInfo,
    msgs: &mut Vec<CosmosMsg>,
    contract_address: &Addr,
    current_timestamp: u64,
    pool_key: &PoolKey,
    x_to_y: bool,
    amount: TokenAmount,
    by_amount_in: bool,
    sqrt_price_limit: SqrtPrice,
) -> Result<CalculateSwapResult, ContractError> {
    let calculate_swap_result = calculate_swap(
        store,
        current_timestamp,
        pool_key,
        x_to_y,
        amount,
        by_amount_in,
        sqrt_price_limit,
    )?;

    for tick in calculate_swap_result.ticks.iter() {
        state::update_tick(store, pool_key, tick.index, tick)?;
    }

    POOLS.save(store, &pool_key.key(), &calculate_swap_result.pool)?;

    let (token_0, token_1) = if x_to_y {
        (&pool_key.token_x, &pool_key.token_y)
    } else {
        (&pool_key.token_y, &pool_key.token_x)
    };

    let asset_0 = Asset {
        info: denom_to_asset_info(api, token_0.as_str()),
        amount: calculate_swap_result.amount_in.into(),
    };

    let asset_1 = Asset {
        info: denom_to_asset_info(api, token_1.as_str()),
        amount: calculate_swap_result.amount_out.into(),
    };

    asset_0.transfer_from(msgs, &info, contract_address.to_string())?;
    asset_1.transfer(msgs, &info)?;

    Ok(calculate_swap_result)
}

pub fn swap_route_internal(
    store: &mut dyn Storage,
    api: &dyn Api,
    env: Env,
    info: &MessageInfo,
    msgs: &mut Vec<CosmosMsg>,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let mut next_swap_amount = amount_in;

    let current_timestamp = env.block.time.millis();

    for swap_hop in &swaps {
        let sqrt_price_limit = if swap_hop.x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        next_swap_amount = swap_internal(
            store,
            api,
            info,
            msgs,
            &env.contract.address,
            current_timestamp,
            &swap_hop.pool_key,
            swap_hop.x_to_y,
            next_swap_amount,
            true,
            sqrt_price_limit,
        )?
        .amount_out;
    }

    Ok(next_swap_amount)
}

pub fn route(
    store: &dyn Storage,
    env: Env,
    amount_in: TokenAmount,
    swaps: Vec<SwapHop>,
) -> Result<TokenAmount, ContractError> {
    let mut next_swap_amount = amount_in;

    let current_timestamp = env.block.time.millis();

    for swap_hop in &swaps {
        let sqrt_price_limit = if swap_hop.x_to_y {
            SqrtPrice::new(MIN_SQRT_PRICE)
        } else {
            SqrtPrice::new(MAX_SQRT_PRICE)
        };

        next_swap_amount = calculate_swap(
            store,
            current_timestamp,
            &swap_hop.pool_key,
            swap_hop.x_to_y,
            next_swap_amount,
            true,
            sqrt_price_limit,
        )?
        .amount_out;
    }

    Ok(next_swap_amount)
}

pub fn tickmap_slice(
    store: &dyn Storage,
    range: impl Iterator<Item = u16>,
    pool_key: &PoolKey,
) -> Vec<(u16, u64)> {
    let mut tickmap_slice: Vec<(u16, u64)> = vec![];

    for chunk_index in range {
        if let Ok(chunk) = state::get_bitmap_item(store, chunk_index, pool_key) {
            tickmap_slice.push((chunk_index, chunk));

            if tickmap_slice.len() == MAX_TICKMAP_QUERY_SIZE {
                return tickmap_slice;
            }
        }
    }

    tickmap_slice
}

pub fn remove_tick_and_flip_bitmap(
    storage: &mut dyn Storage,
    key: &PoolKey,
    tick: &Tick,
) -> Result<(), ContractError> {
    if !tick.liquidity_gross.is_zero() {
        return Err(ContractError::NotEmptyTickDeinitialization);
    }

    state::flip_bitmap(storage, false, tick.index, key.fee_tier.tick_spacing, key)?;

    state::remove_tick(storage, key, tick.index)?;

    Ok(())
}

pub fn denom_to_asset_info(api: &dyn Api, denom: &str) -> AssetInfo {
    if let Ok(contract_addr) = api.addr_validate(denom) {
        AssetInfo::Token { contract_addr }
    } else {
        AssetInfo::NativeToken {
            denom: denom.to_string(),
        }
    }
}
