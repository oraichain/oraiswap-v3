use cosmwasm_std::{coin, Addr, Uint128};
use decimal::{Decimal, Factories};

use crate::{
    create_entry_points_testing,
    liquidity::Liquidity,
    msg::{ExecuteMsg, QueryMsg},
    percentage::Percentage,
    sqrt_price::{calculate_sqrt_price, SqrtPrice},
    tests::helper::MockApp,
    token_amount::TokenAmount,
    FeeTier, PoolKey, Position, MIN_SQRT_PRICE,
};

#[test]
fn test_claim() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    println!("{:?}", initial_amount);
    let mut app = MockApp::new(&[
        ("owner", &[coin(initial_amount, "orai")]),
        ("alice", &[coin(initial_amount, "orai")]),
    ]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

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

    // Claim fee
    let user_amount_before_claim = app.query_token_balance(token_x.as_str(), "alice").unwrap();
    let dex_amount_before_claim = app.query_token_balance(token_x.as_str(), clmm_addr.as_str()).unwrap();

    let claim_fee_msg = ExecuteMsg::ClaimFee { index: 0 };
    app.execute(
        Addr::unchecked("alice"),
        Addr::unchecked(clmm_addr.clone()),
        &claim_fee_msg,
        &[],
    )
    .unwrap();

    let user_amount_after_claim = app.query_token_balance(token_x.as_str(), "alice").unwrap();
    let dex_amount_after_claim = app.query_token_balance(token_x.as_str(), clmm_addr.as_str()).unwrap();
    let position: Position = app.query(
        Addr::unchecked(clmm_addr.clone()),
        &QueryMsg::Position { owner_id: Addr::unchecked("alice"), index: 1 },
    ).unwrap();
    let pool = app.get_pool(clmm_addr.as_str(), token_x.as_str(), token_y.as_str(), fee_tier.clone()).unwrap();
    let expected_tokens_claimed = Uint128::new(5);

    assert_eq!(
        user_amount_after_claim - expected_tokens_claimed,
        user_amount_before_claim
    );
    assert_eq!(
        dex_amount_after_claim + expected_tokens_claimed,
        dex_amount_before_claim
    );
    assert_eq!(position.fee_growth_inside_x, pool.fee_growth_global_x);
    assert_eq!(position.tokens_owed_x, TokenAmount(0));
}


#[test]
fn test_claim_not_owner() {
    let protocol_fee = Percentage::from_scale(6, 3);
    let initial_amount = 10u128.pow(10);
    let mut app = MockApp::new(&[
        ("owner", &[coin(initial_amount, "orai")]),
        ("alice", &[coin(initial_amount, "orai")]),
        ("bob", &[coin(initial_amount, "orai")]),
    ]);
    app.set_token_contract(Box::new(create_entry_points_testing!(cw20_base)));

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

    let claim_fee_msg = ExecuteMsg::ClaimFee { index: 0 };
    let result = app.execute(
        Addr::unchecked("bob"),
        Addr::unchecked(clmm_addr.clone()),
        &claim_fee_msg,
        &[],
    ).unwrap_err();

    assert!(result.contains("error executing WasmMsg"));
}