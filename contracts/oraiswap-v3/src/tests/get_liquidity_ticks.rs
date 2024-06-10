use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use decimal::{Decimal, Factories};

use crate::{
    create_entry_points_testing,
    liquidity::Liquidity,
    msg,
    percentage::Percentage,
    position_to_tick,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    FeeTier, LiquidityTick, PoolKey, LIQUIDITY_TICK_LIMIT,
};

#[test]
fn test_get_liquidity_ticks() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, 500, "alice").unwrap();
    approve!(app, token_y, dex, 500, "alice").unwrap();

    let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let ticks_amount: u32 = get_liquidity_ticks_amount!(app, dex, &pool_key, -10, 10).unwrap();
    assert_eq!(ticks_amount, 2);

    let tickmap: Vec<(u16, u64)> =
        get_tickmap!(app, dex, &pool_key, -10, 10, false, alice).unwrap();
    assert_eq!(tickmap.len(), 2);
    let mut ticks = vec![];
    tickmap.iter().for_each(|(chunk_index, chunk)| {
        for i in 0..64 {
            if chunk & (1 << i) != 0 {
                ticks.push(position_to_tick(
                    *chunk_index,
                    i,
                    pool_key.fee_tier.tick_spacing,
                ));
            }
        }
    });
    assert_eq!(ticks, vec![-10i32, 10]);

    let result: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key, ticks.clone()).unwrap();
    assert_eq!(result.len(), 2);

    let lower_tick = get_tick!(app, dex, pool_key, -10).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, 10).unwrap();

    liquidity_tick_equals!(lower_tick, result[0]);
    liquidity_tick_equals!(upper_tick, result[1]);
}

#[test]
fn test_get_liquidity_ticks_different_tick_spacings() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    let fee_tier_1 = FeeTier::new(Percentage::from_scale(1, 2), 2).unwrap();
    let fee_tier_2 = FeeTier::new(Percentage::from_scale(1, 2), 10).unwrap();

    add_fee_tier!(app, dex, fee_tier_1, "alice").unwrap();
    add_fee_tier!(app, dex, fee_tier_2, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier_1,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier_2,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let pool_key_1 = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier_1).unwrap();
    create_position!(
        app,
        dex,
        pool_key_1,
        -10,
        30,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let pool_key_2 = PoolKey::new(token_x, token_y, fee_tier_2).unwrap();
    create_position!(
        app,
        dex,
        pool_key_2,
        -20,
        40,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let start_index_1 = -10;
    let end_index_1 = 30;

    let start_index_2 = -20;
    let end_index_2 = 40;
    let result: u32 =
        get_liquidity_ticks_amount!(app, dex, &pool_key_1, start_index_1, end_index_1).unwrap();
    assert_eq!(result, 2);
    let result: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key_1, vec![-10, 30]).unwrap();
    assert_eq!(result.len(), 2);

    let result: u32 =
        get_liquidity_ticks_amount!(app, dex, &pool_key_2, start_index_2, end_index_2).unwrap();
    assert_eq!(result, 2);
    let result: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key_2, vec![-20, 40]).unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn test_get_liquidity_ticks_limit() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();

    let mut ticks = vec![];
    for i in 1..=LIQUIDITY_TICK_LIMIT / 2 {
        ticks.push(i as i32);
        ticks.push(-(i as i32));

        create_position!(
            app,
            dex,
            pool_key,
            -(i as i32),
            i as i32,
            Liquidity::new(10),
            SqrtPrice::new(0),
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let result: u32 = get_liquidity_ticks_amount!(
        app,
        dex,
        &pool_key,
        -(LIQUIDITY_TICK_LIMIT as i32),
        LIQUIDITY_TICK_LIMIT as i32
    )
    .unwrap();
    assert_eq!(result, LIQUIDITY_TICK_LIMIT as u32);
    let result: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key, ticks.clone()).unwrap();
    assert_eq!(result.len(), LIQUIDITY_TICK_LIMIT);
}

#[test]
fn test_get_liquidity_ticks_limit_with_spread() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, initial_amount, "alice").unwrap();
    approve!(app, token_y, dex, initial_amount, "alice").unwrap();

    let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();
    let spread = 64;
    let mut ticks = vec![];
    for i in 1..=LIQUIDITY_TICK_LIMIT / 2 {
        let index = (i * spread) as i32;
        ticks.push(index);
        ticks.push(-index);

        create_position!(
            app,
            dex,
            pool_key,
            -index,
            index,
            Liquidity::new(10),
            SqrtPrice::new(0),
            SqrtPrice::max_instance(),
            "alice"
        )
        .unwrap();
    }

    let result: u32 = get_liquidity_ticks_amount!(
        app,
        dex,
        &pool_key,
        -((LIQUIDITY_TICK_LIMIT * spread) as i32) / 2,
        (LIQUIDITY_TICK_LIMIT * spread) as i32 / 2
    )
    .unwrap();
    assert_eq!(result, LIQUIDITY_TICK_LIMIT as u32);
    let result: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key, ticks.clone()).unwrap();
    assert_eq!(result.len(), LIQUIDITY_TICK_LIMIT);
}

#[test]
fn test_get_liquidity_ticks_with_offset() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();

    let initial_amount = 10u128.pow(10);
    let (token_x, token_y) = create_tokens!(app, initial_amount, initial_amount);

    let fee_tier = FeeTier::new(Percentage::from_scale(1, 2), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    create_pool!(
        app,
        dex,
        token_x,
        token_y,
        fee_tier,
        init_sqrt_price,
        init_tick,
        "alice"
    )
    .unwrap();

    approve!(app, token_x, dex, 500, "alice").unwrap();
    approve!(app, token_y, dex, 500, "alice").unwrap();

    let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();
    create_position!(
        app,
        dex,
        pool_key,
        -10,
        10,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    let result: u32 = get_liquidity_ticks_amount!(app, dex, &pool_key, -10, 10).unwrap();
    assert_eq!(result, 2);

    let result_1: Vec<LiquidityTick> =
        get_liquidity_ticks!(app, dex, &pool_key, vec![-10i32, 10]).unwrap();
    assert_eq!(result_1.len(), 2);

    let result_2: Vec<LiquidityTick> = get_liquidity_ticks!(app, dex, &pool_key, vec![10]).unwrap();
    assert_eq!(result_2.len(), 1);

    assert_eq!(result_1[1], result_2[0]);
}
