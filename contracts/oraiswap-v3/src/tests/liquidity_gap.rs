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
fn test_liquidity_gap() {
    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();
    let initial_mint = 10u128.pow(10);

    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::from_scale(1, 2));

    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint);

    let pool_key = PoolKey::new(token_x.to_string(), token_y.to_string(), fee_tier).unwrap();

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

    let lower_tick_index = -10;
    let upper_tick_index = 10;

    let mint_amount = 10u128.pow(10);
    mint!(app, token_x, "alice", mint_amount, "alice").unwrap();
    mint!(app, token_y, "alice", mint_amount, "alice").unwrap();

    approve!(app, token_x, dex, mint_amount, "alice").unwrap();
    approve!(app, token_y, dex, mint_amount, "alice").unwrap();

    let liquidity_delta = Liquidity::from_integer(20_006_000);

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

    let mint_amount = 10067;
    mint!(app, token_x, "bob", mint_amount, "alice").unwrap();

    approve!(app, token_x, dex, mint_amount, "bob").unwrap();

    let dex_x_before = balance_of!(app, token_x, dex);
    let dex_y_before = balance_of!(app, token_y, dex);

    let swap_amount = TokenAmount::new(10067);
    let target_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);
    let quoted_target_sqrt_price = quote!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price
    )
    .unwrap()
    .target_sqrt_price;

    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        quoted_target_sqrt_price,
        "bob"
    )
    .unwrap();

    let pool = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
    let expected_price = calculate_sqrt_price(-10).unwrap();
    let expected_y_amount_out = 9999;

    assert_eq!(pool.liquidity, liquidity_delta);
    assert_eq!(pool.current_tick_index, lower_tick_index);
    assert_eq!(pool.sqrt_price, expected_price);

    let bob_x = balance_of!(app, token_x, "bob");
    let bob_y = balance_of!(app, token_y, "bob");
    let dex_x_after = balance_of!(app, token_x, dex);
    let dex_y_after = balance_of!(app, token_y, dex);

    let delta_dex_x = dex_x_after - dex_x_before;
    let delta_dex_y = dex_y_before - dex_y_after;

    assert_eq!(bob_x, 0);
    assert_eq!(bob_y, expected_y_amount_out);
    assert_eq!(delta_dex_x, swap_amount.get());
    assert_eq!(delta_dex_y, expected_y_amount_out);
    assert_eq!(
        pool.fee_growth_global_x,
        FeeGrowth::new(29991002699190242927121)
    );
    assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    assert_eq!(pool.fee_protocol_token_x, TokenAmount::new(1));
    assert_eq!(pool.fee_protocol_token_y, TokenAmount::new(0));

    // No gain swap
    {
        let swap_amount = TokenAmount(1);
        let target_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);

        swap!(
            app,
            dex,
            pool_key,
            true,
            swap_amount,
            true,
            target_sqrt_price,
            "bob"
        )
        .unwrap_err();
    }

    // Should skip gap and then swap
    let lower_tick_after_swap = -90;
    let upper_tick_after_swap = -50;
    let liquidity_delta = Liquidity::from_integer(20008000);

    approve!(app, token_x, dex, liquidity_delta.get(), "alice").unwrap();
    approve!(app, token_y, dex, liquidity_delta.get(), "alice").unwrap();

    let pool_state = get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();

    create_position!(
        app,
        dex,
        pool_key,
        lower_tick_after_swap,
        upper_tick_after_swap,
        liquidity_delta,
        pool_state.sqrt_price,
        pool_state.sqrt_price,
        "alice"
    )
    .unwrap();

    let swap_amount = TokenAmount::new(5000);
    mint!(app, token_x, "bob", swap_amount.get(), "alice").unwrap();

    approve!(app, token_x, dex, swap_amount.get(), "bob").unwrap();

    let target_sqrt_price = SqrtPrice::new(MIN_SQRT_PRICE);
    let quoted_target_sqrt_price = quote!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price
    )
    .unwrap()
    .target_sqrt_price;

    swap!(
        app,
        dex,
        pool_key,
        true,
        swap_amount,
        true,
        quoted_target_sqrt_price,
        "bob"
    )
    .unwrap();
    get_pool!(app, dex, token_x, token_y, fee_tier).unwrap();
}
