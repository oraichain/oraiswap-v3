use decimal::*;

use crate::{
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::{macros::*, MockApp},
    FeeTier, PoolKey,
};

#[test]
fn test_position_slippage_zero_slippage_and_inside_range() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::from_scale(1, 2));

    let (token_x, token_y) = create_tokens!(app, 10u128.pow(23));
    let pool_key = init_slippage_pool_with_liquidity!(app, dex, token_x, token_y);

    let pool = get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    // zero slippage
    {
        let liquidity_delta = Liquidity::from_integer(1_000_000);
        let known_price = pool.sqrt_price;
        let tick = pool_key.fee_tier.tick_spacing as i32;
        create_position!(
            app,
            dex,
            pool_key,
            -tick,
            tick,
            liquidity_delta,
            known_price,
            known_price,
            "alice"
        )
        .unwrap();
    }
    // inside range
    {
        let liquidity_delta = Liquidity::from_integer(1_000_000);
        let limit_lower = SqrtPrice::new(994734637981406576896367);
        let limit_upper = SqrtPrice::new(1025038048074314166333500);

        let tick = pool_key.fee_tier.tick_spacing as i32;

        create_position!(
            app,
            dex,
            pool_key,
            -tick,
            tick,
            liquidity_delta,
            limit_lower,
            limit_upper,
            "alice"
        )
        .unwrap();
    }
}

#[test]
fn test_position_slippage_below_range() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::from_scale(1, 2));
    let (token_x, token_y) = create_tokens!(app, 10u128.pow(23));
    let pool_key = init_slippage_pool_with_liquidity!(app, dex, token_x, token_y);

    get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    let liquidity_delta = Liquidity::from_integer(1_000_000);
    let limit_lower = SqrtPrice::new(1014432353584998786339859);
    let limit_upper = SqrtPrice::new(1045335831204498605270797);
    let tick = pool_key.fee_tier.tick_spacing as i32;
    create_position!(
        app,
        dex,
        pool_key,
        -tick,
        tick,
        liquidity_delta,
        limit_lower,
        limit_upper,
        "alice"
    )
    .unwrap_err();
}

#[test]
fn test_position_slippage_above_range() {
    let mut app = MockApp::new(&[]);
    let dex = create_dex!(app, Percentage::from_scale(1, 2));
    let (token_x, token_y) = create_tokens!(app, 10u128.pow(23));
    let pool_key = init_slippage_pool_with_liquidity!(app, dex, token_x, token_y);

    get_pool!(app, dex, token_x, token_y, pool_key.fee_tier).unwrap();

    let liquidity_delta = Liquidity::from_integer(1_000_000);
    let limit_lower = SqrtPrice::new(955339206774222158009382);
    let limit_upper = SqrtPrice::new(984442481813945288458906);
    let tick = pool_key.fee_tier.tick_spacing as i32;
    create_position!(
        app,
        dex,
        pool_key,
        -tick,
        tick,
        liquidity_delta,
        limit_lower,
        limit_upper,
        "alice"
    )
    .unwrap_err();
}
