use cosmwasm_std::{Addr, StdResult, Storage};
use cw_storage_plus::{Item, Map};

use crate::{
    flip_bit_at_position, get_bit_at_position, get_search_limit,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tick_to_position, Config, ContractError, Pool, PoolKey, Position, Tick, CHUNK_SIZE, MAX_TICK,
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const POOLS: Map<&[u8], Pool> = Map::new("pools");
pub const POOL_KEYS: Map<&[u8], u16> = Map::new("pool_keys");
pub const POOL_KEYS_BY_INDEX: Map<u16, PoolKey> = Map::new("pool_keys_by_index");
pub const POOL_KEYS_LENGTH: Item<u16> = Item::new("pool_keys_length");

pub const POSITIONS_LENGTH: Map<&[u8], u32> = Map::new("positions_length");
pub const POSITIONS: Map<&[u8], Position> = Map::new("positions");

pub const TICKS: Map<&[u8], Tick> = Map::new("ticks");

pub const BITMAP: Map<&[u8], u64> = Map::new("bitmap");

pub const MAX_LIMIT: u32 = 100;

pub fn get_pool(store: &dyn Storage, pool_key: &PoolKey) -> Result<Pool, ContractError> {
    let pool = POOLS.load(store, &pool_key.key())?;
    Ok(pool)
}

pub fn get_pool_keys_length(store: &dyn Storage) -> u16 {
    POOL_KEYS_LENGTH.load(store).unwrap_or(0)
}

pub fn get_all_pool_keys(
    store: &dyn Storage,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<PoolKey>, ContractError> {
    let from_idx = offset.unwrap_or(0) as u16;
    let to_idx = get_pool_keys_length(store).min(from_idx + limit.unwrap_or(MAX_LIMIT) as u16);

    let pool_keys = (from_idx..to_idx)
        .map(|index| POOL_KEYS_BY_INDEX.load(store, index))
        .collect::<StdResult<Vec<PoolKey>>>()?;

    Ok(pool_keys)
}

pub fn tick_key(pool_key: &PoolKey, index: i32) -> Vec<u8> {
    let mut db_key = pool_key.key();
    db_key.extend_from_slice(&index.to_be_bytes());
    db_key
}

pub fn add_tick(
    store: &mut dyn Storage,
    pool_key: &PoolKey,
    index: i32,
    tick: &Tick,
) -> Result<(), ContractError> {
    let db_key = tick_key(pool_key, index);

    if TICKS.has(store, &db_key) {
        return Err(ContractError::TickAlreadyExist);
    }

    TICKS.save(store, &db_key, tick)?;

    Ok(())
}

pub fn update_tick(
    store: &mut dyn Storage,
    pool_key: &PoolKey,
    index: i32,
    tick: &Tick,
) -> Result<(), ContractError> {
    let db_key = tick_key(pool_key, index);

    if !TICKS.has(store, &db_key) {
        return Err(ContractError::TickNotFound);
    }

    TICKS.save(store, &db_key, tick)?;

    Ok(())
}

pub fn remove_tick(
    store: &mut dyn Storage,
    pool_key: &PoolKey,
    index: i32,
) -> Result<(), ContractError> {
    let db_key = tick_key(pool_key, index);

    if !TICKS.has(store, &db_key) {
        return Err(ContractError::TickNotFound);
    }

    TICKS.remove(store, &db_key);
    Ok(())
}

pub fn get_tick(
    store: &dyn Storage,
    pool_key: &PoolKey,
    index: i32,
) -> Result<Tick, ContractError> {
    let db_key = tick_key(pool_key, index);

    let tick = TICKS
        .load(store, &db_key)
        .map_err(|_| ContractError::TickNotFound)?;

    Ok(tick)
}

pub fn position_key(account_id: &Addr, positions_length: u32) -> Vec<u8> {
    let mut db_key = account_id.as_bytes().to_vec();
    db_key.extend_from_slice(&positions_length.to_be_bytes());
    db_key
}

pub fn add_position(
    store: &mut dyn Storage,
    account_id: &Addr,
    position: &Position,
) -> Result<(), ContractError> {
    let positions_length: u32 = get_position_length(store, account_id);
    let db_key = position_key(account_id, positions_length);
    POSITIONS.save(store, &db_key, position)?;

    POSITIONS_LENGTH.save(store, account_id.as_bytes(), &(positions_length + 1))?;
    Ok(())
}

pub fn update_position(
    store: &mut dyn Storage,
    account_id: &Addr,
    index: u32,
    position: &Position,
) -> Result<(), ContractError> {
    let positions_length = get_position_length(store, account_id);

    if index >= positions_length {
        return Err(ContractError::PositionNotFound);
    }

    let db_key = position_key(account_id, positions_length);

    POSITIONS.save(store, &db_key, position)?;

    Ok(())
}

pub fn remove_position(
    store: &mut dyn Storage,
    account_id: &Addr,
    index: u32,
) -> Result<Position, ContractError> {
    let positions_length = get_position_length(store, account_id);
    let db_key = position_key(account_id, index);
    let position = POSITIONS
        .load(store, &db_key)
        .map_err(|_| ContractError::PositionNotFound)?;

    if index < positions_length - 1 {
        let prev_db_key = position_key(account_id, positions_length - 1);
        let last_position = POSITIONS.load(store, &prev_db_key)?;
        POSITIONS.remove(store, &prev_db_key);
        POSITIONS.save(store, &db_key, &last_position)?;
    } else {
        POSITIONS.remove(store, &db_key);
    }

    POSITIONS_LENGTH.save(store, account_id.as_bytes(), &(positions_length - 1))?;

    Ok(position)
}

pub fn get_position(
    store: &dyn Storage,
    account_id: &Addr,
    index: u32,
) -> Result<Position, ContractError> {
    let db_key = position_key(account_id, index);
    let position = POSITIONS
        .load(store, &db_key)
        .map_err(|_| ContractError::PositionNotFound)?;

    Ok(position)
}

pub fn get_all_positions(
    store: &dyn Storage,
    account_id: &Addr,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Position>, ContractError> {
    let from_idx = offset.unwrap_or(0);
    // maximum 100 items
    let to_idx = get_position_length(store, account_id).min(from_idx + limit.unwrap_or(MAX_LIMIT));
    (from_idx..to_idx)
        .map(|index| get_position(store, account_id, index))
        .collect()
}

pub fn get_position_length(store: &dyn Storage, account_id: &Addr) -> u32 {
    POSITIONS_LENGTH
        .load(store, account_id.as_bytes())
        .unwrap_or(0)
}

pub fn bitmap_key(chunk: u16, pool_key: &PoolKey) -> Vec<u8> {
    let mut db_key = chunk.to_be_bytes().to_vec();
    db_key.append(&mut pool_key.key());
    db_key
}

pub fn next_initialized(
    store: &dyn Storage,
    tick: i32,
    tick_spacing: u16,
    pool_key: &PoolKey,
) -> Option<i32> {
    let limit = get_search_limit(tick, tick_spacing, true);

    if tick + tick_spacing as i32 > MAX_TICK {
        return None;
    }

    // add 1 to not check current tick
    let (mut chunk, mut bit) =
        tick_to_position(tick.checked_add(tick_spacing as i32)?, tick_spacing);
    let (limiting_chunk, limiting_bit) = tick_to_position(limit, tick_spacing);

    while chunk < limiting_chunk || (chunk == limiting_chunk && bit <= limiting_bit) {
        let db_key = bitmap_key(chunk, pool_key);
        let mut shifted = BITMAP.load(store, &db_key).unwrap_or(0) >> bit;

        if shifted != 0 {
            while shifted.checked_rem(2)? == 0 {
                shifted >>= 1;
                bit = bit.checked_add(1)?;
            }

            return if chunk < limiting_chunk || (chunk == limiting_chunk && bit <= limiting_bit) {
                // no possibility of overflow
                let index: i32 = (chunk as i32 * CHUNK_SIZE) + bit as i32;

                Some(
                    index
                        .checked_sub(MAX_TICK / tick_spacing as i32)?
                        .checked_mul(tick_spacing.into())?,
                )
            } else {
                None
            };
        }

        // go to the text chunk
        // if let value = chunk.checked_add(1)? {
        if let Some(value) = chunk.checked_add(1) {
            chunk = value;
        } else {
            return None;
        }
        bit = 0;
    }

    None
}

// tick_spacing - spacing already scaled by tick_spacing
pub fn prev_initialized(
    store: &dyn Storage,
    tick: i32,
    tick_spacing: u16,
    pool_key: &PoolKey,
) -> Option<i32> {
    // don't subtract 1 to check the current tick
    let limit = get_search_limit(tick, tick_spacing, false); // limit scaled by tick_spacing
    let (mut chunk, mut bit) = tick_to_position(tick, tick_spacing);
    let (limiting_chunk, limiting_bit) = tick_to_position(limit, tick_spacing);

    while chunk > limiting_chunk || (chunk == limiting_chunk && bit >= limiting_bit) {
        // always safe due to limitated domain of bit variable
        let mut mask = 1u128 << bit; // left = MSB direction (increase value)
        let db_key = bitmap_key(chunk, pool_key);
        let value = BITMAP.load(store, &db_key).unwrap_or(0) as u128;

        // enter if some of previous bits are initialized in current chunk
        if value.checked_rem(mask.checked_shl(1)?)? > 0 {
            // skip uninitalized ticks
            while value & mask == 0 {
                mask >>= 1;
                bit = bit.checked_sub(1)?;
            }

            // return first initalized tick if limiit is not exceeded, otherswise return None
            return if chunk > limiting_chunk || (chunk == limiting_chunk && bit >= limiting_bit) {
                // no possibility to overflow
                let index: i32 = (chunk as i32 * CHUNK_SIZE) + bit as i32;

                Some(
                    index
                        .checked_sub(MAX_TICK / tick_spacing as i32)?
                        .checked_mul(tick_spacing.into())?,
                )
            } else {
                None
            };
        }

        // go to the next chunk
        // if let value = chunk.checked_sub(1)? {
        if let Some(value) = chunk.checked_sub(1) {
            chunk = value;
        } else {
            return None;
        }
        bit = CHUNK_SIZE as u8 - 1;
    }

    None
}

// Finds closes initialized tick in direction of trade
// and compares its sqrt_price to the sqrt_price limit of the trade
pub fn get_closer_limit(
    store: &dyn Storage,
    sqrt_price_limit: SqrtPrice,
    x_to_y: bool,
    current_tick: i32,
    tick_spacing: u16,
    pool_key: &PoolKey,
) -> Result<(SqrtPrice, Option<(i32, bool)>), ContractError> {
    let closes_tick_index = if x_to_y {
        prev_initialized(store, current_tick, tick_spacing, pool_key)
    } else {
        next_initialized(store, current_tick, tick_spacing, pool_key)
    };

    match closes_tick_index {
        Some(index) => {
            let sqrt_price = calculate_sqrt_price(index)?;

            if (x_to_y && sqrt_price > sqrt_price_limit)
                || (!x_to_y && sqrt_price < sqrt_price_limit)
            {
                Ok((sqrt_price, Some((index, true))))
            } else {
                Ok((sqrt_price_limit, None))
            }
        }
        None => {
            let index = get_search_limit(current_tick, tick_spacing, !x_to_y);
            let sqrt_price = calculate_sqrt_price(index)?;

            if current_tick == index {
                return Err(ContractError::TickLimitReached {});
            }

            if (x_to_y && sqrt_price > sqrt_price_limit)
                || (!x_to_y && sqrt_price < sqrt_price_limit)
            {
                Ok((sqrt_price, Some((index, false))))
            } else {
                Ok((sqrt_price_limit, None))
            }
        }
    }
}

pub fn get_bitmap(store: &dyn Storage, tick: i32, tick_spacing: u16, pool_key: &PoolKey) -> bool {
    let (chunk, bit) = tick_to_position(tick, tick_spacing);
    let db_key = bitmap_key(chunk, pool_key);
    let returned_chunk = BITMAP.load(store, &db_key).unwrap_or(0);
    get_bit_at_position(returned_chunk, bit) == 1
}

pub fn flip_bitmap(
    store: &mut dyn Storage,
    value: bool,
    tick: i32,
    tick_spacing: u16,
    pool_key: &PoolKey,
) -> Result<(), ContractError> {
    let (chunk, bit) = tick_to_position(tick, tick_spacing);
    let db_key = bitmap_key(chunk, pool_key);
    let returned_chunk = BITMAP.load(store, &db_key).unwrap_or(0);
    let check_bit = get_bit_at_position(returned_chunk, bit) == 0;
    if check_bit != value {
        return Err(ContractError::TickReInitialize {});
    }

    BITMAP.save(store, &db_key, &flip_bit_at_position(returned_chunk, bit))?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::math::percentage::Percentage;
    use crate::sqrt_price::SqrtPrice;
    use crate::{FeeTier, MAX_TICK, TICK_SEARCH_RANGE};

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Addr;
    use decimal::*;

    #[test]
    fn test_get_closer_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 1, &pool_key).unwrap();

        // tick limit closer
        {
            let (result, from_tick) = get_closer_limit(
                deps.as_ref().storage,
                SqrtPrice::from_integer(5),
                true,
                100,
                1,
                pool_key,
            )
            .unwrap();

            let expected = SqrtPrice::from_integer(5);
            assert_eq!(result, expected);
            assert_eq!(from_tick, None);
        }
        // trade limit closer
        {
            let (result, from_tick) = get_closer_limit(
                deps.as_ref().storage,
                SqrtPrice::from_scale(1, 1),
                true,
                100,
                1,
                pool_key,
            )
            .unwrap();
            let expected = SqrtPrice::from_integer(1);
            assert_eq!(result, expected);
            assert_eq!(from_tick, Some((0, true)));
        }
        // other direction
        {
            let (result, from_tick) = get_closer_limit(
                deps.as_ref().storage,
                SqrtPrice::from_integer(2),
                false,
                -5,
                1,
                pool_key,
            )
            .unwrap();
            let expected = SqrtPrice::from_integer(1);
            assert_eq!(result, expected);
            assert_eq!(from_tick, Some((0, true)));
        }
        // other direction
        {
            let (result, from_tick) = get_closer_limit(
                deps.as_ref().storage,
                SqrtPrice::from_scale(1, 1),
                false,
                -100,
                10,
                pool_key,
            )
            .unwrap();
            let expected = SqrtPrice::from_scale(1, 1);
            assert_eq!(result, expected);
            assert_eq!(from_tick, None);
        }
    }

    #[test]
    fn test_flip() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        //zero
        {
            let index = 0;

            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, true, index, 1, pool_key).unwrap();
            assert!(get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, false, index, 1, pool_key).unwrap();
            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
        }
        // small
        {
            let index = 7;

            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, true, index, 1, pool_key).unwrap();
            assert!(get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, false, index, 1, pool_key).unwrap();
            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
        }
        // big
        {
            let index = MAX_TICK - 1;

            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, true, index, 1, pool_key).unwrap();
            assert!(get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, false, index, 1, pool_key).unwrap();
            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
        }
        // negative
        {
            let index = MAX_TICK - 40;

            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, true, index, 1, pool_key).unwrap();
            assert!(get_bitmap(deps.as_ref().storage, index, 1, pool_key));
            flip_bitmap(deps.as_mut().storage, false, index, 1, pool_key).unwrap();
            assert!(!get_bitmap(deps.as_ref().storage, index, 1, pool_key));
        }
        // tick spacing
        {
            let index = 20000;
            let tick_spacing = 1000;

            assert!(!get_bitmap(
                deps.as_ref().storage,
                index,
                tick_spacing,
                pool_key
            ));
            flip_bitmap(deps.as_mut().storage, true, index, tick_spacing, pool_key).unwrap();
            assert!(get_bitmap(
                deps.as_ref().storage,
                index,
                tick_spacing,
                pool_key
            ));
            flip_bitmap(deps.as_mut().storage, false, index, tick_spacing, pool_key).unwrap();
            assert!(!get_bitmap(
                deps.as_ref().storage,
                index,
                tick_spacing,
                pool_key
            ));
        }
    }

    #[test]
    fn test_next_initialized_simple() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 5, 1, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, 0, 1, pool_key),
            Some(5)
        );
    }

    #[test]
    fn test_next_initialized_multiple() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 50, 10, pool_key).unwrap();
        flip_bitmap(deps.as_mut().storage, true, 100, 10, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, 0, 10, pool_key),
            Some(50)
        );
        assert_eq!(
            next_initialized(deps.as_ref().storage, 50, 10, pool_key),
            Some(100)
        );
    }

    #[test]
    fn test_next_initialized_current_is_last() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 10, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, 0, 10, pool_key),
            None
        );
    }

    #[test]
    fn test_next_initialized_just_below_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 1, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, -TICK_SEARCH_RANGE, 1, pool_key),
            Some(0)
        );
    }

    #[test]
    fn test_next_initialized_at_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 1, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, -TICK_SEARCH_RANGE - 1, 1, pool_key),
            None
        );
    }

    #[test]
    fn test_next_initialized_further_than_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, MAX_TICK - 10, 1, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, -MAX_TICK + 1, 1, pool_key),
            None
        );
    }

    #[test]
    fn test_next_initialized_hitting_the_limit() {
        let deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        assert_eq!(
            next_initialized(deps.as_ref().storage, MAX_TICK - 22, 4, pool_key),
            None
        );
    }

    #[test]
    fn test_next_initialized_already_at_limit() {
        let deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        assert_eq!(
            next_initialized(deps.as_ref().storage, MAX_TICK - 2, 4, pool_key),
            None
        );
    }

    #[test]
    fn test_next_initialized_at_pos_63() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, MAX_TICK - 63, 1, pool_key).unwrap();
        assert_eq!(
            next_initialized(deps.as_ref().storage, MAX_TICK - 128, 1, pool_key),
            Some(MAX_TICK - 63)
        );
    }

    #[test]
    fn test_prev_initialized_simple() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, -5, 1, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, 0, 1, pool_key),
            Some(-5)
        );
    }

    #[test]
    fn test_prev_initialized_multiple() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, -50, 10, pool_key).unwrap();
        flip_bitmap(deps.as_mut().storage, true, -100, 10, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, 0, 10, pool_key),
            Some(-50)
        );
        assert_eq!(
            prev_initialized(deps.as_ref().storage, -50, 10, pool_key),
            Some(-50)
        );
    }

    #[test]
    fn test_prev_initialized_current_is_last() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 10, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, 0, 10, pool_key),
            Some(0)
        );
    }

    #[test]
    fn test_prev_initialized_next_is_last() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 10, 10, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, 0, 10, pool_key),
            None
        );
    }

    #[test]
    fn test_prev_initialized_just_below_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 1, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, TICK_SEARCH_RANGE, 1, pool_key),
            Some(0)
        );
    }

    #[test]
    fn test_prev_initialized_at_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, 0, 1, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, TICK_SEARCH_RANGE + 1, 1, pool_key),
            None
        );
    }

    #[test]
    fn test_prev_initialized_farther_than_limit() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, -MAX_TICK + 1, 1, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, MAX_TICK - 1, 1, pool_key),
            None
        );
    }

    #[test]
    fn test_prev_initialized_at_pos_63() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        flip_bitmap(deps.as_mut().storage, true, -MAX_TICK + 63, 1, pool_key).unwrap();
        assert_eq!(
            prev_initialized(deps.as_ref().storage, -MAX_TICK + 128, 1, pool_key),
            Some(-MAX_TICK + 63)
        );
    }

    #[test]
    fn test_get_search_limit() {
        // Simple up
        {
            let result = get_search_limit(0, 1, true);
            assert_eq!(result, TICK_SEARCH_RANGE);
        }
        // Simple down
        {
            let result = get_search_limit(0, 1, false);
            assert_eq!(result, -TICK_SEARCH_RANGE);
        }
        // Less simple up
        {
            let start = 60;
            let step = 12;
            let result = get_search_limit(start, step, true);
            let expected = start + TICK_SEARCH_RANGE * step as i32;
            assert_eq!(result, expected);
        }
        // Less simple down
        {
            let start = 60;
            let step = 12;
            let result = get_search_limit(start, step, false);
            let expected = start - TICK_SEARCH_RANGE * step as i32;
            assert_eq!(result, expected);
        }
        // Up to price limit
        {
            let step = 5u16;
            let result = get_search_limit(MAX_TICK - 22, step, true);
            let expected = MAX_TICK - 3;
            assert_eq!(result, expected);
        }
        // At the price limit
        {
            let step = 5u16;
            let result = get_search_limit(MAX_TICK - 3, step, true);
            let expected = MAX_TICK - 3;
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_next_and_prev_initialized() {
        let mut deps = mock_dependencies();
        let token_0: Addr = Addr::unchecked("token_0");
        let token_1: Addr = Addr::unchecked("token_1");
        let fee_tier: FeeTier = FeeTier {
            fee: Percentage::new(1),
            tick_spacing: 1,
        };
        let pool_key = &PoolKey::new(token_0, token_1, fee_tier).unwrap();

        // initalized edges
        {
            for spacing in 1..=10 {
                let max_index = MAX_TICK - MAX_TICK % spacing;
                let min_index = -max_index;

                flip_bitmap(
                    deps.as_mut().storage,
                    true,
                    max_index,
                    spacing as u16,
                    pool_key,
                )
                .unwrap();

                flip_bitmap(
                    deps.as_mut().storage,
                    true,
                    min_index,
                    spacing as u16,
                    pool_key,
                )
                .unwrap();

                let tick_edge_diff = TICK_SEARCH_RANGE / spacing * spacing;

                let prev = prev_initialized(
                    deps.as_ref().storage,
                    min_index + tick_edge_diff,
                    spacing as u16,
                    pool_key,
                );
                let next = next_initialized(
                    deps.as_ref().storage,
                    max_index - tick_edge_diff,
                    spacing as u16,
                    pool_key,
                );

                assert_eq!((prev.is_some(), next.is_some()), (true, true));
                // cleanup
                {
                    flip_bitmap(
                        deps.as_mut().storage,
                        false,
                        max_index,
                        spacing as u16,
                        pool_key,
                    )
                    .unwrap();

                    flip_bitmap(
                        deps.as_mut().storage,
                        false,
                        min_index,
                        spacing as u16,
                        pool_key,
                    )
                    .unwrap();
                }
            }
        }
        // unintalized edges
        for spacing in 1..=1000 {
            // let mut contract = Contract::new();

            let max_index = MAX_TICK - MAX_TICK % spacing;
            let min_index = -max_index;
            let tick_edge_diff = TICK_SEARCH_RANGE / spacing * spacing;

            let prev = prev_initialized(
                deps.as_ref().storage,
                min_index + tick_edge_diff,
                spacing as u16,
                pool_key,
            );
            let next = next_initialized(
                deps.as_ref().storage,
                max_index - tick_edge_diff,
                spacing as u16,
                pool_key,
            );

            assert_eq!((prev.is_some(), next.is_some()), (false, false));
        }
    }
}
