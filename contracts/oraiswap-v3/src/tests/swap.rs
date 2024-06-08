use cosmwasm_std::coin;
use decimal::{Decimal, Factories};

use crate::{
    fee_growth::FeeGrowth,
    liquidity::Liquidity,
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    token_amount::TokenAmount,
    FeeTier, PoolKey, MIN_SQRT_PRICE,
};

use super::helper::MockApp;

#[test]
fn test_swap_x_to_y() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[
        ("owner", &[coin(initial_amount, "orai")]),
        ("alice", &[coin(initial_amount, "orai")]),
    ]);
    app.set_token_contract(Box::new(crate::create_entry_points_testing!(cw20_base)));

    let fee_tier = FeeTier::new(protocol_fee, 10).unwrap();

    let clmm_addr = app.create_dex("owner", protocol_fee).unwrap();

    let _res = app
        .add_fee_tier("owner", clmm_addr.as_str(), fee_tier)
        .unwrap();

    let init_tick = 0;
    let init_sqrt_price = calculate_sqrt_price(init_tick).unwrap();

    let token_x = app.create_token("alice", "tokenx", initial_amount);
    let token_y = app.create_token("alice", "tokeny", initial_amount);

    app.create_pool(
        "alice",
        clmm_addr.as_str(),
        token_x.as_str(),
        token_y.as_str(),
        fee_tier,
        init_sqrt_price,
        init_tick,
    )
    .unwrap();

    app.approve_token("tokenx", "alice", clmm_addr.as_str(), initial_amount)
        .unwrap();
    app.approve_token("tokeny", "alice", clmm_addr.as_str(), initial_amount)
        .unwrap();

    let pool_key = PoolKey::new(token_x.clone(), token_y.clone(), fee_tier).unwrap();

    let lower_tick_index = -20;
    let middle_tick_index = -10;
    let upper_tick_index = 10;

    let liquidity_delta = Liquidity::from_integer(1000000);

    app.create_position(
        "alice",
        clmm_addr.as_str(),
        &pool_key,
        lower_tick_index,
        upper_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
    )
    .unwrap();

    app.create_position(
        "alice",
        clmm_addr.as_str(),
        &pool_key,
        lower_tick_index - 20,
        middle_tick_index,
        liquidity_delta,
        SqrtPrice::new(0),
        SqrtPrice::max_instance(),
    )
    .unwrap();

    let pool = app
        .get_pool(
            clmm_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();

    assert_eq!(pool.liquidity, liquidity_delta);

    let amount = 1000;
    let swap_amount = TokenAmount(amount);

    app.set_token_balances("alice", &[("tokenx", &[("bob", amount)])])
        .unwrap();
    app.approve_token("tokenx", "bob", clmm_addr.as_str(), amount)
        .unwrap();

    let slippage = SqrtPrice::new(MIN_SQRT_PRICE);
    let target_sqrt_price = app
        .quote(
            clmm_addr.as_str(),
            &pool_key,
            true,
            swap_amount,
            true,
            slippage,
        )
        .unwrap()
        .target_sqrt_price;

    let before_dex_x = app
        .query_token_balance(token_x.as_str(), clmm_addr.as_str())
        .unwrap()
        .u128();
    let before_dex_y = app
        .query_token_balance(token_y.as_str(), clmm_addr.as_str())
        .unwrap()
        .u128();

    app.swap(
        "bob",
        clmm_addr.as_str(),
        &pool_key,
        true,
        swap_amount,
        true,
        target_sqrt_price,
    )
    .unwrap();

    // Load states
    let pool = app
        .get_pool(
            clmm_addr.as_str(),
            token_x.as_str(),
            token_y.as_str(),
            fee_tier,
        )
        .unwrap();
    let lower_tick = app
        .get_tick(clmm_addr.as_str(), &pool_key, lower_tick_index)
        .unwrap();
    let middle_tick = app
        .get_tick(clmm_addr.as_str(), &pool_key, middle_tick_index)
        .unwrap();
    let upper_tick = app
        .get_tick(clmm_addr.as_str(), &pool_key, upper_tick_index)
        .unwrap();
    let lower_tick_bit = app
        .is_tick_initialized(clmm_addr.as_str(), &pool_key, lower_tick_index)
        .unwrap();
    let middle_tick_bit = app
        .is_tick_initialized(clmm_addr.as_str(), &pool_key, middle_tick_index)
        .unwrap();
    let upper_tick_bit = app
        .is_tick_initialized(clmm_addr.as_str(), &pool_key, upper_tick_index)
        .unwrap();
    let bob_x = app
        .query_token_balance(token_x.as_str(), "bob")
        .unwrap()
        .u128();
    let bob_y = app
        .query_token_balance(token_y.as_str(), "bob")
        .unwrap()
        .u128();
    let dex_x = app
        .query_token_balance(token_x.as_str(), clmm_addr.as_str())
        .unwrap()
        .u128();
    let dex_y = app
        .query_token_balance(token_y.as_str(), clmm_addr.as_str())
        .unwrap()
        .u128();
    let delta_dex_y = before_dex_y - dex_y;
    let delta_dex_x = dex_x - before_dex_x;
    let expected_y = amount - 10;
    let expected_x = 0u128;

    // Check balances
    assert_eq!(bob_x, expected_x);
    assert_eq!(bob_y, expected_y);
    assert_eq!(delta_dex_x, amount);
    assert_eq!(delta_dex_y, expected_y);

    // Check Pool
    assert_eq!(pool.fee_growth_global_y, FeeGrowth::new(0));
    assert_eq!(
        pool.fee_growth_global_x,
        FeeGrowth::new(40000000000000000000000)
    );
    assert_eq!(pool.fee_protocol_token_y, TokenAmount(0));
    assert_eq!(pool.fee_protocol_token_x, TokenAmount(2));

    // Check Ticks
    assert_eq!(lower_tick.liquidity_change, liquidity_delta);
    assert_eq!(middle_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.liquidity_change, liquidity_delta);
    assert_eq!(upper_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert_eq!(
        middle_tick.fee_growth_outside_x,
        FeeGrowth::new(30000000000000000000000)
    );
    assert_eq!(lower_tick.fee_growth_outside_x, FeeGrowth::new(0));
    assert!(lower_tick_bit);
    assert!(middle_tick_bit);
    assert!(upper_tick_bit);
}
