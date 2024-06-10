use decimal::*;

use crate::{
    fee_growth::FeeGrowth,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
};

#[test]

fn test_create_position() {
    let (mut app, dex) = create_dex!(Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 10;
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
}

#[test]

fn test_position_same_upper_and_lower_tick() {
    let (mut app, dex) = create_dex!(Percentage::new(0));
    let (token_x, token_y) = create_tokens!(app, 500, 500);

    let fee_tier = FeeTier::new(Percentage::new(0), 1).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

    let init_tick = 10;
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
        10,
        10,
        Liquidity::new(10),
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap_err();
}

#[test]

fn test_remove_position() {
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let remove_position_index = 0;

    let initial_mint = 10u128.pow(10);

    let (mut app, dex) = create_dex!(Percentage::from_scale(1, 2));

    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint);

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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

    let lower_tick_index = -20;
    let upper_tick_index = 10;
    let liquidity_delta = Liquidity::from_integer(1_000_000);

    approve!(app, token_x, dex, initial_mint, "alice").unwrap();
    approve!(app, token_y, dex, initial_mint, "alice").unwrap();

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
        "alice"
    )
    .unwrap();

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    assert_eq!(pool_state.liquidity, liquidity_delta);

    let liquidity_delta = Liquidity::new(liquidity_delta.get() * 1_000_000);
    {
        let incorrect_lower_tick_index = lower_tick_index - 50;
        let incorrect_upper_tick_index = upper_tick_index + 50;

        approve!(app, token_x, dex, liquidity_delta.0, "alice").unwrap();
        approve!(app, token_y, dex, liquidity_delta.0, "alice").unwrap();

        create_position!(
            app,
            dex,
            pool_key,
            incorrect_lower_tick_index,
            incorrect_upper_tick_index,
            liquidity_delta,
            pool_state.sqrt_price,
            pool_state.sqrt_price,
            "alice"
        )
        .unwrap();

        let position_state = get_position!(app, dex, 1, "alice").unwrap();
        // Check position
        assert!(position_state.lower_tick_index == incorrect_lower_tick_index);
        assert!(position_state.upper_tick_index == incorrect_upper_tick_index);
    }

    let amount = 1000;
    mint!(app, token_x, "bob", amount, "alice").unwrap();
    let amount_x = balance_of!(app, token_x, "bob");
    assert_eq!(amount_x, amount);

    approve!(app, token_x, dex, amount, "bob").unwrap();

    let pool_state_before = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    let swap_amount = TokenAmount::new(amount);
    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    swap!(app, dex, pool_key, true, swap_amount, true, slippage, "bob").unwrap();

    let pool_state_after = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    assert_eq!(
        pool_state_after.fee_growth_global_x,
        FeeGrowth::new(49999950000049999)
    );
    assert_eq!(pool_state_after.fee_protocol_token_x, TokenAmount(1));
    assert_eq!(pool_state_after.fee_protocol_token_y, TokenAmount(0));

    assert!(pool_state_after
        .sqrt_price
        .lt(&pool_state_before.sqrt_price));

    assert_eq!(pool_state_after.liquidity, pool_state_before.liquidity);
    assert_eq!(pool_state_after.current_tick_index, -10);
    assert_ne!(pool_state_after.sqrt_price, pool_state_before.sqrt_price);

    let amount_x = balance_of!(app, token_x, "bob");
    let amount_y = balance_of!(app, token_y, "bob");
    assert_eq!(amount_x, 0);
    assert_eq!(amount_y, 993);

    // pre load dex balances
    let dex_x_before_remove = balance_of!(app, token_x, dex);
    let dex_y_before_remove = balance_of!(app, token_y, dex);

    // Remove position
    remove_position!(app, dex, remove_position_index, "alice").unwrap();

    // Load states
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    // Check ticks
    get_tick!(app, dex, pool_key, lower_tick_index).unwrap_err();
    get_tick!(app, dex, pool_key, upper_tick_index).unwrap_err();
    let lower_tick_bit = is_tick_initialized!(app, dex, pool_key, lower_tick_index);

    let upper_tick_bit = is_tick_initialized!(app, dex, pool_key, upper_tick_index);
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);
    let expected_withdrawn_x = 499;
    let expected_withdrawn_y = 999;
    let expected_fee_x = 0;

    assert_eq!(
        dex_x_before_remove - dex_x,
        expected_withdrawn_x + expected_fee_x
    );
    assert_eq!(dex_y_before_remove - dex_y, expected_withdrawn_y);

    // Check tickmap
    assert!(!lower_tick_bit);
    assert!(!upper_tick_bit);

    // Check pool
    assert!(pool_state.liquidity == liquidity_delta);
    assert!(pool_state.current_tick_index == -10);
}

#[test]

fn test_position_within_current_tick() {
    let max_tick_test = 177_450; // for tickSpacing 4
    let min_tick_test = -max_tick_test;

    let init_tick = -23028;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let (mut app, dex) = create_dex!(Percentage::new(0));
    let initial_balance = 100_000_000;

    let (token_x, token_y) = create_tokens!(app, initial_balance, initial_balance);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 4).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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

    approve!(app, token_x, dex, initial_balance, "alice").unwrap();
    approve!(app, token_y, dex, initial_balance, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    let lower_tick_index = min_tick_test + 10;
    let upper_tick_index = max_tick_test - 10;

    let liquidity_delta = Liquidity::from_integer(100);

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // Load states
    let position_state = get_position!(app, dex, 0, "alice").unwrap();
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let alice_x = balance_of!(app, token_x, "alice");
    let alice_y = balance_of!(app, token_y, "alice");
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);

    let zero_fee = FeeGrowth::new(0);
    let expected_x_increase = 317;
    let expected_y_increase = 32;

    // Check ticks
    assert!(lower_tick.index == lower_tick_index);
    assert!(upper_tick.index == upper_tick_index);
    assert_eq!(lower_tick.liquidity_gross, liquidity_delta);
    assert_eq!(upper_tick.liquidity_gross, liquidity_delta);
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert!(lower_tick.sign);
    assert!(!upper_tick.sign);

    // Check pool
    assert!(pool_state.liquidity == liquidity_delta);
    assert!(pool_state.current_tick_index == init_tick);

    // Check position
    assert!(position_state.pool_key == pool_key);
    assert!(position_state.liquidity == liquidity_delta);
    assert!(position_state.lower_tick_index == lower_tick_index);
    assert!(position_state.upper_tick_index == upper_tick_index);
    assert!(position_state.fee_growth_inside_x == zero_fee);
    assert!(position_state.fee_growth_inside_y == zero_fee);

    // Check balances
    assert_eq!(alice_x, initial_balance.checked_sub(dex_x).unwrap());
    assert_eq!(alice_y, initial_balance.checked_sub(dex_y).unwrap());
    assert_eq!(dex_x, expected_x_increase);
    assert_eq!(dex_y, expected_y_increase);
}

#[test]

fn test_position_below_current_tick() {
    let init_tick = -23028;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let (mut app, dex) = create_dex!(Percentage::new(0));
    let initial_balance = 10_000_000_000;

    let (token_x, token_y) = create_tokens!(app, initial_balance, initial_balance);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 4).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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

    approve!(app, token_x, dex, initial_balance, "alice").unwrap();
    approve!(app, token_y, dex, initial_balance, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    let lower_tick_index = -46080;
    let upper_tick_index = -23040;

    let liquidity_delta = Liquidity::from_integer(10_000);

    let pool_state_before = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state_before.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // Load states
    let position_state = get_position!(app, dex, 0, "alice").unwrap();
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let alice_x = balance_of!(app, token_x, "alice");
    let alice_y = balance_of!(app, token_y, "alice");
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);

    let zero_fee = FeeGrowth::new(0);
    let expected_x_increase = 0;
    let expected_y_increase = 2162;

    // Check ticks
    assert!(lower_tick.index == lower_tick_index);
    assert!(upper_tick.index == upper_tick_index);
    assert_eq!(lower_tick.liquidity_gross, liquidity_delta);
    assert_eq!(upper_tick.liquidity_gross, liquidity_delta);
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert!(lower_tick.sign);
    assert!(!upper_tick.sign);

    // Check pool
    assert!(pool_state.liquidity == pool_state_before.liquidity);
    assert!(pool_state.current_tick_index == init_tick);

    // Check position
    assert!(position_state.pool_key == pool_key);
    assert!(position_state.liquidity == liquidity_delta);
    assert!(position_state.lower_tick_index == lower_tick_index);
    assert!(position_state.upper_tick_index == upper_tick_index);
    assert!(position_state.fee_growth_inside_x == zero_fee);
    assert!(position_state.fee_growth_inside_y == zero_fee);

    // Check balances
    assert_eq!(alice_x, initial_balance.checked_sub(dex_x).unwrap());
    assert_eq!(alice_y, initial_balance.checked_sub(dex_y).unwrap());

    assert_eq!(dex_x, expected_x_increase);
    assert_eq!(dex_y, expected_y_increase);
}

#[test]

fn test_position_above_current_tick() {
    let init_tick = -23028;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let (mut app, dex) = create_dex!(Percentage::new(0));
    let initial_balance = 10_000_000_000;

    let (token_x, token_y) = create_tokens!(app, initial_balance, initial_balance);

    let fee_tier = FeeTier::new(Percentage::from_scale(2, 4), 4).unwrap();

    add_fee_tier!(app, dex, fee_tier, "alice").unwrap();

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

    approve!(app, token_x, dex, initial_balance, "alice").unwrap();
    approve!(app, token_y, dex, initial_balance, "alice").unwrap();

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();
    let lower_tick_index = -22980;
    let upper_tick_index = 0;
    let liquidity_delta = Liquidity::from_integer(10_000);

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        pool_state.sqrt_price,
        SqrtPrice::max_instance(),
        "alice"
    )
    .unwrap();

    // Load states
    let position_state = get_position!(app, dex, 0, "alice").unwrap();
    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();
    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let alice_x = balance_of!(app, token_x, "alice");
    let alice_y = balance_of!(app, token_y, "alice");
    let dex_x = balance_of!(app, token_x, dex);
    let dex_y = balance_of!(app, token_y, dex);

    let zero_fee = FeeGrowth::new(0);
    let expected_x_increase = 21549;
    let expected_y_increase = 0;

    // Check ticks
    assert!(lower_tick.index == lower_tick_index);
    assert!(upper_tick.index == upper_tick_index);
    assert_eq!(lower_tick.liquidity_gross, liquidity_delta);
    assert_eq!(upper_tick.liquidity_gross, liquidity_delta);
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert!(lower_tick.sign);
    assert!(!upper_tick.sign);

    // Check pool
    assert!(pool_state.liquidity == Liquidity::new(0));
    assert!(pool_state.current_tick_index == init_tick);

    // Check position
    assert!(position_state.pool_key == pool_key);
    assert!(position_state.liquidity == liquidity_delta);
    assert!(position_state.lower_tick_index == lower_tick_index);
    assert!(position_state.upper_tick_index == upper_tick_index);
    assert!(position_state.fee_growth_inside_x == zero_fee);
    assert!(position_state.fee_growth_inside_y == zero_fee);

    // Check balances
    assert_eq!(alice_x, initial_balance.checked_sub(dex_x).unwrap());
    assert_eq!(alice_y, initial_balance.checked_sub(dex_y).unwrap());

    assert_eq!(dex_x, expected_x_increase);
    assert_eq!(dex_y, expected_y_increase);
}
