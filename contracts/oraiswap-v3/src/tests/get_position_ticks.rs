use cosmwasm_std::coin;
use cosmwasm_std::Addr;
use decimal::{Decimal, Factories};

use crate::POSITION_TICK_LIMIT;
use crate::{
    create_entry_points_testing,
    liquidity::Liquidity,
    msg,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    FeeTier, PoolKey, PositionTick,
};

#[test]
fn test_get_position_ticks() {
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

    let result: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked("alice"), 0).unwrap();
    assert_eq!(result.len(), 2);

    let lower_tick = get_tick!(app, dex, pool_key, -10).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, 10).unwrap();

    position_tick_equals!(result[0], lower_tick);
    position_tick_equals!(result[1], upper_tick);
}

#[test]
fn test_get_position_ticks_limit() {
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
    for i in 1..=POSITION_TICK_LIMIT / 2 {
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

    let result: Vec<PositionTick> = get_position_ticks!(app, dex, Addr::unchecked("alice"), 0).unwrap();
    assert_eq!(result.len(), POSITION_TICK_LIMIT);

    for i in 1..=POSITION_TICK_LIMIT / 2 {
        let lower_tick = get_tick!(app, dex, pool_key, -(i as i32)).unwrap();
        let upper_tick = get_tick!(app, dex, pool_key, i as i32).unwrap();

        position_tick_equals!(result[i * 2 - 2], lower_tick);
        position_tick_equals!(result[i * 2 - 1], upper_tick);
    }
}

#[test]
fn test_get_position_ticks_with_offset() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

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

    let result_1: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked("alice"), 0).unwrap();
    assert_eq!(result_1.len(), 4);

    let result_2: Vec<PositionTick> =
        get_position_ticks!(app, dex, Addr::unchecked("alice"), 1).unwrap();
    assert_eq!(result_2.len(), 2);

    assert_eq!(result_1[2], result_2[0]);
    assert_eq!(result_1[3], result_2[1]);
}
