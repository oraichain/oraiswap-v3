use cosmwasm_std::coin;
use decimal::{Decimal, Factories};

use crate::{
    create_entry_points_testing,
    fee_growth::FeeGrowth,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::SqrtPrice,
    tests::helper::{macros::*, MockApp},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
};

#[test]
fn test_cross() {
    let initial_mint = 10u128.pow(10);
    let mut app = MockApp::new(&[("alice", &[coin(initial_mint, "orai")])]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

    let dex = app
        .create_dex("alice", Percentage::from_scale(1, 2))
        .unwrap();
    let (token_x, token_y) = create_tokens!(app, initial_mint, initial_mint);

    init_basic_pool!(app, dex, token_x, token_y);
    init_basic_position!(app, dex, token_x, token_y);
    init_cross_position!(app, dex, token_x, token_y);
    init_cross_swap!(app, dex, token_x.clone(), token_y.clone());

    let fee_tier = FeeTier::new(Percentage::from_scale(6, 3), 10).unwrap();
    let pool_key = PoolKey::new(token_x, token_y, fee_tier).unwrap();

    let upper_tick_index = 10;
    let middle_tick_index = -10;
    let lower_tick_index = -20;

    let upper_tick = get_tick!(app, dex, pool_key, upper_tick_index).unwrap();
    let middle_tick = get_tick!(app, dex, pool_key, middle_tick_index).unwrap();
    let lower_tick = get_tick!(app, dex, pool_key, lower_tick_index).unwrap();

    assert_eq!(
        upper_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );
    assert_eq!(
        middle_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );
    assert_eq!(
        lower_tick.liquidity_change,
        Liquidity::from_integer(1000000)
    );

    assert_eq!(upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(
        middle_tick.fee_growth_outside_x,
        FeeGrowth::new(30000000000000000000000)
    );
    assert_eq!(lower_tick.fee_growth_outside_x, FeeGrowth::new(0));
}
