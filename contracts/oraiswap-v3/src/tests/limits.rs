use decimal::*;

use crate::{
    get_delta_y,
    liquidity::Liquidity,
    logic::{get_liquidity_by_x, get_liquidity_by_y},
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, get_max_tick, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MAX_SQRT_PRICE, MAX_TICK, MIN_SQRT_PRICE,
};

#[test]
fn test_limits_big_deposit_x_and_swap_y() {
    let mut app = MockApp::new(&[]);
    big_deposit_and_swap!(app, true);
}

#[test]
fn test_limits_big_deposit_y_and_swap_x() {
    let mut app = MockApp::new(&[]);
    big_deposit_and_swap!(app, false);
}

#[test]
fn test_limits_big_deposit_both_tokens() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) =
        init_dex_and_tokens!(app, u128::MAX, Percentage::from_scale(1, 2));

    let mint_amount = 2u128.pow(75) - 1;

    approve!(app, token_x, dex, u128::MAX, "alice").unwrap();
    approve!(app, token_y, dex, u128::MAX, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 1).unwrap();

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

    let lower_tick = -(fee_tier.tick_spacing as i32);
    let upper_tick = fee_tier.tick_spacing as i32;
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let liquidity_delta = get_liquidity_by_x(
        TokenAmount(mint_amount),
        lower_tick,
        upper_tick,
        pool.sqrt_price,
        false,
    )
    .unwrap()
    .l;
    let y = get_delta_y(
        calculate_sqrt_price(lower_tick).unwrap(),
        pool.sqrt_price,
        liquidity_delta,
        true,
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let slippage_limit_lower = pool.sqrt_price;
    let slippage_limit_upper = pool.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key,
        lower_tick,
        upper_tick,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();

    let user_amount_x = balance_of!(app, token_x, "alice");
    let user_amount_y = balance_of!(app, token_y, "alice");
    assert_eq!(user_amount_x, u128::MAX - mint_amount);
    assert_eq!(user_amount_y, u128::MAX - y.get());

    let contract_amount_x = balance_of!(app, token_x, dex);
    let contract_amount_y = balance_of!(app, token_y, dex);
    assert_eq!(contract_amount_x, mint_amount);
    assert_eq!(contract_amount_y, y.get());
}

#[test]
fn test_deposit_limits_at_upper_limit() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) =
        init_dex_and_tokens!(app, u128::MAX, Percentage::from_scale(1, 2));

    let mint_amount = 2u128.pow(105) - 1;

    approve!(app, token_x, dex, u128::MAX, "alice").unwrap();
    approve!(app, token_y, dex, u128::MAX, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = get_max_tick(1);
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, init_tick);
    assert_eq!(pool.sqrt_price, calculate_sqrt_price(init_tick).unwrap());

    let position_amount = mint_amount - 1;

    let liquidity_delta = get_liquidity_by_y(
        TokenAmount(position_amount),
        0,
        MAX_TICK,
        pool.sqrt_price,
        false,
    )
    .unwrap()
    .l;

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let slippage_limit_lower = pool.sqrt_price;
    let slippage_limit_upper = pool.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key,
        0,
        MAX_TICK,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();
}

#[test]
fn test_limits_big_deposit_and_swaps() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) =
        init_dex_and_tokens!(app, u128::MAX, Percentage::from_scale(1, 2));

    let mint_amount = 2u128.pow(76) - 1;

    approve!(app, token_x, dex, u128::MAX, "alice").unwrap();
    approve!(app, token_y, dex, u128::MAX, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 1).unwrap();
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

    let pos_amount = mint_amount / 2;
    let lower_tick = -(fee_tier.tick_spacing as i32);
    let upper_tick = fee_tier.tick_spacing as i32;
    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let liquidity_delta = get_liquidity_by_x(
        TokenAmount(pos_amount),
        lower_tick,
        upper_tick,
        pool.sqrt_price,
        false,
    )
    .unwrap()
    .l;

    let y = get_delta_y(
        calculate_sqrt_price(lower_tick).unwrap(),
        pool.sqrt_price,
        liquidity_delta,
        true,
    )
    .unwrap();

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let slippage_limit_lower = pool.sqrt_price;
    let slippage_limit_upper = pool.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key,
        lower_tick,
        upper_tick,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();

    let user_amount_x = balance_of!(app, token_x, "alice");
    let user_amount_y = balance_of!(app, token_y, "alice");
    assert_eq!(user_amount_x, u128::MAX - pos_amount);
    assert_eq!(user_amount_y, u128::MAX - y.get());

    let contract_amount_x = balance_of!(app, token_x, dex);
    let contract_amount_y = balance_of!(app, token_y, dex);
    assert_eq!(contract_amount_x, pos_amount);
    assert_eq!(contract_amount_y, y.get());

    let swap_amount = TokenAmount(mint_amount / 8);

    for i in 1..=4 {
        let (_, sqrt_price_limit) = if i % 2 == 0 {
            (true, SqrtPrice::new(MIN_SQRT_PRICE))
        } else {
            (false, SqrtPrice::new(MAX_SQRT_PRICE))
        };

        swap!(
            app,
            dex,
            pool_key,
            i % 2 == 0,
            swap_amount,
            true,
            sqrt_price_limit,
            "alice"
        )
        .unwrap();
    }
}

#[test]
fn test_limits_full_range_with_max_liquidity() {
    let mut app = MockApp::new(&[]);
    let (dex, token_x, token_y) =
        init_dex_and_tokens!(app, u128::MAX, Percentage::from_scale(1, 2));

    approve!(app, token_x, dex, u128::MAX, "alice").unwrap();
    approve!(app, token_y, dex, u128::MAX, "alice").unwrap();

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 1).unwrap();
    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = get_max_tick(1);
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

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(pool.current_tick_index, init_tick);
    assert_eq!(pool.sqrt_price, calculate_sqrt_price(init_tick).unwrap());

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();
    let liquidity_delta = Liquidity::new(2u128.pow(109) - 1);
    let slippage_limit_lower = pool.sqrt_price;
    let slippage_limit_upper = pool.sqrt_price;
    create_position!(
        app,
        dex,
        pool_key,
        -MAX_TICK,
        MAX_TICK,
        liquidity_delta,
        slippage_limit_lower,
        slippage_limit_upper,
        "alice"
    )
    .unwrap();

    let contract_amount_x = balance_of!(app, token_x, dex);
    let contract_amount_y = balance_of!(app, token_y, dex);

    let expected_x = 0;
    let expected_y = 42534896005851865508212194815854; // < 2^106
    assert_eq!(contract_amount_x, expected_x);
    assert_eq!(contract_amount_y, expected_y);
}
